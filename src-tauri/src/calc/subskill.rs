use std::collections::HashMap;

pub type StatMap = HashMap<String, f64>;

#[derive(Debug, Clone, Default)]
pub struct SubskillEffect {
    pub base: HashMap<String, f64>,
    pub per_rank: HashMap<String, f64>,
}

#[derive(Debug, Clone, Default)]
pub struct AmountSpec {
    pub base: f64,
    pub per_rank: f64,
}

#[derive(Debug, Clone)]
pub enum AppliedStateSpec {
    Name(String),
    Full {
        state: String,
        amount: Option<AmountSpec>,
    },
}

#[derive(Debug, Clone)]
pub struct SubskillProc {
    pub trigger: String,
    pub chance_base: f64,
    pub chance_per_rank: f64,
    pub effects: Option<SubskillEffect>,
    pub applies_states: Vec<AppliedStateSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct SubskillNode {
    pub id: String,
    pub effects: Option<SubskillEffect>,
    pub proc: Option<SubskillProc>,
}

#[derive(Debug, Clone, Default)]
pub struct SubskillOwner {
    pub id: String,
    pub subskills: Vec<SubskillNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppliedStateInfo {
    pub state: String,
    pub trigger: String,
    pub chance: f64,
    pub amount: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SubtreeAggregation {
    pub stats: StatMap,
    pub proc_stats: StatMap,
    pub applied_states: Vec<AppliedStateInfo>,
}

/// Public contract: matches src/store/build.ts:subskillKey; share-link encoder
/// depends on the colon delimiter.
pub fn subskill_key(skill_id: &str, subskill_id: &str) -> String {
    format!("{}:{}", skill_id, subskill_id)
}

pub fn sum_subskill_ranks(owner: &SubskillOwner, ranks: &HashMap<String, u32>) -> u32 {
    owner
        .subskills
        .iter()
        .map(|sub| {
            ranks
                .get(&subskill_key(&owner.id, &sub.id))
                .copied()
                .unwrap_or(0)
        })
        .sum()
}

pub fn has_allocated_subskill(
    sub: &SubskillNode,
    owner: &SubskillOwner,
    ranks: &HashMap<String, u32>,
) -> bool {
    ranks
        .get(&subskill_key(&owner.id, &sub.id))
        .copied()
        .unwrap_or(0)
        > 0
}

pub fn aggregate_subskill_stats(
    owner: &SubskillOwner,
    subskill_ranks: &HashMap<String, u32>,
    enemy_conditions: Option<&HashMap<String, bool>>,
) -> SubtreeAggregation {
    let mut stats: StatMap = HashMap::new();
    let mut proc_stats: StatMap = HashMap::new();
    let mut applied_states: Vec<AppliedStateInfo> = Vec::new();

    for sub in &owner.subskills {
        let rank = subskill_ranks
            .get(&subskill_key(&owner.id, &sub.id))
            .copied()
            .unwrap_or(0);
        if rank == 0 {
            continue;
        }
        let rank_f = rank as f64;

        if let Some(effects) = sub.effects.as_ref() {
            let has_conditional = effects
                .base
                .keys()
                .chain(effects.per_rank.keys())
                .any(|k| is_conditional_key(k));

            if !has_conditional {
                apply_effect(&mut stats, effects, rank_f, 1.0);
            } else {
                let mut base_unc: HashMap<String, f64> = HashMap::new();
                let mut per_rank_unc: HashMap<String, f64> = HashMap::new();
                let mut base_cond: Vec<(String, f64)> = Vec::new();
                let mut per_rank_cond: Vec<(String, f64)> = Vec::new();

                for (k, v) in &effects.base {
                    if is_conditional_key(k) {
                        base_cond.push((k.clone(), *v));
                    } else {
                        base_unc.insert(k.clone(), *v);
                    }
                }
                for (k, v) in &effects.per_rank {
                    if is_conditional_key(k) {
                        per_rank_cond.push((k.clone(), *v));
                    } else {
                        per_rank_unc.insert(k.clone(), *v);
                    }
                }

                let unc_effect = SubskillEffect {
                    base: base_unc,
                    per_rank: per_rank_unc,
                };
                apply_effect(&mut stats, &unc_effect, rank_f, 1.0);

                for (k, v) in base_cond {
                    if is_condition_active(&k, enemy_conditions) {
                        *stats.entry(k).or_insert(0.0) += v;
                    }
                }
                for (k, v) in per_rank_cond {
                    if is_condition_active(&k, enemy_conditions) {
                        *stats.entry(k).or_insert(0.0) += v * rank_f;
                    }
                }
            }
        }

        if let Some(proc) = sub.proc.as_ref() {
            let chance = proc.chance_base + proc.chance_per_rank * rank_f;
            let factor = chance / 100.0;

            if let Some(eff) = proc.effects.as_ref() {
                if factor > 0.0 {
                    apply_effect(&mut proc_stats, eff, rank_f, factor);
                }
            }

            for state in &proc.applies_states {
                match state {
                    AppliedStateSpec::Name(s) => {
                        applied_states.push(AppliedStateInfo {
                            state: s.clone(),
                            trigger: proc.trigger.clone(),
                            chance,
                            amount: None,
                        });
                    }
                    AppliedStateSpec::Full { state, amount } => {
                        let amt = amount.as_ref().map(|a| a.base + a.per_rank * rank_f);
                        let amt_normalized = amt.filter(|v| *v != 0.0);
                        applied_states.push(AppliedStateInfo {
                            state: state.clone(),
                            trigger: proc.trigger.clone(),
                            chance,
                            amount: amt_normalized,
                        });
                    }
                }
            }
        }
    }

    let mut combined = stats.clone();
    for (k, v) in &proc_stats {
        *combined.entry(k.clone()).or_insert(0.0) += v;
    }

    SubtreeAggregation {
        stats: combined,
        proc_stats,
        applied_states,
    }
}

fn apply_effect(out: &mut StatMap, effect: &SubskillEffect, rank: f64, multiplier: f64) {
    if rank <= 0.0 {
        return;
    }
    for (k, v) in &effect.base {
        *out.entry(k.clone()).or_insert(0.0) += v * multiplier;
    }
    for (k, v) in &effect.per_rank {
        *out.entry(k.clone()).or_insert(0.0) += v * rank * multiplier;
    }
}

const CONDITION_SUFFIXES: &[&str] = &[
    "stasis",
    "slow",
    "lightning_break",
    "burning",
    "poisoned",
    "frozen",
    "shocked",
    "bleeding",
    "stunned",
    "low_life",
];

/// Matches TS regex `/_<suffix>$/`: leading `_` required, suffix must end string.
fn matches_suffix(key: &str, suffix: &str) -> bool {
    key.len() > suffix.len()
        && key.ends_with(suffix)
        && key.as_bytes()[key.len() - suffix.len() - 1] == b'_'
}

fn is_conditional_key(key: &str) -> bool {
    CONDITION_SUFFIXES
        .iter()
        .any(|suffix| matches_suffix(key, suffix))
}

fn is_condition_active(key: &str, enemy_conditions: Option<&HashMap<String, bool>>) -> bool {
    let Some(ec) = enemy_conditions else {
        return false;
    };
    for suffix in CONDITION_SUFFIXES {
        if matches_suffix(key, suffix) {
            return ec.get(*suffix).copied().unwrap_or(false);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(base: &[(&str, f64)], per_rank: &[(&str, f64)]) -> SubskillEffect {
        SubskillEffect {
            base: base.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            per_rank: per_rank.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        }
    }

    fn ranks(entries: &[(&str, &str, u32)]) -> HashMap<String, u32> {
        entries
            .iter()
            .map(|(skill, sub, r)| (subskill_key(skill, sub), *r))
            .collect()
    }

    fn enemy(entries: &[(&str, bool)]) -> HashMap<String, bool> {
        entries.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    // ---- subskill_key ----

    #[test]
    fn subskill_key_uses_colon_delimiter() {
        // Must match src/store/build.ts:subskillKey exactly.
        assert_eq!(subskill_key("fireball", "burn"), "fireball:burn");
        assert_eq!(subskill_key("", ""), ":");
    }

    // ---- sum_subskill_ranks / has_allocated_subskill ----

    #[test]
    fn sum_subskill_ranks_combines_all_nodes() {
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![
                SubskillNode {
                    id: "burn".into(),
                    ..Default::default()
                },
                SubskillNode {
                    id: "linger".into(),
                    ..Default::default()
                },
            ],
        };
        let r = ranks(&[("fireball", "burn", 3), ("fireball", "linger", 2)]);
        assert_eq!(sum_subskill_ranks(&owner, &r), 5);
    }

    #[test]
    fn sum_subskill_ranks_returns_zero_when_empty() {
        let owner = SubskillOwner {
            id: "x".into(),
            subskills: vec![],
        };
        assert_eq!(sum_subskill_ranks(&owner, &HashMap::new()), 0);
    }

    #[test]
    fn has_allocated_subskill_threshold_one() {
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                ..Default::default()
            }],
        };
        let sub = &owner.subskills[0];
        let r1 = ranks(&[("fireball", "burn", 1)]);
        assert!(has_allocated_subskill(sub, &owner, &r1));
        let r0 = ranks(&[("fireball", "burn", 0)]);
        assert!(!has_allocated_subskill(sub, &owner, &r0));
        let r_missing = HashMap::new();
        assert!(!has_allocated_subskill(sub, &owner, &r_missing));
    }

    // ---- is_conditional_key / matches_suffix ----

    #[test]
    fn conditional_key_recognition() {
        assert!(is_conditional_key("extra_damage_burning"));
        assert!(is_conditional_key("extra_damage_low_life"));
        assert!(is_conditional_key("damage_lightning_break"));
        assert!(is_conditional_key("foo_slow"));
        assert!(is_conditional_key("foo_frozen"));
    }

    #[test]
    fn non_conditional_keys_rejected() {
        assert!(!is_conditional_key("damage"));
        assert!(!is_conditional_key("burning")); // TS regex requires a leading "_"
        assert!(!is_conditional_key("burning_other"));
        assert!(!is_conditional_key("low_life_extra")); // suffix in middle, not end
        assert!(!is_conditional_key(""));
    }

    // ---- aggregate_subskill_stats ----

    #[test]
    fn aggregate_empty_owner_returns_empty() {
        let owner = SubskillOwner {
            id: "x".into(),
            subskills: vec![],
        };
        let out = aggregate_subskill_stats(&owner, &HashMap::new(), None);
        assert!(out.stats.is_empty());
        assert!(out.proc_stats.is_empty());
        assert!(out.applied_states.is_empty());
    }

    #[test]
    fn aggregate_unallocated_subskills_skipped() {
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                effects: Some(effect(&[("life", 10.0)], &[])),
                ..Default::default()
            }],
        };
        let out = aggregate_subskill_stats(&owner, &HashMap::new(), None);
        assert!(out.stats.is_empty());
    }

    #[test]
    fn aggregate_base_only_effect() {
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                effects: Some(effect(&[("life", 10.0), ("mana", 5.0)], &[])),
                ..Default::default()
            }],
        };
        let r = ranks(&[("fireball", "burn", 1)]);
        let out = aggregate_subskill_stats(&owner, &r, None);
        assert_eq!(out.stats.get("life"), Some(&10.0));
        assert_eq!(out.stats.get("mana"), Some(&5.0));
    }

    #[test]
    fn aggregate_per_rank_scaled_by_rank() {
        // per_rank is multiplied by rank (NOT rank-1 like passive_stats).
        // rank=3, per_rank=5 → 15.
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                effects: Some(effect(&[], &[("life", 5.0)])),
                ..Default::default()
            }],
        };
        let r = ranks(&[("fireball", "burn", 3)]);
        let out = aggregate_subskill_stats(&owner, &r, None);
        assert_eq!(out.stats.get("life"), Some(&15.0));
    }

    #[test]
    fn aggregate_conditional_key_gated_off() {
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                effects: Some(effect(
                    &[("extra_damage_burning", 50.0), ("life", 10.0)],
                    &[],
                )),
                ..Default::default()
            }],
        };
        let r = ranks(&[("fireball", "burn", 1)]);
        let out = aggregate_subskill_stats(&owner, &r, None);
        assert_eq!(out.stats.get("life"), Some(&10.0));
        assert_eq!(out.stats.get("extra_damage_burning"), None);
    }

    #[test]
    fn aggregate_conditional_key_gated_on() {
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                effects: Some(effect(
                    &[("extra_damage_burning", 50.0), ("life", 10.0)],
                    &[("extra_damage_burning", 5.0)],
                )),
                ..Default::default()
            }],
        };
        let r = ranks(&[("fireball", "burn", 3)]);
        let ec = enemy(&[("burning", true)]);
        let out = aggregate_subskill_stats(&owner, &r, Some(&ec));
        // burning active: base 50 + per_rank 5*3 = 65
        assert_eq!(out.stats.get("extra_damage_burning"), Some(&65.0));
        assert_eq!(out.stats.get("life"), Some(&10.0));
    }

    #[test]
    fn aggregate_proc_chance_and_effects() {
        // chance = 20 + 10*3 = 50; factor = 0.5
        // proc.per_rank life=20 * rank=3 * factor=0.5 = 30
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                proc: Some(SubskillProc {
                    trigger: "on_hit".into(),
                    chance_base: 20.0,
                    chance_per_rank: 10.0,
                    effects: Some(effect(&[], &[("life", 20.0)])),
                    applies_states: vec![],
                }),
                ..Default::default()
            }],
        };
        let r = ranks(&[("fireball", "burn", 3)]);
        let out = aggregate_subskill_stats(&owner, &r, None);
        assert_eq!(out.proc_stats.get("life"), Some(&30.0));
        assert_eq!(out.stats.get("life"), Some(&30.0));
    }

    #[test]
    fn aggregate_applies_states_string_form() {
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                proc: Some(SubskillProc {
                    trigger: "on_hit".into(),
                    chance_base: 25.0,
                    chance_per_rank: 0.0,
                    effects: None,
                    applies_states: vec![AppliedStateSpec::Name("burning".into())],
                }),
                ..Default::default()
            }],
        };
        let r = ranks(&[("fireball", "burn", 2)]);
        let out = aggregate_subskill_stats(&owner, &r, None);
        assert_eq!(out.applied_states.len(), 1);
        let info = &out.applied_states[0];
        assert_eq!(info.state, "burning");
        assert_eq!(info.trigger, "on_hit");
        assert_eq!(info.chance, 25.0);
        assert_eq!(info.amount, None);
    }

    #[test]
    fn aggregate_applies_states_full_with_amount() {
        // amount = 5 + 2*3 = 11
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                proc: Some(SubskillProc {
                    trigger: "on_hit".into(),
                    chance_base: 50.0,
                    chance_per_rank: 0.0,
                    effects: None,
                    applies_states: vec![AppliedStateSpec::Full {
                        state: "burning".into(),
                        amount: Some(AmountSpec {
                            base: 5.0,
                            per_rank: 2.0,
                        }),
                    }],
                }),
                ..Default::default()
            }],
        };
        let r = ranks(&[("fireball", "burn", 3)]);
        let out = aggregate_subskill_stats(&owner, &r, None);
        assert_eq!(out.applied_states[0].amount, Some(11.0));
    }

    #[test]
    fn aggregate_applies_states_zero_amount_collapses_to_none() {
        // TS `amount || undefined` collapses falsy 0 to None.
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                proc: Some(SubskillProc {
                    trigger: "on_hit".into(),
                    chance_base: 50.0,
                    chance_per_rank: 0.0,
                    effects: None,
                    applies_states: vec![AppliedStateSpec::Full {
                        state: "burning".into(),
                        amount: Some(AmountSpec {
                            base: 0.0,
                            per_rank: 0.0,
                        }),
                    }],
                }),
                ..Default::default()
            }],
        };
        let r = ranks(&[("fireball", "burn", 3)]);
        let out = aggregate_subskill_stats(&owner, &r, None);
        assert_eq!(out.applied_states[0].amount, None);
    }

    #[test]
    fn aggregate_combined_stats_adds_proc_on_top() {
        // Deterministic + proc effects on the same key sum in the combined
        // output (matching TS `combined = { ...stats }; for procStats...`).
        let owner = SubskillOwner {
            id: "fireball".into(),
            subskills: vec![SubskillNode {
                id: "burn".into(),
                effects: Some(effect(&[("life", 10.0)], &[])),
                proc: Some(SubskillProc {
                    trigger: "on_hit".into(),
                    chance_base: 100.0,
                    chance_per_rank: 0.0,
                    effects: Some(effect(&[("life", 5.0)], &[])),
                    applies_states: vec![],
                }),
                ..Default::default()
            }],
        };
        let r = ranks(&[("fireball", "burn", 1)]);
        let out = aggregate_subskill_stats(&owner, &r, None);
        assert_eq!(out.stats.get("life"), Some(&15.0));
        assert_eq!(out.proc_stats.get("life"), Some(&5.0));
    }
}
