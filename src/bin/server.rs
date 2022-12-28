use std::env;

use pathfinder2::server;

fn main() {
    let listen_at = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    server::start_server(&listen_at, 10, 4);
}
