use crate::types::Address;
use std::fmt::{Display, Formatter};

mod adjacencies;
mod flow;

// An edge from the capacity network is
// from, token, to -> capacity
//
// In the transformation into the flow network, we add two intermediate nodes
// per edge that are potentially shared with other edges:
//
// from -A-> BalanceNode(from, token) -B-> TrustNode(to, token) -C-> to
//
// The capacities (A, B, C) are as follows:
// A: the max of all capacity-netwok edges of the form (from, token, *), or A's balance of "token" tokens.
// B: the actual capacity of the capacity-network edge (from, token, to), or the "send limit" from "from" to "to" in "token" tokens
// C: if "token" is C's token (this is a "send to owner" edge): infinity or the sum of all incoming edges.
//    otherwise: the max of all capacity-network edges of the form (*, token, to) or the trust limit of "to" for "token" tokens.

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Node {
    Node(Address),
    BalanceNode(Address, Address),
    TrustNode(Address, Address),
}

pub fn node_as_address(node: &Node) -> &Address {
    if let Node::Node(address) = node {
        address
    } else {
        panic!()
    }
}

pub fn as_trust_node(node: &Node) -> (&Address, &Address) {
    if let Node::TrustNode(to, token) = node {
        (to, token)
    } else {
        panic!()
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Node::Node(address) => write!(f, "{address}"),
            Node::BalanceNode(from, token) => write!(f, "(bal {from} x {token})"),
            Node::TrustNode(to, token) => write!(f, "(trust {to} x {token})"),
        }
    }
}

pub use crate::graph::flow::compute_flow;
pub use crate::graph::flow::transfers_to_dot;
