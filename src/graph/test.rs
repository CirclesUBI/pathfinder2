#[cfg(test)]
mod test {
    use crate::graph::compute_flow;
    use crate::rpc::call_context::CallContext;
    use crate::types::{Address, Edge, U256};
    use crate::types::edge::EdgeDB;

    fn addresses() -> (Address, Address, Address, Address, Address, Address) {
        (
            Address::from("0x11C7e86fF693e9032A0F41711b5581a04b26Be2E"),
            Address::from("0x22cEDde51198D1773590311E2A340DC06B24cB37"),
            Address::from("0x33cEDde51198D1773590311E2A340DC06B24cB37"),
            Address::from("0x447EDde51198D1773590311E2A340DC06B24cB37"),
            Address::from("0x55c16ce62d26fd51582a646e2e30a3267b1e6d7e"),
            Address::from("0x66c16ce62d26fd51582a646e2e30a3267b1e6d7e"),
        )
    }
    fn build_edges(input: Vec<Edge>) -> EdgeDB {
        EdgeDB::new(input)
    }

    #[test]
    fn direct() {
        let (a, b, t, ..) = addresses();
        let edges = build_edges(vec![Edge {
            from: a,
            to: b,
            token: t,
            capacity: U256::from(10),
        }]);
        let flow = compute_flow(&a, &b, &edges, U256::MAX, None, None, &CallContext::default());
        assert_eq!(
            flow,
            (
                U256::from(10),
                vec![Edge {
                    from: a,
                    to: b,
                    token: t,
                    capacity: U256::from(10)
                }]
            )
        );
    }

    #[test]
    fn one_hop() {
        let (a, b, c, t1, t2, ..) = addresses();
        let edges = build_edges(vec![
            Edge {
                from: a,
                to: b,
                token: t1,
                capacity: U256::from(10),
            },
            Edge {
                from: b,
                to: c,
                token: t2,
                capacity: U256::from(8),
            },
        ]);
        let flow = compute_flow(&a, &c, &edges, U256::MAX, None, None, &CallContext::default());
        assert_eq!(
            flow,
            (
                U256::from(8),
                vec![
                    Edge {
                        from: a,
                        to: b,
                        token: t1,
                        capacity: U256::from(8)
                    },
                    Edge {
                        from: b,
                        to: c,
                        token: t2,
                        capacity: U256::from(8)
                    },
                ]
            )
        );
    }

    #[test]
    fn diamond() {
        let (a, b, c, d, t1, t2) = addresses();
        let edges = build_edges(vec![
            Edge {
                from: a,
                to: b,
                token: t1,
                capacity: U256::from(10),
            },
            Edge {
                from: a,
                to: c,
                token: t2,
                capacity: U256::from(7),
            },
            Edge {
                from: b,
                to: d,
                token: t2,
                capacity: U256::from(9),
            },
            Edge {
                from: c,
                to: d,
                token: t1,
                capacity: U256::from(8),
            },
        ]);
        let mut flow = compute_flow(&a, &d, &edges, U256::MAX, None, None, &CallContext::default());
        flow.1.sort();
        assert_eq!(
            flow,
            (
                U256::from(16),
                vec![
                    Edge {
                        from: a,
                        to: b,
                        token: t1,
                        capacity: U256::from(9)
                    },
                    Edge {
                        from: a,
                        to: c,
                        token: t2,
                        capacity: U256::from(7)
                    },
                    Edge {
                        from: b,
                        to: d,
                        token: t2,
                        capacity: U256::from(9)
                    },
                    Edge {
                        from: c,
                        to: d,
                        token: t1,
                        capacity: U256::from(7)
                    },
                ]
            )
        );
        let mut pruned_flow = compute_flow(&a, &d, &edges, U256::from(6), None, None, &CallContext::default());
        pruned_flow.1.sort();
        assert_eq!(
            pruned_flow,
            (
                U256::from(6),
                vec![
                    Edge {
                        from: a,
                        to: b,
                        token: t1,
                        capacity: U256::from(6)
                    },
                    Edge {
                        from: b,
                        to: d,
                        token: t2,
                        capacity: U256::from(6)
                    },
                ]
            )
        );
    }

    #[test]
    fn trust_transfer_limit() {
        let (a, b, c, d, ..) = addresses();
        let edges = build_edges(vec![
            // The following two edges should be balance-limited,
            // i.e. a -> first intermediate is limited by the max of the two.
            Edge {
                from: a,
                to: b,
                token: a,
                capacity: U256::from(10),
            },
            Edge {
                from: a,
                to: c,
                token: a,
                capacity: U256::from(11),
            },
            // The following two edges should be trust-limited,
            // i.e. the edge from the second (pre-) intermediate is limited
            // by the max of the two.
            Edge {
                from: b,
                to: d,
                token: a,
                capacity: U256::from(9),
            },
            Edge {
                from: c,
                to: d,
                token: a,
                capacity: U256::from(8),
            },
        ]);
        let mut flow = compute_flow(&a, &d, &edges, U256::MAX, None, None, &CallContext::default());
        flow.1.sort();
        println!("{:?}", &flow.1);
        assert_eq!(flow.0, U256::from(9));
    }
}
