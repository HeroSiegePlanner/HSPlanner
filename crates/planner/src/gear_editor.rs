//! Equipped-item editor presentation; draft and calculation operations remain in GearView.
use super::*;
use crate::gear_sections::{
    Section, Tone, chip, empty_note, eyebrow, ghost_button, header_action, hint, icon_button,
    mono_summary, row_box, row_index, section_card, units,
};
use crate::item_tooltip;
use crate::skill_details::stat_name;
use gpui_kit::component::button::{ButtonCustomVariant, ButtonVariants};
use hsplanner_engine::calc::affix::{apply_stars_to_ranged_value, rolled_affix_value_with_stars};
use hsplanner_engine::calc::types::{ItemBase, ItemSet};
use hsplanner_ui::controls::{ButtonSize, ButtonTone, command_button, modal_button};
use hsplanner_ui::scroll::PageScroll;
use hsplanner_ui::tooltip::CursorTooltipExt;
use hsplanner_ui::tooltip_text::TooltipText;

impl GearView {
    fn card(&self, section: Section, cx: &Context<Self>) -> Stateful<Div> {
        let (key, default_open) = (section.id, section.default_open);
        let open = self.section_open(key, default_open);
        section_card(
            section,
            open,
            cx.listener(move |this, _, _, cx| this.toggle_section(key, default_open, cx)),
            cx,
        )
    }

    fn sections_action(
        &self,
        id: &'static str,
        label: &str,
        expanded: bool,
        cx: &Context<Self>,
    ) -> Button {
        command_button(
            id,
            label.to_owned(),
            ButtonTone::Ghost,
            ButtonSize::Small,
            cx,
        )
        .on_click(cx.listener(move |this, _, _, cx| this.set_sections_mode(expanded, cx)))
    }

    pub(super) fn inspector(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let (faint, muted, negative) = (p.faint, p.muted, p.negative);
        let mut content = div()
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .text_size(units(12.))
            .children(self.error.as_ref().map(|error| {
                div()
                    .border_1()
                    .border_color(negative.opacity(0.3))
                    .bg(negative.opacity(0.08))
                    .px_3()
                    .py_2p5()
                    .text_color(negative)
                    .child(error.clone())
            }));
        let Some(item) = self.candidate.clone() else {
            return content.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_5()
                    .p_8()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_size(units(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(tr("gear.no_item_selected")),
                            )
                            .child(div().text_color(muted).child(tr("gear.slot_will_empty"))),
                    )
                    .child(self.picker_button(
                        "choose-empty-item",
                        tr("gear.choose_item"),
                        Picker::Items,
                        cx,
                    )),
            );
        };
        let Some(base) = data::get_item(&item.base_id) else {
            return content.child(tr("gear.unknown_base"));
        };
        content = content.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(eyebrow("configure", tr("gear.configure"), cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(self.sections_action("expand-all", tr("gear.expand_all"), true, cx))
                        .child(div().text_color(faint.opacity(0.5)).child("|"))
                        .child(self.sections_action(
                            "collapse-all",
                            tr("gear.collapse_all"),
                            false,
                            cx,
                        )),
                ),
        );
        if let Some(set) = base.set_id.as_deref().and_then(data::get_set)
            && !set.bonuses.is_empty()
        {
            content = content.child(self.set_section(set, &item, cx));
        }
        if gear::max_sockets(&item) > 0 || !item.socketed.is_empty() {
            content = content.child(self.sockets_section(&item, base, cx));
        }
        if gear::is_relic(base) {
            content = content.child(self.relic_tier_section(&item, window, cx));
        } else {
            content = content.children(self.rolls_section(&item, base, window, cx));
        }
        content = content.children(self.runeword_section(&item, base, cx));
        if data::can_star_forge(&self.slot, &base.rarity) {
            content = content.child(self.stars_section(&item, cx));
        }
        for (key, label, picker, picked) in [
            (
                "random_skill_element",
                tr("gear.random_skill_element"),
                Picker::RandomElement,
                item.random_skill_element.clone(),
            ),
            (
                "random_skill",
                tr("gear.random_skill"),
                Picker::RandomSkill,
                item.random_skill_id
                    .as_deref()
                    .and_then(data::skill_name_by_id)
                    .map(str::to_owned),
            ),
            (
                "grant_subskills",
                tr("gear.subskill_bonus"),
                Picker::Subskill,
                item.subskill_boost_skill_id
                    .as_deref()
                    .and_then(data::skill_name_by_id)
                    .map(str::to_owned),
            ),
            (
                "all_skills_class",
                tr("gear.class_bonus"),
                Picker::Class,
                item.all_skills_class_id
                    .as_deref()
                    .and_then(data::get_class)
                    .map(|class| class.name.clone()),
            ),
        ] {
            let present = (key == "random_skill" && base.random_skill_pool.is_some())
                || base
                    .implicit
                    .as_ref()
                    .is_some_and(|stats| stats.contains_key(key))
                || item.implicit_overrides.contains_key(key)
                || (matches!(key, "all_skills_class" | "random_skill_element")
                    && item.affixes.iter().any(|affix| {
                        data::get_affix(&affix.affix_id)
                            .is_some_and(|affix| affix.stat_key.as_deref() == Some(key))
                    }));
            if present {
                content = content.child(self.pick_section(key, label, picker, picked, cx));
            }
        }
        if hsplanner_build::gear::accepts_affixes(base) || !item.affixes.is_empty() {
            content = content.child(self.affixes_section(&item, base, window, cx));
        }
        if data::forge_kind_for(&base.rarity).is_some() {
            content = content.child(self.forged_section(&item, window, cx));
        }
        if self.slot == "armor" {
            content = content.child(self.augment_section(&item, cx));
        }
        content
    }

    fn set_section(&self, set: &ItemSet, item: &EquippedItem, cx: &Context<Self>) -> Stateful<Div> {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let inventory = if self.mercenary {
            &snapshot.merc_inventory
        } else {
            &snapshot.inventory
        };
        let equipped_ids: Vec<String> = inventory
            .iter()
            .filter(|(slot, _)| **slot != self.slot)
            .map(|(_, equipped)| equipped.base_id.clone())
            .chain(std::iter::once(item.base_id.clone()))
            .collect();
        let count = equipped_ids
            .iter()
            .filter(|id| data::get_item(id).is_some_and(|b| b.set_id.as_deref() == Some(&set.id)))
            .count();
        let mut body = div().px_3p5().py_2p5().flex().flex_col().gap_1p5();
        for bonus in &set.bonuses {
            let active = count >= bonus.pieces as usize;
            body = body.child(
                div()
                    .text_size(units(11.))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap_2()
                            .child(
                                div()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .text_size(units(9.))
                                    .text_color(if active { p.positive } else { p.faint })
                                    .child(TooltipText::new(
                                        SharedString::from(format!("set-bonus-{}", bonus.pieces)),
                                        trf(
                                            "gear.set_bonus_count",
                                            &[("arg0", (bonus.pieces).to_string())],
                                        ),
                                        0.14,
                                    )),
                            )
                            .when(active, |v| {
                                v.child(
                                    div()
                                        .font_family(theme::MONO_FONT_FAMILY)
                                        .text_size(units(10.))
                                        .text_color(p.positive)
                                        .child("✓"),
                                )
                            }),
                    )
                    .children(bonus.descriptions.iter().flatten().map(|line| {
                        div()
                            .ml_3()
                            .text_size(units(10.5))
                            .line_height(relative(1.35))
                            .text_color(if active {
                                p.positive.opacity(0.9)
                            } else {
                                p.muted.opacity(0.55)
                            })
                            .child(line.clone())
                    })),
            );
        }
        body = body.child(
            div()
                .mt_1()
                .pt_2()
                .border_t_1()
                .border_color(p.text.opacity(0.05))
                .child(eyebrow("set-items", tr("gear.set_items"), cx))
                .child(div().ml_3().mt_1().flex().flex_col().gap_0p5().children(
                    set.items.iter().map(|piece| {
                        let worn = equipped_ids.contains(&piece.item_id);
                        div()
                            .text_size(units(10.5))
                            .line_height(relative(1.35))
                            .text_color(if worn {
                                p.positive
                            } else {
                                p.muted.opacity(0.6)
                            })
                            .child(format!(
                                "{} {} ({})",
                                if worn { "✓" } else { "·" },
                                piece.name,
                                piece.slot
                            ))
                    }),
                )),
        );
        self.card(
            Section::new("set-summary", &set.name, Tone::Set)
                .default_open(count >= 2)
                .right(
                    mono_summary(
                        trf(
                            "gear.set_pieces",
                            &[
                                ("count", (count).to_string()),
                                ("arg0", (set.items.len()).to_string()),
                            ],
                        ),
                        p.positive.opacity(0.8),
                    )
                    .into_any_element(),
                )
                .body(body),
            cx,
        )
    }

    fn sockets_section(
        &self,
        item: &EquippedItem,
        base: &ItemBase,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let p = cx.global::<TooltipTheme>();
        let max = gear::max_sockets(item);
        let names: Vec<String> = item
            .socketed
            .iter()
            .flatten()
            .map(|id| socketable_name(id))
            .collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        let summary = match (names.len(), unique.len()) {
            (0, _) => None,
            (n, 1) => Some(format!("{} ×{n}", unique[0].to_uppercase())),
            (n, _) => Some(trf("gear.socketed_count", &[("n", (n).to_string())])),
        };
        let right = div()
            .flex()
            .items_center()
            .gap_2()
            .children(summary.map(|text| mono_summary(text, p.faint)))
            .child(
                icon_button("sockets-minus", "−", cx)
                    .disabled(item.socket_count == 0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.edit(cx, |item| {
                            gear::set_socket_count(item, item.socket_count.saturating_sub(1))
                        })
                    })),
            )
            .child(
                div()
                    .min_w(units(42.))
                    .text_center()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(11.))
                    .text_color(p.accent_hot)
                    .child(format!("{}/{max}", item.socket_count)),
            )
            .child(
                icon_button("sockets-plus", "+", cx)
                    .disabled(item.socket_count >= max)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.edit(cx, |item| {
                            gear::set_socket_count(item, item.socket_count + 1)
                        })
                    })),
            );
        let body = if item.socket_count == 0 {
            empty_note("no-sockets", tr("gear.no_sockets"), cx)
        } else {
            let mut rows = div().p_2().flex().flex_col().gap_1p5();
            for index in 0..item.socket_count as usize {
                rows = rows.child(self.socket_row(item, base, index, cx));
            }
            if let Some(count) = base.sockets.filter(|count| *count != item.socket_count) {
                rows = rows.child(hint(
                    "base-sockets",
                    &trf("gear.base_socket_count", &[("count", (count).to_string())]),
                    cx,
                ));
            }
            rows
        };
        self.card(
            Section::new("sockets", tr("gear.sockets"), Tone::Default)
                .default_open(item.socketed.iter().any(Option::is_some))
                .right(right)
                .body(body),
            cx,
        )
    }

    fn socket_row(
        &self,
        item: &EquippedItem,
        base: &ItemBase,
        index: usize,
        cx: &Context<Self>,
    ) -> Div {
        let p = cx.global::<TooltipTheme>();
        let socketed = item.socketed.get(index).and_then(|id| id.clone());
        let built_in = base
            .rainbow_sockets
            .as_ref()
            .is_some_and(|slots| slots.contains(&(index as u32 + 1)));
        let rainbow = built_in || item.socket_types.get(index) == Some(&SocketType::Rainbow);
        let (name, tier) = match socketed.as_deref() {
            Some(id) => (Some(socketable_name(id)), socketable_tier(id)),
            None => (None, None),
        };
        let icon = name
            .as_deref()
            .and_then(super::presentation::socketable_icon);
        let toggle = if built_in {
            chip(tr("gear.rainbow_short"), p.angelic, p.angelic.opacity(0.6))
                .size(units(20.))
                .flex()
                .items_center()
                .justify_center()
                .into_any_element()
        } else {
            icon_button(
                SharedString::from(format!("socket-type-{index}")),
                if rainbow {
                    tr("gear.rainbow_short")
                } else {
                    tr("gear.normal_short")
                },
                cx,
            )
            .when(rainbow, |b| {
                b.text_color(p.angelic).border_color(p.angelic.opacity(0.6))
            })
            .cursor_tooltip(if rainbow {
                tr("gear.rainbow_socket_hint")
            } else {
                tr("gear.normal_socket_hint")
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.edit(cx, |item| {
                    item.socket_types
                        .resize(item.socket_count as usize, SocketType::Normal);
                    item.socket_types[index] = if rainbow {
                        SocketType::Normal
                    } else {
                        SocketType::Rainbow
                    };
                })
            }))
            .into_any_element()
        };
        let placeholder = div()
            .size(units(12.))
            .flex_shrink_0()
            .rounded_sm()
            .border_1()
            .border_dashed()
            .border_color(p.accent_deep.opacity(0.4));
        let trigger = Button::new(SharedString::from(format!("socket-pick-{index}")))
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(p.panel_secondary.opacity(0.4))
                    .foreground(p.text)
                    .hover(p.panel_secondary),
            )
            .flex_1()
            .min_w_0()
            .h_auto()
            .px_2()
            .py_1()
            .rounded_sm()
            .border_1()
            .border_color(p.accent_deep.opacity(0.25))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .map(|v| match icon {
                                Some(icon) => v.child(
                                    img(icon)
                                        .size(units(16.))
                                        .flex_shrink_0()
                                        .object_fit(ObjectFit::Contain),
                                ),
                                None => v.child(placeholder),
                            })
                            .child(
                                div()
                                    .truncate()
                                    .text_size(units(12.))
                                    .when(name.is_none(), |v| v.italic().text_color(p.faint))
                                    .child(
                                        name.clone()
                                            .unwrap_or_else(|| tr("gear.empty_socket").into()),
                                    ),
                            )
                            .children(tier.map(|tier| {
                                chip(
                                    trf("item.tier_short", &[("tier", (tier).to_string())]),
                                    p.accent_hot.opacity(0.75),
                                    p.accent_deep.opacity(0.4),
                                )
                            })),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(units(9.))
                            .text_color(p.faint)
                            .child(TooltipText::new(
                                SharedString::from(format!("browse-{index}")),
                                tr("gear.browse"),
                                0.14,
                            )),
                    ),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.choose_picker(Picker::Socket(index), window, cx)
            }));
        row_box(cx)
            .flex()
            .items_center()
            .gap_1p5()
            .child(row_index(index, cx))
            .child(toggle)
            .child(trigger)
            .when(socketed.is_some(), |row| {
                row.child(
                    icon_button(SharedString::from(format!("socket-clear-{index}")), "×", cx)
                        .cursor_tooltip(tr("gear.clear_socket"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.edit(cx, |item| {
                                let _ = gear::set_socket(item, index, None);
                            })
                        })),
                )
            })
    }

    fn rolls_section(
        &mut self,
        item: &EquippedItem,
        base: &ItemBase,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let entries = roll_entries(item, base);
        if entries.is_empty() {
            return None;
        }
        let p = cx.global::<TooltipTheme>();
        let (accent_hot, faint, text) = (p.accent_hot, p.faint, p.text);
        let mut rows = div().p_2().flex().flex_col().gap_1p5();
        for entry in &entries {
            let (key, stat, is_skill) = (entry.key.clone(), entry.stat.clone(), entry.is_skill);
            let (lo, hi) = entry.bounds;
            self.roll_slider(
                key.clone(),
                (lo, hi),
                entry.pinned.unwrap_or(hi).clamp(lo, hi),
                RollTarget {
                    affix: None,
                    forged: false,
                    stat,
                    skill: is_skill,
                    relic_tier: false,
                },
                cx,
            );
            let shown = match entry.pinned {
                Some(value) => item_tooltip::format_ranged((value, value), &entry.format_key),
                None => item_tooltip::format_ranged((lo, hi), &entry.format_key),
            };
            let range = item_tooltip::format_ranged((lo, hi), &entry.format_key);
            let pinned = entry.pinned.is_some();
            let reset = entry.clone();
            rows = rows.child(
                row_box(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .items_baseline()
                                    .gap_1p5()
                                    .child(
                                        div()
                                            .font_family(theme::MONO_FONT_FAMILY)
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(accent_hot)
                                            .child(shown),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_color(text.opacity(0.85))
                                            .child(entry.label.clone()),
                                    )
                                    .when(pinned, |v| {
                                        v.child(chip(
                                            tr("gear.custom"),
                                            accent_hot,
                                            accent_hot.opacity(0.6),
                                        ))
                                    }),
                            )
                            .child(
                                icon_button(SharedString::from(format!("reset-{key}")), "×", cx)
                                    .cursor_tooltip(tr("gear.reset_full_range"))
                                    .when(!pinned, |b| b.invisible())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.unpin_roll(&reset, window, cx)
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .mt_1()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(self.roll_control(&key, &entry.label, window, cx))
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .text_size(units(9.))
                                    .text_color(faint)
                                    .child(range),
                            ),
                    ),
            );
        }
        rows = rows.child(hint("rolls-hint", tr("gear.roll_hint"), cx));
        let total = entries.len();
        let pinned_count = entries.iter().filter(|e| e.pinned.is_some()).count();
        let right = div()
            .flex()
            .items_center()
            .gap_2()
            .child(mono_summary(
                if pinned_count > 0 {
                    trf(
                        "gear.pinned_count",
                        &[
                            ("pinned_count", (pinned_count).to_string()),
                            ("total", (total).to_string()),
                        ],
                    )
                } else {
                    trf("gear.rollable_count", &[("total", (total).to_string())])
                },
                if pinned_count > 0 {
                    accent_hot.opacity(0.8)
                } else {
                    faint
                },
            ))
            .when(pinned_count > 0, |v| {
                v.child(
                    header_action("rolls-reset", tr("gear.reset"), true, cx).on_click(cx.listener(
                        move |this, _, window, cx| {
                            for entry in entries.iter().filter(|e| e.pinned.is_some()) {
                                this.unpin_roll(entry, window, cx);
                            }
                        },
                    )),
                )
            });
        Some(
            self.card(
                Section::new("stat-rolls", tr("gear.stat_rolls"), Tone::Default)
                    .default_open(pinned_count > 0)
                    .right(right)
                    .body(rows),
                cx,
            ),
        )
    }

    fn relic_tier_section(
        &mut self,
        item: &EquippedItem,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        const KEY: &str = "relic-tier";
        let p = cx.global::<TooltipTheme>();
        let (accent_hot, faint) = (p.accent_hot, p.faint);
        let tier = gear::relic_tier(item);
        self.roll_slider(
            KEY.to_owned(),
            (1., f64::from(gear::RELIC_MAX_TIER)),
            f64::from(tier),
            RollTarget {
                affix: None,
                forged: false,
                stat: String::new(),
                skill: false,
                relic_tier: true,
            },
            cx,
        );
        let body = div()
            .p_2()
            .flex()
            .flex_col()
            .gap_1p5()
            .child(
                row_box(cx).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(self.roll_control(KEY, tr("gear.relic_tier"), window, cx))
                        .child(
                            div()
                                .flex_shrink_0()
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(units(9.))
                                .text_color(faint)
                                .child(format!("1–{}", gear::RELIC_MAX_TIER)),
                        ),
                ),
            )
            .child(hint("relic-tier-hint", tr("gear.relic_tier_hint"), cx));
        self.card(
            Section::new("relic-tier", tr("gear.relic_tier_title"), Tone::Default)
                .default_open(true)
                .right(mono_summary(
                    trf("item.tier_short", &[("tier", (tier).to_string())]),
                    accent_hot,
                ))
                .body(body),
            cx,
        )
    }

    fn unpin_roll(&mut self, entry: &RollEntry, window: &mut Window, cx: &mut Context<Self>) {
        let stat = entry.stat.clone();
        let is_skill = entry.is_skill;
        self.edit(cx, move |item| {
            if is_skill {
                item.skill_bonus_overrides.remove(&stat);
            } else {
                item.implicit_overrides.remove(&stat);
            }
        });
        self.set_slider(&entry.key, entry.bounds.1, window, cx);
    }

    fn runeword_section(
        &self,
        item: &EquippedItem,
        base: &ItemBase,
        cx: &Context<Self>,
    ) -> Option<Stateful<Div>> {
        let count = compatible_runeword_count(base, gear::max_sockets(item));
        if count == 0 {
            return None;
        }
        let p = cx.global::<TooltipTheme>();
        let active = item_tooltip::runeword_for(base, Some(item)).map(|r| r.name.clone());
        let right = mono_summary(
            trf("gear.compatible_count", &[("count", (count).to_string())]),
            p.faint,
        );
        let body = div().px_3().py_2().child(
            ghost_button("pick-runeword", tr("gear.browse_runewords"), cx).on_click(
                cx.listener(|this, _, window, cx| this.choose_picker(Picker::Runeword, window, cx)),
            ),
        );
        Some(
            self.card(
                Section::new("runeword", tr("gear.runeword_presets"), Tone::Default)
                    .default_open(active.is_some())
                    .right(right)
                    .body(body),
                cx,
            ),
        )
    }

    fn stars_section(&self, item: &EquippedItem, cx: &Context<Self>) -> Stateful<Div> {
        let p = cx.global::<TooltipTheme>();
        let stars = item.stars.unwrap_or(0);
        let right = div().flex().items_center().gap_2().map(|v| {
            if stars > 0 {
                v.child(mono_summary("★".repeat(stars as usize), p.accent_hot))
                    .child(
                        header_action("stars-clear", tr("gear.clear"), true, cx).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.edit(cx, |item| item.stars = Some(0))
                            }),
                        ),
                    )
            } else {
                v.child(mono_summary(tr("gear.no_bonus"), p.faint))
            }
        });
        let mut row = div().flex().items_center().gap_1p5();
        for target in 1..=5u32 {
            let filled = target <= stars;
            row = row.child(
                Button::new(SharedString::from(format!("star-{target}")))
                    .ghost()
                    .h_auto()
                    .p_0()
                    .text_size(units(20.))
                    .line_height(relative(1.))
                    .text_color(if filled {
                        p.accent_hot
                    } else {
                        p.muted.opacity(0.3)
                    })
                    .when(filled, |b| {
                        b.shadow(vec![BoxShadow {
                            color: p.accent_hot.opacity(0.45),
                            offset: point(px(0.), px(0.)),
                            blur_radius: px(10.),
                            spread_radius: px(0.),
                            inset: false,
                        }])
                    })
                    .label("★")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.edit(cx, |item| {
                            let current = item.stars.unwrap_or(0);
                            item.stars = Some(if current == target {
                                target - 1
                            } else {
                                target
                            });
                        })
                    })),
            );
        }
        if stars > 0 {
            row = row.child(
                div()
                    .ml_1p5()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(10.))
                    .text_color(p.faint)
                    .child(format!("{stars}/5")),
            );
        }
        let body = div().p_3().flex().flex_col().gap_2().child(row).child(hint(
            "stars-hint",
            tr("gear.stars_hint"),
            cx,
        ));
        self.card(
            Section::new("stars", tr("gear.stars"), Tone::Default)
                .default_open(stars > 0)
                .right(right)
                .body(body),
            cx,
        )
    }

    fn pick_section(
        &self,
        key: &'static str,
        label: &str,
        picker: Picker,
        picked: Option<String>,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let p = cx.global::<TooltipTheme>();
        let right = match &picked {
            Some(name) => mono_summary(name.to_uppercase(), p.accent_hot),
            None => mono_summary(tr("gear.not_rolled"), p.faint),
        };
        let body = div().px_3().py_2().child(
            ghost_button(
                key,
                &trf(
                    "gear.pick_named",
                    &[("arg0", (label.to_lowercase()).to_string())],
                ),
                cx,
            )
            .on_click(
                cx.listener(move |this, _, window, cx| this.choose_picker(picker, window, cx)),
            ),
        );
        self.card(
            Section::new(key, label, Tone::Default)
                .default_open(picked.is_some())
                .right(right)
                .body(body),
            cx,
        )
    }

    fn affixes_section(
        &mut self,
        item: &EquippedItem,
        base: &ItemBase,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = cx.global::<TooltipTheme>();
        let (accent_hot, accent_deep, text) = (p.accent_hot, p.accent_deep, p.text);
        let stars = data::can_star_forge(&base.slot, &base.rarity)
            .then_some(item.stars)
            .flatten();
        let at_cap = base
            .max_affixes
            .is_some_and(|max| item.affixes.len() as u32 >= max);
        let right = div()
            .flex()
            .items_center()
            .gap_2()
            .child(mono_summary(
                match base.max_affixes {
                    Some(max) => format!("{} / {max}", item.affixes.len()),
                    None => item.affixes.len().to_string(),
                },
                accent_hot.opacity(0.8),
            ))
            .child(
                header_action("affix-add", tr("gear.add"), false, cx)
                    .disabled(at_cap)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.choose_picker(Picker::Affix, window, cx)
                    })),
            );
        let body = if item.affixes.is_empty() {
            empty_note("no-affixes", tr("gear.no_affixes"), cx)
        } else {
            let mut rows = div().p_2().flex().flex_col().gap_1p5();
            for (index, eq) in item.affixes.iter().enumerate() {
                let Some(affix) = data::get_affix(&eq.affix_id) else {
                    continue;
                };
                let value = eq
                    .custom_value
                    .unwrap_or_else(|| rolled_affix_value_with_stars(affix, eq.roll, stars));
                let shown = match &affix.stat_key {
                    Some(key) => item_tooltip::format_ranged((value, value), key),
                    None => item_tooltip::format_affix_value(affix, value),
                };
                let label = item_tooltip::description_without_value(&affix.description);
                let show_name = !label.to_lowercase().contains(&affix.name.to_lowercase());
                let key = format!("affix-roll-{index}");
                let bounds =
                    gear::affix_roll_bounds(&eq.affix_id, stars).filter(|(lo, hi)| lo < hi);
                if let Some(bounds) = bounds {
                    self.roll_slider(
                        key.clone(),
                        bounds,
                        value.abs().clamp(bounds.0, bounds.1),
                        RollTarget {
                            affix: Some(index),
                            forged: false,
                            stat: String::new(),
                            skill: false,
                            relic_tier: false,
                        },
                        cx,
                    );
                }
                rows = rows.child(
                    row_box(cx)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .child(row_index(index, cx))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .items_baseline()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .font_family(theme::MONO_FONT_FAMILY)
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(accent_hot)
                                                .child(shown),
                                        )
                                        .child(
                                            div().truncate().text_color(text.opacity(0.85)).child(
                                                if show_name {
                                                    format!("{label} ({})", affix.name)
                                                } else {
                                                    label
                                                },
                                            ),
                                        )
                                        .child(chip(
                                            trf(
                                                "item.numbered_tier_short",
                                                &[("arg0", (affix.tier).to_string())],
                                            ),
                                            accent_hot.opacity(0.75),
                                            accent_deep.opacity(0.4),
                                        ))
                                        .when(eq.custom_value.is_some(), |v| {
                                            v.child(chip(
                                                tr("gear.custom"),
                                                accent_hot,
                                                accent_hot.opacity(0.6),
                                            ))
                                        }),
                                )
                                .child(
                                    icon_button(
                                        SharedString::from(format!("affix-remove-{index}")),
                                        "×",
                                        cx,
                                    )
                                    .cursor_tooltip(tr("gear.remove_affix"))
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.edit(cx, |item| {
                                                item.affixes.remove(index);
                                            });
                                            this.sliders
                                                .retain(|key, _| !key.starts_with("affix-roll-"));
                                        },
                                    )),
                                ),
                        )
                        .when_some(bounds, |view, (lo, hi)| {
                            view.child(
                                div()
                                    .mt_1()
                                    .pl(units(26.))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(self.roll_control(
                                        &key,
                                        &trf(
                                            "gear.named_roll",
                                            &[("arg0", (affix.name).to_string())],
                                        ),
                                        window,
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .flex_none()
                                            .font_family(theme::MONO_FONT_FAMILY)
                                            .text_size(units(9.))
                                            .text_color(accent_hot.opacity(0.6))
                                            .child(format!("{lo}–{hi}")),
                                    ),
                            )
                        }),
                );
            }
            rows
        };
        self.card(
            Section::new(
                "affixes",
                if base.random_affix_group_id.as_deref() == Some("random_unholy") {
                    tr("gear.unholy_affixes")
                } else {
                    tr("gear.affixes")
                },
                Tone::Default,
            )
            .default_open(!item.affixes.is_empty())
            .right(right)
            .body(body),
            cx,
        )
    }

    fn forged_section(
        &mut self,
        item: &EquippedItem,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = cx.global::<TooltipTheme>();
        let (negative, text, accent_hot, accent_deep) =
            (p.negative, p.text, p.accent_hot, p.accent_deep);
        let right =
            if item.forged_mods.is_empty() {
                header_action("forge-add", tr("gear.add"), true, cx)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.choose_picker(Picker::Forge, window, cx)
                    }))
                    .into_any_element()
            } else {
                mono_summary(
                    trf(
                        "gear.forged_count",
                        &[("arg0", (item.forged_mods.len()).to_string())],
                    ),
                    negative.opacity(0.9),
                )
                .into_any_element()
            };
        let body = if item.forged_mods.is_empty() {
            empty_note("no-forge", tr("gear.no_forged"), cx)
        } else {
            let mut rows = div().p_2().flex().flex_col().gap_1p5();
            for (index, eq) in item.forged_mods.iter().enumerate() {
                let (name, tier) = data::get_crystal_mod(&eq.affix_id)
                    .map(|m| (m.name.clone(), Some(m.tier)))
                    .unwrap_or_else(|| (eq.affix_id.clone(), None));
                let definition = data::get_crystal_mod(&eq.affix_id);
                let range = definition
                    .filter(|a| a.value_min.is_some() && a.value_max.is_some())
                    .map(hsplanner_engine::calc::affix::rolled_affix_range);
                let bounds = range
                    .map(|(a, b)| (a.abs().min(b.abs()), a.abs().max(b.abs())))
                    .filter(|(lo, hi)| lo < hi);
                let key = format!("forge-roll-{index}");
                if let Some(bounds) = bounds {
                    self.roll_slider(
                        key.clone(),
                        bounds,
                        eq.custom_value.map(f64::abs).unwrap_or(bounds.1),
                        RollTarget {
                            affix: Some(index),
                            forged: true,
                            stat: String::new(),
                            skill: false,
                            relic_tier: false,
                        },
                        cx,
                    );
                }
                let shown = definition.zip(range).map(|(a, range)| {
                    let value = eq.custom_value.map(|v| (v, v)).unwrap_or(range);
                    a.stat_key.as_ref().map_or_else(
                        || format!("{}–{}", value.0, value.1),
                        |key| item_tooltip::format_ranged(value, key),
                    )
                });
                rows = rows.child(
                    row_box(cx)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .child(row_index(index, cx))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .items_baseline()
                                        .gap_1p5()
                                        .children(shown.map(|shown| {
                                            div()
                                                .font_family(theme::MONO_FONT_FAMILY)
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(accent_hot)
                                                .child(shown)
                                        }))
                                        .child(
                                            div()
                                                .truncate()
                                                .text_color(text.opacity(0.85))
                                                .child(name.clone()),
                                        )
                                        .children(tier.map(|tier| {
                                            chip(
                                                trf(
                                                    "item.tier_short",
                                                    &[("tier", (tier).to_string())],
                                                ),
                                                accent_hot.opacity(0.75),
                                                accent_deep.opacity(0.4),
                                            )
                                        })),
                                )
                                .child(
                                    icon_button(
                                        SharedString::from(format!("forge-remove-{index}")),
                                        "×",
                                        cx,
                                    )
                                    .cursor_tooltip(tr("gear.remove_forged"))
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.edit(cx, |item| {
                                                let _ = gear::set_forge(item, None);
                                            });
                                            this.sliders
                                                .retain(|key, _| !key.starts_with("forge-roll-"));
                                        },
                                    )),
                                ),
                        )
                        .when_some(bounds, |view, (lo, hi)| {
                            view.child(
                                div()
                                    .mt_1()
                                    .pl(units(26.))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(self.roll_control(
                                        &key,
                                        &trf("gear.forged_roll", &[("name", (name).to_string())]),
                                        window,
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .flex_none()
                                            .font_family(theme::MONO_FONT_FAMILY)
                                            .text_size(units(9.))
                                            .text_color(accent_hot.opacity(0.6))
                                            .child(format!("{lo}–{hi}")),
                                    ),
                            )
                        }),
                );
            }
            rows
        };
        self.card(
            Section::new("forged", tr("gear.forged_crystal"), Tone::Satanic)
                .default_open(!item.forged_mods.is_empty())
                .right(right)
                .body(body),
            cx,
        )
    }

    fn augment_section(&self, item: &EquippedItem, cx: &Context<Self>) -> Stateful<Div> {
        let p = cx.global::<TooltipTheme>();
        let augment = item
            .augment
            .as_ref()
            .and_then(|a| data::get_augment(&a.id).map(|augment| (augment, a.level)));
        let right = match augment {
            Some((augment, level)) => div()
                .flex()
                .items_center()
                .gap_2()
                .child(mono_summary(
                    trf(
                        "gear.augment_level",
                        &[
                            ("arg0", (augment.name.to_uppercase()).to_string()),
                            ("level", (level).to_string()),
                        ],
                    ),
                    p.angelic.opacity(0.9),
                ))
                .child(
                    header_action("augment-remove", tr("gear.remove"), true, cx).on_click(
                        cx.listener(|this, _, _, cx| this.edit(cx, |item| item.augment = None)),
                    ),
                )
                .into_any_element(),
            None => header_action("augment-add", tr("gear.add"), false, cx)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.choose_picker(Picker::Augment, window, cx)
                }))
                .into_any_element(),
        };
        let body = match augment {
            None => empty_note("no-augment", tr("gear.no_augment"), cx),
            Some((augment, level)) => self.augment_body(augment, level, cx),
        };
        self.card(
            Section::new("augment", tr("gear.angelic_augment"), Tone::Angelic)
                .default_open(augment.is_some())
                .right(right)
                .body(body),
            cx,
        )
    }

    fn augment_body(
        &self,
        augment: &hsplanner_engine::calc::types::AngelicAugment,
        level: u32,
        cx: &Context<Self>,
    ) -> Div {
        let p = cx.global::<TooltipTheme>();
        let index = (level.max(1) as usize - 1).min(augment.levels.len().saturating_sub(1));
        let mut stats: Vec<(&String, &f64)> = augment
            .levels
            .get(index)
            .map(|tier| tier.stats.iter().filter(|(_, v)| **v != 0.).collect())
            .unwrap_or_default();
        stats.sort_by(|a, b| a.0.cmp(b.0));
        let boxed = || {
            div()
                .rounded_sm()
                .border_1()
                .border_color(p.angelic.opacity(0.15))
                .bg(p.background.opacity(0.4))
        };
        let label = |id: &'static str, text: &str| {
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(units(9.))
                .text_color(p.angelic.opacity(0.7))
                .child(TooltipText::new(id, text.to_owned(), 0.18))
        };
        div()
            .p_3()
            .flex()
            .flex_col()
            .gap_2p5()
            .child(
                boxed()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .px_2p5()
                    .py_1p5()
                    .child(label("augment-level-label", tr("gear.level")))
                    .child(
                        icon_button("augment-minus", "−", cx)
                            .disabled(level <= 1)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.edit(cx, |item| {
                                    if let Some(a) = &mut item.augment {
                                        a.level = a.level.saturating_sub(1).max(1);
                                    }
                                })
                            })),
                    )
                    .child(
                        div()
                            .w_7()
                            .text_center()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(units(12.))
                            .text_color(p.angelic)
                            .child(level.to_string()),
                    )
                    .child(
                        icon_button("augment-plus", "+", cx)
                            .disabled(level >= 7)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.edit(cx, |item| {
                                    if let Some(a) = &mut item.augment {
                                        a.level = (a.level + 1).min(7);
                                    }
                                })
                            })),
                    )
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(units(10.))
                            .text_color(p.faint)
                            .child("/ 7"),
                    ),
            )
            .when(!stats.is_empty(), |v| {
                v.child(
                    boxed()
                        .p_2()
                        .child(
                            div()
                                .mb_1()
                                .child(label("augment-stats-label", tr("gear.stats"))),
                        )
                        .children(stats.into_iter().map(|(key, value)| {
                            div()
                                .flex()
                                .justify_between()
                                .text_size(units(11.))
                                .child(div().text_color(p.text.opacity(0.8)).child(stat_name(key)))
                                .child(
                                    div()
                                        .font_family(theme::MONO_FONT_FAMILY)
                                        .text_color(p.angelic)
                                        .child(item_tooltip::format_ranged((*value, *value), key)),
                                )
                        })),
                )
            })
    }

    pub(super) fn comparison(&self, window: &Window, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let baseline = snapshot.inventory.get(&self.slot);
        let summary = div()
            .flex()
            .gap(rems(1. / 13.))
            .rounded_sm()
            .border_1()
            .border_color(p.border)
            .overflow_hidden()
            .bg(p.border)
            .child(compare_identity(
                "current-item",
                tr("gear.currently_equipped"),
                baseline,
                cx,
            ))
            .child(compare_identity(
                "selected-item",
                tr("gear.selected"),
                self.candidate.as_ref(),
                cx,
            ));
        let mut details = div().flex().flex_col().gap_4().px_5().pt_3().pb_5();
        let same = same_item(baseline, self.candidate.as_ref());
        let equipped_ids = item_tooltip::equipped_ids(&snapshot.inventory);
        // Both cards repeat the same TooltipText ids; an id'd wrapper keeps a11y nodes distinct.
        let card = |id: &'static str, item: Option<&EquippedItem>| {
            div()
                .id(id)
                .min_w_0()
                .child(item_tooltip::item_card(item, &equipped_ids, window, cx))
        };
        let item_cards = if same {
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .child(compare_heading(
                    "compare-item-heading",
                    tr("gear.item"),
                    Some(tr("gear.unchanged")),
                    cx,
                ))
                .child(card("compare-selected-card", self.candidate.as_ref()))
        } else {
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .child(compare_heading(
                    "compare-items-heading",
                    tr("gear.items"),
                    None,
                    cx,
                ))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .items_start()
                        .gap_3()
                        .child(card("compare-before-card", baseline).flex_1())
                        .child(card("compare-after-card", self.candidate.as_ref()).flex_1()),
                )
        };
        // compare_heading already supplies the reference mb-2. Keep it inside
        // this group so the spacing between sections is not added before a card.
        details = details.child(item_cards);
        if let Some(rows) = &self.differences {
            if rows.is_empty() {
                details = details.child(
                    div()
                        .text_size(rems(11. / 13.))
                        .text_color(p.faint)
                        .child(tr("gear.no_calculated_change")),
                );
            } else {
                let damage = |row: &&PerformanceDiff| {
                    matches!(
                        row.key().split(':').next_back().unwrap_or(row.key()),
                        "hit_dps" | "combined_dps" | "avg_hit"
                    )
                };
                let damage_rows = rows.iter().filter(damage).collect::<Vec<_>>();
                let stat_rows = rows.iter().filter(|row| !damage(row)).collect::<Vec<_>>();
                for (id, title, changes) in [
                    ("damage-changes", tr("gear.active_skill"), damage_rows),
                    ("build-changes", tr("gear.build_stats"), stat_rows),
                ] {
                    if !changes.is_empty() {
                        let count = trf(
                            "gear.changes_count",
                            &[("count", changes.len().to_string())],
                        );
                        details = details.child(
                            div()
                                .id(id)
                                .flex()
                                .flex_col()
                                .child(compare_heading(id, title, Some(&count), cx))
                                .children(changes.into_iter().map(|row| diff_row(row, cx))),
                        );
                    }
                }
            }
        } else {
            details = details.child(
                div()
                    .text_size(rems(11. / 13.))
                    .text_color(p.faint)
                    .child(tr("gear.calculating_changes")),
            );
        }
        let scroll = self.comparison_scroll.clone();
        let weak = cx.entity().downgrade();
        let had_vertical_scroll = self.comparison_has_vertical_scroll;
        let details = div()
            .on_children_prepainted(move |_, _, cx| {
                let has_vertical_scroll = scroll.max_offset().y > px(0.);
                if has_vertical_scroll != had_vertical_scroll {
                    let weak = weak.clone();
                    cx.defer(move |cx| {
                        let _ = weak.update(cx, |view, cx| {
                            if view.comparison_has_vertical_scroll != has_vertical_scroll {
                                view.comparison_has_vertical_scroll = has_vertical_scroll;
                                cx.notify();
                            }
                        });
                    });
                }
            })
            .id("gear-comparison")
            .flex_1()
            .min_h_0()
            .scrollbar_width(rems(if had_vertical_scroll { 10. / 13. } else { 0. }))
            .page_scroll(&self.comparison_scroll)
            .overflow_y_scroll()
            .child(details);
        div()
            .h_full()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(p.shadow.opacity(0.15))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .px_5()
                    .py_4()
                    .border_b_1()
                    .border_color(p.border)
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(p.muted.opacity(0.04), 0.),
                        linear_color_stop(p.shadow.opacity(0.), 1.),
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(editor_eyebrow(
                                        "comparison-heading",
                                        tr("gear.comparison"),
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .text_size(rems(16. / 13.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(p.text.opacity(0.85))
                                            .child(tr("gear.net_change")),
                                    ),
                            )
                            .children(verdict_badge(self.verdict, cx)),
                    )
                    .child(summary),
            )
            .child(details)
    }
}
impl GearView {
    pub(super) fn editor(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        if self.picker_rem != window.rem_size() {
            self.picker_rem = window.rem_size();
            self.picker_list
                .reset(if matches!(self.picker, Picker::Items | Picker::Stash) {
                    self.visible_items.len()
                } else {
                    self.rows.len()
                });
        }
        let (border, muted) = {
            let p = cx.global::<TooltipTheme>();
            (p.border, p.muted)
        };
        if !self.choosing {
            return div()
                .size_full()
                .flex()
                .items_stretch()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .flex()
                        .flex_col()
                        .child(self.item_identity(cx))
                        .child(
                            div()
                                .id("gear-editor")
                                .flex_1()
                                .min_h_0()
                                .w_full()
                                .overflow_y_scroll()
                                .child(self.inspector(window, cx)),
                        ),
                )
                .when(!self.mercenary, |v| {
                    v.child(
                        div()
                            .w(rems(37.5))
                            .flex_none()
                            .h_full()
                            .border_l_1()
                            .border_color(border)
                            .child(self.comparison(window, cx)),
                    )
                });
        }
        if matches!(self.picker, Picker::Items | Picker::Stash) {
            return self.item_picker(cx);
        }
        let modifier = matches!(self.picker, Picker::Affix | Picker::Forge | Picker::Augment);
        let choices = div()
            .id("gear-choices")
            .flex_1()
            .min_h_0()
            .when(self.rows.is_empty(), |view| {
                view.child(
                    div()
                        .p_10()
                        .text_center()
                        .text_size(units(13.))
                        .text_color(muted)
                        .child(tr("gear.no_matches")),
                )
            })
            .when(!self.rows.is_empty(), |view| {
                view.child(
                    list(self.picker_list.clone(), cx.processor(Self::render_choice)).size_full(),
                )
            });
        div().size_full().flex().flex_col().child(
            div()
                .w_full()
                .h_full()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .px_4()
                        .py_3()
                        .border_b_1()
                        .border_color(border)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .when(!modifier, |view| {
                                    view.child(self.picker_button(
                                        "items",
                                        tr("gear.items"),
                                        Picker::Items,
                                        cx,
                                    ))
                                    .when(
                                        !self.is_relic_slot(),
                                        |view| {
                                            view.child(self.picker_button(
                                                "stash",
                                                tr("gear.stash"),
                                                Picker::Stash,
                                                cx,
                                            ))
                                        },
                                    )
                                })
                                .when(modifier, |view| {
                                    let title = match self.picker {
                                        Picker::Forge => tr("gear.pick_satanic_affix"),
                                        Picker::Augment => tr("gear.pick_angelic_augment"),
                                        _ if self
                                            .candidate
                                            .as_ref()
                                            .and_then(|item| data::get_item(&item.base_id))
                                            .is_some_and(|base| {
                                                base.random_affix_group_id.as_deref()
                                                    == Some("random_unholy")
                                            }) =>
                                        {
                                            tr("gear.pick_unholy_affix")
                                        }
                                        _ => tr("gear.add_affix"),
                                    };
                                    view.child(
                                        div()
                                            .text_size(units(17.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(cx.global::<TooltipTheme>().text)
                                            .child(title),
                                    )
                                })
                                .child(div().flex_1())
                                .child(
                                    ghost_button("back-to-configure", tr("gear.back"), cx)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.choosing = false;
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(
                            Input::new(&self.search).planner_style(cx).prefix(
                                gpui_kit::component::Icon::new(
                                    gpui_kit::component::IconName::Search,
                                )
                                .size_3p5()
                                .text_color(cx.global::<TooltipTheme>().faint),
                            ),
                        ),
                )
                .child(choices)
                .when(self.picker == Picker::Affix, |view| {
                    let pool = self
                        .candidate
                        .as_ref()
                        .and_then(|item| data::get_item(&item.base_id))
                        .filter(|base| base.random_affix_group_id.is_none())
                        .and_then(gear::affix_pool_type);
                    view.children(pool.map(|pool| {
                        let label = pool.split_once(':').map_or_else(
                            || pool.to_owned(),
                            |(head, style)| format!("{style} {head}"),
                        );
                        div().px_4().py_2().child(
                            gpui_kit::component::checkbox::Checkbox::new("affixes-outside-pool")
                                .label(trf("gear.all_affixes", &[("label", (label).to_string())]))
                                .checked(self.show_all_affixes)
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.show_all_affixes = *checked;
                                    this.refresh_rows(cx);
                                    cx.notify();
                                })),
                        )
                    }))
                })
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .gap_2()
                        .items_center()
                        .px_4()
                        .py_2()
                        .border_t_1()
                        .border_color(border)
                        .text_size(units(11.))
                        .text_color(muted)
                        .child(trf(
                            "gear.results_count",
                            &[("arg0", (self.rows.len()).to_string())],
                        )),
                ),
        )
    }
}

impl GearView {
    fn render_choice(
        &mut self,
        index: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(row) = self.rows.get(index) else {
            return div().into_any_element();
        };
        let header =
            !row.group.is_empty() && (index == 0 || self.rows[index - 1].group != row.group);
        let current = self.current_pick();
        let p = cx.global::<TooltipTheme>();
        div()
            .w_full()
            .when(header, |view| {
                view.child(
                    div()
                        .px_4()
                        .py_1()
                        .border_b_1()
                        .border_color(p.accent_deep.opacity(0.3))
                        .bg(p.panel_secondary)
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(10.))
                        .text_color(p.accent_hot.opacity(0.7))
                        .child(
                            match row.group {
                                "Prefixes" => tr("gear.prefixes"),
                                "Suffixes" => tr("gear.suffixes"),
                                group => group,
                            }
                            .to_uppercase(),
                        ),
                )
            })
            .child(self.choice_row(row, current.as_deref() == Some(row.id.as_str()), cx))
            .into_any_element()
    }

    /// One row of the generic picker (sockets, runewords, affixes, forge, augment, skills, class).
    fn choice_row(&self, row: &Row, selected: bool, cx: &Context<Self>) -> Stateful<Div> {
        let p = cx.global::<TooltipTheme>();
        let (accent_hot, accent, border, muted, text) =
            (p.accent_hot, p.accent, p.border, p.muted, p.text);
        let id = row.id.clone();
        let description = format!("{}\n{}", row.label, row.detail);
        div()
            .id(SharedString::from(format!("choice-{}", row.id)))
            .cursor_tooltip_view(move |window, cx| {
                gpui_kit::component::tooltip::Tooltip::new(description.clone()).build(window, cx)
            })
            .relative()
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .gap(units(14.))
            .px_4()
            .py_2p5()
            .border_b_1()
            .border_color(border)
            .cursor_pointer()
            .when(selected, |v| v.bg(accent_hot.opacity(0.05)))
            .hover(|v| v.bg(accent_hot.opacity(0.05)))
            .on_click(cx.listener(move |this, _, _, cx| this.pick(&id, cx)))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w_0p5()
                    .bg(accent)
                    .when(!selected, |v| v.invisible()),
            )
            .when(!row.kind.is_empty() || row.icon.is_some(), |view| {
                view.child(
                    div()
                        .size(units(36.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .map(|view| match row.icon.clone() {
                            Some(icon) => view
                                .child(img(icon).size(units(24.)).object_fit(ObjectFit::ScaleDown)),
                            None => view.child(
                                div()
                                    .size(units(20.))
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(row.tone.unwrap_or(accent).opacity(0.6))
                                    .bg(row.tone.unwrap_or(accent).opacity(0.16)),
                            ),
                        }),
                )
            })
            .when(!row.kind.is_empty(), |view| {
                view.child(
                    div()
                        .w(units(56.))
                        .flex_none()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(10.))
                        .text_color(p.faint)
                        .child(match row.kind {
                            "PREFIX" => tr("gear.prefix"),
                            "SUFFIX" => tr("gear.suffix"),
                            "AFFIX" => tr("gear.affix"),
                            "CRYSTAL" => tr("gear.crystal"),
                            "AUGMENT" => tr("gear.augment"),
                            kind => kind,
                        }),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(units(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(row.tone.unwrap_or(text))
                    .child(row.label.clone()),
            )
            .children(row.tier.map(|tier| {
                let tone = if tier >= 4 {
                    theme::rarity_color("relic", cx)
                } else {
                    accent_hot
                };
                div().w(units(42.)).flex_none().child(
                    chip(
                        trf("item.tier_short", &[("tier", (tier).to_string())]),
                        tone,
                        p.accent_deep,
                    )
                    .px_2()
                    .py_0p5()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(11.)),
                )
            }))
            .when(!row.detail.is_empty(), |v| {
                v.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(10.))
                        .text_color(muted.opacity(0.8))
                        .child(row.detail.clone()),
                )
            })
    }
}

fn editor_eyebrow(id: &'static str, text: &'static str, cx: &App) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(rems(10. / 13.))
        .text_color(cx.global::<TooltipTheme>().faint)
        .child(TooltipText::new(id, text.to_uppercase(), 0.18))
}

impl GearView {
    fn item_identity(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let base = self
            .candidate
            .as_ref()
            .and_then(|item| data::get_item(&item.base_id));
        div()
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_3()
            .px_5()
            .py_3()
            .border_b_1()
            .border_color(p.border)
            .bg(p.panel_secondary)
            .child(
                div()
                    .flex_1()
                    .min_w(rems(14.))
                    .child(
                        div()
                            .truncate()
                            .text_size(rems(14. / 13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(
                                base.map_or(p.faint, |base| theme::rarity_color(&base.rarity, cx)),
                            )
                            .child(
                                base.map_or(tr("gear.empty_slot"), |base| base.name.as_str())
                                    .to_owned(),
                            ),
                    )
                    .children(base.map(|base| {
                        div()
                            .text_size(rems(11. / 13.))
                            .text_color(p.muted)
                            .child(rarity_label(&base.rarity))
                    })),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(base.is_some(), |v| {
                        v.child(
                            ghost_button("item-text-edit", tr("gear.text_edit"), cx).on_click(
                                cx.listener(|this, _, window, cx| this.open_text_edit(window, cx)),
                            ),
                        )
                        .when(!self.is_relic_slot(), |v| {
                            v.child(
                                ghost_button("stash-item", tr("gear.save_to_stash"), cx).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        if let Some(item) = this.candidate.clone() {
                                            this.session.update(cx, |session, cx| {
                                                session.edit(|draft| gear::stash(draft, &item));
                                                cx.notify();
                                            });
                                        }
                                    }),
                                ),
                            )
                        })
                        .child(
                            ghost_button("unequip-item", tr("gear.remove"), cx)
                                .text_color(p.negative)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.candidate = None;
                                    this.load_icon();
                                    this.changed(cx);
                                })),
                        )
                    })
                    .child(
                        ghost_button("change-item", tr("gear.change_item"), cx).on_click(
                            cx.listener(|this, _, window, cx| {
                                this.choose_picker(Picker::Items, window, cx)
                            }),
                        ),
                    ),
            )
    }

    fn editor_dirty(&self, cx: &App) -> bool {
        let snapshot = self.session.read(cx).snapshot();
        let inventory = if self.mercenary {
            &snapshot.merc_inventory
        } else {
            &snapshot.inventory
        };
        !same_item(self.candidate.as_ref(), inventory.get(&self.slot))
    }

    pub(super) fn request_editor_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.confirming_close {
            self.keep_editor_open(window, cx);
            return false;
        }
        if !self.editor_dirty(cx) {
            return true;
        }
        self.confirming_close = true;
        self.confirmation_return_focus = window.focused(cx);
        window.focus(&self.confirmation_focus, cx);
        cx.notify();
        false
    }

    fn keep_editor_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.confirming_close = false;
        if let Some(focus) = self.confirmation_return_focus.take() {
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    pub(super) fn editor_footer(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let dirty = self.editor_dirty(cx);
        let label = self
            .candidate
            .as_ref()
            .and_then(|item| data::get_item(&item.base_id))
            .map_or_else(
                || tr("gear.empty_slot").to_string(),
                |base| format!("{} · {}", base.name, rarity_label(&base.rarity)),
            );
        let footer = div()
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .px_5()
            .py_3()
            .border_t_1()
            .border_color(p.border)
            .bg(p.shadow.opacity(0.2))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().size_1p5().flex_none().rounded_full().bg(
                        if self.candidate.is_some() {
                            p.accent
                        } else {
                            p.faint
                        },
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .text_ellipsis()
                            .text_size(rems(12. / 13.))
                            .child(label),
                    )
                    .when(dirty, |v| {
                        v.child(
                            div()
                                .flex_none()
                                .rounded_sm()
                                .border_1()
                                .border_color(p.accent_hot.opacity(0.4))
                                .px_1p5()
                                .py_0p5()
                                .text_size(rems(10. / 13.))
                                .text_color(p.accent_hot)
                                .child(tr("gear.unsaved")),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .gap_2()
                    .child(
                        editor_button("revert-item", tr("gear.revert"), false, cx).on_click(
                            cx.listener(|this, _, window, cx| {
                                this.keep_editor_open(window, cx);
                                this.revert(cx);
                            }),
                        ),
                    )
                    .child(
                        editor_button("cancel-item", tr("gear.cancel"), false, cx).on_click(
                            cx.listener(|this, _, window, cx| {
                                this.confirming_close = false;
                                this.confirmation_return_focus = None;
                                this.editing = false;
                                this.revert(cx);
                                window.close_dialog(cx);
                            }),
                        ),
                    )
                    .child(
                        editor_button("apply-item", tr("gear.apply"), true, cx)
                            .text_color(p.accent_hot)
                            .border_color(p.accent_deep)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.apply(cx);
                                if this.error.is_none() {
                                    this.confirming_close = false;
                                    this.confirmation_return_focus = None;
                                    this.editing = false;
                                    this.revert(cx);
                                    window.close_dialog(cx);
                                }
                            })),
                    ),
            );
        div()
            .flex_none()
            .flex()
            .flex_col()
            .when(self.confirming_close, |view| {
                view.child(
                    div()
                        .id("unsaved-item-confirmation")
                        .track_focus(&self.confirmation_focus)
                        .role(gpui_kit::accesskit::Role::Group)
                        .aria_label(tr("gear.unsaved_item_changes"))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .px_5()
                        .py_3()
                        .border_t_1()
                        .border_color(p.accent_hot.opacity(0.3))
                        .bg(p.accent_hot.opacity(0.08))
                        .text_size(rems(12. / 13.))
                        .text_color(p.accent_hot)
                        .child(tr("gear.unsaved_changes"))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    editor_button(
                                        "keep-editing-item",
                                        tr("gear.keep_editing"),
                                        false,
                                        cx,
                                    )
                                    .on_click(cx.listener(
                                        |this, _, window, cx| this.keep_editor_open(window, cx),
                                    )),
                                )
                                .child(
                                    editor_button("discard-item", tr("gear.discard"), false, cx)
                                        .text_color(p.negative)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.confirming_close = false;
                                            this.confirmation_return_focus = None;
                                            this.editing = false;
                                            this.revert(cx);
                                            window.close_dialog(cx);
                                        })),
                                )
                                .child(
                                    editor_button(
                                        "save-item-before-close",
                                        tr("gear.save"),
                                        true,
                                        cx,
                                    )
                                    .text_color(p.accent_hot)
                                    .border_color(p.accent_deep)
                                    .on_click(cx.listener(
                                        |this, _, window, cx| {
                                            this.apply(cx);
                                            if this.error.is_none() {
                                                this.confirming_close = false;
                                                this.confirmation_return_focus = None;
                                                this.editing = false;
                                                this.revert(cx);
                                                window.close_dialog(cx);
                                            } else {
                                                this.keep_editor_open(window, cx);
                                            }
                                        },
                                    )),
                                ),
                        ),
                )
            })
            .child(footer)
    }
}

pub(crate) fn rarity_label(rarity: &str) -> String {
    match rarity {
        "common" => tr("gear.rarity_common").into(),
        "rare" => tr("gear.rarity_rare").into(),
        "mythic" => tr("gear.rarity_mythic").into(),
        "uncommon" => tr("gear.rarity_superior").into(),
        "satanic_set" => tr("gear.rarity_satanic_set").into(),
        "unholy" => tr("gear.rarity_unholy").into(),
        "relic" => tr("gear.rarity_relic").into(),
        "satanic" => tr("gear.rarity_satanic").into(),
        "heroic" => tr("gear.rarity_heroic").into(),
        "angelic" => tr("gear.rarity_angelic").into(),
        _ => rarity.into(),
    }
}

fn compare_identity(
    id: &'static str,
    title: &'static str,
    item: Option<&EquippedItem>,
    cx: &App,
) -> Stateful<Div> {
    let p = cx.global::<TooltipTheme>();
    let base = item.and_then(|item| data::get_item(&item.base_id));
    div()
        .id(id)
        .flex_1()
        .min_w_0()
        .bg(p.panel_secondary.opacity(0.8))
        .px_3()
        .py_2()
        .child(
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(9. / 13.))
                .text_color(p.muted)
                .child(TooltipText::new(
                    "identity-label",
                    title.to_uppercase(),
                    0.16,
                )),
        )
        .child(
            div()
                .mt_1()
                .text_size(rems(1.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(base.map_or(p.faint, |base| theme::rarity_color(&base.rarity, cx)))
                .child(
                    base.map_or(tr("gear.empty_slot"), |base| base.name.as_str())
                        .to_owned(),
                ),
        )
}

fn compare_heading(id: &'static str, title: &str, trailing: Option<&str>, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .mb_2()
        .flex()
        .items_center()
        .gap_2p5()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.muted)
        .child(TooltipText::new(id, title.to_uppercase(), 0.18))
        .child(div().flex_1().h_px().bg(p.border))
        .children(trailing.map(|trailing| {
            div()
                .font_weight(FontWeight::NORMAL)
                .text_color(p.faint)
                .child(TooltipText::new(
                    SharedString::from(format!("{id}-trailing")),
                    trailing.to_uppercase(),
                    0.14,
                ))
        }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Verdict {
    Upgrade,
    Downgrade,
    Sidegrade,
}

/// The reference's Hit DPS priority, 2% tolerance and count-based stat fallback.
pub(super) fn comparison_verdict(
    before: &hsplanner_engine::calc::build::BuildPerformance,
    after: &hsplanner_engine::calc::build::BuildPerformance,
) -> Verdict {
    if let (Some(bmin), Some(bmax), Some(amin), Some(amax)) = (
        before.hit_dps_min,
        before.hit_dps_max,
        after.hit_dps_min,
        after.hit_dps_max,
    ) {
        let (b, a) = ((bmin + bmax) / 2., (amin + amax) / 2.);
        if b > 0. {
            return if (a - b) / b > 0.02 {
                Verdict::Upgrade
            } else if (a - b) / b < -0.02 {
                Verdict::Downgrade
            } else {
                Verdict::Sidegrade
            };
        }
        if a > b {
            return Verdict::Upgrade;
        }
        if a < b {
            return Verdict::Downgrade;
        }
    }
    let mut balance = 0i32;
    let mut count = |b: Option<&(f64, f64)>, a: Option<&(f64, f64)>| {
        let b = b.map_or(0., |(lo, hi)| (lo + hi) / 2.);
        let a = a.map_or(0., |(lo, hi)| (lo + hi) / 2.);
        if b.abs() < 0.001 && a.abs() < 0.001 {
            return;
        }
        if b.abs() < 0.001 {
            balance += 1;
        } else if a.abs() < 0.001 {
            balance -= 1;
        } else if (a - b).abs() >= 0.001 {
            balance += if a > b { 1 } else { -1 };
        }
    };
    for key in [
        "defense",
        "enhanced_defense",
        "all_skills",
        "enhanced_damage",
        "life",
        "mana",
        "crit_chance",
        "crit_damage",
        "fire_resist",
        "cold_resist",
        "lightning_resist",
        "poison_resist",
        "magic_find",
        "gold_find",
    ] {
        count(before.stats.get(key), after.stats.get(key));
    }
    for attr in &data::game_config().attributes {
        count(
            before.attributes.get(&attr.key),
            after.attributes.get(&attr.key),
        );
    }
    if balance >= 2 {
        Verdict::Upgrade
    } else if balance <= -2 {
        Verdict::Downgrade
    } else {
        Verdict::Sidegrade
    }
}

fn verdict_badge(verdict: Option<Verdict>, cx: &App) -> Option<Div> {
    let p = cx.global::<TooltipTheme>();
    let (label, arrow, color, glow) = match verdict? {
        Verdict::Upgrade => (tr("gear.upgrade"), "▲", p.positive, true),
        Verdict::Downgrade => (tr("gear.downgrade"), "▼", p.negative, true),
        Verdict::Sidegrade => (tr("gear.sidegrade"), "≈", p.muted, false),
    };
    Some(
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1p5()
            .rounded_sm()
            .border_1()
            .border_color(if glow { color } else { p.border_strong })
            .bg(if glow {
                color.opacity(0.08)
            } else {
                p.panel_secondary.opacity(0.6)
            })
            .when(glow, |v| {
                v.shadow(vec![BoxShadow {
                    color: color.opacity(0.12),
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(18.),
                    spread_radius: px(0.),
                    inset: false,
                }])
            })
            .font_family(theme::MONO_FONT_FAMILY)
            .text_size(units(11.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(color)
            .child(
                div()
                    .text_size(units(14.))
                    .line_height(relative(1.))
                    .child(arrow),
            )
            .child(TooltipText::new(
                "verdict-label",
                label.to_uppercase(),
                0.22,
            )),
    )
}

fn diff_row(row: &PerformanceDiff, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let color = if row.delta() > 0. {
        p.positive
    } else {
        p.negative
    };
    let format = |value: (f64, f64)| crate::build_panel::format_range(value, row.is_percent());
    let sign = if row.delta() >= 0. { "+" } else { "" };
    let delta = match row.delta_pct() {
        Some(pct) => format!("{sign}{pct:.1}%"),
        None => format!("{sign}{}", format((row.delta(), row.delta()))),
    };
    div()
        .flex()
        .items_center()
        .gap_2p5()
        .px_1()
        .py_1p5()
        .border_b_1()
        .border_dashed()
        .border_color(p.border)
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(11.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(theme::FONT_FAMILY)
                .text_size(units(12.))
                .text_color(p.text.opacity(0.85))
                .child(crate::stat_labels::performance_name(row)),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_right()
                .text_color(p.faint)
                .child(format(row.before())),
        )
        .child(div().text_color(p.faint).child("→"))
        .child(
            div()
                .flex_shrink_0()
                .text_right()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(format(row.after())),
        )
        .child(
            div()
                .flex_shrink_0()
                .min_w(units(62.))
                .px_1p5()
                .py_0p5()
                .rounded_sm()
                .border_1()
                .border_color(color.opacity(0.35))
                .bg(color.opacity(0.08))
                .text_right()
                .text_size(units(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(delta),
        )
}

#[derive(Clone)]
struct RollEntry {
    key: String,
    stat: String,
    label: String,
    format_key: String,
    bounds: (f64, f64),
    pinned: Option<f64>,
    is_skill: bool,
}

// Ranged implicits and skill bonuses the user can pin; runeword items get no star scaling.
fn roll_entries(item: &EquippedItem, base: &ItemBase) -> Vec<RollEntry> {
    let stars = data::can_star_forge(&base.slot, &base.rarity)
        .then_some(item.stars)
        .flatten();
    let implicit_stars = if item_tooltip::runeword_for(base, Some(item)).is_some() {
        None
    } else {
        stars
    };
    let mut entries = Vec::new();
    if let Some(implicit) = &base.implicit {
        for key in item_tooltip::implicit_keys(base) {
            let (min, max) = implicit[key].as_ranged();
            if key == "random_skill_element"
                || min == max
                || item.implicit_overrides.get(key) == Some(&0.)
            {
                continue;
            }
            entries.push(RollEntry {
                key: format!("implicit:{key}"),
                stat: key.clone(),
                label: stat_name(key),
                format_key: key.clone(),
                bounds: apply_stars_to_ranged_value((min, max), key, implicit_stars),
                pinned: item.implicit_overrides.get(key).copied(),
                is_skill: false,
            });
        }
    }
    if let Some(bonuses) = &base.skill_bonuses {
        for name in item_tooltip::skill_bonus_keys(base) {
            let (min, max) = bonuses[name].as_ranged();
            if min == max || item.skill_bonus_overrides.get(name) == Some(&0.) {
                continue;
            }
            entries.push(RollEntry {
                key: format!("skill:{name}"),
                stat: name.clone(),
                label: trf("gear.skill_bonus", &[("name", (name).to_string())]),
                format_key: String::new(),
                bounds: apply_stars_to_ranged_value((min, max), "item_granted_skill_rank", stars),
                pinned: item.skill_bonus_overrides.get(name).copied(),
                is_skill: true,
            });
        }
    }
    entries
}

fn socketable_name(id: &str) -> String {
    data::get_gem(id)
        .map(|g| g.name.clone())
        .or_else(|| data::get_rune(id).map(|r| r.name.clone()))
        .unwrap_or_else(|| id.to_owned())
}

fn compatible_runeword_count(base: &ItemBase, max_sockets: u32) -> usize {
    if base.rarity != "common" {
        return 0;
    }
    data::runewords()
        .iter()
        .filter(|runeword| {
            runeword.allowed_base_types.contains(&base.base_type)
                && runeword.runes.len() <= max_sockets as usize
        })
        .count()
}

pub(super) fn socketable_tier(id: &str) -> Option<u32> {
    data::get_rune(id).map(|r| r.tier)
}

// Compare serialized field values instead of map iteration order. The reference
// draft also compares item contents, including socket order and optional rolls.
pub(super) fn same_item(a: Option<&EquippedItem>, b: Option<&EquippedItem>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => serde_json::to_value(a)
            .ok()
            .zip(serde_json::to_value(b).ok())
            .is_some_and(|(a, b)| a == b),
        _ => false,
    }
}

fn editor_button(id: &'static str, label: &'static str, primary: bool, cx: &App) -> Button {
    let tone = if primary {
        ButtonTone::Primary
    } else {
        ButtonTone::Neutral
    };
    modal_button(id, label, tone, cx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn runeword_section_requires_an_eligible_base_and_socket_capacity() {
        let angelic = data::get_item("axe_angelic_st_rexis_sundering_axe").unwrap();
        assert_eq!(compatible_runeword_count(angelic, 6), 0);
        let common = data::get_item("base_melee_hand_axe").unwrap();
        assert_eq!(compatible_runeword_count(common, 0), 0);
        assert!(compatible_runeword_count(common, 6) > 0);
    }

    #[::core::prelude::v1::test]
    fn item_roll_controls_follow_the_same_authored_order_as_the_tooltip() {
        let base = data::get_item("axe_angelic_st_rexis_sundering_axe").unwrap();
        let item = EquippedItem {
            base_id: base.id.clone(),
            ..Default::default()
        };
        let actual: Vec<_> = roll_entries(&item, base)
            .into_iter()
            .map(|entry| entry.stat)
            .collect();
        let expected: Vec<_> = item_tooltip::implicit_keys(base)
            .into_iter()
            .filter(|key| {
                let (min, max) = base.implicit.as_ref().unwrap()[*key].as_ranged();
                min != max
            })
            .cloned()
            .collect();
        assert!(!actual.is_empty());
        assert_eq!(actual, expected);
        assert_eq!(actual[0], "enhanced_damage");
    }

    #[::core::prelude::v1::test]
    fn deleted_authored_mod_has_no_roll_control() {
        let base = data::get_item("axe_angelic_st_rexis_sundering_axe").unwrap();
        let mut item = EquippedItem {
            base_id: base.id.clone(),
            ..Default::default()
        };
        assert!(
            roll_entries(&item, base)
                .iter()
                .any(|entry| entry.stat == "enhanced_damage")
        );
        item.implicit_overrides.insert("enhanced_damage".into(), 0.);
        assert!(
            !roll_entries(&item, base)
                .iter()
                .any(|entry| entry.stat == "enhanced_damage")
        );
    }

    #[::core::prelude::v1::test]
    fn verdict_follows_hit_dps_tolerance_even_when_other_damage_moves_oppositely() {
        use hsplanner_engine::calc::build::BuildPerformance;
        let mut before = BuildPerformance::default();
        before.hit_dps_min = Some(100.);
        before.hit_dps_max = Some(100.);
        let mut after = before.clone();
        after.hit_dps_min = Some(101.);
        after.hit_dps_max = Some(101.);
        after.combined_dps_min = Some(10000.);
        after.combined_dps_max = Some(10000.);
        assert_eq!(comparison_verdict(&before, &after), Verdict::Sidegrade);
        after.hit_dps_min = Some(97.);
        after.hit_dps_max = Some(97.);
        assert_eq!(comparison_verdict(&before, &after), Verdict::Downgrade);
    }

    #[::core::prelude::v1::test]
    fn verdict_without_hit_dps_counts_reference_stats_instead_of_damage_units() {
        use hsplanner_engine::calc::build::BuildPerformance;
        let before = BuildPerformance::default();
        let mut after = before.clone();
        after.stats.insert("life".into(), (50., 50.));
        assert_eq!(comparison_verdict(&before, &after), Verdict::Sidegrade);
        after.stats.insert("mana".into(), (10., 10.));
        assert_eq!(comparison_verdict(&before, &after), Verdict::Upgrade);
        assert_eq!(comparison_verdict(&after, &before), Verdict::Downgrade);
    }

    #[core::prelude::v1::test]
    fn draft_comparison_detects_removal_socket_order_and_restored_contents() {
        let baseline = EquippedItem {
            base_id: "test-item".into(),
            socket_count: 2,
            socketed: vec![Some("rune-one".into()), Some("rune-two".into())],
            ..Default::default()
        };
        let mut draft = baseline.clone();
        assert!(same_item(Some(&baseline), Some(&draft)));
        draft.socketed.swap(0, 1);
        assert!(!same_item(Some(&baseline), Some(&draft)));
        assert!(!same_item(Some(&baseline), None));
        draft = baseline.clone();
        assert!(same_item(Some(&baseline), Some(&draft)));
        assert!(same_item(None, None));
    }

    #[core::prelude::v1::test]
    fn draft_comparison_ignores_hash_map_insertion_order_but_detects_roll_edits() {
        let mut baseline = EquippedItem::default();
        baseline.implicit_overrides.insert("armor".into(), 10.);
        baseline.implicit_overrides.insert("life".into(), 20.);
        let mut draft = EquippedItem::default();
        draft.implicit_overrides.insert("life".into(), 20.);
        draft.implicit_overrides.insert("armor".into(), 10.);
        assert!(same_item(Some(&baseline), Some(&draft)));
        draft.implicit_overrides.insert("armor".into(), 11.);
        assert!(!same_item(Some(&baseline), Some(&draft)));
    }
}
