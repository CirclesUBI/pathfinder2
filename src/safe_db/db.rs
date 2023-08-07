use std::cmp::min;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::types::{edge::EdgeDB, Address, Edge, Safe, U256};

#[derive(Debug)]
pub struct DB {
    safes: BTreeMap<Address, Safe>,
    token_owner: BTreeMap<Address, Address>,
    edges: EdgeDB,
}

impl DB {
    pub fn new(safes: BTreeMap<Address, Safe>, token_owner: BTreeMap<Address, Address>) -> DB {
        let mut db = DB {
            safes,
            token_owner,
            edges: EdgeDB::new(vec![]),
        };
        db.compute_edges();
        db
    }

    pub fn safes(&self) -> &BTreeMap<Address, Safe> {
        &self.safes
    }

    pub fn edges(&self) -> &EdgeDB {
        &self.edges
    }

    fn compute_edges(&mut self) {
        let mut edges = vec![];

        // token address -> orga addresses
        let mut organization_accepted_tokens: HashMap<Address, HashSet<Address>> = HashMap::new();

        // Build a map from token address to orga addresses that accept this token
        for (_, safe) in &self.safes {
            for (send_to, percentage) in &safe.limit_percentage {
                if percentage == &0 {
                    continue;
                }

                let receiver_safe = self.safes.get(send_to).unwrap();
                if receiver_safe.organization {
                    //println!("user {} can send {} token to orga {}", user, safe.token_address, send_to);
                    organization_accepted_tokens
                        .entry(safe.token_address)
                        .or_default()
                        .insert(*send_to);
                }
            }
        }

        // Find all safes that have a non-zero balance of tokens that are accepted by an organization
        for (user, safe) in &self.safes {
            for (token, balance) in &safe.balances {
                if balance == &U256::from(0) {
                    continue;
                }
                organization_accepted_tokens
                    .get(token)
                    .map(|organizations| {
                        for organization in organizations {
                            // Add the balance as capacity from 'user' to 'organization'
                            edges.push(Edge {
                                from: *user,
                                to: *organization,
                                token: *token,
                                capacity: *balance,
                            });
                        }
                    });
            }
        }

        for (user, safe) in &self.safes {
            // trust connections
            for (send_to, percentage) in &safe.limit_percentage {
                if *user == *send_to {
                    continue;
                }

                if let Some(receiver_safe) = self.safes.get(send_to) {
                    let limit = self.trust_transfer_limit(safe, receiver_safe, *percentage);
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
                            capacity: *balance,
                        })
                    }
                }
            }
        }
        self.edges = EdgeDB::new(edges)
    }

    /// This method calculates how much of their own tokens a user can send to a given receiver.
    /// The transfer limit is based on the trust relationship between the user and the receiver,
    /// denoted by the trust_percentage argument.
    /// If the receiver is an organization, the method simply returns the balance of the sender, i.e.,
    /// the user can send all their tokens to an organization.
    /// If the receiver is a regular user, the method calculates the transfer limit based on the balance
    /// of the receiver and the trust percentage. The method scales down the receiver's balance based
    /// on the trust percentage and compares it with the original balance to calculate the maximum
    /// amount the user can send.
    /// The maximum amount a user can send is the smaller of the user's balance and the difference between
    /// the scaled receiver's balance and the balance calculated based on the trust percentage.
    /// @returns how much of their own tokens a user can send to receiver.
    fn trust_transfer_limit(&self, sender: &Safe, receiver: &Safe, trust_percentage: u8) -> U256 {
        if receiver.organization {
            // TODO treat this as "return to owner"
            // i.e. limited / only constrained by the balance edge.
            sender.balance(&sender.token_address)
        } else {
            let receiver_balance = receiver.balance(&sender.token_address);

            let amount = (receiver.balance(&receiver.token_address)
                * U256::from(trust_percentage as u128))
                / U256::from(100);

            let scaled_receiver_balance =
                receiver_balance * U256::from((100 - trust_percentage) as u128) / U256::from(100);

            if amount < receiver_balance {
                U256::from(0)
            } else {
                // TODO it should not be "min" - the second constraint is set by the balance edge.
                min(
                    amount - scaled_receiver_balance,
                    sender.balance(&sender.token_address),
                )
            }
        }
    }
}
