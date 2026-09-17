//! Equipment composition; the parent view retains draft and comparison ownership.
use super::*;
use crate::gear_stash::{self, StashRow};
use crate::item_tooltip;
use gpui_kit::component::button::{ButtonCustomVariant, ButtonVariants};
use hsplanner_ui::components::{panel, panel_with_trailing, section_heading};
use hsplanner_ui::controls::segment;
use hsplanner_ui::scroll::PageScroll;
use hsplanner_ui::tooltip::CursorTooltipExt;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

const STASH_HEADER_PX: f32 = 28.;
const STASH_ENTRY_PX: f32 = 56.;

static ITEM_IMAGES: LazyLock<Mutex<HashMap<String, Arc<RenderImage>>>> =
    LazyLock::new(Default::default);

pub(crate) fn item_icon(id: &str) -> Option<Arc<RenderImage>> {
    cached_icon(ITEM_ICONS, id, id)
}

pub(crate) fn socketable_icon(name: &str) -> Option<Arc<RenderImage>> {
    let key = name.to_lowercase();
    cached_icon(SOCKETABLE_ICONS, &key, &format!("socketable/{key}"))
}

pub(crate) fn augment_icon(id: &str) -> Option<Arc<RenderImage>> {
    cached_icon(AUGMENT_ICONS, id, &format!("augment/{id}"))
}

fn cached_icon(atlas: &[(&str, &[u8])], key: &str, cache_key: &str) -> Option<Arc<RenderImage>> {
    let mut images = ITEM_IMAGES.lock().ok()?;
    if let Some(image) = images.get(cache_key) {
        return Some(image.clone());
    }
    let bytes = atlas.iter().find(|(name, _)| *name == key)?.1;
    let image = decode_icon(bytes)?;
    images.insert(cache_key.into(), image.clone());
    Some(image)
}

pub(super) fn decode_icon(bytes: &[u8]) -> Option<Arc<RenderImage>> {
    let mut rgba = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .ok()?
        .into_rgba8();
    for pixel in rgba.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(rgba)])))
}

struct GearEditor {
    owner: Entity<GearView>,
    picker_width: u16,
    _subscription: Subscription,
}

fn uses_item_picker_width(choosing: bool, picker: &Picker) -> u16 {
    if !choosing {
        return 0;
    }
    match picker {
        Picker::Affix | Picker::Forge | Picker::Augment => 640,
        Picker::Items | Picker::Stash => 680,
        _ => 0,
    }
}

impl Render for GearEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .line_height(relative(1.5))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(self.owner.update(cx, |owner, cx| {
                        if owner.choosing
                            && matches!(owner.picker, Picker::Items | Picker::Stash)
                            && owner.picker_context.is_none()
                        {
                            owner.prepare_item_picker(window, cx);
                        }
                        owner.editor(window, cx)
                    })),
            )
            .child(self.owner.update(cx, |owner, cx| owner.editor_footer(cx)))
    }
}

impl GearView {
    fn open_slot(&mut self, slot: String, window: &mut Window, cx: &mut Context<Self>) {
        self.invalidate_item_picker();
        self.slot = slot;
        self.picker = Picker::Items;
        self.show_all_affixes = false;
        self.error = None;
        self.editing = true;
        self.confirming_close = false;
        self.confirmation_return_focus = None;
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.revert(cx);
        self.choosing = self.candidate.is_none();
        if self.choosing {
            self.prepare_item_picker(window, cx);
        }
        let slot_key = self.slot.clone();
        let owner = cx.entity();
        let window_handle = window.window_handle();
        let editor = cx.new(|cx: &mut Context<GearEditor>| GearEditor {
            picker_width: uses_item_picker_width(self.choosing, &self.picker),
            _subscription: cx.observe(&owner, move |editor, owner, cx| {
                let owner = owner.read(cx);
                let picker_width = uses_item_picker_width(owner.choosing, &owner.picker);
                if editor.picker_width != picker_width {
                    editor.picker_width = picker_width;
                    // The retained editor can update without rebuilding the dialog
                    // layer. Refresh that layer only when its width mode changes.
                    cx.defer(move |cx| {
                        let _ = window_handle.update(cx, |_, window, _| window.refresh());
                    });
                }
                cx.notify();
            }),
            owner: owner.clone(),
        });
        let weak = owner.downgrade();
        let mercenary = self.mercenary;
        window.open_dialog(cx, move |dialog, window, cx| {
            let on_close = weak.clone();
            let cancel = weak.clone();
            let palette = cx.global::<TooltipTheme>();
            let view = owner.read(cx);
            let picker_width = uses_item_picker_width(view.choosing, &view.picker);
            let logical_width = if picker_width > 0 {
                f32::from(picker_width)
            } else if mercenary {
                900.
            } else {
                1180.
            };
            let width = (window.viewport_size().width * 0.96)
                .min(window.rem_size() * (logical_width / 13.));
            let heading = gpui_kit::component::dialog::DialogTitle::new()
                .flex_none()
                .child(
                    div()
                        .font_family(theme::FONT_FAMILY)
                        .font_weight(FontWeight::NORMAL)
                        .line_height(relative(1.5))
                        .px_6()
                        .py_4()
                        .border_b_1()
                        .border_color(palette.border)
                        .child(
                            div()
                                .mb_1p5()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().size_1().rounded_full().bg(palette.accent))
                                .child(
                                    div()
                                        .font_family(theme::MONO_FONT_FAMILY)
                                        .text_size(rems(10. / 13.))
                                        .text_color(palette.faint)
                                        .child(hsplanner_ui::tooltip_text::TooltipText::new(
                                            "gear-slot-eyebrow",
                                            tr("gear.slot"),
                                            0.12,
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .text_size(rems(17. / 13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(palette.text)
                                .child(gear_stash::slot_label(&slot_key)),
                        ),
                );
            dialog
                .p_0()
                .gap_0()
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(palette.panel_secondary, 0.),
                    linear_color_stop(palette.background, 1.),
                ))
                .width(width)
                .h(window.viewport_size().height * 0.88)
                .margin_top(window.viewport_size().height * 0.06)
                .overlay_closable(false)
                // Focused buttons keep their native Enter activation. An Enter
                // bubbling from search or the dialog itself must not discard a draft.
                .on_ok(|_, _, _| false)
                .on_cancel(move |_, window, cx| {
                    cancel
                        .update(cx, |owner, cx| owner.request_editor_close(window, cx))
                        .unwrap_or(true)
                })
                .on_close(move |_, _, cx| {
                    let _ = on_close.update(cx, |owner, cx| {
                        owner.editing = false;
                        owner.confirming_close = false;
                        owner.confirmation_return_focus = None;
                        owner.error = None;
                        owner.revert(cx);
                    });
                })
                // The stock title/content split inserts an internal gap_y_2.
                // One column retains DialogTitle semantics without that extra gap.
                .child(
                    div()
                        .size_full()
                        .flex()
                        .flex_col()
                        .child(heading)
                        .child(editor.clone()),
                )
        });
        if self.choosing {
            window.focus(&self.search.read(cx).focus_handle(cx), cx);
        }
    }

    fn slot_cell(
        &self,
        key: &str,
        width: f32,
        height: f32,
        empty: Option<&str>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let inventory = if self.mercenary {
            &snapshot.merc_inventory
        } else {
            &snapshot.inventory
        };
        let item = inventory.get(key);
        let equipped_ids = item_tooltip::equipped_ids(inventory);
        let base = item.and_then(|i| data::get_item(&i.base_id));
        let name = gear_stash::slot_label(key);
        let locked = key == "offhand"
            && inventory
                .get("weapon")
                .and_then(|i| data::get_item(&i.base_id))
                .is_some_and(|b| {
                    b.two_handed.unwrap_or(false) && (self.mercenary || !snapshot.can_offhand(b))
                });
        let foreground = base.map_or(p.faint, |b| theme::rarity_color(&b.rarity, cx));
        let border = base.map_or(theme::inventory_border(locked), |_| {
            if key.starts_with("charm_") {
                p.accent_deep
            } else {
                foreground.opacity(0.4)
            }
        });
        let label = format!(
            "{name}: {}",
            base.map_or(
                if locked {
                    tr("gear.locked")
                } else {
                    tr("gear.empty")
                },
                |b| b.name.as_str()
            )
        );
        let charm = key.starts_with("charm_");
        let key = key.to_owned();
        let tip_id = SharedString::from(format!("slot-tip-{key}"));
        let cell = Button::new(SharedString::from(format!("slot-{key}")))
            .planner_style(cx)
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(base.map_or(theme::inventory_cell(locked), |b| {
                        theme::rarity_surface(&b.rarity, cx)
                    }))
                    .foreground(foreground)
                    .hover(p.panel_secondary),
            )
            .p_0()
            .w(rems(width / 13.))
            .h(rems(height / 13.))
            .border_color(border)
            .when(base.is_none() && !charm, |view| view.border_dashed())
            .accessibility_label(label)
            .child(
                div()
                    .size_full()
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when_some(base.and_then(|b| item_icon(&b.id)), |v, icon| {
                        v.child(
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .bottom_0()
                                .left_0()
                                .when(!charm, |v| v.top_1().right_1().bottom_1().left_1())
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(img(icon).size_full().object_fit(ObjectFit::Contain)),
                        )
                    })
                    .when(base.is_none(), |v| {
                        v.child(
                            div()
                                .text_size(rems(7. / 13.))
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_center()
                                .child(if locked {
                                    tr("gear.two_handed_short").into()
                                } else {
                                    empty.unwrap_or(&name).to_uppercase()
                                }),
                        )
                    })
                    .when(item.is_some_and(|i| i.stars.unwrap_or(0) > 0), |v| {
                        v.child(
                            div()
                                .absolute()
                                .top_0()
                                .left_1()
                                .text_color(p.accent_hot)
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(rems(8. / 13.))
                                .child(format!("★{}", item.and_then(|i| i.stars).unwrap_or(0))),
                        )
                    })
                    .when(item.is_some_and(|i| i.socket_count > 0), |v| {
                        v.child(
                            div()
                                .absolute()
                                .bottom_0()
                                .right_1()
                                .text_color(p.faint)
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(rems(8. / 13.))
                                .child(
                                    item.map(|i| {
                                        format!(
                                            "{}/{}◇",
                                            i.socketed.iter().flatten().count(),
                                            i.socket_count
                                        )
                                    })
                                    .unwrap_or_default(),
                                ),
                        )
                    }),
            )
            .on_click(
                cx.listener(move |this, _, window, cx| this.open_slot(key.clone(), window, cx)),
            );
        match item {
            Some(item) => {
                item_tooltip::with_item_tooltip(tip_id, item, equipped_ids, cell).into_any_element()
            }
            None => cell.into_any_element(),
        }
    }

    fn doll(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let equipment = data::game_config()
            .slots
            .iter()
            .flatten()
            .filter(|s| !s.key.starts_with("charm_"))
            .count();
        let count = snapshot
            .inventory
            .keys()
            .filter(|k| !k.starts_with("charm_"))
            .count();
        let row = || div().flex().gap_3().items_start();
        let col = |width: f32| div().w(rems(width / 13.)).flex().justify_center();
        let relics = div().flex().flex_col().gap_2().children((1..=5).map(|n| {
            self.slot_cell(
                &format!("relic_{n}"),
                44.,
                44.,
                Some(&trf("gear.relic_short", &[("n", (n).to_string())])),
                cx,
            )
        }));
        let potions = div().flex().gap_2().children((1..=4).map(|n| {
            let key = format!("potion_{n}");
            let enabled = !snapshot
                .disabled_potions
                .get(&key)
                .copied()
                .unwrap_or(false);
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .child(self.slot_cell(
                    &key,
                    30.,
                    96.,
                    Some(&trf("gear.potion_short", &[("n", (n).to_string())])),
                    cx,
                ))
                .when(snapshot.inventory.contains_key(&key), |view| {
                    view.child(
                        Button::new(SharedString::from(format!("enabled-{key}")))
                            .planner_style(cx)
                            .custom(
                                ButtonCustomVariant::new(cx)
                                    .color(if enabled {
                                        p.accent_hot
                                    } else {
                                        p.background.opacity(0.)
                                    })
                                    .hover(p.accent_deep),
                            )
                            .size(rems(10. / 13.))
                            .p_0()
                            .rounded_full()
                            .border_color(if enabled {
                                p.accent_hot
                            } else {
                                p.border_strong
                            })
                            .bg(if enabled {
                                p.accent_hot
                            } else {
                                p.background.opacity(0.)
                            })
                            .when(enabled, |button| {
                                button.shadow(vec![BoxShadow {
                                    color: p.accent_hot.opacity(0.5),
                                    offset: point(px(0.), px(0.)),
                                    blur_radius: px(6.),
                                    spread_radius: px(0.),
                                    inset: false,
                                }])
                            })
                            .accessibility_label(trf(
                                if enabled {
                                    "gear.potion_enabled"
                                } else {
                                    "gear.potion_disabled"
                                },
                                &[("n", n.to_string())],
                            ))
                            .cursor_tooltip(if enabled {
                                tr("gear.effects_on")
                            } else {
                                tr("gear.effects_off")
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.session.update(cx, |session, cx| {
                                    session.edit(|draft| {
                                        draft
                                            .snapshot
                                            .disabled_potions
                                            .insert(key.clone(), enabled);
                                    });
                                    cx.notify();
                                });
                            })),
                    )
                })
        }));
        panel_with_trailing(
            "equipment-panel",
            tr("gear.equipment"),
            div()
                .flex()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(10. / 13.))
                .text_color(p.faint)
                .child(
                    div()
                        .text_color(if count > 0 { p.accent_hot } else { p.muted })
                        .child(count.to_string()),
                )
                .child(trf(
                    "gear.equipped_count",
                    &[("equipment", (equipment).to_string())],
                )),
            cx,
        )
        .child(
            div().child(
                div()
                    .p_4()
                    .rounded_sm()
                    .border_1()
                    .border_color(p.border_strong)
                    .bg(theme::inventory_surface())
                    .flex()
                    .gap_3()
                    .child(relics)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                row()
                                    .items_end()
                                    .child(col(96.))
                                    .child(
                                        col(220.)
                                            .child(self.slot_cell("helmet", 88., 88., None, cx)),
                                    )
                                    .child(col(96.).child(self.slot_cell(
                                        "amulet",
                                        44.,
                                        44.,
                                        Some(tr("gear.amulet_short")),
                                        cx,
                                    ))),
                            )
                            .child(
                                row()
                                    .child(
                                        col(96.)
                                            .child(self.slot_cell("weapon", 96., 176., None, cx)),
                                    )
                                    .child(
                                        col(220.)
                                            .h(rems(176. / 13.))
                                            .items_center()
                                            .child(self.slot_cell("armor", 120., 150., None, cx)),
                                    )
                                    .child(
                                        col(96.)
                                            .child(self.slot_cell("offhand", 96., 176., None, cx)),
                                    ),
                            )
                            .child(
                                row()
                                    .child(col(96.))
                                    .child(
                                        col(220.)
                                            .gap_2()
                                            .child(self.slot_cell(
                                                "ring_1",
                                                44.,
                                                44.,
                                                Some(tr("gear.ring_short")),
                                                cx,
                                            ))
                                            .child(self.slot_cell("belt", 68., 44., None, cx))
                                            .child(self.slot_cell(
                                                "ring_2",
                                                44.,
                                                44.,
                                                Some(tr("gear.ring_short")),
                                                cx,
                                            )),
                                    )
                                    .child(col(96.)),
                            )
                            .child(
                                row()
                                    .child(
                                        col(96.)
                                            .child(self.slot_cell("gloves", 88., 88., None, cx)),
                                    )
                                    .child(col(220.).child(potions))
                                    .child(
                                        col(96.).child(self.slot_cell("boots", 88., 88., None, cx)),
                                    ),
                            ),
                    ),
            ),
        )
    }

    fn charms(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let session = self.session.read(cx);
        let inventory = &session.snapshot().inventory;
        let extra = session.state().settings.extra_charm_slot;
        let charms: Vec<_> = inventory
            .iter()
            .filter(|(k, _)| k.starts_with("charm_"))
            .map(|(k, i)| {
                let b = data::get_item(&i.base_id);
                (
                    k.clone(),
                    b.and_then(|b| b.width).unwrap_or(1),
                    b.and_then(|b| b.height).unwrap_or(1),
                )
            })
            .collect();
        let layout = &self.charm_layout;
        let next = data::game_config()
            .slots
            .iter()
            .flatten()
            .find(|s| s.key.starts_with("charm_") && !inventory.contains_key(&s.key))
            .map(|s| s.key.clone());
        let mut grid = div().relative().w(rems(12.5)).h(rems(46.5));
        for cell in 0..33u32 {
            let row = cell / 3;
            let col = cell % 3;
            let blocked = [9, 11, 23].contains(&cell) || (!extra && cell == 21);
            if layout.occupied_cells() & (1 << cell) != 0 && !blocked {
                continue;
            }
            let tile = div()
                .absolute()
                .left(rems(col as f32 * 4.25))
                .top(rems(row as f32 * 4.25))
                .w(rems(4.))
                .h(rems(4.));
            grid = grid.child(if blocked {
                tile.child(
                    div()
                        .size_full()
                        .border_1()
                        .rounded_sm()
                        .border_color(theme::inventory_border(true))
                        .border_dashed()
                        .bg(theme::inventory_cell(true))
                        .opacity(0.6),
                )
                .into_any_element()
            } else {
                let key = next.clone();
                tile.child(
                    Button::new(SharedString::from(format!("empty-charm-{cell}")))
                        .planner_style(cx)
                        .size_full()
                        .p_0()
                        .child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(rems(18. / 13.))
                                .text_color(p.accent_hot.opacity(0.))
                                .hover(|v| v.text_color(p.accent_hot.opacity(0.4)))
                                .child("+"),
                        )
                        .border_color(theme::inventory_border(false))
                        .bg(theme::inventory_cell(false))
                        .disabled(key.is_none())
                        .accessibility_label(tr("gear.add_charm"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if let Some(key) = &key {
                                this.open_slot(key.clone(), window, cx);
                            }
                        })),
                )
                .into_any_element()
            });
        }
        for placed in layout.placements() {
            grid = grid.child(
                div()
                    .absolute()
                    .left(rems(placed.col() as f32 * 4.25))
                    .top(rems(placed.row() as f32 * 4.25))
                    .child(self.slot_cell(
                        placed.slot(),
                        (placed.width() as f32 * 4.25 - 0.25) * 13.,
                        (placed.height() as f32 * 4.25 - 0.25) * 13.,
                        None,
                        cx,
                    )),
            );
        }
        panel_with_trailing(
            "charms-panel",
            tr("gear.charm_inventory"),
            div()
                .flex()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(10. / 13.))
                .text_color(p.faint)
                .child(
                    div()
                        .text_color(if layout.overflow().is_empty() {
                            p.accent_hot
                        } else {
                            p.negative
                        })
                        .child(
                            charms
                                .iter()
                                .map(|(_, w, h)| w * h)
                                .sum::<u32>()
                                .to_string(),
                        ),
                )
                .child(format!(" / {}", if extra { 30 } else { 29 })),
            cx,
        )
        .child(
            div().child(
                div()
                    .p_3()
                    .border_1()
                    .rounded_sm()
                    .border_color(p.border_strong)
                    .bg(theme::inventory_surface())
                    .child(grid),
            ),
        )
        .children(layout.overflow().iter().map(|key| {
            div()
                .px_3()
                .pb_2()
                .child(self.slot_cell(key, 64., 64., None, cx))
                .child(div().text_color(p.negative).child(tr("gear.does_not_fit")))
        }))
    }

    pub(super) fn refresh_stash_rows(&mut self, cx: &App) {
        let query = self.stash_search.read(cx).value().to_string();
        let stash = &self.session.read(cx).draft().stash;
        if self
            .stash_group
            .as_deref()
            .is_some_and(|group| !gear_stash::stash_groups(stash).iter().any(|g| g == group))
        {
            self.stash_group = None;
        }
        let rows = gear_stash::group_stash_rows(stash, &query, self.stash_group.as_deref());
        if self.stash_rows != rows {
            self.stash_list.reset(rows.len());
            self.stash_rows = rows;
        }
    }

    fn stash_list_height(&self) -> Rems {
        let px: f32 = self
            .stash_rows
            .iter()
            .map(|row| match row {
                StashRow::Header { .. } => STASH_HEADER_PX,
                StashRow::Entry { .. } => STASH_ENTRY_PX,
            })
            .sum();
        rems((px / 13.).min(28.))
    }

    fn stash_group_chips(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let groups = gear_stash::stash_groups(&self.session.read(cx).draft().stash);
        let chip = |id: SharedString, label: String, group: Option<String>, active: bool| {
            segment(id, label, active, cx).on_click(cx.listener(move |this, _, _, cx| {
                this.stash_group = group.clone();
                this.refresh_stash_rows(cx);
                cx.notify();
            }))
        };
        div()
            .px_3()
            .pb_2()
            .flex()
            .flex_wrap()
            .gap_1()
            .text_color(p.muted)
            .when(groups.len() > 1, |view| {
                view.child(chip(
                    "stash-group-all".into(),
                    tr("gear.all_slots").into(),
                    None,
                    self.stash_group.is_none(),
                ))
                .children(groups.into_iter().map(|group| {
                    let active = self.stash_group.as_deref() == Some(group.as_str());
                    chip(
                        SharedString::from(format!("stash-group-{group}")),
                        gear_stash::group_label(&group),
                        Some(group),
                        active,
                    )
                }))
            })
    }

    fn render_stash_header(&self, group: &str, count: usize, cx: &Context<Self>) -> AnyElement {
        let p = cx.global::<TooltipTheme>();
        div()
            .w_full()
            .h(px(STASH_HEADER_PX))
            .px_3()
            .flex()
            .items_end()
            .justify_between()
            .gap_2()
            .pb_1()
            .border_b_1()
            .border_color(p.accent_deep.opacity(0.3))
            .font_family(theme::MONO_FONT_FAMILY)
            .text_size(rems(10. / 13.))
            .child(
                div()
                    .text_color(p.accent_hot.opacity(0.8))
                    .child(gear_stash::group_label(group).to_uppercase()),
            )
            .child(div().text_color(p.faint).child(count.to_string()))
            .into_any_element()
    }

    fn render_stash_row(
        &mut self,
        index: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = cx.global::<TooltipTheme>();
        let entry_index = match self.stash_rows.get(index) {
            Some(StashRow::Header { group, count }) => {
                return self.render_stash_header(group, *count, cx);
            }
            Some(StashRow::Entry { index, .. }) => *index,
            None => return div().into_any_element(),
        };
        let session = self.session.read(cx);
        let Some(entry) = session.draft().stash.get(entry_index) else {
            return div().into_any_element();
        };
        let Some(base) = data::get_item(&entry.item.base_id) else {
            return div().into_any_element();
        };
        let target = data::game_config()
            .slots
            .iter()
            .flatten()
            .filter(|slot| gear::accepts(session.snapshot(), &slot.key, base, false))
            .min_by_key(|slot| session.snapshot().inventory.contains_key(&slot.key))
            .map(|slot| slot.key.clone());
        let item = entry.item.clone();
        let remove = entry.id.clone();
        let equipped_ids = item_tooltip::equipped_ids(&session.snapshot().inventory);
        let row = div()
            .w_full()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(p.border)
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .size(rems(36. / 13.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .children(
                        item_icon(&base.id)
                            .map(|icon| img(icon).size_full().object_fit(ObjectFit::ScaleDown)),
                    ),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .text_size(rems(11. / 13.))
                            .text_color(theme::rarity_color(&base.rarity, cx))
                            .child(base.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(rems(9. / 13.))
                            .text_color(p.faint)
                            .child(format!(
                                "{} · ★{} · {}◇",
                                item_tooltip::base_type_label(&base.base_type),
                                item.stars.unwrap_or(0),
                                item.socket_count
                            )),
                    ),
            )
            .children(
                stash_equip_targets(&base.base_type, target)
                    .into_iter()
                    .map(|(label, target)| {
                        let item = item.clone();
                        Button::new(SharedString::from(format!(
                            "equip-{}",
                            target.as_deref().unwrap_or("none")
                        )))
                        .planner_style(cx)
                        .small()
                        .label(label)
                        .disabled(target.is_none())
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                if let Some(slot) = &target {
                                    this.open_slot(slot.clone(), window, cx);
                                    this.candidate = Some(item.clone());
                                    this.choosing = false;
                                    this.load_icon();
                                    this.changed(cx);
                                }
                            },
                        ))
                    }),
            )
            .child(
                hsplanner_ui::controls::icon_button("remove", "×", true, cx)
                    .accessibility_label(trf(
                        "gear.remove_named_from_stash",
                        &[("arg0", (base.name).to_string())],
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session.update(cx, |session, cx| {
                            session.edit(|draft| draft.stash.retain(|entry| entry.id != remove));
                            cx.notify();
                        });
                    })),
            );
        item_tooltip::with_item_tooltip(
            SharedString::from(format!("stash-{}", entry.id)),
            &entry.item,
            equipped_ids,
            row,
        )
        .into_any_element()
    }

    fn stash(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let session = self.session.read(cx);
        let empty = self.stash_rows.is_empty();
        panel("stash-panel", tr("gear.stash"), cx)
            .min_w(rems(280. / 13.))
            .flex_1()
            .child(
                div()
                    .px_3()
                    .pb_2()
                    .child(Input::new(&self.stash_search).planner_style(cx)),
            )
            .child(self.stash_group_chips(cx))
            .child(
                div()
                    .text_size(rems(11. / 13.))
                    .when(!empty, |view| {
                        view.child(
                            list(
                                self.stash_list.clone(),
                                cx.processor(Self::render_stash_row),
                            )
                            .w_full()
                            .h(self.stash_list_height()),
                        )
                    })
                    .when(empty, |v| {
                        v.child(div().px_3().py_8().text_color(p.muted).child(
                            if session.draft().stash.is_empty() {
                                tr("gear.stash_empty")
                            } else {
                                tr("gear.no_matching_items")
                            },
                        ))
                    }),
            )
    }

    fn merc_loadout(&self, window: &Window, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let slots = &hsplanner_engine::calc::mercenary::data().slots;
        let count = slots
            .iter()
            .filter(|key| snapshot.merc_inventory.contains_key(*key))
            .count();
        panel_with_trailing(
            "merc-loadout",
            tr("gear.loadout"),
            div()
                .flex()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(10. / 13.))
                .text_color(p.faint)
                .child(
                    div()
                        .text_color(if count > 0 { p.accent_hot } else { p.muted })
                        .child(count.to_string()),
                )
                .child(trf(
                    "gear.loadout_count",
                    &[("arg0", (slots.len()).to_string())],
                )),
            cx,
        )
        .child(
            div()
                .grid()
                .grid_cols(
                    if window.viewport_size().width >= window.rem_size() * (640. / 13.) {
                        2
                    } else {
                        1
                    },
                )
                .gap_1p5()
                .children(slots.iter().map(|key| {
                    let name = gear_stash::slot_label(key);
                    let item = snapshot.merc_inventory.get(key);
                    let base = item.and_then(|i| data::get_item(&i.base_id));
                    let runeword = base.zip(item).and_then(|(base, item)| {
                        data::detect_runeword(
                            base,
                            &item
                                .socketed
                                .iter()
                                .map(|v| v.as_deref())
                                .collect::<Vec<_>>(),
                        )
                    });
                    let locked = key == "offhand"
                        && snapshot
                            .merc_inventory
                            .get("weapon")
                            .and_then(|i| data::get_item(&i.base_id))
                            .is_some_and(|b| b.two_handed.unwrap_or(false));
                    let foreground = if runeword.is_some() {
                        p.accent_hot
                    } else {
                        base.map_or(p.faint, |b| theme::rarity_color(&b.rarity, cx))
                    };
                    let mut badges = Vec::new();
                    if let Some((base, item)) = base.zip(item) {
                        if let Some((min, max)) = base.defense_min.zip(base.defense_max) {
                            badges.push(trf(
                                "gear.defense_badge",
                                &[("min", (min).to_string()), ("max", (max).to_string())],
                            ));
                        }
                        if let Some((min, max)) = base.damage_min.zip(base.damage_max) {
                            badges.push(trf(
                                "gear.damage_badge",
                                &[("min", (min).to_string()), ("max", (max).to_string())],
                            ));
                        }
                        if item.socket_count > 0 {
                            badges.push(format!(
                                "{}/{}◇",
                                item.socketed.iter().flatten().count(),
                                item.socket_count
                            ));
                        }
                        if data::can_star_forge(key, &base.rarity) && item.stars.unwrap_or(0) > 0 {
                            badges.push("★".repeat(item.stars.unwrap_or(0) as usize));
                        }
                        if let Some(level) = base.requires_level.filter(|level| *level > 0) {
                            badges.push(trf("item.level_short", &[("level", (level).to_string())]));
                        }
                    }
                    let target = key.clone();
                    let row = Button::new(SharedString::from(format!("merc-slot-{key}")))
                        .planner_style(cx)
                        .custom(
                            ButtonCustomVariant::new(cx)
                                .color(base.map_or(p.background.opacity(0.), |b| {
                                    theme::rarity_surface(&b.rarity, cx)
                                }))
                                .hover(p.panel_secondary),
                        )
                        .w_full()
                        .min_w_0()
                        .h_auto()
                        .min_h(rems(3.5))
                        .px_2()
                        .py_1p5()
                        .border_color(base.map_or(p.border, |_| foreground.opacity(0.4)))
                        .when(base.is_none(), |view| view.border_dashed())
                        .accessibility_label(format!(
                            "{name}: {}",
                            base.map_or(
                                if locked {
                                    tr("gear.locked")
                                } else {
                                    tr("gear.empty")
                                },
                                |b| b.name.as_str()
                            )
                        ))
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .w_20()
                                        .flex_shrink_0()
                                        .truncate()
                                        .font_family(theme::MONO_FONT_FAMILY)
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_size(rems(9. / 13.))
                                        .text_color(p.faint)
                                        .child(name.to_uppercase()),
                                )
                                .child(
                                    div()
                                        .size(rems(38. / 13.))
                                        .flex_shrink_0()
                                        .border_1()
                                        .rounded_sm()
                                        .border_color(p.border_strong)
                                        .bg(linear_gradient(
                                            180.,
                                            linear_color_stop(p.background, 0.),
                                            linear_color_stop(p.panel_secondary, 1.),
                                        ))
                                        .children(base.and_then(|b| item_icon(&b.id)).map(|i| {
                                            img(i).size_full().object_fit(ObjectFit::Contain)
                                        })),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .font_family(theme::FONT_FAMILY)
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(rems(
                                                    if base.is_some() { 12. } else { 11. } / 13.,
                                                ))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(foreground)
                                                .child(
                                                    runeword
                                                        .map(|r| r.name.clone())
                                                        .or_else(|| base.map(|b| b.name.clone()))
                                                        .unwrap_or_else(|| {
                                                            if locked {
                                                                tr("gear.locked_two_handed")
                                                            } else {
                                                                tr("gear.empty")
                                                            }
                                                            .into()
                                                        }),
                                                ),
                                        )
                                        .children(base.map(|b| {
                                            div()
                                                .truncate()
                                                .text_size(rems(10. / 13.))
                                                .text_color(p.muted)
                                                .child(if runeword.is_some() {
                                                    trf(
                                                        "gear.runeword_base",
                                                        &[(
                                                            "arg0",
                                                            item_tooltip::base_type_label(
                                                                &b.base_type,
                                                            )
                                                            .to_owned(),
                                                        )],
                                                    )
                                                } else {
                                                    item_tooltip::base_type_label(&b.base_type)
                                                        .to_owned()
                                                })
                                        })),
                                )
                                .when(!badges.is_empty(), |view| {
                                    view.child(
                                        div()
                                            .flex_shrink_0()
                                            .font_family(theme::MONO_FONT_FAMILY)
                                            .text_size(rems(9. / 13.))
                                            .text_color(p.faint)
                                            .child(badges.join(" · ")),
                                    )
                                }),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_slot(target.clone(), window, cx)
                        }));
                    match item {
                        Some(item) => item_tooltip::with_item_tooltip(
                            SharedString::from(format!("merc-slot-tip-{key}")),
                            item,
                            item_tooltip::equipped_ids(&snapshot.merc_inventory),
                            row,
                        )
                        .into_any_element(),
                        None => row.into_any_element(),
                    }
                })),
        )
    }

    pub(super) fn overview(&mut self, window: &Window, cx: &Context<Self>) -> Div {
        if self.stash_rem != window.rem_size() {
            self.stash_rem = window.rem_size();
            self.stash_list.reset(self.stash_rows.len());
        }
        if self.mercenary {
            return self.merc_loadout(window, cx);
        }
        let p = cx.global::<TooltipTheme>();
        div().size_full().bg(p.background).child(
            div()
                .id("gear-overview")
                .size_full()
                .page_scroll(&self.scroll)
                .overflow_y_scroll()
                .child(
                    div()
                        .p_6()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(
                            div()
                                .flex()
                                .items_end()
                                .justify_between()
                                .gap_3()
                                .child(section_heading(
                                    "gear-heading",
                                    tr("gear.loadout"),
                                    tr("gear.gear"),
                                    cx,
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_3()
                                        .child(
                                            div()
                                                .text_size(rems(10. / 13.))
                                                .font_family(theme::MONO_FONT_FAMILY)
                                                .text_color(p.faint)
                                                .child(trf(
                                                    "gear.catalog_counts",
                                                    &[
                                                        (
                                                            "arg0",
                                                            (data::data().items.len()).to_string(),
                                                        ),
                                                        (
                                                            "arg1",
                                                            (data::data().gems.len()).to_string(),
                                                        ),
                                                        (
                                                            "arg2",
                                                            (data::data().runes.len()).to_string(),
                                                        ),
                                                    ],
                                                )),
                                        )
                                        .child(
                                            hsplanner_ui::controls::command_button(
                                                "import-screenshot",
                                                tr("gear.import_screenshot"),
                                                hsplanner_ui::controls::ButtonTone::Neutral,
                                                hsplanner_ui::controls::ButtonSize::Small,
                                                cx,
                                            )
                                            .on_click(
                                                cx.listener(|this, _, window, cx| {
                                                    this.open_import(window, cx)
                                                }),
                                            ),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .flex_wrap()
                                .when(
                                    window.viewport_size().width
                                        < window.rem_size() * (1024. / 13.),
                                    |view| view.justify_center(),
                                )
                                .gap_4()
                                .child(self.doll(cx))
                                .child(self.charms(cx))
                                .child(self.stash(cx)),
                        ),
                ),
        )
    }
}

fn stash_equip_targets(
    base_type: &str,
    target: Option<String>,
) -> Vec<(&'static str, Option<String>)> {
    if base_type.eq_ignore_ascii_case("ring") {
        vec![
            (
                tr("gear.ring_one"),
                target.as_ref().map(|_| "ring_1".into()),
            ),
            (
                tr("gear.ring_two"),
                target.as_ref().map(|_| "ring_2".into()),
            ),
        ]
    } else {
        vec![(tr("gear.equip_dialog"), target)]
    }
}

#[cfg(test)]
mod stash_target_tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn rings_offer_both_slots_even_when_automatic_target_is_first() {
        let ring = data::data()
            .items
            .values()
            .find(|item| item.base_type == "Ring")
            .unwrap();
        let targets = stash_equip_targets(&ring.base_type, Some("ring_1".into()));
        assert_eq!(
            targets,
            vec![
                ("Ring 1…", Some("ring_1".into())),
                ("Ring 2…", Some("ring_2".into()))
            ]
        );
        assert!(
            stash_equip_targets(&ring.base_type, None)
                .iter()
                .all(|(_, target)| target.is_none())
        );
        assert_eq!(
            stash_equip_targets("Armor", Some("armor".into())),
            vec![("Equip…", Some("armor".into()))]
        );
    }
}
