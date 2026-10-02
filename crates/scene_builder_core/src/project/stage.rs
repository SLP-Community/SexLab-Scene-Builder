use serde::{Deserialize, Serialize};
use std::vec;

use crate::project::scene::Scene;

use super::{position::Position, serialize::EncodeBinary, NanoID};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stage {
    pub id: NanoID,
    pub name: String,

    pub positions: Vec<Position>,
    pub tags: Vec<String>,
    pub extra: Extra,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Extra {
    pub fixed_len: f32,
    pub nav_text: String,
    /// SLAL sound category (Squishing / Sucking / SexMix / none / NoSound). Project JSON only.
    #[serde(default)]
    pub sound: String,
}

impl Stage {
    pub fn new(parent_scene: &Scene) -> Self {
        let stage = parent_scene.stages.last();
        let i = parent_scene.stages.len() + 1;
        let n = i; // total after this stage is added; caller should renumber siblings
        Self {
            id: NanoID::new_nanoid(),
            name: format!("Stage {i}/{n}"),
            positions: stage.map_or_else(
                || {
                    // Empty scene has no actors yet; first stage still needs one slot.
                    let n = parent_scene.positions.len().max(1);
                    vec![Position::new(None); n]
                },
                |s| s.positions.iter().map(|p| Position::new(Some(p))).collect(),
            ),
            tags: parent_scene.tags.clone(),
            extra: stage.map_or_else(Extra::default, |s| Extra {
                fixed_len: 0.0,
                nav_text: String::new(),
                sound: s.extra.sound.clone(),
            }),
        }
    }

    /// True when `name` looks like an auto-generated `Stage i/n` label.
    pub fn is_auto_name(name: &str) -> bool {
        let Some(rest) = name.strip_prefix("Stage ") else {
            return false;
        };
        let mut parts = rest.split('/');
        let (Some(a), Some(b), None) = (parts.next(), parts.next(), parts.next()) else {
            return false;
        };
        !a.is_empty()
            && !b.is_empty()
            && a.chars().all(|c| c.is_ascii_digit())
            && b.chars().all(|c| c.is_ascii_digit())
    }

    /// Refresh `Stage i/n` labels so `i` is 1-based index and `n` is the scene total.
    /// Custom names are left alone.
    pub fn renumber_auto_names(scene: &mut Scene) {
        let total = scene.stages.len();
        for (idx, stage) in scene.stages.iter_mut().enumerate() {
            if Self::is_auto_name(&stage.name) {
                stage.name = format!("Stage {}/{}", idx + 1, total);
            }
        }
    }

    pub fn import_offset(&mut self, yaml_obj: &serde_yaml::Sequence) -> Result<(), String> {
        let list: Vec<_> = yaml_obj
            .iter()
            .map_while(|obj| {
                obj.as_mapping().and_then(|mapping| {
                    mapping
                        .get(&"transform".into())
                        .and_then(|obj| obj.as_mapping())
                })
            })
            .collect();
        if list.len() != self.positions.len() {
            return Err(format!(
                "Invalid position length, got {} but exepcted {}",
                list.len(),
                self.positions.len(),
            ));
        }
        for (i, pos_obj) in list.iter().enumerate() {
            self.positions[i].import_offset(*pos_obj)?;
        }

        Ok(())
    }

    pub fn update_to_latest_version(&mut self, old_version: u8) -> Result<(), String> {
        for pos in &mut self.positions {
            pos.update_to_latest_version(old_version)?;
        }
        Ok(())
    }
}

impl EncodeBinary for Stage {
    fn get_byte_size(&self) -> usize {
        self.id.get_byte_size()
            + self.positions.get_byte_size()
            + self.extra.fixed_len.get_byte_size()
            + self.extra.nav_text.get_byte_size()
            + self.tags.get_byte_size()
    }

    fn write_byte(&self, buf: &mut Vec<u8>) -> () {
        self.id.write_byte(buf);
        self.positions.write_byte(buf);
        self.extra.fixed_len.write_byte(buf);
        self.extra.nav_text.write_byte(buf);
        self.tags
            .iter()
            .map(|tag| {
                tag.chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<String>()
                    .to_lowercase()
            })
            .collect::<Vec<_>>()
            .write_byte(buf);
    }
}

impl PartialEq for Stage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
