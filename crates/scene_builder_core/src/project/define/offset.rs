use std::mem::size_of;
use serde::{Deserialize, Serialize};
use crate::project::serialize::EncodeBinary;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Offset {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: f32,
}

impl EncodeBinary for Offset {
    fn get_byte_size(&self) -> usize {
        size_of::<Offset>()
    }

    fn write_byte(&self, buf: &mut Vec<u8>) -> () {
        let x_ = (self.x * 1000.0).round() as i32;
        buf.extend_from_slice(&x_.to_be_bytes());
        let y_ = (self.y * 1000.0).round() as i32;
        buf.extend_from_slice(&y_.to_be_bytes());
        let z_ = (self.z * 1000.0).round() as i32;
        buf.extend_from_slice(&z_.to_be_bytes());
        let r_ = (self.r * 1000.0).round() as i32;
        buf.extend_from_slice(&r_.to_be_bytes());
    }
}
