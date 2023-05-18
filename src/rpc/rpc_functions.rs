use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::Write;
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use json::JsonValue;
use num_bigint::BigUint;
use crate::graph;
use crate::io::{import_from_safes_binary, read_edges_binary, read_edges_csv};
use crate::types::{Address, Edge, U256};
use crate::types::edge::EdgeDB;
use regex::Regex;
use crate::rpc::call_context::CallContext;

pub struct JsonRpcRequest {
    pub id: JsonValue,
    pub method: String,
    pub params: JsonValue,
}

struct InputValidationError(String);
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

pub fn load_edges_binary(edges: &RwLock<Arc<EdgeDB>>, file: &String, _: &CallContext) -> Result<usize, Box<dyn Error>> {
    let updated_edges = read_edges_binary(file)?;
    let len = updated_edges.edge_count();
    /* TODO: When under high read load by compute_transfer, this update mechanism could suffer from write starvation */
    *edges.write().unwrap() = Arc::new(updated_edges);
    Ok(len)
}

pub fn load_edges_csv(edges: &RwLock<Arc<EdgeDB>>, file: &String, _: &CallContext) -> Result<usize, Box<dyn Error>> {
    let updated_edges = read_edges_csv(file)?;
    let len = updated_edges.edge_count();
    /* TODO: When under high read load by compute_transfer, this update mechanism could suffer from write starvation */
    *edges.write().unwrap() = Arc::new(updated_edges);
    Ok(len)
}

pub fn load_safes_binary(edges: &RwLock<Arc<EdgeDB>>, file: &str, call_context: &CallContext) -> Result<usize, Box<dyn Error>> {
    let updated_edges = import_from_safes_binary(file, call_context)?.edges().clone();
    let len = updated_edges.edge_count();
    /* TODO: When under high read load by compute_transfer, this update mechanism could suffer from write starvation */
    *edges.write().unwrap() = Arc::new(updated_edges);
    Ok(len)
}

pub fn compute_transfer(
    request: &JsonRpcRequest,
    edges: &EdgeDB,
    mut socket: TcpStream,
    call_context: &CallContext
) -> Result<(), Box<dyn Error>> {
    socket.write_all(chunked_header().as_bytes())?;

    call_context.log_message(format!("{}", request.params).as_str());

    let parsed_value_param = match request.params["value"].as_str() {
        Some(value_str) => validate_and_parse_u256(value_str)?,
        None => U256::MAX,
    };

    let from_address = validate_and_parse_ethereum_address(&request.params["from"].to_string())?;
    let to_address = validate_and_parse_ethereum_address(&request.params["to"].to_string())?;

    let max_distances = if request.params["iterative"].as_bool().unwrap_or_default() {
        vec![Some(1), Some(2), None]
    } else {
        vec![None]
    };

    let max_transfers = request.params["max_transfers"].as_u64();
    for max_distance in max_distances {
        let (flow, transfers) = graph::compute_flow(
            &from_address,
            &to_address,
            edges,
            parsed_value_param,
            max_distance,
            max_transfers,
            call_context
        );

        call_context.log_message(&format!("Computed flow with max distance {:?}: {}", max_distance, flow));

        let json_rpc_result = jsonrpc_result(
            request.id.clone(),
            json::object! {
                        maxFlowValue: flow.to_decimal(),
                        final: max_distance.is_none(),
                        transferSteps: transfers.into_iter().map(|e| json::object! {
                            from: e.from.to_checksummed_hex(),
                            to: e.to.to_checksummed_hex(),
                            token_owner: e.token.to_checksummed_hex(),
                            value: e.capacity.to_decimal(),
                        }).collect::<Vec<_>>(),
                    },
        );

        call_context.log_message(&format!("Sending response: {}", json_rpc_result));

        socket.write_all(chunked_response(&(json_rpc_result + "\r\n")).as_bytes())?;
    }
    socket.write_all(chunked_close().as_bytes())?;
    Ok(())
}

pub fn update_edges(
    edges: &RwLock<Arc<EdgeDB>>,
    updates: Vec<JsonValue>,
    _: &CallContext
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
    /* TODO: When under high read load by compute_transfer, this update mechanism could suffer from write starvation */
    *edges.write().unwrap() = Arc::new(updating_edges);
    Ok(len)
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

fn jsonrpc_result(id: JsonValue, result: impl Into<JsonValue>) -> String {
    json::object! {
        jsonrpc: "2.0",
        id: id,
        result: result.into(),
    }
    .dump()
}

fn validate_and_parse_u256(value_str: &str) -> Result<U256, Box<dyn Error>> {
    match BigUint::from_str(value_str) {
        Ok(parsed_value) => {
            if parsed_value > U256::MAX.into() {
                Err(Box::new(InputValidationError(format!(
                    "Value {} is too large. Maximum value is {}.",
                    parsed_value, U256::MAX
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
