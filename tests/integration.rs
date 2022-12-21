use pathfinder2::graph::compute_flow;
use pathfinder2::io::{import_from_safes_binary, read_edges_binary};
use pathfinder2::types::edge::EdgeDB;
use pathfinder2::types::{Address, U256};
use std::fmt::Arguments;
use std::process::Command;

const HUB_ADDRESS: &str = "0x29b9a7fBb8995b2423a71cC17cf9810798F6C543";
const TRANSFER_THROUGH_SIG: &str = "transferThrough(address[],address[],address[],uint256[])";
const RPC_URL: &str = "https://rpc.eu-central-2.gateway.fm/v3/gnosis/archival/mainnet?apiKey=uoxaT2YDCCz_aITXD3mwAfjbOwYd2k88.La01lOweqqP4X6zx";

const USER_TO_TOKEN: &str = "userToToken(address)";
const TEST_BLOCK: u64 = 25476000;

// fn call_hub(signature: &str, arguments: Vec<String>, block: u64, from: Option<&Address>) -> String {
//     let output = Command::new("cast")
//         .args(
//             [
//                 vec!["call", HUB_ADDRESS, signature],
//                 arguments.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
//                 vec!["--rpc-url", RPC_URL],
//                 if let Some(from) = from {
//                     vec!["--from", from.to_string().as_str()]
//                 } else {
//                     vec![]
//                 },
//                 vec!["--block", block.to_string().as_str()],
//             ]
//             .concat(),
//         )
//         .output()
//         .expect("Error calling cast.");
//     let stdout = String::from_utf8(output.stdout).unwrap().trim().to_string();
//     let stderr = String::from_utf8(output.stderr).unwrap().trim().to_string();
//     println!("Transfer: {stdout} {stderr}",);
//     assert!(stderr.is_empty());
//     stdout
// }

// fn user_to_token(user: &Address) -> Address {
//     call_hub(USER_TO_TOKEN, user.to_string(), TEST_BLOCK, None);
// }

#[test]
fn debug() {
    let martinsavings = Address::from("0x052b4793d50d37FD3BFcBf93AAC9Cda6292F81Fa");
    let martin = Address::from("0x42cEDde51198D1773590311E2A340DC06B24cB37");
    let vbuterin = Address::from("0xCAABD9353b7E8e09dE8e2cBC02aa5A6C3807e70d");
    let circlescoop = Address::from("0x9BA1Bcd88E99d6E1E03252A70A63FEa83Bf1208c");
    let earlyadopter = Address::from("0x939b2731997922f21ab0a0bab500a949c0fc3550");
    let ubipromoter = Address::from("0x5D976cE82B8851e9d4841a41f97D0Fe42c628617");
    let ernst = Address::from("0x57928Fb15ffB7303b65EDC326dc4dc38150008e1");
    let ajmaq = Address::from("0xeb9784F6A6e3d03466974Cb3a5a77c79afbA14e7");
    let stefan = Address::from("0x11fC3Cb5818C6703d9b49a20285178273FEdca49");
    let sarah = Address::from("0x55E0fF8d8eF8194aBF0F6378076193B4554376C6");
    let ber = Address::from("0x05698e7346ea67cfb088f64ad8962b18137d17c0");
    let kaustubh = Address::from("0x02B50e87C577084b9659a625870b4A6e8a8E9238");
    let daniel = Address::from("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let all = vec![
        martinsavings,
        martin,
        vbuterin,
        circlescoop,
        earlyadopter,
        ubipromoter,
        ernst,
        ajmaq,
        stefan,
        sarah,
        ber,
        kaustubh,
    ];

    let db = import_from_safes_binary("safes.dat").unwrap();

    println!("DEBUC");
    let limit = db
        .safes
        .get(&Address::from("0x9a0bbbbd3789f184ca88f2f6a40f42406cb842ac"))
        .unwrap()
        .trust_transfer_limit(
            db.safes
                .get(&Address::from("0x3cb406def33aed0abd6d02a75fedca8e2e8d1a2e"))
                .unwrap(),
            50,
        );
    println!("transfer limit: {limit}");
    //     token: 0x9a0bbbbd3789f184ca88f2f6a40f42406cb842ac
    // src: 0x9a0bbbbd3789f184ca88f2f6a40f42406cb842ac
    // dest: 0x3cb406def33aed0abd6d02a75fedca8e2e8d1a2e
    // wad: 56215193215459277770
    // println!(
    //     "BALANCE mantin: {:?}",
    //     db.safes.get(&martin).unwrap().balance(&martin)
    // );
    // println!(
    //     "safe martin: {:?}",
    //     db.safes.get(&martin).unwrap() //.balance(&martin)
    // );

    for from in &all {
        for to in &all {
            let value = U256::MAX; //U256::from("100000000000000000");
            let hops = Some(6);
            println!("FroM: {from} to: {to}");
            test_flow(from, to, db.edges(), value, hops);
        }
    }

    // let from = daniel; //Address::from("0x9a0bbbbd3789f184ca88f2f6a40f42406cb842ac"); //ubipromoter; //martin;
    // let to = martin; //Address::from("0x3cb406def33aed0abd6d02a75fedca8e2e8d1a2e");
}

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

fn read_db() -> EdgeDB {
    import_from_safes_binary("safes.dat")
        .unwrap()
        .edges()
        .clone()
}

fn test_flow(
    source: &Address,
    sink: &Address,
    edges: &EdgeDB,
    requested_flow: U256,
    max_distance: Option<u64>,
) {
    let transfers = compute_flow(source, sink, edges, requested_flow, max_distance);
    println!("{transfers:?}");
    if transfers.0 == U256::from(0) {
        return;
    }

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
    // let output = call_hub(
    //     TRANSFER_THROUGH_SIG,
    //     &Address::from(HUB_ADDRESS),
    //     vec![
    //         format!("[{token_owners}]"),
    //         format!("[{froms}]"),
    //         format!("[{tos}]"),
    //         format!("[{amounts}]"),
    //     ],
    //     25476000,
    // );

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
            "--block",
            "25476000",
        ])
        .output()
        .expect("Error calling cast.");
    let stdout = String::from_utf8(output.stdout).unwrap().trim().to_string();
    let stderr = String::from_utf8(output.stderr).unwrap().trim().to_string();
    println!("Transfer: {stdout} {stderr}",);
    assert_eq!(stdout, "0x".to_string());
    assert!(stderr.is_empty());
    println!(
        "Successful transfer of {}",
        transfers.0.to_decimal_fraction()
    );
}
