//! Custom config text: one stat per line, `//` notes, edited in its own dialog.
use super::*;
use gpui_kit::component::{WindowExt, button::ButtonVariants};

pub(super) fn serialize_custom_stats(stats: &[CustomStat]) -> String {
    stats
        .iter()
        .map(|stat| {
            let name = data::game_config()
                .stats
                .iter()
                .find(|def| def.key == stat.stat_key)
                .map(|def| def.name.as_str())
                .unwrap_or(&stat.stat_key);
            format!("{} {name}", stat.value)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn insertable(def: &StatDef) -> bool {
    !def.item_only.unwrap_or(false) && !def.skill_scoped.unwrap_or(false)
}

pub(super) fn parse_custom_text(text: &str) -> (Vec<CustomStat>, Vec<String>) {
    let mut stats = Vec::new();
    let mut issues = Vec::new();
    for (ix, line) in text.lines().enumerate() {
        let line = line.split_once("//").map_or(line, |(stat, _)| stat).trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_lowercase();
        let matched = data::game_config()
            .stats
            .iter()
            .filter(|def| insertable(def))
            .find_map(|def| {
                [def.name.as_str(), def.key.as_str()]
                    .into_iter()
                    .find_map(|name| {
                        let suffix = format!(" {}", name.to_lowercase());
                        lower
                            .strip_suffix(&suffix)
                            .filter(|value| parse_custom_stat_value(value).is_some())
                            .map(|value| {
                                (
                                    def.key.clone(),
                                    value.split_whitespace().collect::<String>(),
                                )
                            })
                    })
            });
        if let Some((stat_key, value)) = matched {
            stats.push(CustomStat { stat_key, value })
        } else {
            issues.push(trf(
                "planner.config.invalid_custom_line",
                &[("0", (ix + 1).to_string())],
            ))
        }
    }
    (stats, issues)
}

/// The text the build was saved with, or one rebuilt from its stats when they no
/// longer match it (older builds, or stats changed outside the dialog).
pub(super) fn custom_text(snapshot: &BuildSnapshot) -> String {
    let typed = &snapshot.custom_stats_text;
    let same = |a: &[CustomStat], b: &[CustomStat]| {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(a, b)| a.stat_key == b.stat_key && a.value == b.value)
    };
    if !typed.is_empty() && same(&parse_custom_text(typed).0, &snapshot.custom_stats) {
        typed.clone()
    } else {
        serialize_custom_stats(&snapshot.custom_stats)
    }
}

impl ConfigView {
    pub(super) fn open_custom_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        let build = self.document.build.clone();
        let text = custom_text(self.session.read(cx).snapshot());
        let editor = cx.new(|cx| CustomStatsEditor::new(text, owner, build, window, cx));
        window.open_dialog(cx, move |dialog, window, cx| {
            let p = cx.global::<TooltipTheme>();
            dialog
                .width((window.rem_size() * (1100. / 13.)).min(window.viewport_size().width * 0.96))
                .h(window.viewport_size().height * 0.88)
                .margin_top(window.viewport_size().height * 0.06)
                .bg(p.background)
                .on_ok(|_, _, _| false)
                .child(
                    div()
                        .size_full()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            gpui_kit::component::dialog::DialogTitle::new()
                                .flex_none()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .font_family(theme::MONO_FONT_FAMILY)
                                                .text_xs()
                                                .text_color(p.accent)
                                                .child(tr("planner.config.custom_heading")),
                                        )
                                        .child(tr("planner.config.custom")),
                                ),
                        )
                        .child(editor.clone()),
                )
        });
    }
}

struct CustomStatsEditor {
    owner: WeakEntity<ConfigView>,
    build: Option<String>,
    text: Entity<TextareaState>,
    search: Entity<InputState>,
    stats: Vec<(String, String, bool)>,
    issues: Vec<String>,
    _subscriptions: Vec<Subscription>,
}
impl CustomStatsEditor {
    fn new(
        value: String,
        owner: WeakEntity<ConfigView>,
        build: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let issues = parse_custom_text(&value).1;
        let text = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx)
                .rows(24)
                .placeholder(tr("planner.config.custom_example"));
            state.set_value(value, window, cx);
            state
        });
        let search = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("planner.stats.search_placeholder"))
        });
        let mut stats: Vec<_> = data::game_config()
            .stats
            .iter()
            .filter(|def| insertable(def))
            .map(|def| {
                (
                    def.key.clone(),
                    def.name.clone(),
                    def.format.as_deref() == Some("percent"),
                )
            })
            .collect();
        stats.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        stats.dedup_by(|a, b| a.1.eq_ignore_ascii_case(&b.1));
        let subscriptions = vec![
            cx.subscribe(&text, |this, text, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.issues = parse_custom_text(&text.read(cx).value().to_string()).1;
                    cx.notify();
                }
            }),
            cx.subscribe(&search, |_, _, _: &InputEvent, cx| cx.notify()),
        ];
        Self {
            owner,
            build,
            text,
            search,
            stats,
            issues,
            _subscriptions: subscriptions,
        }
    }
    fn insert(&mut self, name: &str, percent: bool, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.text.read(cx).value().to_string();
        let offset = value.len();
        let prefix = if value.is_empty() || value.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        let line = format!("{prefix}+1{} {name}", if percent { "%" } else { "" });
        self.text.update(cx, |state, cx| {
            state.set_selected_range(offset..offset, cx);
            state.replace(line, window, cx);
            let number = offset + prefix.len() + 1;
            state.set_selected_range(number..number + 1, cx);
            state.focus(window, cx);
        });
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.text.read(cx).value().trim_end().to_string();
        let (stats, issues) = parse_custom_text(&text);
        if !issues.is_empty() {
            self.issues = issues;
            cx.notify();
            return;
        }
        let build = self.build.clone();
        let saved = self.owner.update(cx, |owner, cx| {
            if owner.document.build != build {
                return false;
            }
            owner.edit(cx, |snapshot| {
                snapshot.custom_stats = stats;
                snapshot.custom_stats_text = text;
            });
            true
        });
        if matches!(saved, Ok(true)) {
            window.close_dialog(cx);
        } else {
            self.issues = vec![tr("gear.text_edit_changed").into()];
            cx.notify();
        }
    }
}
impl Render for CustomStatsEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let (border, muted, text, accent, negative, background) = (
            p.border,
            p.muted,
            p.text,
            p.accent_hot,
            p.negative,
            p.panel_secondary,
        );
        let ready = self.issues.is_empty();
        let query = self.search.read(cx).value().to_lowercase();
        let mut list = div()
            .id("custom-stat-choices")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        let mut matches = 0;
        for (key, name, percent) in &self.stats {
            let label = crate::stat_labels::stat_name(key, name);
            if !name.to_lowercase().contains(&query)
                && !label.to_lowercase().contains(&query)
                && !key.contains(&query)
            {
                continue;
            }
            matches += 1;
            let insert = name.clone();
            let percent = *percent;
            list = list.child(
                Button::new(SharedString::from(format!("insert-custom-{key}")))
                    .ghost()
                    .w_full()
                    .flex_none()
                    .h(rems(26. / 13.))
                    .border_0()
                    .rounded_none()
                    .justify_start()
                    .text_xs()
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .truncate()
                            .text_left()
                            .child(format!("+1{} {label}", if percent { "%" } else { "" })),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.insert(&insert, percent, window, cx)
                    })),
            );
        }
        if matches == 0 {
            list = list.child(
                div()
                    .p_3()
                    .text_xs()
                    .text_color(muted)
                    .child(tr("gear.no_matching_stats")),
            );
        }
        let mut validation = div()
            .id("custom-stats-validation")
            .max_h(rems(12.))
            .overflow_y_scroll()
            .p_3()
            .text_xs();
        if ready {
            validation = validation
                .text_color(muted)
                .child(tr("planner.config.custom_clear"));
        }
        for issue in &self.issues {
            validation = validation.child(div().mb_2().text_color(negative).child(issue.clone()));
        }
        // Bound the editor inside the dialog's title and padding so Save stays on screen.
        div()
            .h((window.viewport_size().height * 0.88 - window.rem_size() * 8.).max(px(0.)))
            .flex_none()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .text_color(text)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .border_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .border_r_1()
                            .border_color(border)
                            .child(
                                div()
                                    .p_2()
                                    .text_xs()
                                    .text_color(muted)
                                    .child(tr("planner.config.custom_text_hint")),
                            )
                            .child(
                                div().flex_1().min_h_0().child(
                                    Textarea::new(&self.text)
                                        .h_full()
                                        .bordered(false)
                                        .aria_label(tr("planner.config.custom"))
                                        .p_3()
                                        .font_family(theme::MONO_FONT_FAMILY)
                                        .text_sm()
                                        .bg(background),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .w(rems(27.))
                            .max_w(relative(0.42))
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .p_2()
                                    .text_xs()
                                    .text_color(accent)
                                    .child(tr("gear.validation")),
                            )
                            .child(validation)
                            .child(
                                div()
                                    .p_2()
                                    .border_t_1()
                                    .border_color(border)
                                    .text_xs()
                                    .child(tr("planner.config.custom_insert")),
                            )
                            .child(Input::new(&self.search).planner_style(cx))
                            .child(list),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .pt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(if ready { accent } else { muted })
                            .child(if ready {
                                tr("gear.ready_to_save")
                            } else {
                                tr("gear.fix_errors")
                            }),
                    )
                    .child(
                        Button::new("cancel-custom-stats")
                            .planner_style(cx)
                            .label(tr("gear.cancel"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    )
                    .child(
                        Button::new("save-custom-stats")
                            .planner_style(cx)
                            .label(tr("gear.save"))
                            .disabled(!ready)
                            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                    ),
            )
    }
}
