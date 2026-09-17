//! Floating tree controls keep the graph as the main surface, as in Tauri.
use super::*;
use gpui_kit::component::{Icon, IconName, Selectable};
use gpui_kit::{
    Background, Focusable, FontWeight, Hsla, SharedString, Styled, linear_color_stop,
    linear_gradient, relative, rems,
};
use hsplanner_ui::scroll::PageScroll;
use hsplanner_ui::controls::PlannerControl;
use hsplanner_ui::tooltip::CursorTooltipExt;
use hsplanner_ui::tooltip_text::TooltipText;

impl TreeView {
    pub(super) fn tree_theme(&self) -> theme::TreeTheme {
        match self.scene.graph.kind {
            TreeKind::Incarnation => theme::TreeTheme::incarnation(),
            TreeKind::Ether => theme::TreeTheme::ether(),
        }
    }

    pub(super) fn surface(&self) -> Background {
        let palette = self.tree_theme();
        linear_gradient(
            180.,
            linear_color_stop(palette.surface(), 0.),
            linear_color_stop(palette.surface_end(), 1.),
        )
    }

    pub(super) fn toolbar(&self, cx: &Context<Self>) -> impl IntoElement {
        let palette = cx.global::<theme::TooltipTheme>();
        let tree = self.tree_theme();
        let searching = !self.search.read(cx).value().trim().is_empty();
        div()
            .absolute()
            .top_3()
            .left_3p5()
            .right_3p5()
            .flex()
            .flex_wrap()
            .justify_end()
            .items_center()
            .gap_1p5()
            .occlude()
            .child(
                div().w_64().child(
                    Input::new(&self.search)
                        .planner_style(cx)
                        .font_family(theme::FONT_FAMILY)
                        .text_size(rems(11. / 13.))
                        .map(|input| Styled::h(input, rems(28.25 / 13.)))
                        .line_height(relative(1.5))
                        .small()
                        .prefix(Icon::new(IconName::Search).small())
                        .when(searching, |input| {
                            input.suffix(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        Button::new("next-search-match")
                                            .planner_style(cx)
                                            .small()
                                            .label(format!("{}", self.search_matches.len()))
                                            .accessibility_label("Next matching node")
                                            .cursor_tooltip("Next matching node (Enter)")
                                            .disabled(self.search_matches.is_empty())
                                            .text_color(tree.accent())
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.next_search_match(cx)
                                            })),
                                    )
                                    .child(
                                        Button::new("clear-tree-search")
                                            .planner_style(cx)
                                            .small()
                                            .icon(IconName::Close)
                                            .accessibility_label("Clear search")
                                            .cursor_tooltip("Clear search")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.search.update(cx, |input, cx| {
                                                    input.set_value("", window, cx)
                                                });
                                                window.focus(
                                                    &this.search.read(cx).focus_handle(cx),
                                                    cx,
                                                );
                                            })),
                                    ),
                            )
                        }),
                ),
            )
            .when(self.scene.graph.kind == TreeKind::Ether, |toolbar| {
                toolbar.child(
                    hsplanner_ui::controls::planner_button(
                        "ether-summary",
                        hsplanner_ui::controls::ButtonTone::Neutral,
                        cx,
                    )
                    .label("Summary")
                    .small()
                    .selected(self.summary_open)
                    .text_color(if self.summary_open {
                        tree.accent()
                    } else {
                        palette.muted
                    })
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(tree.control(), 0.),
                        linear_color_stop(tree.control_end(), 1.),
                    ))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.summary_open = !this.summary_open;
                        this.hovered = None;
                        window.focus(&this.focus, cx);
                        cx.notify();
                    })),
                )
            })
            .when(self.scene.graph.kind == TreeKind::Incarnation, |toolbar| {
                toolbar.child(
                    hsplanner_ui::controls::planner_button(
                        "tree-suggest-toggle",
                        hsplanner_ui::controls::ButtonTone::Neutral,
                        cx,
                    )
                    .label("Suggest")
                    .small()
                    .selected(self.suggest_open)
                    .cursor_tooltip("Suggest nodes that raise DPS within a point budget")
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_suggest(window, cx))),
                )
            })
            .child(self.button("Fit", Command::Fit, cx))
            .child(self.button("Reset", Command::Reset, cx))
    }

    pub(super) fn status_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let palette = cx.global::<theme::TooltipTheme>();
        let accent = self.tree_theme().accent();
        let status = |label: &'static str, value: String, color: Hsla| {
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .when(label != "Zoom", |row| {
                    row.child(div().text_size(rems(4. / 13.)).text_color(color).child("◆"))
                })
                .child(div().text_color(palette.faint).child(TooltipText::new(
                    SharedString::from(format!("tree-status-{label}")),
                    label.to_uppercase(),
                    0.14,
                )))
                .child(div().text_color(color).child(TooltipText::new(
                    SharedString::from(format!("tree-value-{label}")),
                    value,
                    0.14,
                )))
        };
        let separator = || div().w_px().h_3().bg(palette.border);
        div()
            .absolute()
            .bottom_3p5()
            .left_3p5()
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_1p5()
            .rounded_sm()
            .border_1()
            .border_color(palette.border)
            .bg(self.surface())
            .font_family(theme::MONO_FONT_FAMILY)
            .text_size(rems(10. / 13.))
            .line_height(relative(1.5))
            .occlude()
            .child(status(
                "Nodes",
                self.scene.graph.nodes.len().to_string(),
                palette.text,
            ))
            .child(separator())
            .child(status("Allocated", self.selected.len().to_string(), accent))
            .child(separator())
            .child(status(
                "Zoom",
                format!("{:.0}%", self.camera.scale * 100.),
                accent,
            ))
            .when_some(self.build.error.clone(), |bar, error| {
                bar.child(separator())
                    .child(div().text_color(palette.negative).child(error))
                    .child(self.button("Retry calculation", Command::RetryCalculation, cx))
            })
            .when(std::env::var_os("HSPLANNER_DIAGNOSTICS").is_some(), |bar| {
                let stats = self.paint_stats.get();
                let calculation = if self.build.in_flight {
                    "calculating…".to_owned()
                } else if let Some(result) = &self.build.result {
                    format!("calc {:.1} ms", result.milliseconds)
                } else {
                    String::new()
                };
                bar.child(separator())
                    .child(div().text_color(palette.faint).child(format!(
                        "{calculation} · {} visible · canvas {:.2} ms",
                        stats.visible, stats.milliseconds
                    )))
                    .child(self.button(
                        if self.motion_test.is_some() {
                            "Stop motion test"
                        } else {
                            "Run motion test"
                        },
                        Command::Motion,
                        cx,
                    ))
                    .children(
                        self.motion_result
                            .clone()
                            .map(|result| div().text_color(palette.muted).child(result)),
                    )
            })
    }

    pub(super) fn ether_summary_panel(
        &self,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let palette = cx.global::<theme::TooltipTheme>();
        let accent = self.tree_theme().accent();
        let magic_find = self
            .ether_summary
            .iter()
            .find(|entry| entry.key == "etherUnSmall01");
        let mut groups: std::collections::BTreeMap<&str, Vec<_>> =
            std::collections::BTreeMap::new();
        for entry in self
            .ether_summary
            .iter()
            .filter(|entry| entry.key != "etherUnSmall01")
        {
            groups
                .entry(theme::ether_region(&entry.key).0)
                .or_default()
                .push(entry);
        }
        let mut groups: Vec<_> = groups.into_iter().collect();
        groups.sort_by_key(|(name, _)| (*name != "Universal", *name));
        let totals = div()
            .id("ether-summary-scroll")
            .flex_initial()
            .min_h_0()
            .overflow_y_scroll()
            .page_scroll(&self.summary_scroll)
            .child(
                div()
                    .flex_shrink_0()
                    .px_3()
                    .py_2()
                    .flex()
                    .flex_col()
                    .gap_2p5()
                    .when(groups.is_empty(), |body| {
                        body.child(
                            div()
                                .py_2()
                                .text_size(rems(11. / 13.))
                                .italic()
                                .text_center()
                                .text_color(palette.muted)
                                .child("Allocate nodes to see totals."),
                        )
                    })
                    .children(groups.into_iter().map(|(name, entries)| {
                        let color = theme::ether_region(&entries[0].key).1;
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .mb_1p5()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .text_size(rems(9. / 13.))
                                    .text_color(color)
                                    .child(div().text_size(rems(4. / 13.)).child("◆"))
                                    .child(TooltipText::new(
                                        SharedString::from(format!("ether-group-{name}")),
                                        name.to_uppercase(),
                                        0.18,
                                    ))
                                    .child(div().flex_1().h_px().bg(palette.border)),
                            )
                            .child(div().flex().flex_col().gap_2().children(
                                entries.into_iter().map(|entry| {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .line_height(relative(1.375))
                                        .child(
                                            div()
                                                .flex()
                                                .items_baseline()
                                                .justify_between()
                                                .gap_2()
                                                .text_size(rems(11.5 / 13.))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .flex()
                                                        .items_baseline()
                                                        .gap_1()
                                                        .child(
                                                            div()
                                                                .font_family(
                                                                    theme::MONO_FONT_FAMILY,
                                                                )
                                                                .text_size(rems(10. / 13.))
                                                                .text_color(color)
                                                                .child(format!("{}x", entry.count)),
                                                        )
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .min_w_0()
                                                                .text_ellipsis()
                                                                .child(entry.label.clone()),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex_shrink_0()
                                                        .font_family(theme::MONO_FONT_FAMILY)
                                                        .text_size(rems(11. / 13.))
                                                        .text_color(color)
                                                        .child(format!(
                                                            "+{}{}",
                                                            entry.total,
                                                            if entry.is_percent { "%" } else { "" }
                                                        )),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_size(rems(10. / 13.))
                                                .text_color(palette.faint)
                                                .child(entry.description.clone()),
                                        )
                                }),
                            ))
                    })),
            );
        div()
            .w_72()
            .max_h(px((f32::from(window.viewport_size().height)
                - 190. * f32::from(window.rem_size()) / 13.)
                .max(100.)))
            .occlude()
            .flex()
            .flex_col()
            .min_h_0()
            .bg(self.surface())
            .border_1()
            .border_color(palette.border)
            .rounded_sm()
            .child(
                div()
                    .flex_shrink_0()
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(palette.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(10. / 13.))
                            .text_color(accent)
                            .child(div().text_size(rems(5. / 13.)).child("◆"))
                            .child(TooltipText::new(
                                "ether-summary-title",
                                "STAT SUMMARY",
                                0.18,
                            )),
                    )
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(10. / 13.))
                            .text_color(palette.faint)
                            .child(TooltipText::new(
                                "ether-summary-count",
                                format!("{} NODES", self.selected.len()),
                                0.14,
                            )),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(palette.border)
                    .text_size(rems(12. / 13.))
                    .child(div().font_weight(FontWeight::MEDIUM).child("Magic Find"))
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_color(palette.accent_hot)
                            .child(
                                magic_find
                                    .map_or("—".into(), |entry| format!("+{}%", entry.total)),
                            ),
                    ),
            )
            .child(totals)
    }
}
