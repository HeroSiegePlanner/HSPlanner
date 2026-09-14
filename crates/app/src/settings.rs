//! Application preferences dialog, after the reference SettingsModal.
use crate::shell::{Shell, ToggleProfileControls};
use gpui_kit::{
    base::Link,
    component::{WindowExt, checkbox::Checkbox},
    prelude::*,
    *,
};
use hsplanner_build::session::{Session, Settings};
use hsplanner_ui::{
    components::{modal_eyebrow, modal_header},
    controls::segment,
    numbers::compact,
    theme::{self, TooltipTheme, UI_ZOOM_STEPS},
    tooltip_text::TooltipText,
};

const KOFI_URL: &str = "https://ko-fi.com/zium1337";
const GITHUB_URL: &str = "https://github.com/zium1337/HSPlanner";
const SAVE_SHORTCUT: &str = if cfg!(target_os = "macos") {
    "⌘S"
} else {
    "Ctrl+S"
};
const NUMBER_SCALES: [(&str, &str, &str); 4] = [
    ("none", "None", "12,345"),
    ("thousands", "Thousands", "12.3k"),
    ("millions", "Millions", "12.3M"),
    ("billions", "Billions", "12.3B"),
];
const PREVIEW_SAMPLES: [f64; 3] = [45_678., 12_345_678., 2_500_000_000.];

pub(super) fn open(
    session: Entity<Session>,
    shell: WeakEntity<Shell>,
    window: &mut Window,
    cx: &mut App,
) {
    let view = cx.new(|cx| SettingsView::new(session, shell, cx));
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
            .width((window.rem_size() * (560. / 13.)).min(window.viewport_size().width * 0.92))
            .max_h(window.viewport_size().height * 0.86)
            .margin_top(window.viewport_size().height * 0.07)
            .child(view.clone())
    });
}

struct SettingsView {
    session: Entity<Session>,
    shell: WeakEntity<Shell>,
    _subscriptions: Vec<Subscription>,
}

impl SettingsView {
    fn new(session: Entity<Session>, shell: WeakEntity<Shell>, cx: &mut Context<Self>) -> Self {
        let mut subscriptions = vec![cx.observe(&session, |_, _, cx| cx.notify())];
        if let Some(shell) = shell.upgrade() {
            subscriptions.push(cx.observe(&shell, |_, _, cx| cx.notify()));
        }
        Self {
            session,
            shell,
            _subscriptions: subscriptions,
        }
    }

    fn edit(&self, cx: &mut Context<Self>, edit: impl FnOnce(&mut Settings)) {
        self.session.update(cx, |session, cx| {
            let mut settings = session.state().settings.clone();
            edit(&mut settings);
            session.set_settings(settings);
            cx.notify();
        });
    }
}

fn section(id: &'static str, title: &str, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div().flex().flex_col().gap_2().child(
        div()
            .mb_0p5()
            .pb_1p5()
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(p.accent_deep.opacity(0.2))
            .font_family(theme::MONO_FONT_FAMILY)
            .text_size(rems(10. / 13.))
            .text_color(p.accent_hot.opacity(0.7))
            .child(div().size_1().flex_none().rounded_full().bg(p.accent_deep))
            .child(TooltipText::new(id, title.to_uppercase(), 0.18)),
    )
}

fn hint(text: String, color: Hsla) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(rems(10. / 13.))
        .text_color(color)
        .child(text)
}

fn description(text: &'static str, cx: &App) -> Div {
    div()
        .pl_6()
        .text_size(rems(12. / 13.))
        .text_color(cx.global::<TooltipTheme>().muted)
        .child(text)
}

fn external(id: &'static str, label: &'static str, url: &'static str, cx: &App) -> Link {
    Link::new(id)
        .child(label)
        .href(url)
        .text_color(cx.global::<TooltipTheme>().accent_hot)
        .underline()
        .accessibility_label(label)
        .open_with(|url, _, _, cx| cx.open_url(url))
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let settings = self.session.read(cx).state().settings.clone();
        let profile_controls = self
            .shell
            .upgrade()
            .is_some_and(|shell| shell.read(cx).profile_controls);
        let zoom = theme::normalize_zoom(settings.ui_zoom);
        let scale = settings.number_scale.clone();
        let preview = PREVIEW_SAMPLES
            .iter()
            .map(|sample| compact(*sample, &scale))
            .collect::<Vec<_>>()
            .join("  ·  ");
        let saving = section("settings-saving", "Saving", cx)
            .child(
                Checkbox::new("settings-auto-save")
                    .label("Auto-save")
                    .checked(settings.auto_save)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        let checked = *checked;
                        this.edit(cx, |settings| settings.auto_save = checked);
                    })),
            )
            .child(description(
                "Saves changes to the active build as you make them.",
                cx,
            ))
            .child(if settings.auto_save {
                hint(format!("{SAVE_SHORTCUT} still saves instantly"), p.faint)
            } else {
                hint(
                    format!("Manual mode — press {SAVE_SHORTCUT} to save the active build"),
                    p.accent_hot.opacity(0.8),
                )
            });
        let numbers = section("settings-numbers", "Numbers", cx)
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Largest unit"),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1p5()
                    .children(NUMBER_SCALES.into_iter().map(|(key, label, sample)| {
                        segment(
                            SharedString::from(format!("settings-scale-{key}")),
                            format!("{label} · {sample}"),
                            scale == key,
                            cx,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.edit(cx, |settings| settings.number_scale = key.into());
                        }))
                    })),
            )
            .child(
                div()
                    .flex()
                    .gap_1p5()
                    .child(hint("Preview ·".into(), p.faint))
                    .child(hint(preview, p.accent_hot.opacity(0.8))),
            );
        let display = section("settings-display", "Display", cx)
            .child(div().font_weight(FontWeight::SEMIBOLD).child("UI scale"))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1p5()
                    .children(UI_ZOOM_STEPS.into_iter().map(|step| {
                        segment(
                            SharedString::from(format!("settings-zoom-{}", (step * 100.) as u32)),
                            format!("{:.0}%", step * 100.),
                            (zoom - step).abs() < 0.001,
                            cx,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.edit(cx, |settings| settings.ui_zoom = step);
                        }))
                    })),
            )
            .child(hint(
                "Ctrl + / Ctrl − zooms too, this is the one that sticks".into(),
                p.faint,
            ));
        let interface = section("settings-interface", "Interface", cx)
            .child(
                Checkbox::new("settings-profile-controls")
                    .label("Build and profile controls")
                    .checked(profile_controls)
                    .on_click(|_, window, cx| {
                        window.dispatch_action(Box::new(ToggleProfileControls), cx)
                    }),
            )
            .child(description(
                "Shows the build name, profile switcher and save state under the top bar.",
                cx,
            ));
        let credits = section("settings-credits", "Credits", cx)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .child(img(crate::chrome::logo()).size(rems(20. / 13.)))
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(12. / 13.))
                            .text_color(p.accent_hot)
                            .child(TooltipText::new("settings-brand", "HSPLANNER", 0.18)),
                    )
                    .child(
                        div()
                            .rounded_sm()
                            .border_1()
                            .border_color(p.border_strong)
                            .px_1p5()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(10. / 13.))
                            .text_color(p.muted)
                            .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                    ),
            )
            .child(
                div()
                    .text_size(rems(12. / 13.))
                    .text_color(p.muted)
                    .child("Built and maintained by zium."),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(external("settings-kofi", "Support on Ko-fi", KOFI_URL, cx))
                    .child(external("settings-github", "GitHub", GITHUB_URL, cx)),
            )
            .child(
                div()
                    .mt_1()
                    .pt_2p5()
                    .border_t_1()
                    .border_color(p.border)
                    .child(hint(
                        "Fan-made planner. Hero Siege © Panic Art Studios — not affiliated."
                            .to_uppercase(),
                        p.faint,
                    )),
            );
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_size(rems(1.))
            .child(modal_header(
                modal_eyebrow("settings-eyebrow", "Preferences"),
                "Settings",
                Some("Stored on this device".into()),
                cx,
            ))
            .child(
                div()
                    .id("settings-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_6()
                    .py_5()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(saving)
                    .child(numbers)
                    .child(display)
                    .child(interface)
                    .child(credits),
            )
    }
}
