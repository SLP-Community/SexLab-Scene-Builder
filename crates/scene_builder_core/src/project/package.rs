use log::info;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufReader, BufWriter, Write},
    mem::size_of,
    path::{Path, PathBuf},
    vec,
};
use crate::{
    project::{
        define::{Node, Sex},
        position::Position,
        serialize::{make_fnis_lines, make_fnis_lines_slal_sequence, map_race_to_folder},
        fnis_list::{
            lookup_fnis_objects, objects_to_anim_obj as fnis_objects_to_anim_obj,
            parse_fnis_list_file, FnisAnimObjects,
        },
        slanim_source::{objects_to_anim_obj, parse_slanim_source_file, SourceAnim},
    },
    racekeys::{map_legacy_to_racekey, map_racekey_to_legacy},
};

use super::{scene::Scene, serialize::EncodeBinary, stage::Stage, NanoID};

const VERSION: u8 = 4; // current version

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Slsb,
    Slal,
    Both,
}

#[derive(Debug, Default, Clone)]
pub struct EnrichSummary {
    pub files: usize,
    pub animations_in_source: usize,
    pub scenes_enriched: usize,
    pub positions_updated: usize,
    pub unmatched_ids: Vec<String>,
}

impl EnrichSummary {
    pub fn message(&self) -> String {
        self.message_labeled("Source files", "Animations in source")
    }

    pub fn message_fnis(&self) -> String {
        self.message_labeled("AnimList files", "Events with AnimObjects")
    }

    fn message_labeled(&self, files_label: &str, anims_label: &str) -> String {
        let mut msg = format!(
            "{}: {}\n{}: {}\nScenes enriched: {}\nPositions updated: {}",
            files_label,
            self.files,
            anims_label,
            self.animations_in_source,
            self.scenes_enriched,
            self.positions_updated
        );
        if !self.unmatched_ids.is_empty() {
            let preview: Vec<_> = self
                .unmatched_ids
                .iter()
                .take(12)
                .cloned()
                .collect();
            msg.push_str(&format!(
                "\nUnmatched ({}): {}",
                self.unmatched_ids.len(),
                preview.join(", ")
            ));
            if self.unmatched_ids.len() > 12 {
                msg.push_str(", …");
            }
        }
        msg
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    #[serde(default)]
    pub version: u8,
    #[serde(skip)]
    pub pack_path: PathBuf,

    pub pack_name: String,
    pub pack_author: String,
    #[serde(default)]
    pub pack_version: String,
    pub prefix_hash: NanoID,
    pub scenes: IndexMap<NanoID, Scene>,
}

impl Package {
    pub fn new() -> Self {
        Self {
            version: VERSION, // current version
            pack_path: Default::default(),
            pack_name: Default::default(),
            pack_author: Default::default(),
            pack_version: Default::default(),
            prefix_hash: NanoID::new_prefix(),
            scenes: IndexMap::new(),
        }
    }

    pub fn from_file(file: std::fs::File) -> Result<Package, String> {
        serde_json::from_reader(BufReader::new(file))
            .map_err(|e| e.to_string())
            .and_then(|mut package: Package| {
                if package.version < VERSION {
                    package.update_to_latest_version()?;
                }
                info!("Loaded project {}", package.pack_name);
                Ok(package)
            })
    }

    fn update_to_latest_version(&mut self) -> Result<(), String> {
        for (_, scene) in &mut self.scenes {
            if let Err(e) = scene.update_to_latest_version(self.version) {
                return Err(format!("Failed to update scene {}: {}", scene.id.0, e));
            }
        }
        self.version = VERSION;
        Ok(())
    }

    pub fn reset(&mut self) -> &Self {
        *self = Self::new();
        self
    }

    pub fn save_scene(&mut self, scene: Scene) -> &Scene {
        let id = scene.id.clone();
        info!("Saving or inserting Scene: {} / {}", id.0, scene.name);
        self.scenes.insert(id.clone(), scene);
        self.scenes.get(&id).unwrap()
    }

    pub fn discard_scene(&mut self, id: &NanoID) -> Option<Scene> {
        self.scenes.shift_remove(id).map(|s| {
            info!("Deleting Scene: {} / {}", id.0, s.name);
            s
        })
    }

    pub fn get_scene(&self, id: &NanoID) -> Option<&Scene> {
        self.scenes.get(id)
    }

    pub fn get_scene_mut(&mut self, id: &NanoID) -> Option<&mut Scene> {
        self.scenes.get_mut(id)
    }

    pub fn get_stage(&self, id: &NanoID) -> Option<&Stage> {
        for (_, scene) in &self.scenes {
            let stage = scene.get_stage(id);
            if stage.is_some() {
                return stage;
            }
        }
        None
    }

    pub fn load_from_path(path: PathBuf) -> Result<Package, String> {
        let mut pack = Package::from_file(fs::File::open(&path).map_err(|e| e.to_string())?)?;
        pack.pack_path = path;
        Ok(pack)
    }

    pub fn write(&mut self, path: PathBuf) -> Result<(), String> {
        let file = fs::File::create(&path).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(BufWriter::new(file), self)
            .map_err(|e| e.to_string())?;
        self.pack_path = path;
        println!("Saved project {}", self.pack_name);
        Ok(())
    }

    pub fn enrich_from_slanim_paths(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<EnrichSummary, String> {
        let mut summary = EnrichSummary {
            files: paths.len(),
            ..Default::default()
        };
        let mut source_anims: Vec<SourceAnim> = Vec::new();
        for path in paths {
            let parsed = parse_slanim_source_file(path)?;
            summary.animations_in_source += parsed.len();
            source_anims.extend(parsed);
        }

        for anim in &source_anims {
            let mut matched = false;
            for scene in self.scenes.values_mut() {
                if !scene_matches_source(scene, anim) {
                    continue;
                }
                matched = true;
                let updated = apply_source_objects(scene, anim);
                if updated > 0 {
                    summary.scenes_enriched += 1;
                    summary.positions_updated += updated;
                }
            }
            if !matched {
                summary.unmatched_ids.push(anim.id.clone());
            }
        }
        Ok(summary)
    }

    pub fn enrich_from_fnis_paths(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<EnrichSummary, String> {
        let mut summary = EnrichSummary {
            files: paths.len(),
            ..Default::default()
        };
        let mut merged: HashMap<String, FnisAnimObjects> = HashMap::new();
        for path in paths {
            let parsed = parse_fnis_list_file(path)?;
            for (event, objs) in parsed {
                merged.insert(event, objs);
            }
        }
        summary.animations_in_source = merged.len();

        let mut matched_events: HashSet<String> = HashSet::new();
        for scene in self.scenes.values_mut() {
            let updated = apply_fnis_objects(scene, &merged, &mut matched_events);
            if updated > 0 {
                summary.scenes_enriched += 1;
                summary.positions_updated += updated;
            }
        }
        for event in merged.keys() {
            if !matched_events.contains(event) {
                summary.unmatched_ids.push(event.clone());
            }
        }
        summary.unmatched_ids.sort();
        Ok(summary)
    }

    /// Import a SLAL pack folder: locate JSON, then auto-enrich FNIS lists and sources.txt.
    pub fn from_slal_pack(
        dir: PathBuf,
        progress: Option<&dyn crate::Progress>,
    ) -> Result<Package, String> {
        if !dir.is_dir() {
            return Err(format!(
                "SLAL pack path is not a folder: {}",
                dir.display()
            ));
        }
        if let Some(p) = progress {
            p.set_message("Reading SLAL JSON…");
        }
        let json_path = find_slal_json(&dir)?;
        let mut prjct = Package::from_slal(json_path.clone())?;

        let mut fnis_lists = Vec::new();
        let mut source_files = Vec::new();
        if let Some(p) = progress {
            p.set_message("Scanning FNIS lists and sources…");
            p.set_fraction(0.25);
        }
        collect_slal_enrich_files(&dir, &mut fnis_lists, &mut source_files)?;

        if !fnis_lists.is_empty() {
            if let Some(p) = progress {
                p.set_message("Enriching from FNIS lists…");
                p.set_fraction(0.5);
            }
            let summary = prjct.enrich_from_fnis_paths(&fnis_lists)?;
            println!(
                "FNIS enrich: {} list(s), {} event(s), {} position(s) updated, {} unmatched",
                summary.files,
                summary.animations_in_source,
                summary.positions_updated,
                summary.unmatched_ids.len()
            );
        }

        if !source_files.is_empty() {
            if let Some(p) = progress {
                p.set_message("Enriching from SLAnim sources…");
                p.set_fraction(0.75);
            }
            let summary = prjct.enrich_from_slanim_paths(&source_files)?;
            println!(
                "SLAnim sources enrich: {} file(s), {} anim(s), {} scene(s) enriched, {} unmatched",
                summary.files,
                summary.animations_in_source,
                summary.scenes_enriched,
                summary.unmatched_ids.len()
            );
        }

        if let Some(p) = progress {
            p.set_fraction(1.0);
            p.set_message("Done");
        }
        println!(
            "SLAL pack import: JSON {}, FNIS lists {}, sources {} under {}",
            json_path.display(),
            fnis_lists.len(),
            source_files.len(),
            dir.display()
        );
        Ok(prjct)
    }

    pub fn from_slal(path: PathBuf) -> Result<Package, String> {
        let file = fs::File::open(&path).map_err(|e| e.to_string())?;

        let slal: serde_json::Value =
            serde_json::from_reader(BufReader::new(file)).map_err(|e| e.to_string())?;

        let mut prjct = Package::new();
        prjct.version = 0; // SLAL files are always version 0
        // Convert script sets SLAL "name" to the FNIS anim dir (e.g. BPAnims).
        prjct.pack_name = slal["name"]
            .as_str()
            .ok_or("Missing name attribute")?
            .into();

        let anims = slal["animations"]
            .as_array()
            .ok_or("Missing animations attribute")?;
        for animation in anims {
            let mut scene = Scene::default();
            scene.name = animation["name"]
                .as_str()
                .ok_or("Missing name attribute")?
                .into();
            let crt_race = animation["creature_race"].as_str().unwrap_or_default();
            let actors = animation["actors"]
                .as_array()
                .ok_or("Missing actors attribute")?;

            // initialize stages and copy information for every position into the respective stage
            for (n, position) in actors.iter().enumerate() {
                let sex = position["type"].as_str().unwrap_or("male").to_lowercase();
                let events = position["stages"]
                    .as_array()
                    .ok_or("Missing stages attribute")?;
                let actor_add_cum = position
                    .get("add_cum")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let mut actor_strap_on = false;

                if scene.stages.is_empty() {
                    for _ in 0..events.len() {
                        scene.stages.push(Stage::new(&scene));
                    }
                    if scene.stages.is_empty() {
                        return Err("Scene has no stages".into());
                    }
                    for stage in &mut scene.stages {
                        stage.positions = vec![Position::new(None); actors.len()];
                    }
                }
                for (i, evt) in events.iter().enumerate() {
                    let edit_position = &mut scene.stages[i].positions[n];
                    edit_position.event =
                        vec![evt["id"].as_str().ok_or("Missing id attribute")?.into()];
                    edit_position.add_cum = actor_add_cum;
                    // SLAL stage offsets (Skyrim: X=side, Y=forward, Z=up)
                    if let Some(v) = evt.get("side").and_then(|v| v.as_f64()) {
                        edit_position.offset.x = v as f32;
                    }
                    if let Some(v) = evt.get("forward").and_then(|v| v.as_f64()) {
                        edit_position.offset.y = v as f32;
                    }
                    if let Some(v) = evt.get("up").and_then(|v| v.as_f64()) {
                        edit_position.offset.z = v as f32;
                    }
                    if let Some(v) = evt.get("rotate").and_then(|v| v.as_f64()) {
                        edit_position.offset.r = v as f32;
                    }
                    if evt.get("open_mouth").and_then(|v| v.as_bool()) == Some(true) {
                        edit_position.open_mouth = true;
                    }
                    if evt.get("silent").and_then(|v| v.as_bool()) == Some(true) {
                        edit_position.silent = true;
                    }
                    if let Some(sos) = evt.get("sos").and_then(|v| v.as_i64()) {
                        // Persist in project JSON (not .slr); mirrors convert tracking
                        edit_position.schlong = sos.clamp(i8::MIN as i64, i8::MAX as i64) as i8;
                    }
                    // strap_on may be bool or string in the wild
                    if evt.get("strap_on").and_then(|v| v.as_bool()) == Some(true)
                        || evt.get("strap_on").and_then(|v| v.as_str()) == Some("True")
                        || evt.get("strap_on").and_then(|v| v.as_str()) == Some("true")
                    {
                        edit_position.strap_on = true;
                        actor_strap_on = true;
                    }
                    match sex.as_str() {
                        "male" | "type" => {
                            edit_position.sex = Sex {
                                male: true,
                                female: false,
                                futa: false,
                            };
                            edit_position.race = "Human".into();
                        }
                        "female" => {
                            edit_position.sex = Sex {
                                male: false,
                                female: true,
                                futa: false,
                            };
                            edit_position.race = "Human".into();
                        }
                        "creaturemale" => {
                            edit_position.sex = Sex {
                                male: true,
                                female: false,
                                futa: false,
                            };
                            edit_position.race = map_legacy_to_racekey(
                                position["race"].as_str().unwrap_or(crt_race),
                            )?;
                        }
                        "creaturefemale" => {
                            edit_position.sex = Sex {
                                male: false,
                                female: true,
                                futa: false,
                            };
                            edit_position.race = map_legacy_to_racekey(
                                position["race"].as_str().unwrap_or(crt_race),
                            )?;
                        }
                        _ => {
                            return Err(format!("Unrecognized gender: {}", sex));
                        }
                    }
                }
                // slsb-convert.py: strap_on → sex.futa on that human slot
                if actor_strap_on {
                    for stage in &mut scene.stages {
                        if let Some(pos) = stage.positions.get_mut(n) {
                            pos.sex.futa = true;
                        }
                    }
                }
            }
            // finalize stage data, adding climax to last positions
            let tags = animation["tags"]
                .as_str()
                .and_then(|tags| {
                    let list = tags
                        .to_lowercase()
                        .split(',')
                        .map(|str| str.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>();
                    Some(list)
                })
                .unwrap_or_default();
            if let Some(anim_sound) = animation.get("sound").and_then(|v| v.as_str()) {
                let sound = anim_sound.trim();
                if !sound.is_empty() {
                    for stage in &mut scene.stages {
                        if stage.extra.sound.is_empty() {
                            stage.extra.sound = sound.to_string();
                        }
                    }
                }
            }
            let stage_extra = animation
                .get("stages")
                .or_else(|| animation.get("stage"))
                .and_then(|v| v.as_array());
            for (i, stage) in scene.stages.iter_mut().enumerate() {
                stage.tags = tags.clone();
                if let Some(extra_vec) = stage_extra {
                    for extra in extra_vec {
                        let n = extra["number"].as_i64().unwrap_or(-1);
                        // SLAL uses 1-based stage numbers; accept 0-based as a fallback
                        if n != (i as i64 + 1) && n != i as i64 {
                            continue;
                        }
                        // SLAL timer is seconds; SLSB / convert store milliseconds
                        let timer_sec = extra["timer"].as_f64().unwrap_or_default();
                        if timer_sec > 0.0 {
                            stage.extra.fixed_len = (timer_sec * 1000.0).round() as f32;
                        }
                        if let Some(sound) = extra.get("sound").and_then(|v| v.as_str()) {
                            let sound = sound.trim();
                            if !sound.is_empty() {
                                stage.extra.sound = sound.to_string();
                            }
                        }
                    }
                }
            }
            scene.tags = tags;
            // build graph
            scene.root = scene.stages[0].id.clone();
            let mut prev_id: Option<NanoID> = None;
            for stage in scene.stages.iter_mut().rev() {
                let mut value = Node::default();
                if let Some(id) = prev_id {
                    value.dest = vec![id];
                }
                scene.graph.insert(stage.id.clone(), value);
                prev_id = Some(stage.id.clone());
            }
            // Mark climax after graph build (and keep top-level field for IPC;
            // extra.climax is skip_serializing).
            if let Some(last) = scene.stages.last_mut() {
                for position in &mut last.positions {
                    position.climax = true;
                    position.extra.climax = true;
                }
            }
            Stage::renumber_auto_names(&mut scene);
            // add to prjct
            prjct.scenes.insert(scene.id.clone(), scene);
        }
        println!(
            "Loaded {} Animations from {}",
            prjct.scenes.len(),
            path.to_str().unwrap_or_default()
        );
        prjct.update_to_latest_version()?;
        Ok(prjct)
    }

    /// Resolve write roots under `{parent_folder}/{PackName}/`.
    pub fn resolve_export_paths(
        &self,
        parent_folder: &Path,
        kind: ExportKind,
    ) -> (PathBuf, Vec<PathBuf>) {
        let pack_root = parent_folder.join(self.fnis_mod_name());
        let write_roots = match kind {
            ExportKind::Slsb | ExportKind::Slal => vec![pack_root.clone()],
            ExportKind::Both => {
                // SLSB and SLAL FNIS list formats clash in the same tree
                vec![pack_root.join("SLSB"), pack_root.join("SLAL")]
            }
        };
        (pack_root, write_roots)
    }

    pub fn export_into(&self, pack_root: &Path, kind: ExportKind) -> Result<(), String> {
        match kind {
            ExportKind::Slsb => self.build(pack_root.to_path_buf()).map_err(|e| e.to_string()),
            ExportKind::Slal => self.write_slal_pack(&pack_root.to_path_buf()),
            ExportKind::Both => {
                self.build(pack_root.join("SLSB")).map_err(|e| e.to_string())?;
                self.write_slal_pack(&pack_root.join("SLAL"))
            }
        }
    }

    pub fn build(&self, root_dir: PathBuf) -> Result<(), std::io::Error> {
        println!("Compiling project {}", self.pack_name);
        self.write_pack_merged(&root_dir, |staging| {
            self.write_binary_file(staging)?;
            self.write_fnis_files_slsb(staging)?;
            self.generate_behaviors(staging)?;
            Ok(())
        })?;
        info!(
            "Successfully compiled {}",
            root_dir.to_str().unwrap_or_default()
        );
        Ok(())
    }

    fn generate_behaviors(&self, root_dir: &PathBuf) -> Result<(), std::io::Error> {
        match crate::project::behavior_gen::generate_behaviors_under(root_dir) {
            Ok(paths) => {
                info!("Generated {} behavior file(s)", paths.len());
                Ok(())
            }
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
        }
    }

    /// Stage into a sibling dir, then soft-merge into `root_dir` (keep extras like .hkx).
    fn write_pack_merged<F>(&self, root_dir: &PathBuf, write: F) -> Result<(), std::io::Error>
    where
        F: FnOnce(&PathBuf) -> Result<(), std::io::Error>,
    {
        let parent = root_dir.parent().unwrap_or_else(|| Path::new("."));
        let name = root_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("pack");
        let staging = parent.join(format!(
            ".{name}.slsb-staging-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&staging);
        fs::create_dir_all(&staging)?;
        match write(&staging) {
            Ok(()) => {
                let result = merge_dir_contents(&staging, root_dir);
                let _ = fs::remove_dir_all(&staging);
                result
            }
            Err(e) => {
                let _ = fs::remove_dir_all(&staging);
                Err(e)
            }
        }
    }

    pub fn write_slal_pack(&self, root_dir: &PathBuf) -> Result<(), String> {
        self.write_pack_merged(root_dir, |staging| {
            self.write_slal(staging).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e)
            })?;
            self.write_fnis_files_slal(staging).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e)
            })?;
            self.generate_behaviors(staging)?;
            Ok(())
        })
        .map_err(|e| e.to_string())
    }

    pub fn write_slal(&self, root_dir: &PathBuf) -> Result<(), String> {
        let branching = self.branching_scene_names();
        if !branching.is_empty() {
            return Err(format!(
                "SLAL export requires linear stage chains. Branching scenes: {}",
                branching.join(", ")
            ));
        }

        let animations = self
            .scenes
            .values()
            .filter(|s| !s.has_warnings && !s.stages.is_empty())
            .map(|scene| scene_to_slal_animation(scene))
            .collect::<Result<Vec<_>, _>>()?;

        let pack_id = self.fnis_mod_name();

        let root = serde_json::json!({
            "name": pack_id,
            "animations": animations,
        });

        let target_dir = root_dir.join("SLAnims").join("json");
        fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
        let out_path = target_dir.join(format!("{}.json", pack_id));
        let file = fs::File::create(&out_path).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(BufWriter::new(file), &root).map_err(|e| e.to_string())?;
        info!("Wrote SLAL JSON to {}", out_path.display());
        Ok(())
    }

    pub fn branching_scene_names(&self) -> Vec<String> {
        self.scenes
            .values()
            .filter(|scene| scene.graph.values().any(|node| node.dest.len() > 1))
            .map(|scene| {
                if scene.name.is_empty() {
                    scene.id.0.clone()
                } else {
                    scene.name.clone()
                }
            })
            .collect()
    }

    pub fn import_offset_from_path(&mut self, path: PathBuf) -> Result<(), String> {
        let file = fs::File::open(&path).map_err(|e| e.to_string())?;
        let offsetfile: serde_yaml::Mapping =
            serde_yaml::from_reader(BufReader::new(file)).map_err(|e| e.to_string())?;

        for (scene_id_v, stages_v) in offsetfile {
            if !stages_v.is_mapping() {
                continue;
            }
            let scene_id = scene_id_v
                .as_str()
                .ok_or("Not a valid offset file, expected string for scene id".to_string())?
                .to_string();
            if let Some(scene) = self.get_scene_mut(&NanoID(scene_id.clone())) {
                scene.import_offset(
                    stages_v
                        .as_mapping()
                        .ok_or(format!("Expected mapping in scene {}", scene_id))?,
                )?;
            }
        }

        Ok(())
    }

    fn write_binary_file(&self, root_dir: &PathBuf) -> Result<(), std::io::Error> {
        let target_dir = root_dir.join("SKSE").join("SexLab").join("Registry");
        let project_name = format!("{}.slr", self.slr_file_stem());
        let mut buf: Vec<u8> = Vec::new();
        buf.reserve(self.get_byte_size());
        info!(
            "Writing binary file for project {} with size {} at {}",
            project_name,
            buf.capacity(),
            target_dir.to_str().unwrap_or("Unknown path")
        );
        self.write_byte(&mut buf);
        fs::create_dir_all(&target_dir)?;
        fs::File::create(target_dir.join(project_name))?.write(&buf)?;
        Ok(())
    }

    fn write_fnis_files_slsb(&self, root_dir: &PathBuf) -> Result<(), std::io::Error> {
        let mut events: HashMap<&str, Vec<String>> = HashMap::new(); // map<RaceKey, Lines[]>
        let mut control: HashSet<&str> = HashSet::from(["__BLANK__", "__DEFAULT__"]);
        for (_, scene) in &self.scenes {
            if scene.has_warnings {
                continue;
            }
            assert_eq!(
                scene
                    .stages
                    .first()
                    .expect(&format!("Scene {} has 0 Stages", scene.id.0))
                    .positions
                    .len(),
                scene.positions.len()
            );
            for stage in &scene.stages {
                for i in 0..stage.positions.len() {
                    let stage_position = &stage.positions[i];
                    let scene_position = &scene.positions[i];
                    let event = &stage_position.event[0];
                    if control.contains(event.as_str()) {
                        continue;
                    }
                    control.insert(event);
                    let lines = make_fnis_lines(
                        &stage_position.event,
                        &self.prefix_hash.0,
                        stage.extra.fixed_len > 0.0,
                        &split_anim_objs(&stage_position.anim_obj),
                    );
                    insert_fnis_race_lines(&mut events, scene_position.race.as_str(), lines);
                }
            }
        }
        flush_fnis_lists(root_dir, &self.fnis_mod_name(), &events, false)
    }

    fn write_fnis_files_slal(&self, root_dir: &PathBuf) -> Result<(), String> {
        let branching = self.branching_scene_names();
        if !branching.is_empty() {
            return Err(format!(
                "SLAL FNIS export requires linear stage chains. Branching scenes: {}",
                branching.join(", ")
            ));
        }

        let mut events: HashMap<String, Vec<String>> = HashMap::new();
        for scene in self
            .scenes
            .values()
            .filter(|s| !s.has_warnings && !s.stages.is_empty())
        {
            let stages = linear_stage_order(scene)?;
            let anim_id = sanitize_anim_id(&scene.name, &format!("Scene_{}", scene.id.0));

            for actor_idx in 0..scene.positions.len() {
                let race = scene.positions[actor_idx].race.clone();
                let mut stage_rows: Vec<(String, Vec<String>, bool)> = Vec::new();
                for stage in &stages {
                    let pos = stage.positions.get(actor_idx).ok_or_else(|| {
                        format!(
                            "Scene '{}' missing position {} on a stage",
                            anim_id,
                            actor_idx + 1
                        )
                    })?;
                    let event = pos
                        .event
                        .first()
                        .cloned()
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| {
                            format!(
                                "Scene '{}' actor {} has an empty anim event",
                                anim_id,
                                actor_idx + 1
                            )
                        })?;
                    if event == "__BLANK__" || event == "__DEFAULT__" {
                        stage_rows.clear();
                        break;
                    }
                    stage_rows.push((
                        event,
                        split_anim_objs(&pos.anim_obj),
                        stage.extra.fixed_len > 0.0,
                    ));
                }
                if stage_rows.is_empty() {
                    continue;
                }
                let lines =
                    make_fnis_lines_slal_sequence(&anim_id, &stage_rows);
                insert_fnis_race_lines_owned(&mut events, race.as_str(), lines);
            }
        }

        let borrowed: HashMap<&str, Vec<String>> = events
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        flush_fnis_lists(root_dir, &self.fnis_mod_name(), &borrowed, true).map_err(|e| e.to_string())
    }

    /// FNIS / SLAL path stem: package name only (matches convert-script anim dirs).
    pub fn fnis_mod_name(&self) -> String {
        let trimmed = self.pack_name.trim();
        if trimmed.is_empty() {
            self.prefix_hash.0.clone()
        } else {
            trimmed.to_string()
        }
    }

    /// `.slr` stem: `PackName` or `PackName_Version`.
    pub fn slr_file_stem(&self) -> String {
        let base = self.fnis_mod_name();
        let ver = sanitize_slr_version(&self.pack_version);
        if ver.is_empty() {
            base
        } else {
            format!("{base}_{ver}")
        }
    }
}

pub fn merge_dir_contents(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if file_type.is_dir() {
            merge_dir_contents(&entry.path(), &to)?;
        } else if file_type.is_file() {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

pub fn dir_nonempty(path: &Path) -> bool {
    path.is_dir()
        && fs::read_dir(path)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
}

fn json_has_animations_array(path: &Path) -> bool {
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let Ok(v) = serde_json::from_reader::<_, serde_json::Value>(BufReader::new(file)) else {
        return false;
    };
    v.get("animations")
        .and_then(|a| a.as_array())
        .is_some_and(|a| !a.is_empty())
}

/// Prefer root `*.json`, then Registry Source `*.slsb.json`, then any nested SLAL JSON.
fn find_slal_json(dir: &Path) -> Result<PathBuf, String> {
    let mut root_candidates = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"))
                && json_has_animations_array(&path)
            {
                root_candidates.push(path);
            }
        }
    }
    if let Some(p) = root_candidates.into_iter().next() {
        return Ok(p);
    }

    let registry = dir
        .join("SKSE")
        .join("SexLab")
        .join("Registry")
        .join("Source");
    if registry.is_dir() {
        if let Ok(entries) = fs::read_dir(&registry) {
            let mut found = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if path.is_file()
                    && (name.ends_with(".slsb.json") || name.ends_with(".json"))
                    && json_has_animations_array(&path)
                {
                    found.push(path);
                }
            }
            if let Some(p) = found.into_iter().next() {
                return Ok(p);
            }
        }
    }

    let mut nested = Vec::new();
    collect_json_with_animations(dir, &mut nested, 0)?;
    nested.into_iter().next().ok_or_else(|| {
        format!(
            "No SLAL JSON with an animations array found under {}",
            dir.display()
        )
    })
}

fn collect_json_with_animations(
    dir: &Path,
    out: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<(), String> {
    if depth > 8 {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_with_animations(&path, out, depth + 1)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("json"))
            && json_has_animations_array(&path)
        {
            out.push(path);
        }
    }
    Ok(())
}

fn collect_slal_enrich_files(
    dir: &Path,
    fnis_lists: &mut Vec<PathBuf>,
    source_files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    collect_slal_enrich_files_rec(dir, fnis_lists, source_files, 0)
}

fn collect_slal_enrich_files_rec(
    dir: &Path,
    fnis_lists: &mut Vec<PathBuf>,
    source_files: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<(), String> {
    if depth > 10 {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_slal_enrich_files_rec(&path, fnis_lists, source_files, depth + 1)?;
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if name.starts_with("fnis_") && name.ends_with("_list.txt") {
            fnis_lists.push(path);
        } else if name == "sources.txt" || (name.contains("sources") && name.ends_with(".txt")) {
            source_files.push(path);
        }
    }
    Ok(())
}

/// Allow semver dots in `.slr` version segments.
fn sanitize_slr_version(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else if c.is_whitespace() {
                '_'
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches(|c| c == '_' || c == '.')
        .to_string();
    cleaned
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn split_anim_objs(anim_obj: &str) -> Vec<String> {
    // Accept comma-separated (SLSB JSON) and space-separated (FNIS / paste) forms.
    anim_obj
        .split(|c: char| c == ',' || c.is_whitespace())
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn fnis_race_aliases(race: &str) -> Vec<&str> {
    match race {
        "Canine" => vec!["Canine", "Dog", "Wolf"],
        "Dog" | "Wolf" => vec![race, "Canine"],
        "Chaurus" | "Chaurus Reaper" => vec!["Chaurus"],
        "Spider" | "Large Spider" | "Giant Spider" => vec!["Spider"],
        "Boar" | "Boar (Mounted)" | "Boar (Any)" => vec!["Boar (Any)"],
        _ => vec![race],
    }
}

fn insert_fnis_race_lines<'a>(
    events: &mut HashMap<&'a str, Vec<String>>,
    race: &'a str,
    lines: Vec<String>,
) {
    for key in fnis_race_aliases(race) {
        events
            .entry(key)
            .and_modify(|list| list.extend(lines.iter().cloned()))
            .or_insert_with(|| lines.clone());
    }
}

fn insert_fnis_race_lines_owned(
    events: &mut HashMap<String, Vec<String>>,
    race: &str,
    lines: Vec<String>,
) {
    for key in fnis_race_aliases(race) {
        events
            .entry(key.to_string())
            .and_modify(|list| list.extend(lines.iter().cloned()))
            .or_insert_with(|| lines.clone());
    }
}

fn flush_fnis_lists(
    root_dir: &PathBuf,
    pack_name: &str,
    events: &HashMap<&str, Vec<String>>,
    slal_header: bool,
) -> Result<(), std::io::Error> {
    info!("---------------------------------------------------------");
    for (racekey, anim_events) in events {
        let target_folder = map_race_to_folder(racekey)
            .expect(format!("Cannot find folder for RaceKey {}", racekey).as_str());
        // Join component-wise so Linux and Windows both get meshes/actors/<race>/...
        // (a single "meshes\\actors\\..." string is one path segment on Unix).
        let mut path = root_dir.join("meshes").join("actors");
        for part in target_folder.split(['/', '\\']).filter(|p| !p.is_empty()) {
            path.push(part);
        }
        path.push("animations");
        path.push(pack_name);
        let crt = Path::new(&target_folder)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(target_folder.as_str());
        fs::create_dir_all(&path)?;

        let create = |file_path: PathBuf| -> Result<(), std::io::Error> {
            let name = file_path.to_str().unwrap_or("NONE").to_string();
            let file = fs::File::create(file_path)?;
            let mut file = BufWriter::new(file);
            // FNIS / Pandora AnimLists on Windows are CRLF; LF-only lists are often ignored.
            if slal_header {
                write_fnis_line(&mut file, "Version V1.0")?;
                write_fnis_line(&mut file, "")?;
            }
            info!(
                "Adding {} lines to race {} |||||| file: {}",
                anim_events.len(),
                racekey,
                name
            );
            for anim_event in anim_events {
                write_fnis_line(&mut file, anim_event)?;
            }
            Ok(())
        };
        match crt {
            "character" => create(path.join(format!("FNIS_{}_List.txt", pack_name))),
            "canine" => match *racekey {
                "Canine" => create(path.join(format!("FNIS_{}_canine_List.txt", pack_name))),
                "Dog" => create(path.join(format!("FNIS_{}_dog_List.txt", pack_name))),
                _ => create(path.join(format!("FNIS_{}_wolf_List.txt", pack_name))),
            },
            _ => create(path.join(format!("FNIS_{}_{}_List.txt", pack_name, crt))),
        }?;
    }
    info!("---------------------------------------------------------");
    Ok(())
}

fn write_fnis_line(file: &mut BufWriter<fs::File>, line: &str) -> Result<(), std::io::Error> {
    file.write_all(line.as_bytes())?;
    file.write_all(b"\r\n")?;
    Ok(())
}

fn sanitize_anim_id(name: &str, fallback: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn scene_matches_source(scene: &Scene, anim: &SourceAnim) -> bool {
    if !anim.name.is_empty() {
        if scene.name.eq_ignore_ascii_case(&anim.name) {
            return true;
        }
        // SLAnim name prefixes (e.g. "Billyy ") are applied in JSON display names
        if scene
            .name
            .to_ascii_lowercase()
            .ends_with(&anim.name.to_ascii_lowercase())
        {
            return true;
        }
    }
    let sanitized = sanitize_anim_id(&scene.name, "");
    if !sanitized.is_empty() && sanitized == anim.id {
        return true;
    }
    let prefix = format!("{}_A", anim.id);
    for stage in &scene.stages {
        for pos in &stage.positions {
            if pos
                .event
                .iter()
                .any(|e| e == &anim.id || e.starts_with(&prefix))
            {
                return true;
            }
        }
    }
    false
}

fn apply_source_objects(scene: &mut Scene, anim: &SourceAnim) -> usize {
    let mut updated = 0;
    let actor_count = scene
        .stages
        .first()
        .map(|s| s.positions.len())
        .unwrap_or(0);
    for actor_idx in 0..actor_count {
        let src_actor = actor_idx + 1;
        let default_obj = anim.actor_objects.get(&src_actor);
        let stage_map = anim.stage_objects.get(&src_actor);
        for (si, stage) in scene.stages.iter_mut().enumerate() {
            let stage_num = si + 1;
            let obj = stage_map
                .and_then(|m| m.get(&stage_num))
                .or(default_obj);
            let Some(obj) = obj else {
                continue;
            };
            if obj.is_empty() {
                continue;
            }
            let Some(pos) = stage.positions.get_mut(actor_idx) else {
                continue;
            };
            let converted = objects_to_anim_obj(obj);
            if pos.anim_obj != converted {
                pos.anim_obj = converted;
                updated += 1;
            }
        }
    }
    updated
}

/// Apply AnimObjects from FNIS lists onto matching stage positions (overwrites when found).
fn apply_fnis_objects(
    scene: &mut Scene,
    map: &HashMap<String, FnisAnimObjects>,
    matched_events: &mut HashSet<String>,
) -> usize {
    let mut updated = 0;
    for stage in &mut scene.stages {
        for pos in &mut stage.positions {
            let Some(event) = pos.event.first().filter(|e| !e.is_empty()) else {
                continue;
            };
            let Some((map_key, objs)) = lookup_fnis_objects(map, event) else {
                continue;
            };
            matched_events.insert(map_key.to_string());
            let converted = fnis_objects_to_anim_obj(&objs.anim_objs);
            if converted.is_empty() {
                continue;
            }
            if pos.anim_obj != converted {
                pos.anim_obj = converted;
                updated += 1;
            }
        }
    }
    updated
}

fn linear_stage_order(scene: &Scene) -> Result<Vec<&Stage>, String> {
    let label = if scene.name.is_empty() {
        scene.id.0.as_str()
    } else {
        scene.name.as_str()
    };

    for node in scene.graph.values() {
        if node.dest.len() > 1 {
            return Err(format!(
                "Scene '{}' has branching stages (SLAL export requires a linear chain)",
                label
            ));
        }
    }

    if scene.graph.is_empty() {
        return Ok(scene.stages.iter().collect());
    }

    let mut ordered = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut current = Some(scene.root.clone());
    while let Some(id) = current {
        if !visited.insert(id.0.clone()) {
            return Err(format!("Scene '{}' has a cycle in its stage graph", label));
        }
        let stage = scene.get_stage(&id).ok_or_else(|| {
            format!(
                "Scene '{}' references missing stage '{}'",
                label, id.0
            )
        })?;
        ordered.push(stage);
        current = scene
            .graph
            .get(&id)
            .and_then(|n| n.dest.first().cloned());
    }

    if ordered.len() != scene.stages.len() {
        return Err(format!(
            "Scene '{}' is not a single linear chain (unreachable or disconnected stages)",
            label
        ));
    }
    Ok(ordered)
}

fn slal_actor_type(
    info: &super::position_info::PositionInfo,
) -> Result<(String, Option<String>, bool), String> {
    let is_human = info.race == "Human" || info.race.is_empty();
    let male = info.sex.male;
    let female = info.sex.female;
    let futa = info.sex.futa;

    // slsb-convert.py maps SLAL Male+strap_on onto male+futa (and often adds flexible
    // futa on humans). Do not treat futa as Female for the SLAL type string.
    if is_human {
        if male && !female {
            // Male-only, or Male+futa (convert strap_on / flexible-futa pattern)
            Ok(("Male".into(), None, futa))
        } else if female && !male {
            Ok(("Female".into(), None, false))
        } else if male && female {
            // Dual-allowed slot: legacy SLAL uses Male + strap_on
            Ok(("Male".into(), None, true))
        } else if futa {
            // Futa-only: SLAL has no futa type; Male+strap_on is the usual stand-in
            Ok(("Male".into(), None, true))
        } else {
            Err("Position has no sex assigned".into())
        }
    } else {
        let legacy = map_racekey_to_legacy(&info.race)?;
        if female && !male {
            Ok(("CreatureFemale".into(), Some(legacy), false))
        } else {
            Ok(("CreatureMale".into(), Some(legacy), false))
        }
    }
}

/// Prefer the shared `{id}_A#_S#` event prefix over a sanitized scene name.
fn derive_slal_anim_id(scene: &Scene, stages: &[&Stage]) -> String {
    let fallback = sanitize_anim_id(&scene.name, &format!("Scene_{}", scene.id.0));
    let mut derived: Option<String> = None;
    for stage in stages {
        for pos in &stage.positions {
            let Some(event) = pos.event.first().filter(|e| !e.is_empty()) else {
                continue;
            };
            let Some(id) = strip_actor_stage_suffix(event) else {
                return fallback;
            };
            match &derived {
                None => derived = Some(id),
                Some(existing) if existing == &id => {}
                Some(_) => return fallback,
            }
        }
    }
    derived.filter(|id| !id.is_empty()).unwrap_or(fallback)
}

fn strip_actor_stage_suffix(event: &str) -> Option<String> {
    // Match ..._A{n}_S{m} at end of event id
    let bytes = event.as_bytes();
    let mut i = bytes.len();
    // _S digits
    let mut saw_s = false;
    while i > 0 && bytes[i - 1].is_ascii_digit() {
        i -= 1;
        saw_s = true;
    }
    if !saw_s || i < 2 || bytes[i - 1] != b'S' || bytes[i - 2] != b'_' {
        return None;
    }
    i -= 2;
    // _A digits
    let mut saw_a = false;
    while i > 0 && bytes[i - 1].is_ascii_digit() {
        i -= 1;
        saw_a = true;
    }
    if !saw_a || i < 2 || bytes[i - 1] != b'A' || bytes[i - 2] != b'_' {
        return None;
    }
    i -= 2;
    if i == 0 {
        return None;
    }
    Some(event[..i].to_string())
}

/// Best-effort add_cum bitfield from scene tags when not stored on the position.
fn infer_add_cum(tags: &[String], actor_type: &str) -> Option<i64> {
    if actor_type != "Female" && actor_type != "CreatureFemale" {
        return None;
    }
    let lower: Vec<String> = tags.iter().map(|t| t.to_ascii_lowercase()).collect();
    let has = |keys: &[&str]| lower.iter().any(|t| keys.iter().any(|k| t == k));
    let mut bits: i64 = 0;
    if has(&["vaginal", "creampie"]) {
        bits |= 1; // Vaginal
    }
    if has(&["oral", "blowjob", "facial"]) {
        bits |= 2; // Oral
    }
    if has(&["anal"]) {
        bits |= 4; // Anal
    }
    if bits == 0 {
        None
    } else {
        Some(bits)
    }
}

fn slal_timer_seconds(fixed_len_ms: f32) -> f32 {
    // SLSB UI and slsb-convert.py store milliseconds; SLAL uses seconds
    if fixed_len_ms <= 0.0 {
        0.0
    } else {
        fixed_len_ms / 1000.0
    }
}

fn scene_to_slal_animation(scene: &Scene) -> Result<serde_json::Value, String> {
    let stages = linear_stage_order(scene)?;
    let anim_id = derive_slal_anim_id(scene, &stages);
    let anim_name = if scene.name.is_empty() {
        anim_id.clone()
    } else {
        scene.name.clone()
    };

    let mut tag_set: Vec<String> = if !scene.tags.is_empty() {
        scene.tags.clone()
    } else {
        stages
            .first()
            .map(|s| s.tags.clone())
            .unwrap_or_default()
    };
    tag_set.retain(|t| !t.is_empty());
    let tags = tag_set.join(",");

    let mut actors = Vec::new();
    let mut creature_race: Option<String> = None;
    for (actor_idx, info) in scene.positions.iter().enumerate() {
        let (actor_type, race_legacy, strap_on) = slal_actor_type(info)?;
        if let Some(ref r) = race_legacy {
            if creature_race.is_none() {
                creature_race = Some(r.clone());
            }
        }

        let mut actor_stages = Vec::new();
        for stage in &stages {
            let pos = stage.positions.get(actor_idx).ok_or_else(|| {
                format!(
                    "Scene '{}' stage '{}' missing position {}",
                    anim_name, stage.id.0, actor_idx + 1
                )
            })?;
            let event_id = pos
                .event
                .first()
                .cloned()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    format!(
                        "{}_A{}_S{}",
                        anim_id,
                        actor_idx + 1,
                        actor_stages.len() + 1
                    )
                });

            let mut stage_obj = serde_json::json!({ "id": event_id });
            let off = &pos.offset;
            if off.x != 0.0 || off.y != 0.0 || off.z != 0.0 || off.r != 0.0 {
                // Match Skyrim Location: X=side, Y=forward, Z=up
                stage_obj["side"] = serde_json::json!(off.x);
                stage_obj["forward"] = serde_json::json!(off.y);
                stage_obj["up"] = serde_json::json!(off.z);
                stage_obj["rotate"] = serde_json::json!(off.r);
            }
            if pos.open_mouth {
                stage_obj["open_mouth"] = serde_json::json!(true);
            }
            if pos.silent {
                stage_obj["silent"] = serde_json::json!(true);
            }
            if pos.strap_on || strap_on {
                stage_obj["strap_on"] = serde_json::json!(true);
            }
            if pos.schlong != 0 {
                stage_obj["sos"] = serde_json::json!(pos.schlong as i64);
            }
            if pos.event.len() > 1 {
                log::warn!(
                    "SLAL export uses only the first anim event for {} position {}",
                    anim_name,
                    actor_idx + 1
                );
            }
            actor_stages.push(stage_obj);
        }

        let mut actor = serde_json::json!({
            "type": actor_type,
            "stages": actor_stages,
        });
        if let Some(race) = race_legacy {
            actor["race"] = serde_json::json!(race);
        }
        let add_cum = if info.add_cum != 0 {
            Some(info.add_cum as i64)
        } else {
            infer_add_cum(&tag_set, &actor_type)
        };
        if let Some(cum) = add_cum {
            actor["add_cum"] = serde_json::json!(cum);
        }
        actors.push(actor);
    }

    let anim_sound = stages
        .iter()
        .map(|s| s.extra.sound.trim())
        .find(|s| !s.is_empty())
        .map(|s| s.to_string());

    let mut stage_params = Vec::new();
    for (i, stage) in stages.iter().enumerate() {
        let mut stage_param = serde_json::Map::new();
        let mut has_fields = false;
        if stage.extra.fixed_len > 0.0 {
            stage_param.insert(
                "timer".into(),
                serde_json::json!(slal_timer_seconds(stage.extra.fixed_len)),
            );
            has_fields = true;
        }
        let stage_sound = stage.extra.sound.trim();
        if !stage_sound.is_empty() {
            let differs = anim_sound
                .as_deref()
                .map(|a| a != stage_sound)
                .unwrap_or(false);
            if differs {
                stage_param.insert("sound".into(), serde_json::json!(stage_sound));
                has_fields = true;
            }
        }
        if has_fields {
            stage_param.insert("number".into(), serde_json::json!(i + 1));
            stage_params.push(serde_json::Value::Object(stage_param));
        }
        if !stage.extra.nav_text.is_empty() {
            log::warn!(
                "Dropping nav_text on scene '{}' stage {} (not supported in SLAL)",
                anim_name,
                i + 1
            );
        }
    }

    let mut anim = serde_json::json!({
        "id": anim_id,
        "name": anim_name,
        "actors": actors,
    });
    if !tags.is_empty() {
        anim["tags"] = serde_json::json!(tags);
    }
    if let Some(race) = creature_race {
        anim["creature_race"] = serde_json::json!(race);
    }
    if let Some(sound) = anim_sound {
        anim["sound"] = serde_json::json!(sound);
    }
    if !stage_params.is_empty() {
        anim["stages"] = serde_json::json!(stage_params);
    }
    if scene.private {
        log::warn!(
            "Scene '{}' is private; flag is not represented in SLAL JSON",
            anim_name
        );
    }
    Ok(anim)
}

impl EncodeBinary for Package {
    fn get_byte_size(&self) -> usize {
        self.version.get_byte_size()
            + self.pack_name.get_byte_size()
            + self.pack_author.get_byte_size()
            + self.prefix_hash.get_byte_size()
            + self
                .scenes
                .iter()
                .filter(|(_, scene)| !scene.has_warnings && !scene.stages.is_empty())
                .fold(size_of::<u64>(), |acc, (_, scene)| {
                    acc + scene.get_byte_size()
                })
    }

    fn write_byte(&self, buf: &mut Vec<u8>) -> () {
        self.version.write_byte(buf);
        self.pack_name.write_byte(buf);
        self.pack_author.write_byte(buf);
        self.prefix_hash.write_byte(buf);
        buf.extend_from_slice(&(self.scenes.len() as u64).to_be_bytes());
        self.scenes
            .iter()
            .filter(|(_, scene)| !scene.has_warnings && !scene.stages.is_empty())
            .for_each(|(_, scene)| scene.write_byte(buf));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        dir_nonempty, flush_fnis_lists, infer_add_cum, merge_dir_contents,
        scene_to_slal_animation, strip_actor_stage_suffix, write_fnis_line, Package,
    };
    use crate::project::define::Sex;
    use crate::project::position::Position;
    use crate::project::position_info::PositionInfo;
    use crate::project::scene::Scene;
    use crate::project::serialize::map_race_to_folder;
    use crate::project::stage::{Extra as StageExtra, Stage};
    use crate::project::NanoID;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    fn test_stage(id: &str, positions: Vec<Position>, sound: &str) -> Stage {
        Stage {
            id: NanoID(id.into()),
            name: String::new(),
            positions,
            tags: vec![],
            extra: StageExtra {
                fixed_len: 0.0,
                nav_text: String::new(),
                sound: sound.into(),
            },
        }
    }

    fn test_pos(event: &str) -> Position {
        let mut p = Position::new(None);
        p.event = vec![event.into()];
        p
    }

    #[test]
    fn fnis_mod_name_matches_pack_name_for_convert_workflow() {
        let mut pack = Package::new();
        pack.pack_name = "BPAnims".into();
        pack.pack_author = "Unknown".into();
        // Author must not change the FNIS folder — overlays expect animations/BPAnims/
        assert_eq!(pack.fnis_mod_name(), "BPAnims");
        assert_eq!(pack.slr_file_stem(), "BPAnims");

        pack.pack_name = "Billyy_Human".into();
        pack.pack_author = "Billyy".into();
        assert_eq!(pack.fnis_mod_name(), "Billyy_Human");

        pack.pack_name.clear();
        pack.prefix_hash = NanoID("yhd9".into());
        assert_eq!(pack.fnis_mod_name(), "yhd9");
    }

    #[test]
    fn merge_dir_contents_overwrites_and_keeps_extras() {
        let tmp = std::env::temp_dir().join(format!("slsb_merge_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let src = tmp.join("src");
        let dst = tmp.join("dst");
        fs::create_dir_all(src.join("a")).unwrap();
        fs::create_dir_all(dst.join("a")).unwrap();
        fs::write(src.join("a/file.txt"), b"new").unwrap();
        fs::write(dst.join("a/file.txt"), b"old").unwrap();
        fs::write(dst.join("a/clip.hkx"), b"keep").unwrap();
        merge_dir_contents(&src, &dst).unwrap();
        assert_eq!(fs::read(dst.join("a/file.txt")).unwrap(), b"new");
        assert_eq!(fs::read(dst.join("a/clip.hkx")).unwrap(), b"keep");
        assert!(dir_nonempty(&dst));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn pack_version_round_trips_in_project_json_only() {
        let mut pack = Package::new();
        pack.pack_name = "AnPack".into();
        pack.pack_author = "Author".into();
        pack.pack_version = "1.2.3".into();
        let json = serde_json::to_string(&pack).unwrap();
        assert!(json.contains("\"pack_version\":\"1.2.3\""));
        let loaded: Package = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.pack_version, "1.2.3");
        let legacy = r#"{
            "version": 4,
            "pack_name": "AnPack",
            "pack_author": "Author",
            "prefix_hash": "abcd",
            "scenes": {}
        }"#;
        let legacy_pack: Package = serde_json::from_str(legacy).unwrap();
        assert!(legacy_pack.pack_version.is_empty());
    }

    #[test]
    fn slr_uses_pack_name_and_optional_version() {
        let mut pack = Package::new();
        pack.pack_name = "BPAnims".into();
        pack.pack_author = "3jiou".into();
        assert_eq!(pack.fnis_mod_name(), "BPAnims");
        assert_eq!(pack.slr_file_stem(), "BPAnims");

        pack.pack_version = "1.2.3".into();
        assert_eq!(pack.fnis_mod_name(), "BPAnims");
        assert_eq!(pack.slr_file_stem(), "BPAnims_1.2.3");

        let out = std::env::temp_dir().join(format!("slsb_pack_paths_{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(&out).unwrap();
        pack.build(out.clone()).unwrap();
        let slr = out
            .join("SKSE")
            .join("SexLab")
            .join("Registry")
            .join("BPAnims_1.2.3.slr");
        assert!(slr.is_file(), "missing {}", slr.display());
        pack.write_slal(&out).unwrap();
        let slal = out
            .join("SLAnims")
            .join("json")
            .join("BPAnims.json");
        assert!(slal.is_file(), "missing {}", slal.display());
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn strips_actor_stage_event_suffix() {
        assert_eq!(
            strip_actor_stage_suffix("B_B_CGGrind_A1_S1").as_deref(),
            Some("B_B_CGGrind")
        );
        assert_eq!(
            strip_actor_stage_suffix("B_B_CGGrind_A2_S5").as_deref(),
            Some("B_B_CGGrind")
        );
        assert_eq!(strip_actor_stage_suffix("NoSuffix"), None);
    }

    #[test]
    fn infers_vaginal_add_cum_for_female() {
        let tags = vec!["Creampie".into(), "Vaginal".into()];
        assert_eq!(infer_add_cum(&tags, "Female"), Some(1));
        assert_eq!(infer_add_cum(&tags, "Male"), None);
    }

    #[test]
    fn male_with_futa_exports_as_male_strap_on() {
        let info = PositionInfo {
            sex: Sex {
                male: true,
                female: false,
                futa: true,
            },
            ..Default::default()
        };
        let (ty, _, strap) = super::slal_actor_type(&info).unwrap();
        assert_eq!(ty, "Male");
        assert!(strap);
    }

    #[test]
    fn female_with_futa_exports_as_female() {
        let info = PositionInfo {
            sex: Sex {
                male: false,
                female: true,
                futa: true,
            },
            ..Default::default()
        };
        let (ty, _, strap) = super::slal_actor_type(&info).unwrap();
        assert_eq!(ty, "Female");
        assert!(!strap);
    }

    #[test]
    fn slal_export_emits_position_flags_and_sos() {
        let mut female = test_pos("TestAnim_A1_S1");
        female.open_mouth = true;
        female.silent = true;
        let mut male = test_pos("TestAnim_A2_S1");
        male.strap_on = true;
        male.schlong = 4;

        let stage = test_stage("stg1", vec![female, male], "");
        let scene = Scene {
            name: "Test Anim".into(),
            stages: vec![stage],
            positions: vec![
                PositionInfo {
                    sex: Sex {
                        male: false,
                        female: true,
                        futa: false,
                    },
                    ..Default::default()
                },
                PositionInfo {
                    sex: Sex {
                        male: true,
                        female: false,
                        futa: false,
                    },
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let anim = scene_to_slal_animation(&scene).unwrap();
        let actors = anim["actors"].as_array().unwrap();
        let f0 = &actors[0]["stages"][0];
        assert_eq!(f0["open_mouth"], true);
        assert_eq!(f0["silent"], true);
        assert!(f0.get("strap_on").is_none());
        assert!(f0.get("sos").is_none());

        let m0 = &actors[1]["stages"][0];
        assert_eq!(m0["strap_on"], true);
        assert_eq!(m0["sos"], 4);
        assert!(m0.get("open_mouth").is_none());
    }

    #[test]
    fn slal_export_male_futa_still_writes_strap_on() {
        let male = test_pos("TestAnim_A1_S1");
        let stage = test_stage("stg1", vec![male], "");
        let scene = Scene {
            name: "Futa Slot".into(),
            stages: vec![stage],
            positions: vec![PositionInfo {
                sex: Sex {
                    male: true,
                    female: false,
                    futa: true,
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        let anim = scene_to_slal_animation(&scene).unwrap();
        assert_eq!(anim["actors"][0]["type"], "Male");
        assert_eq!(anim["actors"][0]["stages"][0]["strap_on"], true);
    }

    #[test]
    fn slal_export_emits_anim_sound_and_stage_override() {
        let s1 = test_stage("stg1", vec![test_pos("SoundAnim_A1_S1")], "Squishing");
        let s2 = test_stage("stg2", vec![test_pos("SoundAnim_A1_S2")], "Sucking");
        let scene = Scene {
            name: "Sound Test".into(),
            stages: vec![s1, s2],
            positions: vec![PositionInfo {
                sex: Sex {
                    male: false,
                    female: true,
                    futa: false,
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        let anim = scene_to_slal_animation(&scene).unwrap();
        assert_eq!(anim["sound"], "Squishing");
        let stages = anim["stages"].as_array().unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0]["number"], 2);
        assert_eq!(stages[0]["sound"], "Sucking");
    }

    #[test]
    fn enrich_from_fnis_overwrites_anim_obj_by_event() {
        let mut male = test_pos("yhd9B_Billyy_ChairDildo_A1_S1");
        male.anim_obj = String::new();
        let stage = test_stage("stg1", vec![male], "");
        let mut pack = Package::new();
        let scene = Scene {
            name: "Chair Dildo".into(),
            stages: vec![stage],
            positions: vec![PositionInfo {
                sex: Sex {
                    male: false,
                    female: true,
                    futa: false,
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        pack.scenes.insert(scene.id.clone(), scene);

        let tmp = std::env::temp_dir().join(format!(
            "slsb_fnis_enrich_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let list = tmp.join("FNIS_Test_List.txt");
        fs::write(
            &list,
            "s -o B_Billyy_ChairDildo_A1_S1 Chair.hkx AOChairA AOShockyDogDildoB\n",
        )
        .unwrap();

        let summary = pack.enrich_from_fnis_paths(&[list]).unwrap();
        assert_eq!(summary.positions_updated, 1);
        let scene = pack.scenes.values().next().unwrap();
        assert_eq!(
            scene.stages[0].positions[0].anim_obj,
            "AOChairA,AOShockyDogDildoB"
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn from_slal_pack_auto_enriches_fnis_lists() {
        let tmp = std::env::temp_dir().join(format!(
            "slsb_slal_pack_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let json = tmp.join("pack.json");
        fs::write(
            &json,
            r#"{
  "name": "TestPack",
  "animations": [
    {
      "name": "Chair Dildo",
      "creature_race": "",
      "tags": "Furniture",
      "actors": [
        {
          "type": "female",
          "stages": [
            { "id": "yhd9B_Billyy_ChairDildo_A1_S1" }
          ]
        }
      ]
    }
  ]
}"#,
        )
        .unwrap();

        let anim_dir = tmp
            .join("meshes")
            .join("actors")
            .join("character")
            .join("animations")
            .join("TestPack");
        fs::create_dir_all(&anim_dir).unwrap();
        fs::write(
            anim_dir.join("FNIS_TestPack_List.txt"),
            "s -o B_Billyy_ChairDildo_A1_S1 Chair.hkx AOChairA AOShockyDogDildoB\n",
        )
        .unwrap();

        let pack = Package::from_slal_pack(tmp.clone(), None).unwrap();
        assert_eq!(pack.pack_name, "TestPack");
        assert_eq!(pack.scenes.len(), 1);
        let scene = pack.scenes.values().next().unwrap();
        assert_eq!(
            scene.stages[0].positions[0].anim_obj,
            "AOChairA,AOShockyDogDildoB"
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn slal_timer_converts_ms_to_seconds() {
        assert!((super::slal_timer_seconds(5000.0) - 5.0).abs() < f32::EPSILON);
        assert_eq!(super::slal_timer_seconds(0.0), 0.0);
    }

    #[test]
    fn race_folders_use_forward_slashes() {
        assert_eq!(map_race_to_folder("Human").unwrap(), "character");
        assert_eq!(map_race_to_folder("Ash Hopper").unwrap(), "dlc02/scrib");
        assert_eq!(map_race_to_folder("Chaurus Hunter").unwrap(), "dlc01/chaurusflyer");
        assert!(!map_race_to_folder("Boar").unwrap().contains('\\'));
    }

    #[test]
    fn fnis_lists_use_nested_paths_and_crlf() {
        let tmp = std::env::temp_dir().join(format!(
            "slsb_fnis_layout_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mut events: HashMap<&str, Vec<String>> = HashMap::new();
        events.insert("Human", vec!["b -md abcdEvent Event.hkx".into()]);
        events.insert(
            "Ash Hopper",
            vec!["b -md abcdHopper Hopper.hkx".into()],
        );
        flush_fnis_lists(&tmp, "AnPack", &events, false).unwrap();

        let human = tmp
            .join("meshes")
            .join("actors")
            .join("character")
            .join("animations")
            .join("AnPack")
            .join("FNIS_AnPack_List.txt");
        let hopper = tmp
            .join("meshes")
            .join("actors")
            .join("dlc02")
            .join("scrib")
            .join("animations")
            .join("AnPack")
            .join("FNIS_AnPack_scrib_List.txt");
        assert!(human.is_file(), "missing {}", human.display());
        assert!(hopper.is_file(), "missing {}", hopper.display());

        let human_bytes = fs::read(&human).unwrap();
        assert!(
            human_bytes.windows(2).any(|w| w == b"\r\n"),
            "AnimList must use CRLF for Pandora/FNIS on Windows"
        );
        assert_eq!(
            human_bytes.iter().filter(|&&b| b == b'\n').count(),
            human_bytes.windows(2).filter(|w| *w == b"\r\n").count(),
            "every LF must be part of CRLF"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn write_fnis_line_emits_crlf() {
        let tmp = std::env::temp_dir().join(format!("slsb_fnis_line_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("list.txt");
        {
            let file = fs::File::create(&path).unwrap();
            let mut w = std::io::BufWriter::new(file);
            write_fnis_line(&mut w, "b -md abcdE E.hkx").unwrap();
            write_fnis_line(&mut w, "").unwrap();
        }
        assert_eq!(fs::read(&path).unwrap(), b"b -md abcdE E.hkx\r\n\r\n");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn billy_human_build_writes_detectable_layout() {
        let src = PathBuf::from(
            "/mnt/Data/Coding/SLAL Packs/Billy SLP/SLAL_Billyy_Human/SKSE/SexLab/Registry/Source/Billyy_Human.slsb.json",
        );
        if !src.is_file() {
            eprintln!("skip: Billy SLP source missing at {}", src.display());
            return;
        }
        let file = fs::File::open(&src).unwrap();
        let project = Package::from_file(file).unwrap();
        let out = std::env::temp_dir().join(format!(
            "slsb_billy_build_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&out);
        project.build(out.clone()).unwrap();

        let list = out
            .join("meshes")
            .join("actors")
            .join("character")
            .join("animations")
            .join("Billyy_Human")
            .join("FNIS_Billyy_Human_List.txt");
        let behavior = out
            .join("meshes")
            .join("actors")
            .join("character")
            .join("behaviors")
            .join("FNIS_Billyy_Human_Behavior.hkx");
        let slr = out
            .join("SKSE")
            .join("SexLab")
            .join("Registry")
            .join("Billyy_Human.slr");
        assert!(list.is_file(), "missing AnimList {}", list.display());
        assert!(behavior.is_file(), "missing Behavior {}", behavior.display());
        assert!(slr.is_file(), "missing registry {}", slr.display());
        let list_bytes = fs::read(&list).unwrap();
        assert!(list_bytes.windows(2).any(|w| w == b"\r\n"));
        // Behavior gen only succeeds when AnimList path matches Pandora layout.
        assert!(
            crate::project::behavior_gen::behavior_path_for_list(&list).is_some(),
            "AnimList path not recognized as FNIS layout: {}",
            list.display()
        );

        let _ = fs::remove_dir_all(&out);
    }
}
