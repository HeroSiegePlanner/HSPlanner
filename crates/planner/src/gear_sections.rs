//! Quiet-modern building blocks for the gear slot editor, after the reference SectionCard.
use gpui_kit::component::button::Button;
use gpui_kit::{prelude::*, *};
use hsplanner_ui::controls::{self, ButtonSize, ButtonTone, command_button};
use hsplanner_ui::{
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};

pub(crate) fn units(value: f32) -> Rems {
    rems(value / 13.)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    Default,
    Satanic,
    Angelic,
    Set,
}

pub(crate) struct ToneColors {
    pub border: Hsla,
    pub dot: Hsla,
    pub label: Hsla,
    pub bg: Hsla,
}

pub(crate) fn tone_colors(tone: Tone, cx: &App) -> ToneColors {
    let p = cx.global::<TooltipTheme>();
    match tone {
        Tone::Default => ToneColors {
            border: p.border,
            dot: p.accent,
            label: p.text,
            bg: p.panel_secondary,
        },
        Tone::Satanic => ToneColors {
            border: p.negative.opacity(0.25),
            dot: p.negative,
            label: p.negative.opacity(0.9),
            bg: p.negative.opacity(0.06),
        },
        Tone::Angelic => ToneColors {
            border: p.angelic.opacity(0.25),
            dot: p.angelic,
            label: p.angelic.opacity(0.9),
            bg: p.angelic.opacity(0.06),
        },
        Tone::Set => ToneColors {
            border: p.positive.opacity(0.25),
            dot: p.positive,
            label: p.positive.opacity(0.9),
            bg: p.positive.opacity(0.06),
        },
    }
}

pub(crate) struct Section {
    pub id: &'static str,
    pub label: String,
    pub tone: Tone,
    pub default_open: bool,
    pub right: Option<AnyElement>,
    pub body: Option<Div>,
}

impl Section {
    pub(crate) fn new(id: &'static str, label: impl Into<String>, tone: Tone) -> Self {
        Self {
            id,
            label: label.into(),
            tone,
            default_open: false,
            right: None,
            body: None,
        }
    }

    pub(crate) fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub(crate) fn right(mut self, right: impl IntoElement) -> Self {
        self.right = Some(right.into_any_element());
        self
    }

    pub(crate) fn body(mut self, body: Div) -> Self {
        self.body = Some(body);
        self
    }
}

/// Collapsible card: darker header bar with a dot, sentence-case label, summary slot and chevron.
pub(crate) fn section_card(
    section: Section,
    open: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Stateful<Div> {
    let Section {
        id,
        label,
        tone,
        right,
        body,
        ..
    } = section;
    let p = cx.global::<TooltipTheme>();
    let colors = tone_colors(tone, cx);
    div()
        .id(id)
        .flex()
        .flex_col()
        .rounded_lg()
        .border_1()
        .border_color(colors.border)
        .bg(colors.bg)
        .overflow_hidden()
        .child(
            div()
                .id(SharedString::from(format!("{id}-header")))
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .px_4()
                .py_2p5()
                .bg(p.shadow.opacity(0.2))
                .hover(|v| v.bg(p.shadow.opacity(0.1)))
                .cursor_pointer()
                .on_click(on_toggle)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().size(units(6.)).rounded_full().bg(colors.dot))
                        .child(
                            div()
                                .text_size(units(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(colors.label)
                                .child(label),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2p5()
                        .children(right.map(|right| {
                            // Header actions must not toggle the card.
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(right)
                        }))
                        .child(
                            div()
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(units(13.))
                                .line_height(relative(1.))
                                .text_color(p.muted)
                                .child(if open { "▾" } else { "▸" }),
                        ),
                ),
        )
        .when(open, |card| {
            card.children(
                body.map(|body| div().border_t_1().border_color(colors.border).child(body)),
            )
        })
}

pub(crate) fn mono_summary(text: impl Into<String>, color: Hsla) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(10.))
        .text_color(color)
        .truncate()
        .max_w(units(220.))
        .child(text.into())
}

pub(crate) fn chip(text: impl Into<String>, color: Hsla, border: Hsla) -> Div {
    div()
        .px_1()
        .py_px()
        .rounded_sm()
        .border_1()
        .border_color(border)
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(9.))
        .text_color(color)
        .child(text.into())
}

/// Square 20px control for "−", "+", "×", "N", "R"; "×" removes, so its hover reddens.
pub(crate) fn icon_button(id: impl Into<ElementId>, label: &str, cx: &App) -> Button {
    controls::icon_button(id, label, label == "×", cx)
}

pub(crate) fn ghost_button(id: impl Into<ElementId>, label: &str, cx: &App) -> Button {
    command_button(
        id,
        label.to_owned(),
        ButtonTone::Ghost,
        ButtonSize::Small,
        cx,
    )
}

/// Small action in section headers (tr("gear.add"), tr("gear.reset")).
pub(crate) fn header_action(
    id: impl Into<ElementId>,
    label: &str,
    danger: bool,
    cx: &App,
) -> Button {
    let tone = if danger {
        ButtonTone::Danger
    } else {
        ButtonTone::Neutral
    };
    command_button(id, label.to_owned(), tone, ButtonSize::Small, cx)
}

pub(crate) fn row_box(cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .rounded_sm()
        .border_1()
        .border_color(p.accent_deep.opacity(0.15))
        .bg(p.background.opacity(0.4))
        .p_1p5()
}

pub(crate) fn row_index(index: usize, cx: &App) -> Div {
    div()
        .w_3()
        .flex_shrink_0()
        .text_center()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(10.))
        .text_color(cx.global::<TooltipTheme>().faint)
        .child(format!("{}", index + 1))
}

pub(crate) fn hint(id: &'static str, text: &str, cx: &App) -> Div {
    div()
        .pt_0p5()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(9.))
        .line_height(relative(1.4))
        .text_color(cx.global::<TooltipTheme>().faint)
        .child(TooltipText::new(id, text.to_uppercase(), 0.14))
}

pub(crate) fn empty_note(id: &'static str, text: &str, cx: &App) -> Div {
    div()
        .px_3()
        .py_2()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(10.))
        .italic()
        .text_color(cx.global::<TooltipTheme>().faint)
        .child(TooltipText::new(id, text.to_uppercase(), 0.14))
}

pub(crate) fn eyebrow(id: &'static str, text: &str, cx: &App) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(10.))
        .text_color(cx.global::<TooltipTheme>().faint)
        .child(TooltipText::new(id, text.to_uppercase(), 0.18))
}
