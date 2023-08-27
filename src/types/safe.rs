use std::{cmp::min, collections::BTreeMap};

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
    /// @returns how much of a token a user can send to receiver. Assuming receiver is an individual account.
    pub fn trust_transfer_limit(&self, receiver: &Safe, trust_percentage: u8, token: &Address) -> U256 {

        let receiver_balance = receiver.balance(token);

        let amount = (receiver.balance(&receiver.token_address)
            * U256::from(trust_percentage as u128))
            / U256::from(100);
        let scaled_receiver_balance =
            receiver_balance * U256::from((100 - trust_percentage) as u128) / U256::from(100);
        if amount < receiver_balance {
            U256::from(0)
        } else {
            // TODO it should not be "min" - the second constraint
            // is set by the balance edge.
            min(
                amount - scaled_receiver_balance,
                self.balance(token),
            )
        }
    }
}
