use std::env;

use pathfinder2::server;

fn main() {
    let port = if env::args().len() == 1 {
        8080
    } else {
        env::args().nth(1).unwrap().as_str().parse::<u16>().unwrap()
    };
    server::start_server(port, 10, 4);
}
