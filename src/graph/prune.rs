use crate::graph::Node;
use crate::types::{Address, U256};
use std::cmp::min;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

pub fn prune_flow(
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
        .min_by_key(|(a, b, capacity)| (capacity.unwrap(), *a, *b))
    {
        Some((a.clone(), b.clone()))
    } else {
        None
    }
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

/// Removes the edge (potentially partially), removing a given amount of flow.
/// Returns the remaining flow to prune if the edge was too small.
pub fn prune_edge(
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

fn smallest_edge_to(
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
    n: &Node,
) -> Option<(Node, U256)> {
    used_edges
        .iter()
        .filter(|(_, out)| out.contains_key(n))
        .map(|(t, out)| (t, out[n]))
        .min_by_key(|(addr, c)| {
            assert!(*c != U256::from(0));
            (*c, *addr)
        })
        .map(|(t, c)| (t.clone(), c))
}

fn smallest_edge_from(
    used_edges: &HashMap<Node, HashMap<Node, U256>>,
    n: &Node,
) -> Option<(Node, U256)> {
    used_edges.get(n).and_then(|out| {
        out.iter()
            .min_by_key(|(addr, c)| {
                assert!(**c != U256::from(0));
                (*c, *addr)
            })
            .map(|(t, c)| (t.clone(), *c))
    })
}
