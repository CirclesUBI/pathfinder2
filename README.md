# Pathfinder2

Pathfinder is a collection of tools related to  
computing transitive transfers in the  
[CirclesUBI](https://joincircles.net) trust graph.

## Building

This is a rust project, so assuming `cargo` is installed, `cargo build` creates three binaries:  
The `server` (default), the `cli` and the `convert` tool.

All need a file that contains the trust graph edges to work.  
A reasonably up to date edge database file can be obtained from

- https://chriseth.github.io/pathfinder2/edges.dat

### Using the Server

`cargo run --release <ip-address>:<port>` will start a JSON-RPC server listening on the given port.

It implements the interface specified in https://hackmd.io/Gg04t7gjQKeDW2Q6Jchp0Q

The HTTP server implementation ignores most parts of the HTTP request and header, including the path,
so it essentially responds on all paths.

It has two performance parameters that are currently hardcoded in the source:

Number of worker threads: 4

Size of request queue: 10

#### Run with test data
1) Download the balances and trust binary dump from [binary dump from 2023-05-23](graph_at_20230523_15_00.db)
2) Start the server with `cargo run --release <ip-address>:<port>`
3) Import the data with the curl command below
4) Query the server with the curl command below

The data can be imported into a running pathfinder2 server with the following command:
```shell
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "id": "timestamp_value",
    "method": "load_safes_binary",
    "params": {
        "file": "/path/to/graph_at_20230523_15_00.db"
    }
}' \
  "http://<ip>:<port>"
```
afterward the server can be queried with the following command:
```shell
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "id": "timestamp_value",
    "method": "compute_transfer",
    "params": {
        "from": "0x000...",
        "to": "0x000...",
        "value": 999999999999,
        "iterative": false,
        "prune": true
    }
}' \
  "http://<ip>:<port>"
```

### Using the CLI

The CLI will load an edge database file and compute the transitive transfers from one source to one destination. You can limit the number of hops to explore and the maximum amount of circles to transfer.

The options are:

`cargo run --release --bin cli <from> <to> <edges.dat> [<max_hops> [<max_amount>]] [--dot <dotfile>]`

For example:

`cargo run --release --bin cli 0x9BA1Bcd88E99d6E1E03252A70A63FEa83Bf1208c 0x42cEDde51198D1773590311E2A340DC06B24cB37 edges.dat 3 1000000000000000000`

Computes a transfer of at most `1000000000000000000`, exploring 3 hops.

If you specify `--dot <dotfile>`, a graphviz/dot representation of the transfer graph is written to the given file.

### Conversion Tool

The conversion tool can convert between different ways of representing the edge and trust relations in the circles system.  
All data formats are described in https://hackmd.io/Gg04t7gjQKeDW2Q6Jchp0Q

It can read an edge database both in CSV and binary formatand a "safe database" in json and binary format.  
The output is always an edge database in either binary or CSV format.

Example:

`cargo run --bin convert --safes-json safes.json --edges-bin edges.dat`

Converts a safe json file called `safes.json` into a binary edge database file called `edges.dat`.


```shell
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "id": "1",
    "method": "load_safes_binary",
    "params": {
        "file": "/home/daniel/src/CirclesUBI/pathfinder2/graph_at_20230523_15_00.db"
    }
}' \
  "http://localhost:1234"
  
  
  
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "id": "1",
    "method": "compute_transfer",
    "params": {
        "from": "0xde374ece6fa50e781e81aac78e811b33d16912c7",
        "to": "0x6de572faa138048ce8142c4a206eb09a8ec39e45",
        "value": "9999999999999999999999999",
        "iterative": false,
        "prune": true
    }
}' \
  "http://localhost:1234"

```
