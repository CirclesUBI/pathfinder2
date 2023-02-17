use crate::graph;
use crate::io::{import_from_safes_binary, read_edges_binary, read_edges_csv};
use crate::types::edge::EdgeDB;
use crate::types::{Address, Edge, U256};
use json::JsonValue;
use std::error::Error;
use std::io::Read;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::ops::Deref;
use std::sync::mpsc::TrySendError;
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread;

struct JsonRpcRequest {
    id: JsonValue,
    method: String,
    params: JsonValue,
}

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
                println!("Error handling connection: {e}");
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
            Err(e) => println!("Error accepting connection: {e}"),
        }
    }
}

fn handle_connection(
    edges: &RwLock<Arc<EdgeDB>>,
    mut socket: TcpStream,
) -> Result<(), Box<dyn Error>> {
    let request = read_request(&mut socket)?;
    match request.method.as_str() {
        "load_edges_binary" => {
            let response = match load_edges_binary(edges, &request.params["file"].to_string()) {
                Ok(len) => jsonrpc_response(request.id, len),
                Err(e) => {
                    jsonrpc_error_response(request.id, -32000, &format!("Error loading edges: {e}"))
                }
            };
            socket.write_all(response.as_bytes())?;
        }
        "load_edges_csv" => {
            let response = match load_edges_csv(edges, &request.params["file"].to_string()) {
                Ok(len) => jsonrpc_response(request.id, len),
                Err(e) => {
                    jsonrpc_error_response(request.id, -32000, &format!("Error loading edges: {e}"))
                }
            };
            socket.write_all(response.as_bytes())?;
        }
        "load_safes_binary" => {
            let response = match load_safes_binary(edges, &request.params["file"].to_string()) {
                Ok(len) => jsonrpc_response(request.id, len),
                Err(e) => {
                    jsonrpc_error_response(request.id, -32000, &format!("Error loading edges: {e}"))
                }
            };
            socket.write_all(response.as_bytes())?;
        }
        "compute_transfer" => {
            println!("Computing flow");
            let e = edges.read().unwrap().clone();
            compute_transfer(request, e.as_ref(), socket)?;
        }
        "update_edges" => {
            let response = match request.params {
                JsonValue::Array(updates) => match update_edges(edges, updates) {
                    Ok(len) => jsonrpc_response(request.id, len),
                    Err(e) => jsonrpc_error_response(
                        request.id,
                        -32000,
                        &format!("Error updating edges: {e}"),
                    ),
                },
                _ => {
                    jsonrpc_error_response(request.id, -32602, "Invalid arguments: Expected array.")
                }
            };
            socket.write_all(response.as_bytes())?;
        }
        _ => socket
            .write_all(jsonrpc_error_response(request.id, -32601, "Method not found").as_bytes())?,
    };
    Ok(())
}

fn load_edges_binary(edges: &RwLock<Arc<EdgeDB>>, file: &String) -> Result<usize, Box<dyn Error>> {
    let updated_edges = read_edges_binary(file)?;
    let len = updated_edges.edge_count();
    *edges.write().unwrap() = Arc::new(updated_edges);
    Ok(len)
}

fn load_edges_csv(edges: &RwLock<Arc<EdgeDB>>, file: &String) -> Result<usize, Box<dyn Error>> {
    let updated_edges = read_edges_csv(file)?;
    let len = updated_edges.edge_count();
    *edges.write().unwrap() = Arc::new(updated_edges);
    Ok(len)
}

fn load_safes_binary(edges: &RwLock<Arc<EdgeDB>>, file: &str) -> Result<usize, Box<dyn Error>> {
    let updated_edges = import_from_safes_binary(file)?.edges().clone();
    let len = updated_edges.edge_count();
    *edges.write().unwrap() = Arc::new(updated_edges);
    Ok(len)
}

fn compute_transfer(
    request: JsonRpcRequest,
    edges: &EdgeDB,
    mut socket: TcpStream,
) -> Result<(), Box<dyn Error>> {
    socket.write_all(chunked_header().as_bytes())?;
    let max_distances = if request.params["iterative"].as_bool().unwrap_or_default() {
        vec![Some(1), Some(2), None]
    } else {
        vec![None]
    };
    let max_transfers = request.params["max_transfers"].as_u64();
    for max_distance in max_distances {
        let (flow, transfers) = graph::compute_flow(
            &Address::from(request.params["from"].to_string().as_str()),
            &Address::from(request.params["to"].to_string().as_str()),
            edges,
            if request.params.has_key("value") {
                U256::from(request.params["value"].to_string().as_str())
            } else {
                U256::MAX
            },
            max_distance,
            max_transfers,
        );
        println!("Computed flow with max distance {max_distance:?}: {flow}");
        socket.write_all(
            chunked_response(
                &(jsonrpc_result(
                    request.id.clone(),
                    json::object! {
                        flow: flow.to_decimal(),
                        final: max_distance.is_none(),
                        transfers: transfers.into_iter().map(|e| json::object! {
                            from: e.from.to_checksummed_hex(),
                            to: e.to.to_checksummed_hex(),
                            token_owner: e.token.to_checksummed_hex(),
                            value: e.capacity.to_decimal(),
                        }).collect::<Vec<_>>(),
                    },
                ) + "\r\n"),
            )
            .as_bytes(),
        )?;
    }
    socket.write_all(chunked_close().as_bytes())?;
    Ok(())
}

fn update_edges(
    edges: &RwLock<Arc<EdgeDB>>,
    updates: Vec<JsonValue>,
) -> Result<usize, Box<dyn Error>> {
    let updates = updates
        .into_iter()
        .map(|e| Edge {
            from: Address::from(e["from"].to_string().as_str()),
            to: Address::from(e["to"].to_string().as_str()),
            token: Address::from(e["token_owner"].to_string().as_str()),
            capacity: U256::from(e["capacity"].to_string().as_str()),
        })
        .collect::<Vec<_>>();
    if updates.is_empty() {
        return Ok(edges.read().unwrap().edge_count());
    }

    let mut updating_edges = edges.read().unwrap().as_ref().clone();
    for update in updates {
        updating_edges.update(update);
    }
    let len = updating_edges.edge_count();
    *edges.write().unwrap() = Arc::new(updating_edges);
    Ok(len)
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
            length = l[header.len()..].parse::<usize>()?;
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

fn jsonrpc_error_response(id: JsonValue, code: i64, message: &str) -> String {
    let payload = json::object! {
        jsonrpc: "2.0",
        id: id,
        error: {
            code: code,
            message: message
        }
    }
    .dump();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        payload.len(),
        payload
    )
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
