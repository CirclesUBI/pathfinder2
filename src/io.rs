use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Read;

use crate::types::{Address, Edge, U256};

pub fn read_edges_binary(path: &String) -> Result<HashMap<Address, Vec<Edge>>, io::Error> {
    let mut f = File::open(path)?;
    let address_index = read_address_index(&mut f)?;
    read_edges(&mut f, &address_index)
}

fn read_address_index(file: &mut File) -> Result<HashMap<u32, Address>, io::Error> {
    let address_count = read_u32(file)?;
    let mut addresses = HashMap::new();
    for i in 0..address_count {
        let mut buf = [0; 20];
        file.read_exact(&mut buf)?;
        addresses.insert(i, Address::from(buf));
    }
    Ok(addresses)
}

fn read_u32(file: &mut File) -> Result<u32, io::Error> {
    let mut buf = [0; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn read_u8(file: &mut File) -> Result<u8, io::Error> {
    let mut buf = [0; 1];
    file.read_exact(&mut buf)?;
    Ok(u8::from_be_bytes(buf))
}

fn read_address(
    file: &mut File,
    address_index: &HashMap<u32, Address>,
) -> Result<Address, io::Error> {
    let index = read_u32(file)?;
    Ok(address_index[&index])
}

fn read_u256(file: &mut File) -> Result<U256, io::Error> {
    let length = read_u8(file)? as usize;
    let mut bytes = [0u8; 32];
    file.read_exact(&mut bytes[32 - length..32])?;
    let high = u128::from_be_bytes(*<&[u8; 16]>::try_from(&bytes[0..16]).unwrap());
    let low = u128::from_be_bytes(*<&[u8; 16]>::try_from(&bytes[16..32]).unwrap());
    Ok(U256::new(high, low))
}

fn read_edges(
    file: &mut File,
    address_index: &HashMap<u32, Address>,
) -> Result<HashMap<Address, Vec<Edge>>, io::Error> {
    let edge_count = read_u32(file)?;
    let mut edges: HashMap<Address, Vec<Edge>> = HashMap::new();
    for _i in 0..edge_count {
        let from = read_address(file, address_index)?;
        let to = read_address(file, address_index)?;
        let token = read_address(file, address_index)?;
        let capacity = read_u256(file)?;
        edges.entry(from).or_insert(vec![]).push(Edge {
            from,
            to,
            token,
            capacity,
        });
    }
    Ok(edges)
}
