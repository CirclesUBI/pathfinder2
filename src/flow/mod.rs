use crate::types::{Address};

mod adjacencies;
mod flow;

#[derive(Eq, PartialEq, Hash, Clone)]
enum Node {
    Node(Address),
    TokenEdge(Address, Address),
}

pub use crate::flow::flow::compute_flow;
