use std::env;

use pathfinder2::{
    io::{read_edges_binary, read_edges_csv, write_edges_binary, write_edges_csv},
    safe_db::safes_json::import_from_safes_json,
};

fn main() {
    let operation = env::args().nth(1).and_then(|op| {
        if matches!(
            op.as_str(),
            "--safes-json-to-edges-bin"
                | "--safes-json-to-edges-csv"
                | "--edges-csv-to-edges-bin"
                | "--edges-bin-to-edges-csv"
        ) {
            Some(op)
        } else {
            None
        }
    });
    if env::args().len() != 4 || operation.is_none() {
        println!("Usage: convert --safes-json-to-edges-bin <safes.json> <edges.dat>");
        println!("Usage: convert --safes-json-to-edges-csv <safes.json> <edges.csv>");
        println!("Usage: convert --edges-csv-to-edges-bin <edges.csv> <edges.dat>");
        println!("Usage: convert --edges-bin-to-edges-csv <edges.dat> <edges.csv>");
        return;
    }

    let input = env::args().nth(2).unwrap();
    let output = env::args().nth(3).unwrap();
    match operation.unwrap().as_str() {
        "--safes-json-to-edges-bin" => {
            let safes = import_from_safes_json(&input);
            let edges = safes.edges();
            println!("Imported {} edges.", edges.edge_count());
            write_edges_binary(edges, &output).unwrap();
            println!("Export done.");
        }
        "--safes-json-to-edges-csv" => {
            let safes = import_from_safes_json(&input);
            let edges = safes.edges();
            println!("Imported {} edges.", edges.edge_count());
            write_edges_csv(edges, &output).unwrap();
            println!("Export done.");
        }
        "--edges-csv-to-edges-bin" => {
            let edges = &read_edges_csv(&input).unwrap();
            println!("Imported {} edges.", edges.edge_count());
            write_edges_binary(edges, &output).unwrap();
            println!("Export done.");
        }
        "--edges-bin-to-edges-csv" => {
            let edges = &read_edges_binary(&input).unwrap();
            println!("Imported {} edges.", edges.edge_count());
            write_edges_csv(edges, &output).unwrap();
            println!("Export done.");
        }
        _ => unreachable!(),
    }
}
