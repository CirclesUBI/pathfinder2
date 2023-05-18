use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, RwLock};
use json::JsonValue;
use crate::rpc::call_context::CallContext;
use crate::rpc::rpc_functions::{compute_transfer, JsonRpcRequest, load_edges_binary, load_edges_csv, load_safes_binary, update_edges};
use crate::types::edge::EdgeDB;

pub fn handle_connection(
    edges: &RwLock<Arc<EdgeDB>>,
    mut socket: TcpStream,
) -> Result<(), Box<dyn Error>> {
    let request = read_request(&mut socket)?;
    let client_ip = socket.peer_addr()?.to_string();

    let call_context = CallContext::new(&client_ip, &request.id, &request.method);

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
            match load_edges_binary(edges,&request.params["file"].to_string(), &call_context) {
                Ok(len) => respond(&mut socket, request.id, Some(len), None),
                Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, &format!("Error loading edges: {}", e)))),
            }?;
        }
        "load_edges_csv" => {
            match load_edges_csv(edges, &request.params["file"].to_string(), &call_context) {
                Ok(len) => respond(&mut socket, request.id, Some(len), None),
                Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, &format!("Error loading edges: {}", e)))),
            }?;
        }
        "load_safes_binary" => {
            match load_safes_binary(edges, &request.params["file"].to_string(), &call_context) {
                Ok(len) => respond(&mut socket, request.id, Some(len), None),
                Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, &format!("Error loading edges: {}", e)))),
            }?;
        }
        "compute_transfer" => {
            let e = edges.read().unwrap().clone();
            compute_transfer(&request, e.as_ref(), socket, &call_context)?;
        }
        "update_edges" => {
            match request.params {
                JsonValue::Array(updates) => {
                    match update_edges(edges, updates, &call_context) {
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
