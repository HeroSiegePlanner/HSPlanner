//! Presentation blocks for the Skills view detail panel, after the reference SkillDetailsPanel.
use gpui_kit::{prelude::*, *};
use hsplanner_build::BuildSnapshot;
use hsplanner_engine::calc::{
    commands::{SubskillAggregationInput, subskill_aggregation},
    data,
    passive::{
        ManaCostFormula, PassiveSkill, PassiveStats, SkillRank, mana_cost_at_rank,
        passive_stats_at_rank,
    },
    planner::PlannerPerformance,
    rank::normalize_skill_name,
    types::{
        AffixEffect, AppliedStateValue, AttackKindSpec, DamageFormulaSpec, SkillKind, SkillSpec,
    },
};
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::{
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};
use std::collections::HashSet;

pub(crate) fn units(value: f32) -> Rems {
    rems(value / 13.)
}

pub(crate) struct DetailsContext<'a> {
    pub skill: &'a SkillSpec,
    pub snapshot: &'a BuildSnapshot,
    pub performance: Option<&'a PlannerPerformance>,
    pub class_skills: &'a [SkillSpec],
    pub rank: u32,
    pub bonus: (f64, f64),
}

impl DetailsContext<'_> {
    fn allocated(&self) -> bool {
        self.rank > 0
    }
    fn effective(&self) -> (f64, f64) {
        if self.allocated() {
            (
                self.rank as f64 + self.bonus.0,
                self.rank as f64 + self.bonus.1,
            )
        } else {
            (self.rank as f64, self.rank as f64)
        }
    }
    fn stat(&self, key: &str) -> (f64, f64) {
        self.performance
            .and_then(|result| result.computed.stats.get(key))
            .copied()
            .unwrap_or((0., 0.))
    }
}

pub(crate) fn stat_name(key: &str) -> String {
    let stats = &data::game_config().stats;
    if let Some(def) = stats.iter().find(|s| s.key == key) {
        return crate::stat_labels::stat_name(key, &def.name);
    }
    if let Some(base) = key.strip_suffix("_more")
        && let Some(def) = stats.iter().find(|s| s.key == base)
    {
        return trf(
            "planner.skills.total_stat",
            &[("0", crate::stat_labels::stat_name(base, &def.name))],
        );
    }
    key.split('_').map(capitalize).collect::<Vec<_>>().join(" ")
}

fn is_percent(key: &str) -> bool {
    data::game_config()
        .stats
        .iter()
        .find(|s| s.key == key)
        .is_some_and(|s| s.format.as_deref() == Some("percent"))
}

fn round2(value: f64) -> String {
    let rounded = (value * 100.).round() / 100.;
    if rounded.fract() == 0. {
        format!("{rounded:.0}")
    } else {
        format!("{rounded}")
    }
}

pub(crate) fn format_value(value: f64, key: &str, signed: bool) -> String {
    let sign = if signed && value >= 0. { "+" } else { "" };
    let suffix = if is_percent(key) { "%" } else { "" };
    format!("{sign}{}{suffix}", round2(value))
}

pub(crate) fn format_stat_pair(key: &str, min: f64, max: f64) -> String {
    if min == max {
        return format_value(min, key, true);
    }
    let max = format_value(max, key, true);
    format!(
        "{}-{}",
        format_value(min, key, true),
        max.trim_start_matches(['+', '-'])
    )
}

fn format_pair(min: f64, max: f64) -> String {
    if min == max {
        round2(min)
    } else {
        format!("{}-{}", round2(min), round2(max))
    }
}

fn formula(f: DamageFormulaSpec, rank: f64) -> f64 {
    (f.base + f.per_level * rank).max(0.)
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

pub(crate) fn diamond(color: Hsla, size: f32, glow: bool) -> Div {
    div()
        .flex_shrink_0()
        .size(units(size))
        .rounded_sm()
        .bg(color)
        .when(glow, |v| {
            v.shadow(vec![BoxShadow {
                color: color.opacity(0.6),
                offset: point(px(0.), px(0.)),
                blur_radius: units(5.).to_pixels(px(13.)),
                spread_radius: px(0.),
                inset: false,
            }])
        })
}

pub(crate) fn caption(id: impl Into<ElementId>, text: impl Into<String>, color: Hsla) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(11.))
        .text_color(color)
        .child(TooltipText::new(id, text.into().to_uppercase(), 0.16))
}

pub(crate) fn detail_block(
    id: &'static str,
    title: &str,
    trailing: Option<AnyElement>,
    accent: Option<Hsla>,
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    let marker = accent.unwrap_or(p.accent_deep);
    let text = accent.unwrap_or(p.accent_hot.opacity(0.8));
    div()
        .rounded_sm()
        .border_1()
        .border_color(p.border_strong)
        .p_2p5()
        .bg(linear_gradient(
            180.,
            linear_color_stop(p.panel_secondary, 0.),
            linear_color_stop(p.background.opacity(0.7), 1.),
        ))
        .child(
            div()
                .mb_2()
                .pb_1p5()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .border_b_1()
                .border_color(marker.opacity(if accent.is_some() { 0.25 } else { 0.2 }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .child(diamond(marker, 4., false))
                        .child(caption(id, title, text)),
                )
                .children(trailing),
        )
}

fn row(label: impl Into<SharedString>, value: impl IntoElement, cx: &App) -> Div {
    div()
        .flex()
        .justify_between()
        .gap_2()
        .text_size(units(12.))
        .child(
            div()
                .text_color(cx.global::<TooltipTheme>().muted)
                .child(label.into()),
        )
        .child(value)
}

// Leader-dotted stat row: "Label ........ current → next".
fn eff_row(
    label: impl Into<SharedString>,
    current: String,
    next: Option<String>,
    suffix: Option<&'static str>,
    color: Hsla,
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    let next = next.filter(|next| *next != current);
    div()
        .flex()
        .items_baseline()
        .gap_2()
        .min_w_0()
        .text_size(units(12.))
        .child(
            div()
                .truncate()
                .text_color(p.text.opacity(0.8))
                .child(label.into()),
        )
        .child(
            div()
                .flex_1()
                .min_w_2()
                .self_end()
                .mb(units(3.))
                .border_b_1()
                .border_color(p.faint.opacity(0.4)),
        )
        .child(
            div()
                .flex_shrink_0()
                .flex()
                .items_baseline()
                .font_family(theme::MONO_FONT_FAMILY)
                .child(div().text_color(color).child(current))
                .children(suffix.map(|suffix| div().text_color(p.muted).child(suffix)))
                .when_some(next, |v, next| {
                    v.child(div().px_1p5().text_color(p.muted).child("→"))
                        .child(div().text_color(color.opacity(0.65)).child(next))
                }),
        )
}

pub(crate) fn empty_state(cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let section = |id, label, marker: Hsla| {
        div()
            .mb_2()
            .pb_1p5()
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(p.accent_deep.opacity(0.2))
            .child(diamond(marker, 4., marker == p.accent_hot))
            .child(caption(id, label, p.accent_hot.opacity(0.8)))
    };
    let key_chip = |key: &str| {
        div()
            .px_1p5()
            .py_0p5()
            .rounded_sm()
            .border_1()
            .border_color(p.border_strong)
            .bg(p.panel_secondary)
            .font_family(theme::MONO_FONT_FAMILY)
            .text_size(units(9.))
            .text_color(p.muted)
            .child(key.to_owned())
    };
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .child(section(
                    "details-heading",
                    tr("planner.skills.details"),
                    p.accent_hot,
                ))
                .child(
                    div()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(11.))
                        .line_height(relative(1.6))
                        .text_color(p.muted)
                        .child(tr("planner.skills.details_help")),
                ),
        )
        .child(
            div()
                .child(section(
                    "controls-heading",
                    tr("planner.skills.controls"),
                    p.accent_deep,
                ))
                .child(
                    div().flex().flex_col().gap_2().children(
                        [
                            (
                                "select",
                                tr("planner.skills.left_click"),
                                tr("planner.skills.select"),
                            ),
                            ("add", "+", tr("planner.skills.add_one")),
                            (
                                "remove",
                                tr("planner.skills.right_click"),
                                tr("planner.skills.remove_one"),
                            ),
                            ("five", "SHIFT", tr("planner.skills.five_at_time")),
                            ("all", "CTRL/CMD+SHIFT", tr("planner.skills.all_points")),
                            ("subtree", "⚙", tr("planner.skills.open_subtree")),
                        ]
                        .into_iter()
                        .map(|(id, key, label)| {
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(key_chip(key))
                                .child(caption(
                                    SharedString::from(format!("control-{id}")),
                                    label,
                                    p.muted,
                                ))
                        }),
                    ),
                ),
        )
        .child(
            div()
                .child(section(
                    "damage-heading",
                    tr("planner.skills.damage_types"),
                    p.accent_deep,
                ))
                .child(
                    div().flex().flex_wrap().gap_y_2().children(
                        [
                            "physical",
                            "lightning",
                            "cold",
                            "fire",
                            "poison",
                            "arcane",
                            "explosion",
                            "magic",
                        ]
                        .map(|kind| {
                            div()
                                .w(relative(0.5))
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .size(units(6.))
                                        .border_1()
                                        .border_color(theme::damage_color(kind)),
                                )
                                .child(caption(
                                    SharedString::from(format!("legend-{kind}")),
                                    damage_type_name(kind),
                                    p.muted,
                                ))
                        }),
                    ),
                ),
        )
}

pub(crate) fn damage_type_name(kind: &str) -> &str {
    match kind {
        "physical" => tr("planner.common.physical"),
        "fire" => tr("planner.common.fire"),
        "cold" => tr("planner.common.cold"),
        "lightning" => tr("planner.common.lightning"),
        "poison" => tr("planner.common.poison"),
        "arcane" => tr("planner.common.arcane"),
        "explosion" => tr("planner.common.explosion"),
        "magic" => tr("planner.common.magic"),
        _ => kind,
    }
}

fn tag_translation_key(tag: &str) -> Option<&'static str> {
    Some(match tag {
        "Physical" => "planner.common.physical",
        "Arcane" => "planner.common.arcane",
        "Cold" => "planner.common.cold",
        "Fire" => "planner.common.fire",
        "Poison" => "planner.common.poison",
        "Lightning" => "planner.common.lightning",
        "Explosion" => "planner.common.explosion",
        "Magic" => "planner.common.magic",
        "Active" => "planner.common.active",
        "Passive" => "planner.skills.tag_passive",
        "Aura" => "planner.common.aura",
        "Buff" => "planner.common.buff",
        "Area of Effect" => "planner.stat.area_of_effect",
        "Summon" => "planner.config.summon",
        "Sentry" => "planner.config.sentry",
        "Guardian" => "planner.config.guardian",
        "Cast" => "planner.skills.tag_cast",
        "Spell" => "planner.skills.tag_spell",
        "Projectile" => "planner.skills.tag_projectile",
        "Strike" => "planner.skills.tag_strike",
        "Attack" => "planner.skills.tag_attack",
        "Melee" => "planner.skills.tag_melee",
        "Ranged" => "planner.skills.tag_ranged",
        "Movement" => "planner.skills.tag_movement",
        "Orbital" => "planner.skills.tag_orbital",
        "Chain Lightning" => "planner.skills.tag_chain_lightning",
        "Chaining" => "planner.skills.tag_chaining",
        "Blink" => "planner.skills.tag_blink",
        "Shape Shift" => "planner.skills.tag_shape_shift",
        "Charge" => "planner.skills.tag_charge",
        "Shield" => "planner.skills.tag_shield",
        _ => return None,
    })
}

/// Translate only the displayed tag; identity, membership and rules keep the canonical value.
pub(crate) fn tag_name(tag: &str) -> &str {
    tag_translation_key(tag).map(tr).unwrap_or(tag)
}

fn kind_label(kind: SkillKind) -> &'static str {
    match kind {
        SkillKind::Active => tr("planner.skills.kind_active"),
        SkillKind::Passive => tr("planner.skills.kind_passive"),
        SkillKind::Aura => tr("planner.skills.kind_aura"),
        SkillKind::Buff => tr("planner.skills.kind_buff"),
    }
}

pub(crate) fn header(skill: &SkillSpec, icon: Div, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .flex()
        .items_center()
        .gap_2p5()
        .child(
            div()
                .flex_shrink_0()
                .p_1()
                .rounded_sm()
                .border_1()
                .border_color(p.border_strong)
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(p.background, 0.),
                    linear_color_stop(p.panel_secondary, 1.),
                ))
                .child(icon),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .truncate()
                        .text_size(units(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.accent_hot)
                        .child(
                            TooltipText::new("skill-detail-name", skill.name.clone(), 0.02)
                                .glow(Some(p.accent_hot.opacity(0.4))),
                        ),
                )
                .child(caption(
                    "skill-kind",
                    format!(
                        "{} · {}",
                        damage_type_name(skill.damage_type.as_deref().unwrap_or("—")),
                        kind_label(skill.kind)
                    ),
                    p.muted,
                )),
        )
}

pub(crate) fn rank_row(details: &DetailsContext, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let (min, max) = details.effective();
    let has_bonus = details.allocated() && details.bonus != (0., 0.);
    div()
        .flex()
        .items_center()
        .gap_2()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(12.))
        .child(caption(
            "skill-rank-label",
            tr("planner.skills.rank"),
            p.muted,
        ))
        .child(div().text_color(p.accent_hot).child(format_pair(min, max)))
        .child(
            div()
                .text_color(p.muted)
                .child(format!("/ {}", details.skill.max_rank)),
        )
        .when(has_bonus, |v| {
            let (a, b) = details.bonus;
            let bonus = if a == b {
                format!("{}{}", if a >= 0. { "+" } else { "" }, round2(a))
            } else {
                format!(" +{}-{}", round2(a), round2(b))
            };
            v.child(
                div()
                    .text_color(p.muted)
                    .child(format!("({}{bonus})", details.rank)),
            )
        })
}

pub(crate) struct TagView {
    pub tags: Vec<String>,
    pub added: HashSet<String>,
    pub removed: Vec<String>,
}

// Mirrors the frontend tag rewrite: allocated subskills add/remove tags, removes win.
pub(crate) fn effective_tags(skill: &SkillSpec, snapshot: &BuildSnapshot) -> TagView {
    let visible = |tags: &[String]| -> Vec<String> {
        tags.iter()
            .filter(|tag| {
                skill
                    .damage_type
                    .as_deref()
                    .is_none_or(|kind| !tag.eq_ignore_ascii_case(kind))
            })
            .cloned()
            .collect()
    };
    let base_tags = skill.tags.clone().unwrap_or_default();
    let base = visible(&base_tags);
    let mut merged = base_tags.clone();
    let mut removes = Vec::new();
    if let Some(changes) = data::data().subskill_tags.get(&skill.id) {
        for (node, change) in changes {
            let rank = snapshot
                .subskill_ranks
                .get(&format!("{}:{node}", skill.id))
                .copied()
                .unwrap_or(0);
            if rank == 0 {
                continue;
            }
            for tag in &change.add {
                if !merged.contains(tag) {
                    merged.push(tag.clone());
                }
            }
            removes.extend(change.remove.iter().cloned());
        }
    }
    merged.retain(|tag| !removes.contains(tag));
    let tags = visible(&merged);
    let added = tags.iter().filter(|t| !base.contains(t)).cloned().collect();
    let removed = base.iter().filter(|t| !tags.contains(t)).cloned().collect();
    TagView {
        tags,
        added,
        removed,
    }
}

pub(crate) fn tag_chips(view: &TagView, cx: &App) -> Option<Div> {
    if view.tags.is_empty() && view.removed.is_empty() {
        return None;
    }
    let p = cx.global::<TooltipTheme>();
    let chip = |text: &str| {
        div()
            .px_2()
            .py_0p5()
            .rounded_sm()
            .border_1()
            .font_family(theme::MONO_FONT_FAMILY)
            .text_size(units(10.))
            .child(TooltipText::new(
                SharedString::from(format!("tag-{text}")),
                tag_name(text).to_uppercase(),
                0.18,
            ))
    };
    Some(
        div()
            .flex()
            .flex_wrap()
            .gap_1p5()
            .children(view.tags.iter().map(|tag| {
                chip(tag)
                    .border_color(if view.added.contains(tag) {
                        p.accent_hot.opacity(0.8)
                    } else {
                        p.accent_deep.opacity(0.4)
                    })
                    .bg(theme::chrome_gold_surface())
                    .text_color(p.accent_hot.opacity(0.8))
            }))
            .children(view.removed.iter().map(|tag| {
                chip(tag)
                    .border_color(p.border)
                    .text_color(p.faint)
                    .line_through()
                    .opacity(0.6)
            })),
    )
}

pub(crate) fn bonuses_block(details: &DetailsContext, cx: &App) -> Option<Div> {
    let p = cx.global::<TooltipTheme>();
    let skill = details.skill;
    let has_bonus = details.allocated() && details.bonus != (0., 0.);
    let aura = details.stat("buffing_aura_effectiveness");
    let has_aura = skill.kind == SkillKind::Aura && aura != (0., 0.);
    if !has_bonus && !has_aura {
        return None;
    }
    let value = |min: f64, max: f64| {
        div()
            .font_family(theme::MONO_FONT_FAMILY)
            .text_color(p.accent_hot)
            .child(format!("+{}", format_pair(min, max)))
    };
    let mut rows = div().flex().flex_col().gap_1();
    if has_bonus {
        let all = details.stat("all_skills");
        if all != (0., 0.) {
            rows = rows.child(row(
                tr("planner.skills.all_skill_bonus"),
                value(all.0, all.1),
                cx,
            ));
        }
        if let Some(kind) = skill.damage_type.as_deref() {
            let element = details.stat(&format!("{kind}_skills"));
            if element != (0., 0.) {
                rows = rows.child(row(
                    trf(
                        "planner.skills.tag_bonus",
                        &[("0", damage_type_name(kind).to_owned())],
                    ),
                    value(element.0, element.1),
                    cx,
                ));
            }
        }
        let tags = skill.tags.clone().unwrap_or_default();
        for (key, def) in &data::data().affix_tags {
            if !matches!(def.effect, AffixEffect::Rank)
                || !def.tags.iter().all(|t| tags.contains(t))
            {
                continue;
            }
            let bonus = details.stat(key);
            if bonus != (0., 0.) {
                rows = rows.child(row(
                    trf(
                        "planner.skills.tag_bonus",
                        &[(
                            "0",
                            def.tags
                                .iter()
                                .map(|tag| tag_name(tag))
                                .collect::<Vec<_>>()
                                .join(" + "),
                        )],
                    ),
                    value(bonus.0, bonus.1),
                    cx,
                ));
            }
        }
        let item = details
            .performance
            .and_then(|r| {
                r.computed
                    .item_skill_bonuses
                    .get(&normalize_skill_name(&skill.name))
            })
            .copied()
            .unwrap_or((0., 0.));
        if item != (0., 0.) {
            rows = rows.child(row(
                trf(
                    "planner.skills.named_bonus",
                    &[("skill", skill.name.clone())],
                ),
                value(item.0, item.1),
                cx,
            ));
        }
    }
    if has_aura {
        rows = rows.child(row(
            tr("planner.skills.aura_effectiveness"),
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_color(p.accent_hot)
                .child(format_stat_pair(
                    "buffing_aura_effectiveness",
                    aura.0,
                    aura.1,
                )),
            cx,
        ));
    }
    Some(
        detail_block(
            "skill-bonuses",
            tr("planner.skills.bonuses"),
            None,
            None,
            cx,
        )
        .child(rows),
    )
}

fn passive_skill(skill: &SkillSpec) -> PassiveSkill {
    PassiveSkill {
        passive_stats: skill.passive_stats.as_ref().map(|stats| PassiveStats {
            base: stats.base.clone().unwrap_or_default(),
            per_rank: stats.per_rank.clone().unwrap_or_default(),
        }),
        mana_cost_formula: skill.mana_cost_formula.map(|f| ManaCostFormula {
            base: f.base,
            per_level: f.per_level,
        }),
        ranks: skill
            .ranks
            .iter()
            .map(|rank| SkillRank {
                rank: rank.rank,
                mana_cost: rank.mana_cost,
            })
            .collect(),
    }
}

fn base_damage(skill: &SkillSpec, rank: f64) -> Option<(f64, f64)> {
    if let Some(f) = skill.damage_formula {
        let value = formula(f, rank);
        return Some((value, value));
    }
    let table = skill.damage_per_rank.as_ref()?;
    let n = table.len();
    if n == 0 || rank < 1. {
        return None;
    }
    let rank = rank.round() as usize;
    if rank <= n {
        let d = &table[rank - 1];
        return Some((d.min.max(0.), d.max.max(0.)));
    }
    let last = &table[n - 1];
    let prev = if n >= 2 { &table[n - 2] } else { last };
    let over = (rank - n) as f64;
    Some((
        (last.min + (last.min - prev.min) * over).max(0.),
        (last.max + (last.max - prev.max) * over).max(0.),
    ))
}

fn damage_range_label(min: (f64, f64), max: (f64, f64)) -> String {
    if min == max {
        format_pair(min.0, min.1)
    } else {
        format!(
            "{} … {}",
            format_pair(min.0, min.1),
            format_pair(max.0, max.1)
        )
    }
}

pub(crate) fn stats_block(details: &DetailsContext, cx: &App) -> Option<Div> {
    let p = cx.global::<TooltipTheme>();
    let skill = details.skill;
    let allocated = details.allocated();
    let aura = details.stat("buffing_aura_effectiveness");
    let boost = if skill.kind == SkillKind::Aura && aura != (0., 0.) {
        (1. + aura.0 / 100., 1. + aura.1 / 100.)
    } else {
        (1., 1.)
    };
    let (cur_min, cur_max) = if allocated {
        details.effective()
    } else {
        (1., 1.)
    };
    let next = (allocated && details.rank < skill.max_rank).then_some((cur_min + 1., cur_max + 1.));
    let passive = passive_skill(skill);
    let at = |rank: f64| passive_stats_at_rank(&passive, rank.round() as u32);
    let (passive_min, passive_max) = (at(cur_min), at(cur_max));
    let passive_next = next.map(|(a, b)| (at(a), at(b)));
    let mana = |rank: f64| mana_cost_at_rank(&passive, rank.round() as u32);
    let base_min = base_damage(skill, cur_min);
    let base_max = base_damage(skill, cur_max);
    let has_properties = skill.base_cast_rate.is_some()
        || skill.movement_during_use.is_some()
        || skill.range.is_some()
        || skill.base_cooldown.is_some()
        || skill.effect_duration.is_some()
        || skill.hit_model.is_some()
        || skill.requires_level.is_some()
        || skill.requires_skill.is_some();
    let mut keys: Vec<String> = passive_min
        .keys()
        .chain(passive_max.keys())
        .cloned()
        .collect();
    keys.sort_by_key(|key| stat_name(key));
    keys.dedup();
    if base_min.is_none()
        && mana(cur_min).is_none()
        && keys.is_empty()
        && !has_properties
        && skill.attack_scaling.is_none()
    {
        return None;
    }
    let value_color = if allocated { p.accent_hot } else { p.muted };
    let trailing = div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(11.))
        .text_color(p.muted)
        .flex()
        .gap_1()
        .child(trf(
            "planner.skills.rank_lower",
            &[("0", (format_pair(cur_min, cur_max)).to_string())],
        ))
        .children(next.map(|(a, b)| {
            div()
                .flex()
                .gap_1()
                .child(div().text_color(p.accent_deep).child("→"))
                .child(format_pair(a, b))
        }));
    let mut rows = div().flex().flex_col().gap_1();
    if let (Some(min), Some(max)) = (base_min, base_max) {
        let label = match (skill.attack_kind, skill.damage_type.as_deref()) {
            (Some(AttackKindSpec::Attack), Some(kind)) => trf(
                "planner.common.damage_value",
                &[("0", damage_type_name(kind).to_owned())],
            ),
            _ => tr("planner.common.base_damage").into(),
        };
        let next_label = next.and_then(|(a, b)| {
            Some(damage_range_label(
                base_damage(skill, a)?,
                base_damage(skill, b)?,
            ))
        });
        rows = rows.child(eff_row(
            label,
            damage_range_label(min, max),
            next_label,
            None,
            value_color,
            cx,
        ));
    }
    if let Some(scaling) = skill.attack_scaling {
        for (label, f) in [
            (
                tr("planner.common.attack_damage"),
                scaling.weapon_damage_pct,
            ),
            (
                tr("planner.common.attack_rating"),
                scaling.attack_rating_pct,
            ),
        ] {
            let Some(f) = f else { continue };
            let pct = |a: f64, b: f64| {
                let (a, b) = (formula(f, a), formula(f, b));
                if round2(a) == round2(b) {
                    format!("{}%", round2(a))
                } else {
                    format!("{}% - {}%", round2(a), round2(b))
                }
            };
            rows = rows.child(eff_row(
                label,
                pct(cur_min, cur_max),
                next.map(|(a, b)| pct(a, b)),
                None,
                value_color,
                cx,
            ));
        }
    }
    if let (Some(a), Some(b)) = (mana(cur_min), mana(cur_max)) {
        rows = rows.child(eff_row(
            tr("planner.skills.mana_cost"),
            format_pair(a, b),
            next.and_then(|(x, y)| Some(format_pair(mana(x)?, mana(y)?))),
            None,
            if allocated { p.stat_blue } else { p.muted },
            cx,
        ));
    }
    for key in &keys {
        let min = passive_min.get(key).copied().unwrap_or(0.) * boost.0;
        let max = passive_max.get(key).copied().unwrap_or(0.) * boost.1;
        let upcoming = passive_next.as_ref().and_then(|(a, b)| {
            Some(format_stat_pair(
                key,
                *a.get(key)? * boost.0,
                *b.get(key)? * boost.1,
            ))
        });
        rows = rows.child(eff_row(
            stat_name(key),
            format_stat_pair(key, min, max),
            upcoming,
            None,
            value_color,
            cx,
        ));
    }
    let plain = [
        (
            tr("planner.skills.base_cast_rate"),
            skill.base_cast_rate,
            Some("/s"),
        ),
        (
            tr("planner.skills.movement_during_use"),
            skill.movement_during_use,
            Some("%"),
        ),
        (tr("planner.skills.range"), skill.range, None),
        (
            tr("planner.skills.cooldown"),
            skill.base_cooldown,
            Some("s"),
        ),
        (
            tr("planner.skills.effect_duration"),
            skill.effect_duration,
            Some("s"),
        ),
        (
            tr("planner.skills.hit_interval"),
            skill
                .hit_model
                .as_ref()
                .and_then(|model| model.tick_frequency),
            Some("s"),
        ),
        (
            tr("planner.skills.requires_level"),
            skill.requires_level.map(f64::from),
            None,
        ),
    ];
    for (label, value, suffix) in plain {
        if let Some(value) = value {
            rows = rows.child(eff_row(label, round2(value), None, suffix, p.text, cx));
        }
    }
    if let Some(required) = &skill.requires_skill {
        rows = rows.child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap_2()
                .text_size(units(12.))
                .child(
                    div()
                        .text_color(p.text.opacity(0.8))
                        .child(tr("planner.skills.requires_skill")),
                )
                .child(
                    div()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_color(p.muted)
                        .child(format!("«{required}»")),
                ),
        );
    }
    Some(
        detail_block(
            "skill-stats",
            if allocated {
                tr("planner.common.stats")
            } else {
                tr("planner.skills.unlearned_preview")
            },
            Some(trailing.into_any_element()),
            None,
            cx,
        )
        .child(rows),
    )
}

fn synergy_row(
    marker: Hsla,
    glow: bool,
    name: &str,
    value: String,
    value_color: Hsla,
    note: String,
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .py_1()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap_2()
                .text_size(units(12.))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap_1p5()
                        .min_w_0()
                        .child(diamond(marker, 5., glow))
                        .child(
                            div()
                                .truncate()
                                .text_color(p.text.opacity(0.85))
                                .child(name.to_owned()),
                        ),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_color(value_color)
                        .child(value),
                ),
        )
        .child(
            div()
                .pl_3()
                .text_size(units(10.))
                .text_color(p.faint)
                .child(note),
        )
}

pub(crate) fn synergy_blocks(details: &DetailsContext, cx: &App) -> Vec<Div> {
    let p = cx.global::<TooltipTheme>();
    let skill = details.skill;
    let allocated = details.allocated();
    let (cur_min, cur_max) = if allocated {
        details.effective()
    } else {
        (1., 1.)
    };
    let me = normalize_skill_name(&skill.name);
    let mut blocks = Vec::new();
    let provided: Vec<_> = details
        .class_skills
        .iter()
        .flat_map(|other| {
            other
                .bonus_sources
                .iter()
                .flatten()
                .filter(|bs| bs.per == "skill_level" && normalize_skill_name(&bs.source) == me)
                .map(move |bs| (other, bs))
        })
        .collect();
    if !provided.is_empty() {
        let mut list = div().flex().flex_col();
        for (other, bs) in provided {
            list = list.child(synergy_row(
                p.stat_orange,
                true,
                &other.name,
                format_stat_pair(&bs.stat, bs.value * cur_min, bs.value * cur_max),
                if allocated { p.stat_orange } else { p.muted },
                trf(
                    "planner.skills.percent_rank",
                    &[("0", (round2(bs.value)).to_string())],
                ),
                cx,
            ));
        }
        blocks.push(
            detail_block(
                "provides-synergy",
                tr("planner.skills.provides_synergy"),
                None,
                Some(p.stat_orange),
                cx,
            )
            .child(list),
        );
    }
    if let Some(sources) = skill.bonus_sources.as_ref().filter(|s| !s.is_empty()) {
        let mut list = div().flex().flex_col();
        for bs in sources {
            let source_skill = (bs.per == "skill_level")
                .then(|| {
                    details
                        .class_skills
                        .iter()
                        .find(|s| normalize_skill_name(&s.name) == normalize_skill_name(&bs.source))
                })
                .flatten();
            let matched = if !allocated {
                None
            } else if bs.per == "skill_level" {
                source_skill.and_then(|source| {
                    let base = details
                        .snapshot
                        .skill_ranks
                        .get(&source.id)
                        .copied()
                        .unwrap_or(0);
                    if base == 0 {
                        return None;
                    }
                    let bonus = details
                        .performance
                        .and_then(|r| {
                            r.computed
                                .rank_bonuses
                                .get(&normalize_skill_name(&source.name))
                        })
                        .copied()
                        .unwrap_or((0., 0.));
                    Some((
                        (base as f64 + bonus.0) * bs.value,
                        (base as f64 + bonus.1) * bs.value,
                    ))
                })
            } else if bs.per == "attribute_point" {
                details
                    .performance
                    .and_then(|r| {
                        r.computed
                            .attributes
                            .iter()
                            .find(|(key, _)| key.eq_ignore_ascii_case(bs.source.trim()))
                    })
                    .map(|(_, value)| *value)
                    .filter(|value| *value != (0., 0.))
                    .map(|(a, b)| (a * bs.value, b * bs.value))
            } else {
                None
            };

            list = list.child(synergy_row(
                if source_skill.is_some() {
                    p.synergy
                } else {
                    p.faint
                },
                source_skill.is_some(),
                &bs.source,
                matched
                    .map(|(a, b)| format_stat_pair(&bs.stat, a, b))
                    .unwrap_or_else(|| "—".into()),
                if matched.is_some() {
                    p.stat_orange
                } else {
                    p.faint
                },
                trf(
                    if bs.per == "skill_level" {
                        "planner.skills.synergy_rank"
                    } else {
                        "planner.skills.synergy_point"
                    },
                    &[
                        ("value", format_value(bs.value, &bs.stat, true)),
                        ("stat", stat_name(&bs.stat)),
                    ],
                ),
                cx,
            ));
        }
        blocks.push(
            detail_block(
                "receives-synergy",
                tr("planner.skills.receives_synergy"),
                None,
                Some(p.synergy),
                cx,
            )
            .child(list),
        );
    }
    blocks
}

pub(crate) fn subtree_block(details: &DetailsContext, cx: &App) -> Option<Div> {
    let p = cx.global::<TooltipTheme>();
    let skill = details.skill;
    let aggregation = subskill_aggregation(SubskillAggregationInput {
        class_id: skill.class_id.clone(),
        skill_id: skill.id.clone(),
        subskill_ranks: details.snapshot.subskill_ranks.clone(),
        enemy_conditions: details.snapshot.enemy_conditions.clone(),
        season: None,
    });
    let mut stats: Vec<(&String, &f64)> = aggregation
        .stats
        .iter()
        .filter(|(_, v)| **v != 0.)
        .collect();
    stats.sort_by(|a, b| a.0.cmp(b.0));
    let procs: Vec<_> = skill
        .subskills
        .iter()
        .flatten()
        .filter_map(|node| {
            let proc = node.proc.as_ref()?;
            let rank = details
                .snapshot
                .subskill_ranks
                .get(&format!("{}:{}", skill.id, node.id))
                .copied()
                .unwrap_or(0);
            (rank > 0).then_some((node, proc, rank))
        })
        .collect();
    if stats.is_empty() && procs.is_empty() {
        return None;
    }
    let mut block = detail_block(
        "subtree-bonuses",
        tr("planner.skills.subtree_bonuses"),
        None,
        None,
        cx,
    );
    if !stats.is_empty() {
        block = block.child(div().flex().flex_col().gap_1().children(stats.iter().map(
            |(key, value)| {
                row(
                    stat_name(key),
                    div()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_color(p.accent_hot)
                        .child(format_value(**value, key, true)),
                    cx,
                )
            },
        )));
    }
    if !procs.is_empty() {
        let mut list = div()
            .flex()
            .flex_col()
            .gap_1p5()
            .when(!stats.is_empty(), |v| {
                v.mt_2p5()
                    .pt_2()
                    .border_t_1()
                    .border_color(p.accent_deep.opacity(0.3))
            });
        for (node, proc, rank) in procs {
            let chance =
                proc.chance.base.unwrap_or(0.) + proc.chance.per_rank.unwrap_or(0.) * rank as f64;
            let base = proc
                .effects
                .as_ref()
                .and_then(|e| e.base.clone())
                .unwrap_or_default();
            let per_rank = proc
                .effects
                .as_ref()
                .and_then(|e| e.per_rank.clone())
                .unwrap_or_default();
            let mut keys: Vec<&String> = base.keys().chain(per_rank.keys()).collect();
            keys.sort();
            keys.dedup();
            let mut parts: Vec<String> = keys
                .into_iter()
                .filter_map(|key| {
                    let value = base.get(key).copied().unwrap_or(0.)
                        + per_rank.get(key).copied().unwrap_or(0.) * rank as f64;
                    (value != 0.)
                        .then(|| format!("{} {}", format_value(value, key, true), stat_name(key)))
                })
                .collect();
            for state in proc.applies_states.iter().flatten() {
                parts.push(match state {
                    AppliedStateValue::Name(name) => trf(
                        "planner.skills.applies",
                        &[("0", (name.replace('_', " ")).to_string())],
                    ),
                    AppliedStateValue::Full { state, amount } => {
                        let amount = amount
                            .map(|a| a.base.unwrap_or(0.) + a.per_rank.unwrap_or(0.) * rank as f64)
                            .unwrap_or(0.);
                        if amount != 0. {
                            trf(
                                "planner.skills.applies_chance",
                                &[
                                    ("0", (state.replace('_', " ")).to_string()),
                                    ("1", (round2(amount)).to_string()),
                                ],
                            )
                        } else {
                            trf(
                                "planner.skills.applies",
                                &[("0", (state.replace('_', " ")).to_string())],
                            )
                        }
                    }
                });
            }
            list = list.child(
                div()
                    .text_size(units(12.))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .justify_between()
                            .gap_2()
                            .child(div().text_color(p.text).child(node.name.clone()))
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap_1()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .child(
                                        div()
                                            .text_color(p.accent_hot)
                                            .child(format!("{}%", round2(chance))),
                                    )
                                    .child(caption(
                                        SharedString::from(format!("proc-trigger-{}", node.id)),
                                        proc.trigger.replace('_', " "),
                                        p.muted,
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .text_color(p.muted)
                            .line_height(relative(1.35))
                            .child(parts.join(", ")),
                    ),
            );
        }
        block = block.child(list);
    }
    Some(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsplanner_engine::calc::types::DamageRangeSpec;

    #[::core::prelude::v1::test]
    fn canonical_skill_tags_have_localized_presentation() {
        use hsplanner_ui::i18n::{Language, text_for};
        let data = data::data();
        let mut tags = HashSet::new();
        for class in data.classes.values() {
            for skill in data::get_skills_by_class(&class.id) {
                tags.extend(skill.tags.iter().flatten().cloned());
            }
        }
        for changes in data.subskill_tags.values() {
            for change in changes.values() {
                tags.extend(change.add.iter().cloned());
                tags.extend(change.remove.iter().cloned());
            }
        }
        for tag in tags {
            let key =
                tag_translation_key(&tag).unwrap_or_else(|| panic!("Untranslated tag: {tag}"));
            assert_eq!(text_for(Language::English, key), tag);
            for language in [Language::Korean, Language::Russian, Language::Chinese] {
                let name = text_for(language, key);
                assert_ne!(name, tag);
                assert_ne!(name, "⟦missing translation⟧");
            }
        }
        assert_eq!(tag_name("Future Tag"), "Future Tag");
    }

    #[::core::prelude::v1::test]
    fn stat_pairs_format_like_the_reference() {
        assert_eq!(format_pair(3., 3.), "3");
        assert_eq!(format_pair(2.5, 4.), "2.5-4");
        assert_eq!(format_value(12., "no_such_stat_key", true), "+12");
        assert_eq!(format_stat_pair("no_such_stat_key", 5., 8.), "+5-8");
        assert_eq!(stat_name("some_unknown_key"), "Some Unknown Key");
    }

    #[::core::prelude::v1::test]
    fn damage_table_extrapolates_past_the_last_rank() {
        let skill = SkillSpec {
            damage_per_rank: Some(vec![
                DamageRangeSpec { min: 10., max: 20. },
                DamageRangeSpec { min: 15., max: 30. },
            ]),
            ..Default::default()
        };
        assert_eq!(base_damage(&skill, 1.), Some((10., 20.)));
        assert_eq!(base_damage(&skill, 4.), Some((25., 50.)));
        assert_eq!(base_damage(&skill, 0.), None);
    }
}
