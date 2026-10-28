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
    #[serde(default)]
    pub rank_indexing: crate::calc::types::ManaCostRankIndexing,
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
            mana_cost_formula: v
                .mana_cost_formula
                .map(|f| crate::calc::passive::ManaCostFormula {
                    base: f.base,
                    per_level: f.per_level,
                    rank_indexing: f.rank_indexing,
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

// A rank of 0 or less has no passive stats.
pub fn passive_stats_at_rank(skill: PassiveSkillDto, rank: f64) -> HashMap<String, f64> {
    if rank <= 0.0 {
        return HashMap::new();
    }
    crate::calc::passive::passive_stats_at_rank(&skill.into(), rank as u32)
}

pub fn mana_cost_at_rank(skill: PassiveSkillDto, rank: f64) -> Option<f64> {
    crate::calc::passive::mana_cost_at_effective_rank(&skill.into(), rank)
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

pub(crate) fn subskill_aggregation_impl(
    input: &SubskillAggregationInput,
) -> SubskillAggregationOutput {
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

pub fn subskill_aggregation(input: SubskillAggregationInput) -> SubskillAggregationOutput {
    subskill_aggregation_impl(&input)
}
