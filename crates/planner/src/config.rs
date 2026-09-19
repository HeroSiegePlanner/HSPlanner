//! Character and encounter editing. Each control writes through the shared session.
use crate::TreeView;
use gpui_kit::base::Disableable;
use gpui_kit::component::{
    Sizable,
    button::Button,
    checkbox::Checkbox,
    input::{Input, InputEvent, InputState, Textarea, TextareaState},
    radio::Radio,
    select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState},
    slider::SliderState,
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::{
    BuildSnapshot,
    loadout::LoadoutKind,
    session::{Draft, Session},
};
use hsplanner_engine::calc::{
    custom_stat::parse_custom_stat_value,
    data,
    planner::PlannerPerformance,
    types::{CustomStat, SkillKind},
};
use hsplanner_ui::i18n::{Locale, tr, trf};
use hsplanner_ui::scroll::PageScroll;
use hsplanner_ui::{
    components::{panel, panel_with_trailing, section_heading},
    controls::PlannerControl,
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
struct Choice {
    id: String,
    name: String,
    resist_penalty: Option<f64>,
}
impl SelectItem for Choice {
    type Value = String;
    fn title(&self) -> SharedString {
        match self.resist_penalty {
            Some(penalty) => {
                let name = match self.id.as_str() {
                    "normal" => tr("planner.config.normal"),
                    "nightmare" => tr("planner.config.nightmare"),
                    "hell" => tr("planner.config.hell"),
                    "inferno" => tr("planner.config.inferno"),
                    _ => &self.name,
                };
                if penalty == 0. {
                    name.to_owned().into()
                } else {
                    trf(
                        "planner.config.difficulty_option",
                        &[("0", name.to_owned()), ("1", format!("{penalty:.0}"))],
                    )
                    .into()
                }
            }
            None => self.name.clone().into(),
        }
    }
    fn value(&self) -> &String {
        &self.id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum NumberField {
    Level,
    SubskillPoints,
    Resistance(String),
    Kills,
    Entity(String),
    Stack(String),
    Projectile(String),
}

#[derive(Clone, PartialEq, Eq)]
struct ConfigDocument {
    build: Option<String>,
    skills: String,
    class: Option<String>,
}

impl ConfigDocument {
    fn from_draft(draft: &Draft) -> Self {
        Self {
            build: draft.build_id.clone(),
            skills: draft.loadouts.active_id(LoadoutKind::Skills).to_owned(),
            class: draft.snapshot.class_id.clone(),
        }
    }

    fn number_changed(&self, next: &Self, field: &NumberField) -> bool {
        self.build != next.build
            || match field {
                NumberField::SubskillPoints => {
                    self.skills != next.skills || self.class != next.class
                }
                NumberField::Projectile(_) => self.class != next.class,
                _ => false,
            }
    }
}

/// The last value written into a field by the model, rather than by the user.
/// Keeping this separate lets notifications detect a pending edit even before
/// the input's Change/Blur event has reached the view.
struct NumberDraft {
    applied: String,
}

impl NumberDraft {
    fn is_dirty(&self, current: &str) -> bool {
        current != self.applied
    }

    fn reconcile(
        &mut self,
        current: &str,
        incoming: &str,
        focused: bool,
        changed_document: bool,
    ) -> bool {
        if !changed_document && (focused || self.is_dirty(current)) {
            return false;
        }
        self.applied = incoming.to_owned();
        current != incoming
    }
}

pub struct ConfigView {
    session: Entity<Session>,
    tree: Entity<TreeView>,
    observed_performance: Option<Arc<PlannerPerformance>>,
    synced_calculation_revision: u64,
    class: Entity<SelectState<SearchableVec<Choice>>>,
    difficulty: Entity<SelectState<SearchableVec<Choice>>>,
    level_slider: Entity<SliderState>,
    level_slider_focus: FocusHandle,
    numbers: HashMap<NumberField, Entity<InputState>>,
    number_drafts: HashMap<NumberField, NumberDraft>,
    number_subscriptions: HashMap<NumberField, Subscription>,
    custom_editor: Entity<TextareaState>,
    custom_saved: String,
    custom_issues: Vec<String>,
    custom_task: Option<Task<()>>,
    document: ConfigDocument,
    error: Option<String>,
    focus: FocusHandle,
    scroll: ScrollHandle,
    has_vertical_scroll: bool,
    _subscriptions: Vec<Subscription>,
}

impl ConfigView {
    pub fn new(
        session: Entity<Session>,
        tree: Entity<TreeView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut classes = data::data()
            .classes
            .values()
            .map(|class| Choice {
                id: class.id.clone(),
                name: class.name.clone(),
                resist_penalty: None,
            })
            .collect::<Vec<_>>();
        classes.sort_by(|a, b| a.name.cmp(&b.name));
        let class = cx.new(|cx| {
            SelectState::new(SearchableVec::new(classes), None, window, cx).searchable(true)
        });
        let difficulties = data::game_config()
            .difficulties
            .iter()
            .map(|difficulty| Choice {
                id: difficulty.id.clone(),
                name: difficulty.name.clone(),
                resist_penalty: Some(difficulty.resist_penalty),
            })
            .collect::<Vec<_>>();
        let difficulty = cx.new(|cx| {
            SelectState::new(SearchableVec::new(difficulties), None, window, cx).searchable(true)
        });
        let initial_level = session.read(cx).snapshot().level as f32;
        let custom_saved = serialize_custom_stats(&session.read(cx).snapshot().custom_stats);
        let custom_editor = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx)
                .auto_grow(4, 12)
                .placeholder(tr("planner.config.custom_example"));
            state.set_value(custom_saved.clone(), window, cx);
            state
        });
        let level_slider = cx.new(|_| {
            SliderState::new()
                .min(1.)
                .max(data::game_config().max_character_level as f32)
                .step(1.)
                .default_value(initial_level)
        });
        let observed_performance = tree.read(cx).performance();
        let synced_calculation_revision = session.read(cx).calculation_revision();
        let subscriptions = vec![
            cx.observe_global_in::<Locale>(window, |this, window, cx| {
                this.custom_editor.update(cx, |input, cx| {
                    input.set_placeholder(tr("planner.config.custom_example"), window, cx);
                });
                let text = this.custom_editor.read(cx).value().to_string();
                this.custom_issues = parse_custom_text(&text).1;
                if this.error.is_some() {
                    this.error = Some(tr("planner.config.finite_number").into());
                }
                // Choices retain canonical IDs and resolve their titles in the current locale.
                this.difficulty.update(cx, |_, cx| cx.notify());
                cx.notify();
            }),
            cx.subscribe(&custom_editor, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.custom_task = Some(cx.spawn(async move |this, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(200))
                            .await;
                        let _ = this.update(cx, |this, cx| this.commit_custom_stats(cx));
                    }));
                } else if matches!(event, InputEvent::Blur) {
                    this.custom_task = None;
                    this.commit_custom_stats(cx);
                }
            }),
            cx.observe_in(&session, window, |this, _, window, cx| {
                let session = this.session.read(cx);
                let draft = session.draft();
                if this.synced_calculation_revision != session.calculation_revision()
                    || this.document != ConfigDocument::from_draft(draft)
                {
                    this.sync(window, cx);
                }
                cx.notify();
            }),
            cx.observe_in(&tree, window, |this, tree, window, cx| {
                let next = tree.read(cx).performance();
                if this.observed_performance.as_ref().map(Arc::as_ptr)
                    == next.as_ref().map(Arc::as_ptr)
                {
                    return;
                }
                this.observed_performance = next;
                this.sync(window, cx);
                cx.notify();
            }),
            cx.subscribe(
                &class,
                |this, _, event: &SelectEvent<SearchableVec<Choice>>, cx| {
                    if let SelectEvent::Confirm(Some(id)) = event {
                        this.edit(cx, |snapshot| snapshot.set_class(id));
                    }
                },
            ),
            cx.subscribe(
                &difficulty,
                |this, _, event: &SelectEvent<SearchableVec<Choice>>, cx| {
                    if let SelectEvent::Confirm(Some(id)) = event {
                        this.edit(cx, |snapshot| snapshot.difficulty = id.clone());
                    }
                },
            ),
            cx.observe(&level_slider, |this, slider, cx| {
                let level = slider.read(cx).value().end().round() as u32;
                if this.session.read(cx).snapshot().level != level {
                    this.edit(cx, |snapshot| snapshot.set_level(level));
                }
            }),
        ];
        let document = ConfigDocument::from_draft(session.read(cx).draft());
        let mut view = Self {
            session,
            tree,
            observed_performance,
            synced_calculation_revision,
            class,
            difficulty,
            level_slider,
            level_slider_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            numbers: HashMap::new(),
            number_drafts: HashMap::new(),
            number_subscriptions: HashMap::new(),
            custom_editor,
            custom_saved,
            custom_issues: Vec::new(),
            custom_task: None,
            document,
            error: None,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            has_vertical_scroll: false,
            _subscriptions: subscriptions,
        };
        view.sync(window, cx);
        view
    }

    fn edit(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut BuildSnapshot)) {
        self.error = None;
        self.session.update(cx, |session, cx| {
            session.edit(|draft| edit(&mut draft.snapshot));
            cx.notify();
        });
    }

    fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.synced_calculation_revision = self.session.read(cx).calculation_revision();
        let snapshot = self.session.read(cx).snapshot().clone();
        let draft = self.session.read(cx).draft();
        let document = ConfigDocument::from_draft(draft);
        let previous_document = std::mem::replace(&mut self.document, document);
        let changed_document = previous_document.build != self.document.build;
        let custom = serialize_custom_stats(&snapshot.custom_stats);
        if changed_document || custom != self.custom_saved {
            self.custom_task = None;
            self.custom_saved = custom.clone();
            self.custom_issues.clear();
            self.error = None;
            self.custom_editor
                .update(cx, |input, cx| input.set_value(custom, window, cx));
        }
        if self.class.read(cx).selected_value() != snapshot.class_id.as_ref() {
            self.class.update(cx, |state, cx| {
                if let Some(id) = &snapshot.class_id {
                    state.set_selected_value(id, window, cx);
                } else {
                    state.set_selected_index(None, window, cx);
                }
            });
        }
        if self.difficulty.read(cx).selected_value() != Some(&snapshot.difficulty) {
            self.difficulty.update(cx, |state, cx| {
                state.set_selected_value(&snapshot.difficulty, window, cx)
            });
        }
        if self.level_slider.read(cx).value().start() != snapshot.level as f32 {
            self.level_slider.update(cx, |state, cx| {
                state.set_value(snapshot.level as f32, window, cx)
            });
        }
        let mut fields = vec![
            NumberField::Level,
            NumberField::SubskillPoints,
            NumberField::Kills,
        ];
        fields.extend(
            ["fire", "cold", "lightning", "poison", "arcane"]
                .into_iter()
                .map(|key| NumberField::Resistance(key.into())),
        );
        fields.extend(
            ["sentry", "summon", "guardian"]
                .into_iter()
                .map(|key| NumberField::Entity(key.into())),
        );
        fields.extend(
            data::game_config()
                .stack_types
                .iter()
                .map(|stack| NumberField::Stack(stack.key.clone())),
        );
        fields.extend(
            data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""))
                .iter()
                .map(|skill| NumberField::Projectile(skill.id.clone())),
        );
        self.numbers.retain(|field, _| fields.contains(field));
        self.number_drafts.retain(|field, _| fields.contains(field));
        self.number_subscriptions
            .retain(|field, _| fields.contains(field));
        for field in fields {
            let performance = self.tree.read(cx).performance();
            let value = number_text(
                &snapshot,
                &field,
                performance.as_ref().map(|result| &result.current),
            );
            if let Some(input) = self.numbers.get(&field) {
                let current = input.read(cx).value();
                let focused = input.focus_handle(cx).is_focused(window);
                let draft =
                    self.number_drafts
                        .entry(field.clone())
                        .or_insert_with(|| NumberDraft {
                            applied: current.to_string(),
                        });
                let changed_owner = previous_document.number_changed(&self.document, &field);
                if draft.reconcile(&current, &value, focused, changed_owner) {
                    input.update(cx, |input, cx| input.set_value(value, window, cx));
                }
            } else {
                self.number_drafts.insert(
                    field.clone(),
                    NumberDraft {
                        applied: value.clone(),
                    },
                );
                let input = cx.new(|cx| {
                    let mut input = InputState::new(window, cx).placeholder("0");
                    input.set_value(value, window, cx);
                    input
                });
                let key = field.clone();
                let subscription = cx.subscribe_in(
                    &input,
                    window,
                    move |this, _, event: &InputEvent, window, cx| {
                        if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                            this.commit_number(&key, window, cx)
                        }
                    },
                );
                self.numbers.insert(field.clone(), input);
                self.number_subscriptions.insert(field, subscription);
            }
        }
    }

    fn commit_number(&mut self, field: &NumberField, window: &mut Window, cx: &mut Context<Self>) {
        let draft = self.session.read(cx).draft();
        if self
            .document
            .number_changed(&ConfigDocument::from_draft(draft), field)
        {
            self.sync(window, cx);
            return;
        }
        let Some(input) = self.numbers.get(field).cloned() else {
            return;
        };
        let current = input.read(cx).value();
        if !self
            .number_drafts
            .get(field)
            .is_some_and(|draft| draft.is_dirty(&current))
        {
            self.error = None;
            self.refresh_number(field, &input, window, cx);
            cx.notify();
            return;
        }
        let text = current.trim().to_owned();
        let value = if text.is_empty() {
            None
        } else {
            match text.parse::<f64>() {
                Ok(value) if value.is_finite() => Some(value),
                _ => {
                    self.error = Some(tr("planner.config.finite_number").into());
                    cx.notify();
                    return;
                }
            }
        };
        let stack_max = if let NumberField::Stack(key) = field {
            data::game_config()
                .stack_types
                .iter()
                .find(|stack| &stack.key == key)
                .and_then(|stack| {
                    self.tree
                        .read(cx)
                        .performance()
                        .and_then(|result| result.current.stats.get(&stack.max_stat).copied())
                })
                .map(|value| value.1.floor().max(0.) as u32)
        } else {
            None
        };
        self.edit(cx, |snapshot| match field {
            NumberField::Level => snapshot.set_level(value.unwrap_or(1.).max(1.) as u32),
            NumberField::SubskillPoints => {
                snapshot.set_max_subskill_points(value.unwrap_or(20.).max(1.) as u32)
            }
            NumberField::Resistance(key) => {
                if let Some(value) = value {
                    snapshot.enemy_resistances.insert(key.clone(), value);
                } else {
                    snapshot.enemy_resistances.remove(key);
                }
            }
            NumberField::Kills => snapshot.kills_per_sec = value.unwrap_or(0.).max(0.),
            NumberField::Entity(key) => {
                snapshot
                    .entity_rates
                    .insert(key.clone(), value.unwrap_or(1.).max(0.));
            }
            NumberField::Stack(key) => {
                if let Some(value) = value {
                    snapshot.stack_counts.insert(
                        key.clone(),
                        (value.max(0.) as u32).min(stack_max.unwrap_or(u32::MAX)),
                    );
                } else {
                    snapshot.stack_counts.remove(key);
                }
            }
            NumberField::Projectile(key) => {
                if let Some(value) = value {
                    snapshot
                        .skill_projectiles
                        .insert(key.clone(), value.clamp(1., 99.) as u32);
                } else {
                    snapshot.skill_projectiles.remove(key);
                }
            }
        });
        self.refresh_number(field, &input, window, cx);
    }

    fn refresh_number(
        &mut self,
        field: &NumberField,
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let performance = self.tree.read(cx).performance();
        let value = number_text(
            self.session.read(cx).snapshot(),
            field,
            performance.as_ref().map(|result| &result.current),
        );
        self.number_drafts.insert(
            field.clone(),
            NumberDraft {
                applied: value.clone(),
            },
        );
        if input.read(cx).value().as_ref() != value {
            input.update(cx, |input, cx| input.set_value(value, window, cx));
        }
    }

    fn number(&self, field: NumberField, label: impl Into<SharedString>, cx: &App) -> Div {
        let p = cx.global::<TooltipTheme>();
        let (width, height) = match &field {
            NumberField::Level | NumberField::SubskillPoints => (54., 30.),
            NumberField::Resistance(_) => (49., 23.),
            NumberField::Projectile(_) | NumberField::Stack(_) => (45., 23.),
            NumberField::Kills | NumberField::Entity(_) => (65., 27.),
        };
        div().w(rems(width / 13.)).flex_shrink_0().when_some(
            self.numbers.get(&field),
            |view, input| {
                view.child(
                    Input::new(input)
                        .small()
                        .planner_style(cx)
                        .map(|input| Styled::h(input, rems(height / 13.)))
                        .line_height(relative(1.5))
                        .px_1p5()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(rems(12. / 13.))
                        .text_color(p.accent_hot)
                        .when(matches!(field, NumberField::Level), |input| {
                            input.text_center()
                        })
                        .when(!matches!(field, NumberField::Level), |input| {
                            input.text_right()
                        })
                        .aria_label(label),
                )
            },
        )
    }

    fn commit_custom_stats(&mut self, cx: &mut Context<Self>) {
        let text = self.custom_editor.read(cx).value().to_string();
        let (stats, issues) = parse_custom_text(&text);
        self.custom_issues = issues;
        let saved = serialize_custom_stats(&stats);
        if saved != self.custom_saved {
            self.custom_saved = saved;
            self.edit(cx, |snapshot| snapshot.custom_stats = stats);
        }
        cx.notify();
    }

    fn custom_stats(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        panel_with_trailing(
            "config-custom",
            tr("planner.config.custom"),
            count_badge(
                "custom-count",
                self.session.read(cx).snapshot().custom_stats.len(),
                None,
                cx,
            ),
            cx,
        )
        .child(help(tr("planner.config.custom_help"), cx))
        .child(
            Textarea::new(&self.custom_editor)
                .w_full()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(12. / 13.))
                .border_color(p.border_strong)
                .bg(p.background),
        )
        .children(self.custom_issues.iter().map(|issue| {
            div()
                .mt_2()
                .px_2p5()
                .py_1p5()
                .rounded_sm()
                .border_1()
                .border_color(theme::stat_color("strength", cx).opacity(0.4))
                .text_size(rems(10. / 13.))
                .text_color(theme::stat_color("strength", cx))
                .child(issue.clone())
        }))
        .children(
            self.session
                .read(cx)
                .snapshot()
                .custom_stats
                .iter()
                .map(|stat| {
                    let name = data::game_config()
                        .stats
                        .iter()
                        .find(|def| def.key == stat.stat_key)
                        .map(|def| def.name.as_str())
                        .unwrap_or(&stat.stat_key);
                    let name = crate::stat_labels::stat_name(&stat.stat_key, name);
                    div()
                        .mt_1()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(rems(10. / 13.))
                        .text_color(p.positive)
                        .child(format!("→ {} {name}", stat.value))
                }),
        )
    }

    fn level_control(&self, window: &Window, cx: &Context<Self>) -> Stateful<Div> {
        div()
            .id("config-level-range")
            .flex_1()
            .track_focus(&self.level_slider_focus)
            .role(accesskit::Role::Group)
            .aria_label(tr("planner.config.character_level"))
            .rounded_sm()
            .when(self.level_slider_focus.is_focused(window), |view| {
                view.shadow(vec![BoxShadow {
                    color: cx.global::<TooltipTheme>().accent_deep,
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                    spread_radius: px(1.),
                    inset: false,
                }])
            })
            .capture_any_mouse_down(cx.listener(|this, event: &MouseDownEvent, window, cx| {
                if event.button == MouseButton::Left {
                    window.focus(&this.level_slider_focus, cx);
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if !this.level_slider_focus.is_focused(window) {
                    return;
                }
                let slider = this.level_slider.read(cx);
                let value = slider.value().end();
                let next = match event.keystroke.key.as_str() {
                    "left" | "down" => (value - slider.step_value()).max(slider.min_value()),
                    "right" | "up" => (value + slider.step_value()).min(slider.max_value()),
                    "home" => slider.min_value(),
                    "end" => slider.max_value(),
                    _ => return,
                };
                this.level_slider
                    .update(cx, |slider, cx| slider.set_value(next, window, cx));
                cx.stop_propagation();
            }))
            .child(hsplanner_ui::controls::planner_slider(
                &self.level_slider,
                window,
                cx,
            ))
    }

    fn basics(&self, logical_width: f32, window: &Window, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let spent: u32 = snapshot.allocated.values().sum();
        let total = snapshot
            .level
            .saturating_mul(data::game_config().attribute_points_per_level);
        let remaining = total.saturating_sub(spent);
        let mut attributes = panel_with_trailing(
            "config-attributes",
            tr("planner.common.attributes"),
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(label(
                    "attribute-budget",
                    trf(
                        "planner.config.attribute_budget",
                        &[
                            ("remaining", (remaining).to_string()),
                            ("total", (total).to_string()),
                        ],
                    ),
                    if remaining > 0 { p.accent_hot } else { p.muted },
                ))
                .child(
                    Button::new("reset-attributes")
                        .planner_style(cx)
                        .small()
                        .h_auto()
                        .px_2()
                        .py_1()
                        .text_size(rems(10. / 13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.5))
                        .accessibility_label(tr("planner.config.reset_attributes"))
                        .child(TooltipText::new(
                            "reset-attributes-label",
                            tr("planner.config.reset_caps"),
                            0.14,
                        ))
                        .disabled(spent == 0)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.edit(cx, |snapshot| snapshot.allocated.clear())
                        })),
                ),
            cx,
        )
        .child(help(tr("planner.config.allocate_hint"), cx));
        let class = snapshot.class_id.as_deref().and_then(data::get_class);
        let mut rows = div()
            .grid()
            .grid_cols(if logical_width >= 640. { 2 } else { 1 })
            .gap_2();
        for attr in &data::game_config().attributes {
            let key = attr.key.clone();
            let minus = key.clone();
            let added = snapshot.allocated.get(&key).copied().unwrap_or(0);
            let base = data::game_config()
                .default_base_attributes
                .as_ref()
                .and_then(|a| a.get(&key))
                .copied()
                .unwrap_or(0.)
                + class
                    .and_then(|c| c.base_attributes.get(&key))
                    .copied()
                    .unwrap_or(0.);
            rows = rows.child(
                tile(added > 0, cx)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if added > 0 { p.accent_hot } else { p.text })
                                    .child(crate::stat_labels::attribute_name(
                                        &attr.key, &attr.name,
                                    )),
                            )
                            .child(label(
                                SharedString::from(format!("attribute-base-{key}")),
                                trf(
                                    "planner.config.attribute_base",
                                    &[("base", format!("{:.0}", base))],
                                ),
                                p.faint,
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_shrink_0()
                            .items_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(p.border_strong)
                            .bg(linear_gradient(
                                180.,
                                linear_color_stop(p.background, 0.),
                                linear_color_stop(p.panel_secondary, 1.),
                            ))
                            .shadow(vec![BoxShadow {
                                color: p.shadow.opacity(0.5),
                                offset: point(px(0.), px(1.)),
                                blur_radius: px(2.),
                                spread_radius: px(0.),
                                inset: true,
                            }])
                            .child(
                                Button::new(SharedString::from(format!("attribute-remove-{key}")))
                                    .planner_style(cx)
                                    .small()
                                    .h_auto()
                                    .px_2p5()
                                    .py_1p5()
                                    .font_weight(FontWeight::NORMAL)
                                    .line_height(relative(1.))
                                    .border_0()
                                    .child(
                                        // Button's small label slot otherwise overrides
                                        // the reference's 14px Inter spin-control text.
                                        div()
                                            .font_family(theme::FONT_FAMILY)
                                            .text_size(rems(14. / 13.))
                                            .line_height(relative(1.))
                                            .child("−"),
                                    )
                                    .disabled(added == 0)
                                    .accessibility_label(trf(
                                        "planner.common.decrease",
                                        &[(
                                            "0",
                                            crate::stat_labels::attribute_name(
                                                &attr.key, &attr.name,
                                            ),
                                        )],
                                    ))
                                    .on_click(cx.listener(move |this, event, _, cx| {
                                        let count = allocation_step(event, added);
                                        this.edit(cx, |snapshot| {
                                            snapshot.adjust_attribute(&minus, -(count as i32))
                                        });
                                    })),
                            )
                            .child(
                                div()
                                    .w_9()
                                    .text_center()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.accent_hot)
                                    .child(format!("{:.0}", base + added as f64)),
                            )
                            .child(
                                Button::new(SharedString::from(format!("attribute-add-{key}")))
                                    .planner_style(cx)
                                    .small()
                                    .h_auto()
                                    .px_2p5()
                                    .py_1p5()
                                    .font_weight(FontWeight::NORMAL)
                                    .line_height(relative(1.))
                                    .border_0()
                                    .child(
                                        div()
                                            .font_family(theme::FONT_FAMILY)
                                            .text_size(rems(14. / 13.))
                                            .line_height(relative(1.))
                                            .child("+"),
                                    )
                                    .disabled(remaining == 0)
                                    .accessibility_label(trf(
                                        "planner.common.increase",
                                        &[(
                                            "0",
                                            crate::stat_labels::attribute_name(
                                                &attr.key, &attr.name,
                                            ),
                                        )],
                                    ))
                                    .on_click(cx.listener(move |this, event, _, cx| {
                                        let count = allocation_step(event, remaining);
                                        this.edit(cx, |snapshot| {
                                            snapshot.adjust_attribute(&key, count as i32)
                                        });
                                    })),
                            ),
                    ),
            );
        }
        attributes = attributes.child(rows);
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .grid()
                    .grid_cols(if logical_width >= 768. { 2 } else { 1 })
                    .gap_4()
                    .child(
                        panel("config-class", tr("planner.common.class"), cx)
                            .child(help(tr("planner.config.class_help"), cx))
                            .child(
                                Select::new(&self.class)
                                    .planner_style(cx)
                                    .placeholder(tr("planner.config.select_class"))
                                    .search_placeholder(tr("planner.config.search_class"))
                                    .accessibility_label(tr("planner.common.class"))
                                    .w_full(),
                            ),
                    )
                    .child(
                        panel("config-level", tr("planner.common.level"), cx)
                            .child(help(tr("planner.config.level_help"), cx))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(self.level_control(window, cx))
                                    .child(self.number(
                                        NumberField::Level,
                                        tr("planner.config.character_level"),
                                        cx,
                                    )),
                            ),
                    ),
            )
            .child(
                panel_with_trailing(
                    "config-difficulty",
                    tr("planner.common.difficulty"),
                    label(
                        "difficulty-penalty",
                        trf(
                            "planner.config.all_resistances",
                            &[(
                                "0",
                                (data::game_config()
                                    .difficulties
                                    .iter()
                                    .find(|difficulty| difficulty.id == snapshot.difficulty)
                                    .map(|difficulty| difficulty.resist_penalty)
                                    .unwrap_or(0.))
                                .to_string(),
                            )],
                        ),
                        if snapshot.difficulty == "normal" {
                            p.muted
                        } else {
                            p.negative
                        },
                    ),
                    cx,
                )
                .child(help(tr("planner.config.difficulty_help"), cx))
                .child(
                    Select::new(&self.difficulty)
                        .planner_style(cx)
                        .placeholder(tr("planner.config.select_difficulty"))
                        .search_placeholder(tr("planner.config.search_difficulty"))
                        .accessibility_label(tr("planner.common.difficulty"))
                        .w_full(),
                ),
            )
            .child(attributes)
            .child(
                panel(
                    "config-subskill-points",
                    tr("planner.config.subskill_points"),
                    cx,
                )
                .child(help(tr("planner.config.subskill_help"), cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(div().child(tr("planner.config.maximum_subtree")))
                        .child(self.number(
                            NumberField::SubskillPoints,
                            tr("planner.config.maximum_subskills"),
                            cx,
                        )),
                ),
            )
            .child(
                panel("config-charms", tr("planner.config.charm_inventory"), cx)
                    .child(help(tr("planner.config.charm_help"), cx))
                    .child(
                        Checkbox::new("extra-charm-slot")
                            .checked(self.session.read(cx).state().settings.extra_charm_slot)
                            .label(tr("planner.config.extra_charm"))
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.session.update(cx, |session, cx| {
                                    let mut settings = session.state().settings.clone();
                                    settings.extra_charm_slot = *checked;
                                    session.set_settings(settings);
                                    cx.notify();
                                })
                            })),
                    )
                    .child(
                        div()
                            .pl_6()
                            .mt_1()
                            .text_size(rems(12. / 13.))
                            .text_color(p.muted)
                            .child(tr("planner.config.extra_charm_help")),
                    ),
            )
    }

    fn conditions(&self, enemy: bool, small: bool, cx: &Context<Self>) -> Div {
        let snapshot = self.session.read(cx).snapshot();
        let p = cx.global::<TooltipTheme>();
        let (id, title, subtitle, conditions) = if enemy {
            (
                "config-enemy",
                tr("planner.config.enemy_conditions"),
                tr("planner.config.target_conditions"),
                ENEMY_CONDITIONS,
            )
        } else {
            (
                "config-player",
                tr("planner.config.player_conditions"),
                tr("planner.config.self_flags"),
                PLAYER_CONDITIONS,
            )
        };
        let mut rows = div()
            .grid()
            .grid_cols(match (enemy, small) {
                (true, true) => 3,
                (true, false) | (false, true) => 2,
                (false, false) => 1,
            })
            .gap_2();
        for &(key, name, color_key) in conditions {
            let checked = if enemy {
                &snapshot.enemy_conditions
            } else {
                &snapshot.player_conditions
            }
            .get(key)
            .copied()
            .unwrap_or(false);
            rows = rows.child(
                tile(checked, cx).child(
                    Checkbox::new(SharedString::from(format!("{id}-{key}")))
                        .label(tr(name))
                        .checked(checked)
                        .text_color(if color_key.is_empty() {
                            if checked { p.accent_hot } else { p.text }
                        } else {
                            condition_color(color_key, cx)
                        })
                        .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                            this.edit(cx, |snapshot| {
                                if enemy {
                                    &mut snapshot.enemy_conditions
                                } else {
                                    &mut snapshot.player_conditions
                                }
                                .insert(key.into(), *checked);
                            })
                        })),
                ),
            );
        }
        panel_with_trailing(
            id,
            title,
            count_badge(
                format!("{id}-count"),
                conditions
                    .iter()
                    .filter(|(key, _, _)| {
                        if enemy {
                            &snapshot.enemy_conditions
                        } else {
                            &snapshot.player_conditions
                        }
                        .get(*key)
                        .copied()
                        .unwrap_or(false)
                    })
                    .count(),
                Some(conditions.len()),
                cx,
            ),
            cx,
        )
        .child(help(subtitle, cx))
        .child(rows)
    }

    fn resistances(&self, small: bool, cx: &Context<Self>) -> Div {
        panel_with_trailing(
            "config-resistances",
            tr("planner.config.enemy_resistances"),
            count_badge(
                "resistance-count",
                ["fire", "cold", "lightning", "poison", "arcane"]
                    .into_iter()
                    .filter(|key| {
                        self.session
                            .read(cx)
                            .snapshot()
                            .enemy_resistances
                            .contains_key(*key)
                    })
                    .count(),
                Some(5),
                cx,
            ),
            cx,
        )
        .child(help(tr("planner.config.enemy_resistance_help"), cx))
        .child(
            div()
                .grid()
                .grid_cols(if small { 4 } else { 2 })
                .gap_2()
                .children(
                    [
                        ("fire", tr("planner.common.fire")),
                        ("cold", tr("planner.common.cold")),
                        ("lightning", tr("planner.common.lightning")),
                        ("poison", tr("planner.common.poison")),
                        ("arcane", tr("planner.common.arcane")),
                    ]
                    .into_iter()
                    .map(|(key, name)| {
                        tile(false, cx)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(label(
                                SharedString::from(format!("res-label-{key}")),
                                name,
                                condition_color(key, cx),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(self.number(
                                        NumberField::Resistance(key.into()),
                                        trf(
                                            "planner.config.enemy_resistance",
                                            &[("name", (name).to_string())],
                                        ),
                                        cx,
                                    ))
                                    .child("%"),
                            )
                    }),
                ),
        )
    }

    fn buffs(&self, cx: &Context<Self>) -> Div {
        let snapshot = self.session.read(cx).snapshot();
        let skills = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""));
        let buffs = skills
            .iter()
            .filter(|s| {
                s.kind == SkillKind::Buff
                    || s.tags
                        .as_ref()
                        .is_some_and(|tags| tags.iter().any(|tag| tag == "Buff"))
            })
            .collect::<Vec<_>>();
        let mut card = panel_with_trailing(
            "config-buffs",
            tr("planner.config.active_buffs"),
            count_badge(
                "buff-count",
                buffs
                    .iter()
                    .filter(|skill| {
                        snapshot
                            .active_buffs
                            .get(&skill.id)
                            .copied()
                            .unwrap_or(false)
                    })
                    .count(),
                Some(buffs.len()),
                cx,
            ),
            cx,
        )
        .child(help(tr("planner.config.buffs_help"), cx));
        if buffs.is_empty() {
            return card.child(empty(tr("planner.config.no_buffs"), cx));
        }
        for skill in buffs {
            let key = skill.id.clone();
            let checked = snapshot.active_buffs.get(&key).copied().unwrap_or(false);
            let learned = snapshot.skill_ranks.get(&key).copied().unwrap_or(0) > 0;
            card = card.child(
                tile(checked, cx).mb_2().child(
                    Checkbox::new(SharedString::from(format!("config-buff-{key}")))
                        .accessibility_label(skill.name.clone())
                        .child(skill_caption(skill, 32.))
                        .checked(checked)
                        .disabled(!learned)
                        .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                            this.edit(cx, |snapshot| {
                                snapshot.active_buffs.insert(key.clone(), *checked);
                            })
                        })),
                ),
            );
        }
        card
    }

    fn aura(&self, cx: &Context<Self>) -> Div {
        let snapshot = self.session.read(cx).snapshot();
        let skills = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""))
            .iter()
            .filter(|s| s.kind == SkillKind::Aura)
            .collect::<Vec<_>>();
        let mut merc_auras = std::collections::BTreeMap::new();
        for (slot, item) in &snapshot.merc_inventory {
            let Some(base) = data::get_item(&item.base_id) else {
                continue;
            };
            for (name, rank) in data::skill_bonus_entries(base, item) {
                let Some(granted) =
                    data::get_item_granted_skill_by_name(name).filter(|skill| skill.aura)
                else {
                    continue;
                };
                let key = name.trim().to_lowercase();
                let stars = if !granted.star_rank_locked && data::can_star_forge(slot, &base.rarity)
                {
                    item.stars
                } else {
                    None
                };
                let bonus = hsplanner_engine::calc::star_scaling::stat_star_flat_bonus(
                    Some("item_granted_skill_rank"),
                    stars,
                )
                .floor();
                let rank = rank.as_ranged();
                let level = (rank.0.round() + bonus, rank.1.round() + bonus);
                merc_auras
                    .entry(key)
                    .or_insert((name.clone(), base.name.clone(), level));
            }
        }
        let mut card = panel_with_trailing(
            "config-aura",
            tr("planner.config.active_aura"),
            count_badge(
                "aura-count",
                usize::from(snapshot.active_aura_id.is_some())
                    + merc_auras
                        .keys()
                        .filter(|key| {
                            !snapshot
                                .merc_disabled_auras
                                .get(*key)
                                .copied()
                                .unwrap_or(false)
                        })
                        .count(),
                Some(skills.len() + merc_auras.len()),
                cx,
            ),
            cx,
        )
        .child(help(tr("planner.config.aura_help"), cx));
        if skills.is_empty() {
            card = card.child(empty(tr("planner.config.no_auras"), cx));
        } else {
            card = card.child(
                tile(snapshot.active_aura_id.is_none(), cx).mb_2().child(
                    Radio::new("aura-none")
                        .label(tr("planner.common.none"))
                        .checked(snapshot.active_aura_id.is_none())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.edit(cx, |snapshot| snapshot.active_aura_id = None)
                        })),
                ),
            );
        }
        for skill in skills {
            let key = skill.id.clone();
            let checked = snapshot.active_aura_id.as_ref() == Some(&key);
            let learned = snapshot.skill_ranks.get(&key).copied().unwrap_or(0) > 0;
            card = card.child(
                tile(checked, cx).mb_2().child(
                    Radio::new(SharedString::from(format!("config-aura-{key}")))
                        .accessibility_label(skill.name.clone())
                        .child(skill_caption(skill, 32.))
                        .checked(checked)
                        .disabled(!learned)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.edit(cx, |snapshot| snapshot.active_aura_id = Some(key.clone()))
                        })),
                ),
            );
        }
        if !merc_auras.is_empty() {
            card = card.child(div().my_3().child(label(
                "config-merc-aura-label",
                tr("planner.common.mercenary"),
                cx.global::<TooltipTheme>().accent_hot,
            )));
            for (key, (name, item, level)) in merc_auras {
                let enabled = !snapshot
                    .merc_disabled_auras
                    .get(&key)
                    .copied()
                    .unwrap_or(false);
                card = card.child(
                    tile(enabled, cx)
                        .mb_2()
                        .child(
                            Checkbox::new(SharedString::from(format!("config-merc-aura-{key}")))
                                .checked(enabled)
                                .label(trf(
                                    "planner.config.aura_level",
                                    &[
                                        ("name", (name).to_string()),
                                        (
                                            "0",
                                            (crate::build_panel::format_range(level, false))
                                                .to_string(),
                                        ),
                                    ],
                                ))
                                .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                    this.edit(cx, |snapshot| {
                                        snapshot.merc_disabled_auras.insert(key.clone(), !*checked);
                                    })
                                })),
                        )
                        .child(
                            div()
                                .pl_6()
                                .text_size(rems(10. / 13.))
                                .text_color(cx.global::<TooltipTheme>().faint)
                                .child(item),
                        ),
                );
            }
        }
        card
    }

    fn blessings(&self, cx: &Context<Self>) -> Option<Div> {
        let snapshot = self.session.read(cx).snapshot();
        let mut granted = std::collections::BTreeMap::new();
        for item in snapshot.inventory.values() {
            let Some(base) = data::get_item(&item.base_id) else {
                continue;
            };
            for (name, _) in data::skill_bonus_entries(base, item) {
                if let Some(skill) = data::get_item_granted_skill_by_name(name)
                    && let Some(condition) = &skill.condition
                {
                    granted.insert(condition.clone(), skill);
                }
            }
        }
        if granted.is_empty() {
            return None;
        }
        let mut card = panel_with_trailing(
            "config-blessings",
            tr("planner.config.item_blessings"),
            count_badge(
                "blessing-count",
                granted
                    .keys()
                    .filter(|key| {
                        snapshot
                            .player_conditions
                            .get(*key)
                            .copied()
                            .unwrap_or(false)
                    })
                    .count(),
                Some(granted.len()),
                cx,
            ),
            cx,
        )
        .child(help(tr("planner.config.blessings_help"), cx));
        for (key, skill) in granted {
            let checked = snapshot
                .player_conditions
                .get(&key)
                .copied()
                .unwrap_or(false);
            card = card.child(
                tile(checked, cx)
                    .mb_2()
                    .child(
                        Checkbox::new(SharedString::from(format!("config-blessing-{key}")))
                            .checked(checked)
                            .label(skill.name.clone())
                            .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                this.edit(cx, |snapshot| {
                                    snapshot.player_conditions.insert(key.clone(), *checked);
                                })
                            })),
                    )
                    .when_some(skill.description.clone(), |view, description| {
                        view.child(
                            div()
                                .pl_6()
                                .text_size(rems(10. / 13.))
                                .text_color(cx.global::<TooltipTheme>().faint)
                                .child(description),
                        )
                    }),
            );
        }
        Some(card)
    }

    fn procs(&self, cx: &Context<Self>) -> Div {
        let snapshot = self.session.read(cx).snapshot();
        let mut rows = Vec::new();
        for skill in data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or("")) {
            if let Some(proc) = &skill.proc
                && snapshot.skill_ranks.get(&skill.id).copied().unwrap_or(0) > 0
            {
                rows.push((
                    skill.id.clone(),
                    skill.name.clone(),
                    format!(
                        "{}% · {} → {}",
                        proc.chance,
                        proc.trigger.replace("on_", ""),
                        proc.target
                    ),
                ));
            }
            for sub in skill.subskills.iter().flatten() {
                let key = format!("{}:{}", skill.id, sub.id);
                let rank = snapshot.subskill_ranks.get(&key).copied().unwrap_or(0);
                if let Some(proc) = &sub.proc
                    && let Some(target) = &proc.target
                    && rank > 0
                {
                    rows.push((
                        key,
                        sub.name.clone(),
                        format!(
                            "{}% · {} → {}",
                            proc.chance.base.unwrap_or(0.)
                                + proc.chance.per_rank.unwrap_or(0.) * rank as f64,
                            proc.trigger.replace("on_", ""),
                            target
                        ),
                    ));
                }
            }
        }
        let mut item_procs = std::collections::BTreeMap::new();
        for item in snapshot.inventory.values() {
            let Some(base) = data::get_item(&item.base_id) else {
                continue;
            };
            for (name, _) in data::skill_bonus_entries(base, item) {
                if let Some(skill) = data::get_item_granted_skill_by_name(name)
                    && skill
                        .proc_damage
                        .as_ref()
                        .is_some_and(|damage| !damage.is_empty())
                {
                    item_procs.insert(
                        format!("granted:{}", skill.id),
                        (
                            skill.name.clone(),
                            trf(
                                "planner.config.every_seconds",
                                &[(
                                    "0",
                                    (skill.proc_cooldown.unwrap_or(1.5).max(1.5)).to_string(),
                                )],
                            ),
                        ),
                    );
                }
            }
            for proc in base.procs.iter().flatten() {
                let Some(target) = &proc.target else { continue };
                let Some(rank) = proc.cast_level else {
                    continue;
                };
                let normalized = hsplanner_engine::calc::rank::normalize_skill_name(target);
                if let Some(skill) = data::data()
                    .skills_by_class
                    .values()
                    .flatten()
                    .find(|skill| {
                        hsplanner_engine::calc::rank::normalize_skill_name(&skill.name)
                            == normalized
                    })
                {
                    let key = hsplanner_engine::calc::build::item_cast_toggle_key(
                        &item.base_id,
                        &normalized,
                    );
                    item_procs.insert(
                        key,
                        (
                            skill.name.clone(),
                            trf(
                                "planner.config.proc_summary",
                                &[
                                    ("0", (proc.chance).to_string()),
                                    ("1", (proc.trigger.replace("on_", "")).to_string()),
                                    ("rank", (rank).to_string()),
                                    ("2", (base.name).to_string()),
                                ],
                            ),
                        ),
                    );
                }
            }
        }
        rows.extend(
            item_procs
                .into_iter()
                .map(|(key, (name, detail))| (key, name, detail)),
        );
        let mut card = panel_with_trailing(
            "config-procs",
            tr("planner.common.procs"),
            count_badge(
                "proc-count",
                rows.iter()
                    .filter(|(key, _, _)| snapshot.proc_toggles.get(key).copied().unwrap_or(false))
                    .count(),
                Some(rows.len()),
                cx,
            ),
            cx,
        )
        .child(help(tr("planner.config.procs_help"), cx));
        if rows.is_empty() {
            return card.child(empty(tr("planner.config.no_procs"), cx));
        }
        card = card.child(
            div()
                .mb_3()
                .flex()
                .items_center()
                .justify_between()
                .child(label(
                    "kills-per-second-label",
                    tr("planner.config.kills_sec"),
                    cx.global::<TooltipTheme>().faint,
                ))
                .child(self.number(NumberField::Kills, tr("planner.config.kills_second"), cx)),
        );
        for (key, name, detail) in rows {
            let checked = snapshot.proc_toggles.get(&key).copied().unwrap_or(false);
            card = card.child(
                tile(checked, cx)
                    .mb_2()
                    .child(
                        Checkbox::new(SharedString::from(format!("config-proc-{key}")))
                            .checked(checked)
                            .label(name)
                            .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                this.edit(cx, |snapshot| {
                                    snapshot.proc_toggles.insert(key.clone(), *checked);
                                })
                            })),
                    )
                    .child(
                        div()
                            .pl_6()
                            .text_size(rems(10. / 13.))
                            .text_color(cx.global::<TooltipTheme>().faint)
                            .child(detail),
                    ),
            );
        }
        card
    }

    fn dynamic_overrides(&self, cx: &Context<Self>) -> Div {
        let snapshot = self.session.read(cx).snapshot();
        let mut content = div().flex().flex_col().gap_4();
        let mut stacks = Vec::new();
        for stack in &data::game_config().stack_types {
            let max = self
                .tree
                .read(cx)
                .performance()
                .and_then(|result| result.current.stats.get(&stack.max_stat).copied())
                .map(|v| v.1.floor().max(0.) as u32)
                .unwrap_or(0);
            if max > 0 {
                stacks.push((stack, max))
            }
        }
        if !stacks.is_empty() {
            content = content.child(
                panel_with_trailing(
                    "config-stacks",
                    tr("planner.config.combat_stacks"),
                    count_badge(
                        "stack-count",
                        stacks
                            .iter()
                            .filter(|(stack, max)| {
                                snapshot
                                    .stack_counts
                                    .get(&stack.key)
                                    .copied()
                                    .unwrap_or(*max)
                                    < *max
                            })
                            .count(),
                        None,
                        cx,
                    ),
                    cx,
                )
                .child(help(tr("planner.config.stacks_help"), cx))
                .children(stacks.into_iter().map(|(stack, max)| {
                    tile(false, cx)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(stack.name.clone())
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(self.number(
                                    NumberField::Stack(stack.key.clone()),
                                    trf(
                                        "planner.config.stack_count",
                                        &[("0", (stack.name).to_string())],
                                    ),
                                    cx,
                                ))
                                .child(format!("/ {max}")),
                        )
                })),
            );
        }
        let skills = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""));
        let kinds = [
            ("sentry", tr("planner.config.sentry")),
            ("summon", tr("planner.config.summon")),
            ("guardian", tr("planner.config.guardian")),
        ]
        .into_iter()
        .filter(|(_, tag)| {
            skills.iter().any(|skill| {
                snapshot.skill_ranks.get(&skill.id).copied().unwrap_or(0) > 0
                    && hsplanner_engine::calc::subskill::effective_skill_tags(
                        &skill.id,
                        skill.tags.as_deref().unwrap_or(&[]),
                        &snapshot.subskill_ranks,
                    )
                    .iter()
                    .any(|t| t == tag)
            })
        })
        .collect::<Vec<_>>();
        if !kinds.is_empty() {
            content = content.child(
                panel("config-entity-rate", tr("planner.config.entity_rate"), cx)
                    .child(help(tr("planner.config.entity_help"), cx))
                    .children(kinds.into_iter().map(|(key, name)| {
                        div()
                            .mb_2()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(trf(
                                "planner.config.entity_per_second",
                                &[("name", (name).to_string())],
                            ))
                            .child(self.number(
                                NumberField::Entity(key.into()),
                                trf(
                                    "planner.config.entity_attacks",
                                    &[("name", (name).to_string())],
                                ),
                                cx,
                            ))
                    })),
            );
        }
        content
    }

    fn projectiles(&self, small: bool, cx: &Context<Self>) -> Option<Div> {
        let snapshot = self.session.read(cx).snapshot();
        let skills = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""))
            .iter()
            .filter(|skill| {
                skill.kind == SkillKind::Active
                    && (skill.damage_formula.is_some()
                        || skill
                            .damage_per_rank
                            .as_ref()
                            .is_some_and(|r| !r.is_empty()))
                    && (snapshot.skill_projectiles.contains_key(&skill.id)
                        || hsplanner_engine::calc::subskill::effective_skill_tags(
                            &skill.id,
                            skill.tags.as_deref().unwrap_or(&[]),
                            &snapshot.subskill_ranks,
                        )
                        .iter()
                        .any(|tag| tag == "Projectile"))
            })
            .collect::<Vec<_>>();
        if skills.is_empty() {
            return None;
        }
        let p = cx.global::<TooltipTheme>();
        Some(
            panel_with_trailing(
                "config-projectiles",
                tr("planner.config.projectiles"),
                count_badge(
                    "projectile-count",
                    skills
                        .iter()
                        .filter(|skill| snapshot.skill_projectiles.contains_key(&skill.id))
                        .count(),
                    None,
                    cx,
                ),
                cx,
            )
            .child(help(tr("planner.config.projectiles_help"), cx))
            .child(
                div()
                    .grid()
                    .grid_cols(if small { 2 } else { 1 })
                    .gap_2()
                    .children(skills.into_iter().map(|skill| {
                        let overridden = snapshot
                            .skill_projectiles
                            .get(&skill.id)
                            .is_some_and(|value| *value != skill.base_projectiles.unwrap_or(1));
                        let learned = snapshot.skill_ranks.get(&skill.id).copied().unwrap_or(0) > 0;
                        tile(overridden, cx)
                            .px_2p5()
                            .when(!learned && !overridden, |view| {
                                view.opacity(0.6).border_color(p.border)
                            })
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(skill_caption(skill, 28.).text_color(if overridden {
                                p.accent_hot
                            } else {
                                p.text
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(self.number(
                                        NumberField::Projectile(skill.id.clone()),
                                        trf(
                                            "planner.config.skill_projectiles",
                                            &[("0", (skill.name).to_string())],
                                        ),
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .font_family(theme::MONO_FONT_FAMILY)
                                            .text_size(rems(10. / 13.))
                                            .text_color(p.faint)
                                            .child("×"),
                                    ),
                            )
                    })),
            ),
        )
    }
}

impl Render for ConfigView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let logical_width = f32::from(window.viewport_size().width)
            / self.session.read(cx).state().settings.ui_zoom;
        let wide = logical_width >= 1280.;
        let small = logical_width >= 640.;
        let scroll = self.scroll.clone();
        let weak = cx.entity().downgrade();
        let had_vertical_scroll = self.has_vertical_scroll;
        div()
            .on_children_prepainted(move |_, _, cx| {
                let has_vertical_scroll = scroll.max_offset().y > px(0.);
                if has_vertical_scroll != had_vertical_scroll {
                    let weak = weak.clone();
                    cx.defer(move |cx| {
                        let _ = weak.update(cx, |view, cx| {
                            if view.has_vertical_scroll != has_vertical_scroll {
                                view.has_vertical_scroll = has_vertical_scroll;
                                cx.notify();
                            }
                        });
                    });
                }
            })
            .id("configuration")
            .track_focus(&self.focus)
            .size_full()
            .min_h_0()
            .bg(cx.global::<TooltipTheme>().background)
            .scrollbar_width(rems(if self.has_vertical_scroll {
                10. / 13.
            } else {
                0.
            }))
            .child(
                div()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_8()
                    .child(section_heading(
                        "config-heading",
                        tr("planner.config.setup"),
                        tr("planner.config.title"),
                        cx,
                    ))
                    .when_some(self.error.clone(), |view, error| {
                        view.child(
                            div()
                                .text_color(cx.global::<TooltipTheme>().negative)
                                .child(error),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(group_heading(
                                "config-character-heading",
                                tr("planner.common.character"),
                                tr("planner.config.character_help"),
                                cx,
                            ))
                            .child(self.basics(logical_width, window, cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(group_heading(
                                "config-combat-heading",
                                tr("planner.config.combat_title"),
                                tr("planner.config.combat_help"),
                                cx,
                            ))
                            .child(
                                div()
                                    .grid()
                                    .grid_cols(if wide { 2 } else { 1 })
                                    .gap_4()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_4()
                                            .child(self.buffs(cx))
                                            .child(self.aura(cx))
                                            .child(self.procs(cx))
                                            .when_some(self.blessings(cx), |view, panel| {
                                                view.child(panel)
                                            })
                                            .child(self.dynamic_overrides(cx)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_4()
                                            .child(self.conditions(true, small, cx))
                                            .child(self.conditions(false, small, cx))
                                            .child(self.resistances(small, cx))
                                            .when_some(
                                                self.projectiles(small, cx),
                                                |view, panel| view.child(panel),
                                            )
                                            .child(self.custom_stats(cx)),
                                    ),
                            ),
                    ),
            )
            .page_scroll(&self.scroll)
            .overflow_y_scroll()
    }
}

fn number_text(
    snapshot: &BuildSnapshot,
    field: &NumberField,
    performance: Option<&hsplanner_engine::calc::build::BuildPerformance>,
) -> String {
    match field {
        NumberField::Level => snapshot.level.to_string(),
        NumberField::SubskillPoints => snapshot.subskill_point_budget().to_string(),
        NumberField::Resistance(key) => snapshot
            .enemy_resistances
            .get(key)
            .map(ToString::to_string)
            .unwrap_or_default(),
        NumberField::Kills => snapshot.kills_per_sec.to_string(),
        NumberField::Entity(key) => snapshot
            .entity_rates
            .get(key)
            .copied()
            .unwrap_or(1.)
            .to_string(),
        NumberField::Stack(key) => snapshot
            .stack_counts
            .get(key)
            .copied()
            .or_else(|| {
                data::game_config()
                    .stack_types
                    .iter()
                    .find(|stack| &stack.key == key)
                    .and_then(|stack| {
                        performance.and_then(|result| result.stats.get(&stack.max_stat))
                    })
                    .map(|value| value.1.floor().max(0.) as u32)
            })
            .map(|value| value.to_string())
            .unwrap_or_default(),
        NumberField::Projectile(key) => snapshot
            .skill_projectiles
            .get(key)
            .copied()
            .or_else(|| {
                data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""))
                    .iter()
                    .find(|skill| &skill.id == key)
                    .map(|skill| skill.base_projectiles.unwrap_or(1))
            })
            .map(|value| value.to_string())
            .unwrap_or_default(),
    }
}

fn serialize_custom_stats(stats: &[CustomStat]) -> String {
    stats
        .iter()
        .map(|stat| {
            let name = data::game_config()
                .stats
                .iter()
                .find(|def| def.key == stat.stat_key)
                .map(|def| def.name.as_str())
                .unwrap_or(&stat.stat_key);
            format!("{} {name}", stat.value)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_custom_text(text: &str) -> (Vec<CustomStat>, Vec<String>) {
    let mut stats = Vec::new();
    let mut issues = Vec::new();
    for (ix, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_lowercase();
        let matched = data::game_config()
            .stats
            .iter()
            .filter(|def| !def.item_only.unwrap_or(false) && !def.skill_scoped.unwrap_or(false))
            .find_map(|def| {
                [def.name.as_str(), def.key.as_str()]
                    .into_iter()
                    .find_map(|name| {
                        let suffix = format!(" {}", name.to_lowercase());
                        lower
                            .strip_suffix(&suffix)
                            .filter(|value| parse_custom_stat_value(value).is_some())
                            .map(|value| {
                                (
                                    def.key.clone(),
                                    value.split_whitespace().collect::<String>(),
                                )
                            })
                    })
            });
        if let Some((stat_key, value)) = matched {
            stats.push(CustomStat { stat_key, value })
        } else {
            issues.push(trf(
                "planner.config.invalid_custom_line",
                &[("0", (ix + 1).to_string())],
            ))
        }
    }
    (stats, issues)
}

fn allocation_step(event: &ClickEvent, available: u32) -> u32 {
    let modifiers = event.modifiers();
    if modifiers.shift && (modifiers.control || modifiers.platform) {
        available
    } else if modifiers.shift {
        5.min(available)
    } else {
        1.min(available)
    }
}
fn help(text: &str, cx: &App) -> Div {
    div()
        .mb_3()
        .text_size(rems(12. / 13.))
        .line_height(relative(1.625))
        .text_color(cx.global::<TooltipTheme>().muted)
        .child(text.to_owned())
}
fn count_badge(id: impl Into<ElementId>, count: usize, total: Option<usize>, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let text = match total {
        Some(total) => trf(
            "planner.config.active_count",
            &[
                ("count", (count).to_string()),
                ("total", (total).to_string()),
            ],
        ),
        None => trf(
            if count == 1 {
                "planner.config.override_one"
            } else {
                "planner.config.override_count"
            },
            &[("count", count.to_string())],
        ),
    };
    label(id, text, if count > 0 { p.accent_hot } else { p.faint })
}
fn condition_color(element: &str, cx: &App) -> Hsla {
    let role = match element {
        "fire" => "red",
        "cold" => "blue",
        "lightning" => "orange",
        "poison" => "green",
        "arcane" => "purple",
        _ => "armor",
    };
    theme::stat_color(role, cx)
}
fn skill_caption(skill: &hsplanner_engine::calc::types::SkillSpec, size: f32) -> Div {
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap_2()
        .child(div().size(rems(size / 13.)).flex_shrink_0().when_some(
            crate::skills::skill_icon(&skill.class_id, &skill.id),
            |view, image| view.child(img(image).size_full().object_fit(ObjectFit::Contain)),
        ))
        .child(div().min_w_0().child(skill.name.clone()))
}
fn empty(text: &str, cx: &App) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(rems(12. / 13.))
        .text_color(cx.global::<TooltipTheme>().muted)
        .child(text.to_owned())
}
fn label(id: impl Into<ElementId>, text: impl Into<String>, color: Hsla) -> Div {
    div()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(rems(10. / 13.))
        .text_color(color)
        .child(TooltipText::new(id, text.into().to_uppercase(), 0.14))
}
fn tile(selected: bool, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            p.accent_deep
        } else {
            p.border_strong
        })
        .px_3()
        .py_2()
        .bg(theme::config_tile_surface(selected, cx))
        .shadow(vec![BoxShadow {
            color: p.shadow.opacity(0.4),
            offset: point(px(0.), px(1.)),
            blur_radius: px(2.),
            spread_radius: px(0.),
            inset: true,
        }])
}
fn group_heading(id: &'static str, title: &str, subtitle: &str, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let marker_color = p.accent_hot;
    div()
        .pt_1()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    // Tauri's rotated h-1.5 square does not contribute a
                    // body-font line box to the 11px heading row.
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, window, _| {
                            let center = bounds.center();
                            let radius = bounds.size.width / std::f32::consts::SQRT_2;
                            let mut path = PathBuilder::fill();
                            path.move_to(point(center.x, center.y - radius));
                            path.line_to(point(center.x + radius, center.y));
                            path.line_to(point(center.x, center.y + radius));
                            path.line_to(point(center.x - radius, center.y));
                            path.close();
                            if let Ok(path) = path.build() {
                                window.paint_path(path, marker_color);
                            }
                        },
                    )
                    .size_1p5()
                    .flex_shrink_0(),
                )
                .child(
                    div()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(rems(11. / 13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.accent_hot.opacity(0.8))
                        .child(TooltipText::new(id, title.to_uppercase(), 0.2)),
                )
                .child(div().flex_1().h(px(1.)).bg(linear_gradient(
                    90.,
                    linear_color_stop(p.accent.opacity(0.35), 0.),
                    linear_color_stop(p.accent.opacity(0.), 1.),
                ))),
        )
        .child(
            div()
                .mt_1p5()
                .pl_4()
                .text_size(rems(12. / 13.))
                .line_height(relative(1.625))
                .text_color(p.muted)
                .child(subtitle.to_owned()),
        )
}

const ENEMY_CONDITIONS: &[(&str, &str, &str)] = &[
    ("burning", "planner.condition.burning", "fire"),
    ("poisoned", "planner.condition.poisoned", "poison"),
    ("frozenbite", "planner.condition.frostbitten", "cold"),
    ("stunned", "planner.condition.stunned", ""),
    ("bleeding", "planner.condition.bleeding", ""),
    ("shocked", "planner.condition.stasis", "lightning"),
    ("deep_frozen", "planner.condition.deep_frozen", "cold"),
    ("shadow_burn", "planner.condition.shadow_burned", "arcane"),
    ("frozen", "planner.condition.frozen", "cold"),
    ("slow", "planner.condition.slowed", ""),
    ("low_life", "planner.condition.low_life", ""),
    ("serrated_chains", "planner.condition.serrated_chains", ""),
    (
        "lightning_break",
        "planner.condition.lightning_break",
        "lightning",
    ),
    ("fire_break", "planner.condition.fire_break", "fire"),
    ("cold_break", "planner.condition.cold_break", "cold"),
    ("arcane_break", "planner.condition.arcane_break", "arcane"),
    ("poison_break", "planner.condition.poison_break", "poison"),
    ("is_boss", "planner.condition.boss", ""),
];
const PLAYER_CONDITIONS: &[(&str, &str, &str)] = &[
    ("crit_chance_below_40", "planner.condition.crit_below", ""),
    ("life_below_40", "planner.condition.life_below", ""),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn numeric_draft_survives_notifications_between_keystrokes_and_blur() {
        let mut draft = NumberDraft {
            applied: "10".into(),
        };
        // Selecting all and beginning a negative number must survive both a
        // result notification and the focus loss preceding its Blur handler.
        assert!(!draft.reconcile("-", "10", true, false));
        assert!(!draft.reconcile("-2", "10", true, false));
        assert!(!draft.reconcile("-20", "10", false, false));
        assert!(draft.is_dirty("-20"));
        // The successful commit writes its normalized value and clears dirtiness.
        draft.applied = "-20".into();
        assert!(!draft.is_dirty("-20"));
        assert!(draft.reconcile("-20", "15", false, false));
    }

    #[::core::prelude::v1::test]
    fn numeric_draft_defers_clean_focused_updates_without_turning_them_into_edits() {
        let mut draft = NumberDraft {
            applied: "10".into(),
        };
        assert!(!draft.reconcile("10", "20", true, false));
        assert!(!draft.reconcile("10", "30", true, false));
        assert!(!draft.is_dirty("10"));
        assert!(draft.reconcile("10", "30", false, false));
        assert!(!draft.is_dirty("30"));
    }

    #[::core::prelude::v1::test]
    fn changed_owner_replaces_even_focused_dirty_numbers() {
        let mut draft = NumberDraft {
            applied: "10".into(),
        };
        assert!(draft.reconcile("-20", "5", true, true));
        assert!(!draft.is_dirty("5"));
        assert!(draft.reconcile("5", "7", false, false));
    }

    #[::core::prelude::v1::test]
    fn loadout_switches_only_replace_numeric_drafts_owned_by_that_loadout() {
        use hsplanner_build::session::WorkspaceState;

        let mut session = Session::new(WorkspaceState::default());
        let original = ConfigDocument::from_draft(session.draft());
        session
            .add_loadout(LoadoutKind::Gear, "Other gear")
            .unwrap();
        let gear = ConfigDocument::from_draft(session.draft());
        assert!(!original.number_changed(&gear, &NumberField::SubskillPoints));
        assert!(!original.number_changed(&gear, &NumberField::Level));

        session
            .add_loadout(LoadoutKind::Skills, "Other skills")
            .unwrap();
        let skills = ConfigDocument::from_draft(session.draft());
        assert!(gear.number_changed(&skills, &NumberField::SubskillPoints));
        assert!(!gear.number_changed(&skills, &NumberField::Level));
        assert!(!gear.number_changed(&skills, &NumberField::Projectile("charged_bolts".into())));
        assert_eq!(gear.build, skills.build); // Common custom stats keep their pending text.

        let mut budget = NumberDraft {
            applied: "20".into(),
        };
        assert!(budget.reconcile(
            "25",
            "20",
            true,
            gear.number_changed(&skills, &NumberField::SubskillPoints)
        ));
        let mut level = NumberDraft {
            applied: "10".into(),
        };
        assert!(!level.reconcile(
            "99",
            "10",
            true,
            gear.number_changed(&skills, &NumberField::Level)
        ));

        session.edit(|draft| draft.snapshot.set_class("stormweaver"));
        let class = ConfigDocument::from_draft(session.draft());
        assert!(skills.number_changed(&class, &NumberField::Projectile("charged_bolts".into())));
        assert!(!skills.number_changed(&class, &NumberField::Level));
    }

    #[::core::prelude::v1::test]
    fn custom_config_preserves_ranges_duplicates_and_valid_lines() {
        let (stats, issues) =
            parse_custom_text("+50 Life\n12 - 18 Cold Resistance\n3 life\nunknown");
        assert_eq!(
            stats
                .iter()
                .map(|stat| (stat.stat_key.as_str(), stat.value.as_str()))
                .collect::<Vec<_>>(),
            vec![("life", "+50"), ("cold_resistance", "12-18"), ("life", "3")]
        );
        assert_eq!(issues.len(), 1);
        assert!(issues[0].starts_with("line 4"));
    }

    #[::core::prelude::v1::test]
    fn custom_config_rejects_nonfinite_and_incomplete_numbers() {
        let (stats, issues) = parse_custom_text("NaN Life\ninf Life\n5- Life\n\n  ");
        assert!(stats.is_empty());
        assert_eq!(issues.len(), 3);
    }
}
