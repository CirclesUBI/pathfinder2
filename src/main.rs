use std::env;

mod flow;
mod io;
mod server;
mod types;

use server::Server;

fn main() {
    let port = if env::args().len() == 1 {
        8080
    } else {
        env::args().nth(1).unwrap().as_str().parse::<u16>().unwrap()
    };
    Server::start(port);

    // let args: Vec<String> = env::args().collect();
    // if args.len() != 4 {
    //     panic!("Expected three arguments");
    // }
    // let (from_str, to_str, edges_file) = (&args[1], &args[2], &args[3]);

    // println!("Computing flow {from_str} -> {to_str} using {edges_file}");
    // let edges = io::read_edges_binary(edges_file).expect("Error loading edges.");
    // println!("Read {} edges", edges.len());
    // flow::compute_flow(
    //     &Address::from(from_str.as_str()),
    //     &Address::from(to_str.as_str()),
    //     &edges,
    // );
}
