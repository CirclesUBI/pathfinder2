use crate::graph::adjacencies::Adjacencies;
use crate::graph::{as_trust_node, Node};
use crate::types::edge::EdgeDB;
use crate::types::{Address, Edge, U256};
use std::collections::{BTreeMap};
use std::collections::{HashMap, VecDeque};
use crate::graph::augmenting_path::augmenting_path;
use crate::graph::extract_transfers::extract_transfers;
use crate::graph::prune::{prune_edge, prune_flow};
use crate::rpc::call_context::CallContext;

pub fn compute_flow(
    source: &Address,
    sink: &Address,
    edges: &EdgeDB,
    requested_flow: U256,
    max_distance: Option<u64>,
    max_transfers: Option<u64>,
    call_context: &CallContext,
) -> (U256, Vec<Edge>) {
    let mut adjacencies = Adjacencies::new(edges);
    let mut used_edges: HashMap<Node, HashMap<Node, U256>> = HashMap::new();

    let mut flow = U256::default();
    loop {
        let (new_flow, parents) = augmenting_path(source, sink, &mut adjacencies, max_distance);
        if new_flow == U256::default() {
            break;
        }
        flow += new_flow;
        for window in parents.windows(2) {
            if let [node, prev] = window {
                adjacencies.adjust_capacity(prev, node, -new_flow);
                adjacencies.adjust_capacity(node, prev, new_flow);
                if adjacencies.is_adjacent(node, prev) {
                    *used_edges
                        .entry(node.clone())
                        .or_default()
                        .entry(prev.clone())
                        .or_default() -= new_flow;
                } else {
                    *used_edges
                        .entry(prev.clone())
                        .or_default()
                        .entry(node.clone())
                        .or_default() += new_flow;
                }
            } else {
                panic!();
            }
        }
    }

    used_edges.retain(|_, out| {
        out.retain(|_, c| *c != U256::from(0));
        !out.is_empty()
    });

    call_context.log_message(format!("Max flow: {}", flow.to_decimal()).as_str());

    if flow > requested_flow {
        let still_to_prune = prune_flow(source, sink, flow - requested_flow, &mut used_edges);
        flow = requested_flow + still_to_prune;
    }

    if let Some(max_transfers) = max_transfers {
        let lost = reduce_transfers(max_transfers * 3, &mut used_edges);
        call_context.log_message(format!("Capacity lost by transfer count reduction: {}",
            lost.to_decimal_fraction()
        ).as_str());
        flow -= lost;
    }

    let transfers = if flow == U256::from(0) {
        vec![]
    } else {
        extract_transfers(source, sink, &flow, used_edges)
    };
    call_context.log_message(format!("Num transfers: {}", transfers.len()).as_str());
    let simplified_transfers = simplify_transfers(transfers);
    call_context.log_message(format!("After simplification: {}", simplified_transfers.len()).as_str());
    let sorted_transfers = sort_transfers(simplified_transfers);
    (flow, sorted_transfers)
}

pub fn reduce_transfers(
    max_transfers: u64,
    used_edges: &mut HashMap<Node, HashMap<Node, U256>>,
) -> U256 {
    let mut reduced_flow = U256::from(0);
    while used_edges.len() > max_transfers as usize {
        let all_edges = used_edges
            .iter()
            .flat_map(|(f, e)| e.iter().map(|(t, c)| ((f.clone(), t.clone()), c)));
        if all_edges.clone().count() <= max_transfers as usize {
            return reduced_flow;
        }
        let ((f, t), c) = all_edges
            .min_by_key(|(addr, c)| (*c, addr.clone()))
            .unwrap();
        reduced_flow += *c;
        prune_edge(used_edges, (&f, &t), *c);
    }
    reduced_flow
}


fn find_pair_to_simplify(transfers: &Vec<Edge>) -> Option<(usize, usize)> {
    let l = transfers.len();
    (0..l)
        .flat_map(move |x| (0..l).map(move |y| (x, y)))
        .find(|(i, j)| {
            // We do not need matching capacity, but only then will we save
            // a transfer.
            let a = transfers[*i];
            let b = transfers[*j];
            *i != *j && a.to == b.from && a.token == b.token && a.capacity == b.capacity
        })
}

fn simplify_transfers(mut transfers: Vec<Edge>) -> Vec<Edge> {
    // We can simplify the transfers:
    // If we have a transfer (A, B, T) and a transfer (B, C, T),
    // We can always replace both by (A, C, T).

    while let Some((i, j)) = find_pair_to_simplify(&transfers) {
        transfers[i].to = transfers[j].to;
        transfers.remove(j);
    }
    transfers
}

fn sort_transfers(transfers: Vec<Edge>) -> Vec<Edge> {
    // We have to sort the transfers to satisfy the following condition:
    // A user can send away their own tokens only after it has received all (trust) transfers.

    let mut receives_to_wait_for: HashMap<Address, u64> = HashMap::new();
    for e in &transfers {
        *receives_to_wait_for.entry(e.to).or_default() += 1;
        receives_to_wait_for.entry(e.from).or_default();
    }
    let mut result = Vec::new();
    let mut queue = transfers.into_iter().collect::<VecDeque<Edge>>();
    while let Some(e) = queue.pop_front() {
        //println!("queue size: {}", queue.len());
        if *receives_to_wait_for.get(&e.from).unwrap() == 0 {
            *receives_to_wait_for.get_mut(&e.to).unwrap() -= 1;
            result.push(e)
        } else {
            queue.push_back(e);
        }
    }
    result
}
