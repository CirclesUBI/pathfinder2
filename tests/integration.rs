use pathfinder2::graph::compute_flow;
use pathfinder2::io::read_edges_binary;
use pathfinder2::types::edge::EdgeDB;
use pathfinder2::types::{Address, U256};
use std::process::Command;

const HUB_ADDRESS: &str = "0x29b9a7fBb8995b2423a71cC17cf9810798F6C543";
const TRANSFER_THROUGH_SIG: &str = "transferThrough(address[],address[],address[],uint256[])";
const RPC_URL: &str = "https://rpc.gnosischain.com";

#[test]
fn test_flow_chris_martin() {
    let chriseth = Address::from("0x8DC7e86fF693e9032A0F41711b5581a04b26Be2E");
    let martin = Address::from("0x42cEDde51198D1773590311E2A340DC06B24cB37");
    test_flow(&chriseth, &martin, &read_edges(), U256::MAX, None);
    test_flow(&chriseth, &martin, &read_edges(), U256::MAX, Some(2));
    test_flow(
        &chriseth,
        &martin,
        &read_edges(),
        U256::from(71152921504606846976),
        Some(2),
    );
    test_flow(
        &chriseth,
        &martin,
        &read_edges(),
        U256::from(51152921504606846976),
        Some(2),
    );
}

#[test]
fn test_flow_large() {
    let large_source = Address::from("0x9BA1Bcd88E99d6E1E03252A70A63FEa83Bf1208c");
    let large_dest = Address::from("0x939b2731997922f21ab0a0bab500a949c0fc3550");
    test_flow(
        &large_source,
        &large_dest,
        &read_edges(),
        U256::MAX,
        Some(4),
    );
    test_flow(
        &large_source,
        &large_dest,
        &read_edges(),
        U256::MAX,
        Some(6),
    );
}

fn read_edges() -> EdgeDB {
    read_edges_binary(&"edges.dat".to_string()).unwrap()
}

fn test_flow(
    source: &Address,
    sink: &Address,
    edges: &EdgeDB,
    requested_flow: U256,
    max_distance: Option<u64>,
) {
    let transfers = compute_flow(source, sink, edges, requested_flow, max_distance, None);
    println!("{transfers:?}");

    let token_owners = transfers
        .1
        .iter()
        .map(|e| e.token.to_string())
        .collect::<Vec<String>>()
        .join(",");
    let froms = transfers
        .1
        .iter()
        .map(|e| e.from.to_string())
        .collect::<Vec<String>>()
        .join(",");
    let tos = transfers
        .1
        .iter()
        .map(|e| e.to.to_string())
        .collect::<Vec<String>>()
        .join(",");
    let amounts = transfers
        .1
        .iter()
        .map(|e| e.capacity.to_decimal())
        .collect::<Vec<String>>()
        .join(",");
    let output = Command::new("cast")
        .args([
            "call",
            HUB_ADDRESS,
            TRANSFER_THROUGH_SIG,
            &format!("[{token_owners}]"),
            &format!("[{froms}]"),
            &format!("[{tos}]"),
            &format!("[{amounts}]"),
            "--rpc-url",
            RPC_URL,
            "--from",
            &transfers.1[0].from.to_string(),
        ])
        .output()
        .expect("Error calling cast.");
    let stdout = String::from_utf8(output.stdout).unwrap().trim().to_string();
    let stderr = String::from_utf8(output.stderr).unwrap().trim().to_string();
    println!("Transfer: {stdout} {stderr}",);
    assert_eq!(stdout, "0x".to_string());
    assert!(stderr.is_empty());
}
