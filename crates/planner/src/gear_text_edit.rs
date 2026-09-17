//! Retained text draft and validation for the item editor; Save updates only the gear draft.
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::{Textarea, TextareaState};
use hsplanner_build::item_text::{self, ParseResult};
use hsplanner_ui::theme;

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
                                                .child(tr("gear.text_edit_heading")),
                                        )
                                        .child(title.clone()),
                                ),
                        )
                        .child(editor.clone()),
                )
        });
    }
}

struct ItemTextEditor {
    original: EquippedItem,
    owner: WeakEntity<GearView>,
    document: DocumentKey,
    text: Entity<TextareaState>,
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
            let mut state = TextareaState::new(window, cx).rows(24);
            state.set_value(value, window, cx);
            state
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
        Self {
            original,
            owner,
            document,
            text,
            search,
            stats: item_text::custom_stats(),
            result,
            pending: false,
            task: None,
            _subscriptions: subscriptions,
        }
    }
    fn validate(&mut self, cx: &mut Context<Self>) {
        self.pending = true;
        let text = self.text.read(cx).value().to_string();
        let original = self.original.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(200))
                .await;
            let result = cx
                .background_spawn(async move { item_text::parse(&text, &original) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.result = result;
                this.pending = false;
                cx.notify();
            });
        }));
        cx.notify();
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
        // Bound the editor viewport inside the dialog's title and padding so
        // multiline input and list intrinsic heights cannot push Save offscreen.
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
                                    .child(tr("gear.text_fields")),
                            )
                            .child(
                                div().flex_1().min_h_0().child(
                                    Textarea::new(&self.text)
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
                            .child(if self.pending {
                                tr("gear.validating")
                            } else if ready {
                                tr("gear.ready_to_save")
                            } else {
                                tr("gear.fix_errors")
                            }),
                    )
                    .child(
                        Button::new("cancel-text-edit")
                            .planner_style(cx)
                            .label(tr("gear.cancel"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    )
                    .child(
                        Button::new("save-text-edit")
                            .planner_style(cx)
                            .label(tr("gear.save"))
                            .disabled(!ready)
                            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                    ),
            )
    }
}
