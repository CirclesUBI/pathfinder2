use std::{cmp::min, collections::BTreeMap};

use super::{Address, U256};

#[derive(Default, Debug)]
pub struct Safe {
    pub token_address: Address,
    pub balances: BTreeMap<Address, U256>,
    /// Limit percentage in "send to" direction
    pub limit_percentage: BTreeMap<Address, u8>,
    pub organization: bool,
}

impl Safe {
    pub fn balance(&self, token: &Address) -> U256 {
        *self.balances.get(token).unwrap_or(&U256::from(0))
    }
    /// @returns how much of their own tokens a user can send to receiver.
    pub fn trust_transfer_limit(&self, receiver: &Safe, trust_percentage: u8) -> U256 {
        if receiver.organization {
            // TODO treat this as "return to owner"
            self.balance(&self.token_address)
        } else {
            let receiver_balance = receiver.balance(&self.token_address);
            let amount = (receiver.balance(&receiver.token_address)
                * U256::from(trust_percentage as u128))
                / U256::from(100);
            if amount < receiver_balance {
                U256::from(0)
            } else {
                min(amount - receiver_balance, self.balance(&self.token_address))
            }
        }
    }
}
