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
) -> (U256, Vec<Edge>) {
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
    let transfers = if flow == U256::from(0) {
        vec![]
    } else {
        extract_transfers(source, sink, &flow, used_edges)
    };
    println!("Num transfers: {}", transfers.len());
    (flow, transfers)
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
        println!(
            "Finding next transfers. Number of non-zero-balance accounts: {}",
            account_balances.len()
        );
        let edge = next_full_capacity_edge(&mut used_edges, &mut account_balances);
        assert!(account_balances.contains_key(&edge.from));
        account_balances
            .entry(edge.from)
            .and_modify(|balance| *balance -= edge.capacity);
        *account_balances
            .entry(edge.to)
            .or_default() += edge.capacity;
        account_balances.retain(|_account, balance| balance > &mut U256::from(0));
        used_edges
            .entry(Node::Node(edge.from))
            .and_modify(|outgoing| {
                outgoing.remove(&Node::TokenEdge(edge.from, edge.token));
            });
        transfers.push(edge);
    }

    transfers
}

fn next_full_capacity_edge(
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
    account_balances: &HashMap<Address, U256>,
) -> Edge {
    for (account, balance) in account_balances {
        println!("Account: {account} - balance: {balance}");
        for (intermediate, _) in used_edges
            .get(&Node::Node(*account))
            .unwrap_or(&HashMap::default())
        {
            let (from, token) = node_as_token_edge(intermediate);
            for (to_node, capacity) in &used_edges[intermediate] {
                println!(" - used edge to {to_node} with capacity {capacity}");
                let to = node_as_address(to_node);
                if *capacity == U256::from(0) {
                    continue;
                }
                if *balance >= *capacity {
                    println!("Found an edge: {from} -> {to} [{token}] {capacity}");
                    return Edge {
                        from: *from,
                        to: *to,
                        token: *token,
                        capacity: *capacity,
                    };
                }
            }
        }
    }
    panic!();
}

#[cfg(test)]
mod test {
    use super::*;

    fn addresses() -> (Address, Address, Address, Address, Address, Address) {
        (
            Address::from("0x11C7e86fF693e9032A0F41711b5581a04b26Be2E"),
            Address::from("0x22cEDde51198D1773590311E2A340DC06B24cB37"),
            Address::from("0x33cEDde51198D1773590311E2A340DC06B24cB37"),
            Address::from("0x447EDde51198D1773590311E2A340DC06B24cB37"),
            Address::from("0x55c16ce62d26fd51582a646e2e30a3267b1e6d7e"),
            Address::from("0x66c16ce62d26fd51582a646e2e30a3267b1e6d7e"),
        )
    }
    fn build_edges(input: Vec<Edge>) -> HashMap<Address, Vec<Edge>> {
        let mut output: HashMap<Address, Vec<Edge>> = HashMap::new();
        for e in input {
            output.entry(e.from).or_default().push(e);
        }
        output
    }

    #[test]
    fn direct() {
        let (a, b, t, ..) = addresses();
        let edges = build_edges(vec![
            Edge{from: a, to: b, token: t, capacity: U256::from(10)}
        ]);
        let flow = compute_flow(&a, &b, &edges);
        assert_eq!(flow, (U256::from(10), edges[&a].clone()));
    }

    #[test]
    fn one_hop() {
        let (a, b, c, t1, t2, ..) = addresses();
        let edges = build_edges(vec![
            Edge{from: a, to: b, token: t1, capacity: U256::from(10)},
            Edge{from: b, to: c, token: t2, capacity: U256::from(8)},
        ]);
        let flow = compute_flow(&a, &c, &edges);
        assert_eq!(flow, (U256::from(8), vec![
            Edge{from: a, to: b, token: t1, capacity: U256::from(8)},
            Edge{from: b, to: c, token: t2, capacity: U256::from(8)},
        ]));
    }

    #[test]
    fn diamond() {
        let (a, b, c, d, t1, t2) = addresses();
        let edges = build_edges(vec![
            Edge{from: a, to: b, token: t1, capacity: U256::from(10)},
            Edge{from: a, to: c, token: t2, capacity: U256::from(7)},
            Edge{from: b, to: d, token: t2, capacity: U256::from(9)},
            Edge{from: c, to: d, token: t1, capacity: U256::from(8)},
        ]);
        let mut flow = compute_flow(&a, &d, &edges);
        flow.1.sort();
        assert_eq!(flow, (U256::from(16), vec![
            Edge{from: a, to: b, token: t1, capacity: U256::from(9)},
            Edge{from: a, to: c, token: t2, capacity: U256::from(7)},
            Edge{from: b, to: d, token: t2, capacity: U256::from(9)},
            Edge{from: c, to: d, token: t1, capacity: U256::from(7)},
        ]));
    }
}