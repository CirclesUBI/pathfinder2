#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, PartialOrd)]
pub struct Address([u8; 20]);

impl From<[u8; 20]> for Address {
    fn from(item: [u8; 20]) -> Self {
        Address(item)
    }
}
