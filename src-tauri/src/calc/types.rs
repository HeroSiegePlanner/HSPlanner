use std::collections::HashMap;

use serde::Deserialize;

use super::skills::Ranged;

pub type SlotKey = String;
pub type StatMap = HashMap<String, f64>;
pub type RangedStatMap = HashMap<String, RangedValue>;

/// JSON shape `number | [number, number]`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum RangedValue {
    Scalar(f64),
    Range([f64; 2]),
}

impl Default for RangedValue {
    fn default() -> Self {
        Self::Scalar(0.0)
    }
}

impl RangedValue {
    pub fn as_ranged(self) -> Ranged {
        match self {
            Self::Scalar(n) => (n, n),
            Self::Range([a, b]) => (a, b),
        }
    }
}

// ---------- affix ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
pub enum AffixSign {
    #[serde(rename = "+")]
    #[default]
    Plus,
    #[serde(rename = "-")]
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AffixFormat {
    #[default]
    Flat,
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AffixKind {
    Prefix,
    Suffix,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Affix {
    pub id: String,
    #[serde(default)]
    pub group_id: String,
    #[serde(default)]
    pub tier: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub stat_key: Option<String>,
    #[serde(default)]
    pub sign: AffixSign,
    #[serde(default)]
    pub format: AffixFormat,
    #[serde(default)]
    pub value_min: Option<f64>,
    #[serde(default)]
    pub value_max: Option<f64>,
    #[serde(default)]
    pub kind: Option<AffixKind>,
    #[serde(default)]
    pub slots: Option<Vec<SlotKey>>,
}

// ---------- rune / gem / runeword ----------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Rune {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tier: u32,
    #[serde(default)]
    pub stats: StatMap,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Gem {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tier: u32,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub stats: StatMap,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Runeword {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub runes: Vec<String>,
    #[serde(default)]
    pub allowed_base_types: Vec<String>,
    #[serde(default)]
    pub stats: StatMap,
    #[serde(default)]
    pub requires_level: Option<u32>,
    #[serde(default)]
    pub requires_item_level: Option<u32>,
    #[serde(default)]
    pub description: Option<String>,
}

// ---------- item base ----------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ItemBase {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub base_type: String,
    #[serde(default)]
    pub slot: SlotKey,
    #[serde(default)]
    pub rarity: String,
    #[serde(default)]
    pub grade: Option<String>,
    #[serde(default)]
    pub defense_min: Option<f64>,
    #[serde(default)]
    pub defense_max: Option<f64>,
    #[serde(default)]
    pub block_chance: Option<f64>,
    #[serde(default)]
    pub damage_min: Option<f64>,
    #[serde(default)]
    pub damage_max: Option<f64>,
    #[serde(default)]
    pub attack_speed: Option<RangedValue>,
    #[serde(default)]
    pub two_handed: Option<bool>,
    #[serde(default)]
    pub item_level: Option<u32>,
    #[serde(default)]
    pub requires_level: Option<u32>,
    #[serde(default)]
    pub implicit: Option<RangedStatMap>,
    #[serde(default)]
    pub sockets: Option<u32>,
    #[serde(default)]
    pub max_sockets: Option<u32>,
    #[serde(default)]
    pub set_id: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub herobound: Option<bool>,
    #[serde(default)]
    pub quest_item: Option<bool>,
    #[serde(default)]
    pub flavor: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub skill_bonuses: Option<HashMap<String, RangedValue>>,
    #[serde(default)]
    pub unique_effects: Option<Vec<String>>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub max_affixes: Option<u32>,
    #[serde(default)]
    pub socket_transforms: Option<HashMap<String, StatMap>>,
    #[serde(default)]
    pub random_affix_group_id: Option<String>,
}

// ---------- item set ----------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ItemSetPiece {
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub item_id: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ItemSetBonus {
    pub pieces: u32,
    #[serde(default)]
    pub stats: StatMap,
    #[serde(default)]
    pub descriptions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ItemSet {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub items: Vec<ItemSetPiece>,
    #[serde(default)]
    pub bonuses: Vec<ItemSetBonus>,
}

// ---------- augment ----------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AugmentLevel {
    #[serde(default)]
    pub stats: StatMap,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AngelicAugment {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub levels: Vec<AugmentLevel>,
}

// ---------- item-granted skill ----------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PassiveStatsSpec {
    #[serde(default)]
    pub base: Option<StatMap>,
    #[serde(default)]
    pub per_rank: Option<StatMap>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PassiveConvert {
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub pct: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PassiveConverts {
    #[serde(default)]
    pub per_rank: Vec<PassiveConvert>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ItemGrantedSkill {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub passive_stats: Option<PassiveStatsSpec>,
    #[serde(default)]
    pub passive_converts: Option<PassiveConverts>,
    #[serde(default)]
    pub condition: Option<String>,
}

// ---------- character class ----------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CharacterClass {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub base_attributes: HashMap<String, f64>,
    #[serde(default)]
    pub base_stats: HashMap<String, f64>,
    #[serde(default)]
    pub stats_per_level: HashMap<String, f64>,
    #[serde(default)]
    pub stats_per_attribute: HashMap<String, HashMap<String, f64>>,
    #[serde(default)]
    pub starting_skills: Vec<String>,
}

// ---------- skill (full JSON shape) ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SkillKind {
    #[default]
    Active,
    Passive,
    Aura,
    Buff,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DamageRangeSpec {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DamageFormulaSpec {
    pub base: f64,
    pub per_level: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ManaCostFormulaSpec {
    pub base: f64,
    pub per_level: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BonusSourceSpec {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub stat: String,
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub per: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttackKindSpec {
    Attack,
    Spell,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AttackSkillScalingSpec {
    #[serde(default)]
    pub weapon_damage_pct: Option<DamageFormulaSpec>,
    #[serde(default)]
    pub flat_physical_min: Option<DamageFormulaSpec>,
    #[serde(default)]
    pub flat_physical_max: Option<DamageFormulaSpec>,
    #[serde(default)]
    pub attack_rating_pct: Option<DamageFormulaSpec>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillRankSpec {
    pub rank: u32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub stats: Option<StatMap>,
    #[serde(default)]
    pub mana_cost: Option<f64>,
    #[serde(default)]
    pub cooldown: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillProcSpec {
    #[serde(default)]
    pub chance: f64,
    #[serde(default)]
    pub trigger: String,
    #[serde(default)]
    pub target: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubskillEffectSpec {
    #[serde(default)]
    pub base: Option<StatMap>,
    #[serde(default)]
    pub per_rank: Option<StatMap>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AppliedStateValue {
    Name(String),
    Full {
        state: String,
        #[serde(default)]
        amount: Option<AmountSpec>,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AmountSpec {
    #[serde(default)]
    pub base: Option<f64>,
    #[serde(default)]
    pub per_rank: Option<f64>,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChanceSpec {
    #[serde(default)]
    pub base: Option<f64>,
    #[serde(default)]
    pub per_rank: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubskillProcSpec {
    #[serde(default)]
    pub trigger: String,
    #[serde(default)]
    pub chance: ChanceSpec,
    #[serde(default)]
    pub effects: Option<SubskillEffectSpec>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub applies_states: Option<Vec<AppliedStateValue>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubskillNodeSpec {
    pub id: String,
    #[serde(default)]
    pub position_index: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub max_rank: u32,
    #[serde(default)]
    pub effects: Option<SubskillEffectSpec>,
    #[serde(default)]
    pub proc: Option<SubskillProcSpec>,
    #[serde(default)]
    pub requires_subskill: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillSpec {
    pub id: String,
    #[serde(default)]
    pub class_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kind: SkillKind,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub max_rank: u32,
    #[serde(default)]
    pub requires_level: Option<u32>,
    #[serde(default)]
    pub requires_skill: Option<String>,
    #[serde(default)]
    pub ranks: Vec<SkillRankSpec>,
    #[serde(default)]
    pub damage_type: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub movement_during_use: Option<f64>,
    #[serde(default)]
    pub range: Option<f64>,
    #[serde(default)]
    pub base_cast_rate: Option<f64>,
    #[serde(default)]
    pub base_cooldown: Option<f64>,
    #[serde(default)]
    pub effect_duration: Option<f64>,
    #[serde(default)]
    pub damage_per_rank: Option<Vec<DamageRangeSpec>>,
    #[serde(default)]
    pub damage_formula: Option<DamageFormulaSpec>,
    #[serde(default)]
    pub mana_cost_formula: Option<ManaCostFormulaSpec>,
    #[serde(default)]
    pub bonus_sources: Option<Vec<BonusSourceSpec>>,
    #[serde(default)]
    pub passive_stats: Option<PassiveStatsSpec>,
    #[serde(default)]
    pub proc: Option<SkillProcSpec>,
    #[serde(default)]
    pub subskills: Option<Vec<SubskillNodeSpec>>,
    #[serde(default)]
    pub tree: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub attack_kind: Option<AttackKindSpec>,
    #[serde(default)]
    pub attack_scaling: Option<AttackSkillScalingSpec>,
}

// ---------- game config ----------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AttributeDef {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatDef {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub modifies_attribute: Option<String>,
    #[serde(default)]
    pub cap: Option<f64>,
    #[serde(default)]
    pub item_only: Option<bool>,
    #[serde(default)]
    pub skill_scoped: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SlotDef {
    pub key: String,
    pub name: String,
    pub group: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameConfig {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub attributes: Vec<AttributeDef>,
    #[serde(default)]
    pub stats: Vec<StatDef>,
    #[serde(default)]
    pub slots: Option<Vec<SlotDef>>,
    #[serde(default)]
    pub max_character_level: u32,
    #[serde(default)]
    pub ether_max_level: u32,
    #[serde(default)]
    pub attribute_points_per_level: u32,
    #[serde(default)]
    pub talent_points_per_level: u32,
    #[serde(default)]
    pub skill_points_per_level: u32,
    #[serde(default)]
    pub default_base_stats: Option<StatMap>,
    #[serde(default)]
    pub default_base_attributes: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub default_stats_per_attribute: Option<HashMap<String, HashMap<String, f64>>>,
    #[serde(default)]
    pub attribute_divided_stats: Option<HashMap<String, HashMap<String, f64>>>,
}

// ---------- build state ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SocketType {
    #[default]
    Normal,
    Rainbow,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquippedAffix {
    #[serde(default)]
    pub affix_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub tier: u32,
    #[serde(default)]
    pub roll: f64,
    #[serde(default)]
    pub custom_value: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AugmentRef {
    pub id: String,
    pub level: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquippedItem {
    pub base_id: String,
    #[serde(default)]
    pub stars: Option<u32>,
    #[serde(default)]
    pub affixes: Vec<EquippedAffix>,
    #[serde(default)]
    #[allow(dead_code)]
    pub socket_count: u32,
    #[serde(default)]
    pub socketed: Vec<Option<String>>,
    #[serde(default)]
    pub socket_types: Vec<SocketType>,
    #[serde(default)]
    #[allow(dead_code)]
    pub runeword_id: Option<String>,
    #[serde(default)]
    pub forged_mods: Vec<EquippedAffix>,
    #[serde(default)]
    pub augment: Option<AugmentRef>,
    #[serde(default)]
    pub implicit_overrides: HashMap<String, f64>,
}

pub type Inventory = HashMap<SlotKey, EquippedItem>;

// ---------- custom stats ----------

/// User-defined override; `value` is free text parsed by `parse_custom_stat_value`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomStat {
    #[serde(default)]
    pub stat_key: String,
    #[serde(default)]
    pub value: String,
}

// ---------- talent tree ----------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TreeNodeInfo {
    /// Display title (top-level name).
    #[serde(default)]
    pub t: String,
    /// Node kind tag (e.g. "warp", "jewelry", "minor", "notable", ...).
    #[serde(default)]
    pub n: String,
    /// Mod text lines feeding the parser. Empty for jewelry / warp nodes.
    #[serde(default)]
    pub l: Vec<String>,
    /// Optional group tags.
    #[serde(default)]
    pub g: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TreeSocketContent {
    Item {
        id: String,
    },
    Uncut {
        #[serde(default)]
        affixes: Vec<EquippedAffix>,
    },
}
