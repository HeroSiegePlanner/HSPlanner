//! Share dialog: copy the build as a code, or store it on hsplanner.app for a link.
use gpui_kit::{base::Disableable, prelude::*, *};
use hsplanner_build::session::Session;
use hsplanner_ui::components::{
    ModalSize, PlannerDialogExt, modal_body, modal_eyebrow, modal_header, modal_layout,
};
use hsplanner_ui::{
    controls::{ButtonTone, modal_button, segment},
    i18n::tr,
    theme::{self, TooltipTheme},
};

pub(super) fn open(session: Entity<Session>, window: &mut Window, cx: &mut App) {
    let view = cx.new(|_| ShareView {
        session,
        include_stash: false,
        link: None,
        status: None,
        error: None,
        upload: None,
    });
    window.open_planner_modal(cx, ModalSize::Form, move |dialog, _, _| {
        dialog.child(view.clone())
    });
}

struct ShareView {
    session: Entity<Session>,
    include_stash: bool,
    /// The link once the site stored the build, so copying it again uploads nothing.
    link: Option<String>,
    /// Catalog key of what the last action did.
    status: Option<&'static str>,
    error: Option<String>,
    upload: Option<Task<()>>,
}

impl ShareView {
    fn build_name(&self, cx: &App) -> Option<String> {
        let session = self.session.read(cx);
        let id = session.draft().build_id.as_deref()?;
        Some(session.state().library.build(id)?.name.clone())
    }

    fn code(&self, include_stash: bool, cx: &App) -> Result<String, String> {
        let name = self.build_name(cx);
        let draft = self.session.read(cx).draft();
        let stash = if include_stash { &draft.stash[..] } else { &[] };
        hsplanner_build::codec::encode_loadouts_with_stash(
            name.as_deref(),
            &draft.snapshot,
            &draft.notes,
            &draft.loadouts,
            stash,
        )
    }

    fn copied(&mut self, text: String, status: &'static str, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.status = Some(status);
        self.error = None;
        cx.notify();
    }

    fn failed(&mut self, error: String, cx: &mut Context<Self>) {
        self.status = None;
        self.error = Some(error);
        cx.notify();
    }

    fn copy_code(&mut self, cx: &mut Context<Self>) {
        match self.code(self.include_stash, cx) {
            Ok(code) => self.copied(code, "shell.copied", cx),
            Err(error) => self.failed(error, cx),
        }
    }

    /// Stores the build on hsplanner.app and copies its link; the stash stays local.
    fn share_link(&mut self, cx: &mut Context<Self>) {
        if self.upload.is_some() {
            return;
        }
        if let Some(link) = self.link.clone() {
            return self.copied(link, "shell.link_copied", cx);
        }
        let code = match self.code(false, cx) {
            Ok(code) => code,
            Err(error) => return self.failed(error, cx),
        };
        self.status = Some("shell.sharing");
        self.error = None;
        let request = hsplanner_library::site::publish(cx.http_client(), code);
        self.upload = Some(cx.spawn(async move |this, cx| {
            let result = request.await;
            let _ = this.update(cx, |this, cx| {
                this.upload = None;
                match result {
                    Ok(link) => {
                        this.link = Some(link.clone());
                        this.copied(link, "shell.link_copied", cx);
                    }
                    Err(key) => this.failed(key, cx),
                }
            });
        }));
        cx.notify();
    }
}

/// One way to share: what it is, what it does, and its controls underneath.
fn choice(title: &'static str, help: &'static str, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(p.border)
        .bg(p.panel)
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.text)
                .child(tr(title)),
        )
        .child(
            div()
                .text_size(rems(12. / 13.))
                .text_color(p.muted)
                .child(tr(help)),
        )
}

impl Render for ShareView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let (positive, negative, muted, text) = (p.positive, p.negative, p.muted, p.text);
        let has_stash = !self.session.read(cx).draft().stash.is_empty();
        let uploading = self.upload.is_some();
        let outcome = match (&self.error, self.status) {
            (Some(error), _) => {
                let known = error.starts_with("site.error.") || error.starts_with("library.error.");
                let detail = if known {
                    tr(error).to_owned()
                } else {
                    error.clone()
                };
                Some((detail, negative))
            }
            (None, Some(status)) => Some((
                tr(status).to_owned(),
                if uploading { muted } else { positive },
            )),
            (None, None) => None,
        };
        let code = choice("share.code_title", "share.code_help", cx).child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .when(has_stash, |row| {
                    row.child(
                        segment(
                            "share-include-stash",
                            tr("share.include_stash"),
                            self.include_stash,
                            cx,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.include_stash = !this.include_stash;
                            cx.notify();
                        })),
                    )
                })
                .child(
                    modal_button(
                        "share-copy-code",
                        tr("share.copy_code"),
                        ButtonTone::Primary,
                        cx,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.copy_code(cx))),
                ),
        );
        let link_label = if self.link.is_some() {
            "share.copy_link"
        } else {
            "share.create_link"
        };
        let link = choice("share.link_title", "share.link_help", cx).child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .children(self.link.clone().map(|link| {
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(rems(11. / 13.))
                        .text_color(text)
                        .child(link)
                }))
                .child(
                    modal_button("share-create-link", tr(link_label), ButtonTone::Primary, cx)
                        .disabled(uploading)
                        .on_click(cx.listener(|this, _, _, cx| this.share_link(cx))),
                ),
        );
        modal_layout(window)
            .child(modal_header(
                modal_eyebrow("share-eyebrow", tr("chrome.builds")),
                tr("share.title"),
                self.build_name(cx).map(Into::into),
                cx,
            ))
            .child(
                modal_body("share-body")
                    .child(code)
                    .child(link)
                    .children(outcome.map(|(detail, color)| {
                        div()
                            .text_size(rems(12. / 13.))
                            .text_color(color)
                            .child(detail)
                    })),
            )
    }
}
