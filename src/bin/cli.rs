use std::env;
use std::fs::File;
use std::io::Write;

use pathfinder2::graph;
use pathfinder2::io;
use pathfinder2::rpc::call_context::CallContext;
use pathfinder2::types::Address;
use pathfinder2::types::U256;

#[allow(dead_code)]
const HUB_ADDRESS: &str = "0x29b9a7fBb8995b2423a71cC17cf9810798F6C543";
#[allow(dead_code)]
const TRANSFER_THROUGH_SIG: &str = "transferThrough(address[],address[],address[],uint256[])";
#[allow(dead_code)]
const RPC_URL: &str = "https://rpc.gnosischain.com";

fn main() {
    let (dotfile, mut args) =
        if env::args().len() >= 2 && env::args().nth_back(1).unwrap() == "--dot" {
            (
                Some(env::args().last().unwrap()),
                env::args().rev().skip(2).rev().collect::<Vec<_>>(),
            )
        } else {
            (None, env::args().collect::<Vec<_>>())
        };
    let csv = if args.get(1) == Some(&"--csv".to_string()) {
        args = [vec![args[0].clone()], args[2..].to_vec()].concat();
        true
    } else {
        false
    };
    let safes = if args.get(1) == Some(&"--safes".to_string()) {
        args = [vec![args[0].clone()], args[2..].to_vec()].concat();
        true
    } else {
        false
    };
    if safes && csv {
        println!("Options --safes and --csv cannot be used together.");
        return;
    }

    if args.len() < 4 {
        println!("Usage: cli [--csv] [--safes] <from> <to> <edges.dat> [--dot <dotfile>]");
        println!(
            "Usage: cli [--csv] [--safes] <from> <to> <edges.dat> <max_hops>  [--dot <dotfile>]"
        );
        println!(
            "Usage: cli [--csv] [--safes] <from> <to> <edges.dat> <max_hops> <max_flow> [--dot <dotfile>]"
        );
        println!(
            "Usage: cli [--csv] [--safes] <from> <to> <edges.dat> <max_hops> <max_flow> <max_transfers> [--dot <dotfile>]"
        );
        println!("Option --csv reads edges.dat in csv format instead of binary.");
        println!("Option --safes reads a safes.dat file instead of an edges.dat file.");
        return;
    }
    let mut max_hops = None;
    let mut max_flow = U256::MAX;
    let mut max_transfers: Option<u64> = None;
    let (from_str, to_str, edges_file) = (&args[1], &args[2], &args[3]);
    if args.len() >= 5 {
        max_hops = Some(
            args[4]
                .parse()
                .unwrap_or_else(|_| panic!("Expected number of hops, but got: {}", args[4])),
        );
        if args.len() >= 6 {
            max_flow = args[5].as_str().into();
            if args.len() >= 7 {
                max_transfers = Some(args[6].as_str().parse::<i64>().unwrap() as u64);
            }
        }
    }

    println!("Computing flow {from_str} -> {to_str} using {edges_file}");
    let edges = (if csv {
        io::read_edges_csv(edges_file)
    } else if safes {
        io::import_from_safes_binary(edges_file, &CallContext::default()).map(|db| db.edges().clone())
    } else {
        io::read_edges_binary(edges_file)
    })
    .unwrap_or_else(|_| panic!("Error loading edges/safes from file \"{edges_file}\"."));
    println!("Read {} edges", edges.edge_count());
    let (flow, transfers) = graph::compute_flow(
        &Address::from(from_str.as_str()),
        &Address::from(to_str.as_str()),
        &edges,
        max_flow,
        max_hops,
        max_transfers,
        &CallContext::default()
    );
    println!("Found flow: {}", flow.to_decimal());
    //println!("{:?}", transfers);

    let result = json::object! {
        maxFlowValue: flow.to_decimal(),
        transferSteps: transfers.iter().enumerate().map(|(i, e)| {
            json::object!{
                from: e.from.to_checksummed_hex(),
                to: e.to.to_checksummed_hex(),
                token: e.token.to_checksummed_hex(),
                value: e.capacity.to_decimal(),
                step: i,
            }
        }).collect::<Vec<_>>()
    };
    println!("{result}");

    // let token_owners = transfers
    //     .iter()
    //     .map(|e| e.token.to_string())
    //     .collect::<Vec<String>>()
    //     .join(",");
    // let froms = transfers
    //     .iter()
    //     .map(|e| e.from.to_string())
    //     .collect::<Vec<String>>()
    //     .join(",");
    // let tos = transfers
    //     .iter()
    //     .map(|e| e.to.to_string())
    //     .collect::<Vec<String>>()
    //     .join(",");
    // let amounts = transfers
    //     .iter()
    //     .map(|e| e.capacity.to_decimal())
    //     .collect::<Vec<String>>()
    //     .join(",");

    //println!("To check, run the following command (requires foundry):");
    //println!("cast call '{HUB_ADDRESS}' '{TRANSFER_THROUGH_SIG}' '[{token_owners}]' '[{froms}]' '[{tos}]' '[{amounts}]' --rpc-url {RPC_URL} --from {}", &transfers[0].from.to_string());

    if let Some(dotfile) = dotfile {
        File::create(&dotfile)
            .unwrap()
            .write_all(graph::transfers_to_dot(&transfers).as_bytes())
            .unwrap();
        println!("Wrote dotfile {dotfile}.");
    }
}
