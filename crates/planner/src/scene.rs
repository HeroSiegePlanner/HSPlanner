use crate::tree::{Camera, Graph, Node, TreeKind};
use gpui_kit::{
    Bounds, ContentMask, Hsla, PathBuilder, Pixels, RenderImage, Window, fill, point, px, size,
};
use hsplanner_ui::theme::TreeTheme;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
    time::Instant,
};

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

fn render_image(mut rgba: image::RgbaImage) -> Arc<RenderImage> {
    // GPUI's sprite atlas consumes BGRA pixels.
    for pixel in rgba.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(rgba)]))
}

static BACKGROUND: LazyLock<Arc<RenderImage>> = LazyLock::new(|| {
    render_image(
        image::load_from_memory_with_format(
            include_bytes!("../../../assets/atlas/Incarnation_Background.png"),
            image::ImageFormat::Png,
        )
        .expect("valid embedded tree background")
        .into_rgba8(),
    )
});

// GPUI 0.6 has linear gradients only. This shared alpha texture implements the
// reference's elliptical vignette without regenerating pixels during pan/zoom.
static VIGNETTE: LazyLock<Arc<RenderImage>> = LazyLock::new(|| {
    let maximum_opacity = TreeTheme::incarnation().vignette_opacity();
    render_image(image::RgbaImage::from_fn(256, 256, |x, y| {
        let x = x as f32 / 255. * 2. - 1.;
        let y = y as f32 / 255. * 2. - 1.;
        let distance = x.hypot(y) / std::f32::consts::SQRT_2;
        let opacity = ((distance - 0.5) * 2.).clamp(0., 1.);
        image::Rgba([0, 0, 0, (opacity * maximum_opacity * 255.).round() as u8])
    }))
});

pub struct Scene {
    pub graph: Graph,
    icons: HashMap<String, Arc<RenderImage>>,
    painted_edges: Vec<[usize; 2]>,
    dim_icons: HashMap<String, Arc<RenderImage>>,
    background: Arc<RenderImage>,
    vignette: Arc<RenderImage>,
}

#[derive(Clone, Copy, Default)]
pub struct PaintStats {
    pub visible: usize,
    pub milliseconds: f64,
    pub image_errors: usize,
}

pub struct Selection<'a> {
    pub allocated: &'a HashSet<usize>,
    pub preview: &'a HashSet<usize>,
    pub matches: &'a HashSet<usize>,
    pub searching: bool,
    pub socketed: &'a HashSet<usize>,
    pub progression_marker: Option<usize>,
}

struct NodePaint {
    fill: Hsla,
    stroke: Hsla,
    width: f32,
    radius: f32,
}

fn node_paint(
    node: &Node,
    kind: TreeKind,
    allocated: bool,
    preview: bool,
    theme: TreeTheme,
) -> NodePaint {
    let root = node.t == "root";
    let notable = match kind {
        TreeKind::Incarnation => node.r >= 10.,
        TreeKind::Ether => node.t == "big",
    };
    let keystone = kind == TreeKind::Incarnation && node.r >= 12.;
    NodePaint {
        fill: if allocated {
            if keystone {
                theme.keystone()
            } else {
                theme.allocated()
            }
        } else if preview {
            theme.preview()
        } else if root {
            theme.root()
        } else {
            theme.node()
        },
        stroke: if allocated {
            theme.allocated_stroke()
        } else if preview {
            theme.accent()
        } else if root {
            theme.root_stroke()
        } else if keystone {
            theme.keystone_stroke()
        } else if notable {
            theme.notable_stroke()
        } else {
            theme.stroke()
        },
        width: if root {
            2.5
        } else if notable {
            2.
        } else {
            1.
        },
        radius: node.r + if root { 3. } else { 0. },
    }
}

// Warp links affect reachability but are deliberately invisible in Tauri.
fn visible_edge(graph: &Graph, a: usize, b: usize) -> bool {
    let warp = |index: usize| {
        graph
            .info
            .get(&graph.nodes[index].id)
            .is_some_and(|info| info.n == "warp")
    };
    !warp(a) || !warp(b)
}

fn cover_bounds(image: [f32; 2], viewport: [f32; 2]) -> [f32; 4] {
    let scale = (viewport[0] / image[0]).max(viewport[1] / image[1]);
    let width = image[0] * scale;
    let height = image[1] * scale;
    [
        (viewport[0] - width) / 2.,
        (viewport[1] - height) / 2.,
        width,
        height,
    ]
}

impl Scene {
    pub fn load() -> Self {
        Self::from_graph(Graph::load())
    }

    pub fn from_graph(graph: Graph) -> Self {
        let wanted: HashSet<_> = graph.nodes.iter().map(|n| n.icon.as_str()).collect();
        let mut icons = HashMap::new();
        let mut dim_icons = HashMap::new();
        for (key, data) in ICONS.iter().filter(|(key, _)| wanted.contains(key)) {
            let rgba = image::load_from_memory_with_format(data, image::ImageFormat::Png)
                .expect("valid embedded icon")
                .into_rgba8();
            icons.insert((*key).to_owned(), render_image(rgba.clone()));
            let mut dim = rgba;
            for pixel in dim.pixels_mut() {
                pixel.0[3] = (f32::from(pixel.0[3]) * TreeTheme::incarnation().search_opacity())
                    .round() as u8;
            }
            dim_icons.insert((*key).to_owned(), render_image(dim));
        }
        assert!(
            wanted.iter().all(|key| icons.contains_key(*key)),
            "missing node icon"
        );
        log::info!(
            "Loaded {} nodes, {} edges, {} icons",
            graph.nodes.len(),
            graph.edges.len(),
            icons.len()
        );
        let painted_edges = graph
            .edges
            .iter()
            .copied()
            .filter(|[a, b]| visible_edge(&graph, *a, *b))
            .collect();
        Self {
            graph,
            painted_edges,
            icons,
            dim_icons,
            background: BACKGROUND.clone(),
            vignette: VIGNETTE.clone(),
        }
    }

    pub fn icon(&self, node: &Node) -> Arc<RenderImage> {
        self.icons[&node.icon].clone()
    }

    pub fn paint(
        &self,
        camera: Camera,
        selection: Selection,
        theme: TreeTheme,
        bounds: Bounds<Pixels>,
        window: &mut Window,
    ) -> PaintStats {
        let start = Instant::now();
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        let mut stats = PaintStats::default();
        let position = |p: [f32; 2]| bounds.origin + point(px(p[0]), px(p[1]));
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            window.paint_quad(fill(bounds, theme.background()));
            let image = self.background.size(0);
            let [x, y, w, h] = cover_bounds(
                [image.width.0 as f32, image.height.0 as f32],
                [width, height],
            );
            if window
                .paint_image(
                    bounds,
                    Bounds::new(position([x, y]), size(px(w), px(h))),
                    px(0.).into(),
                    self.background.clone(),
                    0,
                    false,
                )
                .is_err()
            {
                stats.image_errors += 1;
            }
            // Widths and radii follow the SVG's world-space transform, including
            // overview zoom. Fixed minimum radii would turn the tree into a dot grid.
            let mut ordinary = PathBuilder::stroke(px(camera.scale * 1.5));
            let mut allocated = PathBuilder::stroke(px(camera.scale * 2.5));
            let mut preview = PathBuilder::stroke(px(camera.scale * 2.));
            for &[a, b] in &self.painted_edges {
                let na = &self.graph.nodes[a];
                let nb = &self.graph.nodes[b];
                let pa = camera.screen([na.x, na.y]);
                let pb = camera.screen([nb.x, nb.y]);
                if pa[0].max(pb[0]) < 0.
                    || pa[0].min(pb[0]) > width
                    || pa[1].max(pb[1]) < 0.
                    || pa[1].min(pb[1]) > height
                {
                    continue;
                }
                let both_allocated =
                    selection.allocated.contains(&a) && selection.allocated.contains(&b);
                let on_path = |id: &usize| selection.preview.contains(id);
                let builder = if both_allocated {
                    &mut allocated
                } else if on_path(&a) && on_path(&b) {
                    &mut preview
                } else {
                    &mut ordinary
                };
                builder.move_to(position(pa));
                builder.line_to(position(pb));
            }
            for (builder, color) in [
                (ordinary, theme.edge()),
                (allocated, theme.allocated_edge()),
                (preview, theme.preview_edge()),
            ] {
                if let Ok(path) = builder.build() {
                    window.paint_path(path, color);
                }
            }

            let opacity = if selection.searching {
                theme.search_opacity()
            } else {
                1.
            };
            for (i, node) in self.graph.nodes.iter().enumerate() {
                let p = camera.screen([node.x, node.y]);
                let paint = node_paint(
                    node,
                    self.graph.kind,
                    selection.allocated.contains(&i),
                    selection.preview.contains(&i),
                    theme,
                );
                let radius = paint.radius * camera.scale;
                let margin = radius + 6. * camera.scale;
                if p[0] + margin < 0.
                    || p[0] - margin > width
                    || p[1] + margin < 0.
                    || p[1] - margin > height
                {
                    continue;
                }
                stats.visible += 1;
                let circle = |r: f32| {
                    Bounds::new(position([p[0] - r, p[1] - r]), size(px(r * 2.), px(r * 2.)))
                };
                // SVG strokes straddle the radius; GPUI borders are inset.
                let outer = radius + paint.width * camera.scale / 2.;
                window.paint_quad(
                    fill(circle(outer), paint.fill.opacity(opacity))
                        .corner_radii(px(outer))
                        .border_widths(px(paint.width * camera.scale))
                        .border_color(paint.stroke.opacity(opacity)),
                );
                let icon = if selection.searching {
                    self.dim_icons[&node.icon].clone()
                } else {
                    self.icon(node)
                };
                let icon_radius = node.r * camera.scale;
                // Keep the full sprite, including its original frame and transparent
                // corners. The reference doesn't mask the icon to a smaller circle.
                if window
                    .paint_image(
                        circle(icon_radius),
                        circle(icon_radius),
                        px(0.).into(),
                        icon,
                        0,
                        false,
                    )
                    .is_err()
                {
                    stats.image_errors += 1;
                }
                if selection.matches.contains(&i) {
                    let r = (node.r + 4.) * camera.scale;
                    window.paint_quad(
                        fill(circle(r + 1.5 * camera.scale), theme.accent().opacity(0.))
                            .corner_radii(px(r + 1.5 * camera.scale))
                            .border_widths(px(3. * camera.scale))
                            .border_color(theme.accent()),
                    );
                }
                if selection.socketed.contains(&node.id) {
                    let r = (node.r + 4.) * camera.scale;
                    window.paint_quad(
                        fill(circle(r + camera.scale), theme.socket().opacity(0.))
                            .corner_radii(px(r + camera.scale))
                            .border_widths(px(2. * camera.scale))
                            .border_color(theme.socket()),
                    );
                }
                if selection.progression_marker == Some(i) {
                    let r = (node.r + 6.) * camera.scale;
                    let stroke = 2.5 * camera.scale;
                    window.paint_quad(
                        fill(circle(r + stroke / 2.), theme.accent().opacity(0.))
                            .corner_radii(px(r + stroke / 2.))
                            .border_widths(px(stroke))
                            .border_color(theme.accent().opacity(0.9)),
                    );
                }
            }
            if window
                .paint_image(
                    bounds,
                    bounds,
                    px(0.).into(),
                    self.vignette.clone(),
                    0,
                    false,
                )
                .is_err()
            {
                stats.image_errors += 1;
            }
        });
        stats.milliseconds = start.elapsed().as_secs_f64() * 1000.;
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_real_node_icon_can_be_decoded() {
        for graph in [Graph::load(), Graph::load_ether()] {
            let scene = Scene::from_graph(graph);
            for node in &scene.graph.nodes {
                let dimensions = scene.icon(node).size(0);
                assert!(dimensions.width.0 > 0 && dimensions.height.0 > 0);
            }
        }
    }

    #[test]
    fn background_covers_without_stretching_in_wide_and_tall_windows() {
        assert_eq!(
            cover_bounds([200., 100.], [100., 100.]),
            [-50., 0., 200., 100.]
        );
        assert_eq!(
            cover_bounds([200., 100.], [400., 100.]),
            [0., -50., 400., 200.]
        );
    }

    #[test]
    fn warp_connections_stay_in_graph_but_are_not_painted() {
        let graph = Graph::load();
        let hidden: Vec<_> = graph
            .edges
            .iter()
            .filter(|[a, b]| !visible_edge(&graph, *a, *b))
            .collect();
        assert_eq!(hidden.len(), 6);
        assert!(
            hidden
                .iter()
                .all(|[a, b]| graph.info[&graph.nodes[*a].id].n == "warp"
                    && graph.info[&graph.nodes[*b].id].n == "warp")
        );
    }
}
