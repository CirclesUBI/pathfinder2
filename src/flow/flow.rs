use crate::flow::adjacencies::Adjacencies;
use crate::flow::{node_as_address, node_as_token_edge, Node};
use crate::types::{Address, Edge, U256};
use std::cmp::min;
use std::collections::HashMap;
use std::collections::VecDeque;

pub fn compute_flow(
    source: &Address,
    sink: &Address,
    edges: &HashMap<Address, Vec<Edge>>,
) -> String {
    let mut adjacencies = Adjacencies::new(edges);
    let mut used_edges: HashMap<Node, HashMap<Node, U256>> = HashMap::new();

    let mut flow = U256::default();
    loop {
        let (new_flow, parents) = augmenting_path(source, sink, &mut adjacencies);
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

    // TODO prune

    println!("Max flow: {flow}");
    let transfers = extract_transfers(source, sink, &flow, used_edges);
    println!("Num transfers: {}", transfers.len());
    flow.to_string()
}

fn augmenting_path(
    source: &Address,
    sink: &Address,
    adjacencies: &mut Adjacencies,
) -> (U256, Vec<Node>) {
    let mut parent = HashMap::new();
    if *source == *sink {
        return (U256::default(), vec![]);
    }
    let mut queue = VecDeque::<(Node, U256)>::new();
    queue.push_back((Node::Node(*source), U256::default() - U256::from(1)));
    while let Some((node, flow)) = queue.pop_front() {
        for (target, capacity) in adjacencies.outgoing_edges_sorted_by_capacity(&node) {
            if !parent.contains_key(&target) && capacity > U256::default() {
                parent.insert(target.clone(), node.clone());
                let new_flow = min(flow, capacity);
                if target == Node::Node(*sink) {
                    return (
                        new_flow,
                        trace(parent, &Node::Node(*source), &Node::Node(*sink)),
                    );
                }
                queue.push_back((target, new_flow));
            }
        }
    }
    (U256::default(), vec![])
}

fn trace(parent: HashMap<Node, Node>, source: &Node, sink: &Node) -> Vec<Node> {
    let mut t = vec![sink.clone()];
    let mut node = sink;
    loop {
        node = parent.get(node).unwrap();
        t.push(node.clone());
        if *node == *source {
            break;
        }
    }
    t
}

fn extract_transfers(
    source: &Address,
    sink: &Address,
    amount: &U256,
    mut used_edges: HashMap<Node, HashMap<Node, U256>>,
) -> Vec<Edge> {
    let mut transfers: Vec<Edge> = Vec::new();
    let mut account_balances: HashMap<Address, U256> = HashMap::new();
    account_balances.insert(*source, amount.clone());

    while !account_balances.is_empty()
        && (account_balances.len() > 1 || *account_balances.iter().nth(0).unwrap().0 != *sink)
    {
        let next = extract_next_transfers(&mut used_edges, &mut account_balances);
        assert!(!next.is_empty());
        transfers.extend(next.into_iter());
    }

    transfers
}

/// Extract the next list of transfers until we get to a situation where
/// we cannot transfer the full balance and start over.
fn extract_next_transfers(
    used_edges: &mut HashMap<Node, HashMap<Node, U256>>,
    account_balances: &mut HashMap<Address, U256>,
) -> Vec<Edge> {
    let mut transfers = Vec::new();

    loop {
        let first_edge = transfers.is_empty();
        if let Some(edge) = next_nonzero_edge(used_edges, account_balances, first_edge) {
            account_balances
                .entry(edge.from)
                .and_modify(|balance| *balance -= edge.capacity);
            account_balances
                .entry(edge.to)
                .and_modify(|balance| *balance += edge.capacity);
            account_balances.retain(|_account, balance| balance > &mut U256::from(0));
            used_edges
                .entry(Node::Node(edge.from))
                .and_modify(|outgoing| {
                    outgoing.remove(&Node::TokenEdge(edge.from, edge.token));
                });
            transfers.push(edge);
        } else {
            return transfers;
        }
    }
}

fn next_nonzero_edge(
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
    account_balances: &HashMap<Address, U256>,
    first_edge: bool,
) -> Option<Edge> {
    for (account, balance) in account_balances {
        for (intermediate, _) in &used_edges[&Node::Node(*account)] {
            let (from, token) = node_as_token_edge(intermediate);
            for (to_node, capacity) in &used_edges[intermediate] {
                let to = node_as_address(to_node);
                if *capacity == U256::from(0) {
                    continue;
                }
                if *balance < *capacity {
                    // We do not have enough balance yet, there will be another transfer along this edge.
                    if first_edge {
                        continue;
                    } else {
                        return None;
                    }
                } else {
                    return Some(Edge {
                        from: *from,
                        to: *to,
                        token: *token,
                        capacity: *capacity,
                    });
                }
            }
        }
    }
    None
}
