use std::env;

use pathfinder2::server;

fn main() {
    let listen_at = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let queue_size =  env::args()
        .nth(2)
        .unwrap_or_else(|| "10".to_string())
        .parse::<usize>()
        .unwrap();;

    let thread_count =  env::args()
        .nth(3)
        .unwrap_or_else(|| "4".to_string())
        .parse::<u64>()
        .unwrap();;

    server::start_server(&listen_at, queue_size, thread_count);
}
