use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::Emitter;

use super::build::{BuildPerformance, BuildPerformanceDeps, compute_build_performance};
use super::skills as calc;
use super::stats::{
    BuildStatsInput, ComputedStats, StatBreakdown, compute_build_stats, compute_stat_breakdown,
};
use super::types::{Affix, CustomStat, EquippedItem, Inventory, TreeSocketContent};

#[derive(Deserialize)]
#[serde(untagged)]
pub enum NumberOrRange {
    Number(f64),
    Range([f64; 2]),
}
impl From<NumberOrRange> for (f64, f64) {
    fn from(v: NumberOrRange) -> Self {
        match v {
            NumberOrRange::Number(n) => (n, n),
            NumberOrRange::Range([a, b]) => (a, b),
        }
    }
}

fn ranged_map(raw: HashMap<String, NumberOrRange>) -> HashMap<String, (f64, f64)> {
    raw.into_iter().map(|(k, v)| (k, v.into())).collect()
}

fn normalized_keys<V>(raw: HashMap<String, V>) -> HashMap<String, V> {
    raw.into_iter().map(|(k, v)| (norm(&k), v)).collect()
}

#[inline]
fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageFormulaDto {
    pub base: f64,
    pub per_level: f64,
}
impl From<DamageFormulaDto> for calc::DamageFormula {
    fn from(v: DamageFormulaDto) -> Self {
        Self {
            base: v.base,
            per_level: v.per_level,
        }
    }
}

#[derive(Deserialize)]
pub struct DamageRowDto {
    pub min: f64,
    pub max: f64,
}
impl From<DamageRowDto> for calc::DamageRow {
    fn from(v: DamageRowDto) -> Self {
        Self {
            min: v.min,
            max: v.max,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "per", rename_all = "snake_case")]
pub enum BonusSourceDto {
    AttributePoint { source: String, value: f64 },
    SkillLevel { source: String, value: f64 },
}
impl From<BonusSourceDto> for calc::BonusSource {
    fn from(v: BonusSourceDto) -> Self {
        match v {
            BonusSourceDto::AttributePoint { source, value } => {
                calc::BonusSource::AttributePoint {
                    source: norm(&source),
                    value,
                }
            }
            BonusSourceDto::SkillLevel { source, value } => calc::BonusSource::SkillLevel {
                source: norm(&source),
                value,
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDto {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub damage_type: Option<String>,
    #[serde(default)]
    pub damage_formula: Option<DamageFormulaDto>,
    #[serde(default)]
    pub damage_per_rank: Option<Vec<DamageRowDto>>,
    #[serde(default)]
    pub bonus_sources: Vec<BonusSourceDto>,
}
impl From<SkillDto> for calc::Skill {
    fn from(v: SkillDto) -> Self {
        calc::Skill {
            name: norm(&v.name),
            tags: v.tags,
            damage_type: v.damage_type,
            damage_formula: v.damage_formula.map(Into::into),
            damage_per_rank: v
                .damage_per_rank
                .map(|t| t.into_iter().map(Into::into).collect()),
            bonus_sources: v.bonus_sources.into_iter().map(Into::into).collect(),
            attack_kind: None,
            attack_scaling: None,
        }
    }
}

// ---------- passive_stats_at_rank / mana_cost_at_rank ----------
// Thin commands over calc/passive.rs so the UI reads passive-rank stats and
// mana cost from the same source the engine uses.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassiveStatsDto {
    #[serde(default)]
    pub base: HashMap<String, f64>,
    #[serde(default)]
    pub per_rank: HashMap<String, f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManaCostFormulaDto {
    pub base: f64,
    pub per_level: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRankDto {
    pub rank: u32,
    #[serde(default)]
    pub mana_cost: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassiveSkillDto {
    #[serde(default)]
    pub passive_stats: Option<PassiveStatsDto>,
    #[serde(default)]
    pub mana_cost_formula: Option<ManaCostFormulaDto>,
    #[serde(default)]
    pub ranks: Vec<SkillRankDto>,
}

impl From<PassiveSkillDto> for super::passive::PassiveSkill {
    fn from(v: PassiveSkillDto) -> Self {
        super::passive::PassiveSkill {
            passive_stats: v.passive_stats.map(|p| super::passive::PassiveStats {
                base: p.base,
                per_rank: p.per_rank,
            }),
            mana_cost_formula: v.mana_cost_formula.map(|f| super::passive::ManaCostFormula {
                base: f.base,
                per_level: f.per_level,
            }),
            ranks: v
                .ranks
                .into_iter()
                .map(|r| super::passive::SkillRank {
                    rank: r.rank,
                    mana_cost: r.mana_cost,
                })
                .collect(),
        }
    }
}

// rank arrives as a JS number; clamp like the former TS helpers (rank <= 0 -> empty / 1).
#[tauri::command]
pub fn passive_stats_at_rank(skill: PassiveSkillDto, rank: f64) -> HashMap<String, f64> {
    if rank <= 0.0 {
        return HashMap::new();
    }
    super::passive::passive_stats_at_rank(&skill.into(), rank as u32)
}

#[tauri::command]
pub fn mana_cost_at_rank(skill: PassiveSkillDto, rank: f64) -> Option<f64> {
    let r = if rank <= 0.0 { 1 } else { rank as u32 };
    super::passive::mana_cost_at_rank(&skill.into(), r)
}

// ---------- parse_custom_stats ----------
// Batched custom-stat input validation so the config UI previews exactly what
// calc/custom_stat.rs will apply.

#[tauri::command]
pub fn parse_custom_stats(values: Vec<String>) -> Vec<Option<[f64; 2]>> {
    values
        .iter()
        .map(|v| super::custom_stat::parse_custom_stat_value(v).map(|(a, b)| [a, b]))
        .collect()
}

// ---------- display_values ----------
// Batched affix/star display math for tooltips and editors; replaces the
// former TS rolledAffixValue*/applyStarsToRangedValue helpers.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffixValueReq {
    pub affix: Affix,
    #[serde(default)]
    pub roll: f64,
    #[serde(default)]
    pub stars: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScaledValueReq {
    pub value: [f64; 2],
    pub stat_key: String,
    #[serde(default)]
    pub stars: Option<u32>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DisplayValuesInput {
    #[serde(default)]
    pub affixes: Vec<AffixValueReq>,
    #[serde(default)]
    pub scaled: Vec<ScaledValueReq>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffixValueOut {
    pub value: f64,
    pub range_min: f64,
    pub range_max: f64,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DisplayValuesOutput {
    pub affixes: Vec<AffixValueOut>,
    pub scaled: Vec<[f64; 2]>,
}

pub fn display_values_impl(input: &DisplayValuesInput) -> DisplayValuesOutput {
    use super::affix::{apply_stars_to_ranged_value, rolled_affix_value_with_stars};
    DisplayValuesOutput {
        affixes: input
            .affixes
            .iter()
            .map(|r| AffixValueOut {
                value: rolled_affix_value_with_stars(&r.affix, r.roll, r.stars),
                range_min: rolled_affix_value_with_stars(&r.affix, 0.0, r.stars),
                range_max: rolled_affix_value_with_stars(&r.affix, 1.0, r.stars),
            })
            .collect(),
        scaled: input
            .scaled
            .iter()
            .map(|r| {
                let out =
                    apply_stars_to_ranged_value((r.value[0], r.value[1]), &r.stat_key, r.stars);
                [out.0, out.1]
            })
            .collect(),
    }
}

// Star scaling reads per-season config, so the season rides as a command param.
#[tauri::command]
pub fn display_values(input: DisplayValuesInput, season: Option<String>) -> DisplayValuesOutput {
    let _scope = crate::calc::season::SeasonScope::enter(season);
    display_values_impl(&input)
}

// ---------- classify_tree_nodes ----------
// Bulk three-way line classification for the tree tooltips; replaces the
// former TS classifyNodeLines so the UI shows exactly what the engine parses.

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeLineClassification {
    pub parsed: Vec<String>,
    pub unsupported: Vec<String>,
}

pub fn classify_tree_nodes_impl() -> HashMap<String, NodeLineClassification> {
    use super::tree::parse::{TreeLineClass, classify_tree_node_line};
    super::data::tree_nodes()
        .iter()
        .map(|(id, node)| {
            let mut out = NodeLineClassification::default();
            for line in &node.l {
                match classify_tree_node_line(line) {
                    TreeLineClass::Stat(_) | TreeLineClass::Meta(_) => {
                        out.parsed.push(line.clone())
                    }
                    TreeLineClass::RecognizedNoStat => {}
                    TreeLineClass::Unknown => out.unsupported.push(line.clone()),
                }
            }
            (id.clone(), out)
        })
        .collect()
}

#[tauri::command]
pub fn classify_tree_nodes(season: Option<String>) -> HashMap<String, NodeLineClassification> {
    let _scope = crate::calc::season::SeasonScope::enter(season);
    classify_tree_nodes_impl()
}

// ---------- subskill_aggregation ----------
// Thin command over calc/subskill.rs so the skill tooltip reads subtree
// bonuses from the same aggregation the engine uses.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubskillAggregationInput {
    pub class_id: String,
    pub skill_id: String,
    #[serde(default)]
    pub subskill_ranks: HashMap<String, u32>,
    #[serde(default)]
    pub enemy_conditions: HashMap<String, bool>,
    #[serde(default)]
    pub season: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedStateOut {
    pub state: String,
    pub trigger: String,
    pub chance: f64,
    pub amount: Option<f64>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubskillAggregationOutput {
    pub stats: HashMap<String, f64>,
    pub proc_stats: HashMap<String, f64>,
    pub applied_states: Vec<AppliedStateOut>,
}

fn subskill_aggregation_impl(input: &SubskillAggregationInput) -> SubskillAggregationOutput {
    let Some(spec) = super::data::get_skills_by_class(&input.class_id)
        .iter()
        .find(|s| s.id == input.skill_id)
    else {
        return SubskillAggregationOutput::default();
    };
    let owner = super::stats::skill_spec_to_subskill_owner(spec);
    let agg = super::subskill::aggregate_subskill_stats(
        &owner,
        &input.subskill_ranks,
        Some(&input.enemy_conditions),
    );
    SubskillAggregationOutput {
        stats: agg.stats,
        proc_stats: agg.proc_stats,
        applied_states: agg
            .applied_states
            .into_iter()
            .map(|s| AppliedStateOut {
                state: s.state,
                trigger: s.trigger,
                chance: s.chance,
                amount: s.amount,
            })
            .collect(),
    }
}

#[tauri::command]
pub fn subskill_aggregation(input: SubskillAggregationInput) -> SubskillAggregationOutput {
    let _scope = crate::calc::season::SeasonScope::enter(input.season.clone());
    subskill_aggregation_impl(&input)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponDto {
    pub name: String,
    pub damage_min: f64,
    pub damage_max: f64,
}
impl From<WeaponDto> for calc::Weapon {
    fn from(v: WeaponDto) -> Self {
        calc::Weapon {
            name: v.name,
            damage_min: v.damage_min,
            damage_max: v.damage_max,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDamageInput {
    pub skill: SkillDto,
    pub allocated_rank: f64,
    #[serde(default)]
    pub attributes: HashMap<String, NumberOrRange>,
    #[serde(default)]
    pub stats: HashMap<String, NumberOrRange>,
    #[serde(default)]
    pub skill_ranks_by_name: HashMap<String, f64>,
    #[serde(default)]
    pub item_skill_bonuses: HashMap<String, (f64, f64)>,
    #[serde(default)]
    pub enemy_conditions: HashMap<String, bool>,
    #[serde(default)]
    pub enemy_resistances: HashMap<String, f64>,
    #[serde(default)]
    pub skills_by_name: HashMap<String, SkillDto>,
    #[serde(default = "default_projectile_count")]
    pub projectile_count: u32,
}
fn default_projectile_count() -> u32 {
    1
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponDamageInput {
    #[serde(default)]
    pub weapon: Option<WeaponDto>,
    #[serde(default)]
    pub stats: HashMap<String, NumberOrRange>,
    #[serde(default)]
    pub enemy_conditions: HashMap<String, bool>,
    #[serde(default)]
    pub enemy_resistances: HashMap<String, f64>,
    #[serde(default)]
    pub projectile_count: Option<u32>,
}

#[derive(Serialize)]
pub struct ExtraSourceOut {
    pub label: String,
    pub pct: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDamageOutput {
    pub effective_rank_min: f64,
    pub effective_rank_max: f64,
    pub base_min: f64,
    pub base_max: f64,
    pub flat_min: f64,
    pub flat_max: f64,
    pub synergy_min_pct: f64,
    pub synergy_max_pct: f64,
    pub skill_damage_min_pct: f64,
    pub skill_damage_max_pct: f64,
    pub extra_damage_pct: f64,
    pub extra_damage_sources: Vec<ExtraSourceOut>,
    pub crit_chance: f64,
    pub crit_damage_pct: f64,
    pub crit_multiplier_avg: f64,
    pub multicast_chance_pct: f64,
    pub multicast_multiplier: f64,
    pub projectile_count: u32,
    pub elemental_break_pct: f64,
    pub elemental_break_multiplier: f64,
    pub enemy_resistance_pct: f64,
    pub resistance_ignored_pct: f64,
    pub effective_resistance_pct: f64,
    pub resistance_multiplier: f64,
    pub hit_min: i64,
    pub hit_max: i64,
    pub crit_min: i64,
    pub crit_max: i64,
    pub final_min: i64,
    pub final_max: i64,
    pub avg_min: i64,
    pub avg_max: i64,
}
impl From<calc::SkillDamageBreakdown> for SkillDamageOutput {
    fn from(v: calc::SkillDamageBreakdown) -> Self {
        Self {
            effective_rank_min: v.effective_rank_min,
            effective_rank_max: v.effective_rank_max,
            base_min: v.base_min,
            base_max: v.base_max,
            flat_min: v.flat_min,
            flat_max: v.flat_max,
            synergy_min_pct: v.synergy_min_pct,
            synergy_max_pct: v.synergy_max_pct,
            skill_damage_min_pct: v.skill_damage_min_pct,
            skill_damage_max_pct: v.skill_damage_max_pct,
            extra_damage_pct: v.extra_damage_pct,
            extra_damage_sources: v
                .extra_damage_sources
                .into_iter()
                .map(|e| ExtraSourceOut {
                    label: e.label.to_string(),
                    pct: e.pct,
                })
                .collect(),
            crit_chance: v.crit_chance,
            crit_damage_pct: v.crit_damage_pct,
            crit_multiplier_avg: v.crit_multiplier_avg,
            multicast_chance_pct: v.multicast_chance_pct,
            multicast_multiplier: v.multicast_multiplier,
            projectile_count: v.projectile_count,
            elemental_break_pct: v.elemental_break_pct,
            elemental_break_multiplier: v.elemental_break_multiplier,
            enemy_resistance_pct: v.enemy_resistance_pct,
            resistance_ignored_pct: v.resistance_ignored_pct,
            effective_resistance_pct: v.effective_resistance_pct,
            resistance_multiplier: v.resistance_multiplier,
            hit_min: v.hit_min,
            hit_max: v.hit_max,
            crit_min: v.crit_min,
            crit_max: v.crit_max,
            final_min: v.final_min,
            final_max: v.final_max,
            avg_min: v.avg_min,
            avg_max: v.avg_max,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponDamageOutput {
    pub has_weapon: bool,
    pub weapon_name: Option<String>,
    pub weapon_damage_min: f64,
    pub weapon_damage_max: f64,
    pub enhanced_damage_min_pct: f64,
    pub enhanced_damage_max_pct: f64,
    pub additive_physical_min: f64,
    pub additive_physical_max: f64,
    pub additive_elemental_min: f64,
    pub additive_elemental_max: f64,
    pub additive_elemental_breakdown: Vec<ExtraSourceOut>,
    pub attack_damage_min_pct: f64,
    pub attack_damage_max_pct: f64,
    pub extra_damage_pct: f64,
    pub extra_damage_sources: Vec<ExtraSourceOut>,
    pub crushing_blow_modifier: f64,
    pub armor_break_pct: f64,
    pub deadly_blow_chance: f64,
    pub hit_chance: f64,
    pub crit_chance: f64,
    pub crit_damage_pct: f64,
    pub crit_multiplier_avg: f64,
    pub attacks_per_second_min: f64,
    pub attacks_per_second_max: f64,
    pub projectile_count: u32,
    pub enemy_phys_res_pct: f64,
    pub phys_resistance_ignored_pct: f64,
    pub hit_min: i64,
    pub hit_max: i64,
    pub crit_min: i64,
    pub crit_max: i64,
    pub avg_min: i64,
    pub avg_max: i64,
    pub open_wounds_min: i64,
    pub open_wounds_max: i64,
    pub dps_min: i64,
    pub dps_max: i64,
}
impl From<calc::WeaponDamageBreakdown> for WeaponDamageOutput {
    fn from(v: calc::WeaponDamageBreakdown) -> Self {
        let map_sources = |xs: Vec<calc::ExtraSource>| {
            xs.into_iter()
                .map(|e| ExtraSourceOut {
                    label: e.label.to_string(),
                    pct: e.pct,
                })
                .collect()
        };
        Self {
            has_weapon: v.has_weapon,
            weapon_name: v.weapon_name,
            weapon_damage_min: v.weapon_damage_min,
            weapon_damage_max: v.weapon_damage_max,
            enhanced_damage_min_pct: v.enhanced_damage_min_pct,
            enhanced_damage_max_pct: v.enhanced_damage_max_pct,
            additive_physical_min: v.additive_physical_min,
            additive_physical_max: v.additive_physical_max,
            additive_elemental_min: v.additive_elemental_min,
            additive_elemental_max: v.additive_elemental_max,
            additive_elemental_breakdown: map_sources(v.additive_elemental_breakdown),
            attack_damage_min_pct: v.attack_damage_min_pct,
            attack_damage_max_pct: v.attack_damage_max_pct,
            extra_damage_pct: v.extra_damage_pct,
            extra_damage_sources: map_sources(v.extra_damage_sources),
            crushing_blow_modifier: v.crushing_blow_modifier,
            armor_break_pct: v.armor_break_pct,
            deadly_blow_chance: v.deadly_blow_chance,
            hit_chance: v.hit_chance,
            crit_chance: v.crit_chance,
            crit_damage_pct: v.crit_damage_pct,
            crit_multiplier_avg: v.crit_multiplier_avg,
            attacks_per_second_min: v.attacks_per_second_min,
            attacks_per_second_max: v.attacks_per_second_max,
            projectile_count: v.projectile_count,
            enemy_phys_res_pct: v.enemy_phys_res_pct,
            phys_resistance_ignored_pct: v.phys_resistance_ignored_pct,
            hit_min: v.hit_min,
            hit_max: v.hit_max,
            crit_min: v.crit_min,
            crit_max: v.crit_max,
            avg_min: v.avg_min,
            avg_max: v.avg_max,
            open_wounds_min: v.open_wounds_min,
            open_wounds_max: v.open_wounds_max,
            dps_min: v.dps_min,
            dps_max: v.dps_max,
        }
    }
}

#[tauri::command]
pub fn compute_skill_damage(input: SkillDamageInput) -> Option<SkillDamageOutput> {
    let skill: calc::Skill = input.skill.into();
    let attributes = ranged_map(normalized_keys(input.attributes));
    let stats = ranged_map(normalized_keys(input.stats));
    let skill_ranks = normalized_keys(input.skill_ranks_by_name);
    let item_bonuses = normalized_keys(input.item_skill_bonuses);
    let skills_by_name: HashMap<String, calc::Skill> = input
        .skills_by_name
        .into_iter()
        .map(|(k, v)| (norm(&k), v.into()))
        .collect();

    let inp = calc::SkillInput {
        skill: &skill,
        allocated_rank: input.allocated_rank,
        attributes: &attributes,
        stats: &stats,
        skill_ranks_by_name: &skill_ranks,
        item_skill_bonuses: &item_bonuses,
        enemy_conditions: &input.enemy_conditions,
        enemy_resistances: &input.enemy_resistances,
        skills_by_name: &skills_by_name,
        projectile_count: input.projectile_count,
    };
    calc::compute_skill_damage(&inp).map(Into::into)
}

#[tauri::command]
pub fn compute_weapon_damage(input: WeaponDamageInput) -> WeaponDamageOutput {
    let weapon: Option<calc::Weapon> = input.weapon.map(Into::into);
    let stats = ranged_map(normalized_keys(input.stats));
    calc::compute_weapon_damage(
        weapon.as_ref(),
        &stats,
        &input.enemy_conditions,
        &input.enemy_resistances,
        input.projectile_count,
    )
    .into()
}

// ---------- compute_build_performance command ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPerformanceInput {
    #[serde(default)]
    pub class_id: Option<String>,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub allocated_attrs: HashMap<String, u32>,
    #[serde(default)]
    pub inventory: Inventory,
    #[serde(default)]
    pub skill_ranks: HashMap<String, u32>,
    #[serde(default)]
    pub subskill_ranks: HashMap<String, u32>,
    #[serde(default)]
    pub active_aura_id: Option<String>,
    #[serde(default)]
    pub active_buffs: HashMap<String, bool>,
    #[serde(default)]
    pub custom_stats: Vec<CustomStat>,
    #[serde(default)]
    pub allocated_tree_nodes: HashSet<u32>,
    #[serde(default)]
    pub tree_socketed: HashMap<u32, TreeSocketContent>,
    #[serde(default)]
    pub main_skill_id: Option<String>,
    #[serde(default)]
    pub enemy_conditions: HashMap<String, bool>,
    #[serde(default)]
    pub player_conditions: HashMap<String, bool>,
    #[serde(default)]
    pub skill_projectiles: HashMap<String, u32>,
    #[serde(default)]
    pub enemy_resistances: HashMap<String, f64>,
    #[serde(default)]
    pub proc_toggles: HashMap<String, bool>,
    #[serde(default)]
    pub kills_per_sec: f64,
    #[serde(default)]
    pub season: Option<String>,
    #[serde(default)]
    pub granted_skill_ranks: HashMap<String, calc::Ranged>,
}

fn perf_deps<'a>(
    input: &'a BuildPerformanceInput,
    inventory: &'a Inventory,
    main_skill_id: Option<&'a str>,
) -> BuildPerformanceDeps<'a> {
    BuildPerformanceDeps {
        class_id: input.class_id.as_deref(),
        level: input.level,
        allocated_attrs: &input.allocated_attrs,
        inventory,
        skill_ranks: &input.skill_ranks,
        subskill_ranks: &input.subskill_ranks,
        active_aura_id: input.active_aura_id.as_deref(),
        active_buffs: &input.active_buffs,
        custom_stats: &input.custom_stats,
        allocated_tree_nodes: &input.allocated_tree_nodes,
        tree_socketed: &input.tree_socketed,
        main_skill_id,
        enemy_conditions: &input.enemy_conditions,
        player_conditions: &input.player_conditions,
        skill_projectiles: &input.skill_projectiles,
        enemy_resistances: &input.enemy_resistances,
        proc_toggles: &input.proc_toggles,
        kills_per_sec: input.kills_per_sec,
        granted_skill_ranks: Some(&input.granted_skill_ranks),
    }
}

#[tauri::command]
pub fn calc_build_performance(input: BuildPerformanceInput) -> BuildPerformance {
    let _scope = crate::calc::season::SeasonScope::enter(input.season.clone());
    compute_build_performance(&perf_deps(
        &input,
        &input.inventory,
        input.main_skill_id.as_deref(),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankSlotItemsInput {
    pub perf: BuildPerformanceInput,
    pub slot: String,
    pub base_ids: Vec<String>,
    #[serde(default)]
    pub active_skill_ids: Vec<String>,
}

// Ranks candidate bases for one slot by combined-DPS midpoint. Multi-skill
// builds mirror the frontend combo fan-out: sum avg-hit DPS per active
// skill, count proc DPS once.
#[tauri::command]
pub fn rank_slot_items(input: RankSlotItemsInput) -> HashMap<String, f64> {
    let _scope = crate::calc::season::SeasonScope::enter(input.perf.season.clone());
    let mid = |a: Option<f64>, b: Option<f64>| match (a, b) {
        (Some(x), Some(y)) => Some((x + y) / 2.0),
        _ => None,
    };
    let mut out: HashMap<String, f64> = HashMap::with_capacity(input.base_ids.len());
    for base_id in &input.base_ids {
        let mut inventory = input.perf.inventory.clone();
        inventory.insert(
            input.slot.clone(),
            EquippedItem {
                base_id: base_id.clone(),
                ..Default::default()
            },
        );
        let dps = if input.active_skill_ids.len() > 1 {
            let mut sum: Option<(f64, f64)> = None;
            let mut proc = (0.0, 0.0);
            for (i, sid) in input.active_skill_ids.iter().enumerate() {
                let p =
                    compute_build_performance(&perf_deps(&input.perf, &inventory, Some(sid)));
                if let (Some(a), Some(b)) = (p.avg_hit_dps_min, p.avg_hit_dps_max) {
                    let s = sum.unwrap_or((0.0, 0.0));
                    sum = Some((s.0 + a, s.1 + b));
                }
                if i == 0 {
                    proc = (p.proc_dps_min, p.proc_dps_max);
                }
            }
            match sum {
                Some((a, b)) => (a + proc.0 + b + proc.1) / 2.0,
                None => (proc.0 + proc.1) / 2.0,
            }
        } else {
            let main = input
                .active_skill_ids
                .first()
                .map(String::as_str)
                .or(input.perf.main_skill_id.as_deref());
            let p = compute_build_performance(&perf_deps(&input.perf, &inventory, main));
            mid(p.combined_dps_min, p.combined_dps_max)
                .or_else(|| mid(p.hit_dps_min, p.hit_dps_max))
                .unwrap_or(0.0)
        };
        out.insert(base_id.clone(), dps);
    }
    out
}

/// Warm-up progress: `current` of `total` tree nodes parsed. Emitted as
/// "warmup-progress" so the boot splash can drive an honest 0–15% slice.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WarmupProgress {
    current: u32,
    total: u32,
}

/// Warms the data Lazy and the tree-mod parser caches so the first real calc
/// after launch isn't stuck compiling ~300 regexes on the UI thread. Reports
/// progress via `on_progress(current, total)`; IO-free so tests can drive it
/// without a Tauri app handle.
pub fn run_warmup<F: FnMut(u32, u32)>(mut on_progress: F) -> bool {
    let d = super::data::data();
    if d.items.is_empty() {
        return false;
    }
    let nodes = super::data::tree_nodes();
    let total = nodes.len() as u32;
    // Cap emitted ticks so a ~1000-node warm-up doesn't flood the event channel.
    let step = (nodes.len() / 20).max(1);
    for (i, node) in nodes.values().enumerate() {
        for line in &node.l {
            let _ = super::tree::parse::parse_tree_node_mod(line);
            let _ = super::tree::parse::parse_tree_node_meta(line);
        }
        if i % step == 0 {
            on_progress(i as u32, total);
        }
    }
    on_progress(total, total);
    true
}

/// Tauri command: runs the warm-up off the event loop so the webview stays
/// responsive, emitting "warmup-progress" for the boot splash.
#[tauri::command]
pub async fn calc_warmup(app: tauri::AppHandle, season: Option<String>) -> bool {
    // Scope lives inside the blocking closure so it never crosses an .await.
    tauri::async_runtime::spawn_blocking(move || {
        let _scope = crate::calc::season::SeasonScope::enter(season);
        run_warmup(|current, total| {
            let _ = app.emit("warmup-progress", WarmupProgress { current, total });
        })
    })
    .await
    .unwrap_or(false)
}

/// Returns full stats plus per-stat source breakdown rendered by StatsView and
/// the tooltips. Reuses `BuildPerformanceInput`; damage/proc fields are unused here.
#[tauri::command]
pub fn calc_build_stats(input: BuildPerformanceInput) -> ComputedStats {
    let _scope = crate::calc::season::SeasonScope::enter(input.season.clone());
    let stats_input = BuildStatsInput {
        class_id: input.class_id.as_deref(),
        level: input.level,
        allocated_attrs: &input.allocated_attrs,
        inventory: &input.inventory,
        skill_ranks: &input.skill_ranks,
        active_aura_id: input.active_aura_id.as_deref(),
        active_buffs: &input.active_buffs,
        custom_stats: &input.custom_stats,
        allocated_tree_nodes: &input.allocated_tree_nodes,
        tree_socketed: &input.tree_socketed,
        player_conditions: &input.player_conditions,
        subskill_ranks: &input.subskill_ranks,
        enemy_conditions: &input.enemy_conditions,
        granted_skill_ranks: Some(&input.granted_skill_ranks),
    };
    compute_build_stats(&stats_input)
}

/// Backs the frontend StatBreakdownModal: re-runs stats and slices out one
/// key's additive/more contributions, per-source subtotals, and final value.
/// `kind` selects stat-sources vs attribute-sources.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatBreakdownInput {
    #[serde(flatten)]
    pub deps: BuildPerformanceInput,
    pub stat_key: String,
    #[serde(default)]
    pub kind: StatBreakdownKind,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum StatBreakdownKind {
    #[default]
    Stat,
    Attribute,
}

#[tauri::command]
pub fn calc_stat_breakdown(input: StatBreakdownInput) -> StatBreakdown {
    let _scope = crate::calc::season::SeasonScope::enter(input.deps.season.clone());
    let stats_input = BuildStatsInput {
        class_id: input.deps.class_id.as_deref(),
        level: input.deps.level,
        allocated_attrs: &input.deps.allocated_attrs,
        inventory: &input.deps.inventory,
        skill_ranks: &input.deps.skill_ranks,
        active_aura_id: input.deps.active_aura_id.as_deref(),
        active_buffs: &input.deps.active_buffs,
        custom_stats: &input.deps.custom_stats,
        allocated_tree_nodes: &input.deps.allocated_tree_nodes,
        tree_socketed: &input.deps.tree_socketed,
        player_conditions: &input.deps.player_conditions,
        subskill_ranks: &input.deps.subskill_ranks,
        enemy_conditions: &input.deps.enemy_conditions,
        granted_skill_ranks: Some(&input.deps.granted_skill_ranks),
    };
    let computed = compute_build_stats(&stats_input);
    let sources = match input.kind {
        StatBreakdownKind::Stat => &computed.stat_sources,
        StatBreakdownKind::Attribute => &computed.attribute_sources,
    };
    let final_value = match input.kind {
        StatBreakdownKind::Stat => computed.stats.get(&input.stat_key).copied(),
        StatBreakdownKind::Attribute => computed.attributes.get(&input.stat_key).copied(),
    };
    compute_stat_breakdown(sources, &input.stat_key, final_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_custom_stats_batch_mirrors_custom_stat_parser() {
        let out = parse_custom_stats(vec![
            "100".to_string(),
            "50-80".to_string(),
            "not a number".to_string(),
        ]);
        assert_eq!(out[0], Some([100.0, 100.0]));
        assert_eq!(out[1], Some([50.0, 80.0]));
        assert_eq!(out[2], None);
    }

    #[test]
    fn display_values_batch_matches_affix_math() {
        let affix = Affix {
            id: "t".into(),
            stat_key: Some("life".into()),
            value_min: Some(10.0),
            value_max: Some(20.0),
            ..Default::default()
        };
        let input = DisplayValuesInput {
            affixes: vec![AffixValueReq {
                affix: affix.clone(),
                roll: 0.5,
                stars: None,
            }],
            scaled: vec![ScaledValueReq {
                value: [10.0, 20.0],
                stat_key: "life".into(),
                stars: Some(0),
            }],
        };
        let out = display_values_impl(&input);
        assert_eq!(
            out.affixes[0].value,
            super::super::affix::rolled_affix_value(&affix, 0.5)
        );
        assert_eq!(out.affixes[0].range_min, 10.0);
        assert_eq!(out.affixes[0].range_max, 20.0);
        assert_eq!(out.scaled[0], [10.0, 20.0]);
    }

    #[test]
    fn classify_tree_nodes_partitions_every_node_line() {
        let map = classify_tree_nodes_impl();
        assert!(!map.is_empty(), "tree data should yield nodes");
        let nodes = super::super::data::tree_nodes();
        for (id, cls) in &map {
            let node = nodes.get(id).expect("classified id exists in data");
            assert!(cls.parsed.len() + cls.unsupported.len() <= node.l.len());
            for line in cls.parsed.iter().chain(cls.unsupported.iter()) {
                assert!(node.l.contains(line), "line must come from the node");
            }
        }
    }

    #[test]
    fn every_tree_node_line_is_supported_in_all_seasons() {
        // Every tree-node line must classify as parsed (stat or meta) in each
        // season; an unrecognized future pattern fails here, not silently.
        for season in ["s9", "s10"] {
            let _scope = crate::calc::season::SeasonScope::enter(Some(season.to_string()));
            let map = classify_tree_nodes_impl();
            let nodes = super::super::data::tree_nodes();
            for (id, cls) in &map {
                assert!(
                    cls.unsupported.is_empty(),
                    "season {season} node {id} ({}) has unsupported lines: {:?}",
                    nodes.get(id).map(|n| n.t.as_str()).unwrap_or(""),
                    cls.unsupported,
                );
            }
        }
    }

    #[test]
    fn subskill_aggregation_unknown_skill_returns_empty() {
        let input = SubskillAggregationInput {
            class_id: "no_such_class".to_string(),
            skill_id: "no_such_skill".to_string(),
            subskill_ranks: HashMap::from([("no_such_skill:sub".to_string(), 3)]),
            enemy_conditions: HashMap::new(),
            season: None,
        };
        let out = subskill_aggregation_impl(&input);
        assert!(out.stats.is_empty());
        assert!(out.proc_stats.is_empty());
        assert!(out.applied_states.is_empty());
    }

    // Proves the item-ranking batch returns one finite score per base id.
    #[test]
    fn rank_slot_items_scores_every_base() {
        let perf: BuildPerformanceInput =
            serde_json::from_str("{}").expect("all fields default");
        let base_ids: Vec<String> = crate::calc::data::data()
            .items
            .keys()
            .take(3)
            .cloned()
            .collect();
        assert!(!base_ids.is_empty(), "items present");
        let out = rank_slot_items(RankSlotItemsInput {
            perf,
            slot: "armor".to_string(),
            base_ids: base_ids.clone(),
            active_skill_ids: Vec::new(),
        });
        assert_eq!(out.len(), base_ids.len());
        assert!(out.values().all(|v| v.is_finite() && *v >= 0.0));
    }
}
