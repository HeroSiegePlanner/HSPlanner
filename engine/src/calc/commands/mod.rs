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

mod damage;
mod misc;
mod performance;

pub use damage::*;
pub use misc::*;
pub use performance::*;

#[cfg(test)]
mod tests;
