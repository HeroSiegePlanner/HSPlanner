use gpui_kit::base::Selectable;
use gpui_kit::component::{
    Sizable,
    button::Button,
    input::{InputEvent, Textarea, TextareaState},
    text::TextView,
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::session::Session;
use hsplanner_ui::controls::PlannerControl;
use hsplanner_ui::i18n::{Locale, tr};
use hsplanner_ui::scroll::PageScroll;
use hsplanner_ui::theme::TooltipTheme;
use hsplanner_ui::tooltip::CursorTooltipExt;
use std::ops::Range;

/// Expand a UTF-8 selection to its touched lines without including the next
/// line when the selection ends immediately after a newline.
fn prefix_selected_lines(
    text: &str,
    selection: Range<usize>,
    prefix: &str,
) -> (Range<usize>, String) {
    let start = text[..selection.start]
        .rfind('\n')
        .map_or(0, |offset| offset + 1);
    let end_anchor = if !selection.is_empty() && text[..selection.end].ends_with('\n') {
        selection.end - 1
    } else {
        selection.end
    };
    let end = text[end_anchor..]
        .find('\n')
        .map_or(text.len(), |offset| end_anchor + offset);
    let replacement = text[start..end]
        .split('\n')
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    (start..end, replacement)
}

pub struct NotesView {
    session: Entity<Session>,
    editor: Entity<TextareaState>,
    // Notes and their editing history are shared by profiles within one build.
    build_id: Option<String>,
    preview: bool,
    scroll: ScrollHandle,
    preview_scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}
impl NotesView {
    pub fn new(session: Entity<Session>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let build_id = session.read(cx).draft().build_id.clone();
        let markdown = session.read(cx).draft().notes.markdown.clone();
        let editor = cx.new(|cx: &mut Context<TextareaState>| {
            cx.observe_global_in::<Locale>(window, |editor, window, cx| {
                editor.set_placeholder(tr("notes.placeholder"), window, cx);
            })
            .detach();
            let mut state = TextareaState::new(window, cx)
                .rows(24)
                .placeholder(tr("notes.placeholder"));
            state.set_value(markdown, window, cx);
            state
        });
        let subscriptions = vec![
            cx.subscribe(&editor, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let markdown = this.editor.read(cx).value().to_string();
                    this.session.update(cx, |session, cx| {
                        session.edit(|draft| draft.notes.markdown = markdown);
                        cx.notify();
                    });
                }
            }),
            cx.observe_in(&session, window, |this, _, window, cx| {
                let draft = this.session.read(cx).draft();
                let markdown = draft.notes.markdown.clone();
                let changed_build = this.build_id != draft.build_id;
                this.build_id = draft.build_id.clone();
                // set_value also clears the native editor's undo history. A
                // duplicated build can have identical text but must not inherit
                // the previous build's editing history.
                if changed_build || this.editor.read(cx).value().as_ref() != markdown {
                    this.editor
                        .update(cx, |editor, cx| editor.set_value(markdown, window, cx));
                }
                cx.notify();
            }),
        ];
        Self {
            session,
            editor,
            build_id,
            preview: false,
            scroll: ScrollHandle::new(),
            preview_scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }
}
impl NotesView {
    fn format_selection(
        &mut self,
        prefix: &str,
        suffix: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = self.editor.read(cx);
        let block = suffix.is_empty();
        let (range, replacement) = if block {
            prefix_selected_lines(editor.value().as_ref(), editor.selected_range(), prefix)
        } else {
            (
                editor.selected_range(),
                format!("{prefix}{}{suffix}", editor.selected_value()),
            )
        };
        self.editor.update(cx, |editor, cx| {
            if block {
                editor.set_selected_range(range, cx);
            }
            editor.replace(replacement, window, cx);
            editor.focus(window, cx);
        });
        let markdown = self.editor.read(cx).value().to_string();
        self.session.update(cx, |session, cx| {
            session.edit(|draft| draft.notes.markdown = markdown);
            cx.notify();
        });
    }
}

impl Render for NotesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<TooltipTheme>();
        let notes = &self.session.read(cx).draft().notes;
        let markdown = notes.markdown.clone();
        let mut toolbar = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(palette.border)
            .bg(palette.panel);
        for (id, label, title, prefix, suffix) in [
            ("bold", "B", tr("notes.bold"), "**", "**"),
            ("italic", "I", tr("notes.italic"), "*", "*"),
            ("strike", "S", tr("notes.strike"), "~~", "~~"),
            ("heading", "H2", tr("notes.heading"), "## ", ""),
            ("subheading", "H3", tr("notes.subheading"), "### ", ""),
            ("bullet", "•⁝", tr("notes.bullets"), "- ", ""),
            ("numbered", "1.", tr("notes.numbered"), "1. ", ""),
            ("link", "↗", tr("notes.link"), "[", "](https://)"),
            ("code", "</>", tr("notes.code"), "`", "`"),
        ] {
            toolbar = toolbar.child(
                Button::new(id)
                    .planner_style(cx)
                    .h_7()
                    .min_w(rems(1.75))
                    .px_1p5()
                    .font_family(hsplanner_ui::theme::FONT_FAMILY)
                    .text_xs()
                    .label(label)
                    .accessibility_label(title)
                    .cursor_tooltip(title)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.format_selection(prefix, suffix, window, cx)
                    })),
            );
        }
        toolbar = toolbar.child(
            Button::new("notes-preview-toggle")
                .planner_style(cx)
                .small()
                .ml_auto()
                .label(if self.preview {
                    tr("notes.edit")
                } else {
                    tr("notes.preview")
                })
                .selected(self.preview)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.preview = !this.preview;
                    cx.notify();
                })),
        );
        let contents = if self.preview {
            div()
                .id("notes-preview")
                .page_scroll(&self.preview_scroll)
                .size_full()
                .overflow_y_scroll()
                .child(
                    div()
                        .min_h_full()
                        .p_4()
                        .child(TextView::markdown("notes-markdown", markdown)),
                )
                .into_any_element()
        } else {
            div()
                .size_full()
                .child(
                    Textarea::new(&self.editor)
                        .h_full()
                        .bordered(false)
                        .aria_label(tr("notes.aria_label"))
                        .p_4()
                        .text_sm()
                        .line_height(relative(1.625))
                        .font_family(hsplanner_ui::theme::FONT_FAMILY)
                        .bg(palette.panel),
                )
                .into_any_element()
        };
        div()
            .id("notes-view")
            .page_scroll(&self.scroll)
            .size_full()
            .overflow_y_scroll()
            .bg(palette.background)
            .child(
                div().p_6().flex().justify_center().child(
                    div()
                        .w_full()
                        .max_w(rems(56.))
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(hsplanner_ui::components::section_heading(
                            "notes-heading",
                            tr("notes.journal"),
                            tr("notes.title"),
                            cx,
                        ))
                        .child(toolbar)
                        .child(
                            div()
                                .h(rems(24.))
                                .flex_none()
                                .rounded_md()
                                .border_1()
                                .border_color(palette.border)
                                .bg(palette.panel)
                                .child(contents),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .text_size(rems(10. / 13.))
                                .text_color(palette.muted)
                                .child(tr("notes.shared_help"))
                                .when_some(notes.original_html.clone(), |view, html| {
                                    view.child(
                                        Button::new("copy-original-notes")
                                            .planner_style(cx)
                                            .small()
                                            .label(tr("notes.copy_html"))
                                            .on_click(move |_, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    html.clone(),
                                                ))
                                            }),
                                    )
                                }),
                        ),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::prefix_selected_lines;
    use std::ops::Range;

    fn apply(text: &str, selection: Range<usize>, prefix: &str) -> String {
        let (range, replacement) = prefix_selected_lines(text, selection, prefix);
        let mut result = text.to_owned();
        result.replace_range(range, &replacement);
        result
    }

    #[::core::prelude::v1::test]
    fn heading_formats_the_current_line_from_a_middle_caret() {
        assert_eq!(
            apply("intro\nAlpha beta\nend", 12..12, "## "),
            "intro\n## Alpha beta\nend"
        );
    }

    #[::core::prelude::v1::test]
    fn lists_prefix_every_touched_line() {
        assert_eq!(
            apply("intro\none\ntwo\noutro", 8..12, "- "),
            "intro\n- one\n- two\noutro"
        );
        assert_eq!(apply("one\ntwo", 0..7, "1. "), "1. one\n1. two");
    }

    #[::core::prelude::v1::test]
    fn selection_ending_at_next_line_does_not_format_that_line() {
        assert_eq!(apply("one\ntwo", 0..4, "- "), "- one\ntwo");
        assert_eq!(apply("one\ntwo\n", 0..8, "- "), "- one\n- two\n");
    }

    #[::core::prelude::v1::test]
    fn unicode_text_uses_byte_boundaries_and_preserves_unselected_lines() {
        let text = "żółw\n🔥 moc\nkoniec";
        assert_eq!(
            apply(
                text,
                text.find('ó').unwrap()..text.find("koniec").unwrap(),
                "### "
            ),
            "### żółw\n### 🔥 moc\nkoniec"
        );
    }

    #[::core::prelude::v1::test]
    fn caret_can_start_a_block_on_an_empty_line() {
        assert_eq!(apply("", 0..0, "## "), "## ");
        assert_eq!(apply("one\n", 4..4, "- "), "one\n- ");
    }
}
