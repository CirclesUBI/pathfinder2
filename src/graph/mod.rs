use crate::types::Address;
use std::fmt::{Display, Formatter};

mod adjacencies;
mod flow;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Node {
    Node(Address),
    TokenEdge(Address, Address),
}

pub fn node_as_address(node: &Node) -> &Address {
    if let Node::Node(address) = node {
        address
    } else {
        panic!()
    }
}

pub fn node_as_token_edge(node: &Node) -> (&Address, &Address) {
    if let Node::TokenEdge(from, token) = node {
        (from, token)
    } else {
        panic!()
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Node::Node(address) => write!(f, "{address}"),
            Node::TokenEdge(address, token) => write!(f, "({address} x {token})"),
        }
    }
}

pub use crate::graph::flow::compute_flow;
pub use crate::graph::flow::transfers_to_dot;
