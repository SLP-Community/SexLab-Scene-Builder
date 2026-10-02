use serde::{Deserialize, Serialize};

use crate::project::{serialize::EncodeBinary, NanoID};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Node {
    pub dest: Vec<NanoID>,
    pub x: f32,
    pub y: f32,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            dest: Default::default(),
            x: 40.0,
            y: 40.0,
        }
    }
}

impl EncodeBinary for Node {
    fn get_byte_size(&self) -> usize {
      self.dest.get_byte_size()
    }

    fn write_byte(&self, buf: &mut Vec<u8>) {
      self.dest.write_byte(buf);
    }
}
