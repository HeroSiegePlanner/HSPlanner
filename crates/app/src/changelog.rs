use gpui_kit::base::Link;
use gpui_kit::component::{
    WindowExt,
    button::Button,
    text::{TextView, TextViewStyle},
};
use gpui_kit::{prelude::*, *};
use hsplanner_ui::i18n::tr;
use hsplanner_ui::{
    controls::PlannerControl,
    theme::{self, TooltipTheme},
};

const NOTES: &str = include_str!("../../../CHANGELOG.md");
const RELEASES: &str = "https://github.com/zium1337/HSPlanner/releases";

pub fn open(window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, |dialog, window, cx| {
        let palette = cx.global::<TooltipTheme>();
        // Dialog's API takes resolved pixels; derive them from zoom and viewport.
        let width = (window.rem_size() * 49.).min(window.bounds().size.width * 0.92);
        let content_height = window.bounds().size.height * 0.58;
        let heading_size = window.rem_size() * 1.2;
        dialog
            .width(width)
            .title(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_color(palette.accent)
                            .child(tr("changelog.title")),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(palette.accent)
                            .child(format!("HSPlanner v{}", env!("CARGO_PKG_VERSION"))),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::NORMAL)
                            .text_color(palette.muted)
                            .child(tr("changelog.history")),
                    ),
            )
            .child(
                div()
                    .id("release-notes-scroll")
                    .max_h(content_height)
                    .min_h_0()
                    .overflow_y_scroll()
                    .text_sm()
                    .text_color(palette.text)
                    .child(
                        TextView::markdown("release-notes", NOTES)
                            .selectable(true)
                            .style(
                                TextViewStyle::default()
                                    .paragraph_gap(rems(0.65))
                                    .heading_font_size(move |_, _| heading_size),
                            ),
                    ),
            )
            .footer(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        Link::new("github-releases")
                            .child(tr("changelog.github"))
                            .href(RELEASES)
                            .text_color(palette.accent)
                            .underline()
                            .accessibility_label(tr("changelog.github"))
                            .open_with(|url, _, _, cx| cx.open_url(url)),
                    )
                    .child(
                        Button::new("close-changelog")
                            .planner_style(cx)
                            .label(tr("common.close"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    ),
            )
    });
}
