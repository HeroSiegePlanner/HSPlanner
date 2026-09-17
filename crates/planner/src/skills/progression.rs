//! Local rank projection. Native rank maps retain counts, not JavaScript key order.
use super::*;
use hsplanner_build::session::Draft;
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::tooltip::CursorTooltipExt;
use std::collections::HashSet;

const ORDER_EXPLANATION: &str = "planner.progression.help";

/// Tauri's rankPointOrder prerequisite repair, using authored class order in place
/// of the insertion order lost by the native HashMap. Keep rank blocks compact.
fn rank_order(
    ranks: &HashMap<String, u32>,
    authored: &[String],
    requires: &HashMap<String, String>,
) -> Vec<String> {
    let mut included = HashSet::new();
    let mut remaining: Vec<_> = authored
        .iter()
        .filter(|id| ranks.contains_key(*id) && included.insert((*id).clone()))
        .cloned()
        .collect();
    // Preserve unrecognized imported counts too, without random HashMap order.
    let mut extra: Vec<_> = ranks
        .keys()
        .filter(|id| !included.contains(*id))
        .cloned()
        .collect();
    extra.sort();
    remaining.extend(extra);
    let mut taken = HashSet::new();
    let mut ordered = Vec::with_capacity(remaining.len());
    while let Some(index) = remaining.iter().position(|id| {
        requires.get(id).is_none_or(|prerequisite| {
            !ranks.contains_key(prerequisite) || taken.contains(prerequisite)
        })
    }) {
        let id = remaining.remove(index);
        taken.insert(id.clone());
        ordered.push(id);
    }
    // Cyclic/imported requirements must not make saved points disappear.
    ordered.extend(remaining);
    ordered
}

pub(super) struct RankProgression {
    identity: (Option<String>, Option<String>, Option<String>),
    source: HashMap<String, u32>,
    prefix: Option<String>,
    order: Vec<String>,
    total: usize,
    step: Option<usize>,
    visible: HashMap<String, u32>,
    marker: Option<String>,
}

impl RankProgression {
    pub(super) fn for_draft(draft: &Draft) -> Self {
        let skills = data::get_skills_by_class(draft.snapshot.class_id.as_deref().unwrap_or(""));
        let authored: Vec<_> = skills.iter().map(|skill| skill.id.clone()).collect();
        let requires = skills
            .iter()
            .filter_map(|skill| {
                skill
                    .requires_skill
                    .as_ref()
                    .map(|id| (skill.id.clone(), id.clone()))
            })
            .collect();
        Self::new(
            (
                draft.build_id.clone(),
                draft.profile_id.clone(),
                draft.snapshot.class_id.clone(),
            ),
            draft.snapshot.skill_ranks.clone(),
            &authored,
            &requires,
        )
    }

    pub(super) fn for_subtree(draft: &Draft, skill: &SkillSpec) -> Self {
        let prefix = format!("{}:", skill.id);
        let authored = skill
            .subskills
            .iter()
            .flatten()
            .map(|node| format!("{prefix}{}", node.id))
            .collect::<Vec<_>>();
        let source = draft
            .snapshot
            .subskill_ranks
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(key, rank)| (key.clone(), *rank))
            .collect();
        let mut result = Self::new(
            (
                draft.build_id.clone(),
                draft.profile_id.clone(),
                draft.snapshot.class_id.clone(),
            ),
            source,
            &authored,
            &HashMap::new(),
        );
        result.prefix = Some(prefix);
        result
    }

    fn new(
        identity: (Option<String>, Option<String>, Option<String>),
        source: HashMap<String, u32>,
        authored: &[String],
        requires: &HashMap<String, String>,
    ) -> Self {
        Self {
            identity,
            order: rank_order(&source, authored, requires),
            total: source.values().map(|rank| *rank as usize).sum(),
            visible: source.clone(),
            source,
            prefix: None,
            step: None,
            marker: None,
        }
    }

    fn matches(&self, draft: &Draft) -> bool {
        self.identity.0 == draft.build_id
            && self.identity.1 == draft.profile_id
            && self.identity.2 == draft.snapshot.class_id
            && if let Some(prefix) = &self.prefix {
                self.source
                    .iter()
                    .all(|(key, rank)| draft.snapshot.subskill_ranks.get(key) == Some(rank))
                    && self.source.len()
                        == draft
                            .snapshot
                            .subskill_ranks
                            .keys()
                            .filter(|key| key.starts_with(prefix))
                            .count()
            } else {
                self.source == draft.snapshot.skill_ranks
            }
    }

    pub(super) fn total(&self) -> usize {
        self.total
    }
    fn current(&self) -> usize {
        self.step.unwrap_or(self.total)
    }
    pub(super) fn is_preview(&self) -> bool {
        self.step.is_some()
    }
    pub(super) fn visible(&self) -> &HashMap<String, u32> {
        &self.visible
    }
    pub(super) fn marker(&self) -> Option<&str> {
        self.marker.as_deref()
    }

    pub(super) fn set_step(&mut self, step: usize) -> bool {
        let next = (self.total >= 2 && step < self.total).then(|| step.max(1));
        if self.step == next {
            return false;
        }
        self.step = next;
        self.marker = None;
        if self.step.is_none() {
            self.visible.clone_from(&self.source);
            return true;
        }
        self.visible.clear();
        let mut remaining = self.current();
        for id in &self.order {
            let count = remaining.min(self.source[id] as usize);
            if count > 0 {
                self.visible.insert(id.clone(), count as u32);
                self.marker = Some(id.clone());
                remaining -= count;
            }
            if remaining == 0 {
                break;
            }
        }
        true
    }

    fn finish(&mut self) -> bool {
        self.set_step(self.total)
    }
}

pub(super) fn slider_state(total: usize) -> SliderState {
    SliderState::new()
        .min(1.)
        .max(total.max(2) as f32)
        .step(1.)
        .default_value(total.max(1) as f32)
}

impl SkillsView {
    pub(super) fn sync_progression(&mut self, cx: &mut Context<Self>) {
        let draft = self.session.read(cx).draft();
        if self.progression.matches(draft) {
            return;
        }
        self.progression = RankProgression::for_draft(draft);
        self.reset_progression_slider(cx);
    }

    fn reset_progression_slider(&self, cx: &mut Context<Self>) {
        self.progression_slider.update(cx, |slider, cx| {
            *slider = slider_state(self.progression.total());
            cx.notify();
        });
    }

    pub(super) fn finish_progression(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.progression.finish() {
            return false;
        }
        self.reset_progression_slider(cx);
        self.refresh_detail(cx);
        cx.notify();
        true
    }

    pub(super) fn progression_bar(&self, window: &Window, cx: &Context<Self>) -> Div {
        render_bar(
            &self.progression,
            &self.progression_slider,
            &self.progression_focus,
            window,
            cx,
        )
    }
}

impl SubtreeView {
    pub(super) fn sync_progression(&mut self, cx: &mut Context<Self>) {
        let draft = self.session.read(cx).draft();
        if self.progression.matches(draft) {
            return;
        }
        self.progression = RankProgression::for_subtree(draft, &self.skill);
        self.reset_progression_slider(cx);
    }

    fn reset_progression_slider(&self, cx: &mut Context<Self>) {
        self.progression_slider.update(cx, |slider, cx| {
            *slider = slider_state(self.progression.total());
            cx.notify();
        });
    }

    pub(super) fn clear_node_preview(&mut self) {
        self.preview = None;
        self.preview_task = None;
        self.preview_request.cancel();
    }

    pub(super) fn finish_progression(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.progression.finish() {
            return false;
        }
        self.clear_node_preview();
        self.reset_progression_slider(cx);
        cx.notify();
        true
    }

    pub(super) fn progression_bar(&self, window: &Window, cx: &Context<Self>) -> Div {
        render_bar(
            &self.progression,
            &self.progression_slider,
            &self.progression_focus,
            window,
            cx,
        )
    }
}

fn render_bar(
    progression: &RankProgression,
    slider: &Entity<SliderState>,
    focus: &FocusHandle,
    window: &Window,
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    let total = progression.total();
    if total < 2 {
        return div();
    }
    let current = progression.current();
    let pointer_focus = focus.clone();
    let keyboard_focus = focus.clone();
    let keyboard_slider = slider.clone();
    let cancel_focus = focus.clone();
    let cancel_slider = slider.clone();
    // Match the actual WebKit intrinsic range width after the reference CSS.
    let width = (window.rem_size() * (129. / 13.)).min(window.viewport_size().width * 0.38);
    div()
        .absolute()
        .left_0()
        .right_0()
        .bottom_3p5()
        .flex()
        .justify_center()
        .child(
            div()
                .id("skills-progression")
                .occlude()
                .flex()
                .max_w_full()
                .min_w_0()
                .items_center()
                .gap_3()
                .px_3()
                .py_1p5()
                .rounded_sm()
                .border_1()
                .border_color(p.border)
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(p.panel_secondary.opacity(0.8), 0.),
                    linear_color_stop(p.background.opacity(0.7), 1.),
                ))
                .track_focus(focus)
                .role(accesskit::Role::Group)
                .aria_label(trf(
                    "planner.progression.step",
                    &[
                        ("current", current.to_string()),
                        ("total", total.to_string()),
                        ("explanation", tr(ORDER_EXPLANATION).to_owned()),
                    ],
                ))
                .when(focus.is_focused(window), |bar| {
                    bar.border_color(p.accent_hot)
                })
                .capture_any_mouse_down(move |event: &MouseDownEvent, window, cx| {
                    if event.button == MouseButton::Left {
                        window.focus(&pointer_focus, cx);
                    }
                })
                .on_action(move |_: &gpui_kit::base::actions::Cancel, window, cx| {
                    // Dialog Escape dispatches Cancel before raw key events. Consume
                    // only an active projection; the next Escape closes normally.
                    if !cancel_focus.is_focused(window)
                        || cancel_slider.read(cx).value().end().round() as usize >= total
                    {
                        cx.propagate();
                        return;
                    }
                    cancel_slider
                        .update(cx, |slider, cx| slider.set_value(total as f32, window, cx));
                })
                .on_key_down(move |event, window, cx| {
                    if !keyboard_focus.is_focused(window) {
                        return;
                    }
                    let current = keyboard_slider.read(cx).value().end().round() as usize;
                    let next = match event.keystroke.key.as_str() {
                        "left" | "down" => current.saturating_sub(1).max(1),
                        "right" | "up" => current.saturating_add(1).min(total),
                        "home" => 1,
                        "end" | "escape" => total,
                        _ => return,
                    };
                    keyboard_slider
                        .update(cx, |slider, cx| slider.set_value(next as f32, window, cx));
                    cx.stop_propagation();
                })
                .child(
                    div()
                        .id("skills-progression-help")
                        .flex_shrink_0()
                        .whitespace_nowrap()
                        .cursor_tooltip_view(|window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(tr(ORDER_EXPLANATION))
                                .build(window, cx)
                        })
                        .child(caption(
                            "skills-progression-heading",
                            tr("planner.progression.heading"),
                            p.faint,
                        )),
                )
                .child(
                    hsplanner_ui::controls::planner_slider(slider, window, cx)
                        .w(width)
                        .flex_shrink_1()
                        .min_w_0(),
                )
                .child(
                    caption(
                        "skills-progression-count",
                        format!("{current} / {total}"),
                        if progression.is_preview() {
                            p.accent_hot
                        } else {
                            p.faint
                        },
                    )
                    .flex_shrink_0()
                    .whitespace_nowrap(),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranks(values: &[(&str, u32)]) -> HashMap<String, u32> {
        values
            .iter()
            .map(|(id, rank)| ((*id).into(), *rank))
            .collect()
    }
    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).into()).collect()
    }
    fn dependencies(values: &[(&str, &str)]) -> HashMap<String, String> {
        values
            .iter()
            .map(|(id, parent)| ((*id).into(), (*parent).into()))
            .collect()
    }
    fn projection(
        values: &[(&str, u32)],
        authored: &[&str],
        requires: &[(&str, &str)],
    ) -> RankProgression {
        RankProgression::new(
            (None, None, None),
            ranks(values),
            &strings(authored),
            &dependencies(requires),
        )
    }

    #[::core::prelude::v1::test]
    fn prerequisite_repair_keeps_authored_branch_order_and_rank_blocks() {
        let mut view = projection(
            &[("c", 1), ("a", 2), ("b", 2)],
            &["c", "a", "b"],
            &[("b", "a"), ("c", "b")],
        );
        assert_eq!(view.order, strings(&["a", "b", "c"]));
        view.set_step(3);
        assert_eq!(view.visible(), &ranks(&[("a", 2), ("b", 1)]));
        assert_eq!(view.marker(), Some("b"));
        let independent = projection(&[("a", 2), ("b", 1)], &["b", "a"], &[]);
        assert_eq!(independent.order, strings(&["b", "a"]));
    }

    #[::core::prelude::v1::test]
    fn imported_missing_prerequisites_cycles_and_unknown_ids_keep_all_points() {
        let mut view = projection(
            &[("x", 2), ("b", 1), ("a", 1), ("unknown", 2)],
            &["a", "x", "b"],
            &[("x", "missing"), ("a", "b"), ("b", "a")],
        );
        assert_eq!(view.order, strings(&["x", "unknown", "a", "b"]));
        view.set_step(5);
        assert_eq!(
            view.visible(),
            &ranks(&[("x", 2), ("unknown", 2), ("a", 1)])
        );
        assert_eq!(view.marker(), Some("a"));
        view.finish();
        assert_eq!(view.visible(), &view.source);
    }

    #[::core::prelude::v1::test]
    fn preview_bounds_and_first_rank_action_restore_without_document_mutation() {
        let source = ranks(&[("a", 2), ("b", 1)]);
        let mut view = projection(&[("a", 2), ("b", 1)], &["a", "b"], &[]);
        assert!(view.set_step(0));
        assert_eq!(view.current(), 1);
        assert_eq!(view.visible(), &ranks(&[("a", 1)]));
        assert!(view.finish()); // Rank handlers consume this action instead of calling Session::edit.
        assert!(!view.finish());
        assert_eq!(view.visible(), &source);
        assert_eq!(view.source, source);
        assert_eq!(view.marker(), None);
        assert!(!view.set_step(usize::MAX));
        assert!(!projection(&[("a", 1)], &["a"], &[]).set_step(0));
    }

    #[::core::prelude::v1::test]
    fn switching_document_or_changing_ranks_invalidates_preview_but_other_edits_do_not() {
        let mut draft = Draft {
            build_id: Some("build-a".into()),
            profile_id: Some("profile-a".into()),
            snapshot: BuildSnapshot {
                skill_ranks: ranks(&[("charged_bolts", 3)]),
                class_id: Some("stormweaver".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut view = RankProgression::for_draft(&draft);
        view.set_step(1);
        draft.snapshot.level += 1;
        assert!(view.matches(&draft));
        draft.profile_id = Some("profile-b".into());
        assert!(!view.matches(&draft));
        let reset = RankProgression::for_draft(&draft);
        assert!(!reset.is_preview());
        draft.snapshot.skill_ranks.insert("charged_bolts".into(), 2);
        assert!(!reset.matches(&draft));
    }

    #[::core::prelude::v1::test]
    fn subtree_scrubbing_preserves_the_session_and_limits_the_projection_to_its_skill() {
        use hsplanner_build::session::WorkspaceState;
        let skill = data::get_skills_by_class("stormweaver")
            .iter()
            .find(|skill| skill.id == "charged_bolts")
            .unwrap();
        let node = skill
            .subskills
            .iter()
            .flatten()
            .find(|node| node.position_index > 0 && node.max_rank >= 2)
            .unwrap();
        let key = format!("{}:{}", skill.id, node.id);
        let snapshot = BuildSnapshot {
            class_id: Some("stormweaver".into()),
            level: 100,
            subskill_ranks: HashMap::from([(key.clone(), 2), ("other:node".into(), 3)]),
            ..Default::default()
        };
        let mut session = Session::new(WorkspaceState {
            draft: Draft {
                snapshot,
                ..Default::default()
            },
            ..Default::default()
        });
        let before = serde_json::to_value(session.draft()).unwrap();
        let mut view = RankProgression::for_subtree(session.draft(), skill);
        assert_eq!(view.total(), 2);
        view.set_step(1);
        assert_eq!(view.visible(), &HashMap::from([(key.clone(), 1)]));
        assert_eq!(view.marker(), Some(key.as_str()));
        view.finish();
        assert_eq!(serde_json::to_value(session.draft()).unwrap(), before);
        assert_eq!(session.revision(), 0);
        assert!(!session.has_undo());

        session.edit(|draft| draft.snapshot.set_subskill_rank(&skill.id, &node.id, 0));
        assert!(!session.snapshot().subskill_ranks.contains_key(&key));
        assert_eq!(session.snapshot().subskill_ranks["other:node"], 3);
        assert!(!view.matches(session.draft()));
        session.undo();
        assert_eq!(serde_json::to_value(session.draft()).unwrap(), before);
    }
}
