use crate::release_notes::Section;
use crate::update::{Download, State, Update, Updater, current_version};
use gpui_kit::base::{Disableable, Link};
use gpui_kit::component::{button::Button, progress::Progress};
use gpui_kit::{prelude::*, *};
use hsplanner_ui::components::{
    ModalSize, PlannerDialogExt, modal_eyebrow, modal_footer, modal_header, modal_label,
};
use hsplanner_ui::controls::{ButtonTone, modal_button};
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::theme::{self, TooltipTheme};

const MEBIBYTE: f64 = 1024. * 1024.;

/// Opens the update dialog; a dialog with nothing to show starts a fresh check.
pub fn open(updater: Entity<Updater>, window: &mut Window, cx: &mut App) {
    updater.update(cx, |updater, cx| {
        if updater.state.update().is_none() && !updater.state.busy() {
            updater.check(false, cx);
        }
    });
    window.open_planner_modal(cx, ModalSize::Standard, move |dialog, window, cx| {
        dialog.child(panel(&updater, window.viewport_size().height * 0.5, cx))
    });
}

fn panel(updater: &Entity<Updater>, notes_height: Pixels, cx: &App) -> Div {
    let state = updater.read(cx).state.clone();
    let current = current_version().to_string();
    let (title, subtitle) = match (&state, state.update()) {
        (_, Some(update)) => (
            trf(
                "update.version_available",
                &[("version", update.version.to_string())],
            ),
            trf("update.running_version", &[("version", current)]),
        ),
        (State::UpToDate, None) => (
            tr("update.up_to_date").to_string(),
            trf("update.latest_hint", &[("version", current)]),
        ),
        (State::Failed { .. }, None) => (
            tr("update.check_failed_title").to_string(),
            trf("update.running_version", &[("version", current)]),
        ),
        _ => (
            tr("update.checking_title").to_string(),
            trf("update.running_version", &[("version", current)]),
        ),
    };
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(modal_header(
            modal_eyebrow("update-eyebrow", tr("update.title")),
            title,
            Some(subtitle.into()),
            cx,
        ))
        .when_some(state.update(), |view, update| {
            view.child(notes(&update.notes, notes_height, cx))
        })
        .child(footer(updater, &state, cx))
}

fn notes(sections: &[Section], max_height: Pixels, cx: &App) -> Stateful<Div> {
    let palette = cx.global::<TooltipTheme>();
    let body = div()
        .id("update-notes")
        .max_h(max_height)
        .min_h_0()
        .overflow_y_scroll()
        .px_6()
        .py_5()
        .flex()
        .flex_col()
        .gap_5()
        .text_sm()
        .text_color(palette.text);
    if sections.is_empty() {
        return body.child(div().text_color(palette.muted).child(tr("update.no_notes")));
    }
    body.children(sections.iter().enumerate().map(|(index, section)| {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .when_some(section.title.clone(), |view, title| {
                view.child(modal_label(
                    SharedString::from(format!("update-section-{index}")),
                    title,
                    cx,
                ))
            })
            .children(section.items.iter().map(|item| {
                div()
                    .flex()
                    .gap_2()
                    .line_height(relative(1.45))
                    .child(div().flex_none().text_color(palette.accent).child("•"))
                    .child(div().flex_1().min_w_0().child(item.clone()))
            }))
    }))
}

fn footer(updater: &Entity<Updater>, state: &State, cx: &App) -> Div {
    let palette = cx.global::<TooltipTheme>();
    let status = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_1p5()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(rems(11. / 13.));
    let status = match state {
        State::Downloading { progress, .. } => {
            status.child(progress_bar(progress.percent(), cx)).child(
                div()
                    .text_color(palette.muted)
                    .child(progress_text(progress)),
            )
        }
        State::Installing(_) | State::Checking | State::Idle => status
            .child(progress_bar(None, cx))
            .when(matches!(state, State::Installing(_)), |view| {
                view.child(
                    div()
                        .text_color(palette.muted)
                        .child(tr("update.installing_hint")),
                )
            }),
        State::Failed { failure, .. } => {
            status.child(div().text_color(palette.negative).child(failure.message()))
        }
        State::Available(update) if update.installer.is_none() => status.child(
            div()
                .text_color(palette.muted)
                .child(tr("update.manual_hint")),
        ),
        State::Available(update) => status.child(
            Link::new("update-release-page")
                .child(tr("update.release_notes"))
                .href(update.page.clone())
                .text_color(palette.accent)
                .underline()
                .accessibility_label(tr("update.github_notes"))
                .open_with(|url, _, _, cx| cx.open_url(url)),
        ),
        State::UpToDate => status,
    };
    modal_footer(cx)
        .items_end()
        .child(status)
        .child(action(updater, state, cx))
}

fn progress_bar(percent: Option<f32>, cx: &App) -> Progress {
    Progress::new("update-progress")
        .color(cx.global::<TooltipTheme>().accent)
        .loading(percent.is_none())
        .value(percent.unwrap_or_default())
        .accessibility_label(tr("update.downloading_hint"))
}

fn progress_text(progress: &Download) -> String {
    let mebibytes = |bytes: u64| format!("{:.1}", bytes as f64 / MEBIBYTE);
    match (progress.percent(), progress.total) {
        (Some(percent), Some(total)) => trf(
            "update.progress",
            &[
                ("percent", format!("{percent:.0}")),
                ("received", mebibytes(progress.received)),
                ("total", mebibytes(total)),
            ],
        ),
        _ => trf(
            "update.progress_unsized",
            &[("received", mebibytes(progress.received))],
        ),
    }
}

fn action(updater: &Entity<Updater>, state: &State, cx: &App) -> Button {
    let updater = updater.clone();
    let (label, busy) = match state {
        State::Available(Update {
            installer: None, ..
        }) => (tr("update.release_page"), false),
        State::Available(_) => (tr("update.restart"), false),
        State::Downloading { .. } => (tr("update.downloading"), true),
        State::Installing(_) => (tr("update.installing"), true),
        State::Failed { .. } => (tr("update.retry"), false),
        State::UpToDate => (tr("update.check_again"), false),
        State::Checking | State::Idle => (tr("update.checking"), true),
    };
    let tone = if matches!(state, State::UpToDate) {
        ButtonTone::Neutral
    } else {
        ButtonTone::Primary
    };
    modal_button("update-action", label, tone, cx)
        .loading(busy)
        .disabled(busy)
        .on_click(move |_, _, cx| {
            updater.update(cx, |updater, cx| match updater.state {
                State::Failed { .. } => updater.retry(cx),
                State::UpToDate => updater.check(false, cx),
                _ => updater.install(cx),
            })
        })
}
