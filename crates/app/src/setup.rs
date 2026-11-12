//! The setup window. A downloaded `…-setup.exe` is this executable: opened outside
//! its installation it installs itself, `--update` replaces the running program
//! without a window, and `--uninstall` is what "Installed apps" runs.
use gpui_kit::base::Disableable;
use gpui_kit::component::{Root, checkbox::Checkbox};
use gpui_kit::{prelude::*, *};
use hsplanner_installer::{Layout, Request};
use hsplanner_ui::components::{
    modal_body, modal_eyebrow, modal_footer, modal_header, modal_label, modal_status,
};
use hsplanner_ui::controls::{ButtonTone, modal_button};
use hsplanner_ui::i18n::{self, tr, trf};
use hsplanner_ui::theme::{FONT_FAMILY, MONO_FONT_FAMILY, TooltipTheme};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const KEY_CONTEXT: &str = "Setup";
const WINDOW_WIDTH: f32 = 540.;
const WINDOW_HEIGHT: f32 = 360.;

gpui_kit::actions!(setup, [Proceed]);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Install,
    Uninstall,
}

/// What installing does to this computer; it names the window and its button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Change {
    Install,
    Update,
    Reinstall,
}

#[derive(Clone, PartialEq, Eq)]
enum Phase {
    Ready,
    Working,
    Done,
    Failed(String),
}

pub fn run(request: Request) {
    let layout = layout().map_err(|error| format!("{error:#}"));
    let (mode, failure) = match request {
        Request::Install => (Mode::Install, None),
        Request::Uninstall => (Mode::Uninstall, None),
        Request::Update { wait_for } => match update(&layout, wait_for) {
            Ok(()) => return,
            // A silent update that failed still owes the user an answer and a retry.
            Err(error) => (Mode::Install, Some(error)),
        },
    };
    show(mode, layout, failure);
}

fn layout() -> anyhow::Result<Layout> {
    // A scratch folder lets debug builds rehearse setup on any platform.
    if cfg!(debug_assertions)
        && let Some(root) = std::env::var_os("HSPLANNER_SETUP_DIR")
    {
        return Ok(Layout::under(Path::new(&root), "HSPlanner-debug"));
    }
    Layout::current_user()
}

fn update(layout: &Result<Layout, String>, wait_for: Option<u32>) -> Result<(), String> {
    let layout = layout.as_ref().map_err(Clone::clone)?;
    let source = std::env::current_exe().map_err(|error| error.to_string())?;
    hsplanner_installer::update(&source, layout, VERSION, wait_for)
        .map_err(|error| format!("{error:#}"))?;
    open(layout).map_err(|error| error.to_string())
}

fn open(layout: &Layout) -> std::io::Result<()> {
    Command::new(layout.executable())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
}

/// The language of the saved library, so an update speaks like the application.
fn language() -> String {
    hsplanner_build::storage::data_directory()
        .and_then(|directory| hsplanner_build::storage::load(&directory))
        .map(|state| state.settings.language)
        .unwrap_or_default()
}

fn change(is_installed: bool, installed_version: Option<&str>) -> Change {
    if !is_installed {
        Change::Install
    } else if installed_version == Some(VERSION) {
        Change::Reinstall
    } else {
        Change::Update
    }
}

fn show(mode: Mode, layout: Result<Layout, String>, failure: Option<String>) {
    let language = language();
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
            cx.bind_keys([
                KeyBinding::new(quit, crate::shell::Quit, None),
                KeyBinding::new("enter", Proceed, Some(KEY_CONTEXT)),
            ]);
            cx.on_action(|_: &crate::shell::Quit, cx| cx.quit());
            cx.set_menus([Menu::new("HSPlanner")
                .items([MenuItem::action(tr("shell.quit"), crate::shell::Quit)])]);
            let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    is_resizable: false,
                    titlebar: Some(TitlebarOptions {
                        title: Some(tr("setup.window_title").into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |window, cx| {
                    window.on_window_should_close(cx, |_, cx| {
                        cx.quit();
                        true
                    });
                    let view = cx.new(|cx| SetupWindow::new(mode, layout, failure, window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("open setup window");
            cx.activate(true);
        });
}

struct SetupWindow {
    mode: Mode,
    layout: Result<Layout, String>,
    change: Change,
    installed_version: Option<String>,
    data_directory: Option<String>,
    desktop_shortcut: bool,
    phase: Phase,
    logo: Arc<Image>,
    focus: FocusHandle,
}

impl SetupWindow {
    fn new(
        mode: Mode,
        layout: Result<Layout, String>,
        failure: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let installed = layout.as_ref().ok().filter(|layout| layout.is_installed());
        let installed_version = installed.and_then(Layout::installed_version);
        let phase = match (&layout, failure) {
            (Err(error), _) => Phase::Failed(error.clone()),
            (Ok(_), Some(error)) => Phase::Failed(error),
            (Ok(_), None) => Phase::Ready,
        };
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            mode,
            change: change(installed.is_some(), installed_version.as_deref()),
            installed_version,
            data_directory: hsplanner_build::storage::data_directory()
                .ok()
                .map(|directory| directory.display().to_string()),
            // A new installation offers the shortcut; an update keeps the user's choice.
            desktop_shortcut: installed.is_none_or(|layout| layout.desktop_shortcut().exists()),
            layout,
            phase,
            logo: crate::chrome::logo(),
            focus,
        }
    }

    fn proceed(&mut self, _: &Proceed, _: &mut Window, cx: &mut Context<Self>) {
        match (&self.phase, self.mode) {
            (Phase::Working, _) => {}
            (Phase::Done, Mode::Uninstall) => cx.quit(),
            (Phase::Done, Mode::Install) => self.open_installed(cx),
            (Phase::Ready | Phase::Failed(_), _) => self.start(cx),
        }
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        let Ok(layout) = self.layout.clone() else {
            return;
        };
        self.phase = Phase::Working;
        cx.notify();
        let (mode, desktop_shortcut) = (self.mode, self.desktop_shortcut);
        let task = cx.background_spawn(async move {
            match mode {
                Mode::Install => {
                    let source = std::env::current_exe()?;
                    hsplanner_installer::install(&source, &layout, VERSION, desktop_shortcut)
                }
                Mode::Uninstall => hsplanner_installer::uninstall(&layout),
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.phase = match result {
                    Ok(()) => Phase::Done,
                    Err(error) => Phase::Failed(format!("{error:#}")),
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn open_installed(&mut self, cx: &mut Context<Self>) {
        let Ok(layout) = &self.layout else {
            return;
        };
        match open(layout) {
            Ok(()) => cx.quit(),
            Err(error) => {
                self.phase = Phase::Failed(error.to_string());
                cx.notify();
            }
        }
    }

    fn heading(&self) -> (SharedString, SharedString) {
        let version = |version: &str| [("version", version.to_owned())];
        match (self.mode, &self.phase, self.change) {
            (Mode::Install, Phase::Done, _) => (
                tr("setup.done.title").into(),
                trf("setup.done.subtitle", &version(VERSION)).into(),
            ),
            (Mode::Uninstall, Phase::Done, _) => (
                tr("setup.uninstalled.title").into(),
                tr("setup.uninstall.subtitle").into(),
            ),
            (Mode::Uninstall, ..) => (
                tr("setup.uninstall.title").into(),
                tr("setup.uninstall.subtitle").into(),
            ),
            (Mode::Install, _, Change::Install) => (
                tr("setup.install.title").into(),
                tr("setup.install.subtitle").into(),
            ),
            (Mode::Install, _, Change::Reinstall) => (
                tr("setup.reinstall.title").into(),
                tr("setup.existing.subtitle").into(),
            ),
            (Mode::Install, _, Change::Update) => (
                tr("setup.update.title").into(),
                match &self.installed_version {
                    Some(installed) => trf("setup.update.subtitle", &version(installed)).into(),
                    None => tr("setup.existing.subtitle").into(),
                },
            ),
        }
    }

    /// The one command of the window; a finished uninstall leaves only closing it.
    fn command(&self) -> Option<(&'static str, ButtonTone)> {
        match (self.mode, &self.phase, self.change) {
            (Mode::Uninstall, Phase::Done, _) => None,
            (Mode::Install, Phase::Done, _) => Some((tr("setup.open"), ButtonTone::Primary)),
            (_, Phase::Failed(_), _) => Some((tr("common.retry"), ButtonTone::Primary)),
            (Mode::Uninstall, ..) => Some((tr("setup.uninstall"), ButtonTone::Danger)),
            (Mode::Install, _, Change::Install) => Some((tr("setup.install"), ButtonTone::Primary)),
            (Mode::Install, _, Change::Update) => Some((tr("setup.update"), ButtonTone::Primary)),
            (Mode::Install, _, Change::Reinstall) => {
                Some((tr("setup.reinstall"), ButtonTone::Primary))
            }
        }
    }
}

impl Render for SetupWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let (panel, text, muted, negative) = (p.panel, p.text, p.muted, p.negative);
        let (title, subtitle) = self.heading();
        let working = self.phase == Phase::Working;
        let removed = self.mode == Mode::Uninstall && self.phase == Phase::Done;
        let location = self
            .layout
            .as_ref()
            .ok()
            .filter(|_| !removed)
            .map(|layout| layout.directory().display().to_string());
        let saved_data = (self.mode == Mode::Uninstall)
            .then(|| self.data_directory.clone())
            .flatten();
        let shortcut = (self.mode == Mode::Install && self.phase != Phase::Done).then(|| {
            Checkbox::new("setup-desktop-shortcut")
                .label(tr("setup.desktop_shortcut"))
                .checked(self.desktop_shortcut)
                .disabled(working)
                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                    this.desktop_shortcut = *checked;
                    cx.notify();
                }))
        });
        let failure = match &self.phase {
            Phase::Failed(error) => Some(trf("setup.failed", &[("error", error.clone())])),
            _ => None,
        };
        let progress = working.then(|| match self.mode {
            Mode::Install => tr("setup.installing"),
            Mode::Uninstall => tr("setup.uninstalling"),
        });
        let command = self.command().map(|(label, tone)| {
            modal_button("setup-proceed", label, tone, cx)
                .disabled(working || self.layout.is_err())
                .loading(working)
                .on_click(cx.listener(|this, _, window, cx| this.proceed(&Proceed, window, cx)))
        });
        // A finished uninstall has nothing left to offer, so no empty bar is drawn.
        let footer = (working || command.is_some()).then(|| {
            modal_footer(cx)
                .children(progress.map(|progress| modal_status(progress, muted)))
                .children(command)
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(panel)
            .text_color(text)
            .font_family(FONT_FAMILY)
            .line_height(relative(1.5))
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::proceed))
            .child(
                div()
                    .relative()
                    .flex_none()
                    .child(modal_header(
                        modal_eyebrow(
                            "setup-eyebrow",
                            trf("setup.eyebrow", &[("version", VERSION.to_owned())]),
                        ),
                        title,
                        Some(subtitle),
                        cx,
                    ))
                    .child(
                        img(self.logo.clone())
                            .absolute()
                            .top_4()
                            .right_6()
                            .size_10(),
                    ),
            )
            .child(
                modal_body("setup-body")
                    .children(
                        location
                            .map(|path| field("setup-location", tr("setup.location"), path, cx)),
                    )
                    .children(
                        saved_data
                            .map(|path| field("setup-data", tr("setup.saved_data"), path, cx)),
                    )
                    // A row of its own keeps the click target on the box and its label.
                    .children(shortcut.map(|shortcut| div().flex().child(shortcut)))
                    .children(failure.map(|failure| {
                        div()
                            .text_size(rems(12. / 13.))
                            .text_color(negative)
                            .child(failure)
                    })),
            )
            .children(footer)
    }
}

fn field(id: &'static str, label: &'static str, value: String, cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(modal_label(id, label, cx))
        .child(
            div()
                .font_family(MONO_FONT_FAMILY)
                .text_size(rems(12. / 13.))
                .text_color(cx.global::<TooltipTheme>().muted)
                .child(value),
        )
}

#[cfg(test)]
mod tests {
    use super::{Change, VERSION, change};

    #[::core::prelude::v1::test]
    fn the_installed_version_decides_what_the_button_promises() {
        assert_eq!(change(false, None), Change::Install);
        assert_eq!(change(false, Some(VERSION)), Change::Install);
        assert_eq!(change(true, Some(VERSION)), Change::Reinstall);
        assert_eq!(change(true, Some("0.0.1")), Change::Update);
        assert_eq!(change(true, None), Change::Update);
    }
}
