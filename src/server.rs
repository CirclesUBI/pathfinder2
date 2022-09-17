use crate::flow;
use crate::io::read_edges_binary;
use crate::types::{Address, Edge};
use json::JsonValue;
use std::collections::HashMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::thread;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
};

struct JsonRpcRequest {
    id: JsonValue,
    method: String,
    params: JsonValue,
}

type EdgeMap = HashMap<Address, Vec<Edge>>;

pub fn start_server(port: u16) {
    let edges: Arc<RwLock<Arc<EdgeMap>>> = Arc::new(RwLock::new(Arc::new(HashMap::new())));

    let listener =
        TcpListener::bind(format!("127.0.0.1:{port}")).expect("Could not create server.");
    loop {
        let c = edges.clone();
        match listener.accept() {
            // TODO limit number of threads
            Ok((socket, _)) => {
                thread::spawn(move || {
                    match handle_connection(c.deref(), socket) {
                        Ok(()) => {}
                        Err(e) => {
                            // TODO respond to the jsonrpc
                            println!("Error handling connection: {e}");
                        }
                    }
                });
            }
            Err(e) => println!("Error accepting connection: {e}"),
        }
    }
}

fn handle_connection(
    edges: &RwLock<Arc<HashMap<Address, Vec<Edge>>>>,
    mut socket: TcpStream,
) -> Result<(), Box<dyn Error>> {
    let request = read_request(&mut socket)?;
    match request.method.as_str() {
        "load_edges_binary" => {
            let updated_edges = read_edges_binary(&request.params["file"].to_string())?;
            let len = updated_edges.len();
            *edges.write().unwrap() = Arc::new(updated_edges);
            socket.write_all(jsonrpc_response(request.id, len).as_bytes())?;
        }
        "compute_transfer" => {
            println!("Computing flow");
            let e = edges.read().unwrap().clone();

            socket.write_all(chunked_header().as_bytes())?;
            let max_distances = if request.params["iterative"].as_bool().unwrap_or_default() {
                vec![Some(1), Some(2), None]
            } else {
                vec![None]
            };
            for max_distance in max_distances {
                let (flow, transfers) = flow::compute_flow(
                    &Address::from(request.params["from"].to_string().as_str()),
                    &Address::from(request.params["to"].to_string().as_str()),
                    //&U256::from(request.params["value"].to_string().as_str()),
                    e.as_ref(),
                    max_distance,
                );
                println!(
                    "Computed flow with max distance {:?}: {}",
                    max_distance, flow
                );
                // TODO error handling
                socket.write_all(
                    chunked_response(
                        &(jsonrpc_result(
                            request.id.clone(),
                            json::object! {
                                flow: flow.to_string(),
                                final: max_distance.is_none(),
                                transfers: transfers.into_iter().map(|e| json::object! {
                                    from: e.from.to_string(),
                                    to: e.to.to_string(),
                                    token: e.token.to_string(),
                                    value: e.capacity.to_string()
                                }).collect::<Vec<_>>(),
                            },
                        ) + "\r\n"),
                    )
                    .as_bytes(),
                )?;
            }
            socket.write_all(chunked_close().as_bytes())?;
        }
        "cancel" => {}
        "update_edges" => {}
        // TODO error handling
        _ => {}
    };
    Ok(())
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

fn jsonrpc_response(id: JsonValue, result: impl Into<json::JsonValue>) -> String {
    let payload = jsonrpc_result(id, result);
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        payload.len(),
        payload
    )
}

fn jsonrpc_result(id: JsonValue, result: impl Into<json::JsonValue>) -> String {
    json::object! {
        jsonrpc: "2.0",
        id: id,
        result: result.into(),
    }
    .dump()
}

fn chunked_header() -> String {
    "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_string()
}

fn chunked_response(data: &str) -> String {
    if data.is_empty() {
        String::new()
    } else {
        format!("{:x}\r\n{}\r\n", data.len(), data)
    }
}

fn chunked_close() -> String {
    "0\r\n\r\n".to_string()
}
