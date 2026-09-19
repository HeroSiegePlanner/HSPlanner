use crate::chrome::{self, BottomBar, SaveState, TopBar};
use crate::loadouts::LoadoutBar;
use gpui_kit::component::{
    Root, WindowExt,
    button::Button,
    dialog::Confirm,
    input::{Input, InputState},
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::{loadout::LoadoutKind, session::Session, storage::Writer};
use hsplanner_library::LibraryView;
use hsplanner_notes::NotesView;
use hsplanner_planner::TreeView;
use hsplanner_ui::controls::PlannerControl;
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::theme::TooltipTheme;
use std::{rc::Rc, sync::Arc, time::Duration};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Section {
    Library,
    Character,
    Tree,
    Ether,
    Skills,
    Gear,
    Merc,
    Stats,
    Config,
    Notes,
}

#[derive(Clone, PartialEq, Action)]
#[action(no_json)]
pub struct SetUiZoom {
    pub(crate) zoom: f32,
}

#[derive(Clone, PartialEq, Action)]
#[action(no_json)]
pub struct SelectSection {
    pub(crate) section: Section,
}

impl Section {
    pub const NAV: [Section; 9] = [
        Section::Character,
        Section::Tree,
        Section::Ether,
        Section::Skills,
        Section::Gear,
        Section::Merc,
        Section::Stats,
        Section::Config,
        Section::Notes,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Section::Library => "shell.library",
            Section::Character => "shell.character",
            Section::Tree => "shell.tree",
            Section::Ether => "shell.ether",
            Section::Skills => "shell.skills",
            Section::Gear => "shell.gear",
            Section::Merc => "shell.merc",
            Section::Stats => "shell.stats",
            Section::Config => "shell.config",
            Section::Notes => "shell.notes",
        }
    }

    pub fn label(self) -> &'static str {
        tr(self.key())
    }
}
gpui_kit::actions!(
    planner_app,
    [
        Save,
        Undo,
        Redo,
        Quit,
        ToggleAutoSave,
        SaveAs,
        ToggleDebugOverlay,
        OpenSettings
    ]
);

pub struct Shell {
    session: Entity<Session>,
    writer: Writer,
    library: Entity<LibraryView>,
    tree: Entity<TreeView>,
    ether: Entity<TreeView>,
    notes: Entity<NotesView>,
    gear: Entity<hsplanner_planner::gear::GearView>,
    merc: Entity<hsplanner_planner::mercenary::MercenaryView>,
    character: Entity<hsplanner_planner::character::CharacterView>,
    config: Entity<hsplanner_planner::config::ConfigView>,
    skills: Entity<hsplanner_planner::skills::SkillsView>,
    stats: Entity<hsplanner_planner::stats::StatsView>,
    sidebar: Entity<hsplanner_planner::stats_sidebar::StatsSidebar>,
    section: Section,
    logo: Arc<Image>,
    updater: Entity<crate::update::Updater>,
    status: String,
    error: Option<String>,
    save_task: Option<Task<()>>,
    saved_flash: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
    closing: bool,
    saving: bool,
    save_again: bool,
    manual_save_pending: bool,
    review_geometry: Option<(Size<Pixels>, Pixels)>,
    focus: FocusHandle,
    #[cfg(debug_assertions)]
    debug: Entity<hsplanner_ui::debug_overlay::DebugOverlay>,
}
impl Shell {
    pub fn new(
        session: Session,
        writer: Writer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        hsplanner_ui::i18n::apply_language(&session.state().settings.language, cx);
        hsplanner_ui::theme::apply_zoom(session.state().settings.ui_zoom, cx);
        let session = cx.new(|_| session);
        let loadout_bars = [
            LoadoutKind::Incarnation,
            LoadoutKind::Ether,
            LoadoutKind::Gear,
            LoadoutKind::Skills,
        ]
        .map(|kind| cx.new(|cx| LoadoutBar::new(session.clone(), kind, window, cx)));
        let library = cx.new(|cx| LibraryView::new(session.clone(), window, cx));
        let tree = cx.new(|cx| {
            TreeView::new(session.clone(), window, cx).with_header_controls(loadout_bars[0].clone())
        });
        let ether = cx.new(|cx| {
            TreeView::new_ether(session.clone(), window, cx)
                .with_header_controls(loadout_bars[1].clone())
        });
        let gear = cx.new(|cx| {
            hsplanner_planner::gear::GearView::new(session.clone(), false, window, cx)
                .with_header_controls(loadout_bars[2].clone())
        });
        let merc = cx.new(|cx| {
            hsplanner_planner::mercenary::MercenaryView::new(
                session.clone(),
                tree.clone(),
                window,
                cx,
            )
            .with_header_controls(loadout_bars[2].clone())
        });
        let notes = cx.new(|cx| NotesView::new(session.clone(), window, cx));
        let character = cx.new(|cx| {
            hsplanner_planner::character::CharacterView::new(
                session.clone(),
                tree.clone(),
                window,
                cx,
            )
        });
        let config = cx.new(|cx| {
            hsplanner_planner::config::ConfigView::new(session.clone(), tree.clone(), window, cx)
        });
        let skills = cx.new(|cx| {
            hsplanner_planner::skills::SkillsView::new(session.clone(), tree.clone(), window, cx)
                .with_header_controls(loadout_bars[3].clone())
        });
        let stats = cx.new(|cx| {
            hsplanner_planner::stats::StatsView::new(session.clone(), tree.clone(), window, cx)
        });
        let sidebar = cx.new(|cx| {
            hsplanner_planner::stats_sidebar::StatsSidebar::new(
                session.clone(),
                tree.clone(),
                window,
                cx,
            )
        });
        let updater = cx.new(crate::update::Updater::new);
        let subscriptions = vec![
            cx.subscribe(&updater, |this, _, _: &crate::update::Installed, cx| {
                this.request_close(cx)
            }),
            cx.observe(&updater, |_, _, cx| cx.notify()),
            cx.on_app_quit(|this, cx| {
                // The platform may quit without closing the window (for example from the Dock).
                // Complete the same serialized writer before GPUI releases the document.
                if this.session.read(cx).is_dirty() {
                    let result = this.session.update(cx, |session, _| {
                        if session.state().settings.auto_save {
                            session.save_build()?;
                        }
                        this.writer.save(session.state())?;
                        this.writer.flush()
                    });
                    if let Err(error) = result {
                        log::error!("Could not finish saving on exit: {error}");
                    }
                }
                async {}
            }),
            cx.observe(&session, |this, _, cx| {
                let language = this.session.read(cx).state().settings.language.clone();
                hsplanner_ui::i18n::apply_language(&language, cx);
                let zoom = hsplanner_ui::theme::normalize_zoom(
                    this.session.read(cx).state().settings.ui_zoom,
                );
                if gpui_kit::component::Theme::global(cx).font_size != px(13. * zoom) {
                    hsplanner_ui::theme::apply_zoom(zoom, cx);
                }
                this.schedule_save(cx);
                cx.notify();
            }),
            cx.observe_global_in::<hsplanner_ui::i18n::Locale>(window, |_, _, cx| {
                cx.set_menus([Menu::new("HSPlanner")
                    .items([MenuItem::action(hsplanner_ui::i18n::tr("shell.quit"), Quit)])]);
                cx.notify();
            }),
            cx.observe(&library, |_, _, cx| cx.notify()),
            cx.subscribe_in(
                &library,
                window,
                |this, _, _: &hsplanner_library::Opened, window, cx| {
                    this.switch(Section::Tree, window, cx);
                    cx.notify();
                },
            ),
        ];
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            session,
            writer,
            library,
            tree,
            ether,
            notes,
            gear,
            merc,
            character,
            config,
            skills,
            stats,
            sidebar,
            section: Section::Library,
            logo: chrome::logo(),
            updater,
            status: "shell.ready".into(),
            error: None,
            save_task: None,
            saved_flash: None,
            _subscriptions: subscriptions,
            closing: false,
            saving: false,
            save_again: false,
            manual_save_pending: false,
            review_geometry: None,
            focus,
            #[cfg(debug_assertions)]
            debug: cx.new(|cx| {
                hsplanner_ui::debug_overlay::DebugOverlay::new(
                    std::env::var_os("HSPLANNER_DIAGNOSTICS").is_some(),
                    cx,
                )
            }),
        }
    }
    fn apply(
        &mut self,
        cx: &mut Context<Self>,
        action: impl FnOnce(&mut Session) -> Result<(), String>,
    ) {
        let result = self.session.update(cx, |session, cx| {
            let result = action(session);
            if result.is_ok() {
                cx.notify();
            }
            result
        });
        self.error = result.err();
        cx.notify();
    }
    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        if !self.session.read(cx).is_dirty() || self.closing {
            return;
        }
        self.status = "shell.unsaved".into();
        self.save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(800))
                .await;
            let _ = this.update(cx, |this, cx| this.persist(false, false, cx));
        }));
    }
    fn persist(&mut self, close: bool, manual: bool, cx: &mut Context<Self>) {
        if self.saving {
            self.save_again = true;
            self.manual_save_pending |= manual;
            self.closing |= close;
            return;
        }
        self.saving = true;
        let prepared = self.session.update(cx, |session, _| {
            if manual || session.state().settings.auto_save {
                session.save_build()?;
            }
            self.writer.save(session.state())
        });
        if let Err(error) = prepared {
            self.saving = false;
            self.error = Some(error);
            self.closing = false;
            cx.notify();
            return;
        }
        self.status = "shell.saving".into();
        let writer = self.writer.clone();
        let flush = cx.background_spawn(async move { writer.flush() });
        cx.spawn(async move |this, cx| {
            let result = flush.await;
            let _ = this.update(cx, |this, cx| {
                this.saving = false;
                match result {
                    Ok(revision) => {
                        this.session
                            .update(cx, |session, _| session.persisted(revision));
                        let dirty = this.session.read(cx).is_dirty();
                        if this.save_again || (this.closing && dirty) {
                            this.save_again = false;
                            let manual = std::mem::take(&mut this.manual_save_pending);
                            this.persist(this.closing, manual, cx);
                        } else if dirty {
                            this.status = "shell.unsaved".into();
                        } else {
                            this.status = "shell.saved".into();
                            this.error = None;
                            if manual {
                                this.flash_saved(cx);
                            }
                            if close || this.closing {
                                cx.quit();
                            }
                        }
                    }
                    Err(error) => {
                        this.error = Some(error);
                        this.closing = false;
                        this.save_again = false;
                        this.manual_save_pending = false;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn flash_saved(&mut self, cx: &mut Context<Self>) {
        self.saved_flash = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1600))
                .await;
            let _ = this.update(cx, |this, cx| {
                this.saved_flash = None;
                cx.notify();
            });
        }));
    }
    fn copy_build_code(&mut self, cx: &mut Context<Self>) {
        let draft = self.session.read(cx).draft();
        match hsplanner_build::codec::encode_loadouts(
            &draft.snapshot,
            &draft.notes,
            &draft.loadouts,
        ) {
            Ok(code) => {
                cx.write_to_clipboard(ClipboardItem::new_string(code));
                self.status = "shell.copied".into();
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }
    fn save_as_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let build_id = self.session.read(cx).draft().build_id.clone();
        let name = build_id
            .as_deref()
            .and_then(|id| self.session.read(cx).state().library.build(id))
            .map(|build| build.name.clone())
            .unwrap_or_else(|| tr("shell.unsaved_build").into());
        let input = cx.new(|cx| {
            let mut input = InputState::new(window, cx);
            input.set_value(name, window, cx);
            input
        });
        let focus = input.read(cx).focus_handle(cx);
        let owner = cx.entity().downgrade();
        // Shell renders the dialog layer, so its builder must not read Shell.
        let dialog_error = cx.new(|_| None::<String>);
        window.open_dialog(cx, move |dialog, _, cx| {
            let target = owner.clone();
            let input_for_save = input.clone();
            let build_id = build_id.clone();
            let error = dialog_error.read(cx).clone();
            let error_for_save = dialog_error.clone();
            dialog
                .title(tr("loadouts.save_as_title"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().child(tr("shell.build_name")))
                        .child(Input::new(&input).planner_style(cx))
                        .children(error.map(|error| {
                            div()
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
                            Button::new("cancel-save-as")
                                .planner_style(cx)
                                .label(tr("component.Dialog.cancel"))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            hsplanner_ui::controls::planner_button(
                                "confirm-save-as",
                                hsplanner_ui::controls::ButtonTone::Primary,
                                cx,
                            )
                            .label(tr("shell.save"))
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(Confirm { secondary: false }), cx)
                            }),
                        ),
                )
                .on_ok(move |_, _, cx| {
                    let name = input_for_save.read(cx).value().to_string();
                    target
                        .update(cx, |this, cx| {
                            this.apply(cx, |session| {
                                if session.draft().build_id != build_id {
                                    return Err(tr("loadouts.changed").into());
                                }
                                if name.trim().is_empty() {
                                    return Err(tr("loadouts.enter_name").into());
                                }
                                session.save_as(&name).map(|_| ())
                            });
                            error_for_save.update(cx, |error, cx| {
                                *error = this.error.clone();
                                cx.notify();
                            });
                            this.error.is_none()
                        })
                        .unwrap_or(false)
                })
        });
        window.focus(&focus, cx);
    }
    pub fn request_close(&mut self, cx: &mut Context<Self>) {
        if !self.closing {
            self.closing = true;
            self.save_task = None;
            self.persist(true, false, cx);
        }
    }
    fn switch(&mut self, section: Section, window: &mut Window, cx: &mut Context<Self>) {
        self.section = section;
        self.stats.update(cx, |view, cx| {
            view.set_active(section == Section::Stats, cx)
        });
        self.skills.update(cx, |view, cx| {
            view.set_active(section == Section::Skills, cx)
        });
        self.library.update(cx, |view, cx| {
            view.set_active(section == Section::Library, cx)
        });
        self.tree
            .update(cx, |view, cx| view.set_active(section == Section::Tree, cx));
        self.ether.update(cx, |view, cx| {
            view.set_active(section == Section::Ether, cx)
        });
        self.gear
            .update(cx, |view, cx| view.set_active(section == Section::Gear, cx));
        self.merc
            .update(cx, |view, cx| view.set_active(section == Section::Merc, cx));
        let focus = match section {
            Section::Tree => self.tree.focus_handle(cx),
            Section::Ether => self.ether.focus_handle(cx),
            _ => self.focus.clone(),
        };
        window.focus(&focus, cx);
        cx.notify();
    }
}
impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if cfg!(debug_assertions) && std::env::var_os("HSPLANNER_REVIEW_SIZE").is_some() {
            let geometry = (window.viewport_size(), window.rem_size());
            if self.review_geometry != Some(geometry) {
                log::info!(
                    "Visual review: viewport={:?}, rem={:?}, device_scale={}",
                    geometry.0,
                    geometry.1,
                    window.scale_factor()
                );
                self.review_geometry = Some(geometry);
            }
        }
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let palette = cx.global::<TooltipTheme>();
        let (panel, text, negative) = (palette.panel, palette.text, palette.negative);
        let session = self.session.read(cx);
        let title = session
            .draft()
            .build_id
            .as_deref()
            .and_then(|id| session.state().library.build(id))
            .map(|b| b.name.clone())
            .unwrap_or_else(|| tr("shell.unsaved_build").into());
        let select = cx.entity().downgrade();
        let auto_save = session.state().settings.auto_save;
        let save = SaveState::derive(
            self.saving,
            session.state().settings.auto_save,
            self.saved_flash.is_some(),
        );
        let content = match self.section {
            Section::Library => self.library.clone().into_any_element(),
            Section::Tree => self.tree.clone().into_any_element(),
            Section::Ether => self.ether.clone().into_any_element(),
            Section::Notes => self.notes.clone().into_any_element(),
            Section::Gear => self.gear.clone().into_any_element(),
            Section::Merc => self.merc.clone().into_any_element(),
            Section::Character => self.character.clone().into_any_element(),
            Section::Config => self.config.clone().into_any_element(),
            Section::Skills => self.skills.clone().into_any_element(),
            Section::Stats => self.stats.clone().into_any_element(),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(panel)
            .text_color(text)
            .font_family(hsplanner_ui::theme::FONT_FAMILY)
            .line_height(relative(1.5))
            .key_context("Planner")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, action: &SelectSection, window, cx| {
                this.switch(action.section, window, cx);
            }))
            .on_action(cx.listener(|this, _: &Save, _, cx| this.persist(false, true, cx)))
            .on_action(cx.listener(|this, _: &Undo, _, cx| {
                this.apply(cx, |s| {
                    s.undo();
                    Ok(())
                })
            }))
            .on_action(cx.listener(|this, _: &Redo, _, cx| {
                this.apply(cx, |s| {
                    s.redo();
                    Ok(())
                })
            }))
            .on_action(cx.listener(|this, _: &Quit, _, cx| this.request_close(cx)))
            .on_action(cx.listener(|this, _: &ToggleDebugOverlay, _, cx| {
                #[cfg(debug_assertions)]
                this.debug.update(cx, |overlay, cx| overlay.toggle(cx));
                #[cfg(not(debug_assertions))]
                let _ = (this, cx);
            }))
            .on_action(cx.listener(|this, _: &SaveAs, window, cx| this.save_as_dialog(window, cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                crate::settings::open(this.session.clone(), window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleAutoSave, _, cx| {
                this.apply(cx, |session| {
                    let mut settings = session.state().settings.clone();
                    settings.auto_save = !settings.auto_save;
                    session.set_settings(settings);
                    Ok(())
                });
            }))
            .on_action(cx.listener(|this, action: &SetUiZoom, _, cx| {
                this.apply(cx, |session| {
                    let mut settings = session.state().settings.clone();
                    settings.ui_zoom = hsplanner_ui::theme::normalize_zoom(action.zoom);
                    session.set_settings(settings);
                    Ok(())
                });
            }))
            .child(
                TopBar::new(
                    self.logo.clone(),
                    self.section,
                    Rc::new(move |section, window, cx| {
                        let _ = select.update(cx, |this, cx| this.switch(section, window, cx));
                    }),
                    Box::new(
                        cx.listener(|this, _, window, cx| {
                            this.switch(Section::Library, window, cx)
                        }),
                    ),
                    Box::new(cx.listener(|this, _, _, cx| this.copy_build_code(cx))),
                )
                .document(title, auto_save)
                .ui_zoom(self.session.read(cx).state().settings.ui_zoom)
                .library_location(self.library.read(cx).location(cx)),
            )
            .children(self.error.as_ref().map(|error| {
                div()
                    .px_3()
                    .py_2()
                    .text_color(negative)
                    .child(trf("shell.action_failed", &[("detail", error.clone())]))
            }))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .items_stretch()
                    .when(self.section != Section::Library, |row| {
                        row.child(
                            div()
                                .w_72()
                                .h_full()
                                .flex_shrink_0()
                                .child(self.sidebar.clone()),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(div().flex_1().min_h_0().child(content)),
                    ),
            )
            .child(BottomBar::new(
                self.session.clone(),
                save,
                (self.status != "shell.saved" && self.status != "shell.ready")
                    .then(|| SharedString::from(tr(&self.status))),
                self.updater.clone(),
            ))
            .children(dialog_layer)
            .map(|root| {
                #[cfg(debug_assertions)]
                let root = root.child(self.debug.clone());
                root
            })
    }
}

pub fn run(
    directory: std::path::PathBuf,
    loaded: Result<(Writer, hsplanner_build::session::WorkspaceState), String>,
) {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            hsplanner_ui::theme::init(cx);
            hsplanner_ui::scroll::init(cx);
            cx.set_http_client(std::sync::Arc::new(
                reqwest_client::ReqwestClient::user_agent(crate::update::USER_AGENT)
                    .expect("http client"),
            ));
            cx.bind_keys([
                KeyBinding::new("secondary-s", Save, Some("Planner")),
                KeyBinding::new("secondary-shift-s", SaveAs, Some("Planner")),
                KeyBinding::new("secondary-z", Undo, Some("Planner")),
                KeyBinding::new("secondary-shift-z", Redo, Some("Planner")),
                KeyBinding::new("secondary-q", Quit, None),
                #[cfg(debug_assertions)]
                KeyBinding::new("secondary-shift-d", ToggleDebugOverlay, None),
            ]);
            let review_size = cfg!(debug_assertions)
                .then(|| std::env::var("HSPLANNER_REVIEW_SIZE").ok())
                .flatten()
                .and_then(|value| {
                    let (width, height) = value.split_once('x')?;
                    let width = width.parse::<f32>().ok()?;
                    let height = height.parse::<f32>().ok()?;
                    (width.is_finite() && height.is_finite() && width >= 960. && height >= 600.)
                        .then_some(size(px(width), px(height)))
                });
            let bounds =
                Bounds::centered(None, review_size.unwrap_or(size(px(1440.), px(960.))), cx);
            let lock_review_size =
                review_size.is_some() && std::env::var_os("HSPLANNER_REVIEW_LOCK_SIZE").is_some();
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(960.), px(600.))),
                    is_resizable: !lock_review_size,
                    titlebar: Some(TitlebarOptions {
                        title: Some(if lock_review_size {
                            "HSPlanner · Visual review".into()
                        } else {
                            "HSPlanner".into()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let shell =
                        cx.new(|cx| crate::startup::Startup::new(directory, loaded, window, cx));
                    let weak = shell.downgrade();
                    let quit = weak.clone();
                    cx.on_action(move |_: &Quit, cx| {
                        let _ = quit.update(cx, |shell, cx| shell.request_close(cx));
                    });
                    cx.set_menus([
                        Menu::new("HSPlanner").items([MenuItem::action(tr("shell.quit"), Quit)])
                    ]);
                    window.on_window_should_close(cx, move |_, cx| {
                        let _ = weak.update(cx, |shell, cx| shell.request_close(cx));
                        false
                    });
                    cx.new(|cx| Root::new(shell, window, cx))
                },
            )
            .expect("open HSPlanner window");
            cx.activate(true);
        });
}
