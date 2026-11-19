//! Skill-tree pictures: a shared backdrop of every node, and a per-build overlay.
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    sync::{LazyLock, OnceLock},
};

use serde_json::Value;

use crate::html::{ASSET_REV, esc};

const CROP_MARGIN: f64 = 90.;
const MIN_CROP: f64 = 520.;

pub static INCARNATION: LazyLock<Tree> = LazyLock::new(|| {
    Tree::parse(
        include_str!("../../../data/incarnation-tree.json"),
        "tree-incarnation.svg",
        "#c9a55a",
        "#c48a3a",
    )
});

pub static ETHER: LazyLock<Tree> = LazyLock::new(|| {
    Tree::parse(
        include_str!("../../../data/ether-tree.json"),
        "tree-ether.svg",
        "#a574c9",
        "#8a5fb0",
    )
});

struct Node {
    x: f64,
    y: f64,
    radius: f64,
}

pub struct Tree {
    pub file: &'static str,
    view: [f64; 4],
    nodes: HashMap<u32, Node>,
    edges: Vec<(u32, u32)>,
    node_color: &'static str,
    edge_color: &'static str,
    backdrop: OnceLock<String>,
}

impl Tree {
    fn parse(
        json: &str,
        file: &'static str,
        node_color: &'static str,
        edge_color: &'static str,
    ) -> Self {
        let layout: Value = serde_json::from_str(json).expect("bundled tree layout is valid JSON");
        let mut view = [0.; 4];
        let numbers = layout["viewBox"].as_str().unwrap_or_default();
        for (slot, number) in view.iter_mut().zip(numbers.split_whitespace()) {
            *slot = number.parse().unwrap_or_default();
        }
        let id = |value: &Value| value.as_u64().and_then(|id| u32::try_from(id).ok());
        let nodes = layout["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|node| {
                let position = Node {
                    x: node["x"].as_f64()?,
                    y: node["y"].as_f64()?,
                    radius: node["r"].as_f64().unwrap_or(7.),
                };
                Some((id(&node["id"])?, position))
            })
            .collect();
        let edges = layout["edges"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|edge| Some((id(&edge[0])?, id(&edge[1])?)))
            .collect();
        Self {
            file,
            view,
            nodes,
            edges,
            node_color,
            edge_color,
            backdrop: OnceLock::new(),
        }
    }

    fn links<'a>(&'a self, keep: impl Fn(u32) -> bool + 'a) -> String {
        let mut path = String::new();
        for (from, to) in &self.edges {
            if let (true, true, Some(a), Some(b)) = (
                keep(*from),
                keep(*to),
                self.nodes.get(from),
                self.nodes.get(to),
            ) {
                let _ = write!(path, "M{} {}L{} {}", a.x, a.y, b.x, b.y);
            }
        }
        path
    }

    /// Every node and link, unallocated; browsers cache it once for all builds.
    pub fn backdrop(&self) -> &str {
        self.backdrop.get_or_init(|| self.draw_backdrop())
    }

    fn draw_backdrop(&self) -> String {
        let [x, y, width, height] = self.view;
        let mut ids: Vec<_> = self.nodes.keys().collect();
        ids.sort();
        let circles: String = ids
            .into_iter()
            .map(|id| {
                let node = &self.nodes[id];
                format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
                    node.x, node.y, node.radius
                )
            })
            .collect();
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{x} {y} {width} {height}\">\
<path d=\"{}\" fill=\"none\" stroke=\"#2a2f3a\" stroke-width=\"2\"/>\
<g fill=\"#1c1d24\" stroke=\"#3a3528\" stroke-width=\"1.5\">{circles}</g></svg>",
            self.links(|_| true)
        )
    }

    /// The allocated nodes drawn over the backdrop, cropped to where the build spent points.
    pub fn overlay(
        &self,
        allocated: &[u32],
        label: &str,
        describe: impl Fn(u32) -> Option<String>,
    ) -> Option<String> {
        let chosen: HashSet<u32> = allocated
            .iter()
            .copied()
            .filter(|id| self.nodes.contains_key(id))
            .collect();
        if chosen.is_empty() {
            return None;
        }
        let mut ids: Vec<_> = chosen.iter().copied().collect();
        ids.sort_unstable();
        let mut circles = String::new();
        for id in &ids {
            let node = &self.nodes[id];
            let title = describe(*id)
                .map_or_else(String::new, |text| format!("<title>{}</title>", esc(&text)));
            let _ = write!(
                circles,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\">{title}</circle>",
                node.x, node.y, node.radius
            );
        }
        let [x, y, width, height] = self.view;
        let [crop_x, crop_y, crop_width, crop_height] = self.crop(&ids);
        Some(format!(
            "<svg class=\"tree\" viewBox=\"{crop_x} {crop_y} {crop_width} {crop_height}\" role=\"img\" aria-label=\"{}\">\
<image href=\"/static/{}?v={ASSET_REV}\" x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\"/>\
<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"3\"/>\
<g fill=\"{}\" stroke=\"#d4cfbf\" stroke-width=\"1.5\">{circles}</g></svg>",
            esc(label),
            self.file,
            self.links(|id| chosen.contains(&id)),
            self.edge_color,
            self.node_color,
        ))
    }

    fn crop(&self, ids: &[u32]) -> [f64; 4] {
        let [view_x, view_y, view_width, view_height] = self.view;
        let span = |pick: fn(&Node) -> f64, start: f64, length: f64| {
            let values = ids.iter().map(|id| pick(&self.nodes[id]));
            let low = values.clone().fold(f64::INFINITY, f64::min) - CROP_MARGIN;
            let high = values.fold(f64::NEG_INFINITY, f64::max) + CROP_MARGIN;
            let size = (high - low).max(MIN_CROP).min(length);
            let origin = ((low + high - size) / 2.).clamp(start, start + length - size);
            (origin.round(), size.round())
        };
        let (x, width) = span(|node| node.x, view_x, view_width);
        let (y, height) = span(|node| node.y, view_y, view_height);
        [x, y, width, height]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_draws_allocated_nodes_and_only_the_links_between_them() {
        let tree = &*INCARNATION;
        let (from, to) = tree.edges[0];
        let apart = *tree
            .nodes
            .keys()
            .find(|id| {
                !tree.edges.iter().any(|(a, b)| {
                    [*a, *b].contains(id) && ([*a, *b].contains(&from) || [*a, *b].contains(&to))
                })
            })
            .unwrap();

        let svg = tree
            .overlay(&[from, to, apart, u32::MAX], "Tree", |id| {
                Some(format!("<node {id}>"))
            })
            .unwrap();

        assert_eq!(svg.matches("<circle").count(), 3);
        assert_eq!(svg.matches('L').count(), 1);
        assert!(svg.contains("&lt;node"));
        assert!(!svg.contains("<node"));
    }

    #[test]
    fn builds_without_allocated_nodes_get_no_picture() {
        assert!(INCARNATION.overlay(&[], "Tree", |_| None).is_none());
        assert!(ETHER.overlay(&[u32::MAX], "Tree", |_| None).is_none());
    }

    #[test]
    fn backdrop_holds_every_node_of_the_layout() {
        let svg = ETHER.backdrop();

        assert_eq!(svg.matches("<circle").count(), ETHER.nodes.len());
        assert!(!svg.contains("<style"));
    }
}
