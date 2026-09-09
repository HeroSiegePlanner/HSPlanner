use crate::chrome::{self, BottomBar, SaveState, TopBar};
use gpui_kit::base::Selectable;
use gpui_kit::component::{
    Root, Sizable,
    button::Button,
    input::{Input, InputState},
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::{session::Session, storage::Writer};
use hsplanner_library::LibraryView;
use hsplanner_notes::NotesView;
use hsplanner_planner::TreeView;
use hsplanner_ui::controls::PlannerControl;
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

    pub fn label(self) -> &'static str {
        match self {
            Section::Library => "Library",
            Section::Character => "Character",
            Section::Tree => "Tree",
            Section::Ether => "Ether",
            Section::Skills => "Skills",
            Section::Gear => "Gear",
            Section::Merc => "Merc",
            Section::Stats => "Stats",
            Section::Config => "Config",
            Section::Notes => "Notes",
        }
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
        ToggleProfileControls,
        ToggleDebugOverlay
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
    profile_controls: bool,
    name: Entity<InputState>,
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
        hsplanner_ui::theme::apply_zoom(session.state().settings.ui_zoom, cx);
        let session = cx.new(|_| session);
        let library = cx.new(|cx| LibraryView::new(session.clone(), window, cx));
        let tree = cx.new(|cx| TreeView::new(session.clone(), window, cx));
        let ether = cx.new(|cx| TreeView::new_ether(session.clone(), window, cx));
        let gear =
            cx.new(|cx| hsplanner_planner::gear::GearView::new(session.clone(), false, window, cx));
        let merc = cx.new(|cx| {
            hsplanner_planner::mercenary::MercenaryView::new(
                session.clone(),
                tree.clone(),
                window,
                cx,
            )
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
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Build / profile name"));
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
                            session.save_profile()?;
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
                let zoom = hsplanner_ui::theme::normalize_zoom(
                    this.session.read(cx).state().settings.ui_zoom,
                );
                if gpui_kit::component::Theme::global(cx).font_size != px(13. * zoom) {
                    hsplanner_ui::theme::apply_zoom(zoom, cx);
                }
                this.schedule_save(cx);
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
            profile_controls: false,
            name,
            logo: chrome::logo(),
            updater,
            status: "Ready".into(),
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
        self.status = "Unsaved changes".into();
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
                session.save_profile()?;
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
        self.status = "Saving…".into();
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
                            this.status = "Unsaved changes".into();
                        } else {
                            this.status = "Saved".into();
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
        match hsplanner_build::codec::encode(&draft.snapshot, &draft.notes) {
            Ok(code) => {
                cx.write_to_clipboard(ClipboardItem::new_string(code));
                self.status = "Build code copied".into();
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
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
        let (panel, text, border, muted, negative) = (
            palette.panel,
            palette.text,
            palette.border,
            palette.muted,
            palette.negative,
        );
        let session = self.session.read(cx);
        let build = session
            .draft()
            .build_id
            .as_deref()
            .and_then(|id| session.state().library.build(id))
            .cloned();
        let active = session.draft().profile_id.clone();
        let title = build
            .as_ref()
            .map(|b| b.name.clone())
            .unwrap_or_else(|| "Unsaved build".into());
        let mut profiles = div()
            .flex()
            .gap_2()
            .items_center()
            .child(div().text_color(muted).child(title.clone()));
        if let Some(build) = build {
            for profile in build.profiles {
                let build_id = build.id.clone();
                let id = profile.id.clone();
                profiles = profiles.child(
                    Button::new(SharedString::from(format!("profile-{id}")))
                        .planner_style(cx)
                        .small()
                        .label(profile.name)
                        .selected(active.as_ref() == Some(&id))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply(cx, |session| session.open(&build_id, Some(&id)))
                        })),
                );
            }
        }
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
            .on_action(cx.listener(|this, _: &ToggleProfileControls, _, cx| {
                this.profile_controls = !this.profile_controls;
                cx.notify();
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
                .document(title, auto_save, self.profile_controls)
                .ui_zoom(self.session.read(cx).state().settings.ui_zoom)
                .library_location(self.library.read(cx).location(cx)),
            )
            .when(self.profile_controls, |view| {
                view.child(
                    div()
                        .p_2()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .items_center()
                        .border_b_1()
                        .border_color(border)
                        .child(profiles)
                        .child(
                            div()
                                .w(rems(15.))
                                .child(Input::new(&self.name).planner_style(cx)),
                        )
                        .child(
                            Button::new("save")
                                .planner_style(cx)
                                .small()
                                .label("Save")
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.persist(false, true, cx)),
                                ),
                        )
                        .child(
                            Button::new("save-as")
                                .planner_style(cx)
                                .small()
                                .label("Save as")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let name = this.name.read(cx).value().to_string();
                                    this.apply(cx, |s| s.save_as(&name).map(|_| ()));
                                })),
                        )
                        .child(
                            Button::new("new-profile")
                                .planner_style(cx)
                                .small()
                                .label("Add profile")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let name = this.name.read(cx).value().to_string();
                                    this.apply(cx, |s| s.add_profile(&name, None));
                                })),
                        )
                        .child(
                            Button::new("rename-profile")
                                .planner_style(cx)
                                .small()
                                .label("Rename profile")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let name = this.name.read(cx).value().to_string();
                                    let id = this.session.read(cx).draft().profile_id.clone();
                                    if let Some(id) = id {
                                        this.apply(cx, |s| s.rename_profile(&id, &name));
                                    }
                                })),
                        )
                        .child(
                            Button::new("delete-profile")
                                .planner_style(cx)
                                .small()
                                .label("Delete profile")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let id = this.session.read(cx).draft().profile_id.clone();
                                    if let Some(id) = id {
                                        this.apply(cx, |s| s.remove_profile(&id));
                                    }
                                })),
                        ),
                )
            })
            .children(self.error.as_ref().map(|error| {
                div()
                    .px_3()
                    .py_2()
                    .text_color(negative)
                    .child(error.clone())
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
                    .child(div().flex_1().min_w_0().h_full().child(content)),
            )
            .child(BottomBar::new(
                save,
                (self.status != "Saved" && self.status != "Ready")
                    .then(|| SharedString::from(self.status.clone())),
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
            cx.set_http_client(std::sync::Arc::new(
                reqwest_client::ReqwestClient::user_agent(crate::update::USER_AGENT)
                    .expect("http client"),
            ));
            cx.bind_keys([
                KeyBinding::new("secondary-s", Save, Some("Planner")),
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
                        Menu::new("HSPlanner").items([MenuItem::action("Quit HSPlanner", Quit)])
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
