use crate::rpc::rpc_handler::handle_connection;
use std::io::Write;
use std::net::TcpListener;
use std::sync::mpsc::TrySendError;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use crate::safe_db::edge_db_dispenser::EdgeDbDispenser;

fn validate_and_parse_ethereum_address(address: &str) -> Result<Address, Box<dyn Error>> {
    let re = Regex::new(r"^0x[0-9a-fA-F]{40}$").unwrap();
    if re.is_match(address) {
        Ok(Address::from(address))
    } else {
        Err(Box::new(InputValidationError(format!(
            "Invalid Ethereum address: {}",
            address
        ))))
    }
}

fn validate_and_parse_u256(value_str: &str) -> Result<U256, Box<dyn Error>> {
    match BigUint::from_str(value_str) {
        Ok(parsed_value) => {
            if parsed_value > U256::MAX.into() {
                Err(Box::new(InputValidationError(format!(
                    "Value {} is too large. Maximum value is {}.",
                    parsed_value,
                    U256::MAX
                ))))
            } else {
                Ok(U256::from_bigint_truncating(parsed_value))
            }
        }
        Err(e) => Err(Box::new(InputValidationError(format!(
            "Invalid value: {}. Couldn't parse value: {}",
            value_str, e
        )))),
    }
}

pub fn start_server(listen_at: &str, queue_size: usize, threads: u64) {
    println!(
        "Starting pathfinder. Listening at {} with {} threads and queue size {}.",
        listen_at, threads, queue_size
    );

    let edge_db_dispenser: Arc<EdgeDbDispenser> = Arc::new(EdgeDbDispenser::new());

    let (sender, receiver) = mpsc::sync_channel(queue_size);
    let protected_receiver = Arc::new(Mutex::new(receiver));
    for _ in 0..threads {
        let rec = protected_receiver.clone();
        let dispenser_clone = Arc::clone(&edge_db_dispenser);
        let t = thread::spawn(move || loop {
            let socket = rec.lock().unwrap().recv().unwrap();
            if let Err(e) = handle_connection(&dispenser_clone, socket) {
                println!("Error handling connection: {}", e);
            }
        });
        println!("Spawned thread: {:?}.", t.thread().id());
    }
    let listener = TcpListener::bind(listen_at).expect("Could not create server.");
    loop {
        match listener.accept() {
            Ok((socket, _)) => match sender.try_send(socket) {
                Ok(()) => {}
                Err(TrySendError::Full(mut socket)) => {
                    let _ = socket.write_all(b"HTTP/1.1 503 Service Unavailable\r\n\r\n");
                }
                Err(TrySendError::Disconnected(_)) => {
                    panic!("Internal communication channel disconnected.");
                }
            },
            Err(e) => println!("Error accepting connection: {}", e),
        }
    }
}
