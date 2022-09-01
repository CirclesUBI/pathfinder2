use crate::types::Address;
use std::fmt::{Display, Formatter};

mod adjacencies;
mod flow;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Node {
    Node(Address),
    TokenEdge(Address, Address),
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Node::Node(address) => write!(f, "{address}"),
            Node::TokenEdge(address, token) => write!(f, "({address} x {token})"),
        }
    }
}

pub use crate::flow::flow::compute_flow;
