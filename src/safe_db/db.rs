use std::collections::BTreeMap;

use crate::types::{Address, Safe};

#[derive(Default, Debug)]
pub struct DB {
    safes: BTreeMap<Address, Safe>,
    token_owner: BTreeMap<Address, Address>,
}

impl DB {
    pub fn new(safes: BTreeMap<Address, Safe>, token_owner: BTreeMap<Address, Address>) -> DB {
        println!("{} safes, {} tokens", safes.len(), token_owner.len());
        DB { safes, token_owner }
    }
}
