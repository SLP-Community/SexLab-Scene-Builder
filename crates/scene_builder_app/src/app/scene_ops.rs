use super::SceneBuilderApp;
use crate::graph;
use crate::graph_layout::{arrange_scene, graph_coords_all_zeros, graph_coords_stacked};
use crate::stage_editor::StageEditorState;
use crate::toasts::ToastKind;
use egui::Context;
use scene_builder_core::project::define::Node as GraphNode;
use scene_builder_core::project::scene::Scene;
use scene_builder_core::project::stage::Stage;
use scene_builder_core::project::NanoID;

impl SceneBuilderApp {
    pub(super) fn add_blank_scene(&mut self) {
        let mut scene = Scene::default();
        scene.name = format!("Scene {}", self.ws.package.scenes.len() + 1);
        let stage = Stage::new(&scene);
        scene.root = stage.id.clone();
        graph::ensure_graph_node(&mut scene, &stage.id, 0);
        scene.stages.push(stage);
        scene.positions = scene
            .stages
            .first()
            .map(|s| {
                s.positions
                    .iter()
                    .map(|p| p.extract_position_info())
                    .collect()
            })
            .unwrap_or_default();
        let id = scene.id.clone();
        self.ws.package.save_scene(scene);
        self.select_scene(id);
        self.mark_dirty();
    }

    pub(super) fn select_scene(&mut self, id: NanoID) {
        self.ws.selected_scene = Some(id.clone());
        self.graph.selected = None;
        self.graph.request_fit();
        if let Some(scene) = self.ws.package.get_scene_mut(&id) {
            if graph_coords_stacked(scene) || graph_coords_all_zeros(scene) {
                arrange_scene(scene);
            }
        }
    }

    pub(super) fn delete_stage_from_scene(&mut self, scene_id: &NanoID, stage_id: &NanoID) {
        let Some(scene) = self.ws.package.get_scene_mut(scene_id) else {
            return;
        };
        self.graph.push_undo(scene);
        scene.stages.retain(|s| &s.id != stage_id);
        scene.graph.remove(stage_id);
        for node in scene.graph.values_mut() {
            node.dest.retain(|d| d != stage_id);
        }
        if scene.root == *stage_id {
            scene.root = scene
                .stages
                .first()
                .map(|s| s.id.clone())
                .unwrap_or_else(NanoID::new_nanoid);
        }
        if self.graph.selected.as_ref() == Some(stage_id) {
            self.graph.selected = None;
        }
        Stage::renumber_auto_names(scene);
        self.mark_dirty();
    }

    pub(super) fn set_scene_root(&mut self, scene_id: &NanoID, stage_id: &NanoID) {
        let Some(scene) = self.ws.package.get_scene_mut(scene_id) else {
            return;
        };
        if scene.stages.iter().any(|s| &s.id == stage_id) {
            scene.root = stage_id.clone();
            self.mark_dirty();
        }
    }
    /// Adds a stage (optionally linked from the previous last stage). Returns the new id.
    pub(super) fn add_stage_to_scene(&mut self, scene_id: &NanoID) -> Option<NanoID> {
        let scene = self.ws.package.get_scene_mut(scene_id)?;
        self.graph.push_undo(scene);
        let stage = Stage::new(scene);
        let id = stage.id.clone();
        let idx = scene.stages.len();
        if scene.stages.is_empty() {
            scene.root = id.clone();
        } else if let Some(prev) = scene.stages.last() {
            let prev_id = prev.id.clone();
            graph::ensure_graph_node(scene, &prev_id, idx.saturating_sub(1));
            if let Some(node) = scene.graph.get_mut(&prev_id) {
                if node.dest.is_empty() {
                    node.dest.push(id.clone());
                }
            }
        }
        graph::ensure_graph_node(scene, &id, idx);
        scene.stages.push(stage);
        Stage::renumber_auto_names(scene);
        self.graph.selected = Some(id.clone());
        self.mark_dirty();
        Some(id)
    }

    /// Validate name/root/reachability, toast problems, then persist has_warnings.
    pub(super) fn store_scene(&mut self, ctx: &Context, scene_id: &NanoID) {
        let Some(scene) = self.ws.package.get_scene(scene_id) else {
            return;
        };
        let mut has_warnings = false;
        let mut do_save = true;

        if scene.name.trim().is_empty() {
            self.toasts.push(
                ctx,
                ToastKind::Error,
                "Missing Name",
                "Add a short, descriptive name to your scene.",
            );
            do_save = false;
        }

        let root_exists = scene.stages.iter().any(|s| s.id == scene.root);
        if !root_exists {
            self.toasts.push(
                ctx,
                ToastKind::Warning,
                "Missing Start Animation",
                "Choose the stage which the scene is supposed to start at.",
            );
            has_warnings = true;
        } else {
            let mut visited = std::collections::HashSet::new();
            let mut queue = vec![scene.root.clone()];
            visited.insert(scene.root.clone());
            while let Some(id) = queue.pop() {
                if let Some(node) = scene.graph.get(&id) {
                    for dest in &node.dest {
                        if scene.stages.iter().any(|s| &s.id == dest)
                            && visited.insert(dest.clone())
                        {
                            queue.push(dest.clone());
                        }
                    }
                }
            }
            if visited.len() < scene.stages.len() {
                self.toasts.push(
                    ctx,
                    ToastKind::Warning,
                    "Unreachable Stages",
                    "Scene contains stages which cannot be reached from the start animation.",
                );
                has_warnings = true;
            }
        }

        if !do_save {
            return;
        }
        if let Some(scene) = self.ws.package.get_scene_mut(scene_id) {
            scene.has_warnings = has_warnings;
        }
        self.status = "Scene stored".into();
    }

    /// Duplicate a stage inside its own scene, offset from the original.
    pub(super) fn clone_stage_in_scene(&mut self, scene_id: &NanoID, stage_id: &NanoID) {
        let Some(scene) = self.ws.package.get_scene_mut(scene_id) else {
            return;
        };
        let Some(orig) = scene.get_stage(stage_id) else {
            return;
        };
        let mut copy = orig.clone();
        copy.id = NanoID::new_nanoid();
        if Stage::is_auto_name(&copy.name) {
            copy.name = "Stage 0/0".into();
        }
        let new_id = copy.id.clone();
        let (x, y) = scene
            .graph
            .get(stage_id)
            .map(|n| (n.x + 40.0, n.y + 40.0))
            .unwrap_or((40.0, 40.0));
        self.graph.push_undo(scene);
        scene.graph.insert(
            new_id.clone(),
            GraphNode {
                dest: Vec::new(),
                x,
                y,
            },
        );
        scene.stages.push(copy);
        Stage::renumber_auto_names(scene);
        self.graph.selected = Some(new_id);
        self.mark_dirty();
    }

    /// Copy a stage into another scene (Clone to… modal target).
    pub(super) fn clone_stage_to_scene(
        &mut self,
        ctx: &Context,
        stage_id: &NanoID,
        from_scene: &NanoID,
        to_scene: &NanoID,
    ) {
        let Some(stage) = self
            .ws
            .package
            .get_scene(from_scene)
            .and_then(|s| s.get_stage(stage_id))
            .cloned()
        else {
            self.toasts.push(
                ctx,
                ToastKind::Error,
                "Clone failed",
                "The source stage no longer exists.",
            );
            return;
        };
        let target_name;
        {
            let Some(target) = self.ws.package.get_scene_mut(to_scene) else {
                return;
            };
            let src_n = stage.positions.len();
            let mut copy = stage;
            copy.id = NanoID::new_nanoid();
            let idx = target.stages.len();
            graph::ensure_graph_node(target, &copy.id, idx);
            let toast_detail;
            if target.stages.is_empty() {
                target.root = copy.id.clone();
                target.positions = copy
                    .positions
                    .iter()
                    .map(|p| p.extract_position_info())
                    .collect();
                toast_detail = format!("Added to \"{}\".", target.name);
            } else {
                let dst_n = target.positions.len();
                if src_n.max(1) != dst_n.max(1) {
                    crate::positions::adopt_scene_position_count(target, src_n);
                    toast_detail = format!(
                        "Added to \"{}\" (scene positions {} → {}).",
                        target.name, dst_n, src_n
                    );
                } else {
                    toast_detail = format!("Added to \"{}\".", target.name);
                }
            }
            target_name = toast_detail;
            if Stage::is_auto_name(&copy.name) {
                copy.name = "Stage 0/0".into();
            }
            target.stages.push(copy);
            Stage::renumber_auto_names(target);
        }
        self.mark_dirty();
        self.toasts
            .push(ctx, ToastKind::Success, "Stage cloned", &target_name);
    }

    pub(super) fn open_stage_editor(&mut self, scene_id: &NanoID, stage_id: &NanoID) {
        let Some(scene) = self.ws.package.get_scene(scene_id) else {
            return;
        };
        let Some(stage) = scene.get_stage(stage_id) else {
            return;
        };
        self.stage_editor = Some(StageEditorState::new(
            scene_id.clone(),
            stage.clone(),
            scene.positions.clone(),
        ));
    }
}
