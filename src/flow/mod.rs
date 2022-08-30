use crate::address::Address;
use crate::edge::Edge;
use crate::u256::U256;

enum Node {
    Node(Address),
    TokenEdge(Address, Address),
}

fn pseudoNode(edge: Edge) -> Node {
    Node::TokenEdge(edge.from, edge.to)
}