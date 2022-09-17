use pathfinder2::flow::compute_flow;
use pathfinder2::io::read_edges_binary;
use pathfinder2::types::Address;

#[test]
fn test_flow() {
    let edges = read_edges_binary(&"edges.dat".to_string()).unwrap();
    let transfers = compute_flow(
        &Address::from("0x8DC7e86fF693e9032A0F41711b5581a04b26Be2E"),
        &Address::from("0x42cEDde51198D1773590311E2A340DC06B24cB37"),
        //&Address::from("0x9f5ff18027adbb65a53086cdc09d12ce463dae0b"),
        &edges,
        None,
    );
    println!("{:?}", transfers);
}
