//! Shared control surfaces from the shipping planner; GPUI retains interaction behavior.
use crate::theme::{self, TooltipTheme};
use crate::tooltip_text::TooltipText;
use gpui_kit::base::Selectable;
use gpui_kit::component::{
    Sizable,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    input::{Input, Textarea},
    select::{Select, SelectDelegate, SelectItem},
};
use gpui_kit::{prelude::*, *};

pub trait PlannerControl: Sized {
    fn planner_style(self, cx: &App) -> Self;
}

/// Neutral = ordinary command, Primary = the one commit of a decision area,
/// Danger = destructive commit, Ghost = quiet inline/toolbar action without a border.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonTone {
    #[default]
    Neutral,
    Primary,
    Danger,
    Ghost,
}

/// Compact = 20px glyph square, Small = 24px toolbar/row control, Regular = 28px dialog action.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Compact,
    #[default]
    Small,
    Regular,
}

/// The one definition of how a planner command button looks. Size and layout stay
/// with the caller (`button_size`, `.w_full()`); surfaces that need their own colours
/// call `.custom(...)` after `planner_style`.
pub fn button_look(button: Button, tone: ButtonTone, cx: &App) -> Button {
    let p = cx.global::<TooltipTheme>();
    let clear = p.background.opacity(0.);
    let (foreground, border, background, hover) = match tone {
        ButtonTone::Neutral => (p.muted, p.border, clear, p.panel_secondary),
        ButtonTone::Primary => (
            p.accent_hot,
            p.accent_deep,
            p.accent_hot.opacity(0.1),
            p.accent_hot.opacity(0.15),
        ),
        ButtonTone::Danger => (
            p.negative,
            p.negative.opacity(0.4),
            clear,
            p.negative.opacity(0.12),
        ),
        ButtonTone::Ghost => (p.muted, clear, clear, p.panel_secondary),
    };
    button
        .custom(
            ButtonCustomVariant::new(cx)
                .color(background)
                .foreground(foreground)
                .hover(hover)
                .active(hover),
        )
        .rounded_md()
        .border_1()
        .border_color(border)
        .font_family(theme::FONT_FAMILY)
        .text_size(rems(12. / 13.))
}

pub fn planner_button(id: impl Into<ElementId>, tone: ButtonTone, cx: &App) -> Button {
    button_look(Button::new(id), tone, cx)
}

impl PlannerControl for Button {
    fn planner_style(self, cx: &App) -> Self {
        button_look(self, ButtonTone::Neutral, cx)
    }
}

fn field_look<T: Styled>(control: T, cx: &App) -> T {
    let palette = cx.global::<TooltipTheme>();
    control
        .rounded_sm()
        .border_color(palette.border_strong)
        .font_family(theme::FONT_FAMILY)
        .bg(linear_gradient(
            180.,
            linear_color_stop(palette.background, 0.),
            linear_color_stop(palette.panel_secondary, 1.),
        ))
        .shadow(vec![BoxShadow {
            color: palette.shadow.opacity(0.5),
            offset: point(px(0.), px(1.)),
            blur_radius: px(2.),
            spread_radius: px(0.),
            inset: true,
        }])
}

impl PlannerControl for Input {
    fn planner_style(self, cx: &App) -> Self {
        field_look(self, cx)
    }
}

impl PlannerControl for Textarea {
    fn planner_style(self, cx: &App) -> Self {
        field_look(self, cx)
    }
}

pub fn button_size(button: Button, size: ButtonSize) -> Button {
    match size {
        ButtonSize::Compact => button.xsmall().size_5().p_0().line_height(relative(1.)),
        ButtonSize::Small => button.small(),
        ButtonSize::Regular => button.small().h_7().px_3(),
    }
}

/// Every labelled command button: tone by meaning, size by placement, Inter sentence case.
pub fn command_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    tone: ButtonTone,
    size: ButtonSize,
    cx: &App,
) -> Button {
    button_size(planner_button(id, tone, cx), size).label(label)
}

/// Dialog footer and form action.
pub fn modal_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    tone: ButtonTone,
    cx: &App,
) -> Button {
    command_button(id, label, tone, ButtonSize::Regular, cx)
}

/// One pill of a group with persistent selection: picker tabs, filter chips, toggles.
pub fn segment(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    active: bool,
    cx: &App,
) -> Button {
    let tone = if active {
        ButtonTone::Primary
    } else {
        ButtonTone::Neutral
    };
    command_button(id, label, tone, ButtonSize::Small, cx).selected(active)
}

/// Underlined section tab; mono caps because tabs are navigation, not commands.
pub fn tab(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    active: bool,
    cx: &App,
) -> Button {
    let p = cx.global::<TooltipTheme>();
    let clear = p.background.opacity(0.);
    let color = if active { p.accent_hot } else { p.faint };
    let id = id.into();
    let label = label.into();
    let label_id = ElementId::NamedChild(std::sync::Arc::new(id.clone()), "label".into());
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(if active {
                    p.accent_hot.opacity(0.04)
                } else {
                    clear
                })
                .foreground(color)
                .hover(p.accent_hot.opacity(0.04))
                .active(clear),
        )
        .flex_1()
        .h_auto()
        .py_3p5()
        .justify_center()
        .rounded_none()
        .border_0()
        .border_b_2()
        .border_color(if active { p.accent_hot } else { clear })
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(rems(11. / 13.))
        .font_weight(FontWeight::SEMIBOLD)
        .selected(active)
        .accessibility_label(label.clone())
        .child(div().text_color(color).child(TooltipText::new(
            label_id,
            label.to_uppercase(),
            0.18,
        )))
}

/// 20px glyph control ("×", "−", "+"); `danger` reddens the hover for removals.
pub fn icon_button(id: impl Into<ElementId>, glyph: &str, danger: bool, cx: &App) -> Button {
    let p = cx.global::<TooltipTheme>();
    let button = command_button(
        id,
        glyph.to_owned(),
        ButtonTone::Ghost,
        ButtonSize::Compact,
        cx,
    );
    if !danger {
        return button;
    }
    let clear = p.background.opacity(0.);
    button.custom(
        ButtonCustomVariant::new(cx)
            .color(clear)
            .foreground(p.muted)
            .hover(p.negative.opacity(0.15))
            .active(p.negative.opacity(0.2)),
    )
}

impl<D: SelectDelegate + 'static> PlannerControl for Select<D>
where
    <D::Item as SelectItem>::Value: PartialEq + Clone,
{
    fn planner_style(self, cx: &App) -> Self {
        let palette = cx.global::<TooltipTheme>();
        self.h(rems(37.5 / 13.))
            .px(rems(12. / 13.))
            .py(rems(8. / 13.))
            .text_size(rems(1.))
            .line_height(relative(1.5))
            .font_family(theme::FONT_FAMILY)
            .font_weight(FontWeight::NORMAL)
            .rounded(rems(6. / 13.))
            .border_color(palette.border)
            .bg(palette.background.opacity(0.6))
    }
}

/// Planner range control; callers own focus and key bindings.
/// The base parts preserve pointer dragging and accessibility increment/decrement actions.
pub fn planner_slider(
    state: &Entity<gpui_kit::component::slider::SliderState>,
    window: &Window,
    cx: &App,
) -> gpui_kit::base::Slider {
    use gpui_kit::base::{Slider, SliderIndicator, SliderThumb, SliderTrack};

    let p = cx.global::<TooltipTheme>();
    let percentage = state.read(cx).percentage().end;
    let zoom = window.rem_size() / px(13.);
    let thumb_shadow = |blur: f32| {
        vec![
            BoxShadow {
                color: rgba(0xffffff0f).into(),
                offset: point(px(0.), px(1.) * zoom),
                blur_radius: px(0.),
                spread_radius: px(0.),
                inset: true,
            },
            BoxShadow {
                color: p.shadow.opacity(0.7),
                offset: point(px(0.), px(1.) * zoom),
                blur_radius: px(3.) * zoom,
                spread_radius: px(0.),
                inset: false,
            },
            BoxShadow {
                color: p.accent_hot.opacity(0.25),
                offset: point(px(0.), px(0.)),
                blur_radius: px(blur) * zoom,
                spread_radius: px(0.),
                inset: false,
            },
        ]
    };
    Slider::new(state)
        .flex_none()
        .h(rems(16. / 13.))
        .w_full()
        .child(
            SliderTrack::new(state)
                .flex()
                .items_center()
                .size_full()
                .px(rems(8. / 13.))
                .cursor_pointer()
                .child(
                    // The thumb stays inside the range's measured outer width.
                    SliderIndicator::new(state)
                        .relative()
                        .w_full()
                        .h(rems(4. / 13.))
                        .child(
                            div()
                                .absolute()
                                .left(rems(-8. / 13.))
                                .right(rems(-8. / 13.))
                                .top_0()
                                .bottom_0()
                                .rounded(rems(2. / 13.))
                                .border(rems(1. / 13.))
                                .border_color(p.border)
                                .bg(linear_gradient(
                                    180.,
                                    linear_color_stop(p.background, 0.),
                                    linear_color_stop(p.panel_secondary, 1.),
                                ))
                                .overflow_hidden()
                                .child(div().h_full().w(relative(percentage)).bg(linear_gradient(
                                    90.,
                                    linear_color_stop(p.accent_deep, 0.),
                                    linear_color_stop(p.accent_hot, 1.),
                                )))
                                .shadow(vec![BoxShadow {
                                    color: p.shadow.opacity(0.6),
                                    offset: point(px(0.), px(1.) * zoom),
                                    blur_radius: px(2.) * zoom,
                                    spread_radius: px(0.),
                                    inset: true,
                                }]),
                        )
                        .child(
                            SliderThumb::new(state)
                                .absolute()
                                .left(relative(percentage))
                                .ml(rems(-8. / 13.))
                                .top(rems(-6. / 13.))
                                .size(rems(16. / 13.))
                                .rounded_full()
                                .border(rems(1.5 / 13.))
                                .border_color(p.accent_deep)
                                .bg(linear_gradient(
                                    180.,
                                    linear_color_stop(rgb(0x3a2e18), 0.),
                                    linear_color_stop(rgb(0x2a2418), 1.),
                                ))
                                .shadow(thumb_shadow(6.))
                                .hover(|style| {
                                    style.border_color(p.accent_hot).shadow(thumb_shadow(12.))
                                }),
                        ),
                ),
        )
}
