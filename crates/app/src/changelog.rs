use gpui_kit::base::Link;
use gpui_kit::component::text::{TextView, TextViewStyle};
use gpui_kit::{prelude::*, *};
use hsplanner_ui::components::{
    ModalSize, PlannerDialogExt, modal_body, modal_eyebrow, modal_footer, modal_header,
    modal_layout,
};
use hsplanner_ui::i18n::tr;
use hsplanner_ui::theme::TooltipTheme;

const NOTES: &str = include_str!("../../../CHANGELOG.md");
const RELEASES: &str = "https://github.com/zium1337/HSPlanner/releases";

pub fn open(window: &mut Window, cx: &mut App) {
    window.open_planner_modal(cx, ModalSize::Large, |dialog, window, cx| {
        let palette = cx.global::<TooltipTheme>();
        let heading_size = window.rem_size() * 1.2;
        dialog.child(
            modal_layout(window)
                .child(modal_header(
                    modal_eyebrow("changelog-eyebrow", tr("changelog.title")),
                    format!("HSPlanner v{}", env!("CARGO_PKG_VERSION")),
                    Some(tr("changelog.history").into()),
                    cx,
                ))
                .child(
                    modal_body("release-notes-scroll")
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
                .child(
                    modal_footer(cx).child(
                        div().flex_1().min_w_0().child(
                            Link::new("github-releases")
                                .child(tr("changelog.github"))
                                .href(RELEASES)
                                .text_color(palette.accent)
                                .underline()
                                .accessibility_label(tr("changelog.github"))
                                .open_with(|url, _, _, cx| cx.open_url(url)),
                        ),
                    ),
                ),
        )
    });
}
