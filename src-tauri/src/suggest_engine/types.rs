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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfCondition {
    CritChanceBelow40,
    LifeBelow40,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedMod {
    pub key: String,
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_condition: Option<SelfCondition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionKind {
    Stat,
    Attribute,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedConversion {
    pub from_key: String,
    pub from_kind: ConversionKind,
    pub to_key: String,
    pub to_kind: ConversionKind,
    pub pct: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisableTarget {
    LifeReplenish,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParsedMeta {
    Convert(ParsedConversion),
    Disable { target: DisableTarget },
}

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
    AttributePoint { source: String, value: f64 },
    SkillLevel { source: String, value: f64 },
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
    pub proc: Option<SkillProc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub base_attributes: HashMap<String, f64>,
    #[serde(default)]
    pub base_stats: HashMap<String, f64>,
    #[serde(default)]
    pub stats_per_level: HashMap<String, f64>,
    #[serde(default)]
    pub start_ids: Vec<u32>,
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
    pub class: Option<ClassInfo>,
    pub level: u32,
    #[serde(default)]
    pub allocated_attributes: HashMap<String, f64>,
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestStep {
    pub node_id: u32,
    pub dps_before: f64,
    pub dps_after: f64,
    pub gain: f64,
    pub is_filler: bool,
}

#[derive(Debug, Clone, Serialize)]
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

