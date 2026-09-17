use crate::shell::{OpenSettings, Section, SelectSection};
use gpui_kit::base::Disableable;
use gpui_kit::component::{
    Icon, IconName, Sizable, WindowExt,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    link::Link,
    menu::DropdownMenu,
};
use gpui_kit::{prelude::*, *};
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::tooltip::CursorTooltipExt;
use hsplanner_ui::{
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};
use std::{rc::Rc, sync::Arc};

const KOFI_URL: &str = "https://ko-fi.com/zium1337";
const SAVE_SHORTCUT: &str = if cfg!(target_os = "macos") {
    "⌘S"
} else {
    "Ctrl+S"
};
const BUILD_CHANNEL: &str = if cfg!(debug_assertions) {
    "chrome.dev"
} else {
    "chrome.stable"
};

// The reference layout is authored in CSS pixels at the theme's 13px base font,
// so these rem values keep the same proportions and follow the native zoom.
fn web_px(value: f32) -> Rems {
    rems(value / 13.)
}

pub fn logo() -> Arc<Image> {
    Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        include_bytes!("../assets/logo.svg").to_vec(),
    ))
}

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type SelectHandler = Rc<dyn Fn(Section, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct TopBar {
    logo: Arc<Image>,
    section: Section,
    on_select: SelectHandler,
    on_library: ClickHandler,
    on_share: ClickHandler,
    build_name: String,
    library_location: String,
    auto_save: bool,
    profile_controls: bool,
    ui_zoom: f32,
}

impl TopBar {
    pub fn new(
        logo: Arc<Image>,
        section: Section,
        on_select: SelectHandler,
        on_library: ClickHandler,
        on_share: ClickHandler,
    ) -> Self {
        Self {
            logo,
            section,
            on_select,
            on_library,
            on_share,
            build_name: String::new(),
            library_location: tr("chrome.recent").into(),
            auto_save: true,
            profile_controls: false,
            ui_zoom: 1.,
        }
    }

    pub fn document(mut self, name: String, auto_save: bool, profile_controls: bool) -> Self {
        self.build_name = name;
        self.auto_save = auto_save;
        self.profile_controls = profile_controls;
        self
    }

    pub fn library_location(mut self, location: String) -> Self {
        self.library_location = location;
        self
    }

    pub fn ui_zoom(mut self, zoom: f32) -> Self {
        self.ui_zoom = zoom;
        self
    }

    fn tab(
        &self,
        section: Section,
        palette: &TooltipTheme,
        rem: Pixels,
        cx: &App,
    ) -> impl IntoElement {
        let active = section == self.section;
        let color = if active {
            palette.accent_hot
        } else {
            palette.muted
        };
        let on_select = self.on_select.clone();
        let button = Button::new(SharedString::from(format!("nav-{}", section.key())))
            .custom(
                ButtonCustomVariant::new(cx)
                    .foreground(color)
                    .hover(palette.panel_secondary),
            )
            .relative()
            .h_full()
            .flex_shrink_0()
            .accessibility_label(section.label())
            .rounded_none()
            .px_3p5()
            .gap_2()
            .child(div().flex_none().size(web_px(4.875)))
            .child(
                // The reference's global `button { font-family: inherit }`
                // overrides its Tailwind font-mono class on navigation buttons.
                mono_label(
                    format!("nav-label-{}", section.key()),
                    section.label(),
                    11.,
                    0.16,
                    None,
                )
                .font_family(theme::FONT_FAMILY)
                .font_weight(FontWeight::NORMAL)
                .line_height(relative(1.5)),
            )
            .on_click(move |_, window, cx| on_select(section, window, cx));
        // Button's label slot clips its content. Paint ornaments in the tab's
        // outer layer so the halo can extend into the padding on every side.
        div()
            .relative()
            .h_full()
            .flex_none()
            .child(button)
            .child(
                div()
                    .absolute()
                    .left_3p5()
                    .top_0()
                    .h_full()
                    .w(web_px(4.875))
                    .flex()
                    .items_center()
                    .child(diamond(
                        if active {
                            palette.accent_hot
                        } else {
                            palette.faint
                        },
                        4.875,
                        active.then_some(palette.accent_hot),
                    )),
            )
            .when(active, |this| {
                let hot = palette.accent_hot;
                this.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left_2()
                        .right_2()
                        .h(web_px(2.))
                        .shadow(vec![BoxShadow {
                            color: hot.opacity(0.45),
                            offset: point(px(0.), px(0.)),
                            blur_radius: rem * (12. / 13.),
                            spread_radius: px(0.),
                            inset: false,
                        }])
                        .flex()
                        .child(div().flex_1().bg(linear_gradient(
                            90.,
                            linear_color_stop(hot.opacity(0.), 0.),
                            linear_color_stop(hot, 1.),
                        )))
                        .child(div().flex_1().bg(linear_gradient(
                            90.,
                            linear_color_stop(hot, 0.),
                            linear_color_stop(hot.opacity(0.), 1.),
                        ))),
                )
            })
    }
}

impl RenderOnce for TopBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = cx.global::<TooltipTheme>();
        let (panel, panel_secondary, border, accent_hot, shadow) = (
            palette.panel,
            palette.panel_secondary,
            palette.border,
            palette.accent_hot,
            palette.shadow,
        );
        if self.section == Section::Library {
            let on_select = self.on_select.clone();
            return div()
                .h_11()
                .flex_none()
                .flex()
                .items_center()
                .px_3()
                .gap_3()
                .border_b_1()
                .border_color(border)
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(panel_secondary, 0.),
                    linear_color_stop(panel, 1.),
                ))
                .child(img(self.logo.clone()).size(web_px(22.)))
                .child(mono_label(
                    "library-brand",
                    "HSPlanner",
                    11.,
                    0.18,
                    Some(accent_hot),
                ))
                .child(divider(border))
                .child(mono_label(
                    "library-breadcrumb",
                    trf(
                        "chrome.breadcrumb",
                        &[("location", self.library_location.clone())],
                    ),
                    11.,
                    0.18,
                    Some(accent_hot),
                ))
                .child(
                    chrome_button("return-to-planner", tr("chrome.planner"), cx)
                        .ml_auto()
                        .label(tr("chrome.back"))
                        .on_click(move |_, window, cx| on_select(Section::Tree, window, cx)),
                )
                .child(
                    chrome_button("settings", tr("settings.title"), cx)
                        .p_1p5()
                        .child(Icon::new(IconName::Settings).size_3p5())
                        .cursor_tooltip(tr("settings.title"))
                        .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(OpenSettings), cx)
                        }),
                )
                .into_any_element();
        }
        let tabs: Vec<_> = Section::NAV
            .iter()
            .map(|section| {
                self.tab(*section, palette, window.rem_size(), cx)
                    .into_any_element()
            })
            .collect();
        let compact = window.viewport_size().width < window.rem_size() * (1150. / 13.);
        // The original English breakpoint needs the actual expansion of the
        // translated labels; otherwise Russian tabs can hide the overflow menu.
        let mut nav_style = window.text_style();
        nav_style.font_family = theme::FONT_FAMILY.into();
        nav_style.font_weight = FontWeight::NORMAL;
        let font_size = window.rem_size() * (11. / 13.);
        let measure = |label: &str, tracking: f32| {
            let label = label.to_uppercase();
            window
                .text_system()
                .shape_line(
                    label.clone().into(),
                    font_size,
                    &[nav_style.to_run(label.len())],
                    None,
                )
                .width
                + font_size * tracking * label.chars().count() as f32
        };
        let locale = hsplanner_ui::i18n::language();
        let tracking = if matches!(
            locale,
            hsplanner_ui::i18n::Language::Korean | hsplanner_ui::i18n::Language::Chinese
        ) {
            0.
        } else {
            0.16
        };
        let expansion = Section::NAV
            .into_iter()
            .map(Section::key)
            .chain(["chrome.filters"])
            .map(|key| {
                measure(tr(key), tracking)
                    - measure(
                        hsplanner_ui::i18n::text_for(hsplanner_ui::i18n::Language::English, key),
                        0.16,
                    )
            })
            .fold(px(0.), |total, width| total + width)
            .max(px(0.));
        let navigation_overflows =
            window.viewport_size().width < window.rem_size() * (1330. / 13.) + expansion;
        let selected_section = self.section;
        div()
            .flex_none()
            .h_11()
            .flex()
            .items_center()
            .px_3()
            .border_b_1()
            .border_color(border)
            .bg(linear_gradient(
                180.,
                linear_color_stop(panel_secondary, 0.),
                linear_color_stop(panel, 1.),
            ))
            .shadow(vec![hairline_shadow(shadow.opacity(0.4), 1.)])
            .child(
                div()
                    .mr_3()
                    .pr_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_r_1()
                    .border_color(border)
                    .child(img(self.logo.clone()).size(web_px(22.)))
                    .when(!compact, |brand| {
                        brand.child(
                            div()
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(web_px(11.))
                                .line_height(relative(1.))
                                .text_color(accent_hot)
                                .child(
                                    TooltipText::new("brand", "HSPLANNER", 0.18)
                                        .glow(Some(accent_hot.opacity(0.5))),
                                ),
                        )
                    }),
            )
            .child(
                div()
                    .id("topbar-navigation")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_x_scroll()
                    .child(
                        div().h_full().flex().items_stretch().children(tabs).child(
                            Button::new("nav-filters")
                                .disabled(true)
                                .h_full()
                                .rounded_none()
                                .px_3p5()
                                .gap_2()
                                .custom(ButtonCustomVariant::new(cx).foreground(palette.muted))
                                .accessibility_label(tr("chrome.filters"))
                                .cursor_tooltip(tr("chrome.filters_unavailable"))
                                .child(diamond(palette.faint, 4.875, None))
                                .child(
                                    mono_label(
                                        "nav-filters-label",
                                        tr("chrome.filters"),
                                        11.,
                                        0.16,
                                        None,
                                    )
                                    .font_family(theme::FONT_FAMILY)
                                    .font_weight(FontWeight::NORMAL)
                                    .line_height(relative(1.5)),
                                ),
                        ),
                    ),
            )
            .when(navigation_overflows, |view| {
                view.child(
                    chrome_button("all-views", tr("chrome.all_views"), cx)
                        .flex_none()
                        .ml_1()
                        .p_1p5()
                        .child(Icon::new(IconName::ChevronDown).size_3p5())
                        .cursor_tooltip(tr("chrome.all_views"))
                        .dropdown_menu(move |menu, _, _| {
                            Section::NAV.into_iter().fold(menu, |menu, section| {
                                menu.menu_with_check(
                                    section.label(),
                                    selected_section == section,
                                    Box::new(SelectSection { section }),
                                )
                            })
                        }),
                )
            })
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .pl_2()
                    .child(divider(border))
                    .child(
                        chrome_button("open-library", tr("chrome.builds"), cx)
                            .child(chrome_icon("bookmark"))
                            .child(tr("chrome.builds"))
                            .when(!compact && !self.build_name.is_empty(), |button| {
                                button
                                    .child(div().text_color(palette.faint).child("·"))
                                    .child(div().max_w(rems(8.)).overflow_hidden().child(
                                        mono_label(
                                            "build-name",
                                            compact_build_name(&self.build_name),
                                            10.,
                                            0.14,
                                            Some(accent_hot),
                                        ),
                                    ))
                            })
                            .cursor_tooltip(trf(
                                "chrome.library_hint",
                                &[("name", self.build_name.clone())],
                            ))
                            .on_click(self.on_library),
                    )
                    .child(
                        hsplanner_ui::controls::planner_button(
                            "share-build",
                            hsplanner_ui::controls::ButtonTone::Primary,
                            cx,
                        )
                        .small()
                        .gap_1p5()
                        .accessibility_label(tr("chrome.share"))
                        .child(chrome_icon("share"))
                        .child(tr("chrome.share"))
                        .cursor_tooltip(tr("chrome.share_hint"))
                        .on_click(self.on_share),
                    )
                    .child(
                        chrome_button("help", tr("chrome.help"), cx)
                            .label("?")
                            .cursor_tooltip(tr("chrome.shortcuts"))
                            .on_click(|_, window, cx| {
                                window.open_dialog(cx, |dialog, _, _| {
                                    dialog.title(tr("chrome.shortcuts")).child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .child(tr("chrome.shortcut_save"))
                                            .child(tr("chrome.shortcut_tree"))
                                            .child(tr("chrome.shortcut_search"))
                                            .child(tr("chrome.shortcut_escape")),
                                    )
                                })
                            }),
                    )
                    .child(
                        chrome_button("settings", tr("settings.title"), cx)
                            .p_1p5()
                            .child(Icon::new(IconName::Settings).size_3p5())
                            .cursor_tooltip(tr("settings.title"))
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(OpenSettings), cx)
                            }),
                    ),
            )
            .into_any_element()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SaveState {
    Saving,
    Auto,
    Saved,
    Manual,
}

impl SaveState {
    pub fn derive(saving: bool, auto_save: bool, saved_flash: bool) -> Self {
        if saving {
            Self::Saving
        } else if auto_save {
            Self::Auto
        } else if saved_flash {
            Self::Saved
        } else {
            Self::Manual
        }
    }
}

#[derive(IntoElement)]
pub struct BottomBar {
    session: Entity<hsplanner_build::session::Session>,
    save: SaveState,
    status: Option<SharedString>,
    updater: Entity<crate::update::Updater>,
}

impl BottomBar {
    pub fn new(
        session: Entity<hsplanner_build::session::Session>,
        save: SaveState,
        status: Option<SharedString>,
        updater: Entity<crate::update::Updater>,
    ) -> Self {
        Self {
            session,
            save,
            status,
            updater,
        }
    }
}

impl RenderOnce for BottomBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = cx.global::<TooltipTheme>();
        let (
            panel,
            panel_secondary,
            border,
            faint,
            muted,
            accent_hot,
            accent_deep,
            positive,
            shadow,
        ) = (
            palette.panel,
            palette.panel_secondary,
            palette.border,
            palette.faint,
            palette.muted,
            palette.accent_hot,
            palette.accent_deep,
            palette.positive,
            palette.shadow,
        );
        let rem = window.rem_size();
        let (dot_color, save_label, save_tooltip) = match self.save {
            SaveState::Saving => (faint, tr("shell.saving"), None),
            SaveState::Auto => (positive, tr("chrome.autosaved"), None),
            SaveState::Saved => (positive, tr("shell.saved"), None),
            SaveState::Manual => (
                accent_hot,
                tr("chrome.manual"),
                Some(trf(
                    "chrome.manual_hint",
                    &[("shortcut", SAVE_SHORTCUT.into())],
                )),
            ),
        };
        let save_text = if self.save == SaveState::Manual {
            trf(
                "chrome.manual_shortcut",
                &[("shortcut", SAVE_SHORTCUT.into())],
            )
        } else {
            save_label.to_string()
        };
        let channel_color = if cfg!(debug_assertions) {
            accent_hot
        } else {
            positive
        };
        let channel_border = if cfg!(debug_assertions) {
            accent_deep
        } else {
            positive
        };
        div()
            .flex_none()
            .h_9()
            .flex()
            .items_center()
            .gap_2p5()
            .px_3()
            .border_t_1()
            .border_color(border)
            .bg(linear_gradient(
                180.,
                linear_color_stop(panel, 0.),
                linear_color_stop(panel_secondary, 1.),
            ))
            .shadow(vec![hairline_shadow(shadow.opacity(0.4), -1.)])
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(diamond(accent_deep, 4., Some(accent_deep)))
                    .child(mono_label(
                        "footer-brand",
                        "HSPlanner",
                        10.,
                        0.18,
                        Some(accent_deep),
                    )),
            )
            .child(div().text_color(faint).child("·"))
            .child(
                Button::new("changelog")
                    .custom(ButtonCustomVariant::new(cx).foreground(faint))
                    .h_auto()
                    .p_0()
                    .accessibility_label(tr("chrome.changelog"))
                    .cursor_tooltip(tr("chrome.changelog"))
                    .child(mono_label(
                        "footer-version",
                        format!("v{}", env!("CARGO_PKG_VERSION")),
                        10.,
                        0.14,
                        Some(faint),
                    ))
                    .on_click(|_, window, cx| crate::changelog::open(window, cx)),
            )
            .child(
                div()
                    .px_1p5()
                    .py_px()
                    .rounded(rem * (3. / 13.))
                    .border_1()
                    .border_color(channel_border.opacity(0.5))
                    .bg(panel_secondary.opacity(0.6))
                    .child(mono_label(
                        "footer-channel",
                        tr(BUILD_CHANNEL),
                        9.,
                        0.18,
                        Some(channel_color),
                    )),
            )
            .child(divider(border))
            .child(update_button(self.updater, cx))
            .children(
                self.status
                    .map(|status| div().text_xs().text_color(muted).child(status)),
            )
            .child(
                chrome_button("report-bug", tr("chrome.report"), cx)
                    .ml_auto()
                    .child(chrome_icon("bug"))
                    .child(tr("chrome.report"))
                    .cursor_tooltip(tr("chrome.report_hint"))
                    .on_click(move |_, window, cx| {
                        crate::bug_report::open(self.session.clone(), window, cx)
                    }),
            )
            .child(
                Link::new("kofi")
                    .href(KOFI_URL)
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .px_2p5()
                    .py_0p5()
                    .rounded(rem * (3. / 13.))
                    .border_1()
                    .border_color(accent_deep)
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(accent_deep.opacity(0.35), 0.),
                        linear_color_stop(accent_deep.opacity(0.2), 1.),
                    ))
                    .text_color(accent_hot)
                    .child(chrome_icon("coffee"))
                    .child(mono_label(
                        "footer-kofi",
                        tr("settings.support"),
                        10.,
                        0.14,
                        Some(accent_hot),
                    )),
            )
            .child(divider(border))
            .child(
                div()
                    .id("save-state")
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .when_some(save_tooltip, |this, tip| {
                        this.cursor_tooltip_view(move |window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(tip.clone())
                                .build(window, cx)
                        })
                    })
                    .child(dot(dot_color, rem))
                    .child(mono_label(
                        "footer-save",
                        save_text,
                        10.,
                        0.14,
                        Some(dot_color),
                    )),
            )
    }
}

fn update_button(updater: Entity<crate::update::Updater>, cx: &App) -> Button {
    use crate::update::State;
    use hsplanner_ui::controls::ButtonTone;
    let state = updater.read(cx).state.clone();
    let (label, tooltip, tone) = match &state {
        State::Idle => (
            tr("update.check").to_string(),
            tr("update.check_hint").to_string(),
            ButtonTone::Neutral,
        ),
        State::Checking => (
            tr("update.checking").to_string(),
            tr("update.checking_hint").to_string(),
            ButtonTone::Neutral,
        ),
        State::UpToDate => (
            tr("update.current").to_string(),
            tr("update.check_again").to_string(),
            ButtonTone::Neutral,
        ),
        State::Available(update) => (
            trf("update.version", &[("version", update.version.to_string())]),
            tr("update.available").to_string(),
            ButtonTone::Primary,
        ),
        State::Installing(_) => (
            tr("update.installing").to_string(),
            tr("update.downloading_hint").to_string(),
            ButtonTone::Primary,
        ),
        State::Failed(message) => (
            tr("update.failed").to_string(),
            message.message(),
            ButtonTone::Danger,
        ),
    };
    let opens_dialog = matches!(
        state,
        State::Available(_) | State::Installing(_) | State::Failed(_)
    );
    hsplanner_ui::controls::planner_button("check-updates", tone, cx)
        .small()
        .gap_1p5()
        .accessibility_label(tr("update.check"))
        .label(label)
        .cursor_tooltip(tooltip)
        .on_click(move |_, window, cx| {
            if opens_dialog {
                crate::update::open_dialog(updater.clone(), window, cx);
            } else {
                updater.update(cx, |updater, cx| updater.check(false, cx));
            }
        })
}

fn chrome_button(id: &'static str, label: &'static str, cx: &App) -> Button {
    hsplanner_ui::controls::planner_button(id, hsplanner_ui::controls::ButtonTone::Neutral, cx)
        .small()
        .gap_1p5()
        .accessibility_label(label)
}

fn chrome_icon(name: &str) -> impl IntoElement {
    static ICONS: std::sync::LazyLock<std::collections::HashMap<&str, Arc<Image>>> =
        std::sync::LazyLock::new(|| {
            [
                (
                    "bookmark",
                    include_bytes!("../assets/bookmark.svg").as_slice(),
                ),
                ("share", include_bytes!("../assets/share.svg").as_slice()),
                ("bug", include_bytes!("../assets/bug.svg").as_slice()),
                ("coffee", include_bytes!("../assets/coffee.svg").as_slice()),
            ]
            .into_iter()
            .map(|(name, data)| {
                (
                    name,
                    Arc::new(Image::from_bytes(ImageFormat::Svg, data.to_vec())),
                )
            })
            .collect()
        });
    img(ICONS[name].clone()).size(web_px(12.)).flex_shrink_0()
}

fn compact_build_name(name: &str) -> String {
    let mut chars = name.chars();
    let prefix: String = chars.by_ref().take(13).collect();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn mono_label(
    id: impl Into<ElementId>,
    text: impl Into<String>,
    size: f32,
    tracking: f32,
    color: Option<Hsla>,
) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(web_px(size))
        .line_height(relative(1.))
        .when_some(color, |this, color| this.text_color(color))
        .child(TooltipText::new(id, text.into().to_uppercase(), tracking))
}

fn divider(color: Hsla) -> impl IntoElement {
    div().w_px().h_4().bg(color)
}

fn dot(color: Hsla, rem: Pixels) -> impl IntoElement {
    div()
        .size_1p5()
        .rounded_full()
        .bg(color)
        .shadow(vec![BoxShadow {
            color: color.opacity(0.65),
            offset: point(px(0.), px(0.)),
            blur_radius: rem * (8. / 13.),
            spread_radius: px(0.),
            inset: false,
        }])
}

fn hairline_shadow(color: Hsla, direction: f32) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.), px(direction)),
        blur_radius: px(0.),
        spread_radius: px(0.),
        inset: false,
    }
}

// CSS rotates a square; the diamond's corners reach past the layout box like the original.
fn diamond(color: Hsla, size: f32, glow: Option<Hsla>) -> impl IntoElement {
    div().flex_none().size(web_px(size)).child(
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let center = bounds.center();
                let radius = bounds.size.width * std::f32::consts::FRAC_1_SQRT_2;
                if let Some(glow) = glow {
                    for step in (1..=3).rev() {
                        let spread = bounds.size.width * 0.4 * step as f32;
                        paint_diamond(window, center, radius + spread, glow.opacity(0.12));
                    }
                }
                paint_diamond(window, center, radius, color);
            },
        )
        .size_full(),
    )
}

fn paint_diamond(window: &mut Window, center: Point<Pixels>, radius: Pixels, color: Hsla) {
    let mut path = PathBuilder::fill();
    path.move_to(point(center.x, center.y - radius));
    path.line_to(point(center.x + radius, center.y));
    path.line_to(point(center.x, center.y + radius));
    path.line_to(point(center.x - radius, center.y));
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::SaveState;

    #[test]
    fn save_state_prefers_saving_then_auto_then_flash() {
        assert_eq!(SaveState::derive(true, true, true), SaveState::Saving);
        assert_eq!(SaveState::derive(false, true, true), SaveState::Auto);
        assert_eq!(SaveState::derive(false, false, true), SaveState::Saved);
        assert_eq!(SaveState::derive(false, false, false), SaveState::Manual);
    }
}
