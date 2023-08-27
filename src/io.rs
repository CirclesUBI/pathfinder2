use crate::safe_db::db::DB;
use crate::types::{Address, Safe, U256};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::{self};

pub fn import_from_safes_binary(path: &str) -> Result<DB, io::Error> {
    let mut f = File::open(path)?;

    let mut safes: BTreeMap<Address, Safe> = Default::default();

    let address_index = read_address_index(&mut f)?;

    // organizations
    for _ in 0..read_u32(&mut f)? {
        let org_address = read_address(&mut f, &address_index)?;
        safes.entry(org_address).or_default().organization = true;
    }

    // trust edges
    for _ in 0..read_u32(&mut f)? {
        let user = read_address(&mut f, &address_index)?;
        assert!(user != Address::default());
        let send_to = read_address(&mut f, &address_index)?;
        assert!(send_to != Address::default());
        let limit_percentage = read_u8(&mut f)?;
        assert!(limit_percentage <= 100);

        if send_to != user && limit_percentage > 0 {
            safes
                .entry(user)
                .or_default()
                .limit_percentage
                .insert(send_to, limit_percentage);
        }
    }

    // balances
    for _ in 0..read_u32(&mut f)? {
        let user = read_address(&mut f, &address_index)?;
        assert!(user != Address::default());
        let token_owner = read_address(&mut f, &address_index)?;
        assert!(token_owner != Address::default());
        let balance = read_u256(&mut f)?;
        if balance != U256::from(0) {
            safes
                .entry(user)
                .or_default()
                .balances
                .insert(token_owner, balance);
        }
    }

    // we use the safe address as token address
    let mut token_owner = BTreeMap::default();
    for (addr, safe) in &mut safes {
        safe.token_address = *addr;
        token_owner.insert(*addr, *addr);
    }

    Ok(DB::new(safes, token_owner))
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
