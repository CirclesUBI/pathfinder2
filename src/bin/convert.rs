use std::env;

use pathfinder2::io::*;
use pathfinder2::rpc::call_context::CallContext;
use pathfinder2::safe_db::safes_json::import_from_safes_json;

fn main() {
    let input_format = env::args().nth(1).and_then(|op| {
        if matches!(
            op.as_str(),
            "--safes-json" | "--safes-bin" | "--edges-csv" | "--edges-bin"
        ) {
            Some(op)
        } else {
            None
        }
    });
    let output_format = env::args().nth(3).and_then(|op| {
        if matches!(op.as_str(), "--edges-csv" | "--edges-bin") {
            Some(op)
        } else {
            None
        }
    });
    if env::args().len() != 5 || input_format.is_none() || output_format.is_none() {
        println!("Usage: convert <input> <input_file> <output> <output_file>");
        println!("  Where <input> is one of:");
        println!("    --safes-json");
        println!("    --safes-bin");
        println!("    --edges-csv");
        println!("    --edges-bin");
        println!("  and <output>is one of:");
        println!("    --edges-csv");
        println!("    --edges-bin");
        return;
    }

    let input_file = env::args().nth(2).unwrap();
    let edges = match input_format.unwrap().as_str() {
        "--safes-json" => {
            let safes = import_from_safes_json(&input_file);
            safes.edges().clone()
        }
        "--safes-bin" => {
            let safes = import_from_safes_binary(&input_file, &CallContext::default()).unwrap();
            safes.edges().clone()
        }
        "--edges-csv" => read_edges_csv(&input_file).unwrap(),
        "--edges-bin" => read_edges_binary(&input_file).unwrap(),
        _ => unreachable!(),
    };
    println!("Imported {} edges.", edges.edge_count());

    let output_file = env::args().nth(4).unwrap();
    match output_format.unwrap().as_str() {
        "--edges-csv" => write_edges_csv(&edges, &output_file).unwrap(),
        "--edges-bin" => write_edges_binary(&edges, &output_file).unwrap(),
        _ => unreachable!(),
    }
    println!("Export done.");
}
