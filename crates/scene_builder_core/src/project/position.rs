use serde::{Deserialize, Serialize};

use super::serialize::{deserialize_vec_or_string, EncodeBinary};
use crate::project::{
    define::{Offset, Sex, Stripping},
    position_info::PositionInfo,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Position {
    #[serde(deserialize_with = "deserialize_vec_or_string")]
    pub event: Vec<String>,
    pub anim_obj: String,
    pub offset: Offset,
    pub strip_data: Stripping,
    #[serde(default)] // addition 2.0
    pub climax: bool,
    #[serde(default)] // addition 2.0
    pub tags: Vec<String>,

    // Unused in .slr binary, but kept for project JSON / SLAL round-trip
    #[serde(default)]
    pub schlong: i8,
    #[serde(default)]
    pub add_cum: i32,
    #[serde(default)]
    pub open_mouth: bool,
    #[serde(default)]
    pub silent: bool,
    #[serde(default)]
    pub strap_on: bool,
    #[serde(skip_serializing, default)]
    pub extra: Extra,
    #[serde(skip_serializing, default)]
    pub sex: Sex,
    #[serde(skip_serializing, default)]
    pub race: String,
    #[serde(skip_serializing, default)]
    pub scale: f32,
}

impl Position {
    pub fn new(reference: Option<&Position>) -> Self {
        Self {
            event: Default::default(),
            offset: reference.map_or_else(|| Offset::default(), |pos| pos.offset.clone()),
            anim_obj: reference.map_or_else(|| String::new(), |pos| pos.anim_obj.clone()),
            strip_data: reference.map_or_else(|| Stripping::default(), |p| p.strip_data.clone()),
            climax: false,
            tags: Default::default(),
            // Unused in binary
            sex: Default::default(),
            race: "Human".into(),
            schlong: reference.map_or(0, |pos| pos.schlong),
            add_cum: reference.map_or(0, |pos| pos.add_cum),
            open_mouth: reference.map_or(false, |pos| pos.open_mouth),
            silent: reference.map_or(false, |pos| pos.silent),
            strap_on: reference.map_or(false, |pos| pos.strap_on),
            extra: Default::default(),
            scale: 1.0,
        }
    }

    pub fn import_offset(&mut self, yaml_obj: &serde_yaml::Mapping) -> Result<(), String> {
        let loc = yaml_obj[&"Location".into()]
            .as_sequence()
            .ok_or("Location is not a sequence")?
            .iter()
            .filter_map(|it| it.as_f64())
            .collect::<Vec<_>>();
        if loc.len() != 3 {
            return Err(format!(
                "Invalid location vector, expected length 3 but got {}",
                loc.len()
            ));
        }
        let rot = yaml_obj[&"Rotation".into()]
            .as_f64()
            .ok_or("Rotation is not a float")?;

        self.offset.x = loc[0] as f32;
        self.offset.y = loc[1] as f32;
        self.offset.z = loc[2] as f32;
        self.offset.r = rot as f32;

        Ok(())
    }

    pub fn update_to_latest_version(&mut self, old_version: u8) -> Result<(), String> {
        if old_version <= 3 {
            self.climax = self.extra.climax;
            self.tags = self.extra.custom.clone();
        }
        Ok(())
    }

    pub fn extract_position_info(&self) -> PositionInfo {
        PositionInfo {
            sex: self.sex.clone(),
            race: self.race.clone(),
            scale: self.scale,
            submissive: self.extra.submissive,
            vampire: self.extra.vampire,
            dead: self.extra.dead,
            add_cum: self.add_cum,
        }
    }
}

impl EncodeBinary for Position {
    fn get_byte_size(&self) -> usize {
        assert!(!self.event.is_empty(), "Event list should not be empty");
        self.event.first().map_or(0, |e| e.get_byte_size())
            + self.climax.get_byte_size()
            + self.offset.get_byte_size()
            + self.strip_data.get_byte_size()
            + self.tags.get_byte_size()
    }

    fn write_byte(&self, buf: &mut Vec<u8>) -> () {
        // Only save initial event, all others are called by Havok
        assert!(!self.event.is_empty(), "Event list should not be empty");
        self.event.first().unwrap().write_byte(buf);
        self.climax.write_byte(buf);
        self.offset.write_byte(buf);
        self.strip_data.write_byte(buf);
        self.tags.write_byte(buf);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Extra {
    pub submissive: bool,
    pub vampire: bool,
    pub climax: bool,
    pub dead: bool,

    #[serde(default)]
    pub custom: Vec<String>,

    // Unused fields, but kept for compatibility
    #[serde(skip_serializing, default)]
    pub handshackles: bool,
    #[serde(skip_serializing, default)]
    pub yoke: bool,
    #[serde(skip_serializing, default)]
    pub armbinder: bool,
    #[serde(skip_serializing, default)]
    pub legbinder: bool,
    #[serde(skip_serializing, default)]
    pub petsuit: bool,
    #[serde(skip_serializing, default)]
    pub optional: bool,
}
