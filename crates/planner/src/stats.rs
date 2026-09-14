//! Read-only planner results, with view-local search, filters and source disclosures.
use crate::{TreeView, build_panel::format_range};
use gpui_kit::base::Selectable;
use gpui_kit::component::{
    Icon, IconName, Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::session::Session;
use hsplanner_engine::calc::{
    build::BuildPerformance,
    data,
    planner::{self, PlannerPerformance},
    skills::calculation::CalculationStep,
    stats::{ComputedStats, compute_stat_breakdown},
    types::SkillKind,
};
use hsplanner_ui::{
    components::{panel, panel_with_trailing},
    controls::{ButtonSize, ButtonTone, PlannerControl, command_button},
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};
use std::{collections::HashSet, sync::Arc};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Damage,
    Stats,
    Skills,
}
impl Filter {
    fn name(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Damage => "Damage",
            Self::Stats => "Stats",
            Self::Skills => "Skills",
        }
    }
}
// Virtualized page rows. Stat rows pair the two balanced columns so only visible
// pairs of the 669 definitions are laid out; the top block stays one measured item.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cell {
    Heading(Group),
    Stat(usize),
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Row {
    Header,
    Calculating,
    Top,
    StatsStart,
    StatsPair {
        left: Option<Cell>,
        right: Option<Cell>,
    },
    StatsEmpty,
    StatsEnd,
}
const LIST_OVERDRAW: Pixels = px(400.);
const TOP_ROW: usize = 2;

fn pair_columns([left, right]: [Vec<Cell>; 2]) -> Vec<Row> {
    if left.is_empty() && right.is_empty() {
        return vec![Row::StatsStart, Row::StatsEmpty, Row::StatsEnd];
    }
    let mut rows = vec![Row::StatsStart];
    let (mut left, mut right) = (left.into_iter(), right.into_iter());
    loop {
        let (left, right) = (left.next(), right.next());
        if left.is_none() && right.is_none() {
            break;
        }
        rows.push(Row::StatsPair { left, right });
    }
    rows.push(Row::StatsEnd);
    rows
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Group {
    Offense,
    Mitigation,
    Resistances,
    Resources,
    Skills,
    World,
    Other,
}
impl Group {
    fn title(self) -> &'static str {
        match self {
            Self::Offense => "Offense",
            Self::Mitigation => "Mitigation",
            Self::Resistances => "Resistances",
            Self::Resources => "Resources",
            Self::Skills => "Skill Bonuses",
            Self::World => "World & Loot",
            Self::Other => "Other",
        }
    }
    fn visible(self, filter: Filter) -> bool {
        filter == Filter::All
            || matches!(
                (self, filter),
                (Self::Offense, Filter::Damage)
                    | (
                        Self::Mitigation | Self::Resistances | Self::Resources,
                        Filter::Stats
                    )
                    | (Self::Skills, Filter::Skills)
            )
    }
}
const RESISTANCE_KEYS: &[&str] = &[
    "fire_resistance",
    "cold_resistance",
    "lightning_resistance",
    "poison_resistance",
    "arcane_resistance",
    "all_resistances",
    "max_fire_resistance",
    "max_cold_resistance",
    "max_lightning_resistance",
    "max_poison_resistance",
    "max_arcane_resistance",
    "max_all_resistances",
    "fire_absorption",
    "cold_absorption",
    "lightning_absorption",
    "poison_absorption",
    "arcane_absorption",
];
const RESOURCE_KEYS: &[&str] = &[
    "life",
    "mana",
    "increased_life",
    "increased_mana",
    "life_replenish",
    "life_replenish_pct",
    "mana_replenish",
    "mana_replenish_pct",
    "life_steal",
    "life_steal_rate",
    "life_steal_instant",
    "mana_steal",
    "mana_steal_rate",
    "life_per_kill",
    "mana_per_kill",
    "mana_cost_reduction",
    "mana_cost_paid_in_life",
    "damage_recouped_as_life",
    "damage_recouped_as_mana",
    "overflow_mana_recouped_as_life",
    "damage_drained_from_mana",
    "max_life_to_mana",
    "max_mana_to_life",
    "overflow_res_to_life",
    "life_replenish_flask",
    "damage_mitigated_flask",
];
const MITIGATION_KEYS: &[&str] = &[
    "defense",
    "enhanced_defense",
    "defense_vs_missiles",
    "damage_reduced",
    "all_damage_taken_reduced_pct",
    "physical_damage_reduction",
    "magic_damage_reduction",
    "magic_absorption",
    "damage_taken_reduced",
    "damage_mitigation",
    "damage_return",
    "damage_return_echo_chance",
    "magic_damage_taken_reduced",
    "stun_freeze_immunity",
    "block_chance",
    "dodge_chance",
    "dodge_spell_hits",
    "suppress_spell_hits",
    "faster_hit_recovery",
    "immune_duration",
    "cc_immune_no_dodge",
    "poison_length_reduced",
    "max_colossus_stacks",
    "max_combat_mitigation_stacks",
];
const SKILL_KEYS: &[&str] = &[
    "all_skills",
    "physical_skills",
    "arcane_skills",
    "cold_skills",
    "fire_skills",
    "poison_skills",
    "lightning_skills",
    "explosion_skills",
    "projectile_skills",
    "summon_skills",
];
const WORLD_KEYS: &[&str] = &[
    "movement_speed",
    "jumping_power",
    "light_radius",
    "experience_gain",
    "magic_find",
    "gold_find",
    "merchant_prices",
    "increased_all_attributes",
];
fn group(key: &str, category: &str) -> Group {
    if RESISTANCE_KEYS.contains(&key) {
        Group::Resistances
    } else if RESOURCE_KEYS.contains(&key) {
        Group::Resources
    } else if MITIGATION_KEYS.contains(&key) {
        Group::Mitigation
    } else if SKILL_KEYS.contains(&key) {
        Group::Skills
    } else if WORLD_KEYS.contains(&key) {
        Group::World
    } else if category == "offense" {
        Group::Offense
    } else {
        Group::Other
    }
}
fn units(value: f32) -> Rems {
    rems(value / 13.)
}
fn decimal(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}
fn decimal_range(value: (f64, f64)) -> String {
    if value.0 == value.1 {
        decimal(value.0)
    } else {
        format!("{}-{}", decimal(value.0), decimal(value.1))
    }
}
/// Presentation-only overview; the full engine trace remains available below it.
fn calculation_overview(step: &CalculationStep) -> Option<(&'static str, &'static str)> {
    let (label, operator) = match step.label() {
        "Base damage" => ("Base damage", ""),
        "Flat added" => ("Added damage", "+"),
        "Physical base" => ("Weapon & added damage", ""),
        "Synergy multiplier" => ("Synergies", "×"),
        "Increased skill damage multiplier" => ("Increased damage", "×"),
        "More skill damage multiplier" => ("More damage", "×"),
        "Attack damage multiplier" => ("Attack damage & synergies", "×"),
        "Skill weapon multiplier" => ("Skill scaling", "×"),
        "Crushing blow + armor break" => ("Crushing blow & armor break", "×"),
        "Deadly blow multiplier" => ("Deadly blow", "×"),
        "Extra damage multiplier" => ("Extra damage", "×"),
        "Enemy damage taken multiplier" => ("Enemy vulnerability", "×"),
        "Elemental break multiplier" => ("Elemental break", "×"),
        "Element resistance break multiplier" => ("Element resistance break", "×"),
        "Resistance multiplier" => ("Enemy resistance", "×"),
        "Hit damage" | "Physical hit" => ("Single hit", "="),
        "Average critical multiplier" => ("Critical average", "×"),
        "Multicast multiplier" => ("Multicast", "×"),
        "Projectiles" => ("Projectiles", "×"),
        "Average physical damage" => ("Average physical damage", "="),
        "Actions per second" | "Entity actions per second" => ("Actions per second", ""),
        "Entity count" => ("Entities", "×"),
        "Hits per cast" => ("Hits per cast", "×"),
        "Average hit DPS" => ("Hit DPS", ""),
        "Proc DPS" => ("Procs", "+"),
        "Ailment DPS" => ("Damage over time", "+"),
        "Execute multiplier" => ("Execute", "×"),
        _ => return None,
    };
    let neutral = match operator {
        "×" => step.value() == (1., 1.),
        "+" => step.value() == (0., 0.),
        _ => false,
    };
    (!neutral).then_some((label, operator))
}

fn heading(id: impl Into<ElementId>, label: &str, cx: &App) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .py_2()
        .font_family(theme::MONO_FONT_FAMILY)
        .text_size(units(9.))
        .text_color(cx.global::<TooltipTheme>().accent_deep)
        .child(TooltipText::new(id, label.to_uppercase(), 0.18))
        .child(
            div()
                .flex_1()
                .h(px(1.))
                .bg(cx.global::<TooltipTheme>().border),
        )
}
fn value_row(label: impl Into<SharedString>, value: impl Into<SharedString>, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .flex()
        .items_baseline()
        .justify_between()
        .gap_3()
        .py_1()
        .text_size(units(11.))
        .child(div().text_color(p.faint).child(label.into()))
        .child(
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_color(p.text)
                .child(value.into()),
        )
}

pub struct StatsView {
    session: Entity<Session>,
    tree: Entity<TreeView>,
    observed_performance: Option<Arc<PlannerPerformance>>,
    query: Entity<InputState>,
    filter: Filter,
    expanded: HashSet<String>,
    active: bool,
    calculation_revision: u64,
    revision: u64,
    skill_performance: Option<Arc<PlannerPerformance>>,
    task: Option<Task<()>>,
    list: ListState,
    rows: Vec<Row>,
    row_filter: Option<(String, Filter, bool)>,
    last_rem: Pixels,
    _subscriptions: Vec<Subscription>,
}
impl StatsView {
    pub fn new(
        session: Entity<Session>,
        tree: Entity<TreeView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search stats, attributes, or skills…")
        });
        let calculation_revision = session.read(cx).calculation_revision();
        let observed_performance = tree.read(cx).performance();
        let subscriptions = vec![
            cx.observe(&session, |this, _, cx| {
                let revision = this.session.read(cx).calculation_revision();
                if revision != this.calculation_revision {
                    this.calculation_revision = revision;
                    this.skill_performance = None;
                    if this.active {
                        this.refresh_skills(cx);
                    }
                }
                this.remeasure_dynamic_rows();
                cx.notify();
            }),
            cx.observe(&tree, |this, tree, cx| {
                let next = tree.read(cx).performance();
                if this.observed_performance.as_ref().map(Arc::as_ptr)
                    == next.as_ref().map(Arc::as_ptr)
                {
                    return;
                }
                this.observed_performance = next;
                this.remeasure_dynamic_rows();
                cx.notify();
            }),
            cx.subscribe(&query, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
        ];
        Self {
            session,
            tree,
            observed_performance,
            query,
            filter: Filter::All,
            expanded: HashSet::new(),
            active: false,
            calculation_revision,
            revision: 0,
            skill_performance: None,
            task: None,
            list: ListState::new(0, ListAlignment::Top, LIST_OVERDRAW),
            rows: Vec::new(),
            row_filter: None,
            last_rem: Pixels::ZERO,
            _subscriptions: subscriptions,
        }
    }
    fn row_for_key(&self, key: &str) -> usize {
        let stats = &data::game_config().stats;
        self.rows
            .iter()
            .position(|row| match row {
                Row::StatsPair { left, right } => [left, right]
                    .into_iter()
                    .any(|cell| matches!(cell, Some(Cell::Stat(ix)) if stats[*ix].key == key)),
                _ => false,
            })
            .unwrap_or(TOP_ROW)
    }
    // Only the top block and expanded disclosures change height with new results.
    fn remeasure_dynamic_rows(&self) {
        let mut rows: Vec<usize> = self
            .expanded
            .iter()
            .map(|key| self.row_for_key(key))
            .chain(std::iter::once(TOP_ROW))
            .collect();
        rows.sort_unstable();
        rows.dedup();
        for row in rows.into_iter().filter(|row| *row < self.list.item_count()) {
            self.list.splice(row..row + 1, 1);
        }
    }
    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.active == active {
            return;
        }
        self.active = active;
        if active && self.skill_performance.is_none() {
            self.refresh_skills(cx);
        } else if !active {
            self.revision = self.revision.wrapping_add(1);
            self.task = None;
        }
        cx.notify();
    }
    fn refresh_skills(&mut self, cx: &mut Context<Self>) {
        self.revision = self.revision.wrapping_add(1);
        let revision = self.revision;
        let snapshot = self.session.read(cx).snapshot();
        let mut input = snapshot.planner_input();
        input.active_skill_ids =
            data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""))
                .iter()
                .filter(|skill| skill.kind == SkillKind::Active)
                .map(|skill| skill.id.clone())
                .collect();
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { Arc::new(planner::evaluate(&input)) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.revision == revision && this.active {
                    this.skill_performance = Some(result);
                    this.task = None;
                    this.remeasure_dynamic_rows();
                    cx.notify();
                }
            });
        }));
    }
    fn toggle(&mut self, key: &str, cx: &mut Context<Self>) {
        if !self.expanded.remove(key) {
            self.expanded.insert(key.into());
        }
        let row = self.row_for_key(key);
        if row < self.list.item_count() {
            self.list.splice(row..row + 1, 1);
        }
        cx.notify();
    }
    fn stat(
        &self,
        key: &str,
        label: &str,
        percent: bool,
        computed: &ComputedStats,
        cx: &Context<Self>,
    ) -> Div {
        let p = cx.global::<TooltipTheme>();
        let raw = computed.stats.get(key).copied().unwrap_or_default();
        let value = computed.stats_combined.get(key).copied().unwrap_or(raw);
        let zero = value.0.abs() < 0.0001 && value.1.abs() < 0.0001;
        let shown = if zero {
            "—".into()
        } else {
            crate::source_breakdown::format_source(value, percent)
        };
        let mut breakdown = compute_stat_breakdown(&computed.stat_sources, key, Some(value));
        breakdown.stat_name = label.to_owned();
        let button = Button::new(SharedString::from(format!("stat-{key}")))
            .ghost()
            .w_full()
            .h_auto()
            .py_1()
            .px_0()
            .rounded_sm()
            .accessibility_label(format!("{label} sources"))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .text_size(units(13.))
                    .text_color(if zero { p.faint } else { p.text })
                    .child(div().flex_1().min_w_0().child(label.to_owned()))
                    .child(
                        div()
                            .text_color(if zero { p.faint } else { p.accent_hot })
                            .font_family(theme::MONO_FONT_FAMILY)
                            .child(shown),
                    ),
            );
        div().child(crate::source_breakdown::trigger(
            SharedString::from(format!("stat-sources-{key}-{}", self.calculation_revision)),
            button,
            breakdown,
            self.session.clone(),
        ))
    }
    fn attributes(
        &self,
        result: &PlannerPerformance,
        query: &str,
        columns: u16,
        cx: &Context<Self>,
    ) -> Option<Div> {
        if !matches!(self.filter, Filter::All | Filter::Stats) {
            return None;
        }
        let p = cx.global::<TooltipTheme>();
        let attributes = data::game_config()
            .attributes
            .iter()
            .filter(|attribute| query.is_empty() || attribute.name.to_lowercase().contains(query))
            .collect::<Vec<_>>();
        if attributes.is_empty() {
            return None;
        }
        let mut strip = div()
            .grid()
            .grid_cols(columns)
            .gap(px(1.))
            .overflow_hidden()
            .rounded_sm()
            .border_1()
            .border_color(p.border)
            .bg(p.border);
        for attribute in attributes {
            let key = format!("attribute-{}", attribute.key);
            let value = result
                .computed
                .attributes
                .get(&attribute.key)
                .copied()
                .unwrap_or_default();
            let button = Button::new(SharedString::from(key))
                .ghost()
                .min_w_0()
                .w_full()
                .h_auto()
                .px_3()
                .py_2p5()
                .rounded_none()
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(p.panel_secondary, 0.),
                    linear_color_stop(p.background, 1.),
                ))
                .accessibility_label(format!("{} sources", attribute.name))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(units(9.))
                                .text_color(p.faint)
                                .child(attribute.name.to_uppercase()),
                        )
                        .child(
                            div()
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_size(units(15.))
                                .text_color(p.text)
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format_range(value, false)),
                        ),
                );
            let mut breakdown = compute_stat_breakdown(
                &result.computed.attribute_sources,
                &attribute.key,
                Some(value),
            );
            breakdown.stat_name = attribute.name.clone();
            strip = strip.child(crate::source_breakdown::trigger(
                SharedString::from(format!(
                    "attribute-sources-{}-{}",
                    attribute.key, self.calculation_revision
                )),
                button,
                breakdown,
                self.session.clone(),
            ));
        }
        Some(panel("attributes-title", "Attributes", cx).child(strip))
    }
    fn calculation_section(
        &self,
        id: &str,
        title: &str,
        steps: &[CalculationStep],
        value: &BuildPerformance,
        cx: &Context<Self>,
    ) -> Div {
        let p = cx.global::<TooltipTheme>();
        let collapsed_key = format!("{id}-collapsed");
        let open = !self.expanded.contains(&collapsed_key);
        let details_key = format!("{id}-all-details");
        let all_details = self.expanded.contains(&details_key);
        let result = steps.iter().rev().find(|step| {
            matches!(
                step.label(),
                "Average damage per cast" | "Average damage per swing" | "Combined DPS"
            )
        });
        let scale = &self.session.read(cx).state().settings.number_scale;
        let compact = |range| hsplanner_ui::numbers::compact_range(range, scale);
        let header = Button::new(SharedString::from(format!("{id}-header")))
            .ghost()
            .w_full()
            .h_auto()
            .px_3()
            .py_2p5()
            .accessibility_label(format!(
                "{} {title}",
                if open { "Collapse" } else { "Expand" }
            ))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(if open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size_3()
                        .text_color(p.muted),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(units(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.text)
                            .child(title.to_owned()),
                    )
                    .children(result.map(|step| {
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_size(units(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.accent_hot)
                            .child(compact(step.value()))
                    })),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.toggle(&collapsed_key, cx)));
        let mut section = div()
            .w_full()
            .min_w_0()
            .border_1()
            .border_color(p.border)
            .rounded_md()
            .child(header);
        if !open {
            return section;
        }
        let mut body = div().px_3().pb_2().flex().flex_col();
        let mut occurrences = std::collections::HashMap::new();
        for step in steps {
            let occurrence = occurrences.entry(step.label()).or_insert(0usize);
            let row_key = format!("{id}-{}-{occurrence}", step.label());
            *occurrence += 1;
            let overview = calculation_overview(step);
            if !all_details && overview.is_none() {
                continue;
            }
            let label = if all_details {
                step.stat_key()
                    .map(crate::skill_details::stat_name)
                    .unwrap_or_else(|| step.label().to_owned())
            } else {
                overview.unwrap().0.to_owned()
            };
            let operator = if all_details { "" } else { overview.unwrap().1 };
            let shown = if operator == "×"
                || step.label().contains("multiplier")
                || step.label().ends_with('%')
                || matches!(
                    step.label(),
                    "Actions per second" | "Entity actions per second" | "Attacks per second"
                ) {
                decimal_range(step.value())
            } else {
                compact(step.value())
            };
            let expanded = self.expanded.contains(&row_key);
            let row = div()
                .w_full()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(units(12.))
                        .flex_none()
                        .text_color(p.faint)
                        .child(operator),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(units(12.))
                        .text_color(p.muted)
                        .child(label.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(12.))
                        .text_color(p.text)
                        .child(shown),
                )
                .child(
                    Icon::new(if step.stat_key().is_some() {
                        IconName::ChevronRight
                    } else if expanded {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .size_3()
                    .text_color(p.faint),
                );
            let button = Button::new(SharedString::from(format!("{row_key}-button")))
                .ghost()
                .w_full()
                .h_auto()
                .px_0()
                .py_1p5()
                .child(row);
            if let Some(key) = step.stat_key() {
                let sources = std::collections::HashMap::from([(
                    key.to_owned(),
                    value
                        .calculation_sources()
                        .get(key)
                        .cloned()
                        .unwrap_or_default(),
                )]);
                let mut breakdown = compute_stat_breakdown(&sources, key, Some(step.value()));
                breakdown.stat_name = label.clone();
                body = body.child(crate::source_breakdown::trigger(
                    SharedString::from(format!("{row_key}-sources")),
                    button.accessibility_label(format!("{label} sources")),
                    breakdown,
                    self.session.clone(),
                ));
            } else {
                let toggle = row_key.clone();
                body = body.child(
                    button
                        .accessibility_label(format!(
                            "{} {label} calculation",
                            if expanded { "Hide" } else { "Show" }
                        ))
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle(&toggle, cx))),
                );
                if expanded {
                    body = body.child(
                        div()
                            .ml_5()
                            .mb_2()
                            .pl_3()
                            .py_2()
                            .pr_2()
                            .border_l_2()
                            .border_color(p.accent_deep)
                            .bg(p.panel_secondary)
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(units(12.))
                                    .text_color(p.muted)
                                    .whitespace_normal()
                                    .child(step.expression().to_owned()),
                            )
                            .child(
                                div()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .text_size(units(12.))
                                    .text_color(p.text)
                                    .child(format!(
                                        "= {}",
                                        hsplanner_engine::calc::skills::calculation::display_range(
                                            step.value()
                                        )
                                    )),
                            ),
                    );
                }
            }
        }
        body = body.child(
            div()
                .mt_1()
                .pt_1()
                .border_t_1()
                .border_color(p.border)
                .flex()
                .items_center()
                .child(
                    command_button(
                        SharedString::from(format!("{id}-details")),
                        if all_details {
                            "Show summary"
                        } else {
                            "Show all details"
                        },
                        ButtonTone::Ghost,
                        ButtonSize::Small,
                        cx,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle(&details_key, cx))),
                ),
        );
        section = section.child(body);
        section
    }

    fn skill_breakdown(&self, id: &str, value: &BuildPerformance, cx: &Context<Self>) -> Div {
        let mut content = div().mt_3().flex().flex_col().gap_2();
        if let Some(damage) = &value.damage {
            content = content.child(self.calculation_section(
                &format!("{id}-element"),
                if value.attack_damage.is_some() {
                    "Elemental damage per cast"
                } else {
                    "Damage per cast"
                },
                damage.calculation(),
                value,
                cx,
            ));
        }
        if let Some(damage) = &value.attack_damage {
            content = content.child(self.calculation_section(
                &format!("{id}-physical"),
                "Damage per swing",
                damage.calculation(),
                value,
                cx,
            ));
        }
        content.child(self.calculation_section(
            &format!("{id}-dps"),
            "Damage per second",
            value.calculation(),
            value,
            cx,
        ))
    }
    fn hero(&self, id: &str, value: &BuildPerformance, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let scale = &self.session.read(cx).state().settings.number_scale;
        let format_range = |value, percent| {
            if percent {
                format_range(value, true)
            } else {
                hsplanner_ui::numbers::compact_range(value, scale)
            }
        };
        let skill = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""))
            .iter()
            .find(|skill| skill.id == id);
        let mut metrics: Vec<(&str, String)> = Vec::new();
        let (label, headline) = if let Some(d) = &value.attack_damage {
            metrics.extend([
                (
                    "Hit damage",
                    format_range(
                        (d.combined_hit_min as f64, d.combined_hit_max as f64),
                        false,
                    ),
                ),
                (
                    "Attack damage",
                    format_range((d.weapon_damage_pct_min, d.weapon_damage_pct_max), true),
                ),
                (
                    "Physical hit",
                    format_range(
                        (d.physical_hit_min as f64, d.physical_hit_max as f64),
                        false,
                    ),
                ),
                (
                    "Elemental hit",
                    if d.poison_hit_max > 0 {
                        format_range((d.poison_hit_min as f64, d.poison_hit_max as f64), false)
                    } else {
                        "—".into()
                    },
                ),
            ]);
            (
                "AVERAGE HIT",
                format_range(
                    (d.combined_avg_min as f64, d.combined_avg_max as f64),
                    false,
                ),
            )
        } else if let Some(d) = &value.damage {
            metrics.extend([
                (
                    "Hit damage",
                    format_range((d.hit_min as f64, d.hit_max as f64), false),
                ),
                (
                    "Crit damage",
                    if d.crit_chance > 0. {
                        format_range((d.crit_min as f64, d.crit_max as f64), false)
                    } else {
                        "—".into()
                    },
                ),
                (
                    "Crit chance",
                    if d.crit_chance > 0. {
                        format_range((d.crit_chance, d.crit_chance), true)
                    } else {
                        "—".into()
                    },
                ),
                (
                    "Crit multi",
                    if d.crit_chance > 0. {
                        format!(
                            "+{}",
                            format_range((d.crit_damage_pct, d.crit_damage_pct), true)
                        )
                    } else {
                        "—".into()
                    },
                ),
            ]);
            if d.crit_chance > 0. {
                (
                    "AVERAGE HIT",
                    format_range((d.avg_min as f64, d.avg_max as f64), false),
                )
            } else {
                (
                    "HIT DAMAGE",
                    format_range((d.hit_min as f64, d.hit_max as f64), false),
                )
            }
        } else {
            ("HIT DAMAGE", "—".into())
        };
        let mut summary = div()
            .flex()
            .flex_wrap()
            .gap_3()
            .mt_2()
            .text_size(units(10.))
            .text_color(p.muted);
        if let Some(cost) = value.skill_costs.get(id) {
            if let Some(range) = cost.mana_min.zip(cost.mana_max) {
                summary = summary.child(format!("{} mana", decimal_range(range)));
            }
            if let Some(range) = cost.cast_rate_min.zip(cost.cast_rate_max) {
                summary = summary.child(format!("{} casts/s", decimal_range(range)));
            }
        }
        let mut tags = div().flex().flex_wrap().gap_1();
        if let Some(skill) = skill {
            for tag in hsplanner_engine::calc::subskill::effective_skill_tags(
                &skill.id,
                skill.tags.as_deref().unwrap_or_default(),
                &snapshot.subskill_ranks,
            ) {
                tags = tags.child(
                    div()
                        .px_1()
                        .py(units(1.))
                        .border_1()
                        .border_color(p.accent_deep.opacity(0.5))
                        .text_color(p.accent_hot)
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(9.))
                        .child(tag.to_uppercase()),
                );
            }
        }
        let leading = div()
            .flex_1()
            .min_w_0()
            .p_4()
            .border_r_1()
            .border_color(p.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(units(9.))
                    .text_color(p.faint)
                    .font_family(theme::MONO_FONT_FAMILY)
                    .child(label)
                    .when_some(
                        skill.and_then(|skill| skill.damage_type.as_deref()),
                        |v, kind| {
                            v.child(
                                div()
                                    .px_1()
                                    .border_1()
                                    .border_color(theme::damage_color(kind).opacity(0.5))
                                    .text_color(theme::damage_color(kind))
                                    .child(kind.to_uppercase()),
                            )
                        },
                    ),
            )
            .child(
                div()
                    .mt_2()
                    .text_size(units(28.))
                    .line_height(relative(1.))
                    .font_family(theme::MONO_FONT_FAMILY)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.accent_hot)
                    .child(headline),
            )
            .child(summary.child(tags));
        let values = div()
            .w(relative(0.455))
            .p_4()
            .flex()
            .flex_wrap()
            .gap_y_3()
            .children(metrics.into_iter().map(|(label, value)| {
                div()
                    .w(relative(0.5))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(units(9.))
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_color(p.faint)
                            .child(label.to_uppercase()),
                    )
                    .child(
                        div()
                            .text_size(units(13.))
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_color(p.text)
                            .child(value),
                    )
            }));
        div()
            .flex()
            .items_stretch()
            .rounded_sm()
            .border_1()
            .border_color(p.border)
            .bg(p.panel)
            .child(leading)
            .child(values)
    }
    fn skills(&self, result: &PlannerPerformance, query: &str, cx: &Context<Self>) -> Vec<Div> {
        if self.filter == Filter::Stats {
            return vec![];
        }
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let skills = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or(""));
        let mut content = Vec::new();
        let main_id = snapshot.active_skill_ids.first().or_else(|| {
            skills
                .iter()
                .filter(|s| {
                    s.kind == SkillKind::Active
                        && snapshot.skill_ranks.get(&s.id).copied().unwrap_or(0) > 0
                })
                .max_by_key(|skill| snapshot.skill_ranks.get(&skill.id).copied().unwrap_or(0))
                .map(|skill| &skill.id)
        });
        if matches!(self.filter, Filter::All | Filter::Damage) && query.is_empty() {
            let main = main_id.and_then(|id| {
                result
                    .per_skill
                    .iter()
                    .chain(
                        self.skill_performance
                            .iter()
                            .flat_map(|result| result.per_skill.iter()),
                    )
                    .find(|skill| &skill.skill_id == id)
            });
            let mut main_panel = panel("main-skill-title", "Main Skill", cx);
            if let Some(main) = main {
                let name = main
                    .performance
                    .active_skill_name
                    .as_deref()
                    .unwrap_or(&main.skill_id);
                main_panel = panel_with_trailing(
                    "main-skill-title",
                    "Main Skill",
                    div()
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(9.))
                        .text_color(p.faint)
                        .child(name.to_uppercase()),
                    cx,
                )
                .child(self.hero(&main.skill_id, &main.performance, cx))
                .child(self.skill_breakdown(
                    &format!("main-{}", main.skill_id),
                    &main.performance,
                    cx,
                ));
            } else {
                main_panel = main_panel.child(
                    div()
                        .py_6()
                        .text_color(p.muted)
                        .text_size(units(12.))
                        .child(
                            "Pick an active skill in the Skills tab to see its damage breakdown.",
                        ),
                );
            }
            content.push(main_panel);
        }
        let performances = self.skill_performance.as_deref().unwrap_or(result);
        let mut cards = div().flex().flex_col().gap_2();
        let mut count = 0;
        for skill in skills.iter().filter(|skill| {
            skill.kind == SkillKind::Active
                && (query.is_empty() || skill.name.to_lowercase().contains(query))
        }) {
            count += 1;
            let key = format!("per-skill-{}", skill.id);
            let toggle = key.clone();
            let open = self.expanded.contains(&key);
            let rank = snapshot.skill_ranks.get(&skill.id).copied().unwrap_or(0);
            let active = snapshot.active_skill_ids.contains(&skill.id);
            let value = performances
                .per_skill
                .iter()
                .find(|entry| entry.skill_id == skill.id)
                .map(|entry| &entry.performance);
            let bonus = performances
                .computed
                .rank_bonuses
                .get(&skill.name.trim().to_lowercase())
                .copied()
                .filter(|_| rank > 0)
                .unwrap_or_default();
            let rank_label = if bonus == (0., 0.) {
                format!("RANK {rank}/{}", skill.max_rank)
            } else {
                format!(
                    "RANK {}/{} ({rank} +{})",
                    decimal_range((rank as f64 + bonus.0, rank as f64 + bonus.1)),
                    skill.max_rank,
                    decimal_range(bonus)
                )
            };
            let damage = value.and_then(|value| {
                value
                    .attack_damage
                    .as_ref()
                    .map(|damage| {
                        (
                            damage.combined_hit_min as f64,
                            damage.combined_hit_max as f64,
                        )
                    })
                    .or_else(|| {
                        value
                            .damage
                            .as_ref()
                            .map(|damage| (damage.final_min as f64, damage.final_max as f64))
                    })
            });
            let damage_label = if rank == 0 {
                "Not learned".into()
            } else {
                damage
                    .map(|value| {
                        format!(
                            "{} damage",
                            hsplanner_ui::numbers::compact_range(
                                value,
                                &self.session.read(cx).state().settings.number_scale
                            )
                        )
                    })
                    .unwrap_or_else(|| "—".into())
            };
            let mut card = div()
                .relative()
                .border_1()
                .border_color(if active { p.accent_deep } else { p.border })
                .rounded_sm()
                .px_3p5()
                .py_2p5()
                .bg(p.panel)
                .opacity(if rank == 0 { 0.55 } else { 1. })
                .when(active, |v| {
                    v.child(
                        div()
                            .absolute()
                            .left_0()
                            .top_2()
                            .bottom_2()
                            .w(px(2.))
                            .bg(p.accent_hot),
                    )
                })
                .child(
                    Button::new(SharedString::from(key))
                        .ghost()
                        .w_full()
                        .h_auto()
                        .p_0()
                        .selected(open)
                        .accessibility_label(format!("{} damage", skill.name))
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_baseline()
                                .gap_2()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_wrap()
                                        .gap_2()
                                        .child(div().text_color(p.text).child(skill.name.clone()))
                                        .child(
                                            div()
                                                .text_size(units(10.))
                                                .text_color(p.faint)
                                                .font_family(theme::MONO_FONT_FAMILY)
                                                .child(rank_label),
                                        ),
                                )
                                .child(
                                    div()
                                        .font_family(theme::MONO_FONT_FAMILY)
                                        .text_size(units(12.))
                                        .text_color(
                                            skill
                                                .damage_type
                                                .as_deref()
                                                .map(theme::damage_color)
                                                .unwrap_or(p.faint),
                                        )
                                        .child(damage_label),
                                )
                                .child(if open { "▾" } else { "▸" }),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle(&toggle, cx))),
                );
            let mut tags = div().flex().flex_wrap().gap_1().mt_2();
            if let Some(kind) = skill.damage_type.as_deref() {
                tags = tags.child(
                    div()
                        .border_1()
                        .border_color(theme::damage_color(kind).opacity(0.5))
                        .text_color(theme::damage_color(kind))
                        .px_1()
                        .py(units(2.))
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(9.))
                        .child(kind.to_uppercase()),
                );
            }
            for tag in hsplanner_engine::calc::subskill::effective_skill_tags(
                &skill.id,
                skill.tags.as_deref().unwrap_or_default(),
                &snapshot.subskill_ranks,
            ) {
                tags = tags.child(
                    div()
                        .px_1()
                        .py(units(2.))
                        .border_1()
                        .border_color(p.accent_deep.opacity(0.5))
                        .font_family(theme::MONO_FONT_FAMILY)
                        .text_size(units(9.))
                        .text_color(p.accent_hot)
                        .child(tag.to_uppercase()),
                );
            }
            card = card.child(tags);
            let mut costs = div()
                .flex()
                .flex_wrap()
                .gap_3()
                .mt_2()
                .text_size(units(11.))
                .text_color(p.muted);
            if let Some(cost) = value.and_then(|value| value.skill_costs.get(&skill.id)) {
                if let Some(range) = cost.mana_min.zip(cost.mana_max) {
                    costs = costs.child(format!("{} mana", decimal_range(range)));
                }
                if let Some(range) = cost.cast_rate_min.zip(cost.cast_rate_max) {
                    costs = costs.child(format!("{} casts/s", decimal_range(range)));
                }
            }
            if let Some(speed) = skill.movement_during_use {
                costs = costs.child(format!("Move {speed}%"));
            }
            costs = costs.child(format!("max rank {}", skill.max_rank));
            card = card.child(costs);
            if open
                && rank > 0
                && let Some(value) = value
            {
                card =
                    card.child(self.skill_breakdown(&format!("main-card-{}", skill.id), value, cx));
            }
            cards = cards.child(card);
        }
        if count == 0 {
            cards = cards.child(
                div()
                    .py_2()
                    .text_color(p.muted)
                    .child("No skills match your search."),
            );
        }
        content.push(
            div()
                .flex()
                .items_center()
                .gap_2p5()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(units(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.muted)
                .child(div().w_3p5().h(px(1.)).bg(p.accent_deep))
                .child(TooltipText::new("per-skill-title", "PER-SKILL DAMAGE", 0.2)),
        );
        content.push(
            div()
                .relative()
                .overflow_hidden()
                .rounded_md()
                .border_1()
                .border_color(p.border)
                .px_4()
                .py_3p5()
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(p.panel_secondary, 0.),
                    linear_color_stop(p.background, 1.),
                ))
                .child(hsplanner_ui::components::corner_marks(cx))
                .child(cards),
        );
        content
    }
    fn ehp(&self, result: &PlannerPerformance, query: &str, cx: &Context<Self>) -> Option<Div> {
        if !matches!(self.filter, Filter::All | Filter::Stats) {
            return None;
        }
        if !query.is_empty()
            && ![
                "effective hp",
                "ehp",
                "hit pool",
                "life",
                "physical",
                "fire",
                "cold",
                "lightning",
                "poison",
                "arcane",
                "resistance",
                "damage reduction",
            ]
            .iter()
            .any(|term| term.contains(query))
        {
            return None;
        }
        let p = cx.global::<TooltipTheme>();
        let mut rows = div().flex().flex_wrap().gap_3();
        let mut disclosures = div();
        for entry in &result.computed.ehp.entries {
            let key = format!("ehp-{}", entry.damage_type);
            let toggle = key.clone();
            let open = self.expanded.contains(&key);
            let worst = result.computed.ehp.worst.as_ref() == Some(&entry.damage_type);
            rows = rows.child(
                Button::new(SharedString::from(key))
                    .ghost()
                    .h_auto()
                    .min_w(units(110.))
                    .flex_1()
                    .p_3()
                    .border_1()
                    .border_color(if worst { p.accent_deep } else { p.border })
                    .selected(open)
                    .accessibility_label(format!("{} effective HP", entry.damage_type))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .text_size(units(9.))
                                    .text_color(theme::damage_color(&entry.damage_type))
                                    .child(entry.damage_type.to_uppercase()),
                            )
                            .child(
                                div()
                                    .text_size(units(15.))
                                    .font_family(theme::MONO_FONT_FAMILY)
                                    .child(
                                        entry
                                            .ehp
                                            .map(|v| format_range((v, v), false))
                                            .unwrap_or_else(|| "Immune".into()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(units(9.))
                                    .text_color(p.faint)
                                    .child(if worst { "LOWEST" } else { "" }),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle(&toggle, cx))),
            );
            if open {
                disclosures = disclosures.child(heading(
                    SharedString::from(format!("ehp-layers-{}", entry.damage_type)),
                    &format!("{} mitigation", entry.damage_type),
                    cx,
                ));
                for layer in &entry.layers {
                    disclosures = disclosures.child(value_row(
                        layer.label.clone(),
                        format_range((layer.pct, layer.pct), true),
                        cx,
                    ));
                }
                disclosures = disclosures.child(value_row(
                    "Damage multiplier",
                    format!("×{:.3}", entry.multiplier),
                    cx,
                ));
            }
        }
        Some(
            panel("ehp-title", "Effective HP", cx)
                .child(rows)
                .child(disclosures),
        )
    }
    fn stat_rows(&self, query: &str) -> Vec<Row> {
        let mut columns: [Vec<Cell>; 2] = [Vec::new(), Vec::new()];
        for category in [
            Group::Offense,
            Group::Mitigation,
            Group::Resistances,
            Group::Resources,
            Group::Skills,
            Group::World,
            Group::Other,
        ] {
            if !category.visible(self.filter) {
                continue;
            }
            let definitions = data::game_config()
                .stats
                .iter()
                .enumerate()
                .filter(|(_, def)| {
                    def.modifies_attribute.is_none()
                        && !def.item_only.unwrap_or(false)
                        && !def.skill_scoped.unwrap_or(false)
                        && group(&def.key, &def.category) == category
                        && (query.is_empty() || def.name.to_lowercase().contains(query))
                })
                .map(|(ix, _)| Cell::Stat(ix))
                .collect::<Vec<_>>();
            if definitions.is_empty() {
                continue;
            }
            let ix = if columns[0].len() <= columns[1].len() {
                0
            } else {
                1
            };
            columns[ix].push(Cell::Heading(category));
            columns[ix].extend(definitions);
        }
        pair_columns(columns)
    }
    fn stat_cell(
        &self,
        cell: Option<Cell>,
        result: &PlannerPerformance,
        cx: &Context<Self>,
    ) -> Div {
        let cell_div = div().flex_1().min_w_0();
        match cell {
            None => cell_div,
            Some(Cell::Heading(category)) => cell_div.child(heading(
                SharedString::from(format!("stat-group-{}", category.title())),
                category.title(),
                cx,
            )),
            Some(Cell::Stat(ix)) => {
                let definition = &data::game_config().stats[ix];
                cell_div.child(self.stat(
                    &definition.key,
                    &definition.name,
                    definition.format.as_deref() == Some("percent"),
                    &result.computed,
                    cx,
                ))
            }
        }
    }
    // The All Stats panel chrome is split across list items so rows can be virtualized.
    fn stats_frame(&self, cx: &App) -> Div {
        let p = cx.global::<TooltipTheme>();
        div()
            .mx_6()
            .px_4()
            .border_l_1()
            .border_r_1()
            .border_color(p.border)
            .bg(p.background)
    }
    fn render_row(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = cx.global::<TooltipTheme>();
        let query = self.query.read(cx).value().trim().to_lowercase();
        let result = self.tree.read(cx).performance();
        let row = match self.rows.get(ix) {
            Some(row) => *row,
            None => return div().into_any_element(),
        };
        // List items are laid out as roots, so they must claim the full width themselves.
        let content: AnyElement = match (row, result) {
            (Row::Header, _) => self.header(window, cx).into_any_element(),
            (Row::Calculating, _) | (Row::Top, None) => div()
                .px_6()
                .py_5()
                .text_color(p.muted)
                .child("Calculating…")
                .into_any_element(),
            (Row::Top, Some(result)) => {
                let logical_width =
                    f32::from(window.viewport_size().width) / (f32::from(window.rem_size()) / 13.);
                let attribute_columns = if logical_width >= 1024. {
                    6
                } else if logical_width >= 640. {
                    3
                } else {
                    2
                };
                div()
                    .px_6()
                    .pb_3p5()
                    .flex()
                    .flex_col()
                    .gap_3p5()
                    .children(self.attributes(&result, &query, attribute_columns, cx))
                    .children(self.skills(&result, &query, cx))
                    .children(self.ehp(&result, &query, cx))
                    .into_any_element()
            }
            (Row::StatsStart, _) => div()
                .mx_6()
                .child(
                    panel("all-stats-title", "All Stats", cx)
                        .rounded_b_none()
                        .border_b_0()
                        .pb_0(),
                )
                .into_any_element(),
            (Row::StatsEmpty, _) => self
                .stats_frame(cx)
                .child(
                    div()
                        .py_3()
                        .text_color(p.muted)
                        .child("No stats match your search."),
                )
                .into_any_element(),
            (Row::StatsPair { left, right }, Some(result)) => self
                .stats_frame(cx)
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap_8()
                        .child(self.stat_cell(left, &result, cx))
                        .child(self.stat_cell(right, &result, cx)),
                )
                .into_any_element(),
            (Row::StatsPair { .. }, None) => self.stats_frame(cx).into_any_element(),
            (Row::StatsEnd, _) => self
                .stats_frame(cx)
                .h_4()
                .mb_6()
                .rounded_b_md()
                .border_b_1()
                .into_any_element(),
        };
        div().w_full().min_w_0().child(content).into_any_element()
    }
}
impl StatsView {
    fn header(&self, _: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        div()
            .px_6()
            .pt_6()
            .pb_3p5()
            .flex()
            .flex_col()
            .gap_3p5()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .text_size(units(22.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.accent_hot)
                            .child(TooltipText::new("stats-title", "Stats", 0.04)),
                    )
                    .child(
                        div().flex().items_center().gap_1p5().children(
                            [Filter::All, Filter::Damage, Filter::Stats, Filter::Skills]
                                .into_iter()
                                .map(|filter| {
                                    Button::new(SharedString::from(format!(
                                        "stats-filter-{}",
                                        filter.name()
                                    )))
                                    .map(|button| {
                                        let tone = if self.filter == filter {
                                            hsplanner_ui::controls::ButtonTone::Primary
                                        } else {
                                            hsplanner_ui::controls::ButtonTone::Neutral
                                        };
                                        hsplanner_ui::controls::button_look(button, tone, cx)
                                    })
                                    .small()
                                    .label(filter.name())
                                    .selected(self.filter == filter)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.filter = filter;
                                        cx.notify();
                                    }))
                                }),
                        ),
                    ),
            )
            .child(Styled::h(
                Input::new(&self.query)
                    .planner_style(cx)
                    .cleanable(true)
                    .aria_label("Search stats, attributes, or skills")
                    .prefix(Icon::new(IconName::Search).size_3p5().text_color(p.faint))
                    .px_3()
                    .py_2()
                    .gap_2p5()
                    .text_size(rems(1.))
                    .line_height(relative(1.5)),
                units(34.5),
            ))
    }
}
impl Render for StatsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let query = self.query.read(cx).value().trim().to_lowercase();
        let ready = self.tree.read(cx).performance().is_some();
        let row_filter = (query, self.filter, ready);
        let rem = window.rem_size();
        if self.row_filter.as_ref() != Some(&row_filter) {
            let mut rows = vec![Row::Header];
            if ready {
                rows.push(Row::Top);
                rows.extend(self.stat_rows(&row_filter.0));
            } else {
                rows.push(Row::Calculating);
            }
            if rows != self.rows {
                self.list.reset(rows.len());
                self.rows = rows;
            }
            self.row_filter = Some(row_filter);
        }
        if rem != self.last_rem {
            self.list.reset(self.rows.len());
            self.last_rem = rem;
        }
        let rows = list(self.list.clone(), cx.processor(Self::render_row))
            .size_full()
            .min_h_0();
        div()
            .id("stats-scroll")
            .size_full()
            .min_h_0()
            .min_w_0()
            .bg(p.background)
            .text_color(p.text)
            .child(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn stat_categories_preserve_reference_priority() {
        assert!(matches!(group("life_steal", "offense"), Group::Resources));
        assert!(matches!(
            group("magic_damage_reduction", "defense"),
            Group::Mitigation
        ));
        assert!(matches!(
            group("max_all_resistances", "defense"),
            Group::Resistances
        ));
        assert!(matches!(
            group("critical_damage", "offense"),
            Group::Offense
        ));
        assert!(matches!(group("all_skills", "utility"), Group::Skills));
    }
    #[::core::prelude::v1::test]
    fn paired_columns_keep_every_cell_and_frame_the_panel() {
        let rows = pair_columns([
            vec![Cell::Heading(Group::Offense), Cell::Stat(0), Cell::Stat(1)],
            vec![Cell::Heading(Group::Other)],
        ]);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0], Row::StatsStart);
        assert_eq!(
            rows[1],
            Row::StatsPair {
                left: Some(Cell::Heading(Group::Offense)),
                right: Some(Cell::Heading(Group::Other))
            }
        );
        assert_eq!(
            rows[3],
            Row::StatsPair {
                left: Some(Cell::Stat(1)),
                right: None
            }
        );
        assert_eq!(rows[4], Row::StatsEnd);
        assert_eq!(
            pair_columns([vec![], vec![]]),
            [Row::StatsStart, Row::StatsEmpty, Row::StatsEnd]
        );
    }
    #[::core::prelude::v1::test]
    fn filter_tabs_show_only_their_reference_sections() {
        assert!(Group::Mitigation.visible(Filter::Stats));
        assert!(!Group::Offense.visible(Filter::Stats));
        assert!(Group::Offense.visible(Filter::Damage));
        assert!(!Group::World.visible(Filter::Skills));
    }
}
