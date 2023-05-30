use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, RwLock};
use json::JsonValue;
use crate::rpc::rpc_functions::{compute_transfer, load_edges_binary, load_edges_csv, load_safes_binary, update_edges};
use crate::types::edge::EdgeDB;

pub struct JsonRpcRequest {
    pub(crate) id: JsonValue,
    pub method: String,
    pub(crate) params: JsonValue,
}

pub struct InputValidationError(pub String);
impl Error for InputValidationError {}

impl Debug for InputValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}", self.0)
    }
}
impl Display for InputValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}", self.0)
    }
}

pub fn handle_connection(
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

fn jsonrpc_response(id: JsonValue, result: impl Into<JsonValue>) -> String {
    let payload = jsonrpc_result(id, result);
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        payload.len(),
        payload
    )
}

pub fn jsonrpc_result(id: JsonValue, result: impl Into<JsonValue>) -> String {
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
