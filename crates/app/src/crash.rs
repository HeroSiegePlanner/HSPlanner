//! A panic saves `crash.log` in the data directory and starts this executable
//! again with `--crash-report`, which shows the log in its own window: the
//! crashed process cannot be trusted to draw anything.
use gpui_kit::base::Disableable;
use gpui_kit::component::Root;
use gpui_kit::component::text::TextView;
use gpui_kit::{prelude::*, *};
use hsplanner_ui::components::{modal_body, modal_eyebrow, modal_footer, modal_header};
use hsplanner_ui::controls::{ButtonTone, modal_button};
use hsplanner_ui::i18n::{self, tr, trf};
use hsplanner_ui::theme::{FONT_FAMILY, TooltipTheme};
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

pub const LOG_NAME: &str = "crash.log";
const REPORT_FLAG: &str = "--crash-report";
const LANGUAGE_FLAG: &str = "--language";

/// Saves the first panic of this process to `log`, then opens the crash window.
pub fn install(log: PathBuf) {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        if REPORTED.swap(true, Ordering::SeqCst) {
            return;
        }
        let text = report(
            &panic_message(info.payload()),
            info.location().map(ToString::to_string),
            std::thread::current().name().unwrap_or("unnamed"),
            &std::backtrace::Backtrace::force_capture().to_string(),
        );
        if std::fs::write(&log, text).is_ok() {
            let language = i18n::language().code();
            let _ = relaunch(&[
                OsStr::new(REPORT_FLAG),
                log.as_os_str(),
                OsStr::new(LANGUAGE_FLAG),
                OsStr::new(language),
            ]);
        }
    }));
}

/// The crash window's `--crash-report <log> --language <code>` arguments.
pub fn report_request(mut args: impl Iterator<Item = OsString>) -> Option<(PathBuf, String)> {
    let (mut log, mut language) = (None, String::new());
    while let Some(arg) = args.next() {
        if arg == REPORT_FLAG {
            log = args.next().map(PathBuf::from);
        } else if arg == LANGUAGE_FLAG {
            language = args
                .next()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
        }
    }
    log.map(|log| (log, language))
}

pub fn show(log: PathBuf, language: String) {
    let text = std::fs::read_to_string(&log).map_err(|error| error.to_string());
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            hsplanner_ui::theme::init(cx);
            hsplanner_ui::scroll::init(cx);
            i18n::apply_language(&language, cx);
            let quit = if cfg!(target_os = "macos") {
                "cmd-q"
            } else {
                "ctrl-q"
            };
            cx.bind_keys([KeyBinding::new(quit, crate::shell::Quit, None)]);
            cx.on_action(|_: &crate::shell::Quit, cx| cx.quit());
            cx.set_menus([Menu::new("HSPlanner")
                .items([MenuItem::action(tr("shell.quit"), crate::shell::Quit)])]);
            let bounds = Bounds::centered(None, size(px(760.), px(560.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(480.), px(360.))),
                    titlebar: Some(TitlebarOptions {
                        title: Some(tr("crash.window_title").into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |window, cx| {
                    window.on_window_should_close(cx, |_, cx| {
                        cx.quit();
                        true
                    });
                    let view = cx.new(|_| CrashWindow {
                        log,
                        text,
                        copied: false,
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("open crash window");
            cx.activate(true);
        });
}

struct CrashWindow {
    log: PathBuf,
    text: Result<String, String>,
    copied: bool,
}

impl Render for CrashWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let (panel, text, negative) = (p.panel, p.text, p.negative);
        let saved = trf("crash.saved", &[("path", self.log.display().to_string())]);
        let log = match &self.text {
            Ok(log) => TextView::markdown("crash-log", fenced(log))
                .selectable(true)
                .into_any_element(),
            Err(error) => div()
                .text_color(negative)
                .child(trf("crash.unreadable", &[("error", error.clone())]))
                .into_any_element(),
        };
        let copy = if self.copied {
            tr("crash.copied")
        } else {
            tr("crash.copy")
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(panel)
            .text_color(text)
            .font_family(FONT_FAMILY)
            .line_height(relative(1.5))
            .child(modal_header(
                modal_eyebrow("crash-eyebrow", tr("crash.eyebrow")),
                tr("crash.heading"),
                Some(saved.into()),
                cx,
            ))
            .child(modal_body("crash-log-scroll").text_xs().child(log))
            .child(
                modal_footer(cx)
                    .child(
                        modal_button("crash-copy", copy, ButtonTone::Neutral, cx)
                            .disabled(self.text.is_err())
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Ok(log) = &this.text {
                                    cx.write_to_clipboard(ClipboardItem::new_string(log.clone()));
                                    this.copied = true;
                                    cx.notify();
                                }
                            })),
                    )
                    .child(
                        modal_button(
                            "crash-restart",
                            tr("crash.restart"),
                            ButtonTone::Primary,
                            cx,
                        )
                        .on_click(|_, _, cx| {
                            if relaunch(&[]).is_ok() {
                                cx.quit();
                            }
                        }),
                    ),
            )
    }
}

fn relaunch(args: &[&OsStr]) -> std::io::Result<()> {
    Command::new(std::env::current_exe()?)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
}

fn report(message: &str, location: Option<String>, thread: &str, backtrace: &str) -> String {
    let mut text = format!(
        "HSPlanner {} crashed on {} ({})\nThread: {thread}\nPanic: {message}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    if let Some(location) = location {
        text.push_str(&format!("Location: {location}\n"));
    }
    let recent = hsplanner_ui::debug_log::lines();
    if !recent.is_empty() {
        text.push_str(&format!("\nRecent log:\n{}\n", recent.join("\n")));
    }
    text.push_str(&format!("\nBacktrace:\n{backtrace}"));
    text
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "Box<dyn Any>".to_owned()
    }
}

/// A Markdown code block that no backtick run inside the log can close early.
fn fenced(log: &str) -> String {
    let longest = log.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}\n{log}\n{fence}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn report_names_the_panic_its_location_and_the_backtrace() {
        let text = report(
            "index out of bounds",
            Some("crates/planner/src/tree.rs:12:5".into()),
            "main",
            "0: hsplanner::main",
        );
        assert!(text.starts_with(&format!("HSPlanner {} crashed", env!("CARGO_PKG_VERSION"))));
        assert!(text.contains("Thread: main\nPanic: index out of bounds\n"));
        assert!(text.contains("Location: crates/planner/src/tree.rs:12:5\n"));
        assert!(text.ends_with("Backtrace:\n0: hsplanner::main"));
        assert!(!report("x", None, "main", "").contains("Location:"));
    }

    #[::core::prelude::v1::test]
    fn panic_message_reads_str_and_string_payloads() {
        assert_eq!(panic_message(&"static"), "static");
        assert_eq!(panic_message(&String::from("owned")), "owned");
        assert_eq!(panic_message(&7_u8), "Box<dyn Any>");
    }

    #[::core::prelude::v1::test]
    fn report_request_needs_the_log_and_keeps_the_language() {
        let args = |list: &[&str]| {
            list.iter()
                .map(OsString::from)
                .collect::<Vec<_>>()
                .into_iter()
        };
        assert_eq!(
            report_request(args(&[
                "-psn_0_1",
                "--crash-report",
                "/tmp/crash.log",
                "--language",
                "ko"
            ])),
            Some((PathBuf::from("/tmp/crash.log"), "ko".into()))
        );
        assert_eq!(
            report_request(args(&["--crash-report", "/tmp/crash.log"])),
            Some((PathBuf::from("/tmp/crash.log"), String::new()))
        );
        assert_eq!(report_request(args(&["--language", "ko"])), None);
        assert_eq!(report_request(args(&[])), None);
    }

    #[::core::prelude::v1::test]
    fn fenced_log_outlasts_backtick_runs_inside_it() {
        assert_eq!(fenced("plain"), "```\nplain\n```");
        assert_eq!(fenced("a ```` b"), "`````\na ```` b\n`````");
    }
}
