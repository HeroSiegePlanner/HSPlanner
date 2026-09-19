//! Category-scoped loadout controls; the session owns selection, content and history.
use gpui_kit::{
    component::{
        Icon, IconName, IndexPath, Sizable, WindowExt,
        button::Button,
        dialog::Confirm,
        input::{Input, InputState},
        menu::{DropdownMenu, PopupMenuItem},
        select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState},
    },
    prelude::*,
    *,
};
use hsplanner_build::{loadout::LoadoutKind, session::Session};
use hsplanner_ui::{
    controls::{ButtonTone, PlannerControl, planner_button},
    i18n::{tr, trf},
    theme::TooltipTheme,
    tooltip::CursorTooltipExt,
};

#[derive(Clone, PartialEq, Eq)]
struct Choice {
    id: String,
    name: String,
}
impl SelectItem for Choice {
    type Value = String;
    fn title(&self) -> SharedString {
        self.name.clone().into()
    }
    fn value(&self) -> &String {
        &self.id
    }
}

pub(super) struct LoadoutBar {
    session: Entity<Session>,
    kind: LoadoutKind,
    select: Entity<SelectState<SearchableVec<Choice>>>,
    choices: Vec<Choice>,
    active: String,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl LoadoutBar {
    pub(super) fn new(
        session: Entity<Session>,
        kind: LoadoutKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let choices = Self::choices(session.read(cx), kind);
        let active = session.read(cx).draft().loadouts.active_id(kind).to_owned();
        let selected = choices.iter().position(|choice| choice.id == active);
        let select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(choices.clone()),
                selected.map(IndexPath::new),
                window,
                cx,
            )
            .searchable(true)
        });
        let subscriptions = vec![
            cx.observe_in(&session, window, |this, _, window, cx| {
                this.sync(window, cx)
            }),
            cx.subscribe_in(
                &select,
                window,
                |this, _, event: &SelectEvent<SearchableVec<Choice>>, window, cx| {
                    if let SelectEvent::Confirm(Some(id)) = event {
                        let kind = this.kind;
                        this.apply(cx, |session| session.switch_loadout(kind, id));
                        this.sync(window, cx);
                    }
                },
            ),
        ];
        Self {
            session,
            kind,
            select,
            choices,
            active,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn choices(session: &Session, kind: LoadoutKind) -> Vec<Choice> {
        session
            .draft()
            .loadouts
            .entries(kind)
            .into_iter()
            .map(|(id, name)| Choice {
                id: id.to_owned(),
                name: name.to_owned(),
            })
            .collect()
    }

    fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let choices = Self::choices(self.session.read(cx), self.kind);
        let active = self
            .session
            .read(cx)
            .draft()
            .loadouts
            .active_id(self.kind)
            .to_owned();
        if self.choices != choices
            || self.active != active
            || self.select.read(cx).selected_value() != Some(&active)
        {
            self.select.update(cx, |select, cx| {
                select.set_items(SearchableVec::new(choices.clone()), window, cx);
                select.set_selected_value(&active, window, cx);
                cx.notify();
            });
            self.choices = choices;
            self.active = active;
        }
        cx.notify();
    }

    fn apply(
        &mut self,
        cx: &mut Context<Self>,
        action: impl FnOnce(&mut Session) -> Result<(), String>,
    ) -> bool {
        let result = self.session.update(cx, |session, cx| {
            let result = action(session);
            if result.is_ok() {
                cx.notify();
            }
            result
        });
        self.error = result.err();
        cx.notify();
        self.error.is_none()
    }

    fn name_dialog(&mut self, duplicate: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let draft = self.session.read(cx).draft();
        let build_id = draft.build_id.clone();
        let active_id = draft.loadouts.active_id(self.kind).to_owned();
        let name = draft.loadouts.active_name(self.kind).to_owned();
        let kind = self.kind;
        let input = cx.new(|cx| {
            let mut input = InputState::new(window, cx);
            input.set_value(
                if duplicate {
                    trf("loadouts.copy_name", &[("name", name)])
                } else {
                    name
                },
                window,
                cx,
            );
            input
        });
        let focus = input.read(cx).focus_handle(cx);
        let owner = cx.entity().downgrade();
        // The menu is transient; return dialog focus to the retained selector.
        window.focus(&self.select.read(cx).focus_handle(cx), cx);
        window.open_dialog(cx, move |dialog, _, cx| {
            let target = owner.clone();
            let input_for_save = input.clone();
            let build_id = build_id.clone();
            let active_id = active_id.clone();
            let error = owner.upgrade().and_then(|view| view.read(cx).error.clone());
            let commit = if duplicate {
                "loadouts.duplicate_commit"
            } else {
                "loadouts.rename_commit"
            };
            dialog
                .title(tr(if duplicate {
                    "loadouts.duplicate_title"
                } else {
                    "loadouts.rename_title"
                }))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().text_sm().child(tr("loadouts.name")))
                        .child(Input::new(&input).planner_style(cx))
                        .children(error.map(|error| {
                            div()
                                .text_sm()
                                .text_color(cx.global::<TooltipTheme>().negative)
                                .child(error)
                        })),
                )
                .footer(
                    div()
                        .flex()
                        .justify_end()
                        .gap_2()
                        .child(
                            Button::new("cancel-loadout-name")
                                .planner_style(cx)
                                .label(tr("component.Dialog.cancel"))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            planner_button("confirm-loadout-name", ButtonTone::Primary, cx)
                                .label(tr(commit))
                                .on_click(|_, window, cx| {
                                    window
                                        .dispatch_action(Box::new(Confirm { secondary: false }), cx)
                                }),
                        ),
                )
                .on_ok(move |_, _, cx| {
                    let name = input_for_save.read(cx).value().to_string();
                    target
                        .update(cx, |this, cx| {
                            this.apply(cx, |session| {
                                if session.draft().build_id != build_id
                                    || session.draft().loadouts.active_id(kind) != active_id
                                {
                                    return Err(tr("loadouts.changed").into());
                                }
                                if name.trim().is_empty() {
                                    return Err(tr("loadouts.enter_name").into());
                                }
                                if duplicate {
                                    session.add_loadout(kind, &name)
                                } else {
                                    session.rename_loadout(kind, &active_id, &name)
                                }
                            })
                        })
                        .unwrap_or(false)
                })
        });
        window.focus(&focus, cx);
    }
}

pub(super) fn category_key(kind: LoadoutKind) -> &'static str {
    match kind {
        LoadoutKind::Incarnation => "loadouts.incarnation",
        LoadoutKind::Ether => "loadouts.ether",
        LoadoutKind::Gear => "loadouts.gear",
        LoadoutKind::Skills => "loadouts.skills",
    }
}

impl Render for LoadoutBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let category = tr(category_key(self.kind));
        let label = trf("loadouts.category", &[("category", category.to_string())]);
        let actions = trf("loadouts.actions", &[("category", category.to_string())]);
        let name = self
            .session
            .read(cx)
            .draft()
            .loadouts
            .active_name(self.kind)
            .to_owned();
        let owner = cx.entity().downgrade();
        let count = self.choices.len();
        div()
            .id(category_key(self.kind))
            .flex_none()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        // Select fills its parent; this compact frame also keeps
                        // long loadout names from pushing the view's commands out.
                        div()
                            .w_56()
                            .h_6()
                            .flex_none()
                            .cursor_tooltip(format!("{label}: {name}"))
                            .child(
                                Select::new(&self.select)
                                    .small()
                                    .w_full()
                                    .bg(p.panel)
                                    .border_color(p.border)
                                    .accessibility_label(label),
                            ),
                    )
                    .child(
                        planner_button("loadout-actions", ButtonTone::Neutral, cx)
                            .small()
                            .h_6()
                            .w_8()
                            .p_0()
                            .bg(p.panel)
                            .border_color(p.border_strong)
                            .text_color(p.text)
                            .child(Icon::new(IconName::Ellipsis).size_4())
                            .accessibility_label(actions.clone())
                            .cursor_tooltip(actions)
                            .dropdown_menu(move |menu, _, _| {
                                let duplicate = owner.clone();
                                let rename = owner.clone();
                                let delete = owner.clone();
                                menu.item(PopupMenuItem::label(name.clone()))
                                    .item(
                                        PopupMenuItem::new(tr("loadouts.duplicate"))
                                            .disabled(count >= 100)
                                            .on_click(move |_, window, cx| {
                                                let _ = duplicate.update(cx, |this, cx| {
                                                    this.name_dialog(true, window, cx);
                                                });
                                            }),
                                    )
                                    .item(PopupMenuItem::new(tr("loadouts.rename")).on_click(
                                        move |_, window, cx| {
                                            let _ = rename.update(cx, |this, cx| {
                                                this.name_dialog(false, window, cx);
                                            });
                                        },
                                    ))
                                    .separator()
                                    .item(
                                        PopupMenuItem::new(tr("loadouts.delete"))
                                            .disabled(count <= 1)
                                            .on_click(move |_, _, cx| {
                                                let _ = delete.update(cx, |this, cx| {
                                                    let kind = this.kind;
                                                    let id = this.active.clone();
                                                    this.apply(cx, |session| {
                                                        session.remove_loadout(kind, &id)
                                                    });
                                                });
                                            }),
                                    )
                            }),
                    ),
            )
            .children(self.error.as_ref().map(|error| {
                div()
                    .max_w_64()
                    .text_sm()
                    .text_color(p.negative)
                    .child(error.clone())
            }))
    }
}
