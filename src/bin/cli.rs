use std::fs::File;
use std::io::Write;

use pathfinder2::graph;
use pathfinder2::io;
use pathfinder2::types::Address;
use pathfinder2::types::U256;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about = "Compute the transitive transfers from one source to one destination", long_about = None)]
struct Cli {
    /// Source address
    from: String,

    /// Destination address
    to: String,

    /// Edges file to use to compute_flow
    edges_file: String, // maybe PathBuff

    /// Number of hops to explore
    max_hops: Option<u64>,

    /// Maximum amount of circles to transfer
    max_transfers: Option<u64>,

    /// Maximum flow to compute
    max_flow: Option<U256>,

    /// Reads edges.dat in csv format instead of binary.
    #[arg(short, long)]
    csv: bool,

    /// Reads a safes.dat file instead of an edges.dat file.
    #[arg(short, long)]
    safes: bool,

    /// <dotfile> a graphviz/dot representation of the transfer graph is written to the given file
    #[arg(short, long)]
    dotfile: Option<String>,

    /// Format the result before printing it
    #[arg(short, long)]
    pretty: bool,
}

#[allow(dead_code)]
const HUB_ADDRESS: &str = "0x29b9a7fBb8995b2423a71cC17cf9810798F6C543";
#[allow(dead_code)]
const TRANSFER_THROUGH_SIG: &str = "transferThrough(address[],address[],address[],uint256[])";
#[allow(dead_code)]
const RPC_URL: &str = "https://rpc.gnosischain.com";

fn main() {
    let cli = Cli::parse();

    // safes and csv are exclusive
    if cli.csv && cli.safes {
        println!("Options --safes and --csv cannot be used together.");
        return;
    }

    let Cli {
        from: from_str,
        to: to_str,
        edges_file,
        max_hops,
        max_transfers,
        mut max_flow,
        ..
    } = cli;

    if max_flow.is_none() {
        max_flow = Some(U256::MAX);
    }

    println!("Computing flow {from_str} -> {to_str} using {edges_file}");

    let edges = if cli.csv {
        io::read_edges_csv(&edges_file)
    } else if cli.safes {
        io::import_from_safes_binary(&edges_file).map(|db| db.edges().clone())
    } else {
        io::read_edges_binary(&edges_file)
    }
    .unwrap_or_else(|_| panic!("Error loading edges/safes from file \"{edges_file}\"."));

    println!("Read {} edges", edges.edge_count());
    let (flow, transfers) = graph::compute_flow(
        &Address::from(from_str.as_str()),
        &Address::from(to_str.as_str()),
        &edges,
        max_flow.unwrap(),
        max_hops,
        max_transfers,
    );
    println!("Found flow: {}", flow.to_decimal());

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

    let result = if cli.pretty {
        json::stringify_pretty(result, 2)
    } else {
        json::stringify(result)
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

    if let Some(dotfile) = cli.dotfile {
        File::create(&dotfile)
            .unwrap()
            .write_all(graph::transfers_to_dot(&transfers).as_bytes())
            .unwrap();
        println!("Wrote dotfile {dotfile}.");
    }
}
