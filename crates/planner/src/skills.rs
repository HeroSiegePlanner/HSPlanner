//! Class skill trees and their inspector. The document owns ranks; this view owns selection.
use hsplanner_ui::tooltip::CursorTooltipExt;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use crate::{
    TreeView,
    build_panel::{format_range, preview_changes},
    build_session::PreviewResult,
    skill_details,
};
use gpui_kit::base::{Disableable, Selectable};
use gpui_kit::component::{
    Icon, IconName, Sizable, WindowExt,
    button::{Button, ButtonVariants},
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::{BuildSnapshot, session::Session};
use hsplanner_engine::calc::{
    data,
    performance_diff::compare_planner,
    planner::{self, PlannerPerformance},
    types::{AppliedStateValue, SkillKind, SkillSpec, SubskillEffectSpec, SubskillNodeSpec},
};
use hsplanner_ui::{
    controls::PlannerControl,
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};

mod preview;
mod progression;
use gpui_kit::component::slider::SliderState;
use preview::PreviewRequest;
use progression::RankProgression;

mod visuals {
    include!(concat!(env!("OUT_DIR"), "/skill-visuals.rs"));
}

static POSITIONS: LazyLock<HashMap<&'static str, (u32, u32)>> = LazyLock::new(|| {
    visuals::SKILL_POSITIONS
        .iter()
        .map(|&(id, row, col)| (id, (row, col)))
        .collect()
});
static ICONS: LazyLock<HashMap<&'static str, Arc<Image>>> = LazyLock::new(|| {
    visuals::SKILL_ICONS
        .iter()
        .chain(visuals::SUBSKILL_ICONS)
        .map(|&(id, bytes)| {
            (
                id,
                Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec())),
            )
        })
        .collect()
});
const CELL: f32 = 84.;
const STEP: f32 = 102.;

fn units(value: f32) -> Rems {
    rems(value / 13.)
}
fn caption(id: impl Into<ElementId>, text: impl Into<SharedString>, color: Hsla) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(10.))
        .text_color(color)
        .child(TooltipText::new(id, text, 0.14))
}
fn position(skill: &SkillSpec) -> Option<(u32, u32)> {
    POSITIONS
        .get(format!("{}/{}", skill.class_id, skill.id).as_str())
        .copied()
}
fn tree_columns(skills: &[&SkillSpec]) -> u32 {
    skills
        .iter()
        .filter_map(|skill| position(skill).map(|(_, col)| col + 1))
        .max()
        .unwrap_or(3)
        .max(3)
}
pub(crate) fn skill_icon(class: &str, id: &str) -> Option<Arc<Image>> {
    ICONS.get(format!("{class}/{id}").as_str()).cloned()
}
fn skill_image(skill: &SkillSpec, size: Rems) -> Div {
    let icon = ICONS.get(format!("{}/{}", skill.class_id, skill.id).as_str());
    div()
        .size(size)
        .flex()
        .items_center()
        .justify_center()
        .when_some(icon, |v, icon| {
            v.child(
                img(icon.clone())
                    .size(size * 0.9)
                    .object_fit(ObjectFit::Contain),
            )
        })
        .when(icon.is_none(), |v| v.child("✦"))
}

fn is_enabled(snapshot: &BuildSnapshot, skill: &SkillSpec) -> bool {
    match skill.kind {
        SkillKind::Active => snapshot.active_skill_ids.contains(&skill.id),
        SkillKind::Aura => snapshot.active_aura_id.as_ref() == Some(&skill.id),
        SkillKind::Buff => snapshot
            .active_buffs
            .get(&skill.id)
            .copied()
            .unwrap_or(false),
        SkillKind::Passive => false,
    }
}
fn allocation_amount(modifiers: Modifiers, available: u32) -> u32 {
    if modifiers.shift && (modifiers.control || modifiers.platform) {
        available
    } else if modifiers.shift {
        5.min(available)
    } else {
        1.min(available)
    }
}

fn adjust_skill_rank(snapshot: &mut BuildSnapshot, id: &str, increase: bool, modifiers: Modifiers) {
    let rank = snapshot.skill_ranks.get(id).copied().unwrap_or(0);
    let points = snapshot
        .level
        .saturating_mul(data::game_config().skill_points_per_level);
    let spent = snapshot.skill_ranks.values().copied().sum();
    let amount = allocation_amount(
        modifiers,
        if increase {
            points.saturating_sub(spent)
        } else {
            rank
        },
    );
    // The document operation owns prerequisites, caps and recursive dependent removal.
    snapshot.set_skill_rank(
        id,
        if increase {
            rank.saturating_add(amount)
        } else {
            rank.saturating_sub(amount)
        },
    );
}

pub struct SkillsView {
    session: Entity<Session>,
    tree: Entity<TreeView>,
    observed_performance: Option<Arc<PlannerPerformance>>,
    selected: Option<String>,
    class_id: Option<String>,
    active: bool,
    calculation_revision: u64,
    detail: Option<Arc<PlannerPerformance>>,
    detail_task: Option<Task<()>>,
    detail_revision: u64,
    tree_scroll: ScrollHandle,
    tree_has_vertical_scroll: bool,
    details_scroll: ScrollHandle,
    details_has_vertical_scroll: bool,
    progression: RankProgression,
    progression_slider: Entity<SliderState>,
    progression_focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}
impl SkillsView {
    pub fn new(
        session: Entity<Session>,
        tree: Entity<TreeView>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let class_id = session.read(cx).snapshot().class_id.clone();
        let calculation_revision = session.read(cx).calculation_revision();
        let observed_performance = tree.read(cx).performance();
        let progression = RankProgression::for_draft(session.read(cx).draft());
        let progression_slider = cx.new(|_| progression::slider_state(progression.total()));
        let subscriptions = vec![
            cx.observe(&session, |this, _, cx| {
                this.sync_progression(cx);
                let class = this.session.read(cx).snapshot().class_id.clone();
                if class != this.class_id {
                    this.selected = None;
                    this.class_id = class;
                }
                let revision = this.session.read(cx).calculation_revision();
                if revision != this.calculation_revision {
                    this.calculation_revision = revision;
                    this.refresh_detail(cx);
                }
                cx.notify();
            }),
            cx.observe(&tree, |this, tree, cx| {
                let next = tree.read(cx).performance();
                if this.observed_performance.as_ref().map(Arc::as_ptr)
                    == next.as_ref().map(Arc::as_ptr)
                {
                    return;
                }
                this.observed_performance = next;
                cx.notify();
            }),
            cx.observe(&progression_slider, |this, slider, cx| {
                let step = slider.read(cx).value().end().round() as usize;
                if this.progression.set_step(step) {
                    this.refresh_detail(cx);
                    cx.notify();
                }
            }),
        ];
        Self {
            session,
            tree,
            observed_performance,
            selected: None,
            class_id,
            active: false,
            calculation_revision,
            detail: None,
            detail_task: None,
            detail_revision: 0,
            tree_scroll: ScrollHandle::new(),
            tree_has_vertical_scroll: false,
            details_scroll: ScrollHandle::new(),
            details_has_vertical_scroll: false,
            progression,
            progression_slider,
            progression_focus: cx.focus_handle().tab_stop(true),
            _subscriptions: subscriptions,
        }
    }
    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.active == active {
            return;
        }
        self.active = active;
        self.refresh_detail(cx);
        cx.notify();
    }
    fn edit(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut BuildSnapshot)) {
        self.session.update(cx, |session, cx| {
            session.edit(|draft| edit(&mut draft.snapshot));
            cx.notify();
        });
    }
    fn select(&mut self, id: &str, cx: &mut Context<Self>) {
        self.selected = Some(id.into());
        self.refresh_detail(cx);
        cx.notify();
    }
    fn refresh_detail(&mut self, cx: &mut Context<Self>) {
        self.detail_revision = self.detail_revision.wrapping_add(1);
        self.detail_task = None;
        self.detail = None;
        if !self.active || self.progression.is_preview() {
            return;
        }
        let Some(id) = self.selected.clone() else {
            return;
        };
        let snapshot = self.session.read(cx).snapshot();
        let Some(skill) = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""))
            .iter()
            .find(|skill| skill.id == id)
        else {
            return;
        };
        if skill.kind != SkillKind::Active {
            return;
        }
        let mut input = snapshot.planner_input();
        input.active_skill_ids = vec![id];
        let revision = self.detail_revision;
        self.detail_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { Arc::new(planner::evaluate(&input)) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if revision == this.detail_revision {
                    this.detail = Some(result);
                    this.detail_task = None;
                    cx.notify();
                }
            });
        }));
    }
    fn change_rank(
        &mut self,
        id: &str,
        increase: bool,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) {
        // Like the reference, the first rank action exits a display-only preview.
        if self.finish_progression(cx) {
            return;
        }
        self.edit(cx, |snapshot| {
            adjust_skill_rank(snapshot, id, increase, modifiers)
        });
    }
    fn toggle(&mut self, id: &str, kind: SkillKind, cx: &mut Context<Self>) {
        self.edit(cx, |s| match kind {
            SkillKind::Active => {
                if s.active_skill_ids.iter().any(|key| key == id) {
                    s.active_skill_ids.retain(|key| key != id);
                } else {
                    s.active_skill_ids.push(id.into());
                }
            }
            SkillKind::Aura => {
                s.active_aura_id = if s.active_aura_id.as_deref() == Some(id) {
                    None
                } else {
                    Some(id.into())
                };
            }
            SkillKind::Buff => {
                let current = s.active_buffs.get(id).copied().unwrap_or(false);
                s.active_buffs.insert(id.into(), !current);
            }
            SkillKind::Passive => {}
        });
    }
    fn subtree(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(skill) = data::get_skills_by_class(self.class_id.as_deref().unwrap_or(""))
            .iter()
            .find(|s| s.id == id)
            .cloned()
        else {
            return;
        };
        let session = self.session.clone();
        let view = cx.new(|cx| SubtreeView::new(session, skill, cx));
        window.open_dialog(cx, move |dialog, window, _| {
            let rem = window.rem_size();
            let viewport = window.viewport_size();
            let width = (viewport.width - rem * 3.).min(rem * (680. / 13.));
            // Modal.tsx header, body padding and footer surround the capped outer board.
            let height = rem * ((subtree_board_size(window) + 160.375) / 13.);
            let margin_top = ((viewport.height - height) / 2.).max(rem * 1.5);
            dialog
                .close_button(false)
                .p_0()
                .gap_0()
                .rounded_xl()
                .w(width)
                .margin_top(margin_top)
                .child(view.clone())
        });
    }
    fn tree_panel(
        &self,
        name: &str,
        skills: &[&SkillSpec],
        window: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let palette = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let visible_ranks = self.progression.visible();
        let performance = self.tree.read(cx).performance();
        let spent: u32 = skills
            .iter()
            .map(|skill| visible_ranks.get(&skill.id).copied().unwrap_or(0))
            .sum();
        let mut types = HashMap::<&str, u32>::new();
        for skill in skills {
            if let Some(kind) = skill.damage_type.as_deref() {
                *types.entry(kind).or_default() += 1;
            }
        }
        let color = skills
            .iter()
            .filter_map(|s| s.damage_type.as_deref())
            .max_by_key(|kind| types[kind])
            .map(theme::damage_color)
            .unwrap_or(palette.accent_hot);
        let cols = tree_columns(skills);
        let rows = skills
            .iter()
            .filter_map(|s| position(s).map(|(row, _)| row + 1))
            .max()
            .unwrap_or(5)
            .max(5);
        let width = cols as f32 * STEP - 18.;
        let height = rows as f32 * STEP - 18.;
        let edges = skills
            .iter()
            .filter_map(|skill| {
                let parent = skills
                    .iter()
                    .find(|s| Some(s.id.as_str()) == skill.requires_skill.as_deref())?;
                let (ar, ac) = position(parent)?;
                let (br, bc) = position(skill)?;
                Some((
                    [ac as f32 * STEP + CELL / 2., ar as f32 * STEP + CELL / 2.],
                    [bc as f32 * STEP + CELL / 2., br as f32 * STEP + CELL / 2.],
                    visible_ranks.get(&parent.id).copied().unwrap_or(0) > 0,
                ))
            })
            .collect::<Vec<_>>();
        let muted = palette.faint.opacity(0.35);
        let mut board = div().relative().w(units(width)).h(units(height)).child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let scale = f32::from(window.rem_size()) / 13.;
                    for (a, b, allocated) in &edges {
                        let mut path = PathBuilder::stroke(px(2. * scale));
                        // Canvas coordinates are resolved pixels; tiles use the same rem scale.
                        let dx = b[0] - a[0];
                        let dy = b[1] - a[1];
                        let distance = (dx * dx + dy * dy).sqrt();
                        let mut start = 0.;
                        while start < distance {
                            let end = (start + 4.).min(distance);
                            path.move_to(
                                bounds.origin
                                    + point(
                                        px((a[0] + dx * start / distance) * scale),
                                        px((a[1] + dy * start / distance) * scale),
                                    ),
                            );
                            path.line_to(
                                bounds.origin
                                    + point(
                                        px((a[0] + dx * end / distance) * scale),
                                        px((a[1] + dy * end / distance) * scale),
                                    ),
                            );
                            start += 9.;
                        }
                        if let Ok(path) = path.build() {
                            window.paint_path(
                                path,
                                if *allocated {
                                    color.opacity(0.55)
                                } else {
                                    muted
                                },
                            );
                        }
                    }
                },
            )
            .absolute()
            .size_full(),
        );
        for skill in skills {
            let Some((row, col)) = position(skill) else {
                continue;
            };
            let id = skill.id.clone();
            let select = id.clone();
            let add = id.clone();
            let remove = id.clone();
            let subtree = id.clone();
            let rank = visible_ranks.get(&id).copied().unwrap_or(0);
            let bonus = performance
                .as_ref()
                .filter(|_| rank > 0)
                .and_then(|result| {
                    result
                        .computed
                        .rank_bonuses
                        .get(&skill.name.trim().to_lowercase())
                })
                .copied()
                .unwrap_or((0., 0.));
            let rank_label = if bonus == (0., 0.) {
                rank.to_string()
            } else {
                let sign = if bonus.0 >= 0. { "+" } else { "" };
                if bonus.0 == bonus.1 {
                    format!("{rank}{sign}{}", format_range(bonus, false))
                } else {
                    format!("{rank}{sign}({})", format_range(bonus, false))
                }
            };
            let selected = self.selected.as_ref() == Some(&id);
            let marker = self.progression.marker() == Some(id.as_str());
            let locked = skill
                .requires_skill
                .as_ref()
                .is_some_and(|required| visible_ranks.get(required).copied().unwrap_or(0) == 0);
            let total = snapshot
                .level
                .saturating_mul(data::game_config().skill_points_per_level);
            let available = total.saturating_sub(snapshot.skill_ranks.values().sum());
            let mut tile = div()
                .id(SharedString::from(format!("skill-node-{id}")))
                .absolute()
                .left(units(col as f32 * STEP))
                .top(units(row as f32 * STEP))
                .size(units(CELL))
                .rounded_sm()
                .when(marker, |tile| {
                    tile.shadow(vec![
                        BoxShadow {
                            color: palette.accent_hot.opacity(0.9),
                            offset: point(px(0.), px(0.)),
                            blur_radius: px(0.),
                            spread_radius: units(2.5).to_pixels(window.rem_size()),
                            inset: false,
                        },
                        BoxShadow {
                            color: palette.accent_hot.opacity(0.6),
                            offset: point(px(0.), px(0.)),
                            blur_radius: units(16.).to_pixels(window.rem_size()),
                            spread_radius: px(0.),
                            inset: false,
                        },
                    ])
                })
                .child(
                    Button::new("select")
                        .ghost()
                        .size(units(CELL))
                        .p_0()
                        .rounded_sm()
                        .border_1()
                        .border_color(if selected {
                            palette.accent_hot
                        } else if rank > 0 {
                            color.opacity(0.65)
                        } else {
                            palette.border
                        })
                        .accessibility_label(format!("Inspect {}", skill.name))
                        .cursor_tooltip(skill.name.clone())
                        .child(skill_image(skill, units(CELL)).opacity(if locked {
                            0.3
                        } else if rank > 0 {
                            1.
                        } else {
                            0.6
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| this.select(&select, cx))),
                )
                .child(
                    div()
                        .absolute()
                        .bottom_0p5()
                        .left_0p5()
                        .min_w_5()
                        .h_5()
                        .flex()
                        .items_center()
                        .justify_center()
                        .px_1()
                        .border_1()
                        .rounded_sm()
                        .border_color(if rank > 0 {
                            palette.accent_deep
                        } else {
                            palette.border
                        })
                        .bg(palette.background)
                        .font_family(theme::MONO_FONT_FAMILY)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_size(units(11.))
                        .text_color(if rank > 0 {
                            palette.accent_hot
                        } else {
                            palette.faint
                        })
                        .child(rank_label),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        if rank > 0 {
                            this.change_rank(&remove, false, event.modifiers, cx);
                        }
                        cx.stop_propagation();
                    }),
                );
            if !locked && available > 0 && rank < skill.max_rank {
                tile = tile.child(
                    Button::new("add")
                        .planner_style(cx)
                        .absolute()
                        .right(rems(-0.375))
                        .top(rems(-0.375))
                        .size_5()
                        .p_0()
                        .bg(theme::chrome_gold_surface())
                        .text_color(palette.accent_hot)
                        .label("+")
                        .accessibility_label(format!("Add point to {}", skill.name))
                        .cursor_tooltip("Add a point · Shift ×5 · Ctrl/Cmd+Shift all")
                        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                            this.change_rank(&add, true, event.modifiers(), cx)
                        })),
                );
            }
            if skill
                .subskills
                .as_ref()
                .is_some_and(|nodes| !nodes.is_empty())
            {
                tile = tile.child(
                    Button::new("subtree")
                        .planner_style(cx)
                        .absolute()
                        .right(rems(-0.375))
                        .bottom(rems(-0.375))
                        .size_5()
                        .p_0()
                        .child(Icon::new(IconName::Settings).size_3())
                        .accessibility_label(format!("Open {} subtree", skill.name))
                        .cursor_tooltip("Open subtree…")
                        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.subtree(&subtree, window, cx)
                        })),
                );
            }
            board = board.child(tile);
        }
        div()
            .w(units(width + 32.))
            .flex_shrink_0()
            .p_4()
            .border_1()
            .border_color(color.opacity(0.22))
            .rounded_md()
            .bg(palette.panel)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pb_2()
                    .mb_3()
                    .border_b_1()
                    .border_color(color.opacity(0.18))
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(units(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color)
                            .child(TooltipText::new(
                                SharedString::from(format!("tree-{name}")),
                                format!("◆ {}", name.to_uppercase()),
                                0.14,
                            )),
                    )
                    .child(div().ml_auto().child(caption(
                        SharedString::from(format!("tree-points-{name}")),
                        format!("{spent} PTS"),
                        palette.faint,
                    ))),
            )
            .child(board)
    }
    fn details(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let class_skills = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""));
        let skill = self
            .selected
            .as_ref()
            .and_then(|id| class_skills.iter().find(|s| &s.id == id));
        let mut content = div().p_4().flex().flex_col().gap_3().bg(linear_gradient(
            180.,
            linear_color_stop(p.panel_secondary, 0.),
            linear_color_stop(p.background, 1.),
        ));
        if self.progression.is_preview() {
            return content.child(
                div()
                    .text_size(units(12.))
                    .text_color(p.muted)
                    .child("Return to the full build to inspect skill details."),
            );
        }
        let Some(skill) = skill else {
            return content.child(skill_details::empty_state(cx));
        };
        let id = skill.id.clone();
        let minus = id.clone();
        let plus = id.clone();
        let subtree = id.clone();
        let kind = skill.kind;
        let rank = snapshot.skill_ranks.get(&id).copied().unwrap_or(0);
        let available = snapshot
            .level
            .saturating_mul(data::game_config().skill_points_per_level)
            .saturating_sub(snapshot.skill_ranks.values().sum());
        let prerequisite_met = skill
            .requires_skill
            .as_ref()
            .is_none_or(|required| snapshot.skill_ranks.get(required).copied().unwrap_or(0) > 0);
        let enabled = is_enabled(snapshot, skill);
        let performance = self
            .detail
            .clone()
            .or_else(|| self.tree.read(cx).performance());
        let bonus = performance
            .as_ref()
            .filter(|_| rank > 0)
            .and_then(|result| {
                result
                    .computed
                    .rank_bonuses
                    .get(&skill.name.trim().to_lowercase())
            })
            .copied()
            .unwrap_or((0., 0.));
        let details = skill_details::DetailsContext {
            skill,
            snapshot,
            performance: performance.as_deref(),
            class_skills,
            rank,
            bonus,
        };
        let point_buttons = div()
            .ml_auto()
            .flex()
            .items_center()
            .gap_1()
            .child(
                Button::new("detail-less")
                    .planner_style(cx)
                    .xsmall()
                    .label("−")
                    .disabled(rank == 0)
                    .accessibility_label("Remove skill point")
                    .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                        this.change_rank(&minus, false, event.modifiers(), cx)
                    })),
            )
            .child(
                Button::new("detail-more")
                    .planner_style(cx)
                    .xsmall()
                    .label("+")
                    .disabled(rank >= skill.max_rank || available == 0 || !prerequisite_met)
                    .accessibility_label("Add skill point")
                    .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                        this.change_rank(&plus, true, event.modifiers(), cx)
                    })),
            );
        let toggle = (kind != SkillKind::Passive).then(|| {
            Button::new("toggle-active")
                .planner_style(cx)
                .xsmall()
                .h_auto()
                .px_2p5()
                .py_1()
                .line_height(relative(1.5))
                .font_weight(FontWeight::SEMIBOLD)
                .border_color(if enabled {
                    p.accent_deep
                } else {
                    p.border_strong
                })
                .bg(if enabled {
                    p.accent_deep.opacity(0.2)
                } else {
                    p.background.opacity(0.)
                })
                .text_color(if enabled { p.accent_hot } else { p.muted })
                .when(enabled, |b| {
                    b.shadow(vec![BoxShadow {
                        color: p.accent_hot.opacity(0.25),
                        offset: point(px(0.), px(0.)),
                        blur_radius: units(10.).to_pixels(px(13.)),
                        spread_radius: px(0.),
                        inset: false,
                    }])
                })
                .child(skill_details::caption(
                    "toggle-active-label",
                    if enabled { "✓ Active" } else { "+ Active" },
                    if enabled { p.accent_hot } else { p.muted },
                ))
                .selected(enabled)
                .on_click(cx.listener(move |this, _, _, cx| this.toggle(&id, kind, cx)))
        });
        let tags = skill_details::effective_tags(skill, snapshot);
        content = content
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap_2p5()
                            .child(div().flex_1().min_w_0().child(skill_details::header(
                                skill,
                                skill_image(skill, units(48.)),
                                cx,
                            )))
                            .children(toggle),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(skill_details::rank_row(&details, cx))
                            .child(point_buttons),
                    ),
            )
            .children(skill_details::tag_chips(&tags, cx))
            .children(skill_details::bonuses_block(&details, cx))
            .children(skill_details::stats_block(&details, cx))
            .children(skill_details::synergy_blocks(&details, cx))
            .children(skill_details::subtree_block(&details, cx))
            .children(skill.description.as_ref().map(|description| {
                div()
                    .text_size(units(12.))
                    .line_height(relative(1.6))
                    .text_color(p.muted)
                    .child(description.clone())
            }));
        if let Some(nodes) = &skill.subskills {
            let prefix = format!("{}:", skill.id);
            let spent: u32 = snapshot
                .subskill_ranks
                .iter()
                .filter(|(key, _)| key.starts_with(&prefix))
                .map(|(_, value)| *value)
                .sum();
            content = content.child(
                Button::new("open-subtree")
                    .planner_style(cx)
                    .label(format!("Subtree… · {spent} points"))
                    .disabled(nodes.is_empty())
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.subtree(&subtree, window, cx)),
                    ),
            );
        }
        content
    }
}
impl Render for SkillsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let skills = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""));
        let class = snapshot
            .class_id
            .as_deref()
            .and_then(data::get_class)
            .map(|class| class.name.clone())
            .unwrap_or_else(|| "Select a class in Config".into());
        let total = snapshot
            .level
            .saturating_mul(data::game_config().skill_points_per_level);
        let spent: u32 = snapshot.skill_ranks.values().sum();
        let mut trees: Vec<(&str, Vec<&SkillSpec>)> = Vec::new();
        for skill in skills {
            let name = skill.tree.as_deref().unwrap_or("Skills");
            if let Some((_, skills)) = trees.iter_mut().find(|(key, _)| *key == name) {
                skills.push(skill);
            } else {
                trees.push((name, vec![skill]));
            }
        }
        let widest_card = trees
            .iter()
            .map(|(_, skills)| tree_columns(skills) as f32 * STEP + 14.)
            .fold(0., f32::max);
        let scroll = self.tree_scroll.clone();
        let weak = cx.entity().downgrade();
        let had_vertical_scroll = self.tree_has_vertical_scroll;
        let details_scroll = self.details_scroll.clone();
        let details_weak = cx.entity().downgrade();
        let had_details_scroll = self.details_has_vertical_scroll;
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .bg(p.background)
            .text_color(p.text)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_3()
                    .px_5()
                    .py_3()
                    .border_b_1()
                    .border_color(p.border)
                    .child(caption("skills-title", "◆ SKILLS", p.faint))
                    .child(
                        div()
                            .text_size(units(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.accent_hot)
                            .child(class),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(caption(
                                "points",
                                format!(
                                    "POINTS {spent} / {total}  ·  {} AVAILABLE",
                                    total.saturating_sub(spent)
                                ),
                                p.accent_hot,
                            ))
                            .child(caption(
                                "skill-modifier-hint",
                                "SHIFT ×5 · CTRL/CMD+SHIFT ALL",
                                p.faint,
                            ))
                            .child(
                                Button::new("reset-skills")
                                    .planner_style(cx)
                                    .small()
                                    .label("Reset")
                                    .disabled(spent == 0)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.edit(cx, |s| s.skill_ranks.clear())
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .flex()
                    .items_stretch()
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child(
                                div()
                                    .relative()
                                    .size_full()
                                    .min_w_0()
                                    .min_h_0()
                                    .on_children_prepainted(move |_, _, cx| {
                                        let has_vertical_scroll = scroll.max_offset().y > px(0.);
                                        if has_vertical_scroll != had_vertical_scroll {
                                            let weak = weak.clone();
                                            cx.defer(move |cx| {
                                                let _ = weak.update(cx, |view, cx| {
                                                    if view.tree_has_vertical_scroll
                                                        != has_vertical_scroll
                                                    {
                                                        view.tree_has_vertical_scroll =
                                                            has_vertical_scroll;
                                                        cx.notify();
                                                    }
                                                });
                                            });
                                        }
                                    })
                                    .id("skill-trees-scroll")
                                    .overflow_scroll()
                                    .track_scroll(&self.tree_scroll)
                                    .child(
                                        div()
                                            // A dual-axis Scrollable wrapper gives its child
                                            // auto width, preventing flex-wrap. Keep this grid
                                            // bound to the actual viewport instead.
                                            .w_full()
                                            .min_w(
                                                units(
                                                    widest_card
                                                        + if self.tree_has_vertical_scroll {
                                                            10.
                                                        } else {
                                                            0.
                                                        },
                                                ) + rems(3.),
                                            )
                                            .p_6()
                                            .when(self.tree_has_vertical_scroll, |view| {
                                                view.pr(rems(1.5) + units(10.))
                                            })
                                            .flex()
                                            .flex_wrap()
                                            .items_start()
                                            .justify_center()
                                            .gap_6()
                                            .children(trees.iter().map(|(name, skills)| {
                                                self.tree_panel(name, skills, window, cx)
                                            }))
                                            .when(trees.is_empty(), |v| {
                                                v.child(
                                                    "Choose a class in Config to view its skills.",
                                                )
                                            }),
                                    ),
                            )
                            .child(self.progression_bar(window, cx)),
                    )
                    .child(
                        div()
                            .w(units(440.))
                            .flex_shrink_0()
                            .min_h_0()
                            .border_l_1()
                            .border_color(p.border)
                            .bg(p.panel)
                            .child(
                                div()
                                    .on_children_prepainted(move |_, _, cx| {
                                        let has_scroll = details_scroll.max_offset().y > px(0.);
                                        if has_scroll != had_details_scroll {
                                            let weak = details_weak.clone();
                                            cx.defer(move |cx| {
                                                let _ = weak.update(cx, |view, cx| {
                                                    if view.details_has_vertical_scroll
                                                        != has_scroll
                                                    {
                                                        view.details_has_vertical_scroll =
                                                            has_scroll;
                                                        cx.notify();
                                                    }
                                                });
                                            });
                                        }
                                    })
                                    .id("skill-details-scroll")
                                    .size_full()
                                    .track_scroll(&self.details_scroll)
                                    .scrollbar_width(units(if self.details_has_vertical_scroll {
                                        10.
                                    } else {
                                        0.
                                    }))
                                    .child(self.details(cx))
                                    .overflow_y_scroll(),
                            ),
                    ),
            )
    }
}

const SUB_POSITIONS: [(f32, f32); 15] = [
    (0.5, 0.87),
    (0.453, 0.724),
    (0.547, 0.724),
    (0.253, 0.579),
    (0.406, 0.579),
    (0.5, 0.579),
    (0.594, 0.579),
    (0.747, 0.579),
    (0.347, 0.399),
    (0.653, 0.399),
    (0.5, 0.289),
    (0.1, 0.579),
    (0.253, 0.109),
    (0.747, 0.109),
    (0.9, 0.579),
];
const SUB_EDGES: [(usize, usize); 20] = [
    (0, 1),
    (0, 2),
    (1, 4),
    (2, 6),
    (3, 4),
    (3, 11),
    (4, 5),
    (4, 8),
    (5, 6),
    (6, 7),
    (6, 9),
    (7, 14),
    (8, 10),
    (8, 11),
    (8, 12),
    (9, 10),
    (9, 13),
    (9, 14),
    (10, 12),
    (10, 13),
];
// Tile diameter as a share of the board, by template role (minor, notable, keystone).
const TILE_PCT: [f32; 3] = [7.8, 9.8, 12.4];
fn tile_role(position: usize) -> usize {
    if position == 0 {
        2
    } else if position >= 11 {
        1
    } else {
        0
    }
}
fn at_rank(base: Option<f64>, per_rank: Option<f64>, rank: u32) -> f64 {
    base.unwrap_or(0.) + per_rank.unwrap_or(0.) * rank as f64
}
fn effect_rows(
    effects: Option<&SubskillEffectSpec>,
    rank: u32,
    next: u32,
) -> Vec<(String, f64, f64)> {
    let Some(effects) = effects else {
        return Vec::new();
    };
    let mut keys: Vec<&String> = effects
        .base
        .iter()
        .chain(effects.per_rank.iter())
        .flat_map(|map| map.keys())
        .collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .map(|key| {
            let base = effects.base.as_ref().and_then(|m| m.get(key)).copied();
            let per = effects.per_rank.as_ref().and_then(|m| m.get(key)).copied();
            (
                key.clone(),
                at_rank(base, per, rank),
                at_rank(base, per, next),
            )
        })
        .collect()
}
fn stat_display(key: &str, value: f64) -> (String, String) {
    let definition = data::game_config().stats.iter().find(|s| s.key == key);
    let label = definition
        .map(|s| s.name.clone())
        .unwrap_or_else(|| key.to_owned());
    let percent = definition.is_some_and(|s| s.format.as_deref() == Some("percent"));
    (label, format_range((value, value), percent))
}

struct SubtreeView {
    session: Entity<Session>,
    skill: SkillSpec,
    preview: Option<(String, Arc<PreviewResult>)>,
    preview_task: Option<Task<()>>,
    preview_request: PreviewRequest,
    preview_node: Option<String>,
    progression: RankProgression,
    progression_slider: Entity<SliderState>,
    progression_focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}
impl SubtreeView {
    fn new(session: Entity<Session>, skill: SkillSpec, cx: &mut Context<Self>) -> Self {
        let progression = RankProgression::for_subtree(session.read(cx).draft(), &skill);
        let progression_slider = cx.new(|_| progression::slider_state(progression.total()));
        let subscriptions = vec![
            cx.observe(&session, |this, _, cx| {
                this.sync_progression(cx);
                this.refresh_node_preview(cx);
                cx.notify();
            }),
            cx.observe(&progression_slider, |this, slider, cx| {
                let step = slider.read(cx).value().end().round() as usize;
                if this.progression.set_step(step) {
                    this.refresh_node_preview(cx);
                    cx.notify();
                }
            }),
        ];
        Self {
            session,
            skill,
            preview: None,
            preview_task: None,
            preview_request: PreviewRequest::default(),
            preview_node: None,
            progression,
            progression_slider,
            progression_focus: cx.focus_handle().tab_stop(true),
            _subscriptions: subscriptions,
        }
    }
    fn preview_for(&self, node_id: &str) -> Option<Arc<PreviewResult>> {
        if self.progression.is_preview() {
            return None;
        }
        self.preview
            .as_ref()
            .filter(|(id, _)| id == node_id)
            .map(|(_, result)| result.clone())
    }
    // Net Change for the hovered node: the build with one more rank, compared in the background.
    fn request_preview(&mut self, node_id: String, cx: &mut Context<Self>) {
        self.preview_node = Some(node_id.clone());
        if self.progression.is_preview()
            || self.preview.as_ref().is_some_and(|(id, _)| *id == node_id)
        {
            return;
        }
        let snapshot = self.session.read(cx).snapshot();
        let Some(node) = self
            .skill
            .subskills
            .iter()
            .flatten()
            .find(|node| node.id == node_id)
        else {
            return;
        };
        let key = format!("{}:{}", self.skill.id, node.id);
        let rank = snapshot.subskill_ranks.get(&key).copied().unwrap_or(0);
        if rank >= node.max_rank {
            return;
        }
        let Some(revision) = self.preview_request.begin(&node_id) else {
            return;
        };
        let before = snapshot.planner_input();
        // Preview the next rank even with no points left, like the reference tooltip.
        let mut next = snapshot.clone();
        next.subskill_ranks.insert(key, rank + 1);
        let after = next.planner_input();
        // Dropping the previous handle cancels its waiter. Its already-running
        // calculation may finish, but only the current target/generation is accepted.
        self.preview_task = None;
        self.preview = None;
        self.preview_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let diffs =
                        compare_planner(&planner::evaluate(&before), &planner::evaluate(&after));
                    Arc::new(PreviewResult {
                        single: diffs.clone(),
                        path: diffs,
                        added: 1,
                        removed: 0,
                    })
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.preview_request.finish(&node_id, revision) {
                    this.preview_task = None;
                    this.preview = Some((node_id, result));
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }
    // Only the subtree re-requests after a change. Tooltips must not: a leaving
    // tooltip lingers 500ms next to the new one and the two would ping-pong forever.
    fn refresh_node_preview(&mut self, cx: &mut Context<Self>) {
        self.clear_node_preview();
        if let Some(node_id) = self.preview_node.clone() {
            self.request_preview(node_id, cx);
        }
    }
    fn points(&self, snapshot: &BuildSnapshot) -> (u32, u32) {
        let prefix = format!("{}:", self.skill.id);
        let spent = snapshot
            .subskill_ranks
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(_, v)| *v)
            .sum();
        let total = snapshot.level / data::game_config().levels_per_subskill_point.max(1);
        (spent, total)
    }
    fn change(&mut self, id: &str, increase: bool, modifiers: Modifiers, cx: &mut Context<Self>) {
        if self.finish_progression(cx) {
            return;
        }
        let snapshot = self.session.read(cx).snapshot();
        let key = format!("{}:{id}", self.skill.id);
        let rank = snapshot.subskill_ranks.get(&key).copied().unwrap_or(0);
        let (spent, total) = self.points(snapshot);
        let amount = allocation_amount(
            modifiers,
            if increase {
                total.saturating_sub(spent)
            } else {
                rank
            },
        );
        let skill = self.skill.id.clone();
        self.session.update(cx, |session, cx| {
            session.edit(|draft| {
                draft.snapshot.set_subskill_rank(
                    &skill,
                    id,
                    if increase {
                        rank.saturating_add(amount)
                    } else {
                        rank.saturating_sub(amount)
                    },
                )
            });
            cx.notify();
        });
    }
    fn node_image(&self, node: &SubskillNodeSpec) -> Option<Arc<Image>> {
        let key = format!(
            "{}/{}_{}_subskill_{}",
            self.skill.class_id, self.skill.class_id, self.skill.id, node.position_index
        );
        ICONS.get(key.as_str()).cloned().or_else(|| {
            (node.position_index == 0)
                .then(|| skill_icon(&self.skill.class_id, &self.skill.id))
                .flatten()
        })
    }
    fn tile(
        &self,
        index: usize,
        node: &SubskillNodeSpec,
        rank: u32,
        board: f32,
        window: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let p = cx.global::<TooltipTheme>();
        let (x, y) = SUB_POSITIONS[index];
        let role = tile_role(index);
        let size = board * TILE_PCT[role] / 100.;
        let root = role == 2;
        let allocated = rank > 0;
        let marker =
            self.progression.marker() == Some(format!("{}:{}", self.skill.id, node.id).as_str());
        let rem_glow = |color: Hsla, blur: f32| BoxShadow {
            color,
            offset: point(px(0.), px(0.)),
            blur_radius: units(blur).to_pixels(px(13.)),
            spread_radius: px(0.),
            inset: false,
        };
        let border = if root {
            p.negative
        } else if allocated {
            p.accent_hot.opacity(0.9)
        } else if role == 1 {
            p.accent_deep
        } else {
            p.border_strong
        };
        let add = node.id.clone();
        let remove = node.id.clone();
        let tooltip_skill = self.skill.clone();
        let tooltip_node = node.clone();
        let subtree = cx.entity().downgrade();
        let mut face = Button::new(SharedString::from(format!("tile-{}", node.id)))
            .ghost()
            .size_full()
            .p(units(size * 0.05))
            .rounded_full()
            .overflow_hidden()
            .border_1()
            .when(allocated || root, |b| b.border_2())
            .border_color(border)
            .bg(if root {
                linear_gradient(
                    180.,
                    linear_color_stop(p.negative.opacity(0.45), 0.),
                    linear_color_stop(p.background, 1.),
                )
            } else {
                linear_gradient(
                    180.,
                    linear_color_stop(p.panel_secondary, 0.),
                    linear_color_stop(p.background, 1.),
                )
            })
            .when(!allocated && !root, |b| b.opacity(0.65))
            .when(root, |b| {
                b.shadow(vec![rem_glow(p.negative.opacity(0.35), 16.)])
            })
            .when(allocated && !root, |b| {
                b.shadow(vec![rem_glow(p.accent_hot.opacity(0.35), 12.)])
            })
            .accessibility_label(format!("{} — rank {rank} of {}", node.name, node.max_rank));
        face = match self.node_image(node) {
            Some(image) => face.child(img(image).size_full().object_fit(ObjectFit::Contain)),
            None => face.child(
                div()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(15.))
                    .text_color(if root {
                        p.angelic
                    } else {
                        p.accent_hot.opacity(0.85)
                    })
                    .child("◆"),
            ),
        };
        if !root {
            face = face.on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                this.change(&add, true, event.modifiers(), cx)
            }));
        }
        div()
            .id(SharedString::from(format!("{}:{}", self.skill.id, node.id)))
            .absolute()
            .left(units(x * board - size / 2.))
            .top(units(y * board - size / 2.))
            .size(units(size))
            .rounded_full()
            .when(marker, |tile| {
                tile.shadow(vec![
                    BoxShadow {
                        color: p.accent_hot.opacity(0.9),
                        offset: point(px(0.), px(0.)),
                        blur_radius: px(0.),
                        spread_radius: units(2.5).to_pixels(window.rem_size()),
                        inset: false,
                    },
                    BoxShadow {
                        color: p.accent_hot.opacity(0.6),
                        offset: point(px(0.), px(0.)),
                        blur_radius: units(16.).to_pixels(window.rem_size()),
                        spread_radius: px(0.),
                        inset: false,
                    },
                ])
            })
            .cursor_tooltip_view(move |_, cx| {
                let skill = tooltip_skill.clone();
                let node = tooltip_node.clone();
                let subtree = subtree.clone();
                if !root {
                    let node_id = node.id.clone();
                    let _ = subtree.update(cx, |this, cx| this.request_preview(node_id, cx));
                }
                cx.new(|cx| SubskillTooltip::new(skill, node, rank, root, subtree, cx))
                    .into()
            })
            .child(face)
            .when(!root, |v| {
                v.child(
                    div()
                        .absolute()
                        .top(units(size + 5.))
                        .left(units(-size))
                        .w(units(size * 3.))
                        .flex()
                        .justify_center()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(10.))
                        .text_color(if allocated { p.accent_hot } else { p.faint })
                        .child(format!("{rank}/{}", node.max_rank)),
                )
            })
            .when(!root, |v| {
                v.on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        this.change(&remove, false, event.modifiers, cx)
                    }),
                )
            })
    }
}
/// Width of the complete bordered square, in the reference's 13px-root units.
fn subtree_board_size(window: &Window) -> f32 {
    let scale = f32::from(window.rem_size()) / 13.;
    let viewport = window.viewport_size();
    (f32::from(viewport.height) / scale - 230.)
        .min(592.)
        .min(f32::from(viewport.width) / scale - 73.5)
        .max(120.)
}

impl Render for SubtreeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let (spent, total) = self.points(snapshot);
        let remaining = total.saturating_sub(spent);
        let color = self
            .skill
            .damage_type
            .as_deref()
            .map(theme::damage_color)
            .unwrap_or(p.accent_hot);
        let rem = window.rem_size();
        // Tauri caps the OUTER square; p-4 and the border are inside that width.
        let outer_board = subtree_board_size(window);
        let board = outer_board - 28.;
        let nodes: HashMap<usize, &SubskillNodeSpec> = self
            .skill
            .subskills
            .iter()
            .flatten()
            .map(|node| (node.position_index as usize, node))
            .collect();
        let rank_at = |index: usize| {
            nodes
                .get(&index)
                .and_then(|node| {
                    self.progression
                        .visible()
                        .get(&format!("{}:{}", self.skill.id, node.id))
                })
                .copied()
                .unwrap_or(0)
        };
        let edges: Vec<_> = SUB_EDGES
            .iter()
            .map(|&(a, b)| {
                let stroke = if rank_at(a) > 0 && rank_at(b) > 0 {
                    p.accent_hot.opacity(0.55)
                } else if a == 0 || b == 0 {
                    p.negative.opacity(0.35)
                } else {
                    p.faint.opacity(0.3)
                };
                (SUB_POSITIONS[a], SUB_POSITIONS[b], stroke)
            })
            .collect();
        let (dash, gap) = (rem * (4. / 13.), rem * (5. / 13.));
        let mut canvas_board = div().relative().size(units(board)).flex_shrink_0().child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    for (a, b, stroke) in edges {
                        let start = bounds.origin
                            + point(bounds.size.width * a.0, bounds.size.height * a.1);
                        let end = bounds.origin
                            + point(bounds.size.width * b.0, bounds.size.height * b.1);
                        let (dx, dy) = (f32::from(end.x - start.x), f32::from(end.y - start.y));
                        let length = (dx * dx + dy * dy).sqrt();
                        let mut path = PathBuilder::stroke(px(2.));
                        let mut t = 0.;
                        while t < length {
                            let stop = (t + f32::from(dash)).min(length);
                            let at =
                                |d: f32| start + point(px(dx * d / length), px(dy * d / length));
                            path.move_to(at(t));
                            path.line_to(at(stop));
                            t += f32::from(dash + gap);
                        }
                        if let Ok(path) = path.build() {
                            window.paint_path(path, stroke);
                        }
                    }
                },
            )
            .absolute()
            .size_full(),
        );
        for index in 0..SUB_POSITIONS.len() {
            match nodes.get(&index) {
                Some(node) => {
                    canvas_board = canvas_board.child(self.tile(
                        index,
                        node,
                        rank_at(index),
                        board,
                        window,
                        cx,
                    ));
                }
                None => {
                    let (x, y) = SUB_POSITIONS[index];
                    let size = board * TILE_PCT[tile_role(index)] / 100.;
                    canvas_board = canvas_board.child(
                        div()
                            .absolute()
                            .left(units(x * board - size / 2.))
                            .top(units(y * board - size / 2.))
                            .size(units(size))
                            .rounded_full()
                            .border_1()
                            .border_color(p.border)
                            .opacity(0.5),
                    );
                }
            }
        }
        let skill_id = self.skill.id.clone();
        let hint = |id: &'static str, text: &'static str| {
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(units(10.))
                .text_color(p.faint)
                .child(TooltipText::new(id, text.to_uppercase(), 0.14))
        };
        div()
            .flex()
            .flex_col()
            .line_height(relative(1.5))
            .text_color(p.text)
            .bg(linear_gradient(
                180.,
                linear_color_stop(p.panel_secondary, 0.),
                linear_color_stop(p.background, 1.),
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .justify_between()
                    .gap_x_3()
                    .gap_y_2()
                    .px_6()
                    .py_4()
                    .border_b_1()
                    .border_color(p.border)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(units(169.))
                            .child(
                                div()
                                    .mb_1p5()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .text_size(units(10.))
                                    .text_color(p.faint)
                                    .child(div().size(rems(0.25)).rounded_full().bg(p.accent))
                                    .child(TooltipText::new(
                                        "subtree-eyebrow",
                                        "SKILL SUBTREE",
                                        0.12,
                                    )),
                            )
                            .child(
                                div()
                                    .text_size(units(17.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(TooltipText::new(
                                        "subtree-title",
                                        self.skill.name.clone(),
                                        -0.01,
                                    )),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_size(units(12.))
                                    .text_color(p.muted)
                                    .child("Specialize · Boost · Change how this skill works"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .ml_auto()
                            .flex_shrink_0()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap_1p5()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .text_size(units(11.))
                                    .text_color(p.faint)
                                    .child("Points")
                                    .child(
                                        div()
                                            .text_size(units(13.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(p.accent_hot)
                                            .child(format!("{spent}/{total}")),
                                    )
                                    .child(caption(
                                        "subtree-remaining",
                                        if remaining > 0 {
                                            format!("{remaining} LEFT")
                                        } else {
                                            "ALL SPENT".into()
                                        },
                                        if remaining > 0 {
                                            p.accent_deep
                                        } else {
                                            p.faint
                                        },
                                    )),
                            )
                            .child(
                                Button::new("reset-subtree")
                                    .planner_style(cx)
                                    .small()
                                    .h_auto()
                                    .px_3p5()
                                    .py_1p5()
                                    .line_height(relative(1.5))
                                    .bg(p.background.opacity(0.))
                                    .child(caption("subtree-reset-label", "RESET", p.muted))
                                    .disabled(spent == 0)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let prefix = format!("{skill_id}:");
                                        this.session.update(cx, |session, cx| {
                                            session.edit(|draft| {
                                                draft
                                                    .snapshot
                                                    .subskill_ranks
                                                    .retain(|key, _| !key.starts_with(&prefix))
                                            });
                                            cx.notify();
                                        });
                                    })),
                            )
                            .child(
                                Button::new("close-subtree")
                                    .planner_style(cx)
                                    .small()
                                    .h_auto()
                                    .px_3()
                                    .py_1p5()
                                    .text_size(units(12.))
                                    .line_height(relative(1.5))
                                    .rounded_md()
                                    .border_color(p.border)
                                    .bg(p.background.opacity(0.))
                                    .label("Close")
                                    .on_click(|_, window, cx| window.close_dialog(cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .relative()
                    .p_5()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .relative()
                            .p_4()
                            .rounded_md()
                            .border_1()
                            .border_color(color.opacity(0.22))
                            .bg(p.panel)
                            .child(
                                // Composite alpha stops before interpolation to avoid the
                                // bright midpoint from straight-alpha gold → opaque black.
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .size_full()
                                    .rounded_md()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .child(div().h(relative(0.22)).flex_shrink_0().bg(
                                        linear_gradient(
                                            180.,
                                            linear_color_stop(
                                                p.panel.blend(color.opacity(0.06)),
                                                0.,
                                            ),
                                            linear_color_stop(p.panel, 1.),
                                        ),
                                    ))
                                    .child(div().flex_1().bg(linear_gradient(
                                        180.,
                                        linear_color_stop(p.panel, 0.),
                                        linear_color_stop(
                                            p.panel.blend(p.background.opacity(0.7)),
                                            1.,
                                        ),
                                    ))),
                            )
                            .shadow(vec![BoxShadow {
                                color: p.shadow.opacity(0.35),
                                offset: point(px(0.), rem * (8. / 13.)),
                                blur_radius: rem * (24. / 13.),
                                spread_radius: px(0.),
                                inset: false,
                            }])
                            .child(hsplanner_ui::components::corner_marks(cx))
                            .child(canvas_board),
                    )
                    .when(self.progression.total() >= 2, |body| {
                        body.child(self.progression_bar(window, cx))
                    })
                    .when(nodes.is_empty(), |v| {
                        v.child(caption(
                            "subtree-empty",
                            format!(
                                "NO SUBSKILLS DEFINED FOR {} YET.",
                                self.skill.name.to_uppercase()
                            ),
                            p.faint,
                        ))
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_6()
                    .py_2p5()
                    .border_t_1()
                    .border_color(p.border)
                    .child(hint("subtree-hint-click", "L-Click add · R-Click remove"))
                    .child(hint("subtree-hint-mods", "Shift ×5 · Ctrl/Cmd+Shift all")),
            )
    }
}

struct SubskillTooltip {
    skill: SkillSpec,
    node: SubskillNodeSpec,
    rank: u32,
    root: bool,
    subtree: WeakEntity<SubtreeView>,
    _subscription: Option<Subscription>,
}
impl SubskillTooltip {
    fn new(
        skill: SkillSpec,
        node: SubskillNodeSpec,
        rank: u32,
        root: bool,
        subtree: WeakEntity<SubtreeView>,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscription = subtree.upgrade().map(|entity| {
            cx.observe(&entity, |this, entity, cx| {
                this.rank = entity
                    .read(cx)
                    .progression
                    .visible()
                    .get(&format!("{}:{}", this.skill.id, this.node.id))
                    .copied()
                    .unwrap_or(0);
                cx.notify();
            })
        });
        Self {
            skill,
            node,
            rank,
            root,
            subtree,
            _subscription: subscription,
        }
    }
    fn rank_value(current: String, next: Option<String>, cx: &App) -> Div {
        let p = cx.global::<TooltipTheme>();
        div()
            .flex_shrink_0()
            .flex()
            .items_baseline()
            .gap_1()
            .font_family(theme::MONO_FONT_FAMILY)
            .child(div().text_color(p.muted).child(current))
            .when_some(next, |v, next| {
                v.child(div().text_color(p.faint).child("›"))
                    .child(div().text_color(p.accent_hot).child(next))
            })
    }
    fn stat(label: impl Into<SharedString>, value: Div, cx: &App) -> Div {
        div()
            .flex_shrink_0()
            .flex()
            .items_baseline()
            .justify_between()
            .gap_3()
            .text_size(units(12.))
            .child(
                div()
                    .text_color(cx.global::<TooltipTheme>().text)
                    .child(label.into()),
            )
            .child(value)
    }
    fn section(cx: &App) -> Div {
        div()
            .flex_shrink_0()
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .gap_0p5()
            .border_t_1()
            .border_color(cx.global::<TooltipTheme>().border.opacity(0.7))
    }
    fn section_label(id: &'static str, text: String, cx: &App) -> Div {
        div()
            .font_family(theme::MONO_FONT_FAMILY)
            .text_size(units(9.))
            .text_color(cx.global::<TooltipTheme>().muted)
            .child(TooltipText::new(id, text.to_uppercase(), 0.14))
    }
}
impl Render for SubskillTooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let rem = window.rem_size();
        let next = (self.rank + 1).min(self.node.max_rank);
        let has_next = next > self.rank;
        let next_value = |current: f64, upcoming: f64, format: &dyn Fn(f64) -> String| {
            (has_next && current != upcoming).then(|| format(upcoming))
        };
        let icon_key = format!(
            "{}/{}_{}_subskill_{}",
            self.skill.class_id, self.skill.class_id, self.skill.id, self.node.position_index
        );
        let mut panel =
            div()
                .flex()
                .flex_col()
                .w(units(400.))
                .max_w(window.viewport_size().width - rem * (24. / 13.))
                .bg(p.panel)
                .text_color(p.text)
                .line_height(relative(1.5))
                .rounded(units(4.))
                .border_1()
                .border_color(p.accent.opacity(0.6))
                .overflow_hidden()
                .shadow(vec![BoxShadow {
                    color: p.shadow.opacity(0.8),
                    offset: point(px(0.), rem * (8. / 13.)),
                    blur_radius: rem * (32. / 13.),
                    spread_radius: px(0.),
                    inset: false,
                }])
                .child(
                    div()
                        .flex_shrink_0()
                        .px_3()
                        .py_2()
                        .flex()
                        .items_center()
                        .gap_2()
                        .bg(linear_gradient(
                            180.,
                            linear_color_stop(p.accent.opacity(0.14), 0.),
                            linear_color_stop(p.accent.opacity(0.04), 1.),
                        ))
                        .when_some(ICONS.get(icon_key.as_str()), |v, image| {
                            v.child(
                                img(image.clone())
                                    .size(units(32.))
                                    .flex_shrink_0()
                                    .object_fit(ObjectFit::Contain),
                            )
                        })
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.accent_hot)
                                        .child(self.node.name.clone()),
                                )
                                .when(!self.root, |v| {
                                    v.child(div().text_size(units(11.)).text_color(p.muted).child(
                                        format!("Rank {} / {}", self.rank, self.node.max_rank),
                                    ))
                                }),
                        ),
                );
        if let Some(description) = &self.node.description {
            panel = panel.child(
                Self::section(cx).child(
                    div()
                        .text_size(units(12.))
                        .text_color(p.text)
                        .child(description.clone()),
                ),
            );
        }
        if let Some(tags) = data::data()
            .subskill_tags
            .get(&self.skill.id)
            .and_then(|by_node| by_node.get(&self.node.id))
        {
            let chip = |text: String, color: Hsla, border: Hsla| {
                div()
                    .px_1p5()
                    .py_0p5()
                    .rounded_sm()
                    .border_1()
                    .border_color(border)
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(9.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color)
                    .child(text)
            };
            panel = panel.child(
                Self::section(cx).child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_1()
                        .child(Self::section_label("subskill-tags", "Tags".into(), cx))
                        .children(tags.add.iter().map(|tag| {
                            chip(format!("+{tag}"), p.accent_hot, p.accent_hot.opacity(0.7))
                        }))
                        .children(
                            tags.remove
                                .iter()
                                .map(|tag| chip(tag.clone(), p.faint, p.border).line_through()),
                        ),
                ),
            );
        }
        let rows = effect_rows(self.node.effects.as_ref(), self.rank, next);
        if !rows.is_empty() {
            let mut section = Self::section(cx);
            for (key, current, upcoming) in rows {
                let (label, shown) = stat_display(&key, current);
                let format = |value: f64| stat_display(&key, value).1;
                section = section.child(Self::stat(
                    label,
                    Self::rank_value(shown, next_value(current, upcoming, &format), cx),
                    cx,
                ));
            }
            panel = panel.child(section);
        }
        if let Some(proc) = &self.node.proc {
            let chance = at_rank(proc.chance.base, proc.chance.per_rank, self.rank);
            let chance_next = at_rank(proc.chance.base, proc.chance.per_rank, next);
            let percent = |value: f64| format!("{value}%");
            let mut trailing: Vec<String> = proc
                .applies_states
                .iter()
                .flatten()
                .map(|state| match state {
                    AppliedStateValue::Name(name) => format!("applies {}", name.replace('_', " ")),
                    AppliedStateValue::Full { state, .. } => {
                        format!("applies {}", state.replace('_', " "))
                    }
                })
                .collect();
            trailing.extend(proc.tags.iter().flatten().cloned());
            let mut section = Self::section(cx).child(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap_3()
                    .mb_1()
                    .child(Self::section_label(
                        "subskill-proc",
                        format!("{} proc", proc.trigger.replace('_', " ")),
                        cx,
                    ))
                    .when(!trailing.is_empty(), |v| {
                        v.child(
                            div()
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(units(9.))
                                .text_color(p.accent_hot.opacity(0.8))
                                .child(trailing.join(" · ").to_uppercase()),
                        )
                    }),
            );
            section = section.child(Self::stat(
                "Proc Chance",
                Self::rank_value(
                    percent(chance),
                    next_value(chance, chance_next, &percent),
                    cx,
                ),
                cx,
            ));
            for (key, current, upcoming) in effect_rows(proc.effects.as_ref(), self.rank, next) {
                let (label, shown) = stat_display(&key, current);
                let format = |value: f64| stat_display(&key, value).1;
                let average = chance / 100. * current;
                let value = Self::rank_value(shown, next_value(current, upcoming, &format), cx)
                    .when(average > 0., |v| {
                        v.child(
                            div()
                                .text_size(units(10.))
                                .text_color(p.faint)
                                .child(format!("(avg {})", stat_display(&key, average).1)),
                        )
                    });
                section = section.child(Self::stat(label, value, cx));
            }
            for state in proc.applies_states.iter().flatten() {
                if let AppliedStateValue::Full { state, amount } = state {
                    let amount = amount.as_ref();
                    let current = at_rank(
                        amount.and_then(|a| a.base),
                        amount.and_then(|a| a.per_rank),
                        self.rank,
                    );
                    let upcoming = at_rank(
                        amount.and_then(|a| a.base),
                        amount.and_then(|a| a.per_rank),
                        next,
                    );
                    section = section.child(Self::stat(
                        state.replace('_', " "),
                        Self::rank_value(
                            percent(current),
                            next_value(current, upcoming, &percent),
                            cx,
                        ),
                        cx,
                    ));
                }
            }
            panel = panel.child(section);
        }
        let subtree = self.subtree.upgrade();
        if !self.root
            && has_next
            && subtree
                .as_ref()
                .is_some_and(|view| !view.read(cx).progression.is_preview())
        {
            let preview = subtree.and_then(|subtree| subtree.read(cx).preview_for(&self.node.id));
            panel = panel.child(Self::section(cx).child(preview_changes(
                preview.as_deref(),
                preview.is_none(),
                Some(8),
                cx,
            )));
        }
        div()
            .id("subskill-tooltip-scroll")
            .max_h(window.viewport_size().height - rem * (24. / 13.))
            .overflow_y_scroll()
            .child(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn allocation_modifiers_respect_the_remaining_budget() {
        assert_eq!(allocation_amount(Modifiers::default(), 12), 1);
        let shift = Modifiers {
            shift: true,
            ..Default::default()
        };
        assert_eq!(allocation_amount(shift, 12), 5);
        assert_eq!(allocation_amount(shift, 3), 3);
        assert_eq!(
            allocation_amount(
                Modifiers {
                    control: true,
                    ..shift
                },
                12
            ),
            12
        );
        assert_eq!(
            allocation_amount(
                Modifiers {
                    platform: true,
                    ..shift
                },
                12
            ),
            12
        );
        assert_eq!(allocation_amount(shift, 0), 0);
    }

    #[::core::prelude::v1::test]
    fn rank_actions_clamp_to_budget_and_maximum_and_remove_zero_rank_keys() {
        let mut snapshot = BuildSnapshot {
            class_id: Some("stormweaver".into()),
            level: 1,
            ..Default::default()
        };
        let all = Modifiers {
            shift: true,
            control: true,
            ..Default::default()
        };
        adjust_skill_rank(&mut snapshot, "charged_bolts", true, all);
        let budget = data::game_config().skill_points_per_level;
        assert_eq!(snapshot.skill_ranks["charged_bolts"], budget);
        adjust_skill_rank(&mut snapshot, "charged_bolts", true, Modifiers::default());
        assert_eq!(snapshot.skill_ranks["charged_bolts"], budget);

        snapshot.level = 100;
        let max = data::get_skills_by_class("stormweaver")
            .iter()
            .find(|skill| skill.id == "charged_bolts")
            .unwrap()
            .max_rank;
        adjust_skill_rank(&mut snapshot, "charged_bolts", true, all);
        assert_eq!(snapshot.skill_ranks["charged_bolts"], max);
        snapshot.active_skill_ids.push("charged_bolts".into());
        adjust_skill_rank(&mut snapshot, "charged_bolts", false, all);
        assert!(!snapshot.skill_ranks.contains_key("charged_bolts"));
        // Removing ranks retains the activation preference, as in Tauri.
        assert_eq!(snapshot.active_skill_ids, ["charged_bolts"]);
        adjust_skill_rank(&mut snapshot, "charged_bolts", false, Modifiers::default());
        assert!(snapshot.skill_ranks.is_empty());
        adjust_skill_rank(&mut snapshot, "unknown-skill", true, all);
        assert!(snapshot.skill_ranks.is_empty());
    }

    #[::core::prelude::v1::test]
    fn skill_coordinates_exist_for_all_playable_class_skills() {
        for class in data::data().classes.values() {
            for skill in data::get_skills_by_class(&class.id) {
                assert!(position(skill).is_some(), "{} / {}", class.id, skill.id);
            }
        }
    }
}
