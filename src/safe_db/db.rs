use std::collections::BTreeMap;

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
        // Universal computation of edges
        // Let's assume that any "token" is represented by the address of its owner in the edges
        // We also assume that the "send_to" relationship is the opposite to the trust relationship
        // List of edges
        let mut edges = vec![];
        // Create the edges from the token holders to anyone who trusts that token
        for (user, safe) in &self.safes {
            for (token, balance) in &safe.balances {
                if let Some(owner) = self.token_owner.get(token) {
                    if *balance != U256::from(0) {
                        if let Some(owner_safe) = self.safes.get(owner) {
                            // "limit_percentage" represents the list of users that accept the "owner_safe"'s token
                            for (send_to, percentage) in &owner_safe.limit_percentage {
                                if percentage == &0 || *user == *send_to {
                                    continue;
                                }
                                if let Some(receiver_safe) = self.safes.get(send_to) {
                                    // If the receiver is an organization, the edge's limit is the balance of the
                                    // sender, i.e., the user can send all their tokens to an organization.
                                    // Likewise, if the receiver is the owner of the token, the edge's limit is
                                    // the sender's balance of that token.
                                    let limit: U256 = if receiver_safe.organization
                                        || *owner == *send_to
                                    {
                                        *balance
                                    } else {
                                        // TODO it should not be "min" - the second constraint
                                        // is set by the balance edge.
                                        safe.trust_transfer_limit(receiver_safe, *percentage, token)
                                    };
                                    if limit != U256::from(0) {
                                        edges.push(Edge {
                                            from: *user,
                                            to: *send_to,
                                            token: *owner,
                                            capacity: limit,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self.edges = EdgeDB::new(edges)
    }
}
