//! Retained text draft and validation for the item editor; Save updates only the gear draft.
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::{
    Editor, EditorState, GutterDecoration, GutterDecorationCollection, OverviewRulerDecoration,
    OverviewRulerDecorationCollection, OverviewRulerLane,
};
use hsplanner_build::item_text::{self, ParseResult};
use hsplanner_ui::components::{
    ModalSize, PlannerDialogExt, modal_eyebrow, modal_footer, modal_header, modal_height,
    modal_layout, modal_status,
};
use hsplanner_ui::controls::{ButtonTone, modal_button};
use hsplanner_ui::theme;
use std::ops::Range;

impl GearView {
    pub(super) fn open_text_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(original) = self.candidate.clone() else {
            return;
        };
        let title =
            data::get_item(&original.base_id).map_or(original.base_id.clone(), |b| b.name.clone());
        let owner = cx.entity().downgrade();
        let document = DocumentKey::from_session(self.session.read(cx));
        let editor = cx.new(|cx| ItemTextEditor::new(original, owner, document, window, cx));
        window.open_planner_modal(cx, ModalSize::Editor, move |dialog, window, cx| {
            dialog
                .h(modal_height(window))
                .overlay_closable(false)
                .on_ok(|_, _, _| false)
                .child(
                    modal_layout(window)
                        .child(modal_header(
                            modal_eyebrow("item-text-eyebrow", tr("gear.text_edit_heading")),
                            title.clone(),
                            None,
                            cx,
                        ))
                        .child(editor.clone()),
                )
        });
    }
}

struct ItemTextEditor {
    original: EquippedItem,
    owner: WeakEntity<GearView>,
    document: DocumentKey,
    text: Entity<EditorState>,
    gutter: GutterDecorationCollection,
    ruler: OverviewRulerDecorationCollection,
    search: Entity<InputState>,
    stats: Vec<(String, String, bool)>,
    result: ParseResult,
    pending: bool,
    task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}
impl ItemTextEditor {
    fn new(
        original: EquippedItem,
        owner: WeakEntity<GearView>,
        document: DocumentKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let value = item_text::serialize(&original).unwrap_or_default();
        let result = item_text::parse(&value, &original);
        let text = cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .soft_wrap(true)
                .folding(false)
                .indent_guides(false);
            state.set_value(value.clone(), window, cx);
            state
        });
        let (gutter, ruler) = text.update(cx, |state, cx| {
            (
                state.create_gutter_decorations_collection(Vec::new(), cx),
                state.create_overview_ruler_decorations_collection(Vec::new(), cx),
            )
        });
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("gear.search_custom_affixes")));
        let subscriptions = vec![
            cx.observe_global_in::<hsplanner_ui::i18n::Locale>(window, |this, window, cx| {
                this.search.update(cx, |input, cx| {
                    input.set_placeholder(tr("gear.search_custom_affixes"), window, cx)
                });
                cx.notify();
            }),
            cx.subscribe(&text, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.validate(cx);
                }
            }),
            cx.subscribe(&search, |_, _, _: &InputEvent, cx| cx.notify()),
        ];
        let editor = Self {
            original,
            owner,
            document,
            text,
            gutter,
            ruler,
            search,
            stats: item_text::custom_stats(),
            result,
            pending: false,
            task: None,
            _subscriptions: subscriptions,
        };
        editor.mark_diagnostics(&value, cx);
        editor
    }
    fn validate(&mut self, cx: &mut Context<Self>) {
        self.pending = true;
        let text = self.text.read(cx).value().to_string();
        let original = self.original.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(200))
                .await;
            let parsed = text.clone();
            let result = cx
                .background_spawn(async move { item_text::parse(&parsed, &original) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.result = result;
                this.pending = false;
                // Line offsets belong to this text; a newer edit validates again.
                if this.text.read(cx).value().as_ref() == text {
                    this.mark_diagnostics(&text, cx);
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    /// Colours the gutter and the scrollbar beside each line a diagnostic names.
    fn mark_diagnostics(&self, text: &str, cx: &mut App) {
        let p = cx.global::<TooltipTheme>();
        let (warning, error) = (p.accent_hot, p.negative);
        let (gutter, ruler) = diagnostic_marks(text, &self.result.diagnostics)
            .into_iter()
            .map(|(range, is_warning)| {
                let color = if is_warning { warning } else { error };
                (
                    GutterDecoration::new(range.clone()).with_color(color),
                    OverviewRulerDecoration::new(range)
                        .with_color(color)
                        .with_lane(OverviewRulerLane::Full),
                )
            })
            .unzip();
        self.gutter.set(gutter, cx);
        self.ruler.set(ruler, cx);
    }
    fn insert(&mut self, name: &str, percent: bool, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.text.read(cx).value().to_string();
        let start = value.find("Implicit:\n").map(|p| p + "Implicit:\n".len());
        let (offset, prefix) = match start {
            Some(start) => (
                value[start..]
                    .find("--------")
                    .map_or(value.len(), |p| start + p),
                "",
            ),
            None => (value.len(), "\n--------\nImplicit:\n"),
        };
        let line = format!(
            "{prefix}+1{} {name} [custom]\n",
            if percent { "%" } else { "" }
        );
        self.text.update(cx, |state, cx| {
            state.set_selected_range(offset..offset, cx);
            state.replace(line, window, cx);
            let number = offset + prefix.len() + 1;
            state.set_selected_range(number..number + 1, cx);
            state.focus(window, cx);
        });
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending {
            return;
        }
        let Some(item) = self.result.item.clone() else {
            return;
        };
        let result = self.owner.update(cx, |owner, cx| {
            if DocumentKey::from_session(owner.session.read(cx)) != self.document
                || !owner.editing
                || owner
                    .candidate
                    .as_ref()
                    .is_none_or(|c| c.base_id != self.original.base_id)
            {
                return false;
            }
            owner.candidate = Some(item);
            owner.sliders.clear();
            owner.load_icon();
            owner.changed(cx);
            true
        });
        if matches!(result, Ok(true)) {
            window.close_dialog(cx)
        } else {
            self.result.item = None;
            self.result.diagnostics.push(item_text::Diagnostic {
                line: 0,
                message: tr("gear.text_edit_changed").into(),
                warning: false,
            });
            cx.notify();
        }
    }
}
impl Render for ItemTextEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let (border, muted, text, accent, negative, background) = (
            p.border,
            p.muted,
            p.text,
            p.accent_hot,
            p.negative,
            p.panel_secondary,
        );
        let ready = !self.pending && self.result.item.is_some();
        let query = self.search.read(cx).value().to_lowercase();
        let mut list = div()
            .id("custom-affix-choices")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        let mut matches = 0;
        for (key, name, percent) in &self.stats {
            if !name.to_lowercase().contains(&query) && !key.contains(&query) {
                continue;
            }
            matches += 1;
            let name = name.clone();
            let insert = name.clone();
            let percent = *percent;
            list = list.child(
                Button::new(SharedString::from(format!("insert-{key}")))
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
                            .child(format!(
                                "+1{} {name} [custom]",
                                if percent { "%" } else { "" }
                            )),
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
            .id("item-text-validation")
            .max_h(rems(12.))
            .overflow_y_scroll()
            .p_3()
            .text_xs();
        if self.pending {
            validation = validation.child(tr("gear.validating"))
        } else if self.result.diagnostics.is_empty() {
            validation = validation
                .text_color(muted)
                .child(tr("gear.validation_clear"))
        }
        for d in &self.result.diagnostics {
            validation = validation.child(
                div()
                    .mb_2()
                    .text_color(if d.warning { accent } else { negative })
                    .child(trf(
                        if d.warning {
                            "gear.warning_line"
                        } else {
                            "gear.error_line"
                        },
                        &[("line", d.line.to_string()), ("message", d.message.clone())],
                    )),
            );
        }
        // The fixed modal height bounds the textarea and list, so Save stays in the footer.
        div()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .text_color(text)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .mx_6()
                    .my_4()
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
                                    .child(tr("gear.text_fields")),
                            )
                            .child(
                                div().flex_1().min_h_0().child(
                                    Editor::new(&self.text)
                                        .h_full()
                                        .bordered(false)
                                        .aria_label(tr("gear.item_text"))
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
                                    .child(tr("gear.custom_affixes_insert")),
                            )
                            .child(Input::new(&self.search).planner_style(cx))
                            .child(list),
                    ),
            )
            .child(
                modal_footer(cx)
                    .child(modal_status(
                        if self.pending {
                            tr("gear.validating")
                        } else if ready {
                            tr("gear.ready_to_save")
                        } else {
                            tr("gear.fix_errors")
                        },
                        if ready { accent } else { muted },
                    ))
                    .child(
                        modal_button("save-text-edit", tr("gear.save"), ButtonTone::Primary, cx)
                            .disabled(!ready)
                            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                    ),
            )
    }
}

/// Byte range of the 1-based line `number`, counted the way `str::lines` counts it.
fn line_range(text: &str, number: usize) -> Option<Range<usize>> {
    let mut start = 0;
    for (index, segment) in text.split_inclusive('\n').enumerate() {
        if index + 1 == number {
            let line = segment
                .strip_suffix('\n')
                .map_or(segment, |line| line.strip_suffix('\r').unwrap_or(line));
            return Some(start..start + line.len());
        }
        start += segment.len();
    }
    None
}

/// The line range and warning flag of every diagnostic that names a line of `text`.
fn diagnostic_marks(
    text: &str,
    diagnostics: &[item_text::Diagnostic],
) -> Vec<(Range<usize>, bool)> {
    diagnostics
        .iter()
        .filter_map(|d| Some((line_range(text, d.line)?, d.warning)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic(line: usize, warning: bool) -> item_text::Diagnostic {
        item_text::Diagnostic {
            line,
            message: String::new(),
            warning,
        }
    }

    #[::core::prelude::v1::test]
    fn diagnostics_mark_the_lines_they_name() {
        let text = "Name\r\n--------\n+5% Damage\n";
        assert_eq!(line_range(text, 1), Some(0..4));
        assert_eq!(line_range(text, 3), Some(15..25));
        assert_eq!(line_range(text, 4), None);
        assert_eq!(
            diagnostic_marks(
                text,
                &[
                    diagnostic(0, false),
                    diagnostic(3, true),
                    diagnostic(9, false)
                ]
            ),
            vec![(15..25, true)]
        );
    }
}
