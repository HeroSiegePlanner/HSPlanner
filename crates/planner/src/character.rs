//! Read-only character overview, composed from the shared planner result.
use crate::TreeView;
use gpui_kit::{prelude::*, *};
use hsplanner_build::{BuildSnapshot, session::Session};
use hsplanner_engine::calc::{
    build::BuildPerformance, data, defense, planner::PlannerPerformance, types::SkillSpec,
};
use hsplanner_ui::scroll::PageScroll;
use hsplanner_ui::{
    assets::class_portrait,
    components::{corner_marks, panel_with_heading_style, section_heading},
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};
use std::sync::Arc;

fn panel(id: impl Into<ElementId>, title: impl Into<SharedString>, cx: &App) -> Div {
    panel_with_heading_style(id, title, None, FontWeight::NORMAL, cx)
}

fn panel_with_trailing(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    trailing: impl IntoElement,
    cx: &App,
) -> Div {
    panel_with_heading_style(
        id,
        title,
        Some(trailing.into_any_element()),
        FontWeight::NORMAL,
        cx,
    )
}

pub struct CharacterView {
    session: Entity<Session>,
    tree: Entity<TreeView>,
    observed_performance: Option<Arc<PlannerPerformance>>,
    focus: FocusHandle,
    scroll: ScrollHandle,
    has_vertical_scroll: bool,
    _subscriptions: Vec<Subscription>,
}

impl CharacterView {
    pub fn new(
        session: Entity<Session>,
        tree: Entity<TreeView>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let observed_performance = tree.read(cx).performance();
        let subscriptions = vec![
            cx.observe(&session, |_, _, cx| cx.notify()),
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
        ];
        Self {
            session,
            tree,
            observed_performance,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            has_vertical_scroll: false,
            _subscriptions: subscriptions,
        }
    }

    fn identity(&self, snapshot: &BuildSnapshot, cx: &App) -> Div {
        let p = cx.global::<TooltipTheme>();
        let session = self.session.read(cx);
        let name = session
            .draft()
            .build_id
            .as_ref()
            .and_then(|id| {
                session
                    .state()
                    .library
                    .builds
                    .iter()
                    .find(|build| &build.id == id)
            })
            .map(|build| build.name.as_str())
            .unwrap_or("Unsaved build");
        let class = snapshot.class_id.as_deref().and_then(data::get_class);
        let attr_spent: u32 = snapshot.allocated.values().sum();
        let skill_spent: u32 = snapshot.skill_ranks.values().sum();
        let portrait = div()
            .relative()
            .size_16()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .border_1()
            .border_color(p.accent_deep.opacity(0.5))
            .bg(p.background)
            .when_some(
                class_portrait(snapshot.class_id.as_deref()),
                |view, image| view.child(img(image).size_11().object_fit(ObjectFit::Contain)),
            )
            .child(
                div()
                    .absolute()
                    .bottom_0p5()
                    .right_1()
                    .text_size(rems(10. / 13.))
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_color(p.accent_hot)
                    .child(snapshot.level.to_string()),
            );
        surface(cx)
            .relative()
            .overflow_hidden()
            .child(corner_marks(cx))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_4()
            .child(
                div().flex().items_center().gap_4().child(portrait).child(
                    div()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(rems(22. / 13.))
                                .line_height(relative(1.25))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.accent_hot)
                                .child(name.to_owned()),
                        )
                        .child(div().mt_1().child(caption_tracked(
                            "character-identity",
                            format!(
                                "{} · Lv {} · Hero Lv {}",
                                class.map(|c| c.name.as_str()).unwrap_or("No class"),
                                snapshot.level,
                                snapshot.allocated_tree_nodes.len()
                            ),
                            p.muted,
                            11.,
                            0.16,
                        ))),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(point_stat(
                        "attr-used",
                        "Attr used",
                        attr_spent,
                        Some(snapshot.level * data::game_config().attribute_points_per_level),
                        cx,
                    ))
                    .child(point_stat(
                        "skill-used",
                        "Skill used",
                        skill_spent,
                        Some(snapshot.level * data::game_config().skill_points_per_level),
                        cx,
                    ))
                    .child(point_stat(
                        "tree-used",
                        "Tree nodes",
                        snapshot.allocated_tree_nodes.len() as u32,
                        None,
                        cx,
                    )),
            )
    }

    fn attributes(
        &self,
        snapshot: &BuildSnapshot,
        result: Option<&BuildPerformance>,
        columns: u16,
        cx: &App,
    ) -> Div {
        let p = cx.global::<TooltipTheme>();
        let class = snapshot.class_id.as_deref().and_then(data::get_class);
        let mut cards = div().grid().grid_cols(columns).gap_3();
        for key in [
            "strength",
            "dexterity",
            "intelligence",
            "energy",
            "vitality",
            "armor",
        ] {
            let Some(definition) = data::game_config()
                .attributes
                .iter()
                .find(|attr| attr.key == key)
            else {
                continue;
            };
            let value = result.and_then(|r| r.attributes.get(key)).copied();
            let base = data::game_config()
                .default_base_attributes
                .as_ref()
                .and_then(|attrs| attrs.get(key))
                .copied()
                .unwrap_or(0.)
                + class
                    .and_then(|c| c.base_attributes.get(key))
                    .copied()
                    .unwrap_or(0.)
                + snapshot.allocated.get(key).copied().unwrap_or(0) as f64;
            let delta = value.map(|v| (v.1 - base).round()).unwrap_or(0.);
            let color = if key == "armor" {
                p.muted
            } else {
                theme::stat_color(key, cx)
            };
            cards = cards.child(
                surface(cx)
                    .relative()
                    .px_3()
                    .py_2p5()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(
                                // Match the reference's rotated h-1 w-1 square;
                                // a font glyph paints much smaller at this scale.
                                canvas(
                                    |_, _, _| (),
                                    move |bounds, _, window, _| {
                                        let center = bounds.center();
                                        let radius = bounds.size.width / std::f32::consts::SQRT_2;
                                        let mut path = PathBuilder::fill();
                                        path.move_to(point(center.x, center.y - radius));
                                        path.line_to(point(center.x + radius, center.y));
                                        path.line_to(point(center.x, center.y + radius));
                                        path.line_to(point(center.x - radius, center.y));
                                        path.close();
                                        if let Ok(path) = path.build() {
                                            window.paint_path(path, color);
                                        }
                                    },
                                )
                                .size_1()
                                .flex_shrink_0(),
                            )
                            .child(caption_tracked(
                                key,
                                definition.name.clone(),
                                p.faint,
                                10.,
                                0.18,
                            )),
                    )
                    .child(
                        div()
                            .mt_1()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_size(rems(26. / 13.))
                            .text_color(color)
                            .flex()
                            .flex_wrap()
                            // Keep punctuation with its number; a range may wrap only
                            // after its separator, as in the browser reference.
                            .children(
                                value
                                    .map(|value| formatted_stat_parts(value, key))
                                    .unwrap_or_else(|| vec!["—".into()])
                                    .into_iter()
                                    .map(|part| {
                                        div().flex_shrink_0().whitespace_nowrap().child(part)
                                    }),
                            ),
                    )
                    .child(caption(
                        format!("{key}-delta"),
                        if delta > 0. {
                            format!("+{delta:.0} added")
                        } else {
                            "base".into()
                        },
                        p.faint,
                        10.,
                    ))
                    .child(
                        div()
                            .absolute()
                            .bottom_0()
                            .left_0()
                            .right_0()
                            .h(rems(2. / 13.))
                            .flex()
                            .child(div().h_full().w(relative(0.5)).bg(linear_gradient(
                                90.,
                                linear_color_stop(color.opacity(0.), 0.),
                                linear_color_stop(color.opacity(0.7), 1.),
                            )))
                            .child(div().h_full().w(relative(0.5)).bg(linear_gradient(
                                90.,
                                linear_color_stop(color.opacity(0.7), 0.),
                                linear_color_stop(color.opacity(0.), 1.),
                            ))),
                    ),
            );
        }
        cards
    }

    fn damage(
        &self,
        snapshot: &BuildSnapshot,
        result: Option<&BuildPerformance>,
        compact: bool,
        cx: &App,
    ) -> Div {
        let p = cx.global::<TooltipTheme>();
        let main = snapshot.active_skill_ids.first().and_then(|id| {
            data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""))
                .iter()
                .find(|skill| &skill.id == id)
        });
        let is_spell = main.is_some_and(|skill| {
            skill
                .tags
                .as_ref()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "Spell"))
        });
        let skill_name = result
            .and_then(|result| result.active_skill_name.as_deref())
            .or_else(|| main.map(|skill| skill.name.as_str()));
        let title = skill_name
            .map(|name| format!("Total DPS · {name}"))
            .unwrap_or_else(|| "Total DPS".into());
        let average = result.and_then(|r| {
            r.damage
                .as_ref()
                .map(|d| d.avg_max as f64)
                .or_else(|| r.attack_damage.as_ref().map(|d| d.combined_avg_max as f64))
        });
        let rate = result.and_then(|r| {
            r.attack_damage
                .as_ref()
                .map(|d| d.attacks_per_second_max)
                .or_else(|| {
                    r.avg_hit_dps_max
                        .zip(average)
                        .filter(|(_, hit)| *hit > 0.)
                        .map(|(dps, hit)| dps / hit)
                })
        });
        let attack_rate = result.is_some_and(|r| r.attack_damage.is_some())
            || main.is_some_and(|skill| skill.uses_attack_speed);
        let hit = result.and_then(|r| {
            r.damage
                .as_ref()
                .map(|d| (d.final_min as f64, d.final_max as f64))
                .or_else(|| {
                    r.attack_damage
                        .as_ref()
                        .map(|d| (d.combined_hit_min as f64, d.combined_hit_max as f64))
                })
        });
        let crit_chance = if is_spell {
            "spell_crit_chance"
        } else {
            "crit_chance"
        };
        let crit_damage = if is_spell {
            "spell_crit_damage"
        } else {
            "crit_damage"
        };
        panel("character-damage", title, cx)
            .child(
                div()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(rems(44. / 13.))
                    .line_height(relative(1.))
                    .text_color(p.accent_hot)
                    .child(TooltipText::new(
                        "character-total-dps",
                        result
                            .and_then(|r| r.combined_dps_min.zip(r.combined_dps_max))
                            .map(integer_range)
                            .unwrap_or_else(|| "—".into()),
                        0.01,
                    )),
            )
            .child(
                div().mt_2().child(match average.zip(rate) {
                    Some((hit, rate)) => div()
                        .flex()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(rems(11. / 13.))
                        .children(
                            [
                                ("character-average-hit", integer(hit), p.muted),
                                ("character-average-label", " AVG HIT × ".into(), p.faint),
                                ("character-cast-rate", decimal(rate), p.muted),
                                ("character-rate-label", " / SEC".into(), p.faint),
                            ]
                            .into_iter()
                            .map(|(id, text, color)| {
                                div()
                                    .flex_shrink_0()
                                    .text_color(color)
                                    .child(TooltipText::new(id, text, 0.14))
                            }),
                        ),
                    None => caption(
                        "character-rate-summary",
                        "select a main skill to see the breakdown",
                        p.faint,
                        11.,
                    ),
                }),
            )
            .child(
                div()
                    .mt_4()
                    .grid()
                    .grid_cols(if compact { 2 } else { 3 })
                    .gap_2p5()
                    .child(metric(
                        "crit-chance",
                        if is_spell {
                            "Spell crit chance"
                        } else {
                            "Crit chance"
                        },
                        stat_text(result, crit_chance),
                        p.accent_hot,
                        cx,
                    ))
                    .child(metric(
                        "crit-damage",
                        if is_spell {
                            "Spell crit damage"
                        } else {
                            "Crit damage"
                        },
                        stat_text(result, crit_damage),
                        p.text,
                        cx,
                    ))
                    .child(metric(
                        "rate",
                        if attack_rate {
                            "Attack rate"
                        } else {
                            "Cast rate"
                        },
                        rate.map(|v| format!("{}/s", decimal(v)))
                            .unwrap_or_else(|| "—".into()),
                        p.text,
                        cx,
                    ))
                    .child(metric(
                        "hit",
                        "Hit damage",
                        hit.map(integer_range).unwrap_or_else(|| "—".into()),
                        p.text,
                        cx,
                    ))
                    .child(metric(
                        "attack-speed",
                        "Attack speed",
                        stat_text(result, "increased_attack_speed"),
                        p.text,
                        cx,
                    ))
                    .child(metric(
                        "enhanced-damage",
                        "Enhanced dmg",
                        stat_text(result, "enhanced_damage"),
                        p.text,
                        cx,
                    )),
            )
    }

    fn defense(&self, result: Option<&BuildPerformance>, cx: &App) -> Div {
        let p = cx.global::<TooltipTheme>();
        let avoidance = [
            ("block_chance", "block"),
            ("dodge_chance", "dodge"),
            ("dodge_spell_hits", "spell dodge"),
        ]
        .into_iter()
        .filter_map(|(key, label)| {
            let value = stat_value(result, key).1;
            (value > 0.).then(|| format!("{label} {}%", decimal(value)))
        })
        .collect::<Vec<_>>();
        let mut card =
            panel_with_trailing(
                "character-defense",
                "Resistances & Defense",
                caption(
                    "resistance-cap",
                    format!(
                        "Capped {}",
                        decimal(
                            result
                                .and_then(|result| defense::effective_cap(
                                    "fire_resistance",
                                    &result.stats
                                ))
                                .unwrap_or(75.)
                        )
                    ),
                    p.faint,
                    10.,
                ),
                cx,
            )
            .child(
                div()
                    .mb_3()
                    .pb_3()
                    .border_b_1()
                    .border_dashed()
                    .border_color(p.accent_deep.opacity(0.25))
                    .child(caption(
                        "avoidance",
                        format!(
                            "Avoidance: {}",
                            if avoidance.is_empty() {
                                "—".into()
                            } else {
                                avoidance.join(" · ")
                            }
                        ),
                        p.muted,
                        10.,
                    ))
                    .when_some(
                        result.filter(|result| !result.defense_insights.is_empty()),
                        |view, result| {
                            view.child(div().mt_2().flex().flex_col().gap_0p5().children(
                                result.defense_insights.iter().enumerate().map(
                                    |(index, insight)| {
                                        div()
                                            .font_family(theme::MONO_FONT_FAMILY)
                                            .text_size(rems(10. / 13.))
                                            .text_color(p.accent_hot.opacity(0.8))
                                            .child(TooltipText::new(
                                                SharedString::from(format!(
                                                    "defense-insight-{index}"
                                                )),
                                                format!("▸ {}", insight.text),
                                                0.06,
                                            ))
                                    },
                                ),
                            ))
                        },
                    ),
            );
        let mut resistance_rows = div().flex().flex_col().gap_2();
        for (key, label) in [
            ("fire_resistance", "Fire"),
            ("cold_resistance", "Cold"),
            ("lightning_resistance", "Lightning"),
            ("poison_resistance", "Poison"),
            ("arcane_resistance", "Arcane"),
        ] {
            let value = stat_value(result, key).1;
            let cap = result
                .and_then(|r| defense::effective_cap(key, &r.stats))
                .unwrap_or(75.);
            let color = theme::stat_color(
                match key {
                    "fire_resistance" => "red",
                    "cold_resistance" => "blue",
                    "lightning_resistance" => "orange",
                    "poison_resistance" => "green",
                    "arcane_resistance" => "purple",
                    _ => "armor",
                },
                cx,
            );
            let width = (value / cap.max(1.)).clamp(0., 1.) as f32;
            resistance_rows = resistance_rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .w_24()
                            .child(caption_tracked(key, label, color, 10., 0.12)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_1p5()
                            .rounded_full()
                            .bg(p.panel_secondary)
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(width))
                                    .rounded_full()
                                    .bg(color.opacity(0.8)),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_right()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(12. / 13.))
                            .text_color(if value == 0. {
                                p.faint
                            } else if value < 0. {
                                p.negative
                            } else {
                                color
                            })
                            .child(if value == 0. {
                                "—".into()
                            } else if value > cap {
                                format!("+{}% ({}%)", decimal(cap), decimal(value))
                            } else {
                                format!("{value:+.0}%")
                            }),
                    ),
            );
        }
        card = card.child(resistance_rows).child(
            div()
                .my_3()
                .border_t_1()
                .border_color(p.accent_deep.opacity(0.25)),
        );
        let mut defense_rows = div()
            .flex()
            .flex_col()
            .gap(rems(1. / 13.))
            .child(defense_row(
                "Life",
                defense_stat_text(result, "life"),
                theme::stat_color("life", cx),
                cx,
            ))
            .child(defense_row(
                "Mana",
                defense_stat_text(result, "mana"),
                theme::stat_color("mana", cx),
                cx,
            ));
        if let Some(result) = result {
            for (label, value) in ehp_rows(&result.ehp) {
                defense_rows = defense_rows.child(defense_row(
                    &label,
                    value.map(integer).unwrap_or_else(|| "∞".into()),
                    p.accent_hot,
                    cx,
                ));
            }
        }
        for (key, label) in [
            ("block_chance", "Block chance"),
            ("physical_damage_reduction", "Phys reduction"),
            ("movement_speed", "Movement speed"),
        ] {
            let armor = stat_value(result, "defense");
            defense_rows = defense_rows.child(
                div()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .py(rems(0.1875))
                    .child(div().flex_1().text_color(p.muted).child(label))
                    .when(
                        key == "physical_damage_reduction" && armor != (0., 0.),
                        |view| {
                            view.child(caption_tracked(
                                "defense-armor-hint",
                                format!("armor {}", formatted_stat(armor, "defense")),
                                p.faint,
                                10.,
                                0.12,
                            ))
                        },
                    )
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_color(p.text)
                            .child(defense_stat_text(result, key)),
                    ),
            );
        }
        card.child(defense_rows)
    }

    fn loadout(&self, snapshot: &BuildSnapshot, wide: bool, cx: &App) -> Div {
        let skills = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""));
        let mut active = snapshot
            .active_skill_ids
            .iter()
            .enumerate()
            .filter_map(|(ix, id)| {
                skills.iter().find(|skill| &skill.id == id).map(|skill| {
                    LoadoutEntry::skill(
                        skill,
                        if ix == 0 { "Main" } else { "Active" },
                        format!("Lv {}", snapshot.skill_ranks.get(id).unwrap_or(&0)),
                    )
                })
            })
            .collect::<Vec<_>>();
        if let Some(aura) = snapshot
            .active_aura_id
            .as_ref()
            .and_then(|id| skills.iter().find(|skill| &skill.id == id))
        {
            active.push(LoadoutEntry::skill(
                aura,
                "Aura",
                format!("Lv {}", snapshot.skill_ranks.get(&aura.id).unwrap_or(&0)),
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for item in snapshot.inventory.values() {
            let Some(base) = data::get_item(&item.base_id) else {
                continue;
            };
            for (name, _) in data::skill_bonus_entries(base, item) {
                let Some(skill) = data::get_item_granted_skill_by_name(name) else {
                    continue;
                };
                let Some(condition) = &skill.condition else {
                    continue;
                };
                if skill.passive_stats.is_some()
                    || skill.passive_converts.is_some()
                    || skill.aura
                    || !snapshot
                        .player_conditions
                        .get(condition)
                        .copied()
                        .unwrap_or(false)
                    || !seen.insert(skill.id.clone())
                {
                    continue;
                }
                active.push(LoadoutEntry {
                    id: format!("granted-{}", skill.id),
                    name: skill.name.clone(),
                    icon: None,
                    sub: "Granted".into(),
                    detail: String::new(),
                });
            }
        }
        let buffs = skills
            .iter()
            .filter(|skill| {
                snapshot
                    .active_buffs
                    .get(&skill.id)
                    .copied()
                    .unwrap_or(false)
            })
            .map(|skill| {
                LoadoutEntry::skill(
                    skill,
                    "Buff",
                    skill
                        .effect_duration
                        .map(|duration| format!("{}s", decimal(duration)))
                        .unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();
        let mut procs = Vec::new();
        for skill in skills {
            if let Some(proc) = &skill.proc
                && snapshot.skill_ranks.get(&skill.id).copied().unwrap_or(0) > 0
                && snapshot
                    .proc_toggles
                    .get(&skill.id)
                    .copied()
                    .unwrap_or(false)
            {
                procs.push(LoadoutEntry::skill(
                    skill,
                    format!("→ {}", proc.target),
                    format!(
                        "{}% · {}",
                        decimal(proc.chance),
                        proc.trigger.replace("on_", "")
                    ),
                ));
            }
            for sub in skill.subskills.iter().flatten() {
                let key = format!("{}:{}", skill.id, sub.id);
                let rank = snapshot.subskill_ranks.get(&key).copied().unwrap_or(0);
                if let Some(proc) = &sub.proc
                    && let Some(target) = &proc.target
                    && rank > 0
                    && snapshot.proc_toggles.get(&key).copied().unwrap_or(false)
                {
                    let mut entry = LoadoutEntry::skill(
                        skill,
                        format!("→ {target}"),
                        format!(
                            "{}% · {}",
                            decimal(
                                proc.chance.base.unwrap_or(0.)
                                    + proc.chance.per_rank.unwrap_or(0.) * rank as f64
                            ),
                            proc.trigger.replace("on_", "")
                        ),
                    );
                    entry.id = key;
                    entry.name = sub.name.clone();
                    procs.push(entry);
                }
            }
        }
        div()
            .grid()
            .grid_cols(if wide { 3 } else { 1 })
            .gap_4()
            .child(loadout_card(
                "active-skills",
                "Active Skills",
                &active,
                "No active skill selected.",
                cx,
            ))
            .child(loadout_card(
                "buffs",
                "Buffs",
                &buffs,
                "No buffs active.",
                cx,
            ))
            .child(loadout_card(
                "procs",
                "Procs",
                &procs,
                "No procs active.",
                cx,
            ))
    }
}

impl Render for CharacterView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.session.read(cx).snapshot();
        let performance = self.tree.read(cx).performance();
        let result = performance.as_ref().map(|result| &result.current);
        let logical_width = f32::from(window.viewport_size().width)
            / self.session.read(cx).state().settings.ui_zoom;
        let wide = logical_width >= 1024.;
        let compact = logical_width < 640.;
        let scroll = self.scroll.clone();
        let weak = cx.entity().downgrade();
        let had_vertical_scroll = self.has_vertical_scroll;
        div()
            .on_children_prepainted(move |_, _, cx| {
                let has_vertical_scroll = scroll.max_offset().y > px(0.);
                if has_vertical_scroll != had_vertical_scroll {
                    let weak = weak.clone();
                    cx.defer(move |cx| {
                        let _ = weak.update(cx, |view, cx| {
                            if view.has_vertical_scroll != has_vertical_scroll {
                                view.has_vertical_scroll = has_vertical_scroll;
                                cx.notify();
                            }
                        });
                    });
                }
            })
            .id("character-overview")
            .track_focus(&self.focus)
            .size_full()
            .min_h_0()
            .scrollbar_width(rems(if self.has_vertical_scroll {
                10. / 13.
            } else {
                0.
            }))
            .bg(cx.global::<TooltipTheme>().background)
            .child(
                div()
                    .p_6()
                    .max_w(rems(1400. / 13.))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(section_heading(
                        "character-heading",
                        "Summary",
                        "Character",
                        cx,
                    ))
                    .child(self.identity(snapshot, cx))
                    .child(self.attributes(
                        snapshot,
                        result,
                        if wide {
                            6
                        } else if compact {
                            2
                        } else {
                            3
                        },
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .when(!wide, |view| view.flex_col())
                            .items_stretch()
                            .gap_4()
                            .child(
                                div()
                                    .flex_grow(1.)
                                    .flex_basis(relative(1.4 / 2.4))
                                    .min_w_0()
                                    .child(
                                        self.damage(snapshot, result, compact, cx)
                                            .when(wide, |view| view.h_full()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_grow(1.)
                                    .flex_basis(relative(1. / 2.4))
                                    .min_w_0()
                                    .child(
                                        self.defense(result, cx).when(wide, |view| view.h_full()),
                                    ),
                            ),
                    )
                    .child(self.loadout(snapshot, wide, cx)),
            )
            .page_scroll(&self.scroll)
            .overflow_y_scroll()
    }
}

fn surface(cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(p.border)
        .bg(linear_gradient(
            180.,
            linear_color_stop(p.panel, 0.),
            linear_color_stop(p.background, 1.),
        ))
}

fn caption(id: impl Into<SharedString>, text: impl Into<String>, color: Hsla, size: f32) -> Div {
    caption_tracked(id, text, color, size, 0.14)
}

fn caption_tracked(
    id: impl Into<SharedString>,
    text: impl Into<String>,
    color: Hsla,
    size: f32,
    tracking: f32,
) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(rems(size / 13.))
        .text_color(color)
        .child(TooltipText::new(
            id.into(),
            text.into().to_uppercase(),
            tracking,
        ))
}

fn point_stat(id: &'static str, label: &str, value: u32, total: Option<u32>, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .min_w_24()
        .rounded_sm()
        .border_1()
        .border_color(p.border_strong)
        .px_3()
        .py_2()
        .bg(linear_gradient(
            180.,
            linear_color_stop(p.background, 0.),
            linear_color_stop(p.panel_secondary, 1.),
        ))
        .child(caption_tracked(id, label, p.faint, 9., 0.18))
        .child(
            div()
                .mt_0p5()
                .flex()
                .items_baseline()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(18. / 13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.accent_hot)
                .child(value.to_string())
                .when_some(total, |view, total| {
                    view.child(
                        div()
                            .text_size(rems(11. / 13.))
                            .font_weight(FontWeight::NORMAL)
                            .text_color(p.faint)
                            .child(format!(" / {total}")),
                    )
                }),
        )
}

fn metric(id: &'static str, label: &str, value: String, color: Hsla, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .rounded_sm()
        .border_1()
        .border_color(p.border_strong)
        .px_3()
        .py_2p5()
        .bg(linear_gradient(
            180.,
            linear_color_stop(p.panel_secondary, 0.),
            linear_color_stop(p.background.opacity(0.7), 1.),
        ))
        .child(caption_tracked(id, label, p.faint, 9., 0.18))
        .child(
            div()
                .mt_1()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(18. / 13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(value),
        )
}

fn defense_row(label: &str, value: String, color: Hsla, cx: &App) -> Div {
    div()
        .flex()
        .items_baseline()
        .justify_between()
        .gap_2()
        .py(rems(0.1875))
        .child(
            div()
                .flex_1()
                .text_color(cx.global::<TooltipTheme>().muted)
                .child(label.to_owned()),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_right()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_color(color)
                .child(value),
        )
}

struct LoadoutEntry {
    id: String,
    name: String,
    icon: Option<std::sync::Arc<Image>>,
    sub: String,
    detail: String,
}
impl LoadoutEntry {
    fn skill(skill: &SkillSpec, sub: impl Into<String>, detail: String) -> Self {
        Self {
            id: skill.id.clone(),
            name: skill.name.clone(),
            icon: crate::skills::skill_icon(&skill.class_id, &skill.id),
            sub: sub.into(),
            detail,
        }
    }
}

fn loadout_card(
    id: &'static str,
    title: &str,
    entries: &[LoadoutEntry],
    empty: &str,
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    let card = panel_with_trailing(
        id,
        title,
        caption(
            format!("{id}-count"),
            format!("{} active", entries.len()),
            p.faint,
            10.,
        ),
        cx,
    );
    if entries.is_empty() {
        return card.child(
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(12. / 13.))
                .text_color(p.muted)
                .child(empty.to_owned()),
        );
    }
    let mut rows = div().flex().flex_col().gap_2();
    for entry in entries {
        rows =
            rows.child(
                div()
                    .id(SharedString::from(format!("{id}-{}", entry.id)))
                    .rounded_sm()
                    .border_1()
                    .border_color(p.border_strong)
                    .px_3()
                    .py_2()
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(p.panel_secondary, 0.),
                        linear_color_stop(p.background.opacity(0.7), 1.),
                    ))
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .child(div().size(rems(32. / 13.)).flex_shrink_0().when_some(
                        entry.icon.clone(),
                        |view, image| {
                            view.child(img(image).size_full().object_fit(ObjectFit::Contain))
                        },
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(rems(1.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(entry.name.clone()),
                            )
                            .child(
                                caption_tracked(
                                    format!("{id}-{}-kind", entry.id),
                                    entry.sub.clone(),
                                    p.accent_deep,
                                    10.,
                                    0.16,
                                )
                                .truncate(),
                            ),
                    )
                    .child(
                        caption(
                            format!("{id}-{}-detail", entry.id),
                            entry.detail.clone(),
                            p.faint,
                            10.,
                        )
                        .flex_shrink_0(),
                    ),
            );
    }
    card.child(rows)
}

fn stat_value(result: Option<&BuildPerformance>, key: &str) -> (f64, f64) {
    result
        .and_then(|result| {
            result
                .stats_combined
                .get(key)
                .or_else(|| result.stats.get(key))
        })
        .copied()
        .unwrap_or((0., 0.))
}
fn stat_text(result: Option<&BuildPerformance>, key: &str) -> String {
    result
        .map(|_| formatted_stat(stat_value(result, key), key))
        .unwrap_or_else(|| "—".into())
}
fn defense_stat_text(result: Option<&BuildPerformance>, key: &str) -> String {
    if stat_value(result, key) == (0., 0.) {
        "—".into()
    } else {
        stat_text(result, key)
    }
}
fn formatted_stat(value: (f64, f64), key: &str) -> String {
    formatted_stat_parts(value, key).concat()
}
fn formatted_stat_parts(value: (f64, f64), key: &str) -> Vec<String> {
    let percent = data::game_config()
        .stats
        .iter()
        .find(|stat| stat.key == key)
        .is_some_and(|stat| stat.format.as_deref() == Some("percent"));
    let sign = if value.0 >= 0. { "+" } else { "" };
    let suffix = if percent { "%" } else { "" };
    if value.0 == value.1 {
        vec![format!("{sign}{}{suffix}", decimal(value.0))]
    } else {
        vec![
            format!("{sign}[{}-", decimal(value.0)),
            format!("{}]{suffix}", decimal(value.1)),
        ]
    }
}
fn integer_range(value: (f64, f64)) -> String {
    if (value.1 - value.0).abs() < 0.5 {
        integer(value.1)
    } else {
        format!("{}–{}", integer(value.0), integer(value.1))
    }
}

fn decimal(value: f64) -> String {
    format!("{value:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}
fn integer(value: f64) -> String {
    let text = format!("{value:.0}");
    let mut result = String::new();
    for (ix, ch) in text.chars().enumerate() {
        if ix > 0
            && (text.len() - ix).is_multiple_of(3)
            && ch.is_ascii_digit()
            && text.as_bytes()[ix - 1].is_ascii_digit()
        {
            result.push(',')
        }
        result.push(ch);
    }
    result
}
fn ehp_rows(result: &defense::EhpResult) -> Vec<(String, Option<f64>)> {
    if result.entries.is_empty() {
        return Vec::new();
    }
    let physical = result
        .entries
        .iter()
        .find(|entry| entry.damage_type == "physical")
        .and_then(|entry| entry.ehp);
    let elements = result
        .entries
        .iter()
        .filter(|entry| entry.damage_type != "physical")
        .collect::<Vec<_>>();
    let same = |a: Option<f64>, b: Option<f64>| a.map(f64::round) == b.map(f64::round);
    if let Some(first) = elements.first()
        && elements.iter().all(|entry| same(entry.ehp, first.ehp))
    {
        return if same(physical, first.ehp) {
            vec![("eHP".into(), physical)]
        } else {
            vec![
                ("Physical eHP".into(), physical),
                ("Elemental eHP".into(), first.ehp),
            ]
        };
    }
    result
        .entries
        .iter()
        .map(|entry| {
            let mut name = entry.damage_type.clone();
            if let Some(first) = name.get_mut(0..1) {
                first.make_ascii_uppercase()
            }
            (format!("{name} eHP"), entry.ehp)
        })
        .collect()
}
