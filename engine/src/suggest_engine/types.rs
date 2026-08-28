use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Ranged = (f64, f64);
pub type StatMap = HashMap<String, Ranged>;
pub type AttrMap = HashMap<String, Ranged>;

#[inline]
pub fn r_min(v: Ranged) -> f64 {
    v.0
}
#[inline]
pub fn r_max(v: Ranged) -> f64 {
    v.1
}
#[inline]
pub fn ranged_add(a: Ranged, b: Ranged) -> Ranged {
    (a.0 + b.0, a.1 + b.1)
}
#[inline]
pub fn ranged_is_zero(v: Ranged) -> bool {
    v.0.abs() < 1e-9 && v.1.abs() < 1e-9
}

// Tree lines are parsed by calc::tree::parse — the engine that consumes the
// resulting stats — so the parsed shapes live there too.
pub use crate::calc::tree::parse::{ConvertKind, DisableTarget, ParsedConversion, ParsedMeta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNodeInfo {
    #[serde(rename = "t", default)]
    pub title: String,
    #[serde(rename = "n", default)]
    pub kind: String,
    #[serde(rename = "l", default)]
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeGraph {
    pub adjacency: HashMap<u32, Vec<u32>>,
    pub start_ids: Vec<u32>,
    pub warp_ids: Vec<u32>,
    pub valuable_ids: Vec<u32>,
    pub jewelry_ids: Vec<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageFormula {
    pub base: f64,
    pub per_level: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DamageRow {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "per", rename_all = "snake_case")]
pub enum BonusSource {
    AttributePoint {
        source: String,
        #[serde(default)]
        stat: String,
        value: f64,
    },
    SkillLevel {
        source: String,
        #[serde(default)]
        stat: String,
        value: f64,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillProc {
    pub chance: f64,
    pub trigger: String,
    pub target: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackScalingRef {
    #[serde(default)]
    pub weapon_damage_pct: Option<DamageFormula>,
    #[serde(default)]
    pub flat_physical_min: Option<DamageFormula>,
    #[serde(default)]
    pub flat_physical_max: Option<DamageFormula>,
    #[serde(default)]
    pub attack_rating_pct: Option<DamageFormula>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub damage_type: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub damage_formula: Option<DamageFormula>,
    #[serde(default)]
    pub damage_per_rank: Option<Vec<DamageRow>>,
    #[serde(default)]
    pub bonus_sources: Vec<BonusSource>,
    #[serde(default)]
    pub base_cast_rate: Option<f64>,
    #[serde(default)]
    pub uses_attack_speed: bool,
    #[serde(default)]
    pub uses_skill_haste: bool,
    #[serde(default)]
    pub proc: Option<SkillProc>,
    #[serde(default)]
    pub attack_kind: Option<String>,
    #[serde(default)]
    pub attack_scaling: Option<AttackScalingRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameConfig {
    #[serde(default)]
    pub attribute_keys: Vec<String>,
    #[serde(default)]
    pub default_base_attributes: HashMap<String, f64>,
    #[serde(default)]
    pub default_base_stats: HashMap<String, f64>,
    #[serde(default)]
    pub default_stats_per_attribute: HashMap<String, HashMap<String, f64>>,
    #[serde(default)]
    pub attribute_divided_stats: HashMap<String, HashMap<String, f64>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrecomputedInput {
    // Unaggregated per-key (min,max) contributions; engine must aggregate once
    // to match TS `computeBuildStatsCore`.
    #[serde(default)]
    pub stat_contributions: HashMap<String, Vec<Ranged>>,
    #[serde(default)]
    pub attr_contributions: HashMap<String, Vec<Ranged>>,
    pub graph: TreeGraph,
    #[serde(default)]
    pub tree_nodes: HashMap<u32, TreeNodeInfo>,
    #[serde(default)]
    pub allocated_tree_nodes: Vec<u32>,
    pub active_skill: Option<SkillRef>,
    pub active_skill_rank: u32,
    #[serde(default)]
    pub skill_ranks_by_name: HashMap<String, f64>,
    #[serde(default)]
    pub inventory: crate::calc::types::Inventory,
    #[serde(default)]
    pub enemy_conditions: HashMap<String, bool>,
    #[serde(default)]
    pub player_conditions: HashMap<String, bool>,
    #[serde(default)]
    pub enemy_resistances: HashMap<String, f64>,
    pub projectile_count: Option<u32>,
    pub budget: u32,
    #[serde(default)]
    pub all_skills: Vec<SkillRef>,
    #[serde(default)]
    pub game_config: GameConfig,
    #[serde(default)]
    pub proc_toggles: HashMap<String, bool>,
    #[serde(default)]
    pub skill_ranks_by_id: HashMap<String, f64>,
    #[serde(default)]
    pub skill_projectiles: HashMap<String, u32>,
    #[serde(default)]
    pub kills_per_sec: f64,
    #[serde(default)]
    pub season: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestStep {
    pub node_id: u32,
    pub dps_before: f64,
    pub dps_after: f64,
    pub gain: f64,
    pub is_filler: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestResult {
    pub added_nodes: Vec<u32>,
    pub sequence: Vec<SuggestStep>,
    pub base_dps: f64,
    pub final_dps: f64,
    pub budget_used: u32,
    pub budget_requested: u32,
    pub unsupported_lines: Vec<String>,
    pub used_starts: Vec<u32>,
}

