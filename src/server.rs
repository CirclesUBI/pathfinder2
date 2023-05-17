use crate::types::edge::EdgeDB;
use std::io::{Write};
use std::net::{TcpListener};
use std::ops::Deref;
use std::sync::mpsc::TrySendError;
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread;
use crate::rpc_handler::handle_connection;

pub fn start_server(listen_at: &str, queue_size: usize, threads: u64) {
    let edges: Arc<RwLock<Arc<EdgeDB>>> = Arc::new(RwLock::new(Arc::new(EdgeDB::default())));

    let (sender, receiver) = mpsc::sync_channel(queue_size);
    let protected_receiver = Arc::new(Mutex::new(receiver));
    for _ in 0..threads {
        let rec = protected_receiver.clone();
        let e = edges.clone();
        thread::spawn(move || loop {
            let socket = rec.lock().unwrap().recv().unwrap();
            if let Err(e) = handle_connection(e.deref(), socket) {
                println!("Error handling connection: {}", e);
            }
        });
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
