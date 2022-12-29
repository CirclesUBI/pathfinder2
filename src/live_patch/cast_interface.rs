use std::process::Command;

use crate::types::{Address, U256};

fn circles_hub() -> Address {
    Address::from("0x29b9a7fBb8995b2423a71cC17cf9810798F6C543")
}

//const TRANSFER_THROUGH_SIG: &str = "transferThrough(address[],address[],address[],uint256[])";

fn call_contract(
    address: &Address,
    signature: &str,
    data: &[&str],
    from: Option<Address>,
) -> String {
    const RPC_URL: &str = "https://rpc.gnosischain.com";
    let output = Command::new("cast")
        .args(
            [
                &["call", &address.to_string(), signature],
                data,
                &["--rpc-url", RPC_URL],
                //[["--from"]], //            "--from",
                //&transfers.1[0].from.to_string(),
            ]
            .concat(),
        )
        .output()
        .expect("Error calling cast.");
    let stdout = String::from_utf8(output.stdout).unwrap().trim().to_string();
    let stderr = String::from_utf8(output.stderr).unwrap().trim().to_string();
    assert!(stderr.is_empty(), "Error in call: {stderr}");
    stdout
}

fn decode_address(input: &str) -> Address {
    assert_eq!(input.len(), 32 * 2 + 2);
    assert_eq!(&input[0..26], "0x000000000000000000000000");
    Address::from(&input[26..])
}

pub fn token_of(user: &Address) -> Address {
    let addr_str = call_contract(
        &circles_hub(),
        "userToToken(address)",
        &[&format!("{user}")],
        None,
    );
    decode_address(&addr_str)
}

pub fn balance_of(user: &Address, token_user: &Address) -> U256 {
    let token = token_of(token_user);
    let result_str = call_contract(&token, "balanceOf(address)", &[&format!("{user}")], None);
    U256::from(result_str.as_str())
}

pub fn is_organization(user: &Address) -> bool {
    let result_str = call_contract(
        &circles_hub(),
        "organizations(address)",
        &[&format!("{user}")],
        None,
    );
    U256::from(result_str.as_str()) != U256::from(0)
}

//const LIMIT: &str = "limits(address,address)";
pub fn limit_percentage(user: &Address, can_send_to: &Address) -> U256 {
    let result_str = call_contract(
        &circles_hub(),
        "limits(address,address)",
        &[&user.to_string(), &can_send_to.to_string()],
        None,
    );
    U256::from(result_str.as_str())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_token_of() {
        let martin = Address::from("0x42cEDde51198D1773590311E2A340DC06B24cB37");
        assert_eq!(
            token_of(&martin),
            Address::from("0x6293268785399bed001cb68a8ee04d50da9c854d"),
        );
    }

    #[test]
    fn test_balance_of() {
        let martin = Address::from("0x42cEDde51198D1773590311E2A340DC06B24cB37");
        assert!(balance_of(&martin, &martin) >= U256::from("0xe3d8a7fc2b4d05726"));
    }

    #[test]
    fn test_is_org() {
        let martin = Address::from("0x42cEDde51198D1773590311E2A340DC06B24cB37");
        assert!(!is_organization(&martin));
        let ber = Address::from("0x05698e7346ea67cfb088f64ad8962b18137d17c0");
        assert!(is_organization(&ber));
    }

    #[test]
    fn test_limit() {
        let martin = Address::from("0x42cEDde51198D1773590311E2A340DC06B24cB37");
        let chriseth = Address::from("0x8DC7e86fF693e9032A0F41711b5581a04b26Be2E");

        assert_eq!(limit_percentage(&martin, &chriseth), U256::from(50));
    }
}
