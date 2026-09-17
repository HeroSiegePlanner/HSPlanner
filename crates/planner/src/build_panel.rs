use gpui_kit::{
    App, Div, FontWeight, InteractiveElement, ParentElement, Stateful, Styled, div, relative, rems,
};
use hsplanner_engine::calc::{performance_diff::PerformanceDiff, skills::Ranged};
use hsplanner_ui::i18n::{tr, trf};

use crate::{build_session::PreviewResult, theme::TooltipTheme};

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
    let label = crate::stat_labels::performance_name(change);
    format!("{sign}{delta} {label}{relative}")
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
                .child(tr("tree.changes.heading")),
        );
    let Some(preview) = preview else {
        return panel.child(div().text_color(palette.faint).child(if pending {
            tr("tree.changes.calculating")
        } else {
            tr("tree.changes.unavailable")
        }));
    };
    let path_label = if preview.removed > 0 {
        trf(
            "tree.changes.removing",
            &[("count", preview.removed.to_string())],
        )
    } else {
        trf(
            "tree.changes.allocating",
            &[("count", preview.added.to_string())],
        )
    };
    let mut groups = vec![(
        "single",
        tr("tree.changes.node").to_owned(),
        &preview.single,
    )];
    if preview.added > 1 || preview.removed > 1 {
        groups.push(("path", path_label, &preview.path));
    }
    for (id, label, changes) in groups {
        let mut group = div()
            .id(id)
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(palette.accent).child(label));
        if changes.is_empty() {
            group = group.child(
                div()
                    .text_color(palette.faint)
                    .child(tr("tree.changes.none")),
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
                group = group.child(div().text_xs().text_color(palette.muted).child(trf(
                    "tree.changes.more",
                    &[("count", (changes.len() - count).to_string())],
                )));
            }
        }
        panel = panel.child(group);
    }
    panel
}
