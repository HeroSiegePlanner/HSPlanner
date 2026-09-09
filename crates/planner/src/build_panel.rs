use gpui_kit::{
    App, Div, FontWeight, InteractiveElement, ParentElement, Stateful, Styled, div, relative, rems,
};
use hsplanner_engine::calc::{performance_diff::PerformanceDiff, skills::Ranged};

use crate::{
    build_session::{BuildSession, PreviewResult},
    theme::TooltipTheme,
};

pub fn format_number(value: f64, percent: bool) -> String {
    let number = if (value - value.round()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    };
    if percent {
        format!("{number}%")
    } else {
        number
    }
}

pub fn format_range(value: Ranged, percent: bool) -> String {
    if (value.1 - value.0).abs() < 0.001 {
        format_number(value.0, percent)
    } else {
        format!(
            "{}–{}",
            format_number(value.0, percent),
            format_number(value.1, percent)
        )
    }
}

fn stat_row(label: &str, value: String, palette: &TooltipTheme) -> Stateful<Div> {
    div()
        .id(gpui_kit::SharedString::from(format!("build-stat-{label}")))
        .flex()
        .justify_between()
        .gap_3()
        .text_size(rems(12. / 13.))
        .child(
            div()
                .text_color(palette.muted)
                .child(gpui_kit::text!(id = "name", label.to_owned())),
        )
        .child(
            div()
                .text_color(palette.text)
                .font_weight(FontWeight::MEDIUM)
                .child(gpui_kit::text!(id = "value", value)),
        )
}

pub fn build_summary(
    session: &BuildSession,
    input: &hsplanner_build::BuildSnapshot,
    cx: &App,
) -> Div {
    let class = input
        .class_id
        .as_deref()
        .and_then(hsplanner_engine::calc::data::get_class)
        .map(|c| c.name.as_str())
        .unwrap_or("Character");
    let palette = cx.global::<TooltipTheme>();
    let mut panel = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(palette.muted)
                .child(format!("{class} · Level {}", input.level)),
        )
        .child(div().text_xs().text_color(palette.faint).child(format!(
            "{} active skills · {}",
            input.active_skill_ids.len(),
            input.difficulty
        )));
    if let Some(error) = &session.error {
        panel = panel.child(
            div()
                .text_sm()
                .text_color(palette.negative)
                .child(error.clone()),
        );
    }
    if let Some(performance) = session.current() {
        let dps = performance
            .combined_dps_min
            .zip(performance.combined_dps_max)
            .map(|range| format_range(range, false))
            .unwrap_or_else(|| "—".into());
        panel = panel.child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(palette.muted)
                        .child(gpui_kit::text!(id = "combined-dps-label", "COMBINED DPS")),
                )
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(palette.accent_hot)
                        .child(gpui_kit::text!(id = "combined-dps-value", dps)),
                ),
        );
        for (key, label) in [("life", "Life"), ("mana", "Mana")] {
            panel = panel.child(stat_row(
                label,
                format_range(
                    performance.stats.get(key).copied().unwrap_or_default(),
                    false,
                ),
                palette,
            ));
        }
        for (key, label) in [
            ("strength", "Strength"),
            ("dexterity", "Dexterity"),
            ("intelligence", "Intelligence"),
            ("energy", "Energy"),
            ("vitality", "Vitality"),
            ("armor", "Armor"),
        ] {
            panel = panel.child(stat_row(
                label,
                format_range(
                    performance.attributes.get(key).copied().unwrap_or_default(),
                    false,
                ),
                palette,
            ));
        }
    } else if session.error.is_none() {
        panel = panel.child(
            div()
                .text_sm()
                .text_color(palette.muted)
                .child("Calculating build…"),
        );
    }
    panel
}

pub(crate) fn change_row(change: &PerformanceDiff, palette: &TooltipTheme) -> Stateful<Div> {
    div()
        .id(gpui_kit::SharedString::from(change.key().to_owned()))
        .text_color(change_color(change, palette))
        .child(gpui_kit::text!(id = "change", format_change(change)))
}

pub fn format_change(change: &PerformanceDiff) -> String {
    let delta = format_number(change.delta(), change.is_percent());
    let sign = if change.delta() > 0. { "+" } else { "" };
    let base = (change.before().0 + change.before().1) / 2.;
    let relative = if base > 0. && !change.is_percent() {
        format!(" ({:+.1}%)", change.delta() / base * 100.)
    } else {
        String::new()
    };
    format!("{sign}{delta} {}{relative}", change.label())
}

fn change_color(change: &PerformanceDiff, palette: &TooltipTheme) -> gpui_kit::Hsla {
    let key = change.key().split(':').next_back().unwrap_or(change.key());
    let direction = match key {
        "hit_dps" | "combined_dps" | "avg_hit" | "life" | "mana" | "armor" | "strength"
        | "dexterity" | "intelligence" | "energy" | "vitality" | "attack_speed" | "cast_speed"
        | "life_regen" | "mana_regen" | "critical_chance" | "critical_damage"
        | "damage_reduction" | "block_chance" | "dodge_chance" => 1.,
        "mana_cost" | "cooldown" | "damage_taken" => -1.,
        _ => return palette.neutral,
    };
    if change.delta() * direction > 0. {
        palette.positive
    } else {
        palette.negative
    }
}

pub fn preview_changes(
    preview: Option<&PreviewResult>,
    pending: bool,
    limit: Option<usize>,
    cx: &App,
) -> Div {
    let palette = cx.global::<TooltipTheme>();
    let mut panel = div()
        .flex()
        .flex_col()
        .gap_2()
        .text_size(rems(12. / 13.))
        .line_height(relative(1.25))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(palette.muted)
                .child("NET CHANGE"),
        );
    let Some(preview) = preview else {
        return panel.child(div().text_color(palette.faint).child(if pending {
            "Calculating changes…"
        } else {
            "Changes unavailable"
        }));
    };
    let path_label = if preview.removed > 0 {
        format!("Removing {} nodes:", preview.removed)
    } else {
        format!("Allocating {} nodes:", preview.added)
    };
    let mut groups = vec![("This node:".to_owned(), &preview.single)];
    if preview.added > 1 || preview.removed > 1 {
        groups.push((path_label, &preview.path));
    }
    for (label, changes) in groups {
        let mut group = div()
            .id(gpui_kit::SharedString::from(label.clone()))
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(palette.accent).child(label));
        if changes.is_empty() {
            group = group.child(
                div()
                    .text_color(palette.faint)
                    .child("No calculated change"),
            );
        } else {
            let count = limit.unwrap_or(changes.len()).min(changes.len());
            group = group.children(
                changes
                    .iter()
                    .take(count)
                    .map(|change| change_row(change, palette)),
            );
            if count < changes.len() {
                group = group.child(div().text_xs().text_color(palette.muted).child(format!(
                    "{} more changes in the side panel",
                    changes.len() - count
                )));
            }
        }
        panel = panel.child(group);
    }
    panel
}
