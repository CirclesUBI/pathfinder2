use std::{collections::BTreeMap};

use super::{Address, U256};

#[derive(Default, Debug)]
pub struct Safe {
    /// The address of the token, or the address of the safe if
    /// the database does not use the distinction.
    pub token_address: Address,
    pub balances: BTreeMap<Address, U256>,
    /// Limit percentage in "send to" direction
    pub limit_percentage: BTreeMap<Address, u8>,
    /// Limit percentage in "user" direction
    pub limit_percentage_in: BTreeMap<Address, u8>,
    pub organization: bool,
}

impl Safe {
    pub fn balance(&self, token: &Address) -> U256 {
        *self.balances.get(token).unwrap_or(&U256::from(0))
    }
}
