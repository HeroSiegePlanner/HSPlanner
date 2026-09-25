use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Deserialize)]
pub struct Node {
    pub id: usize,
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub t: String,
    pub icon: String,
}

#[derive(Clone, Deserialize)]
pub struct Info {
    pub t: String,
    pub n: String,
    pub l: Vec<String>,
    pub note: Option<String>,
    #[serde(default)]
    pub g: Vec<String>,
}

#[derive(Deserialize)]
struct RawGraph {
    nodes: Vec<Node>,
    edges: Vec<[usize; 2]>,
    #[serde(rename = "viewBox")]
    view_box: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeKind {
    #[default]
    Incarnation,
    Ether,
}
impl TreeKind {
    pub fn nodes(self, snapshot: &hsplanner_build::BuildSnapshot) -> &[u32] {
        match self {
            Self::Incarnation => &snapshot.allocated_tree_nodes,
            Self::Ether => &snapshot.allocated_ether_nodes,
        }
    }
    pub fn apply(self, snapshot: &mut hsplanner_build::BuildSnapshot, nodes: &[u32]) {
        match self {
            Self::Incarnation => snapshot.set_tree_nodes(nodes),
            Self::Ether => snapshot.allocated_ether_nodes = nodes.to_vec(),
        }
    }
}

pub struct Graph {
    pub kind: TreeKind,
    pub nodes: Vec<Node>,
    pub edges: Vec<[usize; 2]>,
    pub info: HashMap<usize, Info>,
    pub roots: Vec<usize>,
    adjacency: Vec<Vec<usize>>,
    pub bounds: [f32; 4],
}

impl Graph {
    pub fn load() -> Self {
        let raw: RawGraph =
            serde_json::from_str(include_str!("../../../data/incarnation-tree.json"))
                .expect("valid incarnation tree");
        let info = serde_json::from_str(include_str!("../../../data/incarnation-nodes.json"))
            .expect("valid node descriptions");
        Self::from_raw(TreeKind::Incarnation, raw, info)
    }

    pub fn load_ether() -> Self {
        let value: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/ether-tree.json"))
                .expect("valid Ether tree");
        let raw: RawGraph = serde_json::from_value(value.clone()).expect("valid Ether geometry");
        let info = value["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| {
                let stat = &value["stats"][node["key"].as_str().unwrap()];
                let label = stat["label"].as_str().expect("Ether stat label");
                let line = format!(
                    "{} {}",
                    stat["value"].as_str().unwrap(),
                    stat["desc"].as_str().unwrap()
                );
                (
                    node["id"].as_u64().unwrap() as usize,
                    Info {
                        t: label.into(),
                        n: node["t"].as_str().unwrap().into(),
                        l: vec![line],
                        note: None,
                        g: vec![],
                    },
                )
            })
            .collect();
        Self::from_raw(TreeKind::Ether, raw, info)
    }

    fn from_raw(kind: TreeKind, raw: RawGraph, info: HashMap<usize, Info>) -> Self {
        let index: HashMap<_, _> = raw
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id, i))
            .collect();
        assert_eq!(index.len(), raw.nodes.len(), "duplicate tree IDs");
        let mut adjacency = vec![Vec::new(); raw.nodes.len()];
        let edges: Vec<_> = raw
            .edges
            .into_iter()
            .filter(|[a, b]| a != b)
            .map(|[a, b]| {
                let (a, b) = (index[&a], index[&b]);
                adjacency[a].push(b);
                adjacency[b].push(a);
                [a, b]
            })
            .collect();
        let mut roots: Vec<_> = raw
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.t == "root")
            .map(|(i, _)| i)
            .collect();
        roots.sort_by_key(|&index| raw.nodes[index].id);
        // Fit uses the authored viewBox, including its intentional margins,
        // exactly like the shipping SVG view. Node extents are narrower.
        let view_box: Vec<f32> = raw
            .view_box
            .split_whitespace()
            .map(|value| value.parse().expect("numeric tree viewBox"))
            .collect();
        let [x, y, width, height] = view_box.as_slice() else {
            panic!("tree viewBox must contain four values");
        };
        assert!(*width > 0. && *height > 0., "positive tree viewBox");
        let bounds = [*x, *y, x + width, y + height];
        Self {
            kind,
            nodes: raw.nodes,
            edges,
            info,
            roots,
            adjacency,
            bounds,
        }
    }

    pub fn path_to(&self, selected: &HashSet<usize>, target: usize) -> Vec<usize> {
        let mut sources: Vec<_> = selected
            .iter()
            .copied()
            .chain(self.roots.iter().copied())
            .collect();
        sources.sort_unstable();
        sources.dedup();
        self.path_from(sources, target)
    }

    fn path_from(&self, sources: impl IntoIterator<Item = usize>, target: usize) -> Vec<usize> {
        let mut parent = vec![None; self.nodes.len()];
        let mut queue = VecDeque::new();
        for start in sources {
            if parent[start].is_some() {
                continue;
            }
            parent[start] = Some(start);
            queue.push_back(start);
        }
        while let Some(current) = queue.pop_front() {
            if current == target {
                let mut path = vec![current];
                let mut step = current;
                while parent[step] != Some(step) {
                    step = parent[step].unwrap();
                    path.push(step);
                }
                path.reverse();
                return path;
            }
            for &next in &self.adjacency[current] {
                if parent[next].is_none() {
                    parent[next] = Some(current);
                    queue.push_back(next);
                }
            }
        }
        Vec::new()
    }

    pub fn ordered_toggle(&self, current: &[u32], target: usize) -> Vec<u32> {
        let ids: HashSet<_> = current.iter().copied().collect();
        let mut selected: HashSet<_> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| ids.contains(&(node.id as u32)))
            .map(|(index, _)| index)
            .collect();
        if selected.contains(&target) {
            self.toggle(&mut selected, target);
            let remaining: HashSet<_> = selected
                .iter()
                .map(|&index| self.nodes[index].id as u32)
                .collect();
            return current
                .iter()
                .copied()
                .filter(|id| remaining.contains(id))
                .collect();
        }
        let mut ordered = current.to_vec();
        let by_id: HashMap<_, _> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id as u32, i))
            .collect();
        let sources = current
            .iter()
            .filter_map(|id| by_id.get(id).copied())
            .chain(self.roots.iter().copied());
        for index in self.path_from(sources, target) {
            let id = self.nodes[index].id as u32;
            if !ordered.contains(&id) {
                ordered.push(id);
            }
        }
        ordered
    }

    pub fn toggle(&self, selected: &mut HashSet<usize>, target: usize) {
        if !selected.remove(&target) {
            selected.extend(self.path_to(selected, target));
            return;
        }
        let mut reachable = HashSet::new();
        let mut queue: VecDeque<_> = self
            .roots
            .iter()
            .copied()
            .filter(|id| selected.contains(id))
            .collect();
        while let Some(current) = queue.pop_front() {
            if !reachable.insert(current) {
                continue;
            }
            for &next in &self.adjacency[current] {
                if selected.contains(&next) {
                    queue.push_back(next);
                }
            }
        }
        selected.retain(|id| reachable.contains(id));
    }

    pub fn search(&self, query: &str) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return vec![];
        }
        let id_query = query.strip_prefix('#').unwrap_or(&query);
        let numeric = !id_query.is_empty() && id_query.bytes().all(|c| c.is_ascii_digit());
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                (numeric && node.id.to_string().contains(id_query))
                    || self.info.get(&node.id).is_some_and(|info| {
                        std::iter::once(&info.t)
                            .chain(&info.l)
                            .chain(info.note.iter())
                            .any(|text| text.to_lowercase().contains(&query))
                    })
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn hit(&self, camera: Camera, pos: [f32; 2]) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                let p = camera.screen([n.x, n.y]);
                let distance = (p[0] - pos[0]).hypot(p[1] - pos[1]);
                let radius = n.r + if n.t == "root" { 3. } else { 0. };
                (distance <= (radius * camera.scale).max(5.0)).then_some((i, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub scale: f32,
    pub offset: [f32; 2],
}

impl Camera {
    pub fn screen(self, world: [f32; 2]) -> [f32; 2] {
        [
            world[0] * self.scale + self.offset[0],
            world[1] * self.scale + self.offset[1],
        ]
    }

    pub fn world(self, screen: [f32; 2]) -> [f32; 2] {
        [
            (screen[0] - self.offset[0]) / self.scale,
            (screen[1] - self.offset[1]) / self.scale,
        ]
    }

    pub fn centered(center: [f32; 2], viewport: [f32; 2], scale: f32) -> Self {
        Self {
            scale,
            offset: [
                viewport[0] / 2.0 - center[0] * scale,
                viewport[1] / 2.0 - center[1] * scale,
            ],
        }
    }

    pub fn fit(bounds: [f32; 4], viewport: [f32; 2]) -> Self {
        let scale = (viewport[0] / (bounds[2] - bounds[0]))
            .min(viewport[1] / (bounds[3] - bounds[1]))
            * 0.95;
        Self::centered(
            [(bounds[0] + bounds[2]) / 2.0, (bounds[1] + bounds[3]) / 2.0],
            viewport,
            scale,
        )
    }

    pub fn zoom(&mut self, anchor: [f32; 2], factor: f32) {
        let world = self.world(anchor);
        self.scale = (self.scale * factor).clamp(0.08, 3.5);
        self.offset = [
            anchor[0] - world[0] * self.scale,
            anchor[1] - world[1] * self.scale,
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_accepts_hash_ids_without_confusing_them_with_stat_text() {
        let graph = Graph::load();
        let matches = graph.search("#1025");
        assert_eq!(matches.len(), 1);
        assert_eq!(graph.nodes[matches[0]].id, 1025);
        assert!(graph.search("#does-not-exist").is_empty());
        assert!(graph.search("  ").is_empty());
        assert!(!graph.search("intelligence").is_empty());
    }

    #[test]
    fn ether_paths_keep_allocation_order_and_remove_orphaned_branches() {
        let graph = Graph::load_ether();
        assert_eq!(graph.nodes.len(), 572);
        assert_eq!(graph.roots.len(), 4);
        for target in 0..graph.nodes.len() {
            let path = graph.ordered_toggle(&[], target);
            assert!(graph.info.contains_key(&graph.nodes[target].id));
            // These twelve region labels are decorative and have no edges in the reference data.
            if (393..=404).contains(&graph.nodes[target].id) {
                assert!(path.is_empty());
                continue;
            }
            assert!(path.contains(&(graph.nodes[target].id as u32)));
            let root = graph
                .nodes
                .iter()
                .position(|n| n.id as u32 == path[0])
                .unwrap();
            assert!(graph.ordered_toggle(&path, root).is_empty());
        }
        let first = graph.ordered_toggle(&[], 30);
        let second = graph.ordered_toggle(&first, 200);
        assert!(second.starts_with(&first));
    }

    #[test]
    fn equally_short_paths_follow_the_existing_allocation_order() {
        let raw = RawGraph {
            view_box: "-1 -1 6 2".into(),
            nodes: (0..5)
                .map(|id| Node {
                    id,
                    x: id as f32,
                    y: 0.,
                    r: 1.,
                    t: if id == 0 { "root" } else { "small" }.into(),
                    icon: String::new(),
                })
                .collect(),
            edges: vec![[0, 1], [0, 2], [1, 3], [2, 4], [3, 4]],
        };
        let graph = Graph::from_raw(TreeKind::Incarnation, raw, HashMap::new());
        assert_eq!(graph.ordered_toggle(&[0, 2, 3], 4), [0, 2, 3, 4]);
        assert_eq!(graph.path_from([3, 2, 0], 4), [3, 4]);
    }

    #[test]
    fn real_data_has_descriptions_and_connected_allocation_paths() {
        let graph = Graph::load();
        assert!(graph.nodes.len() > 2000);
        assert_eq!(graph.roots.len(), 8);
        for (i, node) in graph.nodes.iter().enumerate() {
            assert!(graph.info.contains_key(&node.id));
            let path = graph.path_to(&HashSet::new(), i);
            assert!(!path.is_empty(), "unreachable node {}", node.id);
            for pair in path.windows(2) {
                assert!(graph.adjacency[pair[0]].contains(&pair[1]));
            }
        }
    }

    #[test]
    fn every_black_hole_provides_a_direct_path_to_the_other_three() {
        let graph = Graph::load();
        let holes: Vec<_> = [412, 413, 414, 415]
            .into_iter()
            .map(|id| graph.nodes.iter().position(|node| node.id == id).unwrap())
            .collect();
        for &from in &holes {
            for &to in &holes {
                if from == to {
                    continue;
                }
                assert_eq!(graph.path_to(&HashSet::from([from]), to), [from, to]);
                let current = graph.ordered_toggle(&[], from);
                if !current.contains(&(graph.nodes[to].id as u32)) {
                    let next = graph.ordered_toggle(&current, to);
                    assert!(next.starts_with(&current));
                    assert_eq!(next.len(), current.len() + 1);
                    assert_eq!(next.last(), Some(&(graph.nodes[to].id as u32)));
                }
            }
        }
    }

    #[test]
    fn removing_an_allocated_root_cleans_up_its_orphaned_path() {
        let graph = Graph::load();
        let target = graph.nodes.len() - 1;
        let mut selected = HashSet::new();
        graph.toggle(&mut selected, target);
        assert!(selected.contains(&target));
        let root = *graph.roots.iter().find(|r| selected.contains(r)).unwrap();
        graph.toggle(&mut selected, root);
        assert!(selected.is_empty());
    }

    #[test]
    fn cursor_anchored_zoom_keeps_hit_testing_aligned_at_limits() {
        let graph = Graph::load();
        let node = &graph.nodes[5];
        let mut camera = Camera::centered([node.x, node.y], [900.0, 700.0], 0.8);
        let anchor = camera.screen([node.x, node.y]);
        for factor in [1.2, 100.0, 0.0001, 1.7] {
            camera.zoom(anchor, factor);
            let actual = camera.screen([node.x, node.y]);
            assert!((actual[0] - anchor[0]).abs() < 0.001);
            assert!((actual[1] - anchor[1]).abs() < 0.001);
            assert_eq!(graph.hit(camera, anchor), Some(5));
        }
    }

    #[test]
    fn starting_node_frame_remains_clickable_after_tauri_styling() {
        for graph in [Graph::load(), Graph::load_ether()] {
            let index = graph.roots[0];
            let node = &graph.nodes[index];
            let camera = Camera::centered([node.x, node.y], [900., 700.], 1.2);
            let center = camera.screen([node.x, node.y]);
            let on_frame = [center[0] + (node.r + 2.) * camera.scale, center[1]];
            assert_eq!(graph.hit(camera, on_frame), Some(index));
        }
    }

    #[test]
    fn fit_contains_all_nodes_with_padding() {
        let graph = Graph::load();
        let viewport = [920.0, 740.0];
        let camera = Camera::fit(graph.bounds, viewport);
        for node in &graph.nodes {
            let [x, y] = camera.screen([node.x, node.y]);
            assert!((0.0..viewport[0]).contains(&x));
            assert!((0.0..viewport[1]).contains(&y));
        }
    }
}
