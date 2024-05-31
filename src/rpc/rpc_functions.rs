use crate::graph;
use crate::io::import_from_safes_binary;
use crate::rpc::call_context::CallContext;
use crate::types::{Address, U256};
use json::JsonValue;
use num_bigint::BigUint;
use regex::Regex;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

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

pub fn load_safes_binary(file: &str, call_context: &CallContext) -> Result<usize, Box<dyn Error>> {
    let updated_edges = import_from_safes_binary(file)?.edges().clone();
    let len = updated_edges.edge_count();
    call_context.dispenser.update(updated_edges);
    Ok(len)
}

pub fn compute_transfer(
    request: &JsonRpcRequest,
    call_context: &CallContext,
) -> Result<JsonValue, Box<dyn Error>> {
    call_context.log_message(format!("{}", request.params).as_str());
    if call_context.version.is_none() {
        return Err(Box::new(InputValidationError(
            "No edges loaded yet".to_string(),
        )));
    }

    let edges = &call_context.version.as_ref().unwrap().edges;

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

    if let Some(max_distance) = max_distances.into_iter().next() {
        let (flow, transfers) = graph::compute_flow(
            &from_address,
            &to_address,
            edges.as_ref(),
            parsed_value_param,
            max_distance,
            max_transfers,
            call_context,
        );

        call_context.log_message(&format!(
            "Computed flow with max distance {:?}: {}",
            max_distance, flow
        ));

        // TODO: This implementation doesn't support the iterative approach anymore. Re-implement it.
        return Ok(json::object! {
            maxFlowValue: flow.to_decimal(),
            final: max_distance.is_none(),
            transferSteps: transfers.into_iter().map(|e| json::object! {
                from: e.from.to_checksummed_hex(),
                to: e.to.to_checksummed_hex(),
                token_owner: e.token.to_checksummed_hex(),
                value: e.capacity.to_decimal(),
            }).collect::<Vec<_>>(),
        });
    }

    Err(Box::new(InputValidationError(format!(
        "Couldn't find a path for {} CRC between {} -> {}.",
        parsed_value_param, from_address, to_address
    ))))
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
