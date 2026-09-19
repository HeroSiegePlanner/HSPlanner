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
        use gpui_kit::component::text::TextView;
        use hsplanner_build::loadout::LoadoutKind;
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
        let loadouts = div().flex().flex_col().gap_3().children(
            [
                (LoadoutKind::Incarnation, "loadouts.incarnation"),
                (LoadoutKind::Ether, "loadouts.ether"),
                (LoadoutKind::Gear, "loadouts.gear"),
                (LoadoutKind::Skills, "loadouts.skills"),
            ]
            .into_iter()
            .map(|(kind, key)| {
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(div().text_sm().text_color(p.muted).child(tr(key)))
                            .child(
                                div()
                                    .id(key)
                                    .text_sm()
                                    .text_color(p.text)
                                    .truncate()
                                    .cursor_tooltip(build.loadouts.active_name(kind).to_owned())
                                    .child(build.loadouts.active_name(kind).to_owned()),
                            ),
                    )
                    .child(div().flex_none().text_xs().text_color(p.faint).child(trf(
                        "library.loadout_count",
                        &[("count", build.loadouts.count(kind).to_string())],
                    )))
            }),
        );
        let build_id = build.id.clone();
        view = view.child(
            preview_section("library.loadouts", None, cx)
                .child(loadouts)
                .child(
                    div()
                        .mt_3()
                        .text_sm()
                        .text_color(p.muted)
                        .child(tr("library.loadouts_help")),
                )
                .child(
                    Button::new("manage-preview-loadouts")
                        .planner_style(cx)
                        .mt_3()
                        .label(tr("library.manage_loadouts"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open(build_id.clone(), cx);
                        })),
                ),
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
}
