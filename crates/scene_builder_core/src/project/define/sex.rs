use crate::project::serialize::EncodeBinary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sex {
    pub male: bool,
    pub female: bool,
    pub futa: bool,
}

impl EncodeBinary for Sex {
    fn get_byte_size(&self) -> usize {
        size_of::<u8>()
    }

    fn write_byte(&self, buf: &mut Vec<u8>) -> () {
        if !self.male && !self.female && !self.futa {
            panic!("Empty Sex definition");
        }
        buf.push(
            (1 << 0) * self.male as u8 + (1 << 1) * self.female as u8 + (1 << 2) * self.futa as u8,
        );
    }
}

impl Default for Sex {
    fn default() -> Self {
        Self {
            male: true,
            female: false,
            futa: false,
        }
    }
}
