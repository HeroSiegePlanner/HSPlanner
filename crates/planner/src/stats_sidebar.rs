//! Persistent character summary matching the reference's left planner panel.
use crate::TreeView;
use gpui_kit::{prelude::*, *};
use hsplanner_build::{BuildSnapshot, session::Session};
use hsplanner_engine::calc::{
    data,
    defense::{EhpResult, effective_cap},
    planner::PlannerPerformance,
    skills::Ranged,
    types::SkillKind,
};
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::scroll::PageScroll;
use hsplanner_ui::tooltip::CursorTooltipExt;
use hsplanner_ui::{
    numbers::{compact, compact_range},
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};
use std::sync::Arc;

const ATTRIBUTES: &[&str] = &[
    "strength",
    "dexterity",
    "intelligence",
    "energy",
    "vitality",
    "armor",
];
const OFFENSE: &[&str] = &[
    "enhanced_damage",
    "attack_damage",
    "increased_attack_speed",
    "faster_cast_rate",
    "crit_chance",
    "crit_damage",
    "life_steal",
    "mana_steal",
];
const DEFENSE: &[&str] = &[
    "life",
    "mana",
    "life_replenish",
    "mana_replenish",
    "block_chance",
    "physical_damage_reduction",
    "magic_damage_reduction",
];
fn units(value: f32) -> Rems {
    rems(value / 13.)
}
fn section_body() -> Div {
    div().w_full().min_w_0().flex().flex_col().gap(units(1.))
}
fn section(key: &'static str, title: &'static str, content: Div, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(p.border.opacity(0.7))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .mb_2()
                .text_color(p.accent_hot.opacity(0.7))
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(units(10.))
                .child(div().text_color(p.accent_deep).child("◆"))
                .child(TooltipText::new(key, title.to_uppercase(), 0.18)),
        )
        .child(content)
}
fn divider(cx: &App) -> Div {
    div()
        .my_2()
        .border_t_1()
        .border_color(cx.global::<TooltipTheme>().accent_deep.opacity(0.3))
}
fn row(label: &str, value: String, label_color: Hsla, value_color: Hsla) -> Div {
    row_with_note(label, value, None, label_color, value_color)
}
// Keep each numeric range intact. DR belongs below its effective value, not
// in a wrapping flex row where anonymous text can claim the entire row width.
fn row_with_note(
    label: &str,
    value: String,
    note: Option<(String, Hsla)>,
    label_color: Hsla,
    value_color: Hsla,
) -> Div {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_start()
        .justify_between()
        .gap_2()
        .py(rems(0.1875))
        .text_size(units(12.))
        .line_height(relative(1.4))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_color(label_color)
                .child(label.to_owned()),
        )
        .child(
            div()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .items_end()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_color(value_color)
                .text_align(TextAlign::Right)
                .child(div().whitespace_nowrap().child(value))
                .children(note.map(|(note, color)| {
                    div()
                        .whitespace_nowrap()
                        .text_size(units(10.))
                        .text_color(color)
                        .child(note)
                })),
        )
}

// Decimals shrink as values grow so ranges fit the narrow value column.
fn number(value: f64) -> String {
    let decimals = match value.abs() {
        v if v >= 100. => 0,
        v if v >= 10. => 1,
        _ => 2,
    };
    let text = format!("{value:.decimals$}");
    if decimals == 0 {
        text
    } else {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}
fn precise(value: Ranged) -> String {
    let (min, max) = (number(value.0), number(value.1));
    if min == max {
        min
    } else {
        format!("{min}–{max}")
    }
}
fn signed_stat(value: Ranged, percent: bool) -> String {
    let sign = if value.0 >= 0. { "+" } else { "" };
    let suffix = if percent { "%" } else { "" };
    format!("{sign}{}{suffix}", precise(value))
}
fn ehp_rows(ehp: &EhpResult) -> Vec<(String, Option<f64>)> {
    if ehp.entries.is_empty() {
        return vec![];
    }
    let physical = ehp
        .entries
        .iter()
        .find(|entry| entry.damage_type == "physical")
        .and_then(|entry| entry.ehp);
    let elements = ehp
        .entries
        .iter()
        .filter(|entry| entry.damage_type != "physical")
        .collect::<Vec<_>>();
    let same = |a: Option<f64>, b: Option<f64>| a.map(f64::round) == b.map(f64::round);
    if let Some(first) = elements.first()
        && elements.iter().all(|entry| same(entry.ehp, first.ehp))
    {
        if same(physical, first.ehp) {
            return vec![(tr("planner.common.ehp").into(), physical)];
        }
        return vec![
            (tr("planner.common.physical_ehp").into(), physical),
            (tr("planner.common.elemental_ehp").into(), first.ehp),
        ];
    }
    ehp.entries
        .iter()
        .map(|entry| {
            let name = crate::skill_details::damage_type_name(&entry.damage_type);
            (
                trf("planner.common.named_ehp", &[("name", (name).to_string())]),
                entry.ehp,
            )
        })
        .collect()
}

pub struct StatsSidebar {
    session: Entity<Session>,
    performance: Option<Arc<PlannerPerformance>>,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}
impl StatsSidebar {
    pub fn new(
        session: Entity<Session>,
        tree: Entity<TreeView>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let performance = tree.read(cx).performance();
        let subscriptions = vec![
            cx.observe(&session, |_, _, cx| cx.notify()),
            cx.observe(&tree, |this, tree, cx| {
                let performance = tree.read(cx).performance();
                let unchanged = match (&this.performance, &performance) {
                    (Some(previous), Some(current)) => Arc::ptr_eq(previous, current),
                    (None, None) => true,
                    _ => false,
                };
                if !unchanged {
                    this.performance = performance;
                    cx.notify();
                }
            }),
        ];
        Self {
            session,
            performance,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }
    fn active_skills(
        &self,
        snapshot: &BuildSnapshot,
        result: Option<&PlannerPerformance>,
        cx: &Context<Self>,
    ) -> Div {
        let p = cx.global::<TooltipTheme>();
        let skills = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""));
        let active = skills
            .iter()
            .filter(|skill| skill.kind == SkillKind::Active)
            .collect::<Vec<_>>();
        let scale = &self.session.read(cx).state().settings.number_scale;
        let mut content = section_body();
        if active.is_empty() {
            return section(
                "sidebar-active-skills",
                tr("planner.common.active_skills"),
                content.child(
                    div()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(11.))
                        .text_color(p.muted)
                        .child(tr("planner.sidebar.no_skills")),
                ),
                cx,
            );
        }
        if snapshot.active_skill_ids.is_empty() {
            return section(
                "sidebar-active-skills",
                tr("planner.common.active_skills"),
                content.child(
                    div()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(11.))
                        .text_color(p.muted)
                        .child(tr("planner.sidebar.pick_skills")),
                ),
                cx,
            );
        }
        let mut buttons = div().flex().flex_col().gap_1().mb_2();
        for id in &snapshot.active_skill_ids {
            let name = active
                .iter()
                .find(|skill| &skill.id == id)
                .map(|skill| skill.name.as_str())
                .unwrap_or(id);
            let dps = result
                .and_then(|result| result.per_skill.iter().find(|skill| &skill.skill_id == id))
                .and_then(|skill| {
                    skill
                        .performance
                        .hit_dps_min
                        .zip(skill.performance.hit_dps_max)
                })
                .map(|range| compact_range(range, scale))
                .unwrap_or_else(|| "—".into());
            let remove = id.clone();
            let label = trf(
                "planner.sidebar.remove_active",
                &[("name", name.to_string())],
            );
            buttons = buttons.child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl_2()
                    .rounded(units(3.))
                    .bg(p.panel_secondary)
                    .text_size(units(11.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(p.text)
                            .child(name.to_owned()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_color(p.text)
                            .child(dps),
                    )
                    .child(
                        hsplanner_ui::controls::icon_button(
                            SharedString::from(format!("sidebar-active-remove-{id}")),
                            "×",
                            false,
                            cx,
                        )
                        .accessibility_label(label.clone())
                        .cursor_tooltip(label)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.session.update(cx, |session, cx| {
                                session.edit(|draft| {
                                    draft.snapshot.active_skill_ids.retain(|id| id != &remove)
                                });
                                cx.notify();
                            });
                        })),
                    ),
            );
        }
        content = content.child(buttons);
        let primary = snapshot
            .active_skill_ids
            .first()
            .and_then(|id| active.iter().find(|skill| &skill.id == id));
        let Some(skill) = primary else {
            return section(
                "sidebar-active-skills",
                tr("planner.common.active_skills"),
                content,
                cx,
            );
        };
        let rank = snapshot.skill_ranks.get(&skill.id).copied().unwrap_or(0);
        let performance = result.map(|result| &result.current);
        let cost = performance.and_then(|value| value.skill_costs.get(&skill.id));
        let rank_label = cost
            .map(|cost| precise((cost.eff_rank_min, cost.eff_rank_max)))
            .unwrap_or_else(|| rank.to_string())
            .replace('–', "-");
        let bonus = performance
            .and_then(|value| value.rank_bonuses.get(&skill.name.trim().to_lowercase()))
            .copied()
            .unwrap_or_default();
        let rank_note = (bonus != (0., 0.)).then(|| {
            (
                trf(
                    "planner.sidebar.rank_note",
                    &[
                        ("base", rank.to_string()),
                        ("bonus", signed_stat(bonus, false)),
                    ],
                ),
                p.faint,
            )
        });
        content = content.child(row_with_note(
            tr("planner.skills.rank"),
            format!("{rank_label}/{}", skill.max_rank),
            rank_note,
            p.muted,
            p.text,
        ));
        if let Some(cost) = cost {
            content = content.child(row(
                tr("planner.sidebar.mana_cast"),
                cost.mana_min
                    .zip(cost.mana_max)
                    .map(precise)
                    .unwrap_or_else(|| "—".into()),
                p.muted,
                theme::mana_color(),
            ));
            if let Some(life) = cost
                .life_min
                .zip(cost.life_max)
                .filter(|value| value.1 > 0.)
            {
                content = content.child(row(
                    tr("planner.sidebar.life_cast"),
                    precise(life),
                    p.muted,
                    p.negative,
                ));
            }
            if let Some(rate) = cost.entity_rate {
                content = content.child(row(
                    tr("planner.common.attack_rate"),
                    format!("{}/s", precise((rate.min, rate.max))),
                    p.muted,
                    p.text,
                ));
            }
            let rate_label = if cost.entity_rate.is_some() {
                tr("planner.sidebar.spawn_rate")
            } else if skill.uses_attack_speed {
                tr("planner.common.attack_rate")
            } else {
                tr("planner.common.cast_rate")
            };
            content = content
                .child(row(
                    rate_label,
                    cost.cast_rate_min
                        .zip(cost.cast_rate_max)
                        .map(|value| format!("{}/s", precise(value)))
                        .unwrap_or_else(|| "—".into()),
                    p.muted,
                    p.text,
                ))
                .child(row(
                    tr("planner.sidebar.mana_sec"),
                    cost.mana_per_sec_min
                        .zip(cost.mana_per_sec_max)
                        .map(precise)
                        .unwrap_or_else(|| "—".into()),
                    p.muted,
                    if cost.sustainable {
                        p.positive
                    } else if cost.unsustainable {
                        p.negative
                    } else {
                        theme::stat_color("orange", cx)
                    },
                ))
                .child(row(
                    tr("planner.sidebar.mana_regen"),
                    precise((cost.mana_regen_min, cost.mana_regen_max)),
                    p.muted,
                    theme::mana_color(),
                ));
            if let Some(net) = cost.net_min.zip(cost.net_max) {
                content = content.child(row(
                    tr("planner.sidebar.net_mana"),
                    format!("{}{}", if net.0 >= 0. { "+" } else { "" }, precise(net)),
                    p.muted,
                    if net.0 >= 0. {
                        p.positive
                    } else if net.1 < 0. {
                        p.negative
                    } else {
                        theme::stat_color("orange", cx)
                    },
                ));
            }
            if let Some(uptime) = cost.uptime_min.zip(cost.uptime_max) {
                content = content.child(row(
                    tr("planner.sidebar.uptime"),
                    format!("{}%", precise((uptime.0.round(), uptime.1.round()))),
                    p.muted,
                    if uptime.0 >= 100. {
                        p.positive
                    } else if uptime.1 < 75. {
                        p.negative
                    } else {
                        theme::stat_color("orange", cx)
                    },
                ));
            }
        }
        content = content.child(divider(cx));
        if let Some(value) = performance {
            let damage = value.hit_damage();
            content = content.child(row(
                tr("planner.common.hit_damage"),
                damage
                    .map(|value| compact_range(value, scale))
                    .unwrap_or_else(|| "—".into()),
                p.muted,
                p.text,
            ));
            if let Some(count) = value.entity_count {
                content = content.child(row(
                    tr("planner.sidebar.entity_count"),
                    format!("×{}", precise(count)),
                    p.muted,
                    p.text,
                ));
            }
            for (label, range, hide_if_missing) in [
                (
                    tr("planner.sidebar.hit_dps"),
                    value.hit_dps_min.zip(value.hit_dps_max),
                    false,
                ),
                (
                    tr("planner.sidebar.ailment_dps"),
                    value.ailment_dps_min.zip(value.ailment_dps_max),
                    true,
                ),
            ] {
                if hide_if_missing && range.is_none() {
                    continue;
                }
                content = content.child(row(
                    label,
                    range
                        .map(|value| compact_range(value, scale))
                        .unwrap_or_else(|| "—".into()),
                    p.muted,
                    p.text,
                ));
            }
            content = content.child(
                row(
                    tr("planner.sidebar.combined_dps"),
                    value
                        .combined_dps_min
                        .zip(value.combined_dps_max)
                        .map(|value| compact_range(value, scale))
                        .unwrap_or_else(|| "—".into()),
                    p.text,
                    p.accent_hot,
                )
                .mt_1()
                .font_weight(FontWeight::SEMIBOLD),
            );
        }
        section(
            "sidebar-active-skills",
            tr("planner.common.active_skills"),
            content,
            cx,
        )
    }
    fn stat_line(&self, key: &str, result: Option<&PlannerPerformance>, cx: &App) -> Div {
        let p = cx.global::<TooltipTheme>();
        let value = result
            .map(|value| {
                value
                    .current
                    .stats_combined
                    .get(key)
                    .or_else(|| value.current.stats.get(key))
                    .copied()
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let zero = value.0.abs() < 0.00001 && value.1.abs() < 0.00001;
        let def = data::game_config().stats.iter().find(|def| def.key == key);
        let name =
            crate::stat_labels::stat_name(key, def.map(|def| def.name.as_str()).unwrap_or(key));
        let percent = def.is_some_and(|def| def.format.as_deref() == Some("percent"));
        let blue = matches!(key, "mana" | "mana_replenish");
        let color = if zero {
            p.faint
        } else if blue {
            theme::mana_color()
        } else {
            p.text
        };
        let note = result
            .and_then(|value| value.current.diminished_raw.get(key))
            .filter(|_| !zero)
            .map(|raw| (format!("({})", signed_stat(*raw, percent)), p.faint));
        row_with_note(
            &name,
            if zero {
                "—".into()
            } else {
                signed_stat(value, percent)
            },
            note,
            if blue { theme::mana_color() } else { p.muted },
            color,
        )
    }
}
impl Render for StatsSidebar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let class = snapshot.class_id.as_deref().and_then(data::get_class);
        let result = self.performance.clone();
        let performance = result.as_deref();
        let hero = snapshot.allocated_tree_nodes.len();
        let header = div()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(p.border)
            .bg(linear_gradient(
                180.,
                linear_color_stop(p.accent.opacity(0.05), 0.),
                linear_color_stop(p.accent.opacity(0.), 1.),
            ))
            .flex()
            .items_start()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .mb_1()
                            .flex()
                            .items_center()
                            .gap_2()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(units(10.))
                            .text_color(p.faint)
                            .child(div().text_color(p.accent_hot).child("◆"))
                            .child(TooltipText::new(
                                "sidebar-character",
                                tr("planner.sidebar.character"),
                                0.18,
                            )),
                    )
                    .child(
                        div()
                            .text_size(units(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.accent_hot)
                            .child(
                                TooltipText::new(
                                    "sidebar-class",
                                    class
                                        .map(|class| class.name.as_str())
                                        .unwrap_or(tr("planner.common.no_class")),
                                    0.02,
                                )
                                .glow(Some(p.accent_hot.opacity(0.18))),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(10.))
                    .line_height(relative(1.25))
                    .text_color(p.accent_hot)
                    .child(TooltipText::new(
                        "sidebar-level",
                        trf(
                            "planner.sidebar.level",
                            &[("0", (snapshot.level).to_string())],
                        ),
                        0.18,
                    ))
                    .child(TooltipText::new(
                        "sidebar-hero-level",
                        trf(
                            "planner.sidebar.hero_level",
                            &[("hero", (hero).to_string())],
                        ),
                        0.18,
                    )),
            );
        let points = section_body()
            .child(row(
                tr("planner.common.attr_used"),
                format!(
                    "{}/{}",
                    snapshot.allocated.values().sum::<u32>(),
                    snapshot
                        .level
                        .saturating_mul(data::game_config().attribute_points_per_level)
                ),
                p.muted,
                p.text,
            ))
            .child(row(
                tr("planner.common.skill_used"),
                format!(
                    "{}/{}",
                    snapshot.skill_ranks.values().sum::<u32>(),
                    snapshot
                        .level
                        .saturating_mul(data::game_config().skill_points_per_level)
                ),
                p.muted,
                p.text,
            ));
        let mut attributes = section_body();
        for key in ATTRIBUTES {
            if let Some(attribute) = data::game_config()
                .attributes
                .iter()
                .find(|attribute| &attribute.key == key)
            {
                let value = performance
                    .and_then(|value| value.current.attributes.get(*key))
                    .copied()
                    .unwrap_or_default();
                let color = if *key == "armor" {
                    p.text
                } else {
                    theme::stat_color(key, cx)
                };
                attributes = attributes.child(row(
                    &crate::stat_labels::attribute_name(&attribute.key, &attribute.name),
                    signed_stat(value, false),
                    color,
                    p.text,
                ));
            }
        }
        let offense = section_body().children(
            OFFENSE
                .iter()
                .map(|key| self.stat_line(key, performance, cx)),
        );
        let mut defense = section_body();
        for key in DEFENSE {
            defense = defense.child(self.stat_line(key, performance, cx));
        }
        if let Some(result) = performance {
            let rows = ehp_rows(&result.current.ehp);
            if !rows.is_empty() {
                defense = defense.child(divider(cx));
            }
            for (label, value) in rows {
                defense = defense.child(row(
                    &label,
                    value
                        .map(|value| compact(value, "none"))
                        .unwrap_or_else(|| "∞".into()),
                    p.muted,
                    p.text,
                ));
            }
        }
        let mut resistances = section_body();
        for (key, label, tone) in [
            ("fire_resistance", tr("planner.common.fire"), "red"),
            ("cold_resistance", tr("planner.common.cold"), "blue"),
            (
                "lightning_resistance",
                tr("planner.common.lightning"),
                "orange",
            ),
            ("poison_resistance", tr("planner.common.poison"), "green"),
            ("arcane_resistance", tr("planner.common.arcane"), "purple"),
        ] {
            let value = performance
                .and_then(|value| value.current.stats.get(key))
                .copied()
                .unwrap_or_default();
            let cap = performance.and_then(|value| effective_cap(key, &value.current.stats));
            let zero = value == (0., 0.);
            let shown = if zero {
                "—".into()
            } else if let Some(cap) = cap.filter(|cap| value.0 == value.1 && value.0 > *cap) {
                format!("{}% ({}%)", number(cap), number(value.0))
            } else {
                signed_stat(value, true)
            };
            let color = theme::stat_color(tone, cx);
            resistances = resistances.child(row(
                label,
                shown,
                color,
                if zero { p.faint } else { p.text },
            ));
        }
        div()
            .w_full()
            .h_full()
            .flex_shrink_0()
            .min_h_0()
            .border_r_1()
            .border_color(p.border)
            .bg(linear_gradient(
                180.,
                linear_color_stop(p.panel_secondary, 0.),
                linear_color_stop(p.background, 1.),
            ))
            .text_color(p.text)
            .child(
                div()
                    .size_full()
                    .id("planner-stat-sidebar-scroll")
                    // No reserved gutter: the scrollbar overlays the rows' trailing padding.
                    .child(header)
                    .child(self.active_skills(snapshot, performance, cx))
                    .child(section(
                        "sidebar-points",
                        tr("planner.common.points"),
                        points,
                        cx,
                    ))
                    .child(section(
                        "sidebar-attributes",
                        tr("planner.common.attributes"),
                        attributes,
                        cx,
                    ))
                    .child(section(
                        "sidebar-offense",
                        tr("planner.common.offense"),
                        offense,
                        cx,
                    ))
                    .child(section(
                        "sidebar-defense",
                        tr("planner.common.defense"),
                        defense,
                        cx,
                    ))
                    .child(section(
                        "sidebar-resistances",
                        tr("planner.common.resistances"),
                        resistances,
                        cx,
                    ))
                    .page_scroll(&self.scroll)
                    .overflow_y_scroll(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsplanner_engine::calc::defense::EhpEntry;
    #[::core::prelude::v1::test]
    fn sidebar_numbers_drop_decimals_as_values_grow() {
        assert_eq!(signed_stat((270.44, 350.36), true), "+270–350%");
        assert_eq!(signed_stat((61.1, 85.07), false), "+61.1–85.1");
        assert_eq!(signed_stat((3.53, 4.), false), "+3.53–4");
        assert_eq!(signed_stat((66., 66.), true), "+66%");
        assert_eq!(signed_stat((99.996, 100.), false), "+100");
        assert_eq!(signed_stat((-12., -4.), true), "-12–-4%");
    }

    #[::core::prelude::v1::test]
    fn equal_ehp_collapses_and_different_elements_expand() {
        let mut ehp = EhpResult {
            entries: ["physical", "fire", "cold", "lightning", "poison", "arcane"]
                .iter()
                .map(|kind| EhpEntry {
                    damage_type: (*kind).into(),
                    ehp: Some(50.),
                    multiplier: 1.,
                    layers: vec![],
                })
                .collect(),
            worst: None,
        };
        assert_eq!(ehp_rows(&ehp), vec![("eHP".into(), Some(50.))]);
        ehp.entries[0].ehp = Some(100.);
        assert_eq!(ehp_rows(&ehp).len(), 2);
        ehp.entries[1].ehp = Some(75.);
        assert_eq!(ehp_rows(&ehp).len(), 6);
    }
}
