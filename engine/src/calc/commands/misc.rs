use super::*;

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

impl From<PassiveSkillDto> for crate::calc::passive::PassiveSkill {
    fn from(v: PassiveSkillDto) -> Self {
        crate::calc::passive::PassiveSkill {
            passive_stats: v.passive_stats.map(|p| crate::calc::passive::PassiveStats {
                base: p.base,
                per_rank: p.per_rank,
            }),
            mana_cost_formula: v.mana_cost_formula.map(|f| crate::calc::passive::ManaCostFormula {
                base: f.base,
                per_level: f.per_level,
            }),
            ranks: v
                .ranks
                .into_iter()
                .map(|r| crate::calc::passive::SkillRank {
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
    crate::calc::passive::passive_stats_at_rank(&skill.into(), rank as u32)
}

#[tauri::command]
pub fn mana_cost_at_rank(skill: PassiveSkillDto, rank: f64) -> Option<f64> {
    let r = if rank <= 0.0 { 1 } else { rank as u32 };
    crate::calc::passive::mana_cost_at_rank(&skill.into(), r)
}

// ---------- parse_custom_stats ----------
// Batched custom-stat input validation so the config UI previews exactly what
// calc/custom_stat.rs will apply.

#[tauri::command]
pub fn parse_custom_stats(values: Vec<String>) -> Vec<Option<[f64; 2]>> {
    values
        .iter()
        .map(|v| crate::calc::custom_stat::parse_custom_stat_value(v).map(|(a, b)| [a, b]))
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
    use crate::calc::affix::{apply_stars_to_ranged_value, rolled_affix_value_with_stars};
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
    use crate::calc::tree::parse::{TreeLineClass, classify_tree_node_line};
    crate::calc::data::tree_nodes()
        .iter()
        .map(|(id, node)| {
            let mut out = NodeLineClassification::default();
            for line in &node.lines {
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

pub(crate) fn subskill_aggregation_impl(input: &SubskillAggregationInput) -> SubskillAggregationOutput {
    let Some(spec) = crate::calc::data::get_skills_by_class(&input.class_id)
        .iter()
        .find(|s| s.id == input.skill_id)
    else {
        return SubskillAggregationOutput::default();
    };
    let owner = crate::calc::stats::skill_spec_to_subskill_owner(spec);
    let agg = crate::calc::subskill::aggregate_subskill_stats(
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
    // f64: subtree proc weighting produces fractional expected projectile
    // counts (e.g. 15 * 25% = 3.75); truncated like build.rs before use.
    #[serde(default = "default_projectile_count")]
    pub projectile_count: f64,
    /// Only the skill that owns the subtree may pass a non-zero value.
    #[serde(default)]
    pub of_total_damage: f64,
}
