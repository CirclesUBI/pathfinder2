mod flow;
mod io;
mod types;

fn main() {
    let edges = io::read_edges_binary(&String::from("./edges.dat")).expect("Error loading edges.");
    print!("Read {} edges", edges.len());
}
