use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use json::JsonValue;
use crate::types::edge::EdgeDB;
use crate::rpc_functions::{load_edges_binary, load_edges_csv, load_safes_binary, compute_transfer, update_edges, JsonRpcRequest};

pub fn handle_connection(
    edges: &RwLock<Arc<EdgeDB>>,
    mut socket: TcpStream,
) -> Result<(), Box<dyn Error>> {
    let start_time = std::time::Instant::now();
    let request = read_request(&mut socket)?;
    let request_id = request.id.clone();
    let client_ip = socket.peer_addr()?.to_string();

    println!("{}", log_rpc_call(&client_ip, &request.id, &request.method, None));

    fn respond<T: Into<JsonValue>>(
        socket: &mut TcpStream,
        id: JsonValue,
        result: Option<T>,
        error: Option<(i64, &str)>,
    ) -> Result<(), Box<dyn Error>> {
        let response = jsonrpc_response(id, result.map(Into::into), error);
        socket.write_all(response.as_bytes())?;
        Ok(())
    }

    match request.method.as_str() {
        "load_edges_binary" => {
            match load_edges_binary(edges, &request.params["file"].to_string()) {
                Ok(len) => respond(&mut socket, request.id, Some(len), None),
                Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, &format!("Error loading edges: {}", e)))),
            }?;
        }
        "load_edges_csv" => {
            match load_edges_csv(edges, &request.params["file"].to_string()) {
                Ok(len) => respond(&mut socket, request.id, Some(len), None),
                Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, &format!("Error loading edges: {}", e)))),
            }?;
        }
        "load_safes_binary" => {
            match load_safes_binary(edges, &request.params["file"].to_string()) {
                Ok(len) => respond(&mut socket, request.id, Some(len), None),
                Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, &format!("Error loading edges: {}", e)))),
            }?;
        }
        "compute_transfer" => {
            println!("Computing flow");
            let e = edges.read().unwrap().clone();
            compute_transfer(&request, e.as_ref(), socket)?; // Pass a reference to request
        }
        "update_edges" => {
            match request.params {
                JsonValue::Array(updates) => {
                    match update_edges(edges, updates) {
                        Ok(len) => respond(&mut socket, request.id, Some(len), None),
                        Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, &format!("Error loading edges: {}", e)))),
                    }?;
                },
                _ => {
                    respond::<JsonValue>(&mut socket, request.id, None, Some((-32602, "Invalid arguments: Expected array.")))?;
                }
            }
        }
        _ => {
            respond::<JsonValue>(&mut socket, request.id, None, Some((-32601, "Method not found")))?;
        }
    };

    let call_duration = start_time.elapsed().as_millis();
    println!("{}", log_rpc_call(&client_ip, &request_id, &request.method, Some(call_duration)));

    Ok(())
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
        _ => Err(From::from(format!("Invalid JSON-RPC request: {}", request))),
    }
}

fn log_rpc_call(client_ip: &str, request_id: &JsonValue, rpc_function: &str, call_duration: Option<u128>) -> String {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    match call_duration {
        Some(duration) => format!(
            "<- {} [{}] [{}] [{}] took {} ms",
            timestamp, client_ip, request_id, rpc_function, duration
        ),
        None => format!(
            "-> {} [{}] [{}] [{}]",
            timestamp, client_ip, request_id, rpc_function
        ),
    }
}

fn jsonrpc_response(id: JsonValue, result: impl Into<JsonValue>, error: Option<(i64, &str)>) -> String {
    let payload = match error {
        Some((code, message)) => json::object! {
            jsonrpc: "2.0",
            id: id,
            error: {
                code: code,
                message: message
            }
        },
        None => json::object! {
            jsonrpc: "2.0",
            id: id,
            result: result.into(),
        },
    }.dump();

    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        payload.len(),
        payload
    )
}
