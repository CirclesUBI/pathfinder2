use std::collections::{BTreeMap, HashMap, HashSet};

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
        for safe in self.safes.values() {
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
                if let Some(organizations) = organization_accepted_tokens.get(token) {
                    for organization in organizations {
                        // Add the balance as capacity from 'user' to 'organization'
                        edges.push(Edge {
                            from: *user,
                            to: *organization,
                            token: *token,
                            capacity: *balance,
                        });
                    }
                };
            }
        }

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
