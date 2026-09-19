//! Floating tree controls keep the graph as the main surface, as in Tauri.
use super::*;
use gpui_kit::component::{Icon, IconName, Selectable};
use gpui_kit::{
    Background, Focusable, FontWeight, Hsla, SharedString, Styled, linear_color_stop,
    linear_gradient, relative, rems,
};
use hsplanner_ui::controls::PlannerControl;
use hsplanner_ui::scroll::PageScroll;
use hsplanner_ui::tooltip::CursorTooltipExt;
use hsplanner_ui::tooltip_text::TooltipText;

fn region_name(name: &str) -> &str {
    match name {
        "Universal" => tr("tree.region.universal"),
        "Overworld" => tr("tree.region.overworld"),
        "Chaos Tower" => tr("tree.region.chaos_tower"),
        "Chaos Pillars" => tr("tree.region.chaos_pillars"),
        "Shadow Realm" => tr("tree.region.shadow_realm"),
        "Prime Evil" => tr("tree.region.prime_evil"),
        "Unstable Rift" => tr("tree.region.unstable_rift"),
        "Mining" => tr("tree.region.mining"),
        "Eternal Battlefield" => tr("tree.region.eternal_battlefield"),
        "Cursed Spirit" => tr("tree.region.cursed_spirit"),
        "Unholy Siege" => tr("tree.region.unholy_siege"),
        "Dungeons" => tr("tree.region.dungeons"),
        "Ruby Gardens" => tr("tree.region.ruby_gardens"),
        "Colossal Creatures" => tr("tree.region.colossal_creatures"),
        "Other" => tr("tree.region.other"),
        _ => name,
    }
}

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
            .when(self.header_controls.is_none(), |toolbar| {
                toolbar.justify_end()
            })
            .items_center()
            .gap_1p5()
            .occlude()
            .children(self.header_controls.as_ref().map(|controls| {
                div()
                    .min_w_0()
                    .flex_none()
                    .mr_auto()
                    .child(controls.clone())
            }))
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
                                            .accessibility_label(tr("tree.next_match"))
                                            .cursor_tooltip(tr("tree.next_match_hint"))
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
                                            .accessibility_label(tr("tree.clear_search"))
                                            .cursor_tooltip(tr("tree.clear_search"))
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
                    .label(tr("tree.summary"))
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
                    .label(tr("tree.suggest"))
                    .small()
                    .selected(self.suggest_open)
                    .cursor_tooltip(tr("tree.suggest_hint"))
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_suggest(window, cx))),
                )
            })
            .child(self.button(tr("tree.fit"), Command::Fit, cx))
            .child(self.button(tr("tree.reset"), Command::Reset, cx))
    }

    pub(super) fn status_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let palette = cx.global::<theme::TooltipTheme>();
        let accent = self.tree_theme().accent();
        let status = |key: &'static str, value: String, color: Hsla| {
            let label = tr(key);
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .when(key != "tree.zoom", |row| {
                    row.child(div().text_size(rems(4. / 13.)).text_color(color).child("◆"))
                })
                .child(div().text_color(palette.faint).child(TooltipText::new(
                    SharedString::from(format!("tree-status-{key}")),
                    label.to_uppercase(),
                    0.14,
                )))
                .child(div().text_color(color).child(TooltipText::new(
                    SharedString::from(format!("tree-value-{key}")),
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
                "tree.nodes",
                self.scene.graph.nodes.len().to_string(),
                palette.text,
            ))
            .child(separator())
            .child(status(
                "tree.allocated",
                self.selected.len().to_string(),
                accent,
            ))
            .child(separator())
            .child(status(
                "tree.zoom",
                format!("{:.0}%", self.camera.scale * 100.),
                accent,
            ))
            .when_some(self.build.error.clone(), |bar, error| {
                bar.child(separator())
                    .child(div().text_color(palette.negative).child(
                        if error == "tree.calculate_failed" {
                            tr("tree.calculate_failed").to_owned()
                        } else {
                            error
                        },
                    ))
                    .child(self.button(tr("tree.retry"), Command::RetryCalculation, cx))
            })
            .when(std::env::var_os("HSPLANNER_DIAGNOSTICS").is_some(), |bar| {
                let stats = self.paint_stats.get();
                let calculation = if self.build.in_flight {
                    tr("tree.calculating_lower").to_owned()
                } else if let Some(result) = &self.build.result {
                    trf(
                        "tree.calculation_time",
                        &[("time", format!("{:.1}", result.milliseconds))],
                    )
                } else {
                    String::new()
                };
                bar.child(separator())
                    .child(div().text_color(palette.faint).child(trf(
                        "tree.diagnostics",
                        &[
                            ("calculation", calculation),
                            ("visible", stats.visible.to_string()),
                            ("time", format!("{:.2}", stats.milliseconds)),
                        ],
                    )))
                    .child(self.button(
                        if self.motion_test.is_some() {
                            tr("tree.stop_motion")
                        } else {
                            tr("tree.run_motion")
                        },
                        Command::Motion,
                        cx,
                    ))
                    .children(
                        self.motion_result
                            .as_ref()
                            .map(|result| div().text_color(palette.muted).child(result.summary())),
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
                                .child(tr("tree.allocate_help")),
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
                                        region_name(name).to_uppercase(),
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
                                tr("tree.stat_summary"),
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
                                trf(
                                    "tree.node_count",
                                    &[("count", self.selected.len().to_string())],
                                ),
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
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(tr("tree.magic_find")),
                    )
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
