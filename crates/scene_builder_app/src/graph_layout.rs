//! Layered BFS graph layout for stage nodes.

use scene_builder_core::project::define::Node;
use scene_builder_core::project::scene::Scene;
use scene_builder_core::project::NanoID;
use std::collections::{HashMap, HashSet, VecDeque};

pub const LAYOUT_H_GAP: f32 = 360.0;
pub const LAYOUT_V_GAP: f32 = 190.0;
pub const MAX_STAGES_PER_ROW: usize = 5;

/// True when every graph node shares the same (x, y) — typical of stacked SLAL imports.
pub fn graph_coords_stacked(scene: &Scene) -> bool {
    let positions: Vec<(f32, f32)> = scene.graph.values().map(|n| (n.x, n.y)).collect();
    if positions.len() < 2 {
        return false;
    }
    let first = positions[0];
    positions.iter().all(|&p| p == first)
}

/// True when every stored node sits at the origin (or graph is empty / all zeros).
pub fn graph_coords_all_zeros(scene: &Scene) -> bool {
    if scene.graph.is_empty() {
        return !scene.stages.is_empty();
    }
    scene.graph.values().all(|n| n.x == 0.0 && n.y == 0.0)
}

/// BFS layered layout from `scene.root`, writing `Node.x` / `Node.y`.
/// Ensures every stage has a graph entry; existing `dest` lists are preserved.
pub fn arrange_scene(scene: &mut Scene) {
    ensure_all_graph_nodes(scene);

    let node_ids: Vec<NanoID> = scene.stages.iter().map(|s| s.id.clone()).collect();
    if node_ids.is_empty() {
        return;
    }

    let root_id = if node_ids.iter().any(|id| id == &scene.root) {
        scene.root.clone()
    } else {
        node_ids[0].clone()
    };

    let positions = compute_layered_positions(scene, &root_id, &node_ids);
    for (id, (x, y)) in positions {
        if let Some(node) = scene.graph.get_mut(&id) {
            node.x = x;
            node.y = y;
        }
    }
}

fn ensure_all_graph_nodes(scene: &mut Scene) {
    for (i, stage) in scene.stages.iter().enumerate() {
        scene.graph.entry(stage.id.clone()).or_insert_with(|| {
            let col = (i % 4) as f32;
            let row = (i / 4) as f32;
            Node {
                dest: Vec::new(),
                x: 40.0 + col * LAYOUT_H_GAP,
                y: 40.0 + row * LAYOUT_V_GAP,
            }
        });
    }
}

fn compute_layered_positions(
    scene: &Scene,
    root_id: &NanoID,
    node_ids: &[NanoID],
) -> HashMap<NanoID, (f32, f32)> {
    let id_set: HashSet<&NanoID> = node_ids.iter().collect();

    let mut outgoing: HashMap<&NanoID, Vec<&NanoID>> = HashMap::new();
    for id in node_ids {
        let dest: Vec<&NanoID> = scene
            .graph
            .get(id)
            .map(|n| n.dest.iter().filter(|d| id_set.contains(d)).collect())
            .unwrap_or_default();
        outgoing.insert(id, dest);
    }

    let mut level: HashMap<&NanoID, usize> = HashMap::new();
    let mut queue: VecDeque<&NanoID> = VecDeque::new();
    if id_set.contains(root_id) {
        level.insert(root_id, 0);
        queue.push_back(root_id);
    }

    while let Some(id) = queue.pop_front() {
        let lv = *level.get(id).unwrap_or(&0);
        for dest in outgoing.get(id).into_iter().flatten() {
            if !level.contains_key(dest) {
                level.insert(dest, lv + 1);
                queue.push_back(dest);
            }
        }
    }

    let orphans: Vec<&NanoID> = node_ids
        .iter()
        .filter(|id| !level.contains_key(id))
        .collect();

    let mut by_level: HashMap<usize, Vec<&NanoID>> = HashMap::new();
    for (id, lv) in &level {
        by_level.entry(*lv).or_default().push(id);
    }

    let mut ordered: Vec<&NanoID> = Vec::new();
    let mut levels_sorted: Vec<_> = by_level.keys().copied().collect();
    levels_sorted.sort_unstable();
    for lv in &levels_sorted {
        if let Some(ids) = by_level.get(lv) {
            ordered.extend(ids.iter().copied());
        }
    }
    ordered.extend(orphans.iter().copied());

    let linear = orphans.is_empty() && by_level.values().all(|ids| ids.len() <= 1);

    let mut positions = HashMap::new();
    if linear && ordered.len() > MAX_STAGES_PER_ROW {
        for (i, id) in ordered.iter().enumerate() {
            let row = (i / MAX_STAGES_PER_ROW) as f32;
            let col = (i % MAX_STAGES_PER_ROW) as f32;
            positions.insert(
                (*id).clone(),
                (40.0 + col * LAYOUT_H_GAP, 40.0 + row * LAYOUT_V_GAP),
            );
        }
    } else {
        for lv in levels_sorted {
            if let Some(ids) = by_level.get(&lv) {
                for (i, id) in ids.iter().enumerate() {
                    positions.insert(
                        (*id).clone(),
                        (
                            40.0 + (lv as f32) * LAYOUT_H_GAP,
                            40.0 + (i as f32) * LAYOUT_V_GAP,
                        ),
                    );
                }
            }
        }
        if !orphans.is_empty() {
            let max_rows = by_level
                .values()
                .map(|ids| ids.len())
                .max()
                .unwrap_or(1)
                .max(1);
            let orphan_y = 40.0 + (max_rows as f32) * LAYOUT_V_GAP;
            for (i, id) in orphans.iter().enumerate() {
                let row = (i / MAX_STAGES_PER_ROW) as f32;
                let col = (i % MAX_STAGES_PER_ROW) as f32;
                positions.insert(
                    (*id).clone(),
                    (40.0 + col * LAYOUT_H_GAP, orphan_y + row * LAYOUT_V_GAP),
                );
            }
        }
    }

    positions
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene_builder_core::project::stage::Stage;

    #[test]
    fn arrange_linear_chain_spreads_horizontally() {
        let mut scene = Scene::default();
        let mut a = Stage::new(&scene);
        a.name = "A".into();
        let id_a = a.id.clone();
        scene.root = id_a.clone();
        scene.stages.push(a);

        let mut b = Stage::new(&scene);
        b.name = "B".into();
        let id_b = b.id.clone();
        scene.stages.push(b);

        scene.graph.insert(
            id_a.clone(),
            Node {
                dest: vec![id_b.clone()],
                x: 0.0,
                y: 0.0,
            },
        );
        scene.graph.insert(
            id_b.clone(),
            Node {
                dest: vec![],
                x: 0.0,
                y: 0.0,
            },
        );

        assert!(graph_coords_all_zeros(&scene));
        arrange_scene(&mut scene);
        let ax = scene.graph[&id_a].x;
        let bx = scene.graph[&id_b].x;
        assert!(bx > ax, "B should be to the right of A: {ax} vs {bx}");
    }
}
