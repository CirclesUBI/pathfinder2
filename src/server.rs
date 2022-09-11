use crate::flow;
use crate::io::read_edges_binary;
use crate::types::{Address, Edge};
use json::JsonValue;
use std::collections::HashMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use std::thread;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
};

pub struct Server {
    edges: Arc<HashMap<Address, Vec<Edge>>>,
    //threads: Vec<thread::JoinHandle<()>>,
}

struct JsonRpcRequest {
    id: JsonValue,
    method: String,
    params: JsonValue,
}

impl Server {
    pub fn start(port: u16) {
        let mut server = Server {
            edges: Arc::new(HashMap::new()),
            //threads: Vec::new(),
        };

        let listener =
            TcpListener::bind(format!("127.0.0.1:{port}")).expect("Could not create server.");
        loop {
            match listener.accept() {
                Ok((socket, _)) => match server.handle_connection(socket) {
                    Ok(_) => {}
                    Err(e) => println!("Error communicating with client: {e}"),
                },
                Err(e) => println!("Error accepting connection: {e}"),
            }
        }
    }

    fn handle_connection(&mut self, mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
        let request = read_request(&mut socket)?;
        match request.method.as_str() {
            "load_edges_binary" => {
                // TODO do this in its own thread?
                let edges = read_edges_binary(&request.params["file"].to_string())?;
                self.edges = Arc::new(edges);
                socket.write_all(jsonrpc_result(request.id, self.edges.len()).as_bytes())?;
            }
            "compute_transfer" => {
                // TODO limit number of threads
                let edges = self.edges.clone();
                let _thread = thread::spawn(move || {
                    println!("Computing flow");
                    let flow = flow::compute_flow(
                        &Address::from(request.params["from"].to_string().as_str()),
                        &Address::from(request.params["to"].to_string().as_str()),
                        //&U256::from(request.params["value"].to_string().as_str()),
                        edges.as_ref(),
                    );
                    println!("Computed flow");
                    // TODO error handling
                    socket
                        .write_all(
                            jsonrpc_result(request.id, json::JsonValue::from(flow)).as_bytes(),
                        )
                        .unwrap();
                });
                //self.threads.push(thread);
            }
            "cancel" => {}
            "update_edges" => {}
            // TODO error handling
            _ => {}
        }
        Ok(())
    }
}

fn read_request(socket: &mut TcpStream) -> Result<JsonRpcRequest, Box<dyn Error>> {
    let payload = read_payload(socket)?;
    let mut request = json::parse(&String::from_utf8(payload)?)?;
    println!("Request: {request}");
    let id = request["id"].take();
    let params = request["params"].take();
    match request["method"].as_str() {
        Some(method) => Ok(JsonRpcRequest {
            id,
            method: method.to_string(),
            params,
        }),
        _ => Err(From::from("Invalid JSON-RPC request: {request}")),
    }
}

fn read_payload(socket: &mut TcpStream) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut reader = BufReader::new(socket);
    let mut length = 0;
    for result in reader.by_ref().lines() {
        let l = result?;
        if l.is_empty() {
            break;
        }

        let header = "content-length: ";
        if l.to_lowercase().starts_with(header) {
            length = (&l[header.len()..]).parse::<usize>()?;
        }
    }
    let mut payload = vec![0u8; length];

    reader.read_exact(payload.as_mut_slice())?;
    Ok(payload)
}

fn jsonrpc_result(id: JsonValue, result: impl Into<json::JsonValue>) -> String {
    let payload = json::object! {
        jsonrpc: "2.0",
        id: id,
        result: result.into(),
    }
    .dump();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        payload.len(),
        payload
    )
}
