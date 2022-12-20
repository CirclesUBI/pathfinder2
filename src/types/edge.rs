use std::collections::HashMap;

use crate::types::Address;
use crate::types::U256;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct Edge {
    pub from: Address,
    pub to: Address,
    pub token: Address,
    pub capacity: U256,
}

// TODO comparison, hash, etc. can ignore the capacity field.

pub fn eq_up_to_capacity(e1: &Edge, e2: &Edge) -> bool {
    e1.from == e2.from && e1.to == e2.to && e1.token == e2.token
}

#[derive(Debug, Default, Clone)]
pub struct EdgeDB {
    edges: Vec<Edge>,
    outgoing: HashMap<Address, Vec<usize>>,
    incoming: HashMap<Address, Vec<usize>>,
}

impl EdgeDB {
    pub fn new(edges: Vec<Edge>) -> EdgeDB {
        let outgoing = outgoing_index(&edges);
        let incoming = incoming_index(&edges);
        EdgeDB {
            edges,
            outgoing,
            incoming,
        }
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn edges(&self) -> &Vec<Edge> {
        &self.edges
    }

    pub fn update(&mut self, update: Edge) {
        match self.index_of(&update) {
            Some(i) => self.edges[i].capacity = update.capacity,
            None => {
                let i = self.edges.len();
                self.outgoing.entry(update.from).or_default().push(i);
                self.incoming.entry(update.to).or_default().push(i);
                self.edges.push(update);
            }
        }
    }

    pub fn outgoing(&self, source: &Address) -> Vec<&Edge> {
        match self.outgoing.get(source) {
            Some(out) => out
                .iter()
                .map(|i| self.edges.get(*i).unwrap())
                .filter(|e| e.capacity != U256::from(0))
                .collect(),
            None => vec![],
        }
    }

    pub fn incoming(&self, to: &Address) -> Vec<&Edge> {
        match self.incoming.get(to) {
            Some(incoming) => incoming
                .iter()
                .map(|i| self.edges.get(*i).unwrap())
                .filter(|e| e.capacity != U256::from(0))
                .collect(),
            None => vec![],
        }
    }

    fn index_of(&self, e: &Edge) -> Option<usize> {
        self.outgoing.get(&e.from).and_then(|out| {
            for i in out {
                if eq_up_to_capacity(&self.edges[*i], e) {
                    return Some(*i);
                }
            }
            None
        })
    }
}

fn outgoing_index(edges: &[Edge]) -> HashMap<Address, Vec<usize>> {
    let mut index: HashMap<Address, Vec<usize>> = HashMap::new();
    for (i, e) in edges.iter().enumerate() {
        index.entry(e.from).or_default().push(i)
    }
    index
}

fn incoming_index(edges: &[Edge]) -> HashMap<Address, Vec<usize>> {
    let mut index: HashMap<Address, Vec<usize>> = HashMap::new();
    for (i, e) in edges.iter().enumerate() {
        index.entry(e.to).or_default().push(i)
    }
    index
}
