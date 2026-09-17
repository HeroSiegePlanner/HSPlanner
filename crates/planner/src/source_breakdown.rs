//! SourceTooltip.tsx presentation over the existing engine breakdown contract.
use gpui_kit::{
    base::Disableable,
    component::{WindowExt, button::Button},
    prelude::*,
    *,
};
use hsplanner_build::session::Session;
use hsplanner_engine::calc::stats::{
    SourceContribution, SourceType, StatBreakdown, StatTypeSubtotal,
};
use hsplanner_ui::scroll::PageScroll;
use hsplanner_ui::tooltip::CursorTooltipExt;
use hsplanner_ui::{
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};
use std::sync::Arc;

fn units(value: f32) -> Rems {
    rems(value / 13.)
}

fn number(value: f64, precision: usize) -> String {
    let value = if value.abs() < 0.000_001 { 0. } else { value };
    let text = format!("{value:.precision$}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

pub(crate) fn format_total(value: (f64, f64), percent: bool) -> String {
    let sign = if value.0 >= 0. { "+" } else { "" };
    let a = number(value.0, 2);
    let b = number(value.1, 2);
    let range = if a == b { a } else { format!("{a}–{b}") };
    format!("{sign}{range}{}", if percent { "%" } else { "" })
}

pub(crate) fn format_source(value: (f64, f64), percent: bool) -> String {
    let a = number(value.0, 2);
    let b = number(value.1, 2);
    if a == b {
        return format_total(value, percent);
    }
    format!(
        "{}[{a}-{b}]{}",
        if value.0 >= 0. { "+" } else { "" },
        if percent { "%" } else { "" }
    )
}

fn multiplier(value: (f64, f64)) -> String {
    let a = number(1. + value.0 / 100., 3);
    let b = number(1. + value.1 / 100., 3);
    if a == b {
        format!("×{a}")
    } else {
        format!("×{a}–{b}")
    }
}

fn source_label(kind: SourceType) -> &'static str {
    match kind {
        SourceType::Class => "CLASS",
        SourceType::Allocated => "ALLOCATED",
        SourceType::Level => "LEVEL",
        SourceType::Attribute => "ATTRIBUTE",
        SourceType::Item => "ITEM",
        SourceType::Socket => "SOCKET",
        SourceType::Skill => "SKILL",
        SourceType::Subskill => "SUBTREE",
        SourceType::Custom => "CONFIG",
        SourceType::Tree => "TREE",
    }
}
fn source_color(kind: SourceType, p: &TooltipTheme) -> Hsla {
    match kind {
        SourceType::Class => p.text.opacity(0.7),
        SourceType::Allocated => p.accent,
        SourceType::Level => p.muted,
        SourceType::Attribute => p.source_attribute,
        SourceType::Item => p.source_item,
        SourceType::Socket => p.source_socket,
        SourceType::Skill | SourceType::Subskill => p.source_skill,
        SourceType::Custom => p.source_custom,
        SourceType::Tree => p.source_tree,
    }
}
fn display_label(source: &SourceContribution) -> String {
    if let Some(forge) = &source.forge {
        return format!("↳ Forged modifier ({})", forge.mod_name);
    }
    if source.source_type != SourceType::Tree {
        return source.label.clone();
    }
    let label = source
        .label
        .strip_prefix("Tree: ")
        .or_else(|| source.label.strip_prefix("Incarnation: "))
        .unwrap_or(&source.label);
    let Some(start) = label.find(" #") else {
        return label.to_owned();
    };
    let digits = label[start + 2..]
        .chars()
        .take_while(char::is_ascii_digit)
        .count();
    if digits == 0 {
        return label.to_owned();
    }
    format!("{}{}", &label[..start], &label[start + 2 + digits..])
}

// Stable magnitude ordering, with forged contributions immediately below their item.
fn ordered_sources(sources: &[SourceContribution]) -> Vec<&SourceContribution> {
    let mut sorted: Vec<_> = sources.iter().collect();
    sorted.sort_by(|a, b| {
        b.value
            .0
            .abs()
            .max(b.value.1.abs())
            .total_cmp(&a.value.0.abs().max(a.value.1.abs()))
    });
    let mut result = Vec::with_capacity(sorted.len());
    for source in sorted.iter().filter(|s| s.forge.is_none()) {
        result.push(*source);
        if source.source_type == SourceType::Item {
            for child in &sorted {
                if child
                    .forge
                    .as_ref()
                    .is_some_and(|forge| forge.item_name == source.label)
                    && !result.iter().any(|s| std::ptr::eq(*s, *child))
                {
                    result.push(*child);
                }
            }
        }
    }
    for source in sorted {
        if !result.iter().any(|s| std::ptr::eq(*s, source)) {
            result.push(source);
        }
    }
    result
}

fn section(title: &str, value: Option<String>, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .flex()
        .items_baseline()
        .justify_between()
        .gap_2()
        .px_3()
        .py_1()
        .bg(p.accent_deep.opacity(0.1))
        .border_b_1()
        .border_color(p.border.opacity(0.4))
        .text_size(units(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.accent_hot.opacity(0.8))
        .child(TooltipText::new(
            SharedString::from(format!("source-section-{title}")),
            title.to_uppercase(),
            0.12,
        ))
        .children(value.map(|v| {
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_color(p.text.opacity(0.7))
                .child(v)
        }))
}
fn value_row(label: &str, value: String, combined: bool, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .flex()
        .items_baseline()
        .justify_between()
        .gap_2()
        .text_size(units(11.))
        .text_color(if combined {
            p.accent_hot
        } else {
            p.text.opacity(0.7)
        })
        .child(div().min_w_0().child(label.to_owned()))
        .child(
            div()
                .flex_shrink_0()
                .font_family(theme::MONO_FONT_FAMILY)
                .child(value),
        )
}
fn calculation(b: &StatBreakdown, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let mut rows = div().px_3().py_2().flex().flex_col().gap_1();
    let total = format_total(b.combined, b.is_percent);
    if !b.has_more && !b.has_increased {
        rows = rows.child(value_row("Total", total.clone(), true, cx));
    } else {
        let flat = b.has_increased || !b.is_percent;
        rows = rows.child(value_row(
            if flat {
                "Additive (flat)"
            } else {
                "Additive (+)"
            },
            format_total(b.additive_sum, !flat),
            false,
            cx,
        ));
        if b.has_increased {
            rows = rows.child(value_row(
                "Increased (+%)",
                format_total(b.increased_sum, true),
                false,
                cx,
            ));
        }
        if b.has_more {
            rows = rows.child(value_row(
                if flat {
                    "More (×)"
                } else {
                    "Multiplicative (×)"
                },
                multiplier(b.more_sum),
                false,
                cx,
            ));
        }
        rows = rows
            .child(
                div()
                    .border_t_1()
                    .border_dashed()
                    .border_color(p.border.opacity(0.6))
                    .pt_1()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(10.))
                    .text_color(p.text.opacity(0.4))
                    .child(if flat {
                        "flat × (1 + inc/100) × (1 + more/100)"
                    } else {
                        "(1 + add/100) × (1 + more/100) − 1"
                    }),
            )
            .child(
                value_row("Combined", total.clone(), true, cx)
                    .border_t_1()
                    .border_color(p.border.opacity(0.4))
                    .pt_1(),
            );
    }
    if let Some(raw) = b.pre_diminish {
        rows = rows.child(value_row(
            "Before diminishing returns",
            format_total(raw, b.is_percent),
            false,
            cx,
        ));
    }
    div()
        .child(section("Calculation", Some(total), cx))
        .child(rows)
}
fn subtotal_rows(rows: &[StatTypeSubtotal], percent: bool, more: bool, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div().children(rows.iter().map(|s| {
        div()
            .flex()
            .items_baseline()
            .justify_between()
            .gap_2()
            .px_3()
            .py_0p5()
            .text_size(units(11.))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap_1p5()
                    .child(
                        div()
                            .text_size(units(9.))
                            .text_color(source_color(s.source_type, p))
                            .child(source_label(s.source_type)),
                    )
                    .child(
                        div()
                            .text_size(units(10.))
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_color(p.text.opacity(0.6))
                            .child(format!("×{}", s.count)),
                    ),
            )
            .child(
                div()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_color(p.accent_hot)
                    .child(if more {
                        multiplier(s.sum)
                    } else {
                        format_total(s.sum, percent)
                    }),
            )
    }))
}
fn by_source(b: &StatBreakdown, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let mut rows = div().py_1().child(subtotal_rows(
        &b.additive_by_type,
        b.is_percent && !b.has_increased,
        false,
        cx,
    ));
    for (title, sources, more) in [
        ("Increased", &b.increased_by_type, false),
        ("Multiplicative", &b.more_by_type, true),
    ] {
        if !sources.is_empty() {
            rows = rows
                .child(
                    div()
                        .mx_3()
                        .my_1()
                        .border_t_1()
                        .border_dashed()
                        .border_color(p.border.opacity(0.4)),
                )
                .child(
                    div()
                        .px_3()
                        .py_0p5()
                        .text_size(units(9.))
                        .text_color(p.text.opacity(0.4))
                        .child(title.to_uppercase()),
                )
                .child(subtotal_rows(sources, true, more, cx));
        }
    }
    div().child(section("By source", None, cx)).child(rows)
}
fn source_rows(
    rows: &[SourceContribution],
    percent: bool,
    extended: bool,
    snapshot: Option<&hsplanner_build::BuildSnapshot>,
    group: &str,
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    // Identical engine contributions have no distinct domain ID; disambiguate only
    // repeated occurrences within their stable source key and additive/more group.
    let mut occurrences = std::collections::HashMap::<String, usize>::new();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_3()
        .py_2()
        .children(ordered_sources(rows).into_iter().map(|s| {
            let label = display_label(s);
            let row = div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap_2()
                .text_size(units(10.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_baseline()
                        .gap_1p5()
                        .when(s.forge.is_none(), |v| {
                            v.child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(units(9.))
                                    .text_color(source_color(s.source_type, p))
                                    .child(source_label(s.source_type)),
                            )
                        })
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .text_color(if s.forge.is_some() {
                                    p.negative
                                } else {
                                    p.text.opacity(0.8)
                                })
                                .when(!extended, |v| v.truncate())
                                .when(extended, |v| v.whitespace_normal())
                                .child(label),
                        ),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_color(p.accent_hot)
                        .child(format_source(s.value, percent)),
                );
            let key = format!(
                "source-{group}-{}-{}-{:?}-{:?}",
                source_label(s.source_type),
                s.label,
                s.forge,
                s.value
            );
            let occurrence = occurrences.entry(key.clone()).or_default();
            let id = SharedString::from(format!("{key}-{occurrence}"));
            *occurrence += 1;
            match snapshot {
                Some(snapshot) => crate::source_preview::wrap(id.into(), s, snapshot, row),
                None => row.into_any_element(),
            }
        }))
}
fn body(
    b: &StatBreakdown,
    extended: bool,
    snapshot: Option<&hsplanner_build::BuildSnapshot>,
    cx: &App,
) -> Div {
    let mut content = div().when(extended, |v| {
        v.child(calculation(b, cx)).child(by_source(b, cx))
    });
    let grouped = b.has_more || b.has_increased;
    if grouped {
        content = content.child(section(
            if b.is_percent && !b.has_increased {
                "Additive (+)"
            } else {
                "Additive (flat)"
            },
            Some(format_total(
                b.additive_sum,
                b.is_percent && !b.has_increased,
            )),
            cx,
        ));
    }
    content = content.child(source_rows(
        &b.additive_sources,
        b.is_percent && !b.has_increased,
        extended,
        snapshot,
        "additive",
        cx,
    ));
    if b.has_increased {
        content = content
            .child(section(
                "Increased (+%)",
                Some(format_total(b.increased_sum, true)),
                cx,
            ))
            .child(source_rows(
                &b.increased_sources,
                true,
                extended,
                snapshot,
                "increased",
                cx,
            ));
    }
    if b.has_more {
        content = content
            .child(section(
                "Multiplicative (Total)",
                Some(multiplier(b.more_sum)),
                cx,
            ))
            .child(source_rows(
                &b.more_sources,
                true,
                extended,
                snapshot,
                "more",
                cx,
            ));
    }
    content
}

struct SourcesTooltip(Arc<StatBreakdown>);
impl Render for SourcesTooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        div()
            .w(units(320.))
            .max_w(window.viewport_size().width - window.rem_size())
            .max_h(window.viewport_size().height - window.rem_size())
            .overflow_hidden()
            .rounded(units(4.))
            .border_1()
            .border_color(p.accent_deep.opacity(0.6))
            .bg(p.panel)
            .text_color(p.text)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(p.accent.opacity(0.14), 0.),
                        linear_color_stop(p.accent.opacity(0.04), 1.),
                    ))
                    .child(
                        div()
                            .text_size(units(10.))
                            .text_color(p.accent_hot)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(TooltipText::new("sources-title", "SOURCES", 0.12)),
                    )
                    .child(
                        div()
                            .text_size(units(9.))
                            .text_color(p.text.opacity(0.4))
                            .child("RIGHT-CLICK TO PIN"),
                    ),
            )
            .child(body(&self.0, false, None, cx))
    }
}
struct SourceDialog {
    snapshot: hsplanner_build::BuildSnapshot,
    breakdown: Arc<StatBreakdown>,
    scroll: ScrollHandle,
    measured_height: Option<Pixels>,
    _subscription: Subscription,
}
impl Render for SourceDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let max_height = window.viewport_size().height * 0.8;
        let weak = cx.entity().downgrade();
        let measured_height = self.measured_height;
        div()
            .on_children_prepainted(move |bounds, window, cx| {
                let Some(first) = bounds.first() else {
                    return;
                };
                let height = bounds
                    .iter()
                    .map(|b| b.bottom())
                    .fold(first.top(), Pixels::max)
                    - first.top()
                    + px(2.);
                if measured_height != Some(height) {
                    let weak = weak.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            if this.measured_height != Some(height) {
                                this.measured_height = Some(height);
                                cx.notify();
                                window.refresh();
                            }
                        });
                    });
                }
            })
            .flex()
            .flex_col()
            .max_h(max_height)
            .overflow_hidden()
            .text_color(p.text)
            .bg(linear_gradient(
                180.,
                linear_color_stop(p.panel_secondary, 0.),
                linear_color_stop(p.background, 1.),
            ))
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(p.border.opacity(0.7))
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(p.accent.opacity(0.18), 0.),
                        linear_color_stop(p.accent.opacity(0.06), 1.),
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .items_baseline()
                            .gap_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .text_size(units(11.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.accent_hot)
                                    .child(
                                        TooltipText::new(
                                            "breakdown-title",
                                            self.breakdown.stat_name.to_uppercase(),
                                            0.14,
                                        )
                                        .wrap(),
                                    ),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .text_size(units(10.))
                                    .text_color(p.text.opacity(0.6))
                                    .child(self.breakdown.stat_key.clone()),
                            ),
                    )
                    .child(
                        hsplanner_ui::controls::icon_button(
                            "close-source-breakdown",
                            "×",
                            false,
                            cx,
                        )
                        .accessibility_label("Close source breakdown")
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                    ),
            )
            .child(
                div()
                    .id("source-breakdown-scroll")
                    .min_h_0()
                    .max_h(max_height - window.rem_size() * (76. / 13.))
                    .overflow_y_scroll()
                    .page_scroll(&self.scroll)
                    .child(body(&self.breakdown, true, Some(&self.snapshot), cx)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .px_4()
                    .py_1p5()
                    .border_t_1()
                    .border_color(p.border.opacity(0.7))
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(9.))
                    .text_color(p.faint)
                    .child("HOVER OR SELECT ITEM / TREE SOURCES TO PREVIEW · ESC CLOSE"),
            )
    }
}
fn open(
    breakdown: Arc<StatBreakdown>,
    session: Entity<Session>,
    window: &mut Window,
    cx: &mut App,
) {
    let revision = session.read(cx).calculation_revision();
    let view = cx.new(|cx| {
        let subscription = cx.observe_in(&session, window, move |_, session, window, cx| {
            if session.read(cx).calculation_revision() != revision {
                window.close_dialog(cx);
            }
        });
        SourceDialog {
            snapshot: session.read(cx).snapshot().clone(),
            breakdown,
            scroll: ScrollHandle::new(),
            measured_height: None,
            _subscription: subscription,
        }
    });
    window.open_dialog(cx, move |dialog, window, cx| {
        let height = view.read(cx).measured_height;
        dialog
            .opacity(if height.is_some() { 1. } else { 0. })
            .close_button(false)
            .p_0()
            .gap_0()
            .w((window.rem_size() * (520. / 13.)).min(window.viewport_size().width * 0.9))
            .margin_top(
                ((window.viewport_size().height
                    - height.unwrap_or(window.viewport_size().height * 0.8))
                    / 2.)
                    .max(window.viewport_size().height * 0.1),
            )
            .child(view.clone())
    });
}

pub(crate) fn trigger(
    id: impl Into<ElementId>,
    button: Button,
    breakdown: StatBreakdown,
    session: Entity<Session>,
) -> Stateful<Div> {
    let available =
        !breakdown.additive_sources.is_empty() || breakdown.has_increased || breakdown.has_more;
    let breakdown = Arc::new(breakdown);
    let click_data = breakdown.clone();
    let click_session = session.clone();
    div()
        .id(id)
        .w_full()
        .when(available, |v| {
            let hover_data = breakdown.clone();
            v.cursor_tooltip_view(move |_, cx| {
                cx.new(|_| SourcesTooltip(hover_data.clone())).into()
            })
            .on_mouse_down(MouseButton::Right, move |_, window, cx| {
                cx.stop_propagation();
                open(breakdown.clone(), session.clone(), window, cx);
            })
        })
        .child(button.disabled(!available).on_click(move |_, window, cx| {
            if available {
                open(click_data.clone(), click_session.clone(), window, cx);
            }
        }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsplanner_engine::calc::stats::{Forge, compute_stat_breakdown};
    #[::core::prelude::v1::test]
    fn attack_speed_uses_engine_totals_and_formats_factors_not_percent_sums() {
        let source = |kind, value| SourceContribution {
            label: "Attack Speed".into(),
            source_type: kind,
            value,
            forge: None,
        };
        let sources = std::collections::HashMap::from([
            (
                "increased_attack_speed".into(),
                vec![source(SourceType::Item, (170., 227.))],
            ),
            (
                "increased_attack_speed_more".into(),
                vec![source(SourceType::Tree, (5., 5.))],
            ),
        ]);
        let b = compute_stat_breakdown(&sources, "increased_attack_speed", Some((183.5, 243.35)));
        assert_eq!(format_total(b.combined, true), "+183.5–243.35%");
        assert_eq!(format_source(b.additive_sum, true), "+[170-227]%");
        assert_eq!(multiplier(b.more_sum), "×1.05");
        assert_eq!(b.additive_by_type[0].count, 1);
        assert_eq!(multiplier((-5., 10.)), "×0.95–1.1");
        assert_eq!(format_total((0., 0.), false), "+0");
        assert_eq!(format_total((-1.25, 2.5), true), "-1.25–2.5%");
    }
    #[::core::prelude::v1::test]
    fn ordering_retains_duplicates_and_keeps_forge_below_parent() {
        let source = |label: &str, value| SourceContribution {
            label: label.into(),
            source_type: SourceType::Item,
            value: (value, value),
            forge: None,
        };
        let parent = source("Axe", 5.);
        let mut forge = source("Crystal", 50.);
        forge.forge = Some(Forge {
            item_name: "Axe".into(),
            mod_name: "Damage".into(),
            kind: hsplanner_engine::calc::data::ForgeKind::SatanicCrystal,
        });
        let sources = vec![parent, source("Other", 10.), forge, source("Other", 10.)];
        let ordered = ordered_sources(&sources);
        assert_eq!(
            ordered.iter().map(|s| s.label.as_str()).collect::<Vec<_>>(),
            ["Other", "Other", "Axe", "Crystal"]
        );
    }
    #[::core::prelude::v1::test]
    fn tree_labels_remove_identity_but_preserve_condition() {
        let s = SourceContribution {
            label: "Tree: Attack Speed #123 (conditional)".into(),
            source_type: SourceType::Tree,
            value: (3., 3.),
            forge: None,
        };
        assert_eq!(display_label(&s), "Attack Speed (conditional)");
    }
}
