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

        // Universal computation of edges
        // Let's assume that any "token" is represented by the address of its owner
        // We also assume that the "send_to" relationship is the opposite to the trust relationship
        // List of edges
        let mut edges = vec![];
        // token address -> user addresses
        let mut user_accepted_tokens: HashMap<Address, HashSet<Address>> = HashMap::new();
        // Build a map from token address to users that accept that token
        // TODO Can we get this directly from the indexer db?
        for (user, safe) in &self.safes {
            for (send_to, percentage) in &safe.limit_percentage {
                if percentage == &0 {
                    continue;
                }
                //println!("user {} can send token of {} to user {}", user, user, send_to);
                user_accepted_tokens.entry(*user).or_default().insert(*send_to, *percentage);
            }
        }
        // Create the edges from the token holders to anyone who trusts that token
        for (user, safe) in &self.safes {
            for (token, balance) in &safe.balances {
                if let Some(owner) = self.token_owner.get(token) {
                    if *balance != U256::from(0) {
                        user_accepted_tokens.get(token).map(|trusting_users| {
                            for (trusting_user, percentage) in trusting_users {
                                if *user == *trusting_user {
                                    continue;
                                }
                                let limit
                                if trusting_user.organization || *token == *trusting_user {
                                    limit = balance
                                } else {
                                    // TODO it should not be "min" - the second constraint
                                    // is set by the balance edge.
                                    limit = min(
                                        token.trust_transfer_limit(trusting_user, percentage),
                                        balance
                                    );
                                }
                                if limit != U256::from(0) {
                                    edges.push(Edge {
                                        from: *user,
                                        to: *trusting_user,
                                        token: *token,
                                        capacity: limit,
                                    });
                                }
                            }
                        });
                    }
                }
            }
        }
        self.edges = EdgeDB::new(edges)
    }
}
