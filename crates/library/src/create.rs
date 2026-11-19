//! New build dialog: name, class and level in one step, filed in the open folder.
use super::*;
use gpui_kit::component::dialog::Confirm;
use hsplanner_engine::calc::{data, types::CharacterClass};
use hsplanner_ui::components::{
    ModalSize, PlannerDialogExt, modal_body, modal_eyebrow, modal_footer, modal_header,
    modal_label, modal_layout,
};
use hsplanner_ui::controls::{ButtonTone, modal_button, planner_button};

const FALLBACK_CLASS: &str = "viking";
const LEVEL_DIGITS: usize = 5;

/// Where a new or imported build lands, as the dialog subtitle.
pub(super) fn destination(folder: Option<&str>, session: &Entity<Session>, cx: &App) -> String {
    let library = &session.read(cx).state().library;
    let name = folder
        .and_then(|id| library.folders.iter().find(|folder| folder.id == id))
        .map_or(tr("library.unfiled"), |folder| folder.name.as_str());
    trf("library.saved_to", &[("folder", name.to_owned())])
}

pub(super) fn dialog_error(error: &str, cx: &App) -> Div {
    div()
        .text_size(rems(12. / 13.))
        .text_color(cx.global::<TooltipTheme>().negative)
        .child(render_error(error))
}

impl LibraryView {
    pub(super) fn new_build_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let library = cx.entity().downgrade();
        let session = self.session.clone();
        let folder = self.folder.clone();
        let view = cx.new(|cx| NewBuildView::new(library, session, folder, window, cx));
        let focus = view.read(cx).name.read(cx).focus_handle(cx);
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

struct NewBuildView {
    library: WeakEntity<LibraryView>,
    session: Entity<Session>,
    folder: Option<String>,
    name: Entity<InputState>,
    level: Entity<InputState>,
    class_id: String,
    error: Option<String>,
}

impl NewBuildView {
    fn new(
        library: WeakEntity<LibraryView>,
        session: Entity<Session>,
        folder: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let class_id = session
            .read(cx)
            .snapshot()
            .class_id
            .clone()
            .filter(|id| data::get_class(id).is_some())
            .unwrap_or_else(|| FALLBACK_CLASS.into());
        let placeholder = default_name(&class_id, &session, cx);
        let name = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let level = cx.new(|cx| {
            InputState::new(window, cx)
                .validate(|text, _| {
                    text.len() <= LEVEL_DIGITS && text.bytes().all(|b| b.is_ascii_digit())
                })
                .default_value("1")
        });
        Self {
            library,
            session,
            folder,
            name,
            level,
            class_id,
            error: None,
        }
    }

    fn pick(&mut self, class_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.class_id = class_id.to_owned();
        let placeholder = default_name(class_id, &self.session, cx);
        self.name.update(cx, |input, cx| {
            input.set_placeholder(placeholder, window, cx)
        });
        cx.notify();
    }

    /// Creates the build; `false` keeps the dialog open with the reason shown.
    fn commit(&mut self, cx: &mut Context<Self>) -> bool {
        let typed = self.name.read(cx).value().trim().to_owned();
        let name = if typed.is_empty() {
            default_name(&self.class_id, &self.session, cx)
        } else {
            typed
        };
        // The field takes digits only and the session clamps to the game's range.
        let level = self.level.read(cx).value().parse().unwrap_or(1);
        let class_id = self.class_id.clone();
        let folder = self.folder.clone();
        self.error = self
            .library
            .update(cx, |library, cx| {
                let created = library.apply(cx, |session| {
                    session
                        .create_build(&name, &class_id, level, folder)
                        .map(|_| ())
                });
                if created {
                    cx.emit(Opened);
                }
                library.error.take()
            })
            .unwrap_or_else(|_| Some("library.error.missing_build".into()));
        cx.notify();
        self.error.is_none()
    }

    fn class_tile(&self, class: &'static CharacterClass, cx: &mut Context<Self>) -> Button {
        let active = self.class_id == class.id;
        let tone = if active {
            ButtonTone::Primary
        } else {
            ButtonTone::Neutral
        };
        let id = class.id.clone();
        planner_button(
            SharedString::from(format!("new-build-class-{}", class.id)),
            tone,
            cx,
        )
        .selected(active)
        .w(rems(79. / 13.))
        .h_auto()
        .p_1p5()
        .accessibility_label(class.name.clone())
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .child(portrait(Some(&class.id), false, cx))
                .child(
                    div()
                        .w_full()
                        .truncate()
                        .text_center()
                        .text_size(rems(10. / 13.))
                        .child(class.name.clone()),
                ),
        )
        .on_click(cx.listener(move |this, _, window, cx| this.pick(&id, window, cx)))
    }
}

fn default_name(class_id: &str, session: &Entity<Session>, cx: &App) -> String {
    let class = data::get_class(class_id).map_or(class_id, |class| class.name.as_str());
    let base = trf("library.default_build_name", &[("class", class.to_owned())]);
    session.read(cx).state().library.free_name(&base)
}

impl Render for NewBuildView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let max_level = data::game_config().max_character_level.max(1);
        let mut classes: Vec<_> = data::data().classes.values().collect();
        classes.sort_by(|a, b| a.name.cmp(&b.name));
        let tiles: Vec<_> = classes
            .into_iter()
            .map(|class| self.class_tile(class, cx))
            .collect();
        let level = self.level.clone();
        modal_layout(window)
            .child(modal_header(
                modal_eyebrow("new-build-eyebrow", tr("chrome.builds")),
                tr("library.new_build"),
                Some(destination(self.folder.as_deref(), &self.session, cx).into()),
                cx,
            ))
            .child(
                modal_body("new-build-body")
                    .child(modal_label("new-build-name", tr("library.name"), cx))
                    .child(Input::new(&self.name).planner_style(cx))
                    .child(modal_label("new-build-class", tr("library.class"), cx))
                    .child(div().flex().flex_wrap().gap_1p5().children(tiles))
                    .child(modal_label(
                        "new-build-level",
                        tr("planner.common.level"),
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(rems(79. / 13.))
                                    .child(Input::new(&self.level).planner_style(cx)),
                            )
                            .child(
                                modal_button(
                                    "new-build-max-level",
                                    trf("library.level_max", &[("level", max_level.to_string())]),
                                    ButtonTone::Neutral,
                                    cx,
                                )
                                .on_click(move |_, window, cx| {
                                    level.update(cx, |input, cx| {
                                        input.set_value(max_level.to_string(), window, cx)
                                    });
                                }),
                            ),
                    )
                    .children(self.error.as_ref().map(|error| dialog_error(error, cx))),
            )
            .child(
                modal_footer(cx).child(
                    modal_button(
                        "new-build-create",
                        tr("library.create"),
                        ButtonTone::Primary,
                        cx,
                    )
                    .on_click(|_, window, cx| {
                        window.dispatch_action(Box::new(Confirm { secondary: false }), cx)
                    }),
                ),
            )
    }
}
