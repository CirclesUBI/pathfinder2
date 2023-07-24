use super::Address;

pub struct Token {
    #[allow(dead_code)]
    address: Address,
    #[allow(dead_code)]
    owner: Address,
}

impl Token {
    /// @returns how much of a token a user can send to receiver.
    pub fn trust_transfer_limit(&self, receiver: &Safe, trust_percentage: u8) -> U256 {
        if receiver.organization {
            // TODO treat this as "return to owner"
            // i.e. limited / only constrained by the balance edge.
            self.balance(&self.token_address)
        } else {
            let receiver_balance = receiver.balance(&self.address);

            let amount = (receiver.balance(&receiver.token_address)
                * U256::from(trust_percentage as u128))
                / U256::from(100);

            if amount < receiver_balance {
                U256::from(0)
            } else {
                let scaled_receiver_balance =
                    receiver_balance * U256::from((100 - trust_percentage) as u128) / U256::from(100);
                amount - scaled_receiver_balance,
            }
        }
    }
}
