use crate::graph::adjacencies::Adjacencies;
use crate::graph::Node;
use crate::types::{Address, U256};
use std::cmp::min;
use std::collections::{HashMap, VecDeque};

pub fn augmenting_path(
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
            // * 3 because we have three edges per trust connection (two intermediate nodes).
            if depth >= max * 3 {
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
