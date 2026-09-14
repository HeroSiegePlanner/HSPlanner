use super::*;
use gpui_kit::component::{WindowExt, dialog::Confirm};

#[derive(Clone, Copy)]
pub(super) enum EditKind {
    NewBuild,
    NewFolder,
    RenameFolder,
    Rename,
    AddProfile,
}

impl LibraryView {
    pub(super) fn edit_dialog(
        &mut self,
        kind: EditKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = match kind {
            EditKind::NewBuild => "New build",
            EditKind::NewFolder => "New folder",
            EditKind::RenameFolder => "Rename folder",
            EditKind::Rename => "Rename build",
            EditKind::AddProfile => "Add profile",
        };
        self.error = None;
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let build_id = self.selected.clone();
        if matches!(kind, EditKind::Rename) {
            let name = build_id
                .as_ref()
                .and_then(|id| self.session.read(cx).state().library.build(id))
                .map(|b| b.name.clone())
                .unwrap_or_default();
            input.update(cx, |input, cx| input.set_value(name, window, cx));
        }
        if matches!(kind, EditKind::RenameFolder) {
            let name = self
                .session
                .read(cx)
                .state()
                .library
                .folders
                .iter()
                .find(|folder| Some(&folder.id) == self.folder.as_ref())
                .map(|folder| folder.name.clone())
                .unwrap_or_default();
            input.update(cx, |input, cx| input.set_value(name, window, cx));
        }
        let focus = input.read(cx).focus_handle(cx);
        let weak = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let target = weak.clone();
            let input_for_save = input.clone();
            let build_id = build_id.clone();
            let error = weak.upgrade().and_then(|view| view.read(cx).error.clone());
            let content = div()
                .flex()
                .flex_col()
                .gap_3()
                .child(Input::new(&input).planner_style(cx))
                .children(error.map(|error| {
                    div()
                        .text_color(cx.global::<TooltipTheme>().negative)
                        .child(error)
                }))
                .child(
                    Button::new("submit-library-edit")
                        .planner_style(cx)
                        .label(title)
                        .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
                        }),
                );
            dialog.title(title).child(content).on_ok(move |_, _, cx| {
                // Both the button and Enter use Dialog's confirm action. A
                // failed validation keeps the same input and focus mounted;
                // successful dismissal restores focus through the modal host.
                let name = input_for_save.read(cx).value().to_string();
                target
                    .update(cx, |this, cx| {
                        let folder = this.folder.clone();
                        let success = this.apply(cx, |session| match kind {
                            EditKind::NewBuild => session.new_build(&name).map(|_| ()),
                            EditKind::NewFolder => session.edit_library(|library| {
                                library.create_folder(&name, folder).map(|_| ())
                            }),
                            EditKind::RenameFolder => session.edit_library(|library| {
                                library.rename_folder(
                                    folder.as_deref().ok_or("Select a folder")?,
                                    &name,
                                )
                            }),
                            EditKind::Rename => session.edit_library(|library| {
                                library.rename(build_id.as_deref().ok_or("Select a build")?, &name)
                            }),
                            EditKind::AddProfile => session.edit_library(|library| {
                                let build = library
                                    .build_mut(build_id.as_deref().ok_or("Select a build")?)?;
                                let snapshot = build
                                    .profile(&build.active_profile_id)
                                    .or_else(|| build.profiles.first())
                                    .ok_or("Build has no profile")?
                                    .snapshot()?;
                                build.profiles.push(hsplanner_build::library::Profile::new(
                                    &name, &snapshot,
                                )?);
                                build.updated_at = hsplanner_build::library::now();
                                Ok(())
                            }),
                        });
                        if success && matches!(kind, EditKind::NewBuild) {
                            cx.emit(Opened);
                        }
                        success
                    })
                    .unwrap_or(false)
            })
        });
        window.focus(&focus, cx);
    }
}
