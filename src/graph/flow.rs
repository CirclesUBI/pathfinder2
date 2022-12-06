use crate::graph::adjacencies::Adjacencies;
use crate::graph::{node_as_address, node_as_token_edge, Node};
use crate::types::edge::EdgeDB;
use crate::types::{Address, Edge, U256};
use std::cmp::min;
use std::collections::VecDeque;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

pub fn compute_flow(
    source: &Address,
    sink: &Address,
    edges: &EdgeDB,
    requested_flow: U256,
    max_distance: Option<u64>,
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

    println!("Max flow: {flow}");

    if flow > requested_flow {
        let still_to_prune = prune_flow(source, sink, flow - requested_flow, &mut used_edges);
        flow = requested_flow + still_to_prune;
    }

    let transfers = if flow == U256::from(0) {
        vec![]
    } else {
        extract_transfers(source, sink, &flow, used_edges)
    };
    // TODO We can simplify the transfers:
    // If we have a transfer (A, B, T) and a transfer (B, C, T),
    // We can always replace both by (A, C, T).
    println!("Num transfers: {}", transfers.len());
    (flow, transfers)
}

pub fn transfers_to_dot(edges: &Vec<Edge>) -> String {
    let mut out = String::new();
    writeln!(out, "digraph transfers {{").expect("");

    for Edge {
        from,
        to,
        token,
        capacity,
    } in edges
    {
        let t = if token == from {
            "(trust)".to_string()
        } else if token == to {
            String::new()
        } else {
            format!(" ({})", token.short())
        };
        writeln!(
            out,
            "    \"{}\" -> \"{}\" [label=\"{}{}\"];",
            from.short(),
            to.short(),
            capacity.to_decimal_fraction(),
            t
        )
        .expect("");
    }
    writeln!(out, "}}").expect("");
    out
}

fn augmenting_path(
    source: &Address,
    sink: &Address,
    adjacencies: &mut Adjacencies,
    max_distance: Option<u64>,
) -> (U256, Vec<Node>) {
    let mut parent = HashMap::new();
    if *source == *sink {
        return (U256::default(), vec![]);
    }
    let mut queue = VecDeque::<(Node, (u64, U256))>::new();
    queue.push_back((Node::Node(*source), (0, U256::default() - U256::from(1))));
    while let Some((node, (depth, flow))) = queue.pop_front() {
        if let Some(max) = max_distance {
            if depth >= max {
                continue;
            }
        }
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
                queue.push_back((target, (depth + 1, new_flow)));
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

#[allow(dead_code)]
fn to_dot(
    edges: &HashMap<Node, HashMap<Node, U256>>,
    account_balances: &HashMap<Address, U256>,
) -> String {
    let mut out = String::new();
    writeln!(out, "digraph used_edges {{").expect("");

    for (address, balance) in account_balances {
        writeln!(out, "    \"{address}\" [label=\"{address}: {balance}\"];",).expect("");
    }
    for (from, out_edges) in edges {
        for (to, capacity) in out_edges {
            writeln!(out, "    \"{from}\" -> \"{to}\" [label=\"{capacity}\"];",).expect("");
        }
    }
    writeln!(out, "}}").expect("");
    out
}

fn prune_flow(
    source: &Address,
    sink: &Address,
    mut flow_to_prune: U256,
    used_edges: &mut HashMap<Node, HashMap<Node, U256>>,
) -> U256 {
    // Note the path length is negative to sort by longest shortest path first.
    let edges_by_path_length = compute_edges_by_path_length(source, sink, used_edges);

    for edges_here in edges_by_path_length.values() {
        //println!("Shorter path.");
        // As long as `edges` contain an edge with smaller weight than the weight still to prune:
        //   take the smallest such edge and prune it.
        while flow_to_prune > U256::from(0) && !edges_here.is_empty() {
            //println!("Still to prune: {}", flow_to_prune);
            if let Some((s, t)) = smallest_edge_in_set(used_edges, edges_here) {
                if used_edges[&s][&t] > flow_to_prune {
                    break;
                };
                flow_to_prune = prune_edge(used_edges, (&s, &t), flow_to_prune);
            } else {
                break;
            }
        }
    }
    // If there is still flow to prune, take the first element in edgesByPathLength
    // and partially prune its path.
    if flow_to_prune > U256::from(0) {
        //println!("Final stage: Still to prune: {}", flow_to_prune);
        for edges_here in edges_by_path_length.values() {
            for (a, b) in edges_here {
                if !used_edges.contains_key(a) || !used_edges[a].contains_key(b) {
                    continue;
                }
                flow_to_prune = prune_edge(used_edges, (a, b), flow_to_prune);
                if flow_to_prune == U256::from(0) {
                    return U256::from(0);
                }
            }
            if flow_to_prune == U256::from(0) {
                return U256::from(0);
            }
        }
    }
    flow_to_prune
}

/// Returns a map from the negative shortest path length to the edge.
/// The shortest path length is negative so that it is sorted by
/// longest paths first - those are the ones we want to eliminate first.
fn compute_edges_by_path_length(
    source: &Address,
    sink: &Address,
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
) -> BTreeMap<i64, HashSet<(Node, Node)>> {
    let mut result = BTreeMap::<i64, HashSet<(Node, Node)>>::new();
    let from_source = distance_from_source(&Node::Node(*source), used_edges);
    let to_sink = distance_to_sink(&Node::Node(*sink), used_edges);
    for (s, edges) in used_edges {
        for t in edges.keys() {
            let path_length = from_source[s] + 1 + to_sink[t];
            result
                .entry(-path_length)
                .or_default()
                .insert((s.clone(), t.clone()));
        }
    }
    result
}

fn distance_from_source(
    source: &Node,
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
) -> HashMap<Node, i64> {
    let mut distances = HashMap::<Node, i64>::new();
    let mut to_process = VecDeque::<Node>::new();
    distances.insert(source.clone(), 0);
    to_process.push_back(source.clone());

    while let Some(n) = to_process.pop_front() {
        for (t, capacity) in used_edges.get(&n).unwrap_or(&HashMap::new()) {
            if *capacity > U256::from(0) && !distances.contains_key(t) {
                distances.insert(t.clone(), distances[&n] + 1);
                to_process.push_back(t.clone());
            }
        }
    }

    distances
}

fn distance_to_sink(
    sink: &Node,
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
) -> HashMap<Node, i64> {
    distance_from_source(sink, &reverse_edges(used_edges))
}

fn reverse_edges(
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
) -> HashMap<Node, HashMap<Node, U256>> {
    let mut reversed: HashMap<Node, HashMap<Node, U256>> = HashMap::new();
    for (n, edges) in used_edges {
        for (t, capacity) in edges {
            reversed
                .entry(t.clone())
                .or_default()
                .insert(n.clone(), *capacity);
        }
    }
    reversed
}

fn smallest_edge_in_set(
    all_edges: &HashMap<Node, HashMap<Node, U256>>,
    edge_set: &HashSet<(Node, Node)>,
) -> Option<(Node, Node)> {
    if let Some((a, b, _)) = edge_set
        .iter()
        .map(|(a, b)| {
            let capacity = if let Some(out) = all_edges.get(a) {
                if let Some(capacity) = out.get(b) {
                    assert!(*capacity != U256::from(0));
                    Some(capacity)
                } else {
                    None
                }
            } else {
                None
            };
            (a, b, capacity)
        })
        .filter(|(_, _, capacity)| capacity.is_some())
        .min_by_key(|(_, _, capacity)| capacity.unwrap())
    {
        Some((a.clone(), b.clone()))
    } else {
        None
    }
}

fn smallest_edge_from(
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
    n: &Node,
) -> Option<(Node, U256)> {
    used_edges.get(n).and_then(|out| {
        out.iter()
            .min_by_key(|(_, c)| {
                assert!(**c != U256::from(0));
                *c
            })
            .map(|(t, c)| (t.clone(), *c))
    })
}

fn smallest_edge_to(
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
    n: &Node,
) -> Option<(Node, U256)> {
    used_edges
        .iter()
        .filter(|(_, out)| out.contains_key(n))
        .map(|(t, out)| (t, out[n]))
        .min_by_key(|(_, c)| {
            assert!(*c != U256::from(0));
            *c
        })
        .map(|(t, c)| (t.clone(), c))
}

/// Removes the edge (potentially partially), removing a given amount of flow.
/// Returns the remaining flow to prune if the edge was too small.
fn prune_edge(
    used_edges: &mut HashMap<Node, HashMap<Node, U256>>,
    edge: (&Node, &Node),
    flow_to_prune: U256,
) -> U256 {
    let edge_size = min(flow_to_prune, used_edges[edge.0][edge.1]);
    reduce_capacity(used_edges, edge, &edge_size);
    prune_path(used_edges, edge.1, edge_size, PruneDirection::Forwards);
    prune_path(used_edges, edge.0, edge_size, PruneDirection::Backwards);
    flow_to_prune - edge_size
}

fn reduce_capacity(
    used_edges: &mut HashMap<Node, HashMap<Node, U256>>,
    (a, b): (&Node, &Node),
    reduction: &U256,
) {
    let out_edges = used_edges.get_mut(a).unwrap();
    *out_edges.get_mut(b).unwrap() -= *reduction;
    if out_edges[b] == U256::from(0) {
        out_edges.remove_entry(b);
    }
}

#[derive(Clone, Copy)]
enum PruneDirection {
    Forwards,
    Backwards,
}

fn prune_path(
    used_edges: &mut HashMap<Node, HashMap<Node, U256>>,
    n: &Node,
    mut flow_to_prune: U256,
    direction: PruneDirection,
) {
    while let Some((next, mut capacity)) = match direction {
        PruneDirection::Forwards => smallest_edge_from(used_edges, n),
        PruneDirection::Backwards => smallest_edge_to(used_edges, n),
    } {
        capacity = min(flow_to_prune, capacity);
        match direction {
            PruneDirection::Forwards => reduce_capacity(used_edges, (n, &next), &capacity),
            PruneDirection::Backwards => reduce_capacity(used_edges, (&next, n), &capacity),
        };
        prune_path(used_edges, &next, capacity, direction);
        flow_to_prune -= capacity;
        if flow_to_prune == U256::from(0) {
            return;
        }
    }
}

fn extract_transfers(
    source: &Address,
    sink: &Address,
    amount: &U256,
    mut used_edges: HashMap<Node, HashMap<Node, U256>>,
) -> Vec<Edge> {
    let mut transfers: Vec<Edge> = Vec::new();
    let mut account_balances: HashMap<Address, U256> = HashMap::new();
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
        assert!(used_edges.contains_key(&Node::TokenEdge(edge.from, edge.token)));
        used_edges
            .entry(Node::TokenEdge(edge.from, edge.token))
            .and_modify(|outgoing| {
                assert!(outgoing.contains_key(&Node::Node(edge.to)));
                outgoing.remove(&Node::Node(edge.to));
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
        for intermediate in used_edges
            .get(&Node::Node(*account))
            .unwrap_or(&HashMap::new())
            .keys()
        {
            for (to_node, capacity) in &used_edges[intermediate] {
                let (from, token) = node_as_token_edge(intermediate);
                if *balance >= *capacity {
                    return Edge {
                        from: *from,
                        to: *node_as_address(to_node),
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
    fn build_edges(input: Vec<Edge>) -> EdgeDB {
        EdgeDB::new(input)
    }

    #[test]
    fn direct() {
        let (a, b, t, ..) = addresses();
        let edges = build_edges(vec![Edge {
            from: a,
            to: b,
            token: t,
            capacity: U256::from(10),
        }]);
        let flow = compute_flow(&a, &b, &edges, U256::MAX, None);
        assert_eq!(
            flow,
            (
                U256::from(10),
                vec![Edge {
                    from: a,
                    to: b,
                    token: t,
                    capacity: U256::from(10)
                }]
            )
        );
    }

    #[test]
    fn one_hop() {
        let (a, b, c, t1, t2, ..) = addresses();
        let edges = build_edges(vec![
            Edge {
                from: a,
                to: b,
                token: t1,
                capacity: U256::from(10),
            },
            Edge {
                from: b,
                to: c,
                token: t2,
                capacity: U256::from(8),
            },
        ]);
        let flow = compute_flow(&a, &c, &edges, U256::MAX, None);
        assert_eq!(
            flow,
            (
                U256::from(8),
                vec![
                    Edge {
                        from: a,
                        to: b,
                        token: t1,
                        capacity: U256::from(8)
                    },
                    Edge {
                        from: b,
                        to: c,
                        token: t2,
                        capacity: U256::from(8)
                    },
                ]
            )
        );
    }

    #[test]
    fn diamond() {
        let (a, b, c, d, t1, t2) = addresses();
        let edges = build_edges(vec![
            Edge {
                from: a,
                to: b,
                token: t1,
                capacity: U256::from(10),
            },
            Edge {
                from: a,
                to: c,
                token: t2,
                capacity: U256::from(7),
            },
            Edge {
                from: b,
                to: d,
                token: t2,
                capacity: U256::from(9),
            },
            Edge {
                from: c,
                to: d,
                token: t1,
                capacity: U256::from(8),
            },
        ]);
        let mut flow = compute_flow(&a, &d, &edges, U256::MAX, None);
        flow.1.sort();
        assert_eq!(
            flow,
            (
                U256::from(16),
                vec![
                    Edge {
                        from: a,
                        to: b,
                        token: t1,
                        capacity: U256::from(9)
                    },
                    Edge {
                        from: a,
                        to: c,
                        token: t2,
                        capacity: U256::from(7)
                    },
                    Edge {
                        from: b,
                        to: d,
                        token: t2,
                        capacity: U256::from(9)
                    },
                    Edge {
                        from: c,
                        to: d,
                        token: t1,
                        capacity: U256::from(7)
                    },
                ]
            )
        );
        let mut pruned_flow = compute_flow(&a, &d, &edges, U256::from(6), None);
        pruned_flow.1.sort();
        assert_eq!(
            pruned_flow,
            (
                U256::from(6),
                vec![
                    Edge {
                        from: a,
                        to: b,
                        token: t1,
                        capacity: U256::from(6)
                    },
                    Edge {
                        from: b,
                        to: d,
                        token: t2,
                        capacity: U256::from(6)
                    },
                ]
            )
        );
    }
}
