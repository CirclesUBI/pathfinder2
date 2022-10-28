## Pathfinder2

Pathfinder is a collection of tools related to
computing transitive transfers in the
[CirclesUBI](https://joincircles.net) trust graph.

### Building

This is a rust project, so assuming `cargo` is installed, `cargo build`
creates two binaries: The server (default) and the cli.

Both need a file that contains the trust graph edges to work.
A reasonably up to date edge database file can be obtained from
https://chriseth.github.io/pathfinder2/edges.dat


#### Using the Server

`cargo run --release <port>` will start a JSON-RPC server listening on the given port.

It implements the interface specified in https://hackmd.io/Gg04t7gjQKeDW2Q6Jchp0Q

It has two performance parameters that are currently hardcoded in the source:

Number of worker threads: 4

Size of request queue: 10

#### Using the CLI

The CLI will load an edge database file and compute the transitive transfers
from one source to one destination. You can limit the number of hops to explore
and the maximum amount of circles to transfer.


The options are:

`cargo run --release --bin cli <from> <to> <edges.dat> [<max_hops> [<max_amount>]] [--dot <dotfile>]`

For example 

`cargo run --release --bin cli 0x9BA1Bcd88E99d6E1E03252A70A63FEa83Bf1208c 0x42cEDde51198D1773590311E2A340DC06B24cB37 edges.dat 3 1000000000000000000`

Computes a transfer of at most `1000000000000000000`, exploring 3 hops.

If you specify `--dot <dotfile>`, a graphviz/dot representation of the transfer graph is written to the given file.
