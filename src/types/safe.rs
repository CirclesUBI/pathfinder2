use std::{cmp::min, collections::BTreeMap};
use std::ops::Sub;

use super::{Address, U256};

#[derive(Default, Debug)]
pub struct Safe {
    /// The address of the token, or the address of the safe if
    /// the database does not use the distinction.
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
        if receiver.organization || receiver.token_address == self.token_address {
            return self.balance(&self.token_address);
        } else {
            let receiver_balance = receiver.balance(&self.token_address);
            let one_hundred = U256::from(100_u128);

            let max = (receiver.balance(&receiver.token_address)
                * U256::from(trust_percentage as u128))
                / one_hundred;

            if max < receiver_balance {
                return U256::from(0);
            }

            let receiver_balance_scaled = receiver_balance * (one_hundred - U256::from(trust_percentage as u128)) / one_hundred;

            return max.sub(receiver_balance_scaled);
        }
    }
}
