use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, BufRead};
use std::io::{Read, Write};
use std::{collections::HashMap, io::BufReader};

use crate::types::edge::EdgeDB;
use crate::types::{Address, Edge, U256};

pub fn read_edges_binary(path: &String) -> Result<EdgeDB, io::Error> {
    let mut f = File::open(path)?;
    let address_index = read_address_index(&mut f)?;
    read_edges(&mut f, &address_index)
}

pub fn read_edges_csv(path: &String) -> Result<EdgeDB, io::Error> {
    let mut edges = Vec::new();
    let f = BufReader::new(File::open(path)?);
    for line in f.lines() {
        let line = line?;
        match &line.split(',').collect::<Vec<_>>()[..] {
            [] => continue,
            [from, to, token, capacity] => {
                let from = Address::from(unescape(from));
                let to = Address::from(unescape(to));
                let token = Address::from(unescape(token));
                let capacity = U256::from(unescape(capacity));
                edges.push(Edge {
                    from,
                    to,
                    token,
                    capacity,
                });
            }
            _ => {
                return Result::Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Expected from,to,token,capacity, but got {line}"),
                ))
            }
        }
    }
    Ok(EdgeDB::new(edges))
}

pub fn write_edges_binary(edges: &EdgeDB, path: &String) -> Result<(), io::Error> {
    let mut file = File::create(path)?;
    let address_index = write_address_index(&mut file, edges)?;
    write_edges(&mut file, edges, &address_index)
}

pub fn write_edges_csv(edges: &EdgeDB, path: &String) -> Result<(), io::Error> {
    let mut file = File::create(path)?;
    let mut sorted_edges = edges.edges().clone();
    sorted_edges.sort();
    for Edge {
        from,
        to,
        token,
        capacity,
    } in sorted_edges
    {
        writeln!(file, "{from},{to},{token},{capacity}")?;
    }
    Ok(())
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

fn write_address_index(
    file: &mut File,
    edges: &EdgeDB,
) -> Result<HashMap<Address, u32>, io::Error> {
    let mut addresses = BTreeSet::new();
    for Edge {
        from, to, token, ..
    } in edges.edges()
    {
        addresses.insert(*from);
        addresses.insert(*to);
        addresses.insert(*token);
    }
    write_u32(file, addresses.len() as u32)?;
    let mut index = HashMap::new();
    for (i, addr) in addresses.into_iter().enumerate() {
        file.write_all(&addr.to_bytes())?;
        index.insert(addr, i as u32);
    }
    Ok(index)
}

fn read_u32(file: &mut File) -> Result<u32, io::Error> {
    let mut buf = [0; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn write_u32(file: &mut File, v: u32) -> Result<(), io::Error> {
    let buf = v.to_be_bytes();
    file.write_all(&buf)
}

fn read_u8(file: &mut File) -> Result<u8, io::Error> {
    let mut buf = [0; 1];
    file.read_exact(&mut buf)?;
    Ok(u8::from_be_bytes(buf))
}

fn write_u8(file: &mut File, v: u8) -> Result<(), io::Error> {
    let buf = v.to_be_bytes();
    file.write_all(&buf)
}

fn read_address(
    file: &mut File,
    address_index: &HashMap<u32, Address>,
) -> Result<Address, io::Error> {
    let index = read_u32(file)?;
    Ok(address_index[&index])
}

fn write_address(
    file: &mut File,
    address: &Address,
    address_index: &HashMap<Address, u32>,
) -> Result<(), io::Error> {
    write_u32(file, *address_index.get(address).unwrap())
}

fn read_u256(file: &mut File) -> Result<U256, io::Error> {
    let length = read_u8(file)? as usize;
    let mut bytes = [0u8; 32];
    file.read_exact(&mut bytes[32 - length..32])?;
    let high = u128::from_be_bytes(*<&[u8; 16]>::try_from(&bytes[0..16]).unwrap());
    let low = u128::from_be_bytes(*<&[u8; 16]>::try_from(&bytes[16..32]).unwrap());
    Ok(U256::new(high, low))
}

fn write_u256(file: &mut File, v: &U256) -> Result<(), io::Error> {
    let v_bytes = v.to_bytes();
    if v_bytes.is_empty() {
        file.write_all(&[1, 0])
    } else {
        write_u8(file, v_bytes.len() as u8)?;
        file.write_all(&v_bytes)
    }
}

fn read_edges(file: &mut File, address_index: &HashMap<u32, Address>) -> Result<EdgeDB, io::Error> {
    let edge_count = read_u32(file)?;
    let mut edges = Vec::new();
    for _i in 0..edge_count {
        let from = read_address(file, address_index)?;
        let to = read_address(file, address_index)?;
        let token = read_address(file, address_index)?;
        let capacity = read_u256(file)?;
        edges.push(Edge {
            from,
            to,
            token,
            capacity,
        });
    }
    Ok(EdgeDB::new(edges))
}

fn write_edges(
    file: &mut File,
    edges: &EdgeDB,
    address_index: &HashMap<Address, u32>,
) -> Result<(), io::Error> {
    write_u32(file, edges.edge_count() as u32)?;
    let mut sorted_edges = edges.edges().clone();
    sorted_edges.sort();
    for Edge {
        from,
        to,
        token,
        capacity,
    } in &sorted_edges
    {
        write_address(file, from, address_index)?;
        write_address(file, to, address_index)?;
        write_address(file, token, address_index)?;
        write_u256(file, capacity)?;
    }
    Ok(())
}

fn unescape(input: &str) -> &str {
    match input.chars().next() {
        Some('"') | Some('\'') => {
            assert!(input.len() >= 2 && input.chars().last() == input.chars().next());
            &input[1..input.len() - 1]
        }
        _ => input,
    }
}
