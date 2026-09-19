use hsplanner_ui::i18n::{Locale, tr, trf};
use hsplanner_ui::scroll::PageScroll;
use hsplanner_ui::tooltip::CursorTooltipExt;
mod dialogs;
mod query;
use dialogs::EditKind;
mod presentation;
use gpui_kit::base::{Disableable, Selectable};
use gpui_kit::component::{
    Sizable,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::{library::SavedBuild, session::Session};
use hsplanner_ui::controls::PlannerControl;
use hsplanner_ui::theme::TooltipTheme;
use presentation::{label, navigation, portrait, toolbar_button};
use std::{cell::RefCell, cmp::Reverse};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SortColumn {
    Favorite,
    Name,
    Class,
    Level,
    #[default]
    Modified,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SortDirection {
    Ascending,
    #[default]
    Descending,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LibrarySort {
    column: SortColumn,
    direction: SortDirection,
}

impl LibrarySort {
    fn select(&mut self, column: SortColumn) {
        self.direction = if self.column == column {
            match self.direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            }
        } else if matches!(column, SortColumn::Name | SortColumn::Class) {
            SortDirection::Ascending
        } else {
            SortDirection::Descending
        };
        self.column = column;
    }

    fn apply_indices(self, indices: &mut Vec<usize>, builds: &[SavedBuild], recent: bool) {
        if recent {
            indices.sort_by(|&a, &b| builds[b].updated_at.cmp(&builds[a].updated_at));
            indices.truncate(12);
        }
        match self.direction {
            SortDirection::Ascending => indices.sort_by_cached_key(|&i| self.key(&builds[i])),
            SortDirection::Descending => {
                indices.sort_by_cached_key(|&i| Reverse(self.key(&builds[i])))
            }
        }
    }

    #[cfg(test)]
    fn apply(self, builds: &mut Vec<SavedBuild>, recent: bool) {
        let mut indices: Vec<_> = (0..builds.len()).collect();
        self.apply_indices(&mut indices, builds, recent);
        *builds = indices.into_iter().map(|i| builds[i].clone()).collect();
    }

    fn key(self, build: &SavedBuild) -> SortKey {
        match self.column {
            SortColumn::Favorite => SortKey::Number(u32::from(build.favorite)),
            SortColumn::Name => SortKey::Text(build.name.to_lowercase()),
            SortColumn::Class => SortKey::Text(
                build
                    .class_id
                    .as_deref()
                    .and_then(hsplanner_engine::calc::data::get_class)
                    .map(|class| class.name.as_str())
                    .unwrap_or(tr("library.unknown"))
                    .to_lowercase(),
            ),
            SortColumn::Level => SortKey::Number(query::build_summary(build).0),
            SortColumn::Modified => SortKey::Text(build.updated_at.clone()),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SortKey {
    Number(u32),
    Text(String),
}

pub struct Opened;
pub struct LibraryView {
    session: Entity<Session>,
    search: Entity<InputState>,
    name: Entity<InputState>,
    tags: Entity<InputState>,
    code: Entity<InputState>,
    selected: Option<String>,
    folder: Option<String>,
    favorites: bool,
    sort: LibrarySort,
    page: usize,
    error: Option<String>,
    preview_snapshot: Option<hsplanner_build::BuildSnapshot>,
    performance: Option<hsplanner_engine::calc::planner::PlannerPerformance>,
    preview_task: Option<Task<()>>,
    preview_revision: u64,
    active: bool,
    editing: bool,
    importing: bool,
    recent: bool,
    high_level: bool,
    unfiled: bool,
    active_tag: Option<String>,
    columns_scroll: ScrollHandle,
    query_cache: RefCell<query::Cache>,
    list_scroll: ScrollHandle,
    preview_scroll: ScrollHandle,
    folder_scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<Opened> for LibraryView {}

/// Translate domain diagnostics at the presentation boundary, keeping stored errors
/// independent of the current language and preserving low-level details.
fn render_error(error: &str) -> String {
    match error {
        "library.select_folder" | "Select a folder" => tr("library.select_folder").to_owned(),
        "library.select_build" | "Select a build" => tr("library.select_build").to_owned(),
        "library.error.build_limit" | "The library already contains 1,000 builds." => {
            tr("library.error.build_limit").to_owned()
        }
        "library.error.folder_limit" | "The library already contains 500 folders." => {
            tr("library.error.folder_limit").to_owned()
        }
        "library.error.missing_folder" | "Folder no longer exists." => {
            tr("library.error.missing_folder").to_owned()
        }
        "library.error.missing_parent" | "Parent folder no longer exists." => {
            tr("library.error.missing_parent").to_owned()
        }
        "library.error.missing_build" | "Build no longer exists." => {
            tr("library.error.missing_build").to_owned()
        }
        "library.error.enter_name" | "Enter a name." => tr("library.error.enter_name").to_owned(),
        "library.error.save_first" | "Save this build first." => {
            tr("library.error.save_first").to_owned()
        }
        "library.error.share_size" | "This build is too large to share as a code." => {
            tr("library.error.share_size").to_owned()
        }
        "library.error.code_size" | "Build code is too large." => {
            tr("library.error.code_size").to_owned()
        }
        "library.error.invalid_code" | "Invalid build code." => {
            tr("library.error.invalid_code").to_owned()
        }
        "library.error.expanded_size" | "Build code expands beyond the supported size." => {
            tr("library.error.expanded_size").to_owned()
        }
        "library.error.invalid_text" | "Invalid text in build code." => {
            tr("library.error.invalid_text").to_owned()
        }
        "library.error.invalid_data" | "Invalid build data." => {
            tr("library.error.invalid_data").to_owned()
        }
        "library.error.code_version" | "Unsupported build code version." => {
            tr("library.error.code_version").to_owned()
        }
        "library.error.incomplete_code" | "Incomplete build code." => {
            tr("library.error.incomplete_code").to_owned()
        }
        "library.error.tree_nodes" | "Too many tree nodes." => {
            tr("library.error.tree_nodes").to_owned()
        }
        "library.error.equipment" | "Invalid equipment list." => {
            tr("library.error.equipment").to_owned()
        }
        "library.error.slots" | "Too many equipment slots." => tr("library.error.slots").to_owned(),
        "library.error.item" | "Invalid equipped item." => tr("library.error.item").to_owned(),
        "library.error.escaped_code" | "Invalid escaped build code." => {
            tr("library.error.escaped_code").to_owned()
        }
        "library.error.code_text" | "Invalid build code text." => {
            tr("library.error.code_text").to_owned()
        }
        _ => {
            if let Some(field) = error.strip_prefix("Missing or invalid build field: ") {
                trf("library.error.build_field", &[("field", field.to_owned())])
            } else if let Some(detail) = error.strip_prefix("Invalid build fields: ") {
                trf(
                    "library.error.build_fields",
                    &[("detail", detail.to_owned())],
                )
            } else {
                trf("library.error.action", &[("detail", error.to_owned())])
            }
        }
    }
}

fn localized_input(
    placeholder: &'static str,
    window: &mut Window,
    cx: &mut Context<InputState>,
) -> InputState {
    // The observer follows the input entity, including inputs hosted in dialogs.
    // Updating only the placeholder preserves text, selection and undo history.
    cx.observe_global_in::<Locale>(window, move |input, window, cx| {
        input.set_placeholder(tr(placeholder), window, cx);
    })
    .detach();
    InputState::new(window, cx).placeholder(tr(placeholder))
}
impl LibraryView {
    pub fn new(session: Entity<Session>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| localized_input("library.search_placeholder", window, cx));
        let name = cx.new(|cx| localized_input("library.name_placeholder", window, cx));
        let tags = cx.new(|cx| localized_input("library.tags_placeholder", window, cx));
        let code = cx.new(|cx| localized_input("library.code_placeholder", window, cx));
        let subscriptions = vec![
            cx.observe(&session, |this, _, cx| {
                this.query_cache.get_mut().invalidate();
                if this.active && !this.reconcile_selection(cx) {
                    this.refresh_preview(cx);
                }
                cx.notify();
            }),
            cx.subscribe(&search, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.page = 0;
                    this.reconcile_selection(cx);
                    cx.notify();
                }
            }),
        ];
        let initial = session
            .read(cx)
            .draft()
            .build_id
            .as_ref()
            .and_then(|id| session.read(cx).state().library.build(id))
            .or_else(|| session.read(cx).state().library.builds.first())
            .cloned();
        let mut view = Self {
            session,
            search,
            name,
            tags,
            code,
            selected: None,
            folder: None,
            favorites: false,
            sort: LibrarySort::default(),
            page: 0,
            error: None,
            preview_snapshot: None,
            performance: None,
            preview_task: None,
            preview_revision: 0,
            active: true,
            editing: false,
            importing: false,
            recent: true,
            high_level: false,
            unfiled: false,
            active_tag: None,
            columns_scroll: ScrollHandle::default(),
            query_cache: RefCell::default(),
            list_scroll: ScrollHandle::new(),
            preview_scroll: ScrollHandle::new(),
            folder_scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        };
        if let Some(build) = initial {
            view.select(&build, window, cx);
        }
        view
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
        if self.error.is_none() {
            self.query_cache.get_mut().invalidate();
        }
        cx.notify();
        self.error.is_none()
    }
    fn select(&mut self, build: &SavedBuild, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = Some(build.id.clone());
        self.refresh_preview(cx);
        self.name.update(cx, |input, cx| {
            input.set_value(build.name.clone(), window, cx)
        });
        self.tags.update(cx, |input, cx| {
            input.set_value(build.tags.join(", "), window, cx)
        });
        cx.notify();
    }
    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.active == active {
            return;
        }
        self.active = active;
        self.preview_revision += 1;
        self.preview_task = None;
        if active && !self.reconcile_selection(cx) {
            self.refresh_preview(cx);
        }
        cx.notify();
    }

    fn filtered_builds<'a>(&self, cx: &'a App) -> Vec<&'a SavedBuild> {
        let library = &self.session.read(cx).state().library;
        let filter = query::Filter {
            search: self.search.read(cx).value().trim().to_lowercase(),
            favorites: self.favorites,
            unfiled: self.unfiled,
            folder: self.folder.clone(),
            tag: self.active_tag.clone(),
            high_level: self.high_level,
            recent: self.recent,
            sort: self.sort,
        };
        self.query_cache
            .borrow_mut()
            .indices(&library.builds, filter)
            .iter()
            .map(|&i| &library.builds[i])
            .collect()
    }

    fn sort_by(&mut self, column: SortColumn, cx: &mut Context<Self>) {
        self.sort.select(column);
        // Sorting changes only presentation. Keep the selected ID, preview and
        // editing state, and keep its row on the visible page after reordering.
        self.page = self
            .filtered_builds(cx)
            .iter()
            .position(|build| self.selected.as_ref() == Some(&build.id))
            .unwrap_or(0)
            / 25;
        cx.notify();
    }

    fn sort_header(&self, column: SortColumn, title: &'static str, cx: &Context<Self>) -> Button {
        let p = cx.global::<TooltipTheme>();
        let sorted = self.sort.column == column;
        let direction = self.sort.direction;
        let color = if sorted { p.accent_hot } else { p.faint };
        let muted = p.muted;
        let accessible_title = if column == SortColumn::Favorite {
            tr("library.favorite")
        } else {
            title
        };
        let accessible_label = trf(
            match (sorted, direction) {
                (true, SortDirection::Ascending) => "library.sort_ascending",
                (true, SortDirection::Descending) => "library.sort_descending",
                (false, _) => "library.sort_by",
            },
            &[("column", accessible_title.to_owned())],
        );
        let id = match column {
            SortColumn::Favorite => "sort-favorite",
            SortColumn::Name => "sort-name",
            SortColumn::Class => "sort-class",
            SortColumn::Level => "sort-level",
            SortColumn::Modified => "sort-modified",
        };
        Button::new(id)
            .custom(ButtonCustomVariant::new(cx).foreground(color))
            .h_auto()
            .min_w_0()
            .p_0()
            .border_0()
            .rounded_none()
            .font_weight(FontWeight::NORMAL)
            .accessibility_label(accessible_label)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .font_family(hsplanner_ui::theme::MONO_FONT_FAMILY)
                    .text_size(rems(10. / 13.))
                    .line_height(relative(1.5))
                    .text_color(color)
                    .hover(move |style| style.text_color(muted))
                    .child(hsplanner_ui::tooltip_text::TooltipText::new(
                        "sort-label",
                        title.to_uppercase(),
                        0.14,
                    ))
                    .when(sorted, |content| {
                        content.child(
                            canvas(
                                |_, _, _| (),
                                move |bounds, _, window, _| {
                                    let unit = bounds.size.width / 24.;
                                    let (edge, middle) = match direction {
                                        SortDirection::Ascending => (15., 9.),
                                        SortDirection::Descending => (9., 15.),
                                    };
                                    let mut path = PathBuilder::stroke(unit * 3.);
                                    path.move_to(bounds.origin + point(unit * 6., unit * edge));
                                    path.line_to(bounds.origin + point(unit * 12., unit * middle));
                                    path.line_to(bounds.origin + point(unit * 18., unit * edge));
                                    if let Ok(path) = path.build() {
                                        window.paint_path(path, window.text_style().color);
                                    }
                                },
                            )
                            .size_2p5()
                            .flex_none(),
                        )
                    }),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.sort_by(column, cx)))
    }

    fn reconcile_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let builds = self.filtered_builds(cx);
        if self
            .selected
            .as_ref()
            .is_some_and(|id| builds.iter().any(|build| &build.id == id))
        {
            return false;
        }
        self.selected = builds.first().map(|build| build.id.clone());
        self.editing = false;
        self.refresh_preview(cx);
        true
    }

    fn refresh_preview(&mut self, cx: &mut Context<Self>) {
        let snapshot = self
            .selected
            .as_ref()
            .and_then(|id| self.session.read(cx).state().library.build(id))
            .map(|build| build.snapshot.clone());
        let Some(snapshot) = snapshot else {
            self.preview_task = None;
            self.performance = None;
            self.preview_snapshot = None;
            return;
        };
        self.preview_revision += 1;
        let revision = self.preview_revision;
        self.preview_snapshot = Some(snapshot);
        self.performance = None;
        self.preview_task = None;
        if let Some(snapshot) = self.preview_snapshot.clone() {
            let calculation = cx.background_spawn(async move {
                hsplanner_engine::calc::planner::evaluate(&snapshot.planner_input())
            });
            self.preview_task = Some(cx.spawn(async move |this, cx| {
                let performance = calculation.await;
                let _ = this.update(cx, |this, cx| {
                    if this.preview_revision == revision {
                        this.performance = Some(performance);
                        this.preview_task = None;
                        cx.notify();
                    }
                });
            }));
        }
    }

    pub fn location(&self, cx: &App) -> String {
        if self.recent {
            tr("library.recent").into()
        } else if self.favorites {
            tr("library.favorites").into()
        } else if self.unfiled {
            tr("library.unfiled").into()
        } else if let Some(id) = &self.folder {
            self.session
                .read(cx)
                .state()
                .library
                .folders
                .iter()
                .find(|f| &f.id == id)
                .map(|f| f.name.clone())
                .unwrap_or_else(|| tr("library.folder").into())
        } else {
            tr("library.all_builds").into()
        }
    }
    fn folder_destinations(&self, build_id: &str, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let destinations = std::iter::once((None, tr("library.unfiled").to_string())).chain(
            self.session
                .read(cx)
                .state()
                .library
                .folders
                .iter()
                .map(|folder| (Some(folder.id.clone()), folder.name.clone())),
        );
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(label(tr("library.move_to_folder"), p.faint))
            .children(destinations.map(|(folder, name)| {
                let id = build_id.to_string();
                Button::new(SharedString::from(format!(
                    "move-to-{}",
                    folder.as_deref().unwrap_or("unfiled")
                )))
                .planner_style(cx)
                .label(name)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.apply(cx, |session| {
                        session.edit_library(|library| {
                            let build = library.build_mut(&id)?;
                            build.folder_id = folder.clone();
                            build.updated_at = hsplanner_build::library::now();
                            Ok(())
                        })
                    });
                }))
            }))
    }

    fn share(&mut self, id: &str, cx: &mut Context<Self>) {
        let result = self
            .session
            .read(cx)
            .state()
            .library
            .build(id)
            .ok_or_else(|| "library.error.missing_build".to_string())
            .and_then(|build| {
                hsplanner_build::codec::encode_loadouts(
                    &build.snapshot,
                    &build.notes(),
                    &build.loadouts,
                )
            });
        self.error = match result {
            Ok(code) => {
                cx.write_to_clipboard(ClipboardItem::new_string(code));
                None
            }
            Err(error) => Some(error),
        };
        cx.notify();
    }

    fn open(&mut self, id: String, cx: &mut Context<Self>) {
        if self.apply(cx, |session| session.open(&id)) {
            cx.emit(Opened);
        }
    }
}
impl Render for LibraryView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<TooltipTheme>();
        let (panel, border, muted, accent, negative) = (
            palette.panel,
            palette.border,
            palette.muted,
            palette.accent,
            palette.negative,
        );
        let library = &self.session.read(cx).state().library;
        let builds = self.filtered_builds(cx);
        let count = builds.len();
        let total = library.builds.len();
        let favorite_count = library.builds.iter().filter(|b| b.favorite).count();
        let unfiled_count = library
            .builds
            .iter()
            .filter(|b| b.folder_id.is_none())
            .count();
        let mut all_tags = library
            .builds
            .iter()
            .flat_map(|b| b.tags.iter().cloned())
            .collect::<Vec<_>>();
        all_tags.sort();
        all_tags.dedup();
        let active_id = self.session.read(cx).draft().build_id.clone();
        let folders = &library.folders;
        let mut folder_counts = std::collections::HashMap::new();
        for build in &library.builds {
            if let Some(folder) = &build.folder_id {
                *folder_counts.entry(folder.as_str()).or_insert(0usize) += 1;
            }
        }
        let selected = self.selected.as_deref().and_then(|id| library.build(id));
        let rows = builds
            .into_iter()
            .skip(self.page * 25)
            .take(25)
            .map(|build| {
                let id = build.id.clone();
                let chosen = self.selected.as_ref() == Some(&id);
                let (level, points) = query::build_summary(build);
                let favorite_id = id.clone();
                let select_id = id.clone();
                div()
                    .id(SharedString::from(format!("build-{id}")))
                    .relative()
                    .min_h(rems(60. / 13.))
                    .py_2p5()
                    .border_b_1()
                    .border_color(border)
                    .border_l_2()
                    .bg(if chosen {
                        palette.panel
                    } else {
                        palette.background
                    })
                    .hover(|row| row.bg(palette.panel_secondary))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let build = this
                            .session
                            .read(cx)
                            .state()
                            .library
                            .build(&select_id)
                            .cloned();
                        if let Some(build) = build {
                            this.select(&build, window, cx);
                        }
                    }))
                    .px_4()
                    .flex()
                    .items_center()
                    .gap_0()
                    .when(chosen, |row| {
                        row.child(
                            div()
                                .absolute()
                                .left_0()
                                .top_0()
                                .bottom_0()
                                .w(px(2.))
                                .bg(accent),
                        )
                    })
                    .child(
                        Button::new(SharedString::from(format!("star-{id}")))
                            .planner_style(cx)
                            .small()
                            .child(presentation::action_icon(if build.favorite {
                                "star-filled"
                            } else {
                                "star"
                            }))
                            .border_0()
                            .bg(palette.background.opacity(0.))
                            .w(rems(28. / 13.))
                            .flex_none()
                            .p_0()
                            .accessibility_label(tr("library.toggle_favorite"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.apply(cx, |s| {
                                    s.edit_library(|l| {
                                        let b = l.build_mut(&favorite_id)?;
                                        b.favorite = !b.favorite;
                                        Ok(())
                                    })
                                });
                            })),
                    )
                    .child(portrait(build.class_id.as_deref(), false, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .ml_3()
                            .pr_2()
                            .child(
                                div()
                                    .w_full()
                                    .text_ellipsis()
                                    .font_family(hsplanner_ui::theme::FONT_FAMILY)
                                    .text_size(rems(13.5 / 13.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(if chosen { accent } else { palette.text })
                                    .child(build.name.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1p5()
                                    .mt(rems(5. / 13.))
                                    .overflow_hidden()
                                    .when(active_id.as_ref() == Some(&id), |v| {
                                        v.child(label(tr("library.active"), accent))
                                    })
                                    .children(build.tags.iter().map(|tag| {
                                        div()
                                            .px_1()
                                            .border_1()
                                            .border_color(border)
                                            .child(label(tag.clone(), muted))
                                    })),
                            ),
                    )
                    .child(
                        div().w(rems(130. / 13.)).flex_none().child(label(
                            build
                                .class_id
                                .clone()
                                .unwrap_or_else(|| tr("library.unknown").into()),
                            muted,
                        )),
                    )
                    .child(
                        div()
                            .w(rems(64. / 13.))
                            .flex_none()
                            .child(label(format!("{level}/{points}"), muted)),
                    )
                    .child(
                        div()
                            .w(rems(76. / 13.))
                            .flex_none()
                            .child(label(build.season.clone(), muted)),
                    )
                    .child(
                        div().w(rems(140. / 13.)).flex_none().child(label(
                            build
                                .updated_at
                                .replace('T', " ")
                                .chars()
                                .take(16)
                                .collect::<String>(),
                            muted,
                        )),
                    )
            });
        let mut details = div()
            .w(rems(24.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .border_l_1()
            .border_color(border)
            .child(div().text_lg().child(tr("library.build_details")))
            .child(Input::new(&self.name).planner_style(cx))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("new-build")
                            .planner_style(cx)
                            .bg(hsplanner_ui::theme::chrome_gold_surface())
                            .text_color(cx.global::<TooltipTheme>().accent_hot)
                            .border_color(cx.global::<TooltipTheme>().accent_deep)
                            .label(tr("library.new_build"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                let name = this.name.read(cx).value().to_string();
                                if this.apply(cx, |s| s.new_build(&name).map(|_| ())) {
                                    cx.emit(Opened);
                                }
                            })),
                    )
                    .child(
                        Button::new("new-folder")
                            .planner_style(cx)
                            .label(tr("library.new_folder"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                let name = this.name.read(cx).value().to_string();
                                let parent = this.folder.clone();
                                this.apply(cx, |s| {
                                    s.edit_library(|l| l.create_folder(&name, parent).map(|_| ()))
                                });
                            })),
                    ),
            );
        if let Some(build) = selected {
            let rename = build.id.clone();
            let tags = build.id.clone();
            let duplicate = build.id.clone();
            let favorite = build.id.clone();
            let remove = build.id.clone();
            details = details
                .child(
                    Button::new("rename-build")
                        .planner_style(cx)
                        .label(tr("library.rename_selected"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let name = this.name.read(cx).value().to_string();
                            this.apply(cx, |s| s.edit_library(|l| l.rename(&rename, &name)));
                        })),
                )
                .child(Input::new(&self.tags).planner_style(cx))
                .child(
                    Button::new("save-tags")
                        .planner_style(cx)
                        .label(tr("library.save_tags"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let value = this.tags.read(cx).value().to_string();
                            this.apply(cx, |s| s.edit_library(|l| l.set_tags(&tags, &value)));
                        })),
                )
                .child(self.folder_destinations(&build.id, cx))
                .child(
                    Button::new("duplicate-build")
                        .planner_style(cx)
                        .label(tr("library.duplicate"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply(cx, |s| {
                                s.edit_library(|l| l.duplicate(&duplicate).map(|_| ()))
                            });
                        })),
                )
                .child(
                    Button::new("favorite-build")
                        .planner_style(cx)
                        .label(if build.favorite {
                            tr("library.remove_favorite")
                        } else {
                            tr("library.add_favorite")
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply(cx, |s| {
                                s.edit_library(|l| {
                                    let b = l.build_mut(&favorite)?;
                                    b.favorite = !b.favorite;
                                    Ok(())
                                })
                            });
                        })),
                )
                .child(
                    Button::new("delete-build")
                        .planner_style(cx)
                        .label(tr("library.delete_selected"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply(cx, |s| {
                                s.remove_build(&remove);
                                Ok(())
                            });
                            this.selected = None;
                        })),
                );
        }
        let mut folder_list = div()
            .w_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .border_r_1()
            .border_color(border)
            .child(label(tr("library.sidebar_title"), accent))
            .child(div().pt_3().child(label(tr("library.smart"), muted)))
            .child(
                navigation(
                    "recent",
                    tr("library.nav_recent"),
                    total.min(12),
                    self.recent,
                    cx,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.recent = true;
                    this.unfiled = false;
                    this.favorites = false;
                    this.folder = None;
                    this.page = 0;
                    this.reconcile_selection(cx);
                    cx.notify();
                })),
            )
            .child(
                navigation(
                    "all-builds",
                    tr("library.nav_all"),
                    total,
                    !self.recent && !self.unfiled && !self.favorites && self.folder.is_none(),
                    cx,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.folder = None;
                    this.recent = false;
                    this.unfiled = false;
                    this.favorites = false;
                    this.page = 0;
                    this.reconcile_selection(cx);
                    cx.notify();
                })),
            )
            .child(
                navigation(
                    "favorites",
                    tr("library.nav_favorites"),
                    favorite_count,
                    self.favorites,
                    cx,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.favorites = true;
                    this.recent = false;
                    this.unfiled = false;
                    this.folder = None;
                    this.page = 0;
                    this.reconcile_selection(cx);
                    cx.notify();
                })),
            );
        folder_list = folder_list
            .child(
                navigation(
                    "unfiled",
                    tr("library.nav_unfiled"),
                    unfiled_count,
                    self.unfiled,
                    cx,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.unfiled = true;
                    this.recent = false;
                    this.favorites = false;
                    this.folder = None;
                    this.page = 0;
                    this.reconcile_selection(cx);
                    cx.notify();
                })),
            )
            .child(div().pt_3().child(label(tr("library.folders"), muted)));
        for folder in folders {
            let id = folder.id.clone();
            let folder_count = folder_counts.get(id.as_str()).copied().unwrap_or(0);
            folder_list = folder_list.child(
                navigation(
                    SharedString::from(format!("folder-{id}")),
                    &format!("▱ {}", folder.name),
                    folder_count,
                    self.folder.as_ref() == Some(&id),
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.folder = Some(id.clone());
                    this.recent = false;
                    this.unfiled = false;
                    this.favorites = false;
                    this.page = 0;
                    this.reconcile_selection(cx);
                    cx.notify();
                })),
            );
        }
        if let Some(id) = self.folder.clone() {
            let remove = id.clone();
            folder_list = folder_list
                .child(
                    Button::new("rename-folder")
                        .planner_style(cx)
                        .label(tr("library.rename_folder"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.edit_dialog(EditKind::RenameFolder, window, cx)
                        })),
                )
                .child(
                    Button::new("delete-folder")
                        .planner_style(cx)
                        .label(tr("library.remove_folder"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply(cx, |s| {
                                s.edit_library(|l| {
                                    l.remove_folder(&remove, false);
                                    Ok(())
                                })
                            });
                            this.folder = None;
                        })),
                );
        }
        let listing = div()
            .id("library-list")
            .page_scroll(&self.list_scroll)
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .children(rows)
            .when(count == 0, |container| {
                container.child(div().p_6().text_color(muted).child(tr("library.empty")))
            });
        let pagination = div()
            .p_3()
            .flex()
            .items_center()
            .gap_3()
            .child(
                Button::new("previous-page")
                    .planner_style(cx)
                    .label(tr("library.previous"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.page = this.page.saturating_sub(1);
                        cx.notify();
                    })),
            )
            .child(trf(
                "library.pagination",
                &[
                    ("count", count.to_string()),
                    ("page", (self.page + 1).to_string()),
                ],
            ))
            .child(
                Button::new("next-page")
                    .planner_style(cx)
                    .label(tr("library.next"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if (this.page + 1) * 25 < count {
                            this.page += 1;
                        }
                        cx.notify();
                    })),
            );
        let mut filter_bar = div()
            .py_2p5()
            .px_4()
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(border)
            .child(label(tr("library.filter"), muted))
            .child(
                Button::new("clear-tags")
                    .planner_style(cx)
                    .small()
                    .rounded_full()
                    .label(tr("library.all_filter"))
                    .border_color(if self.active_tag.is_none() && !self.high_level {
                        palette.accent_deep
                    } else {
                        border
                    })
                    .text_color(palette.accent_hot)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.active_tag = None;
                        this.high_level = false;
                        this.page = 0;
                        this.reconcile_selection(cx);
                        cx.notify();
                    })),
            );
        for tag in all_tags {
            let chosen = self.active_tag.as_ref() == Some(&tag);
            filter_bar = filter_bar.child(
                Button::new(SharedString::from(format!("tag-{tag}")))
                    .planner_style(cx)
                    .small()
                    .rounded_full()
                    .label(tag.clone())
                    .selected(chosen)
                    .border_color(if chosen { palette.accent_deep } else { border })
                    .text_color(if chosen { palette.accent_hot } else { muted })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.active_tag = if chosen { None } else { Some(tag.clone()) };
                        this.page = 0;
                        this.reconcile_selection(cx);
                        cx.notify();
                    })),
            );
        }
        filter_bar = filter_bar.child(
            Button::new("level-filter")
                .planner_style(cx)
                .small()
                .rounded_full()
                .label(tr("library.high_level_filter"))
                .selected(self.high_level)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.high_level = !this.high_level;
                    this.page = 0;
                    this.reconcile_selection(cx);
                    cx.notify();
                })),
        );
        filter_bar = filter_bar.child(div().ml_auto().child(label(
            trf(
                "library.filtered_count",
                &[("count", count.to_string()), ("total", total.to_string())],
            ),
            accent,
        )));
        let header = div()
            .py_2()
            .px_4()
            .flex()
            .gap_0()
            .items_center()
            .border_b_1()
            .border_color(border)
            .child(div().w(rems(28. / 13.)).flex_none().child(self.sort_header(
                SortColumn::Favorite,
                "★",
                cx,
            )))
            .child(
                div()
                    .flex_1()
                    .child(self.sort_header(SortColumn::Name, tr("library.name"), cx)),
            )
            .child(
                div()
                    .w(rems(130. / 13.))
                    .flex_none()
                    .child(self.sort_header(SortColumn::Class, tr("library.class"), cx)),
            )
            .child(div().w(rems(64. / 13.)).flex_none().child(self.sort_header(
                SortColumn::Level,
                tr("library.level"),
                cx,
            )))
            .child(
                div()
                    .w(rems(76. / 13.))
                    .flex_none()
                    .child(label(tr("library.season"), palette.faint)),
            )
            .child(
                div()
                    .w(rems(140. / 13.))
                    .flex_none()
                    .child(self.sort_header(SortColumn::Modified, tr("library.modified"), cx)),
            );
        let import = div()
            .mx_4()
            .my_5()
            .p_6()
            .rounded_md()
            .border_1()
            .border_dashed()
            .border_color(border)
            .bg(panel)
            .child(
                div()
                    .text_color(palette.text)
                    .child(tr("library.import_heading")),
            )
            .child(
                div()
                    .text_color(muted)
                    .text_sm()
                    .child(tr("library.import_help")),
            )
            .when(self.importing, |view| {
                view.child(
                    div()
                        .mt_3()
                        .flex()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .child(Input::new(&self.code).planner_style(cx)),
                        )
                        .child(
                            Button::new("import-build")
                                .planner_style(cx)
                                .label(tr("library.import"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let code = this.code.read(cx).value().to_string();
                                    if this.apply(cx, |s| s.import_code(&code).map(|_| ())) {
                                        cx.emit(Opened);
                                    }
                                })),
                        ),
                )
            });
        let listing = listing.child(import);
        let center = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(palette.background)
            .child(
                div()
                    .id("library-filters-scroll")
                    .w_full()
                    .h_auto()
                    .flex_none()
                    .overflow_x_scroll()
                    .child(filter_bar.min_w(rems(640. / 13.))),
            )
            .child(
                // Preserve the name and fixed reference columns at the minimum
                // window size; header and rows scroll horizontally together.
                div().relative().flex_1().min_w_0().min_h_0().child(
                    div()
                        .id("library-columns-scroll")
                        .size_full()
                        .overflow_x_scroll()
                        .track_scroll(&self.columns_scroll)
                        .child(
                            div()
                                .w_full()
                                .min_w(rems(640. / 13.))
                                .h_full()
                                .flex()
                                .flex_col()
                                .child(header)
                                .child(listing),
                        ),
                ),
            )
            .when(count > 25, |v| v.child(pagination));
        let preview = self.preview(selected, cx);
        let open_id = selected.as_ref().map(|b| b.id.clone());
        let share_id = selected.map(|build| build.id.clone());
        let details = div()
            .w(rems(360. / 13.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(border)
            .child(
                div()
                    .id("preview-scroll")
                    .page_scroll(&self.preview_scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(preview)
                    .when(self.editing, |view| view.child(details)),
            )
            .child(
                div()
                    .p_3()
                    .flex()
                    .gap_2()
                    .border_t_1()
                    .border_color(border)
                    .child(
                        Button::new("share-selected")
                            .planner_style(cx)
                            .flex_1()
                            .label(tr("library.share"))
                            .disabled(share_id.is_none())
                            .opacity(if share_id.is_none() { 0.4 } else { 1. })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(id) = &share_id {
                                    this.share(id, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("open-selected")
                            .planner_style(cx)
                            .flex_1()
                            .label(tr("library.open_build"))
                            .disabled(open_id.is_none())
                            .opacity(if open_id.is_none() { 0.4 } else { 1. })
                            .bg(hsplanner_ui::theme::chrome_gold_surface())
                            .text_color(accent)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(id) = &open_id {
                                    this.open(id.clone(), cx);
                                }
                            })),
                    ),
            );
        let sidebar = div()
            .w(rems(240. / 13.))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(border)
            .child(
                div()
                    .id("folder-scroll")
                    .page_scroll(&self.folder_scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(folder_list),
            )
            .child(
                div()
                    .p_3()
                    .border_t_1()
                    .border_color(border)
                    .child(label(tr("library.local_library"), muted))
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(trf("library.total_count", &[("count", total.to_string())])),
                    ),
            );
        let body = div()
            .w_full()
            .flex_1()
            .min_h_0()
            .flex()
            .items_stretch()
            .child(sidebar)
            .child(center)
            .child(details);
        let selected_id = selected.as_ref().map(|b| b.id.clone());
        let duplicate_id = selected_id.clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(panel)
            .child(
                div()
                    .h(rems(38. / 13.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        Button::new("toolbar-new")
                            .planner_style(cx)
                            .label(tr("library.toolbar_new"))
                            .text_color(accent)
                            .bg(hsplanner_ui::theme::chrome_gold_surface())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.edit_dialog(EditKind::NewBuild, window, cx)
                            })),
                    )
                    .child(
                        toolbar_button(
                            "toolbar-import",
                            tr("library.toolbar_import"),
                            "import",
                            cx,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.importing = !this.importing;
                            cx.notify();
                        })),
                    )
                    .child(
                        toolbar_button("toolbar-copy", tr("library.copy"), "copy", cx)
                            .disabled(duplicate_id.is_none())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(id) = &duplicate_id {
                                    this.apply(cx, |s| {
                                        s.edit_library(|l| l.duplicate(id).map(|_| ()))
                                    });
                                }
                            })),
                    )
                    .child(
                        toolbar_button("toolbar-rename", tr("library.rename"), "rename", cx)
                            .disabled(selected_id.is_none())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.edit_dialog(EditKind::Rename, window, cx)
                            })),
                    )
                    .child(
                        toolbar_button("toolbar-delete", tr("library.delete"), "delete", cx)
                            .disabled(selected_id.is_none())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(id) = &selected_id {
                                    this.apply(cx, |s| {
                                        s.remove_build(id);
                                        Ok(())
                                    });
                                    this.selected = None;
                                }
                            })),
                    )
                    .child(
                        toolbar_button(
                            "toolbar-folder",
                            tr("library.toolbar_folder"),
                            "newfolder",
                            cx,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.edit_dialog(EditKind::NewFolder, window, cx)
                        })),
                    )
                    .child(
                        Button::new("manage-build")
                            .planner_style(cx)
                            .label("…")
                            .cursor_tooltip(tr("library.manage"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editing = !this.editing;
                                if let Some(build) = this
                                    .selected
                                    .as_ref()
                                    .and_then(|id| this.session.read(cx).state().library.build(id))
                                    .cloned()
                                {
                                    this.select(&build, window, cx);
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        div().ml_auto().w(rems(24.)).child(
                            Input::new(&self.search)
                                .planner_style(cx)
                                .prefix(presentation::action_icon("search")),
                        ),
                    ),
            )
            .children(self.error.as_ref().map(|error| {
                div()
                    .px_4()
                    .py_2()
                    .text_color(negative)
                    .child(render_error(error))
            }))
            .child(body)
    }
}

#[cfg(test)]
mod sorting_tests {
    use super::{LibrarySort, SavedBuild, SortColumn, SortDirection};
    use hsplanner_build::{BuildSnapshot, library::Library, notes::Notes};

    fn build(name: &str, level: u32) -> SavedBuild {
        let mut library = Library::default();
        library
            .create(
                name,
                &BuildSnapshot {
                    class_id: Some("stormweaver".into()),
                    level,
                    ..Default::default()
                },
                &Notes::default(),
                &[],
                None,
            )
            .unwrap();
        library.builds.remove(0)
    }

    fn names(builds: &[SavedBuild]) -> Vec<&str> {
        builds.iter().map(|build| build.name.as_str()).collect()
    }

    #[test]
    fn headers_toggle_and_choose_reference_default_directions() {
        let mut sort = LibrarySort::default();
        assert_eq!(sort.column, SortColumn::Modified);
        assert_eq!(sort.direction, SortDirection::Descending);
        sort.select(SortColumn::Modified);
        assert_eq!(sort.direction, SortDirection::Ascending);
        for column in [SortColumn::Name, SortColumn::Class] {
            sort.select(column);
            assert_eq!(sort.direction, SortDirection::Ascending);
            sort.select(column);
            assert_eq!(sort.direction, SortDirection::Descending);
        }
        for column in [
            SortColumn::Favorite,
            SortColumn::Level,
            SortColumn::Modified,
        ] {
            sort.select(column);
            assert_eq!(sort.direction, SortDirection::Descending);
        }
    }

    #[test]
    fn level_sort_uses_the_composed_build_snapshot() {
        let mut edited = build("Edited", 2);
        edited.snapshot.level = 90;
        let middle = build("Middle", 40);
        let early = build("Early", 10);
        let mut builds = vec![early, middle, edited];
        LibrarySort {
            column: SortColumn::Level,
            direction: SortDirection::Descending,
        }
        .apply(&mut builds, false);
        assert_eq!(names(&builds), ["Edited", "Middle", "Early"]);
    }

    #[test]
    fn favorite_sort_is_stable_in_both_directions() {
        let first = build("First", 1);
        let mut favorite = build("Favorite", 1);
        favorite.favorite = true;
        let second = build("Second", 1);
        let source = vec![first, favorite, second];
        for (direction, expected) in [
            (SortDirection::Ascending, ["First", "Second", "Favorite"]),
            (SortDirection::Descending, ["Favorite", "First", "Second"]),
        ] {
            let mut builds = source.clone();
            LibrarySort {
                column: SortColumn::Favorite,
                direction,
            }
            .apply(&mut builds, false);
            assert_eq!(names(&builds), expected);
        }
    }

    #[test]
    fn recent_membership_precedes_sorting_and_keeps_selected_document_identity() {
        let mut builds = (1..=13)
            .map(|index| {
                let mut build = build(&format!("Build {index:02}"), index);
                build.updated_at = format!("2026-09-{index:02}T10:00:00Z");
                build
            })
            .collect::<Vec<_>>();
        let selected = builds[7].id.clone();
        let selected_loadouts = builds[7].loadouts.clone();
        LibrarySort {
            column: SortColumn::Name,
            direction: SortDirection::Ascending,
        }
        .apply(&mut builds, true);
        assert_eq!(builds.len(), 12);
        assert_eq!(builds.first().unwrap().name, "Build 02");
        assert_eq!(builds.last().unwrap().name, "Build 13");
        let selected = builds.iter().find(|build| build.id == selected).unwrap();
        for kind in [
            hsplanner_build::loadout::LoadoutKind::Incarnation,
            hsplanner_build::loadout::LoadoutKind::Ether,
            hsplanner_build::loadout::LoadoutKind::Gear,
            hsplanner_build::loadout::LoadoutKind::Skills,
        ] {
            assert_eq!(
                selected.loadouts.active_id(kind),
                selected_loadouts.active_id(kind)
            );
        }
    }

    #[test]
    fn text_columns_use_case_insensitive_names_and_displayed_class_names() {
        let mut unknown = build("alpha", 1);
        unknown.class_id = Some("aaa-invalid-class".into());
        let known = build("Zulu", 1);
        let mut builds = vec![known, unknown];
        LibrarySort {
            column: SortColumn::Name,
            direction: SortDirection::Ascending,
        }
        .apply(&mut builds, false);
        assert_eq!(names(&builds), ["alpha", "Zulu"]);
        LibrarySort {
            column: SortColumn::Class,
            direction: SortDirection::Ascending,
        }
        .apply(&mut builds, false);
        assert_eq!(names(&builds), ["Zulu", "alpha"]);
    }
}
