use std::env;

use pathfinder2::server;

fn main() {
    let listen_at_arg = env::args().nth(1);
    if listen_at_arg.is_some() {
        server::start_server(listen_at_arg.unwrap().as_str(), 10, 4);
    } else {
        server::start_server("127.0.0.1:8080", 10, 4);
    }
}
