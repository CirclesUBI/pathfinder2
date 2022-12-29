use std::collections::BTreeMap;

use crate::types::{edge::EdgeDB, Address, Edge, Safe, U256};

#[derive(Default, Debug)]
pub struct DB {
    safes: BTreeMap<Address, Safe>,
    token_owner: BTreeMap<Address, Address>,
    edges: EdgeDB,
}

impl DB {
    pub fn new(safes: BTreeMap<Address, Safe>, token_owner: BTreeMap<Address, Address>) -> DB {
        println!("{} safes, {} tokens", safes.len(), token_owner.len());
        let mut db = DB {
            safes,
            token_owner,
            ..Default::default()
        };
        db.compute_edges();
        db
    }

    /// Updates the balance of the given user and token.
    /// Does not automatically update the transfer edge set.
    /// @remark Only properly works on a system where token address is the owner's address.
    pub fn update_balance(&mut self, user: Address, token: Address, balance: U256) {
        *self
            .safes
            .entry(user)
            .or_insert(Safe {
                token_address: user,
                organization: false,
                ..Default::default()
            })
            .balances
            .entry(token)
            .or_default() = balance;
    }

    /// Updates the trust percentage of the given user and transaction receiver.
    /// Does not automatically update the transfer edge set.
    /// @remark Only properly works on a system where token address is the owner's address.
    pub fn update_limit_percentage(&mut self, user: Address, can_send_to: Address, percentage: u8) {
        *self
            .safes
            .entry(user)
            .or_insert(Safe {
                token_address: user,
                organization: false,
                ..Default::default()
            })
            .limit_percentage
            .entry(can_send_to)
            .or_default() = percentage;
    }

    pub fn edges(&self) -> &EdgeDB {
        &self.edges
    }

    pub fn compute_edges(&mut self) {
        let mut edges = vec![];
        for (user, safe) in &self.safes {
            // trust connections
            for (send_to, percentage) in &safe.limit_percentage {
                if *user == *send_to {
                    continue;
                }
                if let Some(receiver_safe) = self.safes.get(send_to) {
                    // TODO should return "limited or not"
                    // edge should contain token balance and transfer limit (which can be unlimited)
                    let limit = safe.trust_transfer_limit(receiver_safe, *percentage);
                    if limit != U256::from(0) {
                        edges.push(Edge {
                            from: *user,
                            to: *send_to,
                            token: *user,
                            capacity: limit,
                        })
                    }
                }
            }
            // send tokens back to owner
            for (token, balance) in &safe.balances {
                if let Some(owner) = self.token_owner.get(token) {
                    if *user != *owner && *balance != U256::from(0) {
                        edges.push(Edge {
                            from: *user,
                            to: *owner,
                            token: *owner,
                            // TODO capacity should be only limited by own balance.
                            capacity: *balance,
                        })
                    }
                }
            }
        }
        self.edges = EdgeDB::new(edges)
    }
}
