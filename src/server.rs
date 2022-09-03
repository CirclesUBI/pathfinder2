use json;
use json::JsonValue;
use std::error::Error;
use std::io::{BufRead, BufReader, ErrorKind};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
};

pub fn start(port: u16) {
    let listener =
        TcpListener::bind(format!("127.0.0.1:{port}")).expect("Could not create server.");
    loop {
        match listener.accept() {
            Ok((socket, address)) => match handle_connection(socket, address) {
                Ok(_) => {}
                Err(e) => println!("Error communicating with client: {e}"),
            },
            Err(e) => println!("Error accepting connection: {e}"),
        }
    }
}

struct JsonRpcRequest {
    id: JsonValue,
    method: String,
    params: JsonValue,
}

fn handle_connection(mut socket: TcpStream, address: SocketAddr) -> Result<(), Box<dyn Error>> {
    let request = read_request(socket)?;
    match request.method.as_str() {
        "load_edges_binary" => {}
        "compute_transfer" => {}
        "cancel" => {}
        "update_edges" => {}
        _ => {}
    }
    Ok(())
}

fn read_request(mut socket: TcpStream) -> Result<JsonRpcRequest, Box<dyn Error>> {
    // let mut buf_reader = BufReader::new(&mut socket);
    // let http_request: Vec<_> = buf_reader
    //     .by_ref()
    //     .lines()
    //     .map(|result| result.unwrap())
    //     .take_while(|line| !line.is_empty())
    //     .collect();
    // println!("{http_request:?}");
    // let mut buf = [0; 74];
    // buf_reader.read_exact(&mut buf)?;
    // println!("payload: {buf:?}");

    // let response = "HTTP/1.1 200 OK\r\n\r\n";

    // socket.write_all(response.as_bytes()).unwrap();
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

fn read_payload(socket: TcpStream) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut reader = BufReader::new(socket);
    let mut length = 0;
    for result in reader.by_ref().lines() {
        let l = result?;
        if l.is_empty() {
            break;
        }

        let header = "content-length: ";
        if l.to_lowercase().starts_with(header) {
            length = usize::from_str_radix(&l[header.len()..], 10)?;
        }
    }
    let mut payload = vec![0u8; length];

    reader.read_exact(payload.as_mut_slice())?;
    Ok(payload)
}
