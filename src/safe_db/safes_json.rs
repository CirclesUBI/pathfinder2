use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::read_to_string;

use crate::types::{Address, Safe};

use super::db::DB;

pub fn import_from_safes_json(file: &str) -> DB {
    let contents = read_to_string(file).unwrap();
    let db: Safes = serde_json::from_str(&contents).unwrap();

    let mut safes: BTreeMap<Address, Safe> = Default::default();
    let mut token_owner: BTreeMap<Address, Address> = Default::default();

    for json_safe in &db.safes {
        let address: Address = json_safe.id.into();
        let mut s = Safe {
            organization: json_safe.organization,
            ..Default::default()
        };
        for balance in &json_safe.balances {
            let token_address: Address = balance.token.id.into();
            let owner: Address = balance.token.owner.id.into();
            s.balances.insert(token_address, balance.amount.into());
            if owner == address {
                s.token_address = token_address;
            }
            token_owner.insert(token_address, owner);
        }
        safes.insert(address, s);
    }

    for json_safe in db.safes {
        for connection in json_safe.outgoing.iter().chain(json_safe.incoming.iter()) {
            let send_to: Address = connection.can_send_to_address.into();
            let user: Address = connection.user_address.into();
            let limit_percentage: u8 = connection.limit_percentage.parse().unwrap();
            assert!(limit_percentage <= 100);
            if send_to != Address::default()
                && user != Address::default()
                && send_to != user
                && limit_percentage > 0
            {
                safes
                    .get_mut(&user)
                    .unwrap()
                    .limit_percentage
                    .insert(send_to, limit_percentage);
            }
        }
    }
    return DB::new(safes, token_owner, None);
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct Safes<'a> {
    #[allow(dead_code)]
    block_number: &'a str,
    safes: Vec<JsonSafe<'a>>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct JsonSafe<'a> {
    id: &'a str,
    organization: bool,
    outgoing: Vec<Edge<'a>>,
    incoming: Vec<Edge<'a>>,
    balances: Vec<Balance<'a>>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct Edge<'a> {
    #[allow(dead_code)]
    limit: Option<&'a str>,
    limit_percentage: &'a str,
    can_send_to_address: &'a str,
    user_address: &'a str,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct Balance<'a> {
    amount: &'a str,
    token: Token<'a>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct Token<'a> {
    id: &'a str,
    owner: Owner<'a>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct Owner<'a> {
    id: &'a str,
}
