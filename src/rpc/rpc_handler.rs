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
        error: Option<(i64, String)>,
        call_context: &CallContext,
    ) -> Result<(), Box<dyn Error>> {
        if let Some((code, message)) = error.as_ref() {
            call_context.log_message(&format!("Error (code: {}): {}", code, message));
        }
        let response_json = jsonrpc_serialize_response(id, result.map(Into::into), error.as_ref().map(|(c, m)| (*c, m.as_str())));
        let rpc_response = jsonrpc_response(response_json.to_string());

        call_context.log_message(&format!("Result: {:?}", response_json));

        socket.write_all(rpc_response.as_bytes())?;
        Ok(())
    }


    match request.method.as_str() {
        "load_edges_binary" => {
            match load_edges_binary(edges, &request.params["file"].to_string(), &call_context) {
                Ok(len) => respond(&mut socket, request.id, Some(len), None, &call_context),
                Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, format!("Error loading edges: {}", e))), &call_context)
            }?;
        }
        "load_edges_csv" => {
            match load_edges_csv(edges, &request.params["file"].to_string(), &call_context) {
                Ok(len) => respond(&mut socket, request.id, Some(len), None, &call_context),
                Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, format!("Error loading edges: {}", e))), &call_context)
            }?;
        }
        "load_safes_binary" => {
            match load_safes_binary(edges, &request.params["file"].to_string(), &call_context) {
                Ok(len) => respond(&mut socket, request.id, Some(len), None, &call_context),
                Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, format!("Error loading safes: {}", e))), &call_context),
            }?;
        }
        "compute_transfer" => {
            let e = edges.read().unwrap().clone();
            match compute_transfer(&request, e.as_ref(), &call_context) {
                Ok(result) => respond(&mut socket, request.id, Some(result), None, &call_context),
                Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, format!("Error computing transfer path edges: {}", e))), &call_context),
            }?;
        }
        "update_edges" => {
            match request.params {
                JsonValue::Array(updates) => {
                    match update_edges(edges, updates, &call_context) {
                        Ok(len) => respond(&mut socket, request.id, Some(len), None, &call_context),
                        Err(e) => respond::<JsonValue>(&mut socket, request.id, None, Some((-32000, format!("Error updating edges: {}", e))), &call_context),
                    }?;
                },
                _ => {
                    respond::<JsonValue>(&mut socket, request.id, None, Some((-32602, "Invalid arguments: Expected array.".to_string())), &call_context)?;
                }
            }
        }
        _ => {
            respond::<JsonValue>(&mut socket, request.id, None, Some((-32601, "Method not found".to_string())), &call_context)?;
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

fn jsonrpc_serialize_response(id: JsonValue, result: impl Into<JsonValue>, error: Option<(i64, &str)>) -> String {
    let mut response = json::object! {
        jsonrpc: "2.0",
        id: id,
    };
    if let Some((code, message)) = error {
        response.insert("error", json::object! {
            code: code,
            message: message,
        }).unwrap();
    } else {
        response.insert("result", result.into()).unwrap();
    }
    response.dump()
}

fn jsonrpc_response(json_payload:String) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        json_payload.len(),
        json_payload
    )
}
