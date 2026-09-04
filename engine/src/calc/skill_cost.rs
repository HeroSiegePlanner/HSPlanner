//! Per-skill mana / life cost, cast rate, entity swing rate and mana sustain.
//! One source for the Stats cards, the left panel and the web share.
use std::collections::HashMap;

use serde::Serialize;

use super::affix_tags;
use super::build::DEFAULT_ENTITY_RATE;
use super::passive::{self, ManaCostFormula, PassiveSkill, SkillRank};
use super::skills::{r_max, rg, Ranged, StatMap};
use super::types::{AffixEffect, SkillSpec};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRate {
    pub base: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCost {
    pub eff_rank_min: f64,
    pub eff_rank_max: f64,
    pub base_mana_min: Option<f64>,
    pub base_mana_max: Option<f64>,
    pub mcr_max: f64,
    pub base_rate: Option<f64>,
    pub speed_max: f64,
    pub mana_min: Option<f64>,
    pub mana_max: Option<f64>,
    pub life_min: Option<f64>,
    pub life_max: Option<f64>,
    pub cast_rate_min: Option<f64>,
    pub cast_rate_max: Option<f64>,
    pub entity_rate: Option<EntityRate>,
    pub mana_per_sec_min: Option<f64>,
    pub mana_per_sec_max: Option<f64>,
    pub mana_regen_min: f64,
    pub mana_regen_max: f64,
    pub sustainable: bool,
    pub unsustainable: bool,
    pub net_min: Option<f64>,
    pub net_max: Option<f64>,
    pub uptime_min: Option<f64>,
    pub uptime_max: Option<f64>,
}

pub struct SkillCostInput<'a> {
    pub skill: &'a SkillSpec,
    pub eff_rank: Ranged,
    /// Shared stats with the skill's own overrides and `_more` totals folded in.
    pub stats: &'a StatMap,
    pub tags: &'a [String],
    pub entity_rates: &'a HashMap<String, f64>,
}

pub fn mana_cost_at_rank(skill: &SkillSpec, rank: u32) -> Option<f64> {
    let passive = PassiveSkill {
        passive_stats: None,
        mana_cost_formula: skill.mana_cost_formula.as_ref().map(|f| ManaCostFormula {
            base: f.base,
            per_level: f.per_level,
        }),
        ranks: skill
            .ranks
            .iter()
            .map(|r| SkillRank {
                rank: r.rank,
                mana_cost: r.mana_cost,
            })
            .collect(),
    };
    passive::mana_cost_at_rank(&passive, rank)
}

/// Entities swing on their own cadence: the config knob scaled by their own
/// attack-speed stats, unless a subskill pins the rate outright.
pub fn entity_rate(
    kind: &str,
    tags: &[String],
    stats: &StatMap,
    entity_rates: &HashMap<String, f64>,
) -> EntityRate {
    let kind_key = kind.to_lowercase();
    let fixed = r_max(rg(stats, &format!("{kind_key}_attack_rate_fixed")));
    if fixed > 0.0 {
        return EntityRate {
            base: fixed,
            min: fixed,
            max: fixed,
        };
    }
    let bonus = affix_tags::sum_for(AffixEffect::AttackSpeed, tags, stats);
    let base = entity_rates
        .get(&kind_key)
        .copied()
        .unwrap_or(DEFAULT_ENTITY_RATE);
    EntityRate {
        base,
        min: base * (1.0 + bonus.0 / 100.0),
        max: base * (1.0 + bonus.1 / 100.0),
    }
}

/// Base casts per second and the speed stat scaling them. Attack-speed skills
/// swing with the weapon; cooldown-gated ones fire once per cooldown.
pub fn cast_rate(skill: &SkillSpec, stats: &StatMap) -> (Option<f64>, Ranged) {
    if skill.uses_attack_speed {
        let aps = r_max(rg(stats, "attacks_per_second"));
        let base = if aps > 0.0 {
            Some(aps)
        } else {
            skill.base_cast_rate
        };
        return (base, rg(stats, "increased_attack_speed"));
    }
    if skill.uses_skill_haste {
        let base = skill
            .base_cooldown
            .filter(|cd| *cd > 0.0)
            .map(|cd| 1.0 / cd)
            .or(skill.base_cast_rate);
        return (base, rg(stats, "skill_haste"));
    }
    (skill.base_cast_rate, rg(stats, "faster_cast_rate"))
}

pub fn compute_skill_cost(input: &SkillCostInput<'_>) -> SkillCost {
    let stats = input.stats;
    let (rank_min, rank_max) = input.eff_rank;
    let cost_at = |rank: f64| mana_cost_at_rank(input.skill, rank.max(1.0) as u32);
    let (base_mana_min, base_mana_max) = match (cost_at(rank_min), cost_at(rank_max)) {
        (Some(a), Some(b)) => (Some(a.min(b)), Some(a.max(b))),
        (a, b) => (a.or(b), a.or(b)),
    };
    let mcr = rg(stats, "mana_cost_reduction");
    let cost_min = base_mana_min.map(|c| (c * (1.0 - mcr.1 / 100.0)).max(0.0));
    let cost_max = base_mana_max.map(|c| (c * (1.0 - mcr.0 / 100.0)).max(0.0));
    // "+X% of Your Mana Costs are taken from life instead" splits every cast.
    let paid = rg(stats, "mana_cost_paid_in_life");
    let pct_min = paid.0.clamp(0.0, 100.0);
    let pct_max = paid.1.clamp(0.0, 100.0);
    let mana_min = cost_min.map(|c| c * (1.0 - pct_max / 100.0));
    let mana_max = cost_max.map(|c| c * (1.0 - pct_min / 100.0));
    let life_min = cost_min.map(|c| c * (pct_min / 100.0));
    let life_max = cost_max.map(|c| c * (pct_max / 100.0));

    let (base_rate, speed) = cast_rate(input.skill, stats);
    let cast_rate_min = base_rate.map(|r| r * (1.0 + speed.0 / 100.0));
    let cast_rate_max = base_rate.map(|r| r * (1.0 + speed.1 / 100.0));
    let entity_rate = affix_tags::entity_tag_for(input.tags)
        .map(|kind| entity_rate(kind, input.tags, stats, input.entity_rates));

    let mana_per_sec_min = mana_min.zip(cast_rate_min).map(|(m, r)| m * r);
    let mana_per_sec_max = mana_max.zip(cast_rate_max).map(|(m, r)| m * r);
    let regen = rg(stats, "mana_replenish");
    let uptime = |per_sec: f64, regen: f64| {
        if per_sec <= 0.0 {
            100.0
        } else {
            (regen / per_sec * 100.0).min(100.0)
        }
    };
    SkillCost {
        eff_rank_min: rank_min,
        eff_rank_max: rank_max,
        base_mana_min,
        base_mana_max,
        mcr_max: mcr.1,
        base_rate,
        speed_max: speed.1,
        mana_min,
        mana_max,
        life_min,
        life_max,
        cast_rate_min,
        cast_rate_max,
        entity_rate,
        mana_per_sec_min,
        mana_per_sec_max,
        mana_regen_min: regen.0,
        mana_regen_max: regen.1,
        sustainable: mana_per_sec_max.is_some_and(|m| m <= regen.0),
        unsustainable: mana_per_sec_min.is_some_and(|m| m > regen.1),
        net_min: mana_per_sec_max.map(|m| regen.0 - m),
        net_max: mana_per_sec_min.map(|m| regen.1 - m),
        uptime_min: mana_per_sec_max.map(|m| uptime(m, regen.0)),
        uptime_max: mana_per_sec_min.map(|m| uptime(m, regen.1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::types::{ManaCostFormulaSpec, SkillRankSpec};

    fn stats(pairs: &[(&str, f64)]) -> StatMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), (*v, *v)))
            .collect()
    }

    fn formula(base: f64, per_level: f64) -> Option<ManaCostFormulaSpec> {
        Some(ManaCostFormulaSpec { base, per_level })
    }

    fn rank_row(rank: u32, mana_cost: f64) -> SkillRankSpec {
        SkillRankSpec {
            rank,
            mana_cost: Some(mana_cost),
            ..Default::default()
        }
    }

    fn thunder_fury() -> SkillSpec {
        SkillSpec {
            id: "thunder_fury".into(),
            name: "Thunder Fury".into(),
            base_cast_rate: Some(2.0),
            mana_cost_formula: formula(11.0, 1.0),
            ..Default::default()
        }
    }

    fn cost(skill: &SkillSpec, rank: f64, stats: &StatMap) -> SkillCost {
        let no_rates = HashMap::new();
        compute_skill_cost(&SkillCostInput {
            skill,
            eff_rank: (rank, rank),
            stats,
            tags: &[],
            entity_rates: &no_rates,
        })
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn formula_cost_is_floored_and_rank_clamped() {
        let s = SkillSpec {
            mana_cost_formula: formula(12.0, 2.0),
            ranks: vec![rank_row(1, 12.0)],
            ..Default::default()
        };
        assert_eq!(mana_cost_at_rank(&s, 1), Some(12.0));
        assert_eq!(mana_cost_at_rank(&s, 20), Some(50.0));
        assert_eq!(mana_cost_at_rank(&s, 0), Some(12.0));
        let frac = SkillSpec {
            mana_cost_formula: formula(5.0, 0.5),
            ..Default::default()
        };
        assert_eq!(mana_cost_at_rank(&frac, 2), Some(5.0));
        assert_eq!(mana_cost_at_rank(&frac, 3), Some(6.0));
    }

    #[test]
    fn rank_rows_then_first_row_then_nothing() {
        let s = SkillSpec {
            ranks: vec![rank_row(1, 10.0), rank_row(2, 14.0)],
            ..Default::default()
        };
        assert_eq!(mana_cost_at_rank(&s, 2), Some(14.0));
        assert_eq!(mana_cost_at_rank(&s, 7), Some(10.0));
        assert_eq!(mana_cost_at_rank(&SkillSpec::default(), 5), None);
    }

    #[test]
    fn plain_cast_skill_without_bonuses() {
        let c = cost(&thunder_fury(), 5.0, &stats(&[]));
        assert_eq!(c.eff_rank_min, 5.0);
        assert_eq!(c.mana_min, Some(15.0));
        assert_eq!(c.cast_rate_min, Some(2.0));
        assert_eq!(c.mana_per_sec_min, Some(30.0));
        assert_eq!(c.mana_regen_min, 0.0);
        assert!(!c.sustainable);
        assert!(c.unsustainable);
        assert_eq!(c.uptime_min, Some(0.0));
    }

    #[test]
    fn attack_speed_skill_rates_off_weapon_swings() {
        let kunai = SkillSpec {
            uses_attack_speed: true,
            base_cast_rate: None,
            ..thunder_fury()
        };
        let c = cost(
            &kunai,
            5.0,
            &stats(&[
                ("attacks_per_second", 1.25),
                ("increased_attack_speed", 60.0),
                ("faster_cast_rate", 999.0),
            ]),
        );
        assert!(close(c.cast_rate_min.unwrap(), 2.0));
        assert!(close(c.cast_rate_max.unwrap(), 2.0));
        assert!(cost(&kunai, 5.0, &stats(&[])).cast_rate_min.is_none());
    }

    #[test]
    fn cooldown_skill_rates_off_skill_haste_only() {
        let orb = SkillSpec {
            uses_skill_haste: true,
            base_cooldown: Some(1.75),
            ..Default::default()
        };
        let (base, speed) = cast_rate(
            &orb,
            &stats(&[("skill_haste", 75.0), ("faster_cast_rate", 500.0)]),
        );
        assert!(close(base.unwrap(), 1.0 / 1.75));
        assert!(close(base.unwrap() * (1.0 + speed.1 / 100.0), 1.0));
        let bolt = SkillSpec {
            base_cast_rate: Some(2.0),
            ..Default::default()
        };
        let (base, speed) = cast_rate(
            &bolt,
            &stats(&[("faster_cast_rate", 50.0), ("skill_haste", 999.0)]),
        );
        assert_eq!(base, Some(2.0));
        assert!(close(base.unwrap() * (1.0 + speed.1 / 100.0), 3.0));
    }

    #[test]
    fn mana_paid_in_life_splits_the_cast() {
        let c = cost(
            &thunder_fury(),
            5.0,
            &stats(&[("mana_cost_paid_in_life", 25.0)]),
        );
        assert_eq!(c.mana_min, Some(11.25));
        assert_eq!(c.life_min, Some(3.75));
        assert_eq!(c.mana_per_sec_min, Some(22.5));
        let clamped = cost(
            &thunder_fury(),
            5.0,
            &stats(&[("mana_cost_paid_in_life", 150.0)]),
        );
        assert_eq!(clamped.mana_min, Some(0.0));
    }

    #[test]
    fn mana_cost_reduction_never_goes_negative() {
        let c = cost(
            &thunder_fury(),
            5.0,
            &stats(&[("mana_cost_reduction", 120.0)]),
        );
        assert_eq!(c.mana_min, Some(0.0));
        assert_eq!(c.base_mana_min, Some(15.0));
        assert_eq!(c.mcr_max, 120.0);
    }

    #[test]
    fn regen_covering_the_worst_case_is_sustainable() {
        let c = cost(&thunder_fury(), 5.0, &stats(&[("mana_replenish", 999.0)]));
        assert!(c.sustainable);
        assert!(!c.unsustainable);
        assert_eq!(c.uptime_min, Some(100.0));
        assert_eq!(c.net_min, Some(969.0));
    }

    #[test]
    fn entity_rate_knob_and_pin() {
        let tags = vec!["Sentry".to_string()];
        let rates: HashMap<String, f64> = [("sentry".to_string(), 2.5)].into_iter().collect();
        let knob = entity_rate("Sentry", &tags, &stats(&[]), &rates);
        assert_eq!(knob.base, 2.5);
        assert_eq!(knob.min, 2.5);
        let pinned = entity_rate(
            "Sentry",
            &tags,
            &stats(&[("sentry_attack_rate_fixed", 4.0)]),
            &rates,
        );
        assert_eq!((pinned.base, pinned.min, pinned.max), (4.0, 4.0, 4.0));
        assert_eq!(
            entity_rate("Sentry", &tags, &stats(&[]), &HashMap::new()).base,
            DEFAULT_ENTITY_RATE
        );
    }
}
