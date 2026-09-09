//! Repeated presentation from the Tauri reference. Callers retain behavior and state.
use crate::{
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};
use gpui_kit::{prelude::*, *};

pub fn section_heading(
    id: impl Into<ElementId>,
    eyebrow: impl Into<SharedString>,
    title: impl Into<SharedString>,
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    let id = id.into();
    let title_id = ElementId::NamedChild(std::sync::Arc::new(id.clone()), "title".into());
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(10. / 13.))
                .text_color(p.faint)
                .child(div().text_color(p.accent_hot).child("◆"))
                .child(TooltipText::new(id, eyebrow.into().to_uppercase(), 0.18)),
        )
        .child(
            div()
                .text_size(rems(22. / 13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.accent_hot)
                .child(TooltipText::new(title_id, title, 0.02)),
        )
}

pub fn panel(id: impl Into<ElementId>, title: impl Into<SharedString>, cx: &App) -> Div {
    panel_with_heading_style(id, title, None, FontWeight::SEMIBOLD, cx)
}

/// A reference panel whose header reserves space for counts or contextual actions.
pub fn panel_with_trailing(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    trailing: impl IntoElement,
    cx: &App,
) -> Div {
    panel_with_heading_style(
        id,
        title,
        Some(trailing.into_any_element()),
        FontWeight::SEMIBOLD,
        cx,
    )
}

/// Character summaries use a regular heading; editable reference panels use semibold.
pub fn panel_with_heading_style(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    trailing: Option<AnyElement>,
    heading_weight: FontWeight,
    cx: &App,
) -> Div {
    let (id, title) = (id.into(), title.into());
    let p = cx.global::<TooltipTheme>();
    div()
        .relative()
        .overflow_hidden()
        .min_w_0()
        .rounded_md()
        .border_1()
        .border_color(p.border)
        .p_4()
        .bg(linear_gradient(
            180.,
            linear_color_stop(p.panel, 0.),
            linear_color_stop(p.background, 1.),
        ))
        .child(corner_marks(cx))
        .child(
            div()
                .mb_3()
                .pb_2()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .border_b_1()
                .border_color(p.accent_deep.opacity(0.2))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap_2()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(rems(10. / 13.))
                        .font_weight(heading_weight)
                        .text_color(p.accent_hot.opacity(0.7))
                        .child("◆")
                        .child(TooltipText::new(id, title.to_uppercase(), 0.18)),
                )
                .when_some(trailing, |view, trailing| {
                    view.child(div().flex_shrink_0().child(trailing))
                }),
        )
}

/// The four short accent strokes used by the reference Panel and Character cards.
/// The one-pixel offset aligns the strokes with the containing panel's border.
pub fn corner_marks(cx: &App) -> Div {
    let color = cx.global::<TooltipTheme>().accent_deep.opacity(0.45);
    let mark = || div().absolute().size(rems(8. / 13.)).border_color(color);
    div()
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .left_0()
        .child(mark().top(px(-1.)).left(px(-1.)).border_t_1().border_l_1())
        .child(mark().top(px(-1.)).right(px(-1.)).border_t_1().border_r_1())
        .child(
            mark()
                .bottom(px(-1.))
                .left(px(-1.))
                .border_b_1()
                .border_l_1(),
        )
        .child(
            mark()
                .bottom(px(-1.))
                .right(px(-1.))
                .border_b_1()
                .border_r_1(),
        )
}

pub fn stat_row(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    color: Hsla,
    cx: &App,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .py_1()
        .text_size(rems(11. / 13.))
        .child(
            div()
                .text_color(cx.global::<TooltipTheme>().muted)
                .child(label.into()),
        )
        .child(
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_color(color)
                .child(value.into()),
        )
}
