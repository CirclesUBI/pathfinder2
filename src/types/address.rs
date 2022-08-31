#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq, PartialOrd)]
pub struct Address([u8; 20]);

impl From<[u8; 20]> for Address {
    fn from(item: [u8; 20]) -> Self {
        Address(item)
    }
}

impl From<&str> for Address {
    fn from(item: &str) -> Self {
        assert!(item.starts_with("0x"));
        assert!(item.len() == 2 + 20 * 2);
        let mut data = [0u8; 20];
        for i in (2..item.len()).step_by(2) {
            data[i / 2 - 1] = u8::from_str_radix(&item[i..i + 2], 16).unwrap();
        }
        Address(data)
    }
}
