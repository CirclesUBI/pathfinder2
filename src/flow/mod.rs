mod adjacencies;

use crate::types::{Address, Edge, U256};

#[derive(Eq, PartialEq, Hash, Clone)]
enum Node {
    Node(Address),
    TokenEdge(Address, Address),
}
