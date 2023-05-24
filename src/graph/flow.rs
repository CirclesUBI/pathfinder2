use crate::graph::adjacencies::Adjacencies;
use crate::graph::{Node};
use crate::types::edge::EdgeDB;
use crate::types::{Address, Edge, U256};
use std::collections::{HashMap, VecDeque};
use crate::graph::augmenting_path::augmenting_path;
use crate::graph::extract_transfers::extract_transfers;
use crate::graph::prune::{prune_edge, prune_flow};
use crate::rpc::call_context::CallContext;

/**
 The following is a description of how the max flow algorithm is implemented in this codebase.

 1) Setting up the Graph:
 The network graph is initialized with the provided edges and capacities.
 Nodes represent addresses, and edges represent the available flow capacity between nodes.

 2) Finding Paths:
 The Ford-Fulkerson algorithm is applied to find augmenting paths.
 The algorithm starts at the source node and explores the network graph looking for paths to the sink node.
 The selection of which node to traverse next is performed in a breadth-first manner, prioritizing the ones with the highest remaining capacity.
 However, the actual method could in theory be any traversal method.

 3) Sending Flow:
 When an augmenting path is found (a path from the source to the sink with some unused capacity), the algorithm sends a flow along this path.
 The amount of flow sent equals the bottleneck capacity (the smallest capacity on the path).
 This flow is now part of the total flow from the source to the sink.
 It also adjusts the remaining capacities of the edges on this path accordingly.
 The algorithm maintains a data structure, used_edges, to keep track of these adjustments.

 4) Maximizing Flow:
 The algorithm keeps track of the total flow sent from the source to the sink.
 It continues finding augmenting paths and sending flow along them until no more augmenting paths can be found.
 At this point, the total flow sent from the source to the sink is the maximum possible flow under the capacity constraints of the edges.
 This is the solution to the max-flow problem.

 5) Repeating the Process:
 The algorithm repeats the process of finding paths and sending flow until it is no longer able to find a path from the source to the sink.
 This implies that we've achieved the maximum flow possible in the network under the given conditions.

 6) Flow Capacity Adjustments:
 The adjustments to the edge capacities are stored in the used_edges structure, allowing the tracking of how much flow has been sent along each edge in the network.
 This structure is used later in the computation to prune the flow if it exceeds the requested amount and to reduce transfers if they exceed a specified maximum number.
*/

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

    let flow = compute_max_flow(source, sink, &mut adjacencies, &mut used_edges, max_distance);
    call_context.log_message(format!("Max flow: {}", flow.to_decimal()).as_str());

    let flow = prune_excess_flow(source, sink, flow, requested_flow, &mut used_edges);
    call_context.log_message(format!("Flow after pruning: {}", flow.to_decimal()).as_str());

    let flow = reduce_transfers_if_needed(max_transfers, flow, &mut used_edges, call_context);
    call_context.log_message(format!("Flow after limiting transfer steps to {}: {}", max_transfers.unwrap_or_default(), flow.to_decimal()).as_str());

    let transfers = create_sorted_transfers(source, sink, flow, used_edges, call_context);
    // call_context.log_message(format!("Transfers: {:?}", transfers).as_str());

    (flow, transfers)
}

fn compute_max_flow(
    source: &Address,
    sink: &Address,
    adjacencies: &mut Adjacencies,
    used_edges: &mut HashMap<Node, HashMap<Node, U256>>,
    max_distance: Option<u64>,
) -> U256 {
    let mut flow = U256::default();
    loop {
        let (new_flow, parents) = augmenting_path(source, sink, adjacencies, max_distance);
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
    flow
}

fn prune_excess_flow(
    source: &Address,
    sink: &Address,
    flow: U256,
    requested_flow: U256,
    used_edges: &mut HashMap<Node, HashMap<Node, U256>>,
) -> U256 {
    if flow > requested_flow {
        let still_to_prune = prune_flow(source, sink, flow - requested_flow, used_edges);
        return requested_flow + still_to_prune;
    }
    flow
}

fn reduce_transfers_if_needed(
    max_transfers: Option<u64>,
    flow: U256,
    used_edges: &mut HashMap<Node, HashMap<Node, U256>>,
    call_context: &CallContext,
) -> U256 {
    if let Some(max_transfers) = max_transfers {
        let lost = reduce_transfers(max_transfers * 3, used_edges);
        call_context.log_message(format!("Capacity lost by transfer count reduction: {}",
                                         lost.to_decimal_fraction()
        ).as_str());
        return flow - lost;
    }
    flow
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

fn create_sorted_transfers(
    source: &Address,
    sink: &Address,
    flow: U256,
    used_edges: HashMap<Node, HashMap<Node, U256>>,
    call_context: &CallContext,
) -> Vec<Edge> {
    if flow == U256::from(0) {
        return vec![];
    }

    let transfers = extract_transfers(source, sink, &flow, used_edges);
    call_context.log_message(format!("Num transfers: {}", transfers.len()).as_str());

    let simplified_transfers = simplify_transfers(transfers);
    call_context.log_message(format!("After simplification: {}", simplified_transfers.len()).as_str());

    sort_transfers(simplified_transfers)
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
