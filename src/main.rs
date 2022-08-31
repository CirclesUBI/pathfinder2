use std::env;


mod flow;
mod io;
mod types;

use types::Address;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        panic!("Expected three arguments");
    }
    let (from_str, to_str, edges_file) = (&args[1], &args[2], &args[3]);

    println!("Computing flow {from_str} -> {to_str} using {edges_file}");
    let edges = io::read_edges_binary(edges_file).expect("Error loading edges.");
    print!("Read {} edges", edges.len());
    flow::compute_flow(&Address::from(from_str.as_str()), &Address::from(to_str.as_str()), &edges);
}
