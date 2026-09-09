use crate::shell::Shell;
use gpui_kit::component::button::Button;
use gpui_kit::{prelude::*, *};
use hsplanner_build::{
    session::{Session, WorkspaceState},
    storage::Writer,
};
use hsplanner_ui::controls::PlannerControl;
use hsplanner_ui::theme::TooltipTheme;
use std::path::PathBuf;

pub struct Startup {
    directory: PathBuf,
    shell: Option<Entity<Shell>>,
    error: Option<String>,
    loading: bool,
}
impl Startup {
    pub fn new(
        directory: PathBuf,
        loaded: Result<(Writer, WorkspaceState), String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self {
            directory,
            shell: None,
            error: None,
            loading: false,
        };
        view.loaded(loaded, window, cx);
        view
    }
    fn loaded(
        &mut self,
        result: Result<(Writer, WorkspaceState), String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        match result {
            Ok((writer, state)) => {
                self.error = None;
                self.shell = Some(cx.new(|cx| Shell::new(Session::new(state), writer, window, cx)));
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }
    fn retry(&mut self, recover: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        let directory = self.directory.clone();
        let task = cx.background_spawn(async move {
            if recover {
                Writer::recover(directory)
            } else {
                Writer::open(directory)
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| this.loaded(result, window, cx));
        })
        .detach();
        cx.notify();
    }
    pub fn request_close(&mut self, cx: &mut Context<Self>) {
        if let Some(shell) = &self.shell {
            shell.update(cx, |shell, cx| shell.request_close(cx));
        } else {
            cx.quit();
        }
    }
}
impl Render for Startup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(shell) = &self.shell {
            return shell.clone().into_any_element();
        }
        let palette = cx.global::<TooltipTheme>();
        div().size_full().flex().items_center().justify_center().bg(palette.panel).text_color(palette.text)
            .child(div().max_w(rems(48.)).p_6().flex().flex_col().gap_4()
                .child(div().text_2xl().child(gpui_kit::text!(id="startup-title","Your library could not be opened")))
                .child(gpui_kit::text!(id="startup-error",self.error.clone().unwrap_or_default()))
                .child(div().text_color(palette.muted).child("The saved file stays in place. Restoring the previous copy also keeps the unreadable file for recovery."))
                .child(div().flex().gap_3()
                    .child(Button::new("retry-open").planner_style(cx).label(if self.loading {"Loading…"} else {"Retry"}).on_click(cx.listener(|this,_,window,cx|this.retry(false,window,cx))))
                    .child(Button::new("restore-backup").planner_style(cx).label("Restore previous copy").on_click(cx.listener(|this,_,window,cx|this.retry(true,window,cx))))))
            .into_any_element()
    }
}
