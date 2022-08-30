use crate::flow::Node;
use crate::types::{Address, Edge, U256};
use std::cmp::Reverse;
use std::collections::HashMap;
use std::collections::HashSet;

struct Adjacencies<'a> {
    edges: &'a HashMap<Address, Vec<Edge>>,
    lazy_adjacencies: HashMap<Node, HashMap<Node, U256>>,
    capacity_adjustments: HashMap<Node, HashMap<Node, U256>>,
}

fn pseudo_node(edge: Edge) -> Node {
    Node::TokenEdge(edge.from, edge.token)
}

fn source_address_of(node: &Node) -> &Address {
    match node {
        Node::Node(addr) => addr,
        Node::TokenEdge(from, _) => from,
    }
}

impl<'a> Adjacencies<'a> {
    pub fn new(edges: &'a HashMap<Address, Vec<Edge>>) -> Self {
        Adjacencies {
            edges,
            lazy_adjacencies: HashMap::new(),
            capacity_adjustments: HashMap::new(),
        }
    }

    pub fn outgoing_edges_sorted_by_capacity(&mut self, from: &Node) -> Vec<(Node, U256)> {
        let mut adjacencies = self.adjacencies_from(from);
        if let Some(adjustments) = self.capacity_adjustments.get(from) {
            for (node, c) in adjustments {
                *adjacencies.entry(node.clone()).or_default() += *c;
            }
        }
        let mut result = adjacencies.into_iter().collect::<Vec<(Node, U256)>>();
        result.sort_unstable_by_key(|(_, capacity)| Reverse(*capacity));
        result
    }

    pub fn adjust_capacity(&mut self, from: Node, to: Node, adjustment: U256) {
        *self
            .capacity_adjustments
            .entry(from)
            .or_default()
            .entry(to)
            .or_default() += adjustment;
    }

    pub fn is_adjacent(&mut self, from: &Node, to: &Node) -> bool {
        // TODO More efficiently?
        if let Some(capacity) = self.adjacencies_from(from).get(to) {
            *capacity > U256::from(0)
        } else {
            false
        }
    }

    fn adjacencies_from(&mut self, from: &Node) -> HashMap<Node, U256> {
        self.lazy_adjacencies
            .entry(from.clone())
            .or_insert_with(|| {
                let mut result: HashMap<Node, U256> = HashMap::new();
                for edge in &self.edges[source_address_of(from)] {
                    match from {
                        Node::Node(_) => {
                            // One edge from "from" to "from x token" with a capacity
                            // as the max over all contributing edges (the balance of the sender)
                            result
                                .entry(pseudo_node(*edge))
                                .and_modify(|c| {
                                    if edge.capacity > *c {
                                        *c = edge.capacity;
                                    }
                                })
                                .or_insert(edge.capacity);
                        }
                        Node::TokenEdge(_, _) => {
                            // Another edge from "from x token" to "to" with its
                            // own capacity (based on the trust)
                            if pseudo_node(*edge) == *from {
                                result.insert(Node::Node(edge.to), edge.capacity);
                            }
                        }
                    }
                }
                result
            })
            .clone()
    }
}
