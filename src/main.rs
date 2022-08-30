mod address;
mod edge;
mod io;
mod u256;

fn main() {
    let edges = io::read_edges_binary(&String::from("./edges.dat")).expect("Error loading edges.");
    print!("Read {} edges", edges.len());
}
