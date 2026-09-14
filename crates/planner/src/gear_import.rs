//! Import an item from a game tooltip screenshot, after ImportScreenshotModal.tsx.
use super::*;
use gpui_kit::base::Disableable;
use gpui_kit::component::WindowExt;
use hsplanner_engine::calc::types::ItemBase;
use hsplanner_engine::tooltip_parse::{self, LineStatus, TooltipParseResult};
use hsplanner_ui::{
    components::{modal_eyebrow, modal_footer, modal_header},
    controls::{ButtonSize, ButtonTone, command_button, modal_button, segment},
};

impl GearView {
    pub(super) fn open_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.new(|_| ImportView::new(self.session.clone(), self.mercenary));
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
                .width((window.rem_size() * (560. / 13.)).min(window.viewport_size().width * 0.94))
                .max_h(window.viewport_size().height * 0.86)
                .margin_top(window.viewport_size().height * 0.07)
                .overlay_closable(false)
                .on_ok(|_, _, _| false)
                .child(view.clone())
        });
    }
}

struct ImportView {
    session: Entity<Session>,
    mercenary: bool,
    busy: bool,
    image: Option<Arc<Image>>,
    result: Option<TooltipParseResult>,
    error: Option<String>,
    debug_open: bool,
}

fn preview_format(bytes: &[u8]) -> Option<ImageFormat> {
    match image::guess_format(bytes).ok()? {
        image::ImageFormat::Png => Some(ImageFormat::Png),
        image::ImageFormat::Jpeg => Some(ImageFormat::Jpeg),
        image::ImageFormat::WebP => Some(ImageFormat::Webp),
        image::ImageFormat::Gif => Some(ImageFormat::Gif),
        image::ImageFormat::Bmp => Some(ImageFormat::Bmp),
        _ => None,
    }
}

impl ImportView {
    fn new(session: Entity<Session>, mercenary: bool) -> Self {
        Self {
            session,
            mercenary,
            busy: false,
            image: None,
            result: None,
            error: None,
            debug_open: false,
        }
    }

    fn choose_file(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import screenshot".into()),
        });
        cx.spawn(async move |this, cx| {
            let bytes = match paths.await {
                Ok(Ok(Some(paths))) => match paths.first().map(std::fs::read) {
                    Some(Ok(bytes)) => Ok(Some(bytes)),
                    Some(Err(error)) => Err(format!("Could not read the image: {error}")),
                    None => Ok(None),
                },
                Ok(Ok(None)) => Ok(None),
                _ => Err("Could not open the image picker.".into()),
            };
            let _ = this.update(cx, |this, cx| match bytes {
                Ok(Some(bytes)) => this.analyze(bytes, cx),
                Ok(None) => {}
                Err(error) => {
                    this.error = Some(error);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let clipboard = cx.read_from_clipboard_async();
        cx.spawn(async move |this, cx| {
            let bytes = match clipboard.await {
                Ok(Some(item)) => item.entries().iter().find_map(|entry| match entry {
                    ClipboardEntry::Image(image) => Some(image.bytes().to_vec()),
                    _ => None,
                }),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| match bytes {
                Some(bytes) => this.analyze(bytes, cx),
                None => {
                    this.error = Some("Copy a tooltip screenshot first, then paste it.".into());
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn analyze(&mut self, bytes: Vec<u8>, cx: &mut Context<Self>) {
        self.busy = true;
        self.error = None;
        self.result = None;
        self.image =
            preview_format(&bytes).map(|format| Arc::new(Image::from_bytes(format, bytes.clone())));
        cx.notify();
        let season = self.session.read(cx).snapshot().season.clone();
        let task = cx.background_spawn(async move {
            hsplanner_engine::ocr::ocr_image_bytes(&bytes)
                .map(|lines| tooltip_parse::parse_tooltip_lines(lines, Some(season)))
        });
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match outcome {
                    Ok(result) => this.result = Some(result),
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn item(&self) -> Option<(EquippedItem, &'static ItemBase)> {
        let parsed = self.result.as_ref()?.equipped.clone()?;
        let base = data::get_item(&parsed.base_id)?;
        Some((EquippedItem::from(parsed), base))
    }

    /// First free slot that accepts the item, else the first matching slot (overwrite).
    fn target_slot(&self, base: &ItemBase, cx: &App) -> Option<String> {
        let snapshot = self.session.read(cx).snapshot();
        let inventory = if self.mercenary {
            &snapshot.merc_inventory
        } else {
            &snapshot.inventory
        };
        data::game_config()
            .slots
            .iter()
            .flatten()
            .filter(|slot| gear::accepts(snapshot, &slot.key, base, self.mercenary))
            .min_by_key(|slot| inventory.contains_key(&slot.key))
            .map(|slot| slot.key.clone())
    }

    fn equip(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((item, base)) = self.item() else {
            return;
        };
        let Some(slot) = self.target_slot(base, cx) else {
            self.error = Some("No slot can take this item right now.".into());
            cx.notify();
            return;
        };
        let mercenary = self.mercenary;
        let result = self.session.update(cx, |session, cx| {
            let extra = session.state().settings.extra_charm_slot;
            let mut result = Ok(());
            session.edit(|draft| {
                result = gear::commit(&mut draft.snapshot, &slot, Some(item), mercenary, extra)
            });
            if result.is_ok() {
                cx.notify();
            }
            result
        });
        match result {
            Ok(()) => window.close_dialog(cx),
            Err(error) => {
                self.error = Some(error);
                cx.notify();
            }
        }
    }

    fn stash(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((item, _)) = self.item() else {
            return;
        };
        self.session.update(cx, |session, cx| {
            session.edit(|draft| gear::stash(draft, &item));
            cx.notify();
        });
        window.close_dialog(cx);
    }
}

fn line_row(line: &tooltip_parse::TooltipLine, p: &TooltipTheme) -> Div {
    let color = match line.status {
        LineStatus::Matched => p.text,
        LineStatus::Warning => p.accent_hot,
        LineStatus::Ignored => p.faint,
    };
    let prefix = if line.status == LineStatus::Warning {
        "⚠ "
    } else {
        ""
    };
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(rems(11. / 13.))
        .text_color(color)
        .flex()
        .flex_wrap()
        .gap_1()
        .child(format!("{prefix}{}", line.text))
        .children(
            line.detail
                .as_ref()
                .filter(|_| line.status != LineStatus::Ignored)
                .map(|detail| div().text_color(p.faint).child(format!("— {detail}"))),
        )
}

impl Render for ImportView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let item = self.item();
        let base = item.as_ref().map(|(_, base)| *base);
        let subtitle = base.map_or_else(
            || "Paste a tooltip screenshot or choose a file".to_owned(),
            |base| format!("{} — {}", base.name, base.rarity),
        );
        let target = base.and_then(|base| self.target_slot(base, cx));
        let slot_label = target.as_ref().and_then(|key| {
            data::game_config()
                .slots
                .iter()
                .flatten()
                .find(|slot| &slot.key == key)
                .map(|slot| slot.name.clone())
        });
        let warnings: Vec<_> = self
            .result
            .iter()
            .flat_map(|result| result.lines.iter())
            .filter(|line| line.status == LineStatus::Warning)
            .cloned()
            .collect();
        let mut body = div()
            .id("import-body")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px_6()
            .py_4()
            .flex()
            .flex_col()
            .gap_3()
            .text_size(rems(12. / 13.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        modal_button(
                            "import-choose",
                            if self.busy {
                                "Reading…"
                            } else {
                                "Choose image"
                            },
                            ButtonTone::Neutral,
                            cx,
                        )
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _, _, cx| this.choose_file(cx))),
                    )
                    .child(
                        modal_button("import-paste", "Paste image", ButtonTone::Neutral, cx)
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| this.paste(cx))),
                    )
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(10. / 13.))
                            .text_color(p.faint)
                            .child("from a file or the clipboard"),
                    ),
            );
        if let Some(image) = &self.image {
            body = body.child(
                div()
                    .h(rems(160. / 13.))
                    .w_full()
                    .rounded_sm()
                    .border_1()
                    .border_color(p.border)
                    .flex()
                    .justify_start()
                    .child(img(image.clone()).h_full().object_fit(ObjectFit::Contain)),
            );
        }
        let errors = self
            .error
            .iter()
            .cloned()
            .chain(self.result.iter().flat_map(|result| result.errors.clone()));
        body = body.children(errors.map(|error| {
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(11. / 13.))
                .text_color(p.negative)
                .child(error)
        }));
        if let Some(result) = &self.result
            && item.is_some()
        {
            if !warnings.is_empty() {
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(rems(10. / 13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.accent_hot)
                                .child(format!("MANUAL REVIEW ({})", warnings.len())),
                        )
                        .children(warnings.iter().map(|line| line_row(line, p))),
                );
            }
            body = body
                .child(
                    segment(
                        "import-debug",
                        format!("Debug ({} lines)", result.lines.len()),
                        self.debug_open,
                        cx,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.debug_open = !this.debug_open;
                        cx.notify();
                    })),
                )
                .when(self.debug_open, |body| {
                    body.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .children(result.lines.iter().map(|line| line_row(line, p))),
                    )
                });
        }
        let relic = base.is_some_and(gear::is_relic);
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(modal_header(
                modal_eyebrow("import-eyebrow", "Import"),
                "Import from screenshot",
                Some(subtitle.into()),
                cx,
            ))
            .child(body)
            .child(
                modal_footer(cx)
                    .child(
                        command_button(
                            "import-stash",
                            "Add to stash",
                            ButtonTone::Neutral,
                            ButtonSize::Regular,
                            cx,
                        )
                        .disabled(item.is_none() || relic)
                        .on_click(cx.listener(|this, _, window, cx| this.stash(window, cx))),
                    )
                    .child(
                        modal_button(
                            "import-equip",
                            slot_label.map_or_else(
                                || "Equip".to_owned(),
                                |name| format!("Equip ({name})"),
                            ),
                            ButtonTone::Primary,
                            cx,
                        )
                        .disabled(target.is_none())
                        .on_click(cx.listener(|this, _, window, cx| this.equip(window, cx))),
                    ),
            )
    }
}
