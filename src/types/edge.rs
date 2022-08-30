use crate::types::Address;
use crate::types::U256;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, PartialOrd)]
pub struct Edge {
    pub from: Address,
    pub to: Address,
    pub token: Address,
    pub capacity: U256,
}

// TODO comparison, hash, etc. can ignore the capacity field.

// TODO can we derive it?
impl Eq for Edge {}
