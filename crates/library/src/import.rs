//! Import dialog: a share code or link, a preview of what it holds, and the name to save it under.
use super::*;
use crate::create::{destination, dialog_error};
use gpui_kit::component::dialog::Confirm;
use hsplanner_build::{
    codec::{self, SharedBuild},
    loadout::LoadoutKind,
    site,
};
use hsplanner_engine::calc::data;
use hsplanner_ui::components::{
    ModalSize, PlannerDialogExt, modal_body, modal_eyebrow, modal_footer, modal_header,
    modal_label, modal_layout,
};
use hsplanner_ui::controls::{ButtonTone, modal_button};

impl LibraryView {
    pub(super) fn import_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let library = cx.entity().downgrade();
        let session = self.session.clone();
        let folder = self.folder.clone();
        let view = cx.new(|cx| ImportView::new(library, session, folder, window, cx));
        let focus = view.read(cx).code.read(cx).focus_handle(cx);
        window.open_planner_modal(cx, ModalSize::Standard, move |dialog, _, _| {
            let target = view.clone();
            dialog
                .overlay_closable(false)
                .child(view.clone())
                .on_ok(move |_, _, cx| target.update(cx, |view, cx| view.commit(cx)))
        });
        window.focus(&focus, cx);
    }
}

struct ImportView {
    library: WeakEntity<LibraryView>,
    session: Entity<Session>,
    folder: Option<String>,
    code: Entity<InputState>,
    name: Entity<InputState>,
    /// The decoded code, or why it cannot be read; `None` while the field is empty.
    shared: Option<Result<SharedBuild, String>>,
    /// The name the dialog last filled in, so a name the user typed is never replaced.
    suggested: String,
    error: Option<String>,
    /// The download of a hsplanner.app link; replacing it drops a superseded request.
    fetch: Option<Task<()>>,
    _subscription: Subscription,
}

impl ImportView {
    fn new(
        library: WeakEntity<LibraryView>,
        session: Entity<Session>,
        folder: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let code = cx.new(|cx| localized_input("library.code_placeholder", window, cx));
        let name = cx.new(|cx| InputState::new(window, cx));
        let subscription =
            cx.subscribe_in(&code, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.decode(window, cx);
                }
            });
        let view = Self {
            library,
            session,
            folder,
            code,
            name,
            shared: None,
            suggested: String::new(),
            error: None,
            fetch: None,
            _subscription: subscription,
        };
        view.paste(false, window, cx);
        view
    }

    /// Fills the code from the clipboard. Opening the dialog takes only a readable
    /// code or a hsplanner.app link, so unrelated clipboard text never lands in the
    /// field unasked.
    fn paste(&self, requested: bool, window: &mut Window, cx: &mut Context<Self>) {
        let clipboard = cx.read_from_clipboard_async();
        cx.spawn_in(window, async move |this, cx| {
            let text = clipboard.await.ok().flatten().and_then(|item| item.text());
            let _ = this.update_in(cx, |this, window, cx| {
                let text = text.as_deref().map(str::trim).unwrap_or_default();
                if text.is_empty() || text.len() > codec::MAX_CODE_LENGTH {
                    if requested {
                        this.error = Some(
                            if text.is_empty() {
                                "library.error.clipboard"
                            } else {
                                "library.error.code_size"
                            }
                            .into(),
                        );
                        cx.notify();
                    }
                    return;
                }
                let known = site::build_id(text).is_some() || codec::decode_shared(text).is_ok();
                if !requested && !known {
                    return;
                }
                let text = text.to_owned();
                this.code
                    .update(cx, |input, cx| input.set_value(text, window, cx));
                this.decode(window, cx);
            });
        })
        .detach();
    }

    // ponytail: decodes on the UI thread; codes are capped at 200 kB and take a few
    // milliseconds. Move to a background task with a revision check if that grows.
    fn decode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let code = self.code.read(cx).value();
        self.error = None;
        self.shared = None;
        self.fetch = None;
        if let Some(id) = site::build_id(&code) {
            let request = crate::site::fetch_code(cx.http_client(), id.to_owned());
            self.fetch = Some(cx.spawn_in(window, async move |this, cx| {
                let fetched = request.await;
                let _ = this.update_in(cx, |this, window, cx| {
                    this.show(
                        fetched.and_then(|code| codec::decode_shared(&code)),
                        window,
                        cx,
                    );
                });
            }));
        } else if !code.trim().is_empty() {
            return self.show(codec::decode_shared(&code), window, cx);
        }
        cx.notify();
    }

    fn show(
        &mut self,
        shared: Result<SharedBuild, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.shared = Some(shared);
        if let Some(Ok(shared)) = &self.shared {
            let base = shared.name.clone().unwrap_or_else(|| {
                trf(
                    "library.imported_name",
                    &[("class", class_name(shared).to_owned())],
                )
            });
            let suggestion = self.session.read(cx).state().library.free_name(&base);
            let typed = self.name.read(cx).value();
            if typed.trim().is_empty() || typed == self.suggested {
                self.name.update(cx, |input, cx| {
                    input.set_value(suggestion.clone(), window, cx)
                });
                self.suggested = suggestion;
            }
        }
        cx.notify();
    }

    /// Imports the build; `false` keeps the dialog open with the reason shown.
    fn commit(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(Ok(shared)) = self.shared.clone() else {
            return false;
        };
        let name = self.name.read(cx).value().to_string();
        let folder = self.folder.clone();
        self.error = self
            .library
            .update(cx, |library, cx| {
                let imported = library.apply(cx, |session| {
                    session.import_shared(shared, &name, folder).map(|_| ())
                });
                if imported {
                    cx.emit(Opened);
                }
                library.error.take()
            })
            .unwrap_or_else(|_| Some("library.error.missing_build".into()));
        cx.notify();
        self.error.is_none()
    }

    fn summary(&self, shared: &SharedBuild, cx: &App) -> Div {
        let p = cx.global::<TooltipTheme>();
        let snapshot = &shared.snapshot;
        let extra_loadouts: usize = LoadoutKind::ALL
            .into_iter()
            .map(|kind| shared.loadouts.count(kind).saturating_sub(1))
            .sum();
        let counts = [
            ("shell.tree", snapshot.allocated_tree_nodes.len()),
            ("shell.gear", snapshot.inventory.len()),
            (
                "shell.skills",
                snapshot
                    .skill_ranks
                    .values()
                    .filter(|&&rank| rank > 0)
                    .count(),
            ),
            ("gear.stash", shared.stash.len()),
            ("library.loadouts", extra_loadouts),
        ];
        div()
            .flex()
            .items_center()
            .gap_3()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(p.border)
            .bg(p.panel)
            .child(portrait(snapshot.class_id.as_deref(), true, cx))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .child(div().text_color(p.text).child(trf(
                        "library.import_summary",
                        &[
                            ("class", class_name(shared).to_owned()),
                            ("level", snapshot.level.to_string()),
                        ],
                    )))
                    .child(
                        div().flex().flex_wrap().gap_x_3().gap_y_1().children(
                            counts
                                .into_iter()
                                .filter(|&(_, count)| count > 0)
                                .map(|(key, count)| label(format!("{} {count}", tr(key)), p.muted)),
                        ),
                    ),
            )
    }
}

fn class_name(shared: &SharedBuild) -> &str {
    shared
        .snapshot
        .class_id
        .as_deref()
        .and_then(data::get_class)
        .map_or(tr("library.unknown"), |class| class.name.as_str())
}

impl Render for ImportView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let readable = matches!(self.shared, Some(Ok(_)));
        let error = self
            .error
            .as_deref()
            .or(match &self.shared {
                Some(Err(error)) => Some(error.as_str()),
                _ => None,
            })
            .map(|error| dialog_error(error, cx));
        let mut body = modal_body("import-build-body")
            .child(modal_label(
                "import-build-code",
                tr("library.import_code"),
                cx,
            ))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&self.code).planner_style(cx)),
                    )
                    .child(
                        modal_button(
                            "import-build-paste",
                            tr("library.paste"),
                            ButtonTone::Neutral,
                            cx,
                        )
                        .on_click(cx.listener(|this, _, window, cx| this.paste(true, window, cx))),
                    ),
            );
        body = match &self.shared {
            Some(Ok(shared)) => body
                .child(self.summary(shared, cx))
                .child(modal_label("import-build-name", tr("library.name"), cx))
                .child(Input::new(&self.name).planner_style(cx)),
            Some(Err(_)) => body,
            None if self.fetch.is_some() => body.child(
                div()
                    .text_size(rems(12. / 13.))
                    .text_color(p.muted)
                    .child(tr("library.import_fetching")),
            ),
            None => body.child(
                div()
                    .text_size(rems(12. / 13.))
                    .text_color(p.muted)
                    .child(tr("library.import_waiting")),
            ),
        };
        modal_layout(window)
            .child(modal_header(
                modal_eyebrow("import-build-eyebrow", tr("chrome.builds")),
                tr("library.import_title"),
                Some(destination(self.folder.as_deref(), &self.session, cx).into()),
                cx,
            ))
            .child(body.children(error))
            .child(
                modal_footer(cx).child(
                    modal_button(
                        "import-build-commit",
                        tr("library.import"),
                        ButtonTone::Primary,
                        cx,
                    )
                    .disabled(!readable)
                    .on_click(|_, window, cx| {
                        window.dispatch_action(Box::new(Confirm { secondary: false }), cx)
                    }),
                ),
            )
    }
}
