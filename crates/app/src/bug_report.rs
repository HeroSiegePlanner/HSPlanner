//! A retained report draft. Nothing leaves the device until Send report is activated.
use crate::bug_report_transport::{self as transport, MAX_SHOTS, Report, Shot};
use gpui_kit::{
    base::Disableable,
    component::{
        IndexPath, WindowExt,
        checkbox::Checkbox,
        input::{Input, InputEvent, InputState, Textarea, TextareaState},
        select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState},
    },
    prelude::*,
    *,
};
use hsplanner_build::session::Session;
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::{
    components::{modal_eyebrow, modal_footer, modal_header, modal_label, modal_status},
    controls::{ButtonTone, PlannerControl, icon_button, modal_button},
    theme::TooltipTheme,
};

const KINDS: [&str; 3] = ["report.kind_bug", "report.kind_data", "report.kind_idea"];
const IDEA: usize = 2;

#[derive(Clone)]
struct Kind(usize);
impl SelectItem for Kind {
    type Value = usize;
    fn title(&self) -> SharedString {
        tr(KINDS[self.0]).into()
    }
    fn value(&self) -> &usize {
        &self.0
    }
}

pub(super) fn open(session: Entity<Session>, window: &mut Window, cx: &mut App) {
    let draft = session.read(cx).draft();
    let build = hsplanner_build::codec::encode(&draft.snapshot, &draft.notes)
        .ok()
        .map(|code| {
            let label = session
                .read(cx)
                .state()
                .library
                .builds
                .iter()
                .find(|build| Some(&build.id) == draft.build_id.as_ref())
                .map(|build| build.name.clone())
                .unwrap_or_else(|| tr("report.current_build").into());
            (label, code)
        });
    let editor = cx.new(|cx| ReportEditor::new(build, window, cx));
    window.open_dialog(cx, move |dialog, window, cx| {
        let palette = cx.global::<TooltipTheme>();
        dialog
            .p_0()
            .gap_0()
            .rounded_xl()
            .bg(linear_gradient(
                180.,
                linear_color_stop(palette.panel_secondary, 0.),
                linear_color_stop(palette.background, 1.),
            ))
            .width((window.rem_size() * 36.).min(window.viewport_size().width * 0.94))
            .max_h(window.viewport_size().height * 0.88)
            .margin_top(window.viewport_size().height * 0.06)
            .overlay_closable(false)
            .on_ok(|_, _, _| false)
            .child(editor.clone())
    });
}
struct ReportEditor {
    title: Entity<InputState>,
    description: Entity<TextareaState>,
    steps: Entity<TextareaState>,
    expected: Entity<TextareaState>,
    contact: Entity<InputState>,
    kind_select: Entity<SelectState<SearchableVec<Kind>>>,
    kind: usize,
    build: Option<(String, String)>,
    attach: bool,
    shots: Vec<Shot>,
    endpoint: Option<String>,
    busy: bool,
    loading: bool,
    sent: bool,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}
impl ReportEditor {
    fn new(build: Option<(String, String)>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let title =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("report.title_placeholder")));
        let description = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(4)
                .placeholder(tr("report.description_placeholder"))
        });
        let steps = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(4)
                .placeholder(tr("report.steps_placeholder"))
        });
        let expected = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(2)
                .placeholder(tr("report.expected_placeholder"))
        });
        let contact =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("report.contact_placeholder")));
        let kind_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new((0..KINDS.len()).map(Kind).collect::<Vec<_>>()),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let mut subscriptions = vec![cx.subscribe(
            &kind_select,
            |this, _, event: &SelectEvent<SearchableVec<Kind>>, cx| {
                if let SelectEvent::Confirm(Some(kind)) = event {
                    this.kind = *kind;
                    cx.notify();
                }
            },
        )];
        for input in [&title, &contact] {
            subscriptions.push(cx.observe(input, |_, _, cx| cx.notify()));
            subscriptions.push(cx.subscribe(input, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.error = None;
                    cx.notify();
                }
            }));
        }
        for input in [&description, &steps, &expected] {
            subscriptions.push(cx.observe(input, |_, _, cx| cx.notify()));
            subscriptions.push(cx.subscribe(input, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.error = None;
                    cx.notify();
                }
            }));
        }
        Self {
            title,
            description,
            steps,
            expected,
            contact,
            kind_select,
            kind: 0,
            attach: true,
            build,
            shots: vec![],
            endpoint: transport::endpoint(),
            busy: false,
            loading: false,
            sent: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }
    fn report(&self, cx: &App) -> Report {
        Report {
            kind: self.kind,
            title: self.title.read(cx).value().to_string(),
            description: self.description.read(cx).value().to_string(),
            steps: if self.kind == IDEA {
                String::new()
            } else {
                self.steps.read(cx).value().to_string()
            },
            expected: if self.kind == IDEA {
                String::new()
            } else {
                self.expected.read(cx).value().to_string()
            },
            contact: self.contact.read(cx).value().to_string(),
            build: self.attach.then(|| self.build.clone()).flatten(),
            shots: self.shots.clone(),
        }
    }
    fn send(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.sent || self.loading {
            return;
        }
        let Some(endpoint) = self.endpoint.clone() else {
            return;
        };
        let report = self.report(cx);
        if let Err(error) = report.validate() {
            self.error = Some(error);
            cx.notify();
            return;
        }
        self.busy = true;
        self.error = None;
        cx.notify();
        let http = cx.http_client();
        let task =
            cx.background_spawn(async move { transport::send(http, endpoint, report).await });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(()) => this.sent = true,
                    Err(error) => this.error = Some(error),
                };
                cx.notify();
            });
        })
        .detach();
    }
    fn accept(&mut self, incoming: Vec<Result<Shot, String>>, cx: &mut Context<Self>) {
        self.loading = false;
        self.error = None;
        for result in incoming {
            match result {
                Ok(shot) if self.shots.len() < MAX_SHOTS => self.shots.push(shot),
                Ok(_) => {
                    self.error = Some(trf("report.max_shots", &[("count", MAX_SHOTS.to_string())]))
                }
                Err(error) => self.error = Some(error),
            }
        }
        cx.notify();
    }
    fn choose_images(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.sent || self.loading {
            return;
        }
        self.loading = true;
        cx.notify();
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(tr("report.add_screenshots").into()),
        });
        cx.spawn(async move |this, cx| {
            let results = match paths.await {
                Ok(Ok(Some(paths))) => {
                    cx.background_spawn(async move {
                        paths
                            .into_iter()
                            .take(MAX_SHOTS + 1)
                            .map(|path| Shot::from_path(&path))
                            .collect::<Vec<_>>()
                    })
                    .await
                }
                Ok(Ok(None)) => vec![],
                _ => vec![Err(tr("report.picker_failed").into())],
            };
            let _ = this.update(cx, |this, cx| this.accept(results, cx));
        })
        .detach();
    }
    fn paste_image(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.sent || self.loading {
            return;
        }
        self.loading = true;
        cx.notify();
        let clipboard = cx.read_from_clipboard_async();
        cx.spawn(async move |this, cx| {
            let incoming = match clipboard.await {
                Ok(Some(item)) => item
                    .entries()
                    .iter()
                    .filter_map(|entry| match entry {
                        ClipboardEntry::Image(image) => Some(image.bytes().to_vec()),
                        _ => None,
                    })
                    .take(MAX_SHOTS + 1)
                    .collect::<Vec<_>>(),
                _ => vec![],
            };
            let results = if incoming.is_empty() {
                vec![Err(tr("report.paste_hint").into())]
            } else {
                cx.background_spawn(async move {
                    incoming
                        .into_iter()
                        .map(|bytes| Shot::from_bytes(tr("report.pasted").into(), bytes))
                        .collect::<Vec<_>>()
                })
                .await
            };
            let _ = this.update(cx, |this, cx| this.accept(results, cx));
        })
        .detach();
    }
    fn status(&self, validation: Option<&String>) -> String {
        if self.endpoint.is_none() {
            tr("report.unconfigured").into()
        } else if self.sent {
            tr("report.sent").into()
        } else if self.busy {
            tr("report.sending").into()
        } else if self.loading {
            tr("report.reading").into()
        } else if let Some(error) = self.error.as_ref().or(validation) {
            error.clone()
        } else if self.attach && self.build.is_some() {
            tr("report.sends_build").into()
        } else {
            tr("report.sends_system").into()
        }
    }
}
impl Render for ReportEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let locked = self.busy || self.sent;
        let validation = self.report(cx).validate().err();
        let status = self.status(validation.as_ref());
        let status_color = if self.sent {
            p.positive
        } else if self.error.is_some() || self.endpoint.is_none() {
            p.negative
        } else {
            p.faint
        };
        let mut fields = div()
            .id("bug-report-fields")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_5()
            .flex()
            .flex_col()
            .gap_2p5()
            .text_size(rems(12. / 13.))
            .child(field(
                tr("report.type"),
                Select::new(&self.kind_select)
                    .planner_style(cx)
                    .accessibility_label(tr("report.type"))
                    .disabled(locked)
                    .w_full(),
                cx,
            ))
            .child(field(
                tr("report.title"),
                Input::new(&self.title)
                    .aria_label(tr("report.title"))
                    .planner_style(cx)
                    .disabled(locked),
                cx,
            ))
            .child(field(
                tr("report.description"),
                Textarea::new(&self.description)
                    .h_24()
                    .aria_label(tr("report.description"))
                    .planner_style(cx)
                    .disabled(locked),
                cx,
            ))
            .when(self.kind != IDEA, |view| {
                view.child(field(
                    tr("report.steps_optional"),
                    Textarea::new(&self.steps)
                        .h_24()
                        .aria_label(tr("report.steps"))
                        .planner_style(cx)
                        .disabled(locked),
                    cx,
                ))
                .child(field(
                    tr("report.expected_optional"),
                    Textarea::new(&self.expected)
                        .h_16()
                        .aria_label(tr("report.expected"))
                        .planner_style(cx)
                        .disabled(locked),
                    cx,
                ))
            })
            .child(field(
                tr("report.screenshots"),
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(
                        modal_button(
                            "report-add-images",
                            tr("report.add_image"),
                            ButtonTone::Neutral,
                            cx,
                        )
                        .disabled(locked || self.loading || self.shots.len() >= MAX_SHOTS)
                        .on_click(cx.listener(|this, _, _, cx| this.choose_images(cx))),
                    )
                    .child(
                        modal_button(
                            "report-paste-image",
                            tr("report.paste_image"),
                            ButtonTone::Neutral,
                            cx,
                        )
                        .disabled(locked || self.loading || self.shots.len() >= MAX_SHOTS)
                        .on_click(cx.listener(|this, _, _, cx| this.paste_image(cx))),
                    )
                    .child(
                        div()
                            .text_size(rems(11. / 13.))
                            .text_color(p.faint)
                            .child(trf(
                                "report.image_limit",
                                &[("count", MAX_SHOTS.to_string())],
                            )),
                    ),
                cx,
            ));
        for shot in &self.shots {
            let id = shot.id.clone();
            fields = fields.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(rems(11. / 13.))
                    .text_color(p.muted)
                    .child(div().min_w_0().truncate().child(shot.name.clone()))
                    .child(
                        icon_button(
                            SharedString::from(format!("remove-shot-{id}")),
                            "×",
                            true,
                            cx,
                        )
                        .accessibility_label(trf(
                            "report.remove_image",
                            &[("name", shot.name.clone())],
                        ))
                        .disabled(locked)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.shots.retain(|shot| shot.id != id);
                            cx.notify();
                        })),
                    ),
            );
        }
        fields = fields.child(field(
            tr("report.discord_optional"),
            Input::new(&self.contact)
                .aria_label(tr("report.discord"))
                .planner_style(cx)
                .disabled(locked),
            cx,
        ));
        if let Some((label, _)) = &self.build {
            fields = fields.child(
                div().mt_2().text_color(p.muted).child(
                    Checkbox::new("report-attach-build")
                        .label(trf("report.attach_build", &[("name", label.clone())]))
                        .checked(self.attach)
                        .disabled(locked)
                        .on_click(cx.listener(|this, checked, _, cx| {
                            this.attach = *checked;
                            cx.notify();
                        })),
                ),
            );
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(modal_header(
                modal_eyebrow("bug-report-eyebrow", tr("report.feedback")),
                tr("report.heading"),
                Some(tr("report.subtitle").into()),
                cx,
            ))
            .child(fields)
            .child(
                modal_footer(cx)
                    .child(modal_status(status, status_color))
                    .when(self.sent, |view| {
                        view.child(
                            modal_button("report-done", tr("report.done"), ButtonTone::Primary, cx)
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                    })
                    .when(!self.sent, |view| {
                        view.child(
                            modal_button(
                                "report-send",
                                if self.busy {
                                    tr("report.sending")
                                } else {
                                    tr("report.send")
                                },
                                ButtonTone::Primary,
                                cx,
                            )
                            .loading(self.busy)
                            .disabled(
                                locked
                                    || self.loading
                                    || self.endpoint.is_none()
                                    || validation.is_some(),
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.send(cx))),
                        )
                    }),
            )
    }
}
fn field(label: &'static str, control: impl IntoElement, cx: &App) -> Div {
    div()
        .flex_none()
        .flex()
        .flex_col()
        .gap_2()
        .child(modal_label(label, label, cx))
        .child(control)
}
