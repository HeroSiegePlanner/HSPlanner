use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct PassiveStats {
    pub base: HashMap<String, f64>,
    pub per_rank: HashMap<String, f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct ManaCostFormula {
    pub base: f64,
    pub per_level: f64,
}

#[derive(Debug, Clone)]
pub struct SkillRank {
    pub rank: u32,
    pub mana_cost: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct PassiveSkill {
    pub passive_stats: Option<PassiveStats>,
    pub mana_cost_formula: Option<ManaCostFormula>,
    pub ranks: Vec<SkillRank>,
}

/// JS `Math.round` semantics (half rounds toward +∞); diverges from Rust's
/// away-from-zero on .5 ties.
#[inline]
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

pub fn passive_stats_at_rank(skill: &PassiveSkill, rank: u32) -> HashMap<String, f64> {
    let mut out: HashMap<String, f64> = HashMap::new();
    let Some(ps) = skill.passive_stats.as_ref() else {
        return out;
    };
    if rank == 0 {
        return out;
    }
    for (k, v) in &ps.base {
        out.insert(k.clone(), *v);
    }
    let factor = (rank as f64) - 1.0;
    for (k, v) in &ps.per_rank {
        let cur = out.get(k).copied().unwrap_or(0.0);
        out.insert(k.clone(), cur + v * factor);
    }
    for v in out.values_mut() {
        *v = js_round(*v * 1000.0) / 1000.0;
    }
    out
}

pub fn mana_cost_at_rank(skill: &PassiveSkill, rank: u32) -> Option<f64> {
    let r = rank.max(1);
    if let Some(f) = skill.mana_cost_formula {
        return Some((f.base + f.per_level * ((r as f64) - 1.0)).floor());
    }
    if let Some(exact) = skill.ranks.iter().find(|sr| sr.rank == r) {
        if let Some(mc) = exact.mana_cost {
            return Some(mc);
        }
    }
    skill.ranks.first().and_then(|sr| sr.mana_cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passive(base: &[(&str, f64)], per_rank: &[(&str, f64)]) -> PassiveStats {
        PassiveStats {
            base: base.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            per_rank: per_rank.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        }
    }

    // ---- passive_stats_at_rank ----

    #[test]
    fn passive_no_stats_returns_empty() {
        let skill = PassiveSkill::default();
        let out = passive_stats_at_rank(&skill, 5);
        assert!(out.is_empty());
    }

    #[test]
    fn passive_zero_rank_returns_empty() {
        let skill = PassiveSkill {
            passive_stats: Some(passive(&[("life", 10.0)], &[("life", 1.0)])),
            ..Default::default()
        };
        let out = passive_stats_at_rank(&skill, 0);
        assert!(out.is_empty());
    }

    #[test]
    fn passive_base_only_returns_base_values() {
        let skill = PassiveSkill {
            passive_stats: Some(passive(&[("life", 20.0), ("mana", 10.0)], &[])),
            ..Default::default()
        };
        let out = passive_stats_at_rank(&skill, 1);
        assert_eq!(out.get("life"), Some(&20.0));
        assert_eq!(out.get("mana"), Some(&10.0));
    }

    #[test]
    fn passive_per_rank_at_rank_1_is_zero_contribution() {
        // rank=1 means perRank factor is (1-1)=0, so per_rank values don't add yet.
        let skill = PassiveSkill {
            passive_stats: Some(passive(&[], &[("life", 5.0)])),
            ..Default::default()
        };
        let out = passive_stats_at_rank(&skill, 1);
        assert_eq!(out.get("life"), Some(&0.0));
    }

    #[test]
    fn passive_per_rank_progression() {
        // rank=3 → factor = 2. life = 5 * 2 = 10.
        let skill = PassiveSkill {
            passive_stats: Some(passive(&[], &[("life", 5.0)])),
            ..Default::default()
        };
        let out = passive_stats_at_rank(&skill, 3);
        assert_eq!(out.get("life"), Some(&10.0));
    }

    #[test]
    fn passive_base_plus_per_rank() {
        // base=20, per_rank=5, rank=4 → 20 + 5*3 = 35.
        let skill = PassiveSkill {
            passive_stats: Some(passive(&[("life", 20.0)], &[("life", 5.0)])),
            ..Default::default()
        };
        let out = passive_stats_at_rank(&skill, 4);
        assert_eq!(out.get("life"), Some(&35.0));
    }

    #[test]
    fn passive_rounds_to_three_decimals() {
        // base=0.1, per_rank=0.3333, rank=3 → 0.1 + 0.3333*2 = 0.7666.
        // js_round(0.7666 * 1000) / 1000 = js_round(766.6) / 1000 = 767 / 1000 = 0.767.
        let skill = PassiveSkill {
            passive_stats: Some(passive(&[("crit_chance", 0.1)], &[("crit_chance", 0.3333)])),
            ..Default::default()
        };
        let out = passive_stats_at_rank(&skill, 3);
        let v = out.get("crit_chance").copied().unwrap();
        assert!((v - 0.767).abs() < 1e-12, "got {v}");
    }

    #[test]
    fn passive_per_rank_only_for_unmentioned_base_key() {
        // per_rank inserts a key that wasn't in base. Treated as starting from 0.
        // rank=2, per_rank=7 → 0 + 7*1 = 7.
        let skill = PassiveSkill {
            passive_stats: Some(passive(&[], &[("mana", 7.0)])),
            ..Default::default()
        };
        let out = passive_stats_at_rank(&skill, 2);
        assert_eq!(out.get("mana"), Some(&7.0));
    }

    // ---- mana_cost_at_rank ----

    #[test]
    fn mana_cost_no_formula_no_ranks_returns_none() {
        let skill = PassiveSkill::default();
        assert_eq!(mana_cost_at_rank(&skill, 1), None);
    }

    #[test]
    fn mana_cost_formula_floors_result() {
        // base=10, per_level=2.5, rank=3 → 10 + 2.5*2 = 15.0 → floor = 15.
        let skill = PassiveSkill {
            mana_cost_formula: Some(ManaCostFormula {
                base: 10.0,
                per_level: 2.5,
            }),
            ..Default::default()
        };
        assert_eq!(mana_cost_at_rank(&skill, 3), Some(15.0));
        // base=10, per_level=2.5, rank=2 → 12.5 → floor = 12.
        assert_eq!(mana_cost_at_rank(&skill, 2), Some(12.0));
    }

    #[test]
    fn mana_cost_zero_rank_treated_as_rank_1() {
        let skill = PassiveSkill {
            mana_cost_formula: Some(ManaCostFormula {
                base: 5.0,
                per_level: 3.0,
            }),
            ..Default::default()
        };
        // rank=0 clamped to 1 → 5 + 3*0 = 5.
        assert_eq!(mana_cost_at_rank(&skill, 0), Some(5.0));
        assert_eq!(mana_cost_at_rank(&skill, 1), Some(5.0));
    }

    #[test]
    fn mana_cost_exact_rank_lookup_when_no_formula() {
        let skill = PassiveSkill {
            ranks: vec![
                SkillRank {
                    rank: 1,
                    mana_cost: Some(5.0),
                },
                SkillRank {
                    rank: 2,
                    mana_cost: Some(8.0),
                },
                SkillRank {
                    rank: 3,
                    mana_cost: Some(12.0),
                },
            ],
            ..Default::default()
        };
        assert_eq!(mana_cost_at_rank(&skill, 1), Some(5.0));
        assert_eq!(mana_cost_at_rank(&skill, 2), Some(8.0));
        assert_eq!(mana_cost_at_rank(&skill, 3), Some(12.0));
    }

    #[test]
    fn mana_cost_falls_back_to_first_rank_when_exact_missing() {
        // No formula, no exact match — falls back to ranks[0].mana_cost.
        let skill = PassiveSkill {
            ranks: vec![
                SkillRank {
                    rank: 1,
                    mana_cost: Some(7.0),
                },
                SkillRank {
                    rank: 3,
                    mana_cost: Some(15.0),
                },
            ],
            ..Default::default()
        };
        assert_eq!(mana_cost_at_rank(&skill, 2), Some(7.0));
        assert_eq!(mana_cost_at_rank(&skill, 99), Some(7.0));
    }

    #[test]
    fn mana_cost_formula_takes_precedence_over_ranks_table() {
        let skill = PassiveSkill {
            mana_cost_formula: Some(ManaCostFormula {
                base: 100.0,
                per_level: 0.0,
            }),
            ranks: vec![SkillRank {
                rank: 1,
                mana_cost: Some(5.0),
            }],
            ..Default::default()
        };
        // Formula path returns 100, not 5.
        assert_eq!(mana_cost_at_rank(&skill, 1), Some(100.0));
    }
}
