use super::*;
use hsplanner_ui::tooltip::CursorTooltipExt;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

pub(super) fn portrait(class: Option<&str>, large: bool, cx: &App) -> Div {
    static ICONS: LazyLock<HashMap<&str, Arc<Image>>> = LazyLock::new(|| {
        [
            (
                "amazon",
                include_bytes!("../../../assets/classes/amazon.webp").as_slice(),
            ),
            (
                "bard",
                include_bytes!("../../../assets/classes/bard.webp").as_slice(),
            ),
            (
                "butcher",
                include_bytes!("../../../assets/classes/butcher.webp").as_slice(),
            ),
            (
                "demon_slayer",
                include_bytes!("../../../assets/classes/demon_slayer.webp").as_slice(),
            ),
            (
                "demonspawn",
                include_bytes!("../../../assets/classes/demonspawn.webp").as_slice(),
            ),
            (
                "exo",
                include_bytes!("../../../assets/classes/exo.webp").as_slice(),
            ),
            (
                "illusionist",
                include_bytes!("../../../assets/classes/illusionist.webp").as_slice(),
            ),
            (
                "jotunn",
                include_bytes!("../../../assets/classes/jotunn.webp").as_slice(),
            ),
            (
                "marauder",
                include_bytes!("../../../assets/classes/marauder.webp").as_slice(),
            ),
            (
                "marksman",
                include_bytes!("../../../assets/classes/marksman.webp").as_slice(),
            ),
            (
                "necromancer",
                include_bytes!("../../../assets/classes/necromancer.webp").as_slice(),
            ),
            (
                "nomad",
                include_bytes!("../../../assets/classes/nomad.webp").as_slice(),
            ),
            (
                "paladin",
                include_bytes!("../../../assets/classes/paladin.webp").as_slice(),
            ),
            (
                "pirate",
                include_bytes!("../../../assets/classes/pirate.webp").as_slice(),
            ),
            (
                "plague_doctor",
                include_bytes!("../../../assets/classes/plague_doctor.webp").as_slice(),
            ),
            (
                "prophet",
                include_bytes!("../../../assets/classes/prophet.webp").as_slice(),
            ),
            (
                "pyromancer",
                include_bytes!("../../../assets/classes/pyromancer.webp").as_slice(),
            ),
            (
                "redneck",
                include_bytes!("../../../assets/classes/redneck.webp").as_slice(),
            ),
            (
                "samurai",
                include_bytes!("../../../assets/classes/samurai.webp").as_slice(),
            ),
            (
                "shaman",
                include_bytes!("../../../assets/classes/shaman.webp").as_slice(),
            ),
            (
                "shield_lancer",
                include_bytes!("../../../assets/classes/shield_lancer.webp").as_slice(),
            ),
            (
                "stormweaver",
                include_bytes!("../../../assets/classes/stormweaver.webp").as_slice(),
            ),
            (
                "viking",
                include_bytes!("../../../assets/classes/viking.webp").as_slice(),
            ),
            (
                "white_mage",
                include_bytes!("../../../assets/classes/white_mage.webp").as_slice(),
            ),
        ]
        .into_iter()
        .map(|(id, bytes)| {
            (
                id,
                Arc::new(Image::from_bytes(ImageFormat::Webp, bytes.to_vec())),
            )
        })
        .collect()
    });
    let palette = cx.global::<TooltipTheme>();
    div()
        .flex_none()
        .size(rems(if large { 54. / 13. } else { 2.75 }))
        .rounded_sm()
        .border_1()
        .border_color(palette.border_strong)
        .bg(palette.background)
        .flex()
        .items_center()
        .justify_center()
        .when_some(class.and_then(|id| ICONS.get(id)), |view, icon| {
            view.child(
                img(icon.clone())
                    .when(large, |icon| icon.size(rems(40. / 13.)))
                    .when(!large, |icon| icon.size_full())
                    .object_fit(ObjectFit::Contain),
            )
        })
}

pub(super) fn label(text: impl Into<String>, color: Hsla) -> Div {
    let text = text.into();
    div()
        .font_family(hsplanner_ui::theme::MONO_FONT_FAMILY)
        .text_size(rems(10. / 13.))
        .text_color(color)
        .child(hsplanner_ui::tooltip_text::TooltipText::new(
            SharedString::from(format!("label-{text}")),
            text.to_uppercase(),
            0.14,
        ))
}

pub(super) fn navigation(
    id: impl Into<ElementId>,
    title: &str,
    count: usize,
    chosen: bool,
    cx: &App,
) -> Button {
    use gpui_kit::component::button::{ButtonCustomVariant, ButtonVariants};
    let p = cx.global::<TooltipTheme>();
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .foreground(if chosen { p.accent_hot } else { p.muted })
                .hover(p.panel_secondary),
        )
        .h_8()
        .rounded_none()
        .px_2()
        .w_full()
        .border_l_2()
        .border_color(if chosen { p.accent } else { p.panel })
        .when(chosen, |button| {
            button.bg(hsplanner_ui::theme::library_highlight(cx))
        })
        .accessibility_label(format!("{title} · {count}"))
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap_2()
                .font_family(hsplanner_ui::theme::FONT_FAMILY)
                .child(title.to_string())
                .child(
                    div()
                        .ml_auto()
                        .text_size(rems(10. / 13.))
                        .text_color(p.faint)
                        .child(count.to_string()),
                ),
        )
}

pub(super) fn toolbar_button(
    id: &'static str,
    title: &'static str,
    icon: &'static str,
    cx: &App,
) -> Button {
    hsplanner_ui::controls::planner_button(id, hsplanner_ui::controls::ButtonTone::Neutral, cx)
        .small()
        .gap_2()
        .accessibility_label(title)
        .child(action_icon(icon))
        .child(title)
}

pub(super) fn action_icon(name: &str) -> impl IntoElement {
    static ICONS: LazyLock<HashMap<&str, Arc<Image>>> = LazyLock::new(|| {
        [
            ("star", include_bytes!("../assets/star.svg").as_slice()),
            (
                "star-filled",
                include_bytes!("../assets/star-filled.svg").as_slice(),
            ),
            ("import", include_bytes!("../assets/import.svg").as_slice()),
            ("copy", include_bytes!("../assets/copy.svg").as_slice()),
            ("rename", include_bytes!("../assets/rename.svg").as_slice()),
            ("delete", include_bytes!("../assets/delete.svg").as_slice()),
            (
                "newfolder",
                include_bytes!("../assets/newfolder.svg").as_slice(),
            ),
            ("search", include_bytes!("../assets/search.svg").as_slice()),
        ]
        .into_iter()
        .map(|(name, bytes)| {
            (
                name,
                Arc::new(Image::from_bytes(ImageFormat::Svg, bytes.to_vec())),
            )
        })
        .collect()
    });
    img(ICONS[name].clone()).size(rems(12. / 13.)).flex_none()
}

fn preview_section(key: &str, count: Option<usize>, cx: &App) -> Stateful<Div> {
    let title = tr(key);
    let p = cx.global::<TooltipTheme>();
    div()
        .id(SharedString::from(format!("preview-section-{key}")))
        .py_3p5()
        .border_b_1()
        .border_color(p.border)
        .child(
            div()
                .mb_2()
                .flex()
                .justify_between()
                .items_center()
                .child(label(title, p.faint))
                .children(count.map(|count| label(format!("· {count}"), p.accent_deep))),
        )
}

fn preview_ehp(result: &hsplanner_engine::calc::defense::EhpResult) -> Vec<(String, Option<f64>)> {
    if result.entries.is_empty() {
        return vec![];
    }
    let physical = result
        .entries
        .iter()
        .find(|e| e.damage_type == "physical")
        .and_then(|e| e.ehp);
    let elements = result
        .entries
        .iter()
        .filter(|e| e.damage_type != "physical")
        .collect::<Vec<_>>();
    let same = |a: Option<f64>, b: Option<f64>| a.map(f64::round) == b.map(f64::round);
    if let Some(first) = elements.first()
        && elements.iter().all(|e| same(e.ehp, first.ehp))
    {
        return if same(physical, first.ehp) {
            vec![(tr("library.ehp").into(), physical)]
        } else {
            vec![
                (tr("library.physical_ehp").into(), physical),
                (tr("library.elemental_ehp").into(), first.ehp),
            ]
        };
    }
    result
        .entries
        .iter()
        .map(|e| {
            (
                trf(
                    "library.damage_ehp",
                    &[("damage", damage_name(&e.damage_type).to_owned())],
                ),
                e.ehp,
            )
        })
        .collect()
}

fn damage_name(kind: &str) -> &str {
    match kind {
        "physical" => tr("library.damage.physical"),
        "fire" => tr("library.damage.fire"),
        "cold" => tr("library.damage.cold"),
        "lightning" => tr("library.damage.lightning"),
        "poison" => tr("library.damage.poison"),
        "arcane" => tr("library.damage.arcane"),
        "magic" => tr("library.damage.magic"),
        _ => kind,
    }
}

impl LibraryView {
    pub(super) fn preview(&self, build: Option<&SavedBuild>, cx: &Context<Self>) -> Stateful<Div> {
        use gpui_kit::component::{
            button::{ButtonCustomVariant, ButtonVariants},
            text::TextView,
        };
        use hsplanner_ui::{
            numbers::{compact, compact_range},
            theme,
        };
        let p = cx.global::<TooltipTheme>();
        let mut view = div()
            .id(SharedString::from(format!(
                "library-preview-{}",
                build.map_or("empty", |build| build.id.as_str())
            )))
            .px_4()
            .pb_3p5()
            .flex()
            .flex_col()
            .child(
                div()
                    .pt_3()
                    .pb_2()
                    .child(label(tr("library.preview"), p.accent_hot)),
            );
        let Some(build) = build else {
            return view.child(
                div()
                    .py_12()
                    .text_center()
                    .text_color(p.muted)
                    .child(label(tr("library.no_selection"), p.faint))
                    .child(
                        div()
                            .mt_2()
                            .text_size(rems(11. / 13.))
                            .child(tr("library.preview_help")),
                    ),
            );
        };
        let snapshot = self.preview_snapshot.as_ref();
        let nodes = snapshot.map_or(0, |s| s.allocated_tree_nodes.len());
        let class_name = build
            .class_id
            .as_deref()
            .and_then(hsplanner_engine::calc::data::get_class)
            .map(|c| c.name.as_str())
            .unwrap_or(tr("library.unknown"));
        view = view.child(
            div()
                .flex()
                .gap_3p5()
                .items_center()
                .py_2()
                .pb_3p5()
                .border_b_1()
                .border_color(p.border)
                .child(portrait(build.class_id.as_deref(), true, cx))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .child(
                            div()
                                .truncate()
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(rems(15. / 13.))
                                .text_color(p.text)
                                .child(build.name.clone()),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_size(rems(10. / 13.))
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_color(p.faint)
                                .child(StyledText::new(trf(
                                    "library.build_summary",
                                    &[
                                        ("class", class_name.to_owned()),
                                        ("level", snapshot.map_or(1, |s| s.level).to_string()),
                                        ("hero", nodes.to_string()),
                                        ("profiles", build.profiles.len().to_string()),
                                        ("season", build.season.clone()),
                                    ],
                                ))),
                        )
                        .when(!build.tags.is_empty(), |v| {
                            v.child(div().mt_3().flex().flex_wrap().gap_1p5().children(
                                build.tags.iter().map(|tag| {
                                    div()
                                        .px_1p5()
                                        .py(px(1.))
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(p.border)
                                        .bg(p.panel_secondary)
                                        .font_family(theme::MONO_FONT_FAMILY)
                                        .text_size(rems(9. / 13.))
                                        .text_color(p.muted)
                                        .child(tag.to_uppercase())
                                }),
                            ))
                        }),
                ),
        );
        let perf = self.performance.as_ref();
        let scale = &self.session.read(cx).state().settings.number_scale;
        let range = |value| compact_range(value, scale);
        let stat = |key: &str| {
            perf.and_then(|v| v.current.stats.get(key))
                .copied()
                .map(range)
                .unwrap_or_else(|| "—".into())
        };
        let percent = |key: &str, prefix: &str| {
            perf.and_then(|v| v.current.stats.get(key))
                .map(|&(lo, hi)| {
                    let fmt = |n: f64| format!("{prefix}{}%", compact(n, "none"));
                    if (lo - hi).abs() < 0.5 {
                        fmt(lo)
                    } else {
                        format!("{}–{}", fmt(lo), fmt(hi))
                    }
                })
                .unwrap_or_else(|| "—".into())
        };
        let mut stats = vec![
            (tr("library.life").to_string(), stat("life"), p.negative),
            (tr("library.mana").into(), stat("mana"), theme::mana_color()),
            (
                tr("library.crit").into(),
                percent("crit_chance", ""),
                p.text,
            ),
            (
                tr("library.crit_damage").into(),
                percent("crit_damage", "+"),
                p.text,
            ),
        ];
        let resists = ["fire", "cold", "lightning", "poison", "arcane"]
            .iter()
            .map(|kind| {
                perf.map(|v| {
                    compact(
                        v.current
                            .stats
                            .get(&format!("{kind}_resistance"))
                            .map_or(0., |r| r.1),
                        "none",
                    )
                })
                .unwrap_or_else(|| "0".into())
            })
            .collect::<Vec<_>>()
            .join("/");
        stats.push((tr("library.resists").into(), resists, p.text));
        stats.push((
            tr("library.nodes_skills").into(),
            snapshot
                .map(|s| format!("{nodes} · {}", s.skill_ranks.len()))
                .unwrap_or_else(|| "—".into()),
            p.text,
        ));
        stats.push((
            tr("library.ether").into(),
            snapshot
                .map(|s| s.allocated_ether_nodes.len().to_string())
                .unwrap_or_else(|| "—".into()),
            p.text,
        ));
        stats.push((
            tr("library.mercenary").into(),
            snapshot
                .and_then(|s| s.merc_class_id.as_ref())
                .map(|id| {
                    hsplanner_engine::calc::mercenary::data()
                        .classes
                        .iter()
                        .find(|c| &c.id == id)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| id.clone())
                })
                .unwrap_or_else(|| "—".into()),
            p.text,
        ));
        if let Some(perf) = perf {
            stats.extend(
                preview_ehp(&perf.current.ehp)
                    .into_iter()
                    .map(|(name, value)| {
                        (
                            name,
                            value
                                .map(|n| compact(n, "none"))
                                .unwrap_or_else(|| "∞".into()),
                            p.accent_hot,
                        )
                    }),
            );
        }
        let dps = perf
            .and_then(|v| v.current.combined_dps_min.zip(v.current.combined_dps_max))
            .map(range)
            .unwrap_or_else(|| "—".into());
        let grid = div()
            .grid()
            .grid_cols(2)
            .gap(px(1.))
            .rounded_md()
            .border_1()
            .border_color(p.border)
            .bg(p.border)
            .overflow_hidden()
            .children(stats.iter().map(|(name, value, color)| {
                div()
                    .min_w_0()
                    .bg(p.panel)
                    .px_3()
                    .py_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .truncate()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(9.5 / 13.))
                            .text_color(p.faint)
                            .child(name.to_uppercase()),
                    )
                    .child(
                        div()
                            .truncate()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(1.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(*color)
                            .child(value.clone()),
                    )
            }))
            .when(stats.len() % 2 == 1, |v| v.child(div().bg(p.panel)));
        view = view.child(
            div()
                .py_3p5()
                .border_b_1()
                .border_color(p.border)
                .child(
                    div()
                        .mb_3p5()
                        .px_4()
                        .py_3()
                        .rounded_md()
                        .border_1()
                        .border_color(p.border)
                        .bg(theme::library_highlight(cx))
                        .child(label(tr("library.combined_dps"), p.accent_hot.opacity(0.6)))
                        .child(
                            div()
                                .mt_1()
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(rems(23. / 13.))
                                .line_height(relative(1.25))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.accent_hot)
                                .child(dps),
                        ),
                )
                .child(grid),
        );
        if self.preview_task.is_some() {
            view = view.child(div().pt_2().child(label(tr("library.computing"), p.faint)));
        } else if snapshot.is_none() {
            view = view.child(
                div()
                    .pt_2()
                    .child(label(tr("library.unreadable"), p.negative)),
            );
        }
        let mut profiles = div().flex().flex_col().gap(rems(5. / 13.));
        for profile in &build.profiles {
            let (build_id, profile_id) = (build.id.clone(), profile.id.clone());
            let rename_build = build.id.clone();
            let rename_id = profile.id.clone();
            let rename_name = profile.name.clone();
            let duplicate_build = build.id.clone();
            let duplicate_id = profile.id.clone();
            let remove_build = build.id.clone();
            let remove_id = profile.id.clone();
            let active = profile.id == build.active_profile_id;
            let action = |id: &str, icon: &str, tooltip: &str| {
                Button::new(SharedString::from(format!("{id}-{}", profile.id)))
                    .ghost()
                    .planner_style(cx)
                    .border_0()
                    .p_0()
                    .size(rems(24. / 13.))
                    .child(action_icon(icon))
                    .cursor_tooltip(tooltip.to_owned())
                    .accessibility_label(tooltip.to_owned())
            };
            profiles = profiles.child(
                div()
                    .id(SharedString::from(format!(
                        "preview-profile-{}",
                        profile.id
                    )))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2p5()
                    .py(rems(7. / 13.))
                    .rounded_sm()
                    .border_1()
                    .border_color(if active { p.accent_deep } else { p.border })
                    .bg(if active {
                        p.accent_hot.opacity(0.05)
                    } else {
                        p.panel_secondary
                    })
                    .child(
                        Button::new("switch")
                            .custom(ButtonCustomVariant::new(cx).foreground(if active {
                                p.accent_hot
                            } else {
                                p.text
                            }))
                            .planner_style(cx)
                            .border_0()
                            .h_auto()
                            .p_0()
                            .min_w_0()
                            .flex_1()
                            .accessibility_label(if active {
                                tr("library.active_profile")
                            } else {
                                tr("library.switch_profile")
                            })
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .flex()
                                    .gap_2()
                                    .items_center()
                                    .text_size(rems(11.5 / 13.))
                                    .font_family(theme::FONT_FAMILY)
                                    .child(
                                        div()
                                            .text_size(rems(8. / 13.))
                                            .text_color(if active { p.accent_hot } else { p.faint })
                                            .child("◆"),
                                    )
                                    .child(div().min_w_0().truncate().child(profile.name.clone()))
                                    .when(active, |v| {
                                        v.child(
                                            div()
                                                .font_family(theme::MONO_FONT_FAMILY)
                                                .text_size(rems(9. / 13.))
                                                .text_color(p.accent_deep)
                                                .child(tr("library.active_badge")),
                                        )
                                    }),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !active {
                                    this.apply(cx, |s| s.open(&build_id, Some(&profile_id)));
                                }
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_0p5()
                            .child(
                                action("rename", "rename", tr("library.rename_profile")).on_click(
                                    cx.listener(move |this, _, window, cx| {
                                        this.rename_preview_profile(
                                            rename_build.clone(),
                                            rename_id.clone(),
                                            rename_name.clone(),
                                            window,
                                            cx,
                                        )
                                    }),
                                ),
                            )
                            .child(
                                action("duplicate", "copy", tr("library.duplicate_profile"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.apply(cx, |session| {
                                            session.edit_library(|library| {
                                                let build = library.build_mut(&duplicate_build)?;
                                                if build.profiles.len() >= 100 {
                                                    return Err("library.profile_limit".into());
                                                }
                                                let source = build
                                                    .profile(&duplicate_id)
                                                    .ok_or("library.missing_profile")?;
                                                let copied =
                                                    hsplanner_build::library::Profile::new(
                                                        &trf(
                                                            "library.profile_copy_name",
                                                            &[("name", source.name.clone())],
                                                        ),
                                                        &source.snapshot()?,
                                                    )?;
                                                build.profiles.push(copied);
                                                build.updated_at = hsplanner_build::library::now();
                                                Ok(())
                                            })
                                        });
                                    })),
                            )
                            .child(
                                action("remove", "delete", tr("library.remove_profile"))
                                    .disabled(build.profiles.len() <= 1)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.apply(cx, |session| {
                                            if session.draft().build_id.as_ref()
                                                == Some(&remove_build)
                                            {
                                                session.remove_profile(&remove_id)
                                            } else {
                                                session.edit_library(|library| {
                                                    let build = library.build_mut(&remove_build)?;
                                                    if build.profiles.len() <= 1 {
                                                        return Err("library.keep_profile".into());
                                                    }
                                                    build.profiles.retain(|p| p.id != remove_id);
                                                    if build.active_profile_id == remove_id {
                                                        build.active_profile_id =
                                                            build.profiles[0].id.clone();
                                                    }
                                                    build.updated_at =
                                                        hsplanner_build::library::now();
                                                    Ok(())
                                                })
                                            }
                                        });
                                    })),
                            ),
                    ),
            );
        }
        profiles = profiles.child(
            Button::new("add-preview-profile")
                .planner_style(cx)
                .mt(rems(3. / 13.))
                .h_auto()
                .px_2p5()
                .py(rems(7. / 13.))
                .border_dashed()
                .border_color(p.border_strong)
                .text_color(p.faint)
                .text_size(rems(11. / 13.))
                .label(tr("library.add_profile_button"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.edit_dialog(EditKind::AddProfile, window, cx)
                })),
        );
        view = view.child(
            preview_section("library.profiles", Some(build.profiles.len()), cx).child(profiles),
        );
        if let Some(snapshot) = snapshot
            && !snapshot.active_skill_ids.is_empty()
        {
            let count = snapshot.active_skill_ids.len();
            let skills = div().flex().flex_col().gap(rems(5. / 13.)).children(
                snapshot.active_skill_ids.iter().map(|id| {
                    let name = snapshot
                        .class_id
                        .as_deref()
                        .and_then(|class| {
                            hsplanner_engine::calc::data::get_skills_by_class(class)
                                .iter()
                                .find(|s| s.id == *id)
                        })
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| id.clone());
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .px_2p5()
                        .py(rems(7. / 13.))
                        .rounded_sm()
                        .border_1()
                        .border_color(p.border)
                        .bg(p.panel_secondary)
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(rems(12. / 13.))
                        .text_color(p.text)
                        .child(
                            div()
                                .text_size(rems(10. / 13.))
                                .text_color(p.accent)
                                .child("◆"),
                        )
                        .child(div().min_w_0().truncate().child(name))
                }),
            );
            view = view.child(
                preview_section(
                    if count > 1 {
                        "library.main_skills"
                    } else {
                        "library.main_skill"
                    },
                    (count > 1).then_some(count),
                    cx,
                )
                .child(skills),
            );
        }
        let notes = build.notes().markdown;
        if !notes.trim().is_empty() {
            view = view.child(
                preview_section("library.notes", None, cx)
                    .border_b_0()
                    .child(
                        div()
                            .px_2p5()
                            .py_2()
                            .border_1()
                            .rounded_sm()
                            .border_color(p.border)
                            .bg(p.panel_secondary)
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(rems(12. / 13.))
                            .text_color(p.muted)
                            .child(TextView::markdown(
                                SharedString::from(format!("preview-notes-{}", build.id)),
                                notes,
                            )),
                    ),
            );
        }
        view
    }

    fn rename_preview_profile(
        &mut self,
        build_id: String,
        profile_id: String,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use gpui_kit::component::WindowExt;
        let input = cx.new(|cx| {
            let mut input = localized_input("library.profile_name", window, cx);
            input.set_value(name, window, cx);
            input
        });
        let focus = input.read(cx).focus_handle(cx);
        let owner = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let owner = owner.clone();
            let input_for_save = input.clone();
            let build_id = build_id.clone();
            let profile_id = profile_id.clone();
            dialog
                .title(tr("library.rename_profile"))
                .child(Input::new(&input).planner_style(cx))
                .footer(
                    Button::new("save-profile-name")
                        .planner_style(cx)
                        .label(tr("library.save"))
                        .on_click(move |_, window, cx| {
                            let name = input_for_save.read(cx).value().to_string();
                            let _ = owner.update(cx, |this, cx| {
                                if this.apply(cx, |session| {
                                    session.edit_library(|library| {
                                        let build = library.build_mut(&build_id)?;
                                        build
                                            .profiles
                                            .iter_mut()
                                            .find(|p| p.id == profile_id)
                                            .ok_or("library.missing_profile")?
                                            .name = hsplanner_build::library::clean_name(&name)?;
                                        build.updated_at = hsplanner_build::library::now();
                                        Ok(())
                                    })
                                }) {
                                    window.close_dialog(cx);
                                }
                            });
                        }),
                )
        });
        window.focus(&focus, cx);
    }
}
