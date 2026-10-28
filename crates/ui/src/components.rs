//! Shared presentation blocks; callers keep behavior and state.
use crate::{
    controls::{ButtonTone, PlannerControl, modal_button},
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};
use gpui_kit::component::{
    WindowExt,
    dialog::{Confirm, Dialog, DialogEntrance, DialogTitle},
    input::{Input, InputState},
};
use gpui_kit::{prelude::*, *};

/// Every planner dialog fades in and settles briefly instead of sliding down from the top.
pub const DIALOG_ENTRANCE: DialogEntrance = DialogEntrance::FadeSlide;

/// Modal widths in planner pixels (13 px rem), chosen by the decision the modal holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModalSize {
    /// One short field: save as, rename, new folder.
    Form,
    /// A panel of settings, notes or results.
    Standard,
    /// A board or a list with details: skill subtree, jewelry, changelog.
    Large,
    /// A text editor with a reference column.
    Editor,
    /// Content that sets its own width, such as the gear slot picker.
    Custom(f32),
}

impl ModalSize {
    fn width(self) -> f32 {
        match self {
            Self::Form => 448.,
            Self::Standard => 560.,
            Self::Large => 680.,
            Self::Editor => 1100.,
            Self::Custom(width) => width,
        }
    }
}

pub trait PlannerDialogExt {
    /// `WindowExt::open_dialog` with the shared planner entrance.
    fn open_planner_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;

    /// A planner dialog on the shared modal surface (see [`modal_frame`]).
    fn open_planner_modal<F>(&mut self, cx: &mut App, size: ModalSize, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;
}

impl PlannerDialogExt for Window {
    fn open_planner_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        self.open_dialog(cx, move |dialog, window, cx| {
            build(dialog.entrance(DIALOG_ENTRANCE), window, cx)
        });
    }

    fn open_planner_modal<F>(&mut self, cx: &mut App, size: ModalSize, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        self.open_planner_dialog(cx, move |dialog, window, cx| {
            build(modal_frame(dialog, size, window, cx), window, cx)
        });
    }
}

/// Share of the window above a modal, and the most of its height a modal takes.
const MODAL_TOP: f32 = 0.06;
const MODAL_HEIGHT: f32 = 0.88;

/// The shared modal surface: no stock padding (header, body and footer bring their own),
/// the panel gradient, a width by size and the top at 6% of the window. Dismissal is
/// Dialog's own close button and Escape; footers hold only real actions.
pub fn modal_frame(dialog: Dialog, size: ModalSize, window: &Window, cx: &App) -> Dialog {
    let p = cx.global::<TooltipTheme>();
    let viewport = window.viewport_size();
    dialog
        .p_0()
        .gap_0()
        .rounded_xl()
        .bg(linear_gradient(
            180.,
            linear_color_stop(p.panel_secondary, 0.),
            linear_color_stop(p.background, 1.),
        ))
        .width((window.rem_size() * (size.width() / 13.)).min(viewport.width * 0.94))
        .margin_top(viewport.height * MODAL_TOP)
}

/// Height of a modal that keeps one size while its content changes (editors, pickers).
pub fn modal_height(window: &Window) -> Pixels {
    window.viewport_size().height * MODAL_HEIGHT
}

/// The header, body and footer column of a modal. Dialog applies its own `max_h` after
/// the caller's style, so the cap lives here: a long body scrolls between a header and
/// a footer that stay in view.
pub fn modal_layout(window: &Window) -> Div {
    div()
        .size_full()
        .max_h(modal_height(window))
        .flex()
        .flex_col()
}

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

/// Modal eyebrow: mono caps with tracking; callers append accent spans.
pub fn modal_eyebrow(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Div {
    div().flex().items_center().gap_1().child(TooltipText::new(
        id,
        text.into().to_uppercase(),
        0.12,
    ))
}

/// Modal header: accent dot + eyebrow, 17px title, optional muted subtitle.
/// Right padding leaves room for Dialog's built-in close button.
pub fn modal_header(
    eyebrow: impl IntoElement,
    title: impl Into<SharedString>,
    subtitle: Option<SharedString>,
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .flex_none()
        .px_6()
        .py_4()
        .pr_10()
        .border_b_1()
        .border_color(p.border)
        .font_family(theme::FONT_FAMILY)
        .child(
            div()
                .mb_1p5()
                .flex()
                .items_center()
                .gap_2()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(10. / 13.))
                .text_color(p.faint)
                .child(div().size_1().flex_none().rounded_full().bg(p.accent))
                .child(eyebrow),
        )
        .child(
            DialogTitle::new()
                .text_size(rems(17. / 13.))
                .font_weight(FontWeight::SEMIBOLD)
                .line_height(relative(1.5))
                .text_color(p.text)
                .child(title.into()),
        )
        .when_some(subtitle, |view, subtitle| {
            view.child(
                div()
                    .mt_1()
                    .text_size(rems(12. / 13.))
                    .text_color(p.muted)
                    .child(subtitle),
            )
        })
}

/// LABEL_CLASS: mono caps field label.
pub fn modal_label(id: impl Into<ElementId>, text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(rems(10. / 13.))
        .text_color(cx.global::<TooltipTheme>().faint)
        .child(TooltipText::new(id, text.into().to_uppercase(), 0.18))
}

/// Scrolling modal content between the header and the footer.
pub fn modal_body(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .px_6()
        .py_4()
        .flex()
        .flex_col()
        .gap_3()
}

/// One named field (save as, rename, new folder). The button and Enter both send
/// Dialog's Confirm, so the caller's `on_ok` validates and keeps the dialog open on error.
pub fn name_form(
    eyebrow: impl Into<SharedString>,
    title: impl Into<SharedString>,
    label: impl Into<SharedString>,
    input: &Entity<InputState>,
    error: Option<SharedString>,
    commit: impl Into<SharedString>,
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(modal_header(
            modal_eyebrow("name-form-eyebrow", eyebrow),
            title,
            None,
            cx,
        ))
        .child(
            modal_body("name-form-body")
                .gap_2()
                .child(modal_label("name-form-label", label, cx))
                .child(Input::new(input).planner_style(cx))
                .children(error.map(|error| {
                    div()
                        .text_size(rems(12. / 13.))
                        .text_color(p.negative)
                        .child(error)
                })),
        )
        .child(modal_footer(cx).child(
            modal_button("name-form-commit", commit, ButtonTone::Primary, cx).on_click(
                |_, window, cx| window.dispatch_action(Box::new(Confirm { secondary: false }), cx),
            ),
        ))
}

/// MODAL_FOOTER_CLASS: top border, actions right-aligned; a `modal_status` child fills the left.
pub fn modal_footer(cx: &App) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .px_6()
        .py_3()
        .border_t_1()
        .border_color(cx.global::<TooltipTheme>().border)
}

pub fn modal_status(text: impl Into<SharedString>, color: Hsla) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .truncate()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(rems(11. / 13.))
        .text_color(color)
        .child(text.into())
}
