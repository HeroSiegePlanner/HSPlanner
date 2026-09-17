//! A display-only projection of saved allocation order; the document stays intact.
use super::*;
use gpui_kit::component::slider::SliderState;
use gpui_kit::{Div, Entity, KeyDownEvent, SharedString, rems};

fn progression_order(
    allocated: &[usize],
    roots: &HashSet<usize>,
    adjacency: &HashMap<usize, Vec<usize>>,
) -> Vec<usize> {
    let mut remaining = allocated.to_vec();
    let mut result = Vec::with_capacity(remaining.len());
    let mut taken = HashSet::new();
    while let Some(index) = remaining.iter().position(|id| {
        roots.contains(id)
            || adjacency
                .get(id)
                .is_some_and(|neighbors| neighbors.iter().any(|id| taken.contains(id)))
    }) {
        let id = remaining.remove(index);
        taken.insert(id);
        result.push(id);
    }
    result.extend(remaining);
    result
}

fn graph_order(graph: &tree::Graph, allocated: &[u32]) -> Vec<usize> {
    let indices: HashMap<_, _> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id as u32, index))
        .collect();
    let allocated = allocated
        .iter()
        .filter_map(|id| indices.get(id).copied())
        .collect::<Vec<_>>();
    let mut adjacency: HashMap<_, Vec<_>> = HashMap::new();
    for &[a, b] in &graph.edges {
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
    }
    progression_order(
        &allocated,
        &graph.roots.iter().copied().collect(),
        &adjacency,
    )
}

#[derive(Default)]
pub(super) struct ProgressionPreview {
    order: Vec<usize>,
    step: Option<usize>,
    visible: HashSet<usize>,
}

impl ProgressionPreview {
    pub(super) fn new(graph: &tree::Graph, allocated: &[u32]) -> Self {
        let mut preview = Self::default();
        preview.reset(graph_order(graph, allocated));
        preview
    }

    fn reset(&mut self, order: Vec<usize>) {
        self.order = order;
        self.step = None;
        self.visible = self.order.iter().copied().collect();
    }

    pub(super) fn total(&self) -> usize {
        self.order.len()
    }
    pub(super) fn current(&self) -> usize {
        self.step.unwrap_or(self.total())
    }
    pub(super) fn is_preview(&self) -> bool {
        self.step.is_some()
    }
    pub(super) fn marker(&self) -> Option<usize> {
        self.step.and_then(|step| self.order.get(step - 1).copied())
    }
    pub(super) fn visible(&self) -> &HashSet<usize> {
        &self.visible
    }

    fn set_step(&mut self, step: usize) -> bool {
        let next = (self.total() >= 2 && step < self.total()).then(|| step.max(1));
        if next == self.step {
            return false;
        }
        self.step = next;
        self.visible = self.order[..self.current()].iter().copied().collect();
        true
    }
}

pub(super) fn slider_state(total: usize) -> SliderState {
    SliderState::new()
        .min(1.)
        .max(total.max(2) as f32)
        .step(1.)
        .default_value(total.max(1) as f32)
}

impl TreeView {
    pub(super) fn observe_progression(
        &mut self,
        slider: &Entity<SliderState>,
        cx: &mut Context<Self>,
    ) {
        // Observe values, including native accessibility increments, which do not
        // emit SliderEvent::Change in the pinned base component.
        self.subscriptions
            .push(cx.observe(slider, |this, slider, cx| {
                let step = slider.read(cx).value().end().round() as usize;
                if this.progression.set_step(step) {
                    this.clear_progression_hover();
                    cx.notify();
                }
            }));
    }

    fn clear_progression_hover(&mut self) {
        self.hovered = None;
        self.example = None;
        self.inspected = None;
        self.drag = None;
        self.dragged = false;
        // Keep the real build request/results. Scrubbing never schedules a
        // partial-build calculation; late hover results have no visible target.
    }

    pub(super) fn reset_progression(&mut self, cx: &mut Context<Self>) {
        self.progression = ProgressionPreview::new(
            &self.scene.graph,
            self.scene.graph.kind.nodes(&self.build_input),
        );
        self.clear_progression_hover();
        self.progression_slider.update(cx, |slider, cx| {
            *slider = slider_state(self.progression.total());
            cx.notify();
        });
    }

    /// Consumes a node click/keyboard allocation while viewing an earlier step.
    pub(super) fn finish_progression(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.progression.is_preview() {
            return false;
        }
        self.progression.set_step(self.progression.total());
        self.clear_progression_hover();
        self.progression_slider.update(cx, |slider, cx| {
            *slider = slider_state(self.progression.total());
            cx.notify();
        });
        cx.notify();
        true
    }

    fn progression_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.progression_focus.is_focused(window) {
            return;
        }
        let current = self.progression.current();
        let total = self.progression.total();
        let next = match event.keystroke.key.as_str() {
            "left" | "down" => current.saturating_sub(1).max(1),
            "right" | "up" => current.saturating_add(1).min(total),
            "home" => 1,
            "end" | "escape" => total,
            _ => return,
        };
        self.progression_slider
            .update(cx, |slider, cx| slider.set_value(next as f32, window, cx));
        if event.keystroke.key == "escape" {
            window.focus(&self.focus, cx);
        }
        cx.stop_propagation();
    }

    pub(super) fn progression_bar(&self, window: &Window, cx: &Context<Self>) -> Div {
        let p = cx.global::<theme::TooltipTheme>();
        let total = self.progression.total();
        if total < 2 {
            return div();
        }
        let current = self.progression.current();
        // WebKit uses its 129px intrinsic range width after the global width rule.
        let width = (window.rem_size() * (129. / 13.)).min(window.viewport_size().width * 0.38);
        let stack_controls = self.dimensions()[0] < f32::from(window.rem_size()) * (940. / 13.);
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom_3p5()
            .when(stack_controls, |view| view.bottom(rems(3.5)))
            .flex()
            .justify_center()
            .child(
                div()
                    .id("tree-progression")
                    .occlude()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_1p5()
                    .rounded_sm()
                    .border_1()
                    .border_color(p.border)
                    .bg(gpui_kit::linear_gradient(
                        180.,
                        gpui_kit::linear_color_stop(p.panel_secondary.opacity(0.8), 0.),
                        gpui_kit::linear_color_stop(p.background.opacity(0.7), 1.),
                    ))
                    .track_focus(&self.progression_focus)
                    .role(gpui_kit::accesskit::Role::Group)
                    .aria_label(trf(
                        "tree.progression_step",
                        &[
                            ("current", current.to_string()),
                            ("total", total.to_string()),
                        ],
                    ))
                    .when(self.progression_focus.is_focused(window), |view| {
                        view.border_color(self.tree_theme().accent())
                    })
                    .capture_any_mouse_down(cx.listener(
                        |this, event: &MouseDownEvent, window, cx| {
                            if event.button == MouseButton::Left {
                                window.focus(&this.progression_focus, cx);
                            }
                        },
                    ))
                    .on_key_down(cx.listener(Self::progression_key))
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(10. / 13.))
                            .text_color(p.faint)
                            .child(tooltip_text::TooltipText::new(
                                "progression-heading",
                                tr("tree.progression"),
                                0.14,
                            )),
                    )
                    .child(
                        hsplanner_ui::controls::planner_slider(
                            &self.progression_slider,
                            window,
                            cx,
                        )
                        .w(width),
                    )
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(10. / 13.))
                            .text_color(if self.progression.is_preview() {
                                self.tree_theme().accent()
                            } else {
                                p.faint
                            })
                            .child(tooltip_text::TooltipText::new(
                                SharedString::from("progression-count"),
                                format!("{current} / {total}"),
                                0.14,
                            )),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(allocated: &[usize], roots: &[usize], edges: &[[usize; 2]]) -> Vec<usize> {
        let mut adjacent: HashMap<_, Vec<_>> = HashMap::new();
        for &[a, b] in edges {
            adjacent.entry(a).or_default().push(b);
            adjacent.entry(b).or_default().push(a);
        }
        progression_order(allocated, &roots.iter().copied().collect(), &adjacent)
    }

    #[test]
    fn connects_imported_order_without_sorting_branch_choices() {
        assert_eq!(
            order(&[4, 1, 3, 2], &[1], &[[1, 2], [2, 4], [1, 3]]),
            [1, 3, 2, 4]
        );
        assert_eq!(
            order(&[1, 4, 2, 3], &[1], &[[1, 2], [2, 4], [1, 3]]),
            [1, 2, 4, 3]
        );
    }

    #[test]
    fn keeps_disconnected_nodes_and_multiple_roots_in_reference_order() {
        assert_eq!(order(&[9, 8, 1], &[1], &[]), [1, 9, 8]);
        assert_eq!(
            order(&[8, 2, 7, 1], &[1, 7], &[[7, 8], [1, 2]]),
            [7, 8, 1, 2]
        );
    }

    #[test]
    fn scrubbing_changes_only_prefix_and_marker_and_full_step_clears_marker() {
        let saved = vec![1, 2, 4, 3];
        let mut preview = ProgressionPreview::default();
        preview.reset(saved.clone());
        assert!(preview.set_step(3));
        assert_eq!(preview.visible(), &HashSet::from([1, 2, 4]));
        assert_eq!(preview.marker(), Some(4));
        assert_eq!(saved, [1, 2, 4, 3]);
        assert!(preview.set_step(0));
        assert_eq!(preview.current(), 1);
        assert_eq!(preview.marker(), Some(1));
        assert!(preview.set_step(4));
        assert!(!preview.is_preview());
        assert_eq!(preview.marker(), None);
        assert_eq!(preview.visible(), &HashSet::from([1, 2, 3, 4]));
    }

    #[test]
    fn document_reset_clears_preview_even_when_allocations_are_identical() {
        let mut preview = ProgressionPreview::default();
        preview.reset(vec![1, 2, 3]);
        preview.set_step(1);
        preview.reset(vec![1, 2, 3]);
        assert!(!preview.is_preview());
        assert_eq!(preview.current(), 3);
        preview.reset(vec![7]);
        assert!(!preview.set_step(0));
        assert_eq!(preview.marker(), None);
    }

    #[test]
    fn both_real_trees_preserve_domain_node_order_in_visible_prefixes() {
        for graph in [tree::Graph::load(), tree::Graph::load_ether()] {
            let root = graph.roots[0];
            let neighbor = graph
                .edges
                .iter()
                .find_map(|&[a, b]| {
                    if a == root {
                        Some(b)
                    } else if b == root {
                        Some(a)
                    } else {
                        None
                    }
                })
                .unwrap();
            let saved = vec![graph.nodes[neighbor].id as u32, graph.nodes[root].id as u32];
            let mut preview = ProgressionPreview::new(&graph, &saved);
            assert_eq!(preview.total(), 2);
            preview.set_step(1);
            assert_eq!(preview.marker(), Some(root));
            assert_eq!(preview.visible(), &HashSet::from([root]));
        }
    }
}
