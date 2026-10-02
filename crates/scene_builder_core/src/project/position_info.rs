use super::serialize::EncodeBinary;
use crate::project::define::Sex;
use crate::racekeys::get_race_key_bytes;
use serde::{Deserialize, Serialize};
use std::mem::size_of;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionInfo {
    pub sex: Sex,
    pub race: String,
    pub scale: f32,
    pub submissive: bool,
    pub vampire: bool,
    pub dead: bool,
    /// SLAL actor add_cum bitfield (JSON project only; not in .slr binary)
    #[serde(default)]
    pub add_cum: i32,
}

impl EncodeBinary for PositionInfo {
    fn get_byte_size(&self) -> usize {
        self.sex.get_byte_size() +
        self.race.get_byte_size() +
        size_of::<bool>() * 3 + // submissive, vampire, dead
        size_of::<f32>() // scale
    }

    fn write_byte(&self, buf: &mut Vec<u8>) -> () {
        buf.push(get_race_key_bytes(&self.race).unwrap());
        self.sex.write_byte(buf);
        self.scale.write_byte(buf);
        buf.push(
            (1 << 0) * self.submissive as u8
                + (1 << 1) * self.vampire as u8
                + (1 << 2) * self.dead as u8,
        );
    }
}

impl Default for PositionInfo {
    fn default() -> Self {
        PositionInfo {
            sex: Default::default(),
            race: String::from("Human"),
            scale: 1.0,
            submissive: false,
            vampire: false,
            dead: false,
            add_cum: 0,
        }
    }
}
