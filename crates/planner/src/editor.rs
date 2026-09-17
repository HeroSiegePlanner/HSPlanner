use crate::{TreeView, build_panel::format_range};
use gpui_kit::base::Selectable;
use gpui_kit::component::{
    Sizable,
    button::Button,
    checkbox::Checkbox,
    input::{Input, InputEvent, InputState},
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::{BuildSnapshot, session::Session};
use hsplanner_engine::calc::{data, types::SkillKind};
use hsplanner_ui::controls::{PlannerControl, icon_button};
use hsplanner_ui::i18n::{Locale, tr, trf};
use hsplanner_ui::theme::TooltipTheme;

fn condition_name(key: &str) -> &str {
    match key {
        "burning" => tr("tree.condition.burning"),
        "poisoned" => tr("tree.condition.poisoned"),
        "frozenbite" => tr("tree.condition.frozenbite"),
        "stunned" => tr("tree.condition.stunned"),
        "bleeding" => tr("tree.condition.bleeding"),
        "shocked" => tr("tree.condition.shocked"),
        "deep_frozen" => tr("tree.condition.deep_frozen"),
        "shadow_burn" => tr("tree.condition.shadow_burn"),
        "frozen" => tr("tree.condition.frozen"),
        "slow" => tr("tree.condition.slow"),
        "low_life" => tr("tree.condition.low_life"),
        "serrated_chains" => tr("tree.condition.serrated_chains"),
        "lightning_break" => tr("tree.condition.lightning_break"),
        "fire_break" => tr("tree.condition.fire_break"),
        "cold_break" => tr("tree.condition.cold_break"),
        "arcane_break" => tr("tree.condition.arcane_break"),
        "poison_break" => tr("tree.condition.poison_break"),
        "is_boss" => tr("tree.condition.is_boss"),
        "crit_chance_below_40" => tr("tree.condition.crit_chance_below_40"),
        "life_below_40" => tr("tree.condition.life_below_40"),
        "fire" => tr("tree.condition.fire"),
        "cold" => tr("tree.condition.cold"),
        "lightning" => tr("tree.condition.lightning"),
        "poison" => tr("tree.condition.poison"),
        "arcane" => tr("tree.condition.arcane"),
        _ => key,
    }
}

#[derive(Clone, Copy)]
pub enum Panel {
    Character,
    Skills,
    Stats,
    Config,
}
pub struct EditorView {
    session: Entity<Session>,
    tree: Entity<TreeView>,
    panel: Panel,
    level: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}
impl EditorView {
    pub fn new(
        session: Entity<Session>,
        tree: Entity<TreeView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let level = cx.new(|cx: &mut Context<InputState>| {
            cx.observe_global_in::<Locale>(window, |input, window, cx| {
                input.set_placeholder(tr("tree.editor.level_placeholder"), window, cx);
            })
            .detach();
            let mut input =
                InputState::new(window, cx).placeholder(tr("tree.editor.level_placeholder"));
            input.set_value(session.read(cx).snapshot().level.to_string(), window, cx);
            input
        });
        let subscriptions = vec![
            cx.observe_in(&session, window, |this, _, window, cx| {
                let value = this.session.read(cx).snapshot().level.to_string();
                if this.level.read(cx).value().as_ref() != value {
                    this.level
                        .update(cx, |input, cx| input.set_value(value, window, cx));
                }
                cx.notify();
            }),
            cx.observe(&tree, |_, _, cx| cx.notify()),
            cx.subscribe(&level, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur)
                    && let Ok(level) = this.level.read(cx).value().parse::<u32>()
                {
                    this.edit(cx, |snapshot| snapshot.set_level(level));
                }
            }),
        ];
        Self {
            session,
            tree,
            panel: Panel::Character,
            level,
            _subscriptions: subscriptions,
        }
    }
    pub fn show(&mut self, panel: Panel, cx: &mut Context<Self>) {
        self.panel = panel;
        cx.notify();
    }
    fn edit(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut BuildSnapshot)) {
        self.session.update(cx, |session, cx| {
            session.edit(|draft| edit(&mut draft.snapshot));
            cx.notify();
        });
    }
    fn character(&self, cx: &Context<Self>) -> Div {
        let snapshot = self.session.read(cx).snapshot();
        let mut classes = data::data().classes.values().collect::<Vec<_>>();
        classes.sort_by_key(|class| &class.name);
        let mut content = div()
            .flex()
            .flex_col()
            .gap_4()
            .child(div().text_2xl().child(tr("tree.editor.character")))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children(classes.into_iter().map(|class| {
                        let id = class.id.clone();
                        Button::new(SharedString::from(format!("class-{id}")))
                            .planner_style(cx)
                            .label(class.name.clone())
                            .selected(snapshot.class_id.as_deref() == Some(&id))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.edit(cx, |snapshot| snapshot.set_class(&id))
                            }))
                    })),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(tr("tree.editor.level"))
                    .child(
                        div()
                            .w(rems(8.))
                            .child(Input::new(&self.level).planner_style(cx)),
                    ),
            );
        let spent: u32 = snapshot.allocated.values().sum();
        content = content.child(trf(
            "tree.editor.attribute_points",
            &[(
                "count",
                snapshot
                    .level
                    .saturating_mul(data::game_config().attribute_points_per_level)
                    .saturating_sub(spent)
                    .to_string(),
            )],
        ));
        for attribute in &data::game_config().attributes {
            let key = attribute.key.clone();
            let subtract = key.clone();
            let rank = snapshot.allocated.get(&key).copied().unwrap_or(0);
            content = content.child(
                div()
                    .id(SharedString::from(format!("attribute-{key}")))
                    .flex()
                    .gap_3()
                    .items_center()
                    .child(div().w(rems(14.)).child(gpui_kit::text!(
                        id = "attribute-name",
                        attribute.name.clone()
                    )))
                    .child(icon_button("less", "−", false, cx).on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.edit(cx, |snapshot| snapshot.adjust_attribute(&subtract, -1))
                        },
                    )))
                    .child(rank.to_string())
                    .child(icon_button("more", "+", false, cx).on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.edit(cx, |snapshot| snapshot.adjust_attribute(&key, 1))
                        },
                    ))),
            );
        }
        content.child(
            div()
                .flex()
                .gap_2()
                .children(data::game_config().difficulties.iter().map(|difficulty| {
                    let key = difficulty.id.clone();
                    Button::new(SharedString::from(format!("difficulty-{key}")))
                        .planner_style(cx)
                        .label(difficulty.name.clone())
                        .selected(snapshot.difficulty == key)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.edit(cx, |snapshot| snapshot.difficulty = key.clone())
                        }))
                })),
        )
    }
    fn skills(&self, cx: &Context<Self>) -> Div {
        let snapshot = self.session.read(cx).snapshot();
        let palette = cx.global::<TooltipTheme>();
        let mut content = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().text_2xl().child(tr("tree.editor.skills")));
        for skill in data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or("")) {
            let id = skill.id.clone();
            let minus = id.clone();
            let plus = id.clone();
            let active = id.clone();
            let kind = skill.kind;
            let rank = snapshot.skill_ranks.get(&id).copied().unwrap_or(0);
            let enabled = match kind {
                SkillKind::Active => snapshot.active_skill_ids.contains(&id),
                SkillKind::Aura => snapshot.active_aura_id.as_ref() == Some(&id),
                SkillKind::Buff => snapshot.active_buffs.get(&id).copied().unwrap_or(false),
                SkillKind::Passive => false,
            };
            let mut card = div()
                .id(SharedString::from(format!("skill-{id}")))
                .p_3()
                .border_1()
                .border_color(palette.border)
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .text_lg()
                                .child(gpui_kit::text!(id = "skill-name", skill.name.clone())),
                        )
                        .child(icon_button("less", "−", false, cx).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.edit(cx, |s| s.set_skill_rank(&minus, rank.saturating_sub(1)))
                            },
                        )))
                        .child(format!("{rank} / {}", skill.max_rank))
                        .child(icon_button("more", "+", false, cx).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.edit(cx, |s| s.set_skill_rank(&plus, rank + 1))
                            },
                        )))
                        .when(kind != SkillKind::Passive, |row| {
                            row.child(
                                Checkbox::new("active")
                                    .label(tr("tree.editor.active"))
                                    .checked(enabled)
                                    .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                        this.edit(cx, |s| match kind {
                                            SkillKind::Active => {
                                                s.active_skill_ids.retain(|v| v != &active);
                                                if *checked {
                                                    s.active_skill_ids.push(active.clone());
                                                }
                                            }
                                            SkillKind::Aura => {
                                                s.active_aura_id = checked.then(|| active.clone())
                                            }
                                            SkillKind::Buff => {
                                                s.active_buffs.insert(active.clone(), *checked);
                                            }
                                            SkillKind::Passive => {}
                                        })
                                    })),
                            )
                        }),
                )
                .children(skill.description.clone().map(|description| {
                    div().text_sm().text_color(palette.muted).child(description)
                }));
            if let Some(subskills) = &skill.subskills {
                for node in subskills {
                    let key = format!("{}:{}", id, node.id);
                    let subrank = snapshot.subskill_ranks.get(&key).copied().unwrap_or(0);
                    let main = id.clone();
                    let node_id = node.id.clone();
                    let plus_main = main.clone();
                    let plus_node = node_id.clone();
                    card =
                        card.child(
                            div()
                                .id(SharedString::from(key))
                                .flex()
                                .items_center()
                                .gap_2()
                                .pl_4()
                                .child(div().flex_1().text_sm().child(gpui_kit::text!(
                                    id = "subskill-name",
                                    node.name.clone()
                                )))
                                .child(icon_button("less", "−", false, cx).on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.edit(cx, |s| {
                                            s.set_subskill_rank(
                                                &main,
                                                &node_id,
                                                subrank.saturating_sub(1),
                                            )
                                        })
                                    },
                                )))
                                .child(format!("{subrank} / {}", node.max_rank))
                                .child(icon_button("more", "+", false, cx).on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.edit(cx, |s| {
                                            s.set_subskill_rank(&plus_main, &plus_node, subrank + 1)
                                        })
                                    },
                                ))),
                        );
                }
            }
            content = content.child(card);
        }
        content
    }
    fn stats(&self, cx: &Context<Self>) -> Div {
        let palette = cx.global::<TooltipTheme>();
        let mut content = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().text_2xl().child(tr("tree.editor.stats")));
        let Some(performance) = self.tree.read(cx).performance() else {
            return content.child(tr("tree.editor.calculating"));
        };
        for skill in &performance.per_skill {
            content = content.child(
                div().text_lg().child(trf(
                    "tree.editor.skill_dps",
                    &[
                        (
                            "skill",
                            skill
                                .performance
                                .active_skill_name
                                .as_deref()
                                .unwrap_or(&skill.skill_id)
                                .to_owned(),
                        ),
                        (
                            "dps",
                            format_range(
                                (
                                    skill.performance.combined_dps_min.unwrap_or(0.),
                                    skill.performance.combined_dps_max.unwrap_or(0.),
                                ),
                                false,
                            ),
                        ),
                    ],
                )),
            );
        }
        for (group, stats, sources) in [
            (
                tr("tree.editor.attributes"),
                &performance.computed.attributes,
                &performance.computed.attribute_sources,
            ),
            (
                tr("tree.editor.statistics"),
                &performance.computed.stats,
                &performance.computed.stat_sources,
            ),
        ] {
            content = content.child(div().text_xl().child(group));
            let mut stats = stats.iter().collect::<Vec<_>>();
            stats.sort_by_key(|(key, _)| *key);
            for (key, value) in stats {
                let definition = data::game_config()
                    .stats
                    .iter()
                    .find(|stat| stat.key == *key);
                let label = definition
                    .map(|stat| stat.name.clone())
                    .unwrap_or_else(|| key.clone());
                let percent =
                    definition.is_some_and(|stat| stat.format.as_deref() == Some("percent"));
                content = content.child(
                    div()
                        .id(SharedString::from(format!("{group}-{key}")))
                        .py_2()
                        .border_b_1()
                        .border_color(palette.border)
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .child(label)
                                .child(format_range(*value, percent)),
                        )
                        .children(sources.get(key).into_iter().flatten().map(|source| {
                            div()
                                .pl_3()
                                .text_sm()
                                .text_color(palette.muted)
                                .child(format!(
                                    "{}: {}",
                                    source.label,
                                    format_range(source.value, percent)
                                ))
                        })),
                );
            }
        }
        content
    }
    fn config(&self, cx: &Context<Self>) -> Div {
        let snapshot = self.session.read(cx).snapshot();
        let mut content = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().text_2xl().child(tr("tree.editor.encounter")));
        for key in [
            "burning",
            "poisoned",
            "frozenbite",
            "stunned",
            "bleeding",
            "shocked",
            "deep_frozen",
            "shadow_burn",
            "frozen",
            "slow",
            "low_life",
            "serrated_chains",
            "lightning_break",
            "fire_break",
            "cold_break",
            "arcane_break",
            "poison_break",
            "is_boss",
        ] {
            let enabled = snapshot.enemy_conditions.get(key).copied().unwrap_or(false);
            content = content.child(
                Checkbox::new(SharedString::from(format!("enemy-{key}")))
                    .label(trf(
                        "tree.editor.enemy_condition",
                        &[("condition", condition_name(key).to_owned())],
                    ))
                    .checked(enabled)
                    .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                        this.edit(cx, |s| {
                            s.enemy_conditions.insert(key.into(), *checked);
                        })
                    })),
            )
        }
        for key in ["crit_chance_below_40", "life_below_40"] {
            let enabled = snapshot
                .player_conditions
                .get(key)
                .copied()
                .unwrap_or(false);
            content = content.child(
                Checkbox::new(SharedString::from(format!("player-{key}")))
                    .label(trf(
                        "tree.editor.player_condition",
                        &[("condition", condition_name(key).to_owned())],
                    ))
                    .checked(enabled)
                    .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                        this.edit(cx, |s| {
                            s.player_conditions.insert(key.into(), *checked);
                        })
                    })),
            )
        }
        for key in ["fire", "cold", "lightning", "poison", "arcane"] {
            let value = snapshot.enemy_resistances.get(key).copied().unwrap_or(85.);
            content = content.child(
                div()
                    .id(SharedString::from(format!("resistance-{key}")))
                    .flex()
                    .gap_3()
                    .items_center()
                    .child(trf(
                        "tree.editor.resistance",
                        &[
                            ("element", condition_name(key).to_owned()),
                            ("value", format!("{value:.0}")),
                        ],
                    ))
                    .child(
                        Button::new("less")
                            .planner_style(cx)
                            .label("−5")
                            .small()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.edit(cx, |s| {
                                    s.enemy_resistances
                                        .insert(key.into(), (value - 5.).max(-100.));
                                })
                            })),
                    )
                    .child(
                        Button::new("more")
                            .planner_style(cx)
                            .label("+5")
                            .small()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.edit(cx, |s| {
                                    s.enemy_resistances
                                        .insert(key.into(), (value + 5.).min(100.));
                                })
                            })),
                    ),
            );
        }
        content
    }
}
impl Render for EditorView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.panel {
            Panel::Character => self.character(cx),
            Panel::Skills => self.skills(cx),
            Panel::Stats => self.stats(cx),
            Panel::Config => self.config(cx),
        };
        div()
            .id("character-editor")
            .size_full()
            .overflow_y_scroll()
            .p_5()
            .bg(cx.global::<TooltipTheme>().panel)
            .child(content)
    }
}
