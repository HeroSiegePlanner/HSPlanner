//! Greedy DPS optimizer for the Incarnation tree, driven by the engine's suggest_engine.
use super::*;
use gpui_kit::component::slider::SliderState;
use gpui_kit::{Div, Entity, FontWeight, SharedString, Window, relative, rems};
use hsplanner_engine::suggest_engine::{
    command::run_suggest_controlled,
    types::{SuggestInput, SuggestResult, SuggestStep, TreeGraph},
};
use hsplanner_engine::task_control::Cancellation;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

const MIN_BUDGET: u32 = 1;
const MAX_BUDGET: u32 = 200;
const DEFAULT_BUDGET: u32 = 10;
const PROGRESS_TICK: Duration = Duration::from_millis(120);

#[derive(Default, PartialEq)]
pub(super) enum Phase {
    #[default]
    Idle,
    Computing,
    Done,
    Failed(String),
}

#[derive(Default)]
pub(super) struct SuggestPanel {
    pub budget: u32,
    pub phase: Phase,
    pub result: Option<SuggestResult>,
    pub added: HashSet<usize>,
    run_id: u64,
    progress: Arc<(AtomicU32, AtomicU32)>,
    cancellation: Option<Cancellation>,
    unsupported_expanded: bool,
}

impl SuggestPanel {
    pub(super) fn new() -> Self {
        Self {
            budget: DEFAULT_BUDGET,
            ..Default::default()
        }
    }

    pub(super) fn clear(&mut self) {
        if let Some(cancellation) = self.cancellation.take() {
            cancellation.cancel();
        }
        self.run_id += 1;
        self.phase = Phase::Idle;
        self.result = None;
        self.added.clear();
        self.unsupported_expanded = false;
    }

    fn progress(&self) -> (u32, u32) {
        (
            self.progress.0.load(Ordering::Relaxed),
            self.progress.1.load(Ordering::Relaxed),
        )
    }
}

pub(super) fn slider_state() -> SliderState {
    SliderState::new()
        .min(MIN_BUDGET as f32)
        .max(MAX_BUDGET as f32)
        .step(1.)
        .default_value(DEFAULT_BUDGET as f32)
}

/// Same classification as the Tauri `treeGraph`/`treeSuggest` modules.
pub(super) fn tree_graph(graph: &tree::Graph) -> TreeGraph {
    let id = |index: usize| graph.nodes[index].id as u32;
    let kind = |index: usize| {
        graph
            .info
            .get(&graph.nodes[index].id)
            .map_or("", |info| info.n.as_str())
    };
    let mut adjacency: HashMap<u32, Vec<u32>> = graph
        .nodes
        .iter()
        .map(|node| (node.id as u32, Vec::new()))
        .collect();
    for [a, b] in &graph.edges {
        adjacency
            .get_mut(&id(*a))
            .expect("edge endpoint")
            .push(id(*b));
        adjacency
            .get_mut(&id(*b))
            .expect("edge endpoint")
            .push(id(*a));
    }
    let indices = 0..graph.nodes.len();
    let warp_ids: Vec<u32> = indices
        .clone()
        .filter(|&ix| kind(ix) == "warp")
        .map(id)
        .collect();
    let jewelry_ids: Vec<u32> = indices
        .clone()
        .filter(|&ix| kind(ix) == "jewelry")
        .map(id)
        .collect();
    let valuable_ids = indices
        .filter(|&ix| {
            let node = &graph.nodes[ix];
            let n = kind(ix);
            n != "warp"
                && n != "root"
                && node.t != "root"
                && (n == "jewelry" || n == "big" || node.r >= 10.)
        })
        .map(id)
        .collect();
    TreeGraph {
        adjacency,
        start_ids: graph.roots.iter().map(|&ix| id(ix)).collect(),
        warp_ids,
        valuable_ids,
        jewelry_ids,
    }
}

fn apply_order(current: &[u32], sequence: &[SuggestStep]) -> Vec<u32> {
    let mut ordered = current.to_vec();
    for step in sequence {
        if !ordered.contains(&step.node_id) {
            ordered.push(step.node_id);
        }
    }
    ordered
}

fn gain_percent(base: f64, final_dps: f64) -> String {
    if base <= 0. {
        return if final_dps > 0. {
            "+∞%".into()
        } else {
            "0%".into()
        };
    }
    let percent = (final_dps - base) / base * 100.;
    format!("{}{percent:.1}%", if percent > 0. { "+" } else { "" })
}

impl TreeView {
    pub(super) fn observe_suggest_budget(
        &mut self,
        slider: &Entity<SliderState>,
        cx: &mut Context<Self>,
    ) {
        self.subscriptions
            .push(cx.observe(slider, |this, slider, cx| {
                let budget = slider.read(cx).value().end().round() as u32;
                if this.suggest.budget != budget {
                    this.suggest.budget = budget.clamp(MIN_BUDGET, MAX_BUDGET);
                    cx.notify();
                }
            }));
    }

    pub(super) fn toggle_suggest(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.suggest_open = !self.suggest_open;
        if self.suggest_open {
            self.inspector_open = false;
            self.summary_open = false;
        } else {
            self.suggest.clear();
        }
        self.hovered = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn run_suggest(&mut self, cx: &mut Context<Self>) {
        self.suggest.clear();
        let run_id = self.suggest.run_id;
        let cancellation = Cancellation::default();
        let progress = Arc::new((AtomicU32::new(0), AtomicU32::new(self.suggest.budget)));
        self.suggest.cancellation = Some(cancellation.clone());
        self.suggest.progress = progress.clone();
        self.suggest.phase = Phase::Computing;
        let planner = self.build_input.planner_input();
        let mut perf = planner.build;
        perf.allocated_tree_nodes = self
            .selected
            .iter()
            .map(|&ix| self.scene.graph.nodes[ix].id as u32)
            .collect();
        let input = SuggestInput {
            perf,
            active_skill_ids: planner.active_skill_ids,
            graph: tree_graph(&self.scene.graph),
            budget: self.suggest.budget,
        };
        let task = cx.background_spawn(async move {
            run_suggest_controlled(&input, &cancellation, |current, total| {
                progress.0.store(current, Ordering::Relaxed);
                progress.1.store(total, Ordering::Relaxed);
            })
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.suggest.run_id != run_id {
                    return;
                }
                this.suggest.cancellation = None;
                match result {
                    Ok(result) => {
                        let ids: HashSet<u32> = result.added_nodes.iter().copied().collect();
                        this.suggest.added = this
                            .scene
                            .graph
                            .nodes
                            .iter()
                            .enumerate()
                            .filter(|(_, node)| ids.contains(&(node.id as u32)))
                            .map(|(ix, _)| ix)
                            .collect();
                        this.suggest.result = Some(result);
                        this.suggest.phase = Phase::Done;
                    }
                    Err(error) => this.suggest.phase = Phase::Failed(error),
                }
                cx.notify();
            });
        })
        .detach();
        // ponytail: progress lives in atomics; a short timer repaints while computing
        // instead of threading a channel through the engine callback.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(PROGRESS_TICK).await;
                let running = this
                    .update(cx, |this, cx| {
                        let running =
                            this.suggest.run_id == run_id && this.suggest.phase == Phase::Computing;
                        if running {
                            cx.notify();
                        }
                        running
                    })
                    .unwrap_or(false);
                if !running {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    fn apply_suggest(&mut self, cx: &mut Context<Self>) {
        let Some(result) = self
            .suggest
            .result
            .as_ref()
            .filter(|result| !result.added_nodes.is_empty())
        else {
            return;
        };
        let nodes = apply_order(
            self.scene.graph.kind.nodes(&self.build_input),
            &result.sequence,
        );
        self.session.update(cx, |session, cx| {
            session.edit(|draft| self.scene.graph.kind.apply(&mut draft.snapshot, &nodes));
            cx.notify();
        });
        self.sync_document(cx);
        self.suggest.clear();
        cx.notify();
    }

    pub(super) fn suggest_panel(&self, window: &Window, cx: &Context<Self>) -> Div {
        let p = cx.global::<theme::TooltipTheme>();
        let accent = self.tree_theme().accent();
        let computing = self.suggest.phase == Phase::Computing;
        let (current, total) = self.suggest.progress();
        let mono = |id: &'static str, text: String, color| {
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(10. / 13.))
                .text_color(color)
                .child(tooltip_text::TooltipText::new(id, text, 0.14))
        };
        let status = match &self.suggest.phase {
            Phase::Idle => "Greedy DPS optimizer".to_string(),
            Phase::Computing => format!("Step {current} / {total}"),
            Phase::Done => self.suggest.result.as_ref().map_or(String::new(), |r| {
                format!("Used {} of {}", r.budget_used, r.budget_requested)
            }),
            Phase::Failed(_) => "Last run errored".to_string(),
        };
        let can_apply = self
            .suggest
            .result
            .as_ref()
            .is_some_and(|r| !r.added_nodes.is_empty());
        let mut panel = div()
            .w(rems(26.))
            .max_h_full()
            .flex()
            .flex_col()
            .rounded_sm()
            .border_1()
            .border_color(p.border)
            .bg(p.panel)
            .text_color(p.text)
            .child(
                div()
                    .p_4()
                    .border_b_1()
                    .border_color(p.border)
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(mono(
                        "suggest-eyebrow",
                        "TALENT TREE OPTIMIZER".into(),
                        p.faint,
                    ))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.accent_hot)
                            .child("Suggest Nodes"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(mono(
                                "suggest-budget-label",
                                "NODES TO ALLOCATE".into(),
                                p.faint,
                            ))
                            .child(
                                div()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .text_color(p.accent_hot)
                                    .child(self.suggest.budget.to_string()),
                            ),
                    )
                    .child(
                        hsplanner_ui::controls::planner_slider(&self.suggest_slider, window, cx)
                            .w_full(),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(mono("suggest-status", status.to_uppercase(), p.faint))
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .when(self.suggest.phase == Phase::Done, |row| {
                                        row.child(
                                            hsplanner_ui::controls::planner_button(
                                                "suggest-reset",
                                                hsplanner_ui::controls::ButtonTone::Neutral,
                                                cx,
                                            )
                                            .small()
                                            .label("Reset")
                                            .on_click(
                                                cx.listener(|this, _, _, cx| {
                                                    this.suggest.clear();
                                                    cx.notify();
                                                }),
                                            ),
                                        )
                                    })
                                    .child(
                                        hsplanner_ui::controls::planner_button(
                                            "suggest-run",
                                            if computing {
                                                hsplanner_ui::controls::ButtonTone::Danger
                                            } else {
                                                hsplanner_ui::controls::ButtonTone::Primary
                                            },
                                            cx,
                                        )
                                        .small()
                                        .label(match self.suggest.phase {
                                            Phase::Computing => "Cancel",
                                            Phase::Done => "Recalculate",
                                            _ => "Calculate",
                                        })
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                if computing {
                                                    this.suggest.clear();
                                                    cx.notify();
                                                } else {
                                                    this.run_suggest(cx);
                                                }
                                            }),
                                        ),
                                    ),
                            ),
                    )
                    .when(computing, |section| {
                        let fraction = if total > 0 {
                            (current as f32 / total as f32).min(1.)
                        } else {
                            0.
                        };
                        section.child(
                            div()
                                .h(rems(6. / 13.))
                                .w_full()
                                .rounded_sm()
                                .border_1()
                                .border_color(p.border)
                                .bg(p.panel_secondary)
                                .child(
                                    div().h_full().w(relative(fraction)).rounded_sm().bg(accent),
                                ),
                        )
                    }),
            );
        if let Phase::Failed(error) = &self.suggest.phase {
            panel = panel.child(
                div()
                    .px_4()
                    .py_3()
                    .text_sm()
                    .text_color(p.negative)
                    .child(error.clone()),
            );
        }
        panel = match (&self.suggest.phase, &self.suggest.result) {
            (Phase::Done, Some(result)) => panel.child(self.suggest_results(result, cx)),
            _ => panel.child(div().px_4().py_5().text_sm().text_color(p.muted).child(
                "Choose how many nodes to add. The optimizer walks the frontier of your allocation and greedily picks the highest-DPS neighbour each step, falling back to the shortest path toward a notable or jewelry socket when no immediate gain is available.",
            )),
        };
        panel.child(
            div()
                .p_3()
                .border_t_1()
                .border_color(p.border)
                .flex()
                .items_center()
                .justify_between()
                .child(mono(
                    "suggest-footer",
                    match &self.suggest.phase {
                        Phase::Done => format!("{} NODES READY", self.suggest.added.len()),
                        Phase::Computing => "OPTIMIZING".into(),
                        Phase::Failed(_) => "ERROR".into(),
                        Phase::Idle => "CONFIGURE BUDGET".into(),
                    },
                    match &self.suggest.phase {
                        Phase::Done => p.accent_hot,
                        Phase::Failed(_) => p.negative,
                        _ => p.faint,
                    },
                ))
                .child(
                    hsplanner_ui::controls::planner_button(
                        "suggest-apply",
                        hsplanner_ui::controls::ButtonTone::Primary,
                        cx,
                    )
                    .small()
                    .label("Apply")
                    .disabled(!can_apply)
                    .on_click(cx.listener(|this, _, _, cx| this.apply_suggest(cx))),
                ),
        )
    }

    fn suggest_results(&self, result: &SuggestResult, cx: &Context<Self>) -> Div {
        let p = cx.global::<theme::TooltipTheme>();
        let scale = self.session.read(cx).state().settings.number_scale.clone();
        let dps = |value: f64| hsplanner_ui::numbers::compact(value, &scale);
        let gain = result.final_dps - result.base_dps;
        let stat = |label: &'static str, value: String, sub: Option<String>, color| {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(rems(9. / 13.))
                        .text_color(p.faint)
                        .child(label),
                )
                .child(
                    div()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_color(color)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(value),
                )
                .children(sub.map(|sub| div().text_xs().text_color(color).child(sub)))
        };
        let mut section = div().flex().flex_col().min_h_0().child(
            div()
                .flex()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(p.border)
                .child(stat("BASE DPS", dps(result.base_dps), None, p.text))
                .child(stat("FINAL DPS", dps(result.final_dps), None, p.accent_hot))
                .child(stat(
                    "GAIN",
                    format!("{}{}", if gain > 0. { "+" } else { "" }, dps(gain)),
                    Some(gain_percent(result.base_dps, result.final_dps)),
                    if gain > 0. { p.positive } else { p.muted },
                )),
        );
        let rows: Vec<Div> = result
            .sequence
            .iter()
            .enumerate()
            .map(|(index, step)| self.suggest_row(index, step, cx))
            .collect();
        section = section.child(
            div()
                .id("suggest-sequence")
                .overflow_y_scroll()
                .max_h(rems(20.))
                .when(rows.is_empty(), |list| {
                    list.child(
                        div()
                            .p_6()
                            .text_center()
                            .text_sm()
                            .text_color(p.muted)
                            .child("No improvements found within budget"),
                    )
                })
                .children(rows),
        );
        if !result.unsupported_lines.is_empty() {
            let mut lines: Vec<&String> = result.unsupported_lines.iter().collect();
            lines.dedup();
            let expanded = self.suggest.unsupported_expanded;
            let shown = if expanded { lines.len() } else { 0 };
            section =
                section.child(
                    div()
                        .id("suggest-unsupported")
                        .px_4()
                        .py_2()
                        .border_t_1()
                        .border_color(p.stat_orange.opacity(0.4))
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(rems(10. / 13.))
                        .text_color(p.stat_orange)
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.suggest.unsupported_expanded = !this.suggest.unsupported_expanded;
                            cx.notify();
                        }))
                        .child(format!(
                            "▲ {} unsupported mod line{} (treated as 0 DPS) {}",
                            result.unsupported_lines.len(),
                            if result.unsupported_lines.len() == 1 {
                                ""
                            } else {
                                "s"
                            },
                            if expanded { "−" } else { "+" }
                        ))
                        .children(lines.into_iter().take(shown).map(|line| {
                            div().pl_4().text_color(p.muted).child(format!("· {line}"))
                        })),
                );
        }
        section
    }

    fn suggest_row(&self, index: usize, step: &SuggestStep, cx: &Context<Self>) -> Div {
        let p = cx.global::<theme::TooltipTheme>();
        let info = self.scene.graph.info.get(&(step.node_id as usize));
        let name = info
            .map(|info| info.t.trim())
            .filter(|name| !name.is_empty())
            .map_or_else(|| format!("Node #{}", step.node_id), str::to_string);
        let (badge, badge_color) = match info.map(|info| info.n.as_str()) {
            Some("jewelry") => ("SOCKET", p.stat_blue),
            Some("big") => ("NOTABLE", p.accent_hot),
            Some(_) => ("MINOR", p.muted),
            None => ("NODE", p.faint),
        };
        let scale = self.session.read(cx).state().settings.number_scale.clone();
        let (gain, gain_color) = if step.is_filler {
            ("Path".to_string(), p.faint)
        } else {
            (
                format!("+{} DPS", hsplanner_ui::numbers::compact(step.gain, &scale)),
                if step.gain > 0. { p.positive } else { p.muted },
            )
        };
        div()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_1p5()
            .border_b_1()
            .border_color(p.border)
            .child(
                div()
                    .w(rems(1.6))
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(rems(10. / 13.))
                    .text_color(p.faint)
                    .child(format!("{:02}", index + 1)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .child(SharedString::from(name)),
            )
            .child(
                div()
                    .px_1p5()
                    .rounded_sm()
                    .border_1()
                    .border_color(badge_color.opacity(0.5))
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(rems(9. / 13.))
                    .text_color(badge_color)
                    .child(badge),
            )
            .child(
                div()
                    .w(rems(6.))
                    .text_right()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(rems(11. / 13.))
                    .text_color(gain_color)
                    .child(gain),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{SuggestStep, apply_order, gain_percent, tree_graph};
    use crate::tree;
    use std::collections::HashSet;

    #[test]
    fn graph_payload_matches_the_tauri_classification() {
        let graph = tree::Graph::load();
        let payload = tree_graph(&graph);
        assert!(!payload.start_ids.is_empty());
        assert!(!payload.jewelry_ids.is_empty());
        let valuable: HashSet<_> = payload.valuable_ids.iter().collect();
        assert!(payload.jewelry_ids.iter().all(|id| valuable.contains(id)));
        assert!(payload.warp_ids.iter().all(|id| !valuable.contains(id)));
        assert!(payload.start_ids.iter().all(|id| !valuable.contains(id)));
        for (id, neighbours) in &payload.adjacency {
            for neighbour in neighbours {
                assert!(
                    payload.adjacency[neighbour].contains(id),
                    "edge {id}-{neighbour} is one-way"
                );
            }
        }
        let notable_by_radius = graph
            .nodes
            .iter()
            .filter(|node| node.r >= 10. && node.t != "root")
            .filter(|node| graph.info.get(&node.id).is_none_or(|info| info.n != "warp"))
            .count();
        assert!(payload.valuable_ids.len() >= notable_by_radius);
    }

    #[test]
    fn engine_suggests_reachable_nodes_within_budget_for_the_example_build() {
        let graph = tree::Graph::load();
        let planner = crate::build_session::example_input().planner_input();
        let input = super::SuggestInput {
            perf: planner.build,
            active_skill_ids: planner.active_skill_ids,
            graph: tree_graph(&graph),
            budget: 2,
        };
        let cancellation = super::Cancellation::default();
        let result = super::run_suggest_controlled(&input, &cancellation, |_, _| {}).unwrap();
        assert!(result.added_nodes.len() <= 2, "{:?}", result.added_nodes);
        assert_eq!(result.added_nodes.len(), result.sequence.len());
        assert!(result.final_dps >= result.base_dps);
        assert!(
            result
                .added_nodes
                .iter()
                .all(|id| input.graph.adjacency.contains_key(id))
        );
    }

    #[test]
    fn apply_keeps_current_order_and_appends_new_nodes_once() {
        let step = |node_id| SuggestStep {
            node_id,
            dps_before: 0.,
            dps_after: 0.,
            gain: 0.,
            is_filler: false,
        };
        let ordered = apply_order(&[5, 7], &[step(7), step(9), step(11), step(9)]);
        assert_eq!(ordered, vec![5, 7, 9, 11]);
    }

    #[test]
    fn gain_percent_handles_zero_base() {
        assert_eq!(gain_percent(100., 125.), "+25.0%");
        assert_eq!(gain_percent(100., 90.), "-10.0%");
        assert_eq!(gain_percent(0., 5.), "+∞%");
        assert_eq!(gain_percent(0., 0.), "0%");
    }
}
