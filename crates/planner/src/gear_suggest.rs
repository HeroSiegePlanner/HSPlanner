//! Suggest upgrades: a background search over other items and item modifications, applied step by step.
use super::*;
use crate::item_tooltip::{describe_affix_value, equipped_ids, with_item_tooltip};
use hsplanner_build::gear_suggest::{
    self, Choice, GearChange, GearStep, GearSuggestion, RankBy, SuggestOptions,
};
use hsplanner_engine::calc::affix::rolled_affix_value;
use hsplanner_engine::calc::skills::Ranged;
use hsplanner_engine::task_control::Cancellation;
use hsplanner_ui::components::{
    ModalSize, PlannerDialogExt, modal_body, modal_eyebrow, modal_footer, modal_header,
    modal_label, modal_layout, modal_status,
};
use hsplanner_ui::controls::{ButtonSize, ButtonTone, command_button, modal_button, segment};
use hsplanner_ui::numbers::{compact, compact_range};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

const PROGRESS_TICK: Duration = Duration::from_millis(120);
const OPTIONS: [&str; 6] = [
    "gear.suggest.other_items",
    "gear.suggest.sockets",
    "gear.suggest.crystals",
    "gear.suggest.augments",
    "gear.suggest.affixes",
    "gear.suggest.stars",
];
const RANKINGS: [(RankBy, &str); 2] = [
    (RankBy::CombinedDps, "planner.sidebar.combined_dps"),
    (RankBy::HitDps, "planner.sidebar.hit_dps"),
];

fn flag(options: &mut SuggestOptions, index: usize) -> &mut bool {
    match index {
        0 => &mut options.other_items,
        1 => &mut options.sockets,
        2 => &mut options.crystals,
        3 => &mut options.augments,
        4 => &mut options.affixes,
        _ => &mut options.stars,
    }
}

impl GearView {
    pub(super) fn open_suggest(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.new(|_| SuggestView::new(self.session.clone()));
        window.open_planner_modal(cx, ModalSize::Large, move |dialog, _, _| {
            dialog
                .overlay_closable(false)
                .on_ok(|_, _, _| false)
                .child(view.clone())
        });
    }
}

enum Phase {
    Idle,
    Running,
    Done(GearSuggestion),
    Failed(String),
}

struct SuggestView {
    session: Entity<Session>,
    options: SuggestOptions,
    phase: Phase,
    /// Steps hold only in order, so what is applied is always a prefix.
    applied: usize,
    error: Option<String>,
    run_id: u64,
    progress: Arc<(AtomicU32, AtomicU32)>,
    cancellation: Option<Cancellation>,
}

impl Drop for SuggestView {
    fn drop(&mut self) {
        self.stop();
    }
}

impl SuggestView {
    fn new(session: Entity<Session>) -> Self {
        Self {
            session,
            options: SuggestOptions::default(),
            phase: Phase::Idle,
            applied: 0,
            error: None,
            run_id: 0,
            progress: Arc::default(),
            cancellation: None,
        }
    }

    fn stop(&mut self) {
        if let Some(cancellation) = self.cancellation.take() {
            cancellation.cancel();
        }
        self.run_id += 1;
    }

    fn run(&mut self, cx: &mut Context<Self>) {
        self.stop();
        let run_id = self.run_id;
        let cancellation = Cancellation::default();
        let progress = Arc::new((AtomicU32::new(0), AtomicU32::new(1)));
        self.cancellation = Some(cancellation.clone());
        self.progress = progress.clone();
        self.phase = Phase::Running;
        self.applied = 0;
        self.error = None;
        let snapshot = self.session.read(cx).snapshot().clone();
        let options = self.options;
        let task = cx.background_spawn(async move {
            gear_suggest::suggest(&snapshot, options, &cancellation, |done, total| {
                progress.0.store(done, Ordering::Relaxed);
                progress.1.store(total, Ordering::Relaxed);
            })
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.run_id != run_id {
                    return;
                }
                this.cancellation = None;
                this.phase = match result {
                    Ok(result) => Phase::Done(result),
                    Err(error) => Phase::Failed(trf("gear.suggest.failed", &[("detail", error)])),
                };
                cx.notify();
            });
        })
        .detach();
        // ponytail: progress lives in atomics; a short timer repaints while the search runs.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(PROGRESS_TICK).await;
                let running = this
                    .update(cx, |this, cx| {
                        let running = this.run_id == run_id && matches!(this.phase, Phase::Running);
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

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.stop();
        self.phase = Phase::Failed(tr("gear.suggest.cancelled").into());
        cx.notify();
    }

    /// Applies the pending steps up to `end` as one undoable edit; nothing changes when one
    /// of them no longer fits.
    fn apply(&mut self, end: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Done(result) = &self.phase else {
            return;
        };
        let total = result.steps.len();
        let end = end.min(total);
        let steps: Vec<GearStep> = result.steps[self.applied.min(end)..end].to_vec();
        let outcome = self.session.update(cx, |session, cx| {
            let outcome = gear_suggest::apply_steps(session, &steps);
            cx.notify();
            outcome
        });
        match outcome {
            Ok(()) => {
                self.applied = self.applied.max(end);
                self.error = None;
                if self.applied == total {
                    window.close_dialog(cx);
                }
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }

    fn options_section(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let running = matches!(self.phase, Phase::Running);
        let mut chips = div().flex().flex_wrap().gap_2();
        for (index, key) in OPTIONS.into_iter().enumerate() {
            let mut options = self.options;
            chips = chips.child(
                segment(
                    SharedString::from(format!("gear-suggest-option-{index}")),
                    tr(key),
                    *flag(&mut options, index),
                    cx,
                )
                .disabled(running)
                .on_click(cx.listener(move |this, _, _, cx| {
                    let value = flag(&mut this.options, index);
                    *value = !*value;
                    cx.notify();
                })),
            );
        }
        let rankings = RANKINGS
            .into_iter()
            .enumerate()
            .map(|(index, (rank_by, key))| {
                segment(
                    SharedString::from(format!("gear-suggest-rank-{index}")),
                    tr(key),
                    self.options.rank_by == rank_by,
                    cx,
                )
                .disabled(running)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.options.rank_by = rank_by;
                    cx.notify();
                }))
            });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(modal_label(
                "gear-suggest-consider",
                tr("gear.suggest.consider"),
                cx,
            ))
            .child(chips)
            .child(div().mt_1().child(modal_label(
                "gear-suggest-rank-by",
                tr("gear.suggest.rank_by"),
                cx,
            )))
            .child(div().flex().gap_2().children(rankings))
            .child(
                div()
                    .text_size(rems(11. / 13.))
                    .line_height(relative(1.45))
                    .text_color(p.muted)
                    .child(tr("gear.suggest.assumptions")),
            )
    }

    fn progress_section(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let (done, total) = (
            self.progress.0.load(Ordering::Relaxed),
            self.progress.1.load(Ordering::Relaxed),
        );
        let fraction = if total > 0 {
            (done as f32 / total as f32).min(1.)
        } else {
            0.
        };
        let slots = gear_suggest::SLOTS;
        let last = gear_suggest::PASSES * slots.len() - 1;
        let label = match done.checked_sub(1).map(|index| (index as usize).min(last)) {
            Some(index) => trf(
                if index < slots.len() {
                    "gear.suggest.checking"
                } else {
                    "gear.suggest.rechecking"
                },
                &[(
                    "slot",
                    crate::gear_stash::slot_label(slots[index % slots.len()]),
                )],
            ),
            None => tr("gear.suggest.measuring").into(),
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_sm().text_color(p.muted).child(label))
            .child(
                div()
                    .h(rems(6. / 13.))
                    .w_full()
                    .rounded_sm()
                    .border_1()
                    .border_color(p.border)
                    .bg(p.panel_secondary)
                    .child(
                        div()
                            .h_full()
                            .w(relative(fraction))
                            .rounded_sm()
                            .bg(p.accent_hot),
                    ),
            )
    }

    /// The stat panel's three numbers now and with every change, as the panel shows them.
    fn summary(&self, result: &GearSuggestion, scale: &str, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let now = self
            .applied
            .checked_sub(1)
            .and_then(|index| result.steps.get(index))
            .map_or(result.before, |step| step.after);
        let end = result.steps.last().map_or(result.before, |step| step.after);
        let range =
            |value: Option<Ranged>| value.map_or_else(|| "—".into(), |v| compact_range(v, scale));
        let mid = |value: Option<Ranged>| value.map(|(low, high)| (low + high) / 2.);
        let label_width = rems(7.);
        let cell = |content: String, color: Hsla| {
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_color(color)
                .child(content)
        };
        let header = div()
            .flex()
            .gap_3()
            .child(div().w(label_width).flex_none())
            .children(
                [
                    ("gear-suggest-now", tr("gear.suggest.base_dps")),
                    ("gear-suggest-final", tr("gear.suggest.final_dps")),
                    ("gear-suggest-gain", tr("gear.suggest.gain")),
                ]
                .map(|(id, label)| div().flex_1().min_w_0().child(modal_label(id, label, cx))),
            );
        let rows = [
            (
                tr("planner.common.hit_damage"),
                now.hit_damage,
                end.hit_damage,
                false,
            ),
            (
                tr("planner.sidebar.hit_dps"),
                now.hit_dps,
                end.hit_dps,
                result.rank_by == RankBy::HitDps,
            ),
            (
                tr("planner.sidebar.combined_dps"),
                now.combined_dps,
                end.combined_dps,
                result.rank_by == RankBy::CombinedDps,
            ),
        ]
        .map(|(label, before, after, is_ranked)| {
            let (gain, color) = match mid(before).zip(mid(after)) {
                Some((from, to)) => (
                    crate::tree_suggest::gain_percent(from, to),
                    if to > from { p.positive } else { p.muted },
                ),
                None => ("—".into(), p.muted),
            };
            div()
                .flex()
                .items_baseline()
                .gap_3()
                .when(is_ranked, |row| row.font_weight(FontWeight::SEMIBOLD))
                .child(
                    div()
                        .w(label_width)
                        .flex_none()
                        .text_color(p.muted)
                        .child(label),
                )
                .child(cell(range(before), p.text))
                .child(cell(range(after), p.accent_hot))
                .child(cell(gain, color))
        });
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .px_4()
            .py_3()
            .rounded_sm()
            .border_1()
            .border_color(p.border)
            .bg(p.panel_secondary)
            .text_size(rems(12. / 13.))
            .child(header)
            .children(rows)
    }

    fn step_row(
        &self,
        index: usize,
        step: &GearStep,
        rank_by: RankBy,
        scale: &str,
        cx: &Context<Self>,
    ) -> Div {
        let p = cx.global::<TooltipTheme>();
        let base = data::get_item(&step.item.base_id);
        let name = base.map_or_else(|| step.item.base_id.clone(), |base| base.name.clone());
        let color = base.map_or(p.text, |base| theme::rarity_color(&base.rarity, cx));
        let context = match &step.before {
            None => Some(tr("gear.suggest.empty_slot").to_owned()),
            Some(before) if before.base_id != step.item.base_id => Some(trf(
                "gear.suggest.replaces",
                &[("name", item_name(&before.base_id))],
            )),
            Some(_) => None,
        };
        let equipped = equipped_ids(&self.session.read(cx).snapshot().inventory);
        let rows = change_rows(step).into_iter().map(|(kind, text)| {
            div()
                .flex()
                .gap_3()
                .text_size(rems(11. / 13.))
                .child(
                    div()
                        .w(rems(7.))
                        .flex_none()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(rems(10. / 13.))
                        .text_color(p.faint)
                        .child(tr(kind).to_uppercase()),
                )
                .child(div().flex_1().min_w_0().text_color(p.text).child(text))
        });
        let cleared = step.cleared.iter().map(|slot| {
            div()
                .text_size(rems(11. / 13.))
                .text_color(p.negative)
                .child(trf(
                    "gear.suggest.unequips",
                    &[("slot", crate::gear_stash::slot_label(slot))],
                ))
        });
        let action = if index < self.applied {
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(10. / 13.))
                .text_color(p.positive)
                .child(tr("gear.suggest.applied").to_uppercase())
                .into_any_element()
        } else {
            command_button(
                SharedString::from(format!("gear-suggest-apply-{index}")),
                if index == self.applied {
                    tr("gear.suggest.apply")
                } else {
                    tr("gear.suggest.apply_through")
                },
                ButtonTone::Neutral,
                ButtonSize::Small,
                cx,
            )
            .on_click(cx.listener(move |this, _, window, cx| this.apply(index + 1, window, cx)))
            .into_any_element()
        };
        div()
            .flex()
            .items_start()
            .gap_4()
            .py_3()
            .border_b_1()
            .border_color(p.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(modal_label(
                        SharedString::from(format!("gear-suggest-slot-{index}")),
                        crate::gear_stash::slot_label(&step.slot),
                        cx,
                    ))
                    .child(with_item_tooltip(
                        SharedString::from(format!("gear-suggest-item-{index}")),
                        &step.item,
                        equipped,
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color)
                            .child(name),
                    ))
                    .children(context.map(|text| div().text_xs().text_color(p.muted).child(text)))
                    .child(div().mt_1().flex().flex_col().gap_0p5().children(rows))
                    .children(cleared),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap_1()
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.positive)
                            .child(crate::tree_suggest::gain_percent(
                                step.dps_before,
                                step.dps_after,
                            )),
                    )
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(10. / 13.))
                            .text_color(p.faint)
                            .child(trf(
                                match rank_by {
                                    RankBy::CombinedDps => "gear.suggest.dps",
                                    RankBy::HitDps => "gear.suggest.hit_dps",
                                },
                                &[(
                                    "dps",
                                    rank_by.of(&step.after).map_or_else(
                                        || compact(step.dps_after, scale),
                                        |range| compact_range(range, scale),
                                    ),
                                )],
                            )),
                    )
                    .child(action),
            )
    }
}

fn item_name(id: &str) -> String {
    data::get_item(id).map_or_else(|| id.to_owned(), |base| base.name.clone())
}

fn socketable(id: Option<&str>) -> String {
    id.map_or_else(
        || tr("gear.suggest.empty").to_owned(),
        editor::socketable_name,
    )
}

fn affix_text(id: &str) -> String {
    let Some(affix) = data::get_affix(id).or_else(|| data::get_crystal_mod(id)) else {
        return id.to_owned();
    };
    describe_affix_value(affix, rolled_affix_value(affix, 1.).abs())
        .or_else(|| (!affix.description.is_empty()).then(|| affix.description.clone()))
        .unwrap_or_else(|| affix.name.clone())
}

fn replacement(from: Option<String>, to: Option<String>) -> String {
    match (from, to) {
        (Some(from), Some(to)) => trf("gear.suggest.replacing", &[("to", to), ("from", from)]),
        (None, Some(to)) => to,
        (Some(from), None) => trf("gear.suggest.removes", &[("name", from)]),
        (None, None) => String::new(),
    }
}

fn augment_text(from: Option<(String, u32)>, to: Option<(String, u32)>) -> String {
    let name = |id: &str| data::get_augment(id).map_or_else(|| id.to_owned(), |a| a.name.clone());
    let level = |level: u32| trf("gear.suggest.level", &[("level", level.to_string())]);
    match (from, to) {
        (Some((old, before)), Some((id, after))) if old == id => {
            format!("{} · {} → {after}", name(&id), level(before))
        }
        (from, to) => replacement(
            from.map(|(id, lv)| format!("{} · {}", name(&id), level(lv))),
            to.map(|(id, lv)| format!("{} · {}", name(&id), level(lv))),
        ),
    }
}

fn choice_row(choice: Choice, value: &str) -> (&'static str, String) {
    let skill = || data::skill_name_by_id(value).unwrap_or(value).to_owned();
    match choice {
        Choice::Element => ("gear.suggest.kind_element", element_label(value)),
        Choice::RandomSkill => ("gear.suggest.kind_random_skill", skill()),
        Choice::Subskill => ("gear.suggest.kind_subskill", skill()),
        Choice::Class => (
            "gear.suggest.kind_class",
            data::get_class(value).map_or_else(|| value.to_owned(), |class| class.name.clone()),
        ),
    }
}

/// Readable rows for a step's changes; identical socket edits fold into one counted row.
fn change_rows(step: &GearStep) -> Vec<(&'static str, String)> {
    fn flush(
        rows: &mut Vec<(&'static str, String)>,
        sockets: &mut Vec<(Option<String>, Option<String>, usize)>,
    ) {
        for (from, to, count) in sockets.drain(..) {
            let text = replacement(
                from.as_deref().map(|id| socketable(Some(id))),
                Some(socketable(to.as_deref())),
            );
            let text = if count > 1 {
                format!("{text} ×{count}")
            } else {
                text
            };
            rows.push(("gear.suggest.kind_sockets", text));
        }
    }
    let mut rows = Vec::new();
    let mut sockets: Vec<(Option<String>, Option<String>, usize)> = Vec::new();
    for change in step.changes() {
        if let GearChange::Socket { from, to, .. } = change {
            match sockets.iter_mut().find(|(f, t, _)| *f == from && *t == to) {
                Some(entry) => entry.2 += 1,
                None => sockets.push((from, to, 1)),
            }
            continue;
        }
        flush(&mut rows, &mut sockets);
        let row = match change {
            GearChange::Item { .. } | GearChange::Socket { .. } => continue,
            GearChange::Stars { from, to } => {
                ("gear.suggest.kind_stars", format!("{from} → {to} ★"))
            }
            GearChange::Choice(choice, value) => choice_row(choice, &value),
            GearChange::Affix { from, to } => (
                "gear.suggest.kind_affix",
                replacement(
                    from.as_deref().map(affix_text),
                    to.as_deref().map(affix_text),
                ),
            ),
            GearChange::Crystal { from, to } => (
                "gear.suggest.kind_crystal",
                replacement(
                    from.as_deref().map(affix_text),
                    to.as_deref().map(affix_text),
                ),
            ),
            GearChange::Runeword(id) => {
                let word = data::data().runewords.iter().find(|rw| rw.id == id);
                let text = word.map_or(id, |rw| {
                    let runes: Vec<String> = rw
                        .runes
                        .iter()
                        .map(|r| editor::socketable_name(r))
                        .collect();
                    format!("{} · {}", rw.name, runes.join(" "))
                });
                ("gear.suggest.kind_runeword", text)
            }
            GearChange::Augment { from, to } => {
                ("gear.suggest.kind_augment", augment_text(from, to))
            }
        };
        rows.push(row);
    }
    flush(&mut rows, &mut sockets);
    rows
}

impl Render for SuggestView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let scale = self.session.read(cx).state().settings.number_scale.clone();
        let mut body = modal_body("gear-suggest-body")
            .gap_4()
            .child(self.options_section(cx));
        let mut status = (String::new(), p.faint);
        match &self.phase {
            Phase::Idle => {}
            Phase::Running => body = body.child(self.progress_section(cx)),
            Phase::Failed(message) => {
                body = body.child(
                    div()
                        .text_sm()
                        .text_color(p.negative)
                        .child(message.clone()),
                )
            }
            Phase::Done(result) => {
                status.0 = trf(
                    "gear.suggest.calculations",
                    &[("count", result.evaluations.to_string())],
                );
                body = body.child(self.summary(result, &scale, cx));
                body = if result.steps.is_empty() {
                    body.child(
                        div()
                            .py_4()
                            .text_center()
                            .text_sm()
                            .text_color(p.muted)
                            .child(tr("gear.suggest.none")),
                    )
                } else {
                    let second = result.steps.iter().position(|step| step.pass > 0);
                    body.child(div().flex().flex_col().children(
                        result.steps.iter().enumerate().map(|(index, step)| {
                            div()
                                .flex()
                                .flex_col()
                                .when(second == Some(index), |rows| {
                                    rows.pt_3().child(modal_label(
                                        "gear-suggest-second-pass",
                                        tr("gear.suggest.second_pass"),
                                        cx,
                                    ))
                                })
                                .child(self.step_row(index, step, result.rank_by, &scale, cx))
                        }),
                    ))
                };
            }
        }
        if let Some(error) = &self.error {
            status = (error.clone(), p.negative);
        }
        let running = matches!(self.phase, Phase::Running);
        let total = match &self.phase {
            Phase::Done(result) => result.steps.len(),
            _ => 0,
        };
        let done = matches!(self.phase, Phase::Done(_));
        let mut options = self.options;
        let any = (0..OPTIONS.len()).any(|index| *flag(&mut options, index));
        let mut footer = modal_footer(cx).child(modal_status(status.0, status.1));
        footer = if running {
            footer.child(
                modal_button(
                    "gear-suggest-cancel",
                    tr("gear.suggest.cancel"),
                    ButtonTone::Neutral,
                    cx,
                )
                .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
            )
        } else {
            footer.child(
                modal_button(
                    "gear-suggest-run",
                    if done {
                        tr("gear.suggest.again")
                    } else {
                        tr("gear.suggest.find")
                    },
                    if done {
                        ButtonTone::Neutral
                    } else {
                        ButtonTone::Primary
                    },
                    cx,
                )
                .disabled(!any)
                .on_click(cx.listener(|this, _, _, cx| this.run(cx))),
            )
        };
        if done {
            footer = footer.child(
                modal_button(
                    "gear-suggest-apply-all",
                    tr("gear.suggest.apply_all"),
                    ButtonTone::Primary,
                    cx,
                )
                .disabled(self.applied >= total)
                .on_click(cx.listener(move |this, _, window, cx| this.apply(total, window, cx))),
            );
        }
        modal_layout(window)
            .child(modal_header(
                modal_eyebrow("gear-suggest-eyebrow", tr("gear.suggest.eyebrow")),
                tr("gear.suggest.title"),
                Some(tr("gear.suggest.subtitle").into()),
                cx,
            ))
            .child(body)
            .child(footer)
    }
}
