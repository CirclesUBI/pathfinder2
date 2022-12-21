use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Copy, Default, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Address([u8; 20]);

impl Address {
    pub fn short(&self) -> String {
        format!("{self}")[..8].to_string()
    }

    pub fn to_bytes(self) -> [u8; 20] {
        self.0
    }
}

impl Debug for Address {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl From<[u8; 20]> for Address {
    fn from(item: [u8; 20]) -> Self {
        Address(item)
    }
}

impl From<&str> for Address {
    fn from(item: &str) -> Self {
        let item = item.strip_prefix("0x").unwrap_or(item);
        assert!(item.len() == 20 * 2);
        let mut data = [0u8; 20];
        data.iter_mut().enumerate().for_each(|(i, b)| {
            *b = u8::from_str_radix(&item[2 * i..2 * i + 2], 16).unwrap();
        });
        Address(data)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "0x")?;
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}
