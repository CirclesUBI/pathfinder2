use std::collections::BTreeMap;

use super::{Address, U256};

#[derive(Default, Debug)]
pub struct Safe {
    pub token_address: Address,
    pub balances: BTreeMap<Address, U256>,
    /// Limit percentage in "send to" direction
    pub limit_percentage: BTreeMap<Address, u8>,
    pub organization: bool,
}
