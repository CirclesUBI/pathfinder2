use crate::graph::{as_trust_node, Node};
use crate::types::{Address, Edge, U256};
use std::collections::{BTreeMap, HashMap};

pub fn extract_transfers(
    source: &Address,
    sink: &Address,
    amount: &U256,
    mut used_edges: HashMap<Node, HashMap<Node, U256>>,
) -> Vec<Edge> {
    let mut transfers: Vec<Edge> = Vec::new();
    let mut account_balances: BTreeMap<Address, U256> = BTreeMap::new();
    account_balances.insert(*source, *amount);

    while !account_balances.is_empty()
        && (account_balances.len() > 1 || *account_balances.iter().next().unwrap().0 != *sink)
    {
        let edge = next_full_capacity_edge(&used_edges, &account_balances);
        assert!(account_balances[&edge.from] >= edge.capacity);
        account_balances
            .entry(edge.from)
            .and_modify(|balance| *balance -= edge.capacity);
        *account_balances.entry(edge.to).or_default() += edge.capacity;
        account_balances.retain(|_account, balance| balance > &mut U256::from(0));
        assert!(used_edges.contains_key(&Node::BalanceNode(edge.from, edge.token)));
        used_edges
            .entry(Node::BalanceNode(edge.from, edge.token))
            .and_modify(|outgoing| {
                assert!(outgoing.contains_key(&Node::TrustNode(edge.to, edge.token)));
                outgoing.remove(&Node::TrustNode(edge.to, edge.token));
            });
        transfers.push(edge);
    }

    transfers
}

fn next_full_capacity_edge(
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
    account_balances: &BTreeMap<Address, U256>,
) -> Edge {
    for (account, balance) in account_balances {
        let edge = used_edges
            .get(&Node::Node(*account))
            .map(|v| {
                v.keys().flat_map(|intermediate| {
                    used_edges[intermediate]
                        .iter()
                        .filter(|(_, capacity)| *balance >= **capacity)
                        .map(|(trust_node, capacity)| {
                            let (to, token) = as_trust_node(trust_node);
                            Edge {
                                from: *account,
                                to: *to,
                                token: *token,
                                capacity: *capacity,
                            }
                        })
                })
            })
            .and_then(|edges| edges.min());
        if let Some(edge) = edge {
            return edge;
        }
    }
    panic!();
}
