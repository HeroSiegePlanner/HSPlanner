use super::calculation::{number, range, scalar, stat_inputs, CalculationStep};
use std::collections::HashMap;

use super::{
    collect_extra_damage, r_max, r_min, rg, AttrMap, BonusSource, ConditionMap, ExtraSource,
    ItemSkillBonuses, Ranged, ResistMap, Skill, SkillDamageBreakdown, SkillRanks, StatMap,
    ELEMENTS,
};
use crate::calc::affix_tags;
use crate::calc::types::{AffixEffect, DamageScaling};

pub(super) struct ElementKeys {
    pub skills: &'static str,
    pub skill_damage: &'static str,
    pub skill_damage_more: &'static str,
    pub flat_skill_damage: &'static str,
    pub ignore_res: &'static str,
    pub legacy_ignore_res: &'static str,
}

const ELEMENT_KEYS: &[(&str, ElementKeys)] = &[
    (
        "fire",
        ElementKeys {
            skills: "fire_skills",
            skill_damage: "fire_skill_damage",
            skill_damage_more: "fire_skill_damage_more",
            flat_skill_damage: "flat_fire_skill_damage",
            ignore_res: "ignore_fire_res",
            legacy_ignore_res: "enemy_fire_resist",
        },
    ),
    (
        "cold",
        ElementKeys {
            skills: "cold_skills",
            skill_damage: "cold_skill_damage",
            skill_damage_more: "cold_skill_damage_more",
            flat_skill_damage: "flat_cold_skill_damage",
            ignore_res: "ignore_cold_res",
            legacy_ignore_res: "enemy_cold_resist",
        },
    ),
    (
        "lightning",
        ElementKeys {
            skills: "lightning_skills",
            skill_damage: "lightning_skill_damage",
            skill_damage_more: "lightning_skill_damage_more",
            flat_skill_damage: "flat_lightning_skill_damage",
            ignore_res: "ignore_lightning_res",
            legacy_ignore_res: "enemy_lightning_resist",
        },
    ),
    (
        "poison",
        ElementKeys {
            skills: "poison_skills",
            skill_damage: "poison_skill_damage",
            skill_damage_more: "poison_skill_damage_more",
            flat_skill_damage: "flat_poison_skill_damage",
            ignore_res: "ignore_poison_res",
            legacy_ignore_res: "enemy_poison_resist",
        },
    ),
    (
        "arcane",
        ElementKeys {
            skills: "arcane_skills",
            skill_damage: "arcane_skill_damage",
            skill_damage_more: "arcane_skill_damage_more",
            flat_skill_damage: "flat_arcane_skill_damage",
            ignore_res: "ignore_arcane_res",
            legacy_ignore_res: "enemy_arcane_resist",
        },
    ),
    (
        "physical",
        ElementKeys {
            skills: "physical_skills",
            skill_damage: "physical_skill_damage",
            skill_damage_more: "physical_skill_damage_more",
            flat_skill_damage: "flat_physical_skill_damage",
            ignore_res: "ignore_physical_res",
            legacy_ignore_res: "enemy_physical_resist",
        },
    ),
    (
        "magic",
        ElementKeys {
            skills: "magic_skills",
            skill_damage: "magic_skill_damage",
            skill_damage_more: "magic_skill_damage_more",
            flat_skill_damage: "flat_magic_skill_damage",
            ignore_res: "ignore_magic_res",
            legacy_ignore_res: "enemy_magic_resist",
        },
    ),
    (
        "explosion",
        ElementKeys {
            skills: "explosion_skills",
            skill_damage: "explosion_skill_damage",
            skill_damage_more: "explosion_skill_damage_more",
            flat_skill_damage: "flat_explosion_skill_damage",
            ignore_res: "ignore_explosion_res",
            legacy_ignore_res: "enemy_explosion_resist",
        },
    ),
];

pub(super) fn element_keys(damage_type: &str) -> Option<&'static ElementKeys> {
    ELEMENT_KEYS
        .iter()
        .find(|(k, _)| *k == damage_type)
        .map(|(_, v)| v)
}

fn tag_skills_bonus(stats: &StatMap, skill: &Skill) -> (f64, f64) {
    crate::calc::rank::tag_skills_sum(stats, &skill.tags)
}

// Tag-gated "increased skill damage" for the caster's own hit. Returns
// (additive_pct, more_multiplier). Which stat belongs to which tag lives in
// data/affix-tags.json; flat "+X to Guardian Damage" lands in the flat slot of
// compute_skill_damage, not here.
fn archetype_skill_damage(stats: &StatMap, tags: &[String]) -> (Ranged, Ranged) {
    (
        affix_tags::sum_for(AffixEffect::Damage, tags, stats),
        affix_tags::more_for(AffixEffect::DamageMore, tags, stats),
    )
}

// Shared by the spell and attack paths: each bonus source contributes
// value% per effective source rank (or per attribute point). The attack path
// consumes only attack_damage synergies; the spell path everything else
// (attack_rating is no damage stat on either side).
pub(super) fn bonus_source_synergy_pct(
    s: &Skill,
    attributes: &AttrMap,
    stats: &StatMap,
    skill_ranks_by_name: &SkillRanks,
    skills_by_name: &HashMap<String, Skill>,
    item_skill_bonuses: &ItemSkillBonuses,
    for_attack: bool,
    repeated_rank: Option<f64>,
) -> (Ranged, Vec<CalculationStep>) {
    let mut trace = Vec::new();
    let mut synergy_min = 0.0;
    let mut synergy_max = 0.0;
    for bs in &s.bonus_sources {
        // Summons keep attack_damage synergies in their spell hit; for
        // attack-kind skills the attack path consumes them instead.
        let is_attack_kind = s.attack_kind == Some(super::AttackKind::Attack);
        let wanted = if for_attack {
            bs.stat() == "attack_damage"
        } else {
            bs.stat() != "attack_rating" && !(bs.stat() == "attack_damage" && is_attack_kind)
        };
        if !wanted {
            continue;
        }
        match bs {
            BonusSource::AttributePoint { source, value, .. } => {
                let v = attributes.get(source).copied().unwrap_or((0.0, 0.0));
                trace.push(CalculationStep::new(
                    format!("Synergy · {source}"),
                    format!(
                        "{} attribute points × {}% per point",
                        range(v),
                        number(*value)
                    ),
                    (r_min(v) * value, r_max(v) * value),
                ));
                synergy_min += r_min(v) * value;
                synergy_max += r_max(v) * value;
            }
            BonusSource::SkillLevel { source, value, .. } => {
                // LoadTalentDamage's repeat branch uses the repeated cast's
                // level for each skill synergy, even an unlearned source.
                if let Some(rank) = repeated_rank {
                    trace.push(CalculationStep::new(
                        format!("Synergy · {source}"),
                        format!(
                            "{} repeated-cast rank × {}% per rank",
                            number(rank),
                            number(*value)
                        ),
                        scalar(rank * value),
                    ));
                    synergy_min += rank * value;
                    synergy_max += rank * value;
                    continue;
                }
                let br = *skill_ranks_by_name.get(source).unwrap_or(&0.0);
                if br <= 0.0 {
                    continue;
                }
                if let Some(s2) = skills_by_name.get(source) {
                    let all = rg(stats, "all_skills");
                    let (el_min, el_max): Ranged =
                        match s2.damage_type.as_deref().and_then(element_keys) {
                            Some(k) => {
                                let e = rg(stats, k.skills);
                                (r_min(e), r_max(e))
                            }
                            None => (0.0, 0.0),
                        };
                    let it = item_skill_bonuses
                        .get(source)
                        .copied()
                        .unwrap_or((0.0, 0.0));
                    let (tg_min, tg_max) = tag_skills_bonus(stats, s2);
                    let ranks = (
                        br + r_min(all) + el_min + tg_min + it.0,
                        br + r_max(all) + el_max + tg_max + it.1,
                    );
                    trace.push(CalculationStep::new(format!("Synergy · {source}"), format!("({} allocated + {} all skills + {} element + {} tags + {} item ranks) × {}% per rank", number(br), range(all), range((el_min, el_max)), range((tg_min, tg_max)), range(it), number(*value)), (ranks.0 * value, ranks.1 * value)));
                    synergy_min += (br + r_min(all) + el_min + tg_min + it.0) * value;
                    synergy_max += (br + r_max(all) + el_max + tg_max + it.1) * value;
                } else {
                    trace.push(CalculationStep::new(
                        format!("Synergy · {source}"),
                        format!("{} rank × {}% per rank", number(br), number(*value)),
                        scalar(br * value),
                    ));
                    synergy_min += br * value;
                    synergy_max += br * value;
                }
            }
        }
    }
    ((synergy_min, synergy_max), trace)
}

pub struct SkillInput<'a> {
    pub skill: &'a Skill,
    pub allocated_rank: f64,
    pub attributes: &'a AttrMap,
    pub stats: &'a StatMap,
    pub skill_ranks_by_name: &'a SkillRanks,
    pub item_skill_bonuses: &'a ItemSkillBonuses,
    pub enemy_conditions: &'a ConditionMap,
    pub enemy_resistances: &'a ResistMap,
    pub skills_by_name: &'a HashMap<String, Skill>,
    pub projectile_count: u32,
    /// Sum of the skill's own subtree `of_total_damage`, already weighted by
    /// proc chance. Only the owning skill ever sees it.
    pub of_total_damage: f64,
    /// The owning skill's `skillScoped` subtree values. These never enter the
    /// shared stat map, so every key read from here needs its own line.
    pub scoped: &'a StatMap,
    /// Flat damage converted from a build stat (`conversion_*`, mode `flat`),
    /// resolved in `build.rs`.
    pub conversion_flat: f64,
    /// Additive skill damage % converted from a build stat (`conversion_*`,
    /// mode `per500` or self-referential), resolved in `build.rs`.
    pub conversion_skill_damage_pct: f64,
}

pub fn compute_skill_damage(input: &SkillInput<'_>) -> Option<SkillDamageBreakdown> {
    compute_skill_damage_with_rank(input, None, false)
}

/// A triggered cast's explicit level replaces the effective rank in the game;
/// its all/element/tag/item skill bonuses must not be added a second time.
pub(crate) fn compute_skill_damage_at_rank(
    input: &SkillInput<'_>,
    effective_rank: f64,
) -> Option<SkillDamageBreakdown> {
    compute_skill_damage_with_rank(input, Some(effective_rank), false)
}

/// Temporal Echo passes an explicit effective level and the game's repeat flag.
/// See temporal-echo-evidence.md: repeats replace skill-synergy levels and do
/// not enter the on-cast branch that generates additional multicast casts.
pub(crate) fn compute_repeated_skill_damage_at_rank(
    input: &SkillInput<'_>,
    effective_rank: f64,
) -> Option<SkillDamageBreakdown> {
    compute_skill_damage_with_rank(input, Some(effective_rank), true)
}

fn compute_skill_damage_with_rank(
    input: &SkillInput<'_>,
    fixed_rank: Option<f64>,
    repeated: bool,
) -> Option<SkillDamageBreakdown> {
    let s = input.skill;
    if input.allocated_rank == 0.0 {
        return None;
    }
    let has_formula = s.damage_formula.is_some();
    let has_table = s.damage_per_rank.as_ref().is_some_and(|t| !t.is_empty());
    if !has_formula && !has_table {
        return None;
    }

    let keys = s.damage_type.as_deref().and_then(element_keys);
    let staged_elemental = s.damage_scaling == DamageScaling::RankAndFlat;
    let is_spell = s.tags.iter().any(|t| t == "Spell");

    let all_skills = rg(input.stats, "all_skills");
    let (elem_min, elem_max): Ranged = match keys {
        Some(k) => {
            let e = rg(input.stats, k.skills);
            (r_min(e), r_max(e))
        }
        None => (0.0, 0.0),
    };

    let item = input
        .item_skill_bonuses
        .get(&s.name)
        .copied()
        .unwrap_or((0.0, 0.0));
    let (tag_min, tag_max) = tag_skills_bonus(input.stats, s);
    let eff_min = fixed_rank
        .unwrap_or(input.allocated_rank + r_min(all_skills) + elem_min + tag_min + item.0);
    let eff_max = fixed_rank
        .unwrap_or(input.allocated_rank + r_max(all_skills) + elem_max + tag_max + item.1);

    // Clamp to >= 0: a skill never deals negative damage, even when the
    // linear formula extrapolates below zero at low ranks.
    let (base_min, base_max) = if let Some(f) = &s.damage_formula {
        (
            (f.base + f.per_level * eff_min).max(0.0),
            (f.base + f.per_level * eff_max).max(0.0),
        )
    } else {
        let t = s.damage_per_rank.as_ref().unwrap();
        let n = t.len() as i64;
        // Beyond the table, extrapolate at its final per-rank slope so +skills keep scaling base damage.
        let val = |eff: f64, is_max: bool| -> f64 {
            if n == 0 {
                return 0.0;
            }
            let r = (eff as i64).max(1);
            let field = |i: usize| {
                let d = &t[i];
                if is_max {
                    d.max
                } else {
                    d.min
                }
            };
            if r <= n {
                field((r - 1) as usize).max(0.0)
            } else {
                let last = field((n - 1) as usize);
                let prev = if n >= 2 {
                    field((n - 2) as usize)
                } else {
                    last
                };
                (last + (last - prev) * (r - n) as f64).max(0.0)
            }
        };
        (val(eff_min, false), val(eff_max, true))
    };

    // Magic and Elemental Skill Damage cover the five elements; a physical hit never sees them.
    let is_elemental = s
        .damage_type
        .as_deref()
        .is_some_and(|dt| ELEMENTS.contains(&dt));
    let mut flat_keys = vec!["flat_skill_damage"];
    if is_elemental {
        flat_keys.extend(["flat_elemental_skill_damage", "flat_magic_skill_damage"]);
    }
    flat_keys.extend(keys.map(|k| k.flat_skill_damage));
    let mut flat_min = 0.0;
    let mut flat_max = 0.0;
    for k in flat_keys {
        let v = rg(input.stats, k);
        flat_min += r_min(v);
        flat_max += r_max(v);
    }
    let (bound_flat, bound_multiplier) = super::damage_bound_bonuses(input.stats);
    flat_min += bound_flat.0;
    flat_max += bound_flat.1;
    let tag_flat = affix_tags::sum_for(AffixEffect::FlatDamage, &s.tags, input.stats);
    flat_min += tag_flat.0;
    flat_max += tag_flat.1;
    flat_min += input.conversion_flat;
    flat_max += input.conversion_flat;

    let ((synergy_min, synergy_max), synergy_steps) = bonus_source_synergy_pct(
        s,
        input.attributes,
        input.stats,
        input.skill_ranks_by_name,
        input.skills_by_name,
        input.item_skill_bonuses,
        false,
        fixed_rank.filter(|_| repeated),
    );

    let magic = if is_elemental {
        rg(input.stats, "magic_skill_damage")
    } else {
        (0.0, 0.0)
    };
    let elem = keys
        .map(|k| rg(input.stats, k.skill_damage))
        .unwrap_or((0.0, 0.0));
    let (arch_add, arch_more) = if staged_elemental {
        // Runtime 255 and Spell-gated 256 belong to later whole-value stages.
        // Keep other tag sources in their existing stages until source-audited.
        let add = affix_tags::keys_for(AffixEffect::Damage, &s.tags)
            .into_iter()
            .filter(|key| *key != "spell_damage")
            .fold((0.0, 0.0), |acc, key| {
                let value = rg(input.stats, key);
                (acc.0 + value.0, acc.1 + value.1)
            });
        let more = affix_tags::keys_for(AffixEffect::DamageMore, &s.tags)
            .into_iter()
            .filter(|key| *key != "spell_damage_more")
            .fold((1.0, 1.0), |acc, key| {
                let value = rg(input.stats, key);
                (
                    acc.0 * (1.0 + value.0 / 100.0),
                    acc.1 * (1.0 + value.1 / 100.0),
                )
            });
        (add, more)
    } else {
        archetype_skill_damage(input.stats, &s.tags)
    };
    let skill_dmg_min = r_min(magic) + r_min(elem) + arch_add.0 + input.conversion_skill_damage_pct;
    let skill_dmg_max = r_max(magic) + r_max(elem) + arch_add.1 + input.conversion_skill_damage_pct;

    let magic_more = if is_elemental {
        rg(input.stats, "magic_skill_damage_more")
    } else {
        (0.0, 0.0)
    };
    let elem_more = keys
        .map(|k| rg(input.stats, k.skill_damage_more))
        .unwrap_or((0.0, 0.0));
    let skill_more_min =
        (1.0 + r_min(magic_more) / 100.0) * (1.0 + r_min(elem_more) / 100.0) * arch_more.0;
    let skill_more_max =
        (1.0 + r_max(magic_more) / 100.0) * (1.0 + r_max(elem_more) / 100.0) * arch_more.1;

    let (mut extra_mult, mut extra_sources) = collect_extra_damage(
        input.stats,
        input.enemy_conditions,
        s.damage_type.as_deref(),
    );
    let total_damage = if staged_elemental {
        let total = rg(input.stats, "damage");
        let spell_total = if is_spell {
            rg(input.stats, "spell_damage_more")
        } else {
            (0.0, 0.0)
        };
        // Target conditions are evaluated later, independently of runtime190.
        // Preserve their existing additive pool while removing the known total
        // damage source before forming the generic helper's T pool.
        extra_mult = 1.0
            + extra_sources
                .iter()
                .filter(|source| source.stat_key != Some("damage"))
                .map(|source| source.pct)
                .sum::<f64>()
                / 100.0;
        (total.0 + spell_total.0, total.1 + spell_total.1)
    } else {
        (0.0, 0.0)
    };
    let total_mult = (1.0 + total_damage.0 / 100.0, 1.0 + total_damage.1 / 100.0);
    let spell_damage = if staged_elemental {
        rg(input.stats, "spell_damage")
    } else {
        (0.0, 0.0)
    };
    if input.of_total_damage != 0.0 {
        extra_sources.push(ExtraSource {
            stat_key: None,
            label: "Subtree",
            pct: input.of_total_damage,
        });
        extra_mult *= (1.0 + input.of_total_damage / 100.0).max(0.0);
    }
    if super::self_damage_disabled(input.stats, &s.tags) {
        extra_mult = 0.0;
        extra_sources.push(ExtraSource {
            stat_key: Some("self_damage_disabled"),
            label: "Mechanical Engineering: self damage disabled",
            pct: -100.0,
        });
    }

    let extra_pct = (extra_mult * (total_mult.0 + total_mult.1) * 0.5 - 1.0) * 100.0;

    // Crit belongs to the weapon swing the attack path computes; an attack's
    // elemental member only crits on the share a subskill converts.
    let crit_portion = if s.attack_kind == Some(super::AttackKind::Attack) {
        (r_max(rg(input.scoped, "crit_damage_portion"))
            + r_max(rg(input.stats, "crit_damage_portion")))
        .clamp(0.0, 100.0)
            / 100.0
    } else {
        1.0
    };
    let crit = {
        let base = super::crit_factors_with_tags(input.stats, is_spell, &s.tags);
        super::CritFactors {
            chance: if crit_portion > 0.0 { base.chance } else { 0.0 },
            damage_pct: if crit_portion > 0.0 {
                base.damage_pct
            } else {
                0.0
            },
            on_crit_mult: 1.0 - crit_portion + crit_portion * base.on_crit_mult,
            avg_mult: 1.0 - crit_portion + crit_portion * base.avg_mult,
        }
    };

    let enemy_res_pct = s
        .damage_type
        .as_deref()
        .and_then(|dt| input.enemy_resistances.get(dt).copied())
        .unwrap_or(0.0);
    // Compatibility for raw damage API callers using old stat keys. Build stats
    // already normalize these keys and fan ignore_all_res out to each element;
    // never add ignore_all_res here again.
    let raw_ignore = keys
        .map(|k| {
            let implicit = if is_elemental {
                r_max(rg(input.stats, k.legacy_ignore_res))
                    + r_max(rg(input.stats, "enemy_all_resist"))
            } else {
                0.0
            };
            r_max(rg(input.stats, k.ignore_res)) + implicit
        })
        .unwrap_or(0.0);
    let ignore_res_pct = raw_ignore.clamp(0.0, 100.0);
    let eff_res_pct = enemy_res_pct * (1.0 - ignore_res_pct / 100.0);
    let resistance_mult = 1.0 - eff_res_pct / 100.0;

    // LoadAllModifiers selects stat235 when the owning skill's damage types
    // include physical, otherwise stat236. Spell tags do not decide this:
    // Blade Barrier is a Spell with physical + arcane damage.
    let elemental_break_source = if s.attack_scaling.is_some()
        || s.damage_type.as_deref() == Some("physical")
    {
        "elemental_break_on_strike"
    } else {
        "elemental_break_on_spell"
    };
    let elemental_break_pct = if is_elemental {
        let base = r_max(rg(input.stats, "elemental_break"))
            + r_max(rg(input.scoped, "elemental_break"));
        let on = r_max(rg(input.stats, elemental_break_source))
            + r_max(rg(input.scoped, elemental_break_source));
        (base + on).max(0.0)
    } else {
        0.0
    };
    let elemental_break_mult = 1.0 + elemental_break_pct / 100.0;

    // Per-element resistance break applies only to the hit's own element and only
    // while the matching enemy toggle is on; scoped values add to the shared ones.
    let element_break_pct = match s.damage_type.as_deref() {
        Some(dt)
            if ELEMENTS.contains(&dt)
                && *input
                    .enemy_conditions
                    .get(&format!("{dt}_break"))
                    .unwrap_or(&false) =>
        {
            let key = format!("{dt}_break");
            (r_max(rg(input.stats, &key)) + r_max(rg(input.scoped, &key))).max(0.0)
        }
        _ => 0.0,
    };
    // CalculateEndDamage adds the universal and matching typed Break fractions
    // before multiplying damage. The two bonuses do not amplify each other.
    let combined_break_mult = 1.0 + (elemental_break_pct + element_break_pct) / 100.0;
    let (critical_break_average, critical_break_step) = super::critical_break_average_multiplier(
        s.damage_type.as_deref().unwrap_or_default(),
        element_break_pct,
        elemental_break_pct,
        input.stats,
        input.scoped,
    );

    let damage_taken_pct = r_max(rg(input.scoped, "enemy_damage_taken_increased")).max(0.0);
    let damage_taken_mult = 1.0 + damage_taken_pct / 100.0;

    // The runtime elemental path adds the intercept after the rank/flat
    // term's synergy and generic modifiers. Only verified metadata opts in.
    let unscaled_base = if staged_elemental {
        s.damage_formula
            .as_ref()
            .map_or(0.0, |formula| formula.base)
    } else {
        0.0
    };
    let scaled_min = unscaled_base
        + (base_min - unscaled_base + flat_min)
            * (1.0 + synergy_min / 100.0)
            * (1.0 + skill_dmg_min / 100.0);
    let scaled_max = unscaled_base
        + (base_max - unscaled_base + flat_max)
            * (1.0 + synergy_max / 100.0)
            * (1.0 + skill_dmg_max / 100.0);
    let common_subtree = rg(input.scoped, "subtree_damage");
    let elemental_subtree = rg(input.scoped, "elemental_subtree_damage");
    let movement_conversion = rg(input.scoped, "subtree_damage_per_movement_speed");
    let movement_speed = rg(input.stats, "movement_speed");
    let subtree = (
        common_subtree.0
            + elemental_subtree.0
            + movement_conversion.0 * movement_speed.0.max(0.0) / 100.0,
        common_subtree.1
            + elemental_subtree.1
            + movement_conversion.1 * movement_speed.1.max(0.0) / 100.0,
    );
    let subtree_mult = (
        (1.0 + subtree.0 / 100.0).max(0.0),
        (1.0 + subtree.1 / 100.0).max(0.0),
    );
    // LoadTalentDamage's ordinary elemental branch uses ceil; positive stat255
    // first multiplies and floors. The class then floors after its own S pool.
    let generic_damage = |scaled: f64, more: f64, total: f64, spell: f64| {
        let value = scaled * more * total;
        if staged_elemental {
            if spell > 0.0 {
                (value * (1.0 + spell / 100.0)).floor()
            } else {
                value.ceil()
            }
        } else {
            value
        }
    };
    let generic_rounded = (
        generic_damage(
            scaled_min * bound_multiplier.0,
            skill_more_min,
            total_mult.0,
            spell_damage.0,
        ),
        generic_damage(
            scaled_max * bound_multiplier.1,
            skill_more_max,
            total_mult.1,
            spell_damage.1,
        ),
    );

    // Movement transformations retain this fraction of the helper result;
    // their value is not a percentage subtracted from 100 (YYC Amazon166/177).
    let retained = input
        .scoped
        .get("retained_damage_percent")
        .copied()
        .unwrap_or((100.0, 100.0));
    let before_subtree = if input.scoped.contains_key("retained_damage_percent") {
        (
            generic_rounded.0 * retained.0.max(0.0) / 100.0,
            generic_rounded.1 * retained.1.max(0.0) / 100.0,
        )
    } else {
        generic_rounded
    };
    let target_mult = extra_mult
        * damage_taken_mult
        * combined_break_mult
        * resistance_mult;
    let cast_hit = |value: f64| {
        if staged_elemental {
            value.floor()
        } else {
            value
        }
    };
    let hit_min = cast_hit(before_subtree.0 * subtree_mult.0) * target_mult;
    let hit_max = cast_hit(before_subtree.1 * subtree_mult.1) * target_mult;
    // These are positive bonuses in the same additive subtree pool, weighted
    // by their independent cast rolls during aggregation. They affect the
    // expectation, not every ordinary hit. Do not multiply them by S again.
    let proc_subtree = rg(input.scoped, "subtree_damage_on_proc");
    let third_attack = rg(input.scoped, "third_attack_subtree_damage");
    let expected_subtree_mult = (
        (1.0 + (subtree.0 + proc_subtree.0 + third_attack.0 / 3.0) / 100.0).max(0.0),
        (1.0 + (subtree.1 + proc_subtree.1 + third_attack.1 / 3.0) / 100.0).max(0.0),
    );
    let expected_hit = (
        before_subtree.0 * expected_subtree_mult.0 * target_mult * critical_break_average.0,
        before_subtree.1 * expected_subtree_mult.1 * target_mult * critical_break_average.1,
    );

    let crit_min_f = hit_min * crit.on_crit_mult;
    let crit_max_f = hit_max * crit.on_crit_mult;
    // The shared multicast stat stays Spell-gated; a subtree that grants
    // multicast to its own skill applies whatever that skill is tagged.
    let shared_multicast = if is_spell {
        r_max(rg(input.stats, "multicast_chance"))
    } else {
        0.0
    };
    // Multicast re-casts the spell itself. A sentry/summon/guardian skill only
    // spawns its entity, and the game never multicasts that.
    let multicast_chance = if repeated || affix_tags::is_entity(&s.tags) {
        0.0
    } else {
        (shared_multicast + r_max(rg(input.scoped, "multicast_chance"))).max(0.0)
    };
    let multicast_mult = 1.0 + multicast_chance / 100.0;
    let projectiles = input.projectile_count.max(1);
    // Fractional chance-weighted extra projectiles must not be truncated into
    // the ordinary integer projectile count (e.g. Tiny Storms at rank one).
    let extra_projectiles = rg(input.scoped, "expected_additional_projectiles");
    let expected_projectiles = (
        projectiles as f64 + extra_projectiles.0.max(0.0),
        projectiles as f64 + extra_projectiles.1.max(0.0),
    );
    let double_damage = super::double_damage_factor(input.stats, input.scoped);
    let elemental_double = rg(input.scoped, "elemental_double_damage_chance");
    let double_damage = (
        double_damage.0 * (1.0 + elemental_double.0.clamp(0.0, 100.0) / 100.0),
        double_damage.1 * (1.0 + elemental_double.1.clamp(0.0, 100.0) / 100.0),
    );
    let avg_min_f =
        expected_hit.0 * crit.avg_mult * double_damage.0 * multicast_mult * expected_projectiles.0;
    let avg_max_f =
        expected_hit.1 * crit.avg_mult * double_damage.1 * multicast_mult * expected_projectiles.1;

    let mut calculation = Vec::new();
    stat_inputs(&mut calculation, input.stats, ["all_skills"]);
    if let Some(keys) = keys {
        stat_inputs(&mut calculation, input.stats, [keys.skills]);
    }
    stat_inputs(
        &mut calculation,
        input.stats,
        affix_tags::keys_for(AffixEffect::Rank, &s.tags),
    );
    calculation.push(CalculationStep::new(
        "Effective rank",
        if let Some(rank) = fixed_rank {
            format!(
                "Explicit triggered cast rank {}; skill-rank bonuses are not added",
                number(rank)
            )
        } else {
            format!(
                "{} allocated + {} all skills + {} element + {} tags + {} item ranks",
                number(input.allocated_rank),
                range(all_skills),
                range((elem_min, elem_max)),
                range((tag_min, tag_max)),
                range(item)
            )
        },
        (eff_min, eff_max),
    ));
    let base_formula = s.damage_formula.as_ref().map(|f| format!("max(0, {} + {} × {} rank)", number(f.base), number(f.per_level), range((eff_min, eff_max)))).unwrap_or_else(|| "Damage table at effective rank; above the table, extend its final per-rank slope; minimum 0".into());
    calculation.push(CalculationStep::new(
        "Base damage",
        base_formula,
        (base_min, base_max),
    ));
    stat_inputs(
        &mut calculation,
        input.stats,
        [
            "minimum_damage_flat",
            "maximum_damage_flat",
            "minimum_damage_pct",
            "maximum_damage_pct",
        ],
    );
    if bound_flat != (0.0, 0.0) || bound_multiplier != (1.0, 1.0) {
        calculation.push(CalculationStep::new(
            "Minimum/maximum damage bonuses",
            format!("{} flat added to the matching bound in this component's damage type; {} bound multipliers apply before final damage stages", range(bound_flat), range(bound_multiplier)),
            (scaled_min * bound_multiplier.0, scaled_max * bound_multiplier.1),
        ));
    }

    stat_inputs(&mut calculation, input.stats, ["flat_skill_damage"]);
    if is_elemental {
        stat_inputs(
            &mut calculation,
            input.stats,
            [
                "flat_elemental_skill_damage",
                "flat_magic_skill_damage",
                "magic_skill_damage",
                "magic_skill_damage_more",
            ],
        );
    }
    if let Some(keys) = keys {
        stat_inputs(
            &mut calculation,
            input.stats,
            [
                keys.flat_skill_damage,
                keys.skill_damage,
                keys.skill_damage_more,
                keys.ignore_res,
            ],
        );
        if is_elemental {
            stat_inputs(
                &mut calculation,
                input.stats,
                [keys.legacy_ignore_res, "enemy_all_resist"],
            );
        }
    }
    for effect in [
        AffixEffect::FlatDamage,
        AffixEffect::Damage,
        AffixEffect::DamageMore,
    ] {
        stat_inputs(
            &mut calculation,
            input.stats,
            affix_tags::keys_for(effect, &s.tags),
        );
    }
    calculation.push(CalculationStep::new(
        "Converted flat damage",
        "Skill subtree conversion from build stats",
        scalar(input.conversion_flat),
    ));
    calculation.push(CalculationStep::new(
        "Flat added",
        "Sum of matching flat damage stats + converted flat damage",
        (flat_min, flat_max),
    ));
    calculation.extend(synergy_steps);
    calculation.push(CalculationStep::new(
        "Synergy multiplier",
        format!("1 + {}% / 100", range((synergy_min, synergy_max))),
        (1.0 + synergy_min / 100.0, 1.0 + synergy_max / 100.0),
    ));
    calculation.push(CalculationStep::new(
        "Converted skill damage %",
        "Skill subtree conversion from build stats",
        scalar(input.conversion_skill_damage_pct),
    ));
    calculation.push(CalculationStep::new(
        "Increased skill damage multiplier",
        format!(
            "1 + ({} magic + {} element + {} tags + {} conversion)% / 100",
            range(magic),
            range(elem),
            range(arch_add),
            number(input.conversion_skill_damage_pct)
        ),
        (1.0 + skill_dmg_min / 100.0, 1.0 + skill_dmg_max / 100.0),
    ));
    calculation.push(CalculationStep::new(
        "More skill damage multiplier",
        format!(
            "(1 + {}% / 100) × (1 + {}% / 100) × {} tag multipliers",
            range(magic_more),
            range(elem_more),
            range(arch_more)
        ),
        (skill_more_min, skill_more_max),
    ));
    calculation.push(CalculationStep::new(
        "Damage after synergy and generic bonuses",
        format!(
            "{} unscaled base + ({} base − {} unscaled base + {} flat) × {} synergy × {} increased",
            number(unscaled_base),
            range((base_min, base_max)),
            number(unscaled_base),
            range((flat_min, flat_max)),
            range((1.0 + synergy_min / 100.0, 1.0 + synergy_max / 100.0)),
            range((1.0 + skill_dmg_min / 100.0, 1.0 + skill_dmg_max / 100.0))
        ),
        (scaled_min, scaled_max),
    ));
    calculation.push(CalculationStep::new(
        "Own subtree damage multiplier",
        format!(
            "max(0, 1 + {}% / 100); applies to the complete skill damage including its base",
            range(subtree)
        ),
        subtree_mult,
    ));
    if staged_elemental {
        stat_inputs(
            &mut calculation,
            input.stats,
            ["damage", "spell_damage_more", "spell_damage"],
        );
        calculation.push(CalculationStep::new(
            "Total damage multiplier",
            "1 + (damage + Spell-tagged total magic damage) / 100; additive runtime190 + 256 pool",
            total_mult,
        ));
        calculation.push(CalculationStep::new(
            "Damage after generic helper rounding",
            format!("ceil(damage after generic bonuses × more × total); if spell damage {}% is positive, multiply by it and floor first", range(spell_damage)),
            generic_rounded,
        ));
    }
    if input.scoped.contains_key("retained_damage_percent") {
        calculation.push(CalculationStep::new(
            "Damage retained by transformation",
            format!("{} generic damage × {}% retained / 100, before the own-subtree multiplier and cast rounding", range(generic_rounded), range(retained)),
            before_subtree,
        ));
    }
    calculation.push(CalculationStep::new(
        "Expected own subtree damage multiplier",
        format!("max(0, 1 + ({}% guaranteed + {}% chance-weighted bonuses + {}% every-third-attack bonus / 3) / 100)", range(subtree), range(proc_subtree), range(third_attack)),
        expected_subtree_mult,
    ));
    for source in &extra_sources {
        stat_inputs(&mut calculation, input.stats, source.stat_key);
        calculation.push(CalculationStep::new(
            format!("Extra damage · {}", source.label),
            "Applied bonus %; ranged build bonuses use the mean of their endpoints",
            scalar(source.pct),
        ));
    }
    calculation.push(CalculationStep::new(
        "Extra damage multiplier",
        if staged_elemental {
            format!("Target-condition damage pool × max(0, 1 + {}% legacy subtree / 100); total damage has its own earlier stage", number(input.of_total_damage))
        } else { format!(
            "(1 + sum of applicable build extra damage % / 100) × max(0, 1 + {}% subtree / 100)",
            number(input.of_total_damage)
        ) },
        scalar(extra_mult),
    ));
    calculation.push(CalculationStep::new(
        "Enemy damage taken multiplier",
        format!(
            "1 + {}% / 100 from this skill's subtree",
            number(damage_taken_pct)
        ),
        scalar(damage_taken_mult),
    ));
    stat_inputs(
        &mut calculation,
        input.stats,
        [
            "elemental_break",
            elemental_break_source,
        ],
    );
    calculation.push(CalculationStep::new(
        "Elemental Break %",
        format!(
            "{}% contribution to the combined Break multiplier (elemental hits only)",
            number(elemental_break_pct)
        ),
        scalar(elemental_break_pct),
    ));
    calculation.push(CalculationStep::new(
        "Element resistance Break %",
        format!(
            "{}% contribution to the combined Break multiplier; requires matching enemy condition",
            number(element_break_pct)
        ),
        scalar(element_break_pct),
    ));
    calculation.push(CalculationStep::new(
        "Combined Break multiplier",
        format!("1 + ({}% Elemental Break + {}% matching typed Break) / 100; the two Break bonuses add", number(elemental_break_pct), number(element_break_pct)),
        scalar(combined_break_mult),
    ));
    calculation.extend(critical_break_step);
    calculation.push(CalculationStep::new(
        "Effective enemy resistance %",
        format!(
            "{}% configured resistance × (1 − {}% ignored / 100); ignore clamped to 0–100%",
            number(enemy_res_pct),
            number(ignore_res_pct)
        ),
        scalar(eff_res_pct),
    ));
    calculation.push(CalculationStep::new(
        "Resistance multiplier",
        format!("1 − {}% / 100", number(eff_res_pct)),
        scalar(resistance_mult),
    ));
    calculation.push(CalculationStep::new(
        "Hit before rounding",
        format!(
            "{} generic damage × {} own subtree {}; then × {} target modifiers",
            range(before_subtree),
            range(subtree_mult),
            if staged_elemental {
                "(floor at cast)"
            } else {
                ""
            },
            number(target_mult)
        ),
        (hit_min, hit_max),
    ));
    calculation.push(CalculationStep::new(
        "Hit damage",
        "Floor hit before rounding; one projectile",
        (hit_min.floor(), hit_max.floor()),
    ));
    stat_inputs(
        &mut calculation,
        input.stats,
        if is_spell {
            ["spell_crit_chance", "spell_crit_damage"]
        } else {
            ["crit_chance", "crit_damage"]
        },
    );
    let crit_more = if is_spell {
        0.0
    } else {
        r_max(rg(input.stats, "crit_damage_more"))
    };
    if !is_spell {
        stat_inputs(
            &mut calculation,
            input.stats,
            ["crit_damage_more", "crit_damage_portion"],
        );
    }
    calculation.push(CalculationStep::new(
        "Critical damage multiplier",
        format!(
            "1 − {} share + {} share × (1 + {}% critical damage / 100) × (1 + {}% more / 100)",
            number(crit_portion),
            number(crit_portion),
            number(crit.damage_pct),
            number(crit_more)
        ),
        scalar(crit.on_crit_mult),
    ));
    calculation.push(CalculationStep::new(
        "Critical hit",
        format!(
            "floor({} unrounded hit × {})",
            range((hit_min, hit_max)),
            number(crit.on_crit_mult)
        ),
        (crit_min_f.floor(), crit_max_f.floor()),
    ));
    calculation.push(CalculationStep::new(
        "Average critical multiplier",
        format!(
            "1 + clamp({}%, 0, 95) / 100 × ({} critical multiplier − 1)",
            number(crit.chance),
            number(crit.on_crit_mult)
        ),
        scalar(crit.avg_mult),
    ));
    if is_spell {
        stat_inputs(&mut calculation, input.stats, ["multicast_chance"]);
    }
    calculation.push(CalculationStep::new(
        "Multicast multiplier",
        format!(
            "1 + {}% / 100; repeats and entity-spawning skills cannot multicast",
            number(multicast_chance)
        ),
        scalar(multicast_mult),
    ));
    calculation.push(CalculationStep::new(
        "Projectiles",
        "Configured/base projectile count plus subtree modifiers; minimum 1",
        scalar(projectiles as f64),
    ));
    calculation.push(CalculationStep::new(
        "Expected projectiles per cast",
        format!(
            "{} ordinary + {} chance-weighted additional projectiles",
            projectiles,
            range(extra_projectiles)
        ),
        expected_projectiles,
    ));
    calculation.push(CalculationStep::new("Double damage expectation", "(1 + clamp(double damage chance, 0, 100) / 100) × (1 + clamp(element-only double damage chance, 0, 100) / 100); independent of critical hits", double_damage));
    calculation.push(CalculationStep::new(
        "Expected damage before critical hits",
        format!("{} generic damage × {} expected subtree × {} target modifiers × {} Critical Break adjustment; proc expectations are averaged before the final floor", range(before_subtree), range(expected_subtree_mult), number(target_mult), range(critical_break_average)),
        expected_hit,
    ));
    calculation.push(CalculationStep::new(
        "Average damage per cast",
        format!(
            "floor({} expected hit × {} average crit × {} double damage expectation × {} multicast × {} projectiles)",
            range(expected_hit),
            number(crit.avg_mult),
            range(double_damage),
            number(multicast_mult),
            range(expected_projectiles)
        ),
        (avg_min_f.floor(), avg_max_f.floor()),
    ));

    Some(SkillDamageBreakdown {
        calculation,
        effective_rank_min: eff_min,
        effective_rank_max: eff_max,
        base_min,
        base_max,
        flat_min,
        flat_max,
        synergy_min_pct: synergy_min,
        synergy_max_pct: synergy_max,
        skill_damage_min_pct: skill_dmg_min,
        skill_damage_max_pct: skill_dmg_max,
        extra_damage_pct: extra_pct,
        extra_damage_sources: extra_sources,
        crit_chance: crit.chance,
        crit_damage_pct: crit.damage_pct,
        crit_multiplier_avg: crit.avg_mult,
        multicast_chance_pct: multicast_chance,
        multicast_multiplier: multicast_mult,
        projectile_count: projectiles,
        elemental_break_pct,
        elemental_break_multiplier: elemental_break_mult,
        enemy_resistance_pct: enemy_res_pct,
        resistance_ignored_pct: ignore_res_pct,
        effective_resistance_pct: eff_res_pct,
        resistance_multiplier: resistance_mult,
        hit_min: hit_min.floor() as i64,
        hit_max: hit_max.floor() as i64,
        crit_min: crit_min_f.floor() as i64,
        crit_max: crit_max_f.floor() as i64,
        final_min: hit_min.floor() as i64,
        final_max: hit_max.floor() as i64,
        avg_min: avg_min_f.floor() as i64,
        avg_max: avg_max_f.floor() as i64,
    })
}

#[cfg(test)]
#[path = "damage_stage_tests.rs"]
mod stage_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(t: &[&str]) -> Vec<String> {
        t.iter().map(|s| s.to_string()).collect()
    }

    fn stats(pairs: &[(&str, f64)]) -> StatMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), (*v, *v)))
            .collect()
    }

    fn plain_skill() -> Skill {
        Skill {
            name: "Test Bolt".into(),
            tags: tags(&["Spell"]),
            damage_type: Some("lightning".into()),
            damage_formula: Some(crate::calc::skills::DamageFormula {
                base: 100.0,
                per_level: 0.0,
            }),
            damage_scaling: DamageScaling::Full,
            damage_per_rank: None,
            bonus_sources: Vec::new(),
            attack_kind: None,
            attack_scaling: None,
        }
    }

    fn hit_max_with(of_total_damage: f64) -> i64 {
        let skill = plain_skill();
        let empty_stats = StatMap::new();
        let empty_attrs = AttrMap::new();
        let ranks = SkillRanks::new();
        let bonuses = ItemSkillBonuses::new();
        let conds = ConditionMap::new();
        let resists = ResistMap::new();
        let by_name = HashMap::new();
        let input = SkillInput {
            skill: &skill,
            allocated_rank: 1.0,
            attributes: &empty_attrs,
            stats: &empty_stats,
            skill_ranks_by_name: &ranks,
            item_skill_bonuses: &bonuses,
            enemy_conditions: &conds,
            enemy_resistances: &resists,
            skills_by_name: &by_name,
            projectile_count: 1,
            of_total_damage,
            scoped: &empty_stats,
            conversion_flat: 0.0,
            conversion_skill_damage_pct: 0.0,
        };
        compute_skill_damage(&input).expect("breakdown").hit_max
    }

    #[test]
    fn repeated_cast_uses_explicit_rank_for_unlearned_synergy_without_multicast() {
        let mut skill = plain_skill();
        skill.tags = vec!["Spell".into()];
        skill.damage_formula = Some(super::super::DamageFormula {
            base: 4.0,
            per_level: 18.0,
        });
        skill.bonus_sources = vec![
            BonusSource::SkillLevel {
                source: "unlearned".into(),
                stat: "fire_skill_damage".into(),
                value: 15.0,
            },
            BonusSource::AttributePoint {
                source: "Intelligence".into(),
                stat: "fire_skill_damage".into(),
                value: 10.0,
            },
        ];
        let attrs = AttrMap::from([("Intelligence".into(), (2.0, 2.0))]);
        let stats = StatMap::from([
            ("all_skills".into(), (100.0, 100.0)),
            ("multicast_chance".into(), (100.0, 100.0)),
        ]);
        let scoped = StatMap::from([("multicast_chance".into(), (100.0, 100.0))]);
        let input = SkillInput {
            skill: &skill,
            allocated_rank: 2.0,
            attributes: &attrs,
            stats: &stats,
            skill_ranks_by_name: &SkillRanks::new(),
            item_skill_bonuses: &ItemSkillBonuses::new(),
            enemy_conditions: &ConditionMap::new(),
            enemy_resistances: &ResistMap::new(),
            skills_by_name: &HashMap::new(),
            projectile_count: 1,
            of_total_damage: 0.0,
            scoped: &scoped,
            conversion_flat: 0.0,
            conversion_skill_damage_pct: 0.0,
        };
        let echo = compute_repeated_skill_damage_at_rank(&input, 3.0).unwrap();
        assert_eq!(
            (echo.effective_rank_min, echo.effective_rank_max),
            (3.0, 3.0)
        );
        assert_eq!(echo.base_max, 58.0);
        assert_eq!(echo.synergy_max_pct, 65.0); // 3 × 15 + 2 × 10
        assert_eq!(echo.multicast_multiplier, 1.0);
        assert_eq!(echo.avg_max, 95); // floor(58 × 1.65)

        let ordinary = compute_skill_damage_at_rank(&input, 3.0).unwrap();
        assert_eq!(
            ordinary.synergy_max_pct, 20.0,
            "ordinary explicit-rank procs retain their own synergy semantics"
        );
        assert_eq!(ordinary.multicast_multiplier, 3.0);
    }

    struct Case {
        damage_type: &'static str,
        tags: Vec<String>,
        attack_kind: Option<super::super::AttackKind>,
        stats: StatMap,
        scoped: StatMap,
        conditions: ConditionMap,
        conversion_flat: f64,
        conversion_skill_damage_pct: f64,
    }

    impl Default for Case {
        fn default() -> Self {
            Case {
                damage_type: "lightning",
                tags: tags(&["Spell"]),
                attack_kind: None,
                stats: StatMap::new(),
                scoped: StatMap::new(),
                conditions: ConditionMap::new(),
                conversion_flat: 0.0,
                conversion_skill_damage_pct: 0.0,
            }
        }
    }

    fn breakdown(case: &Case) -> SkillDamageBreakdown {
        let mut skill = plain_skill();
        skill.damage_type = Some(case.damage_type.to_string());
        skill.tags = case.tags.clone();
        skill.attack_kind = case.attack_kind;
        let empty_attrs = AttrMap::new();
        let ranks = SkillRanks::new();
        let bonuses = ItemSkillBonuses::new();
        let resists = ResistMap::new();
        let by_name = HashMap::new();
        let input = SkillInput {
            skill: &skill,
            allocated_rank: 1.0,
            attributes: &empty_attrs,
            stats: &case.stats,
            skill_ranks_by_name: &ranks,
            item_skill_bonuses: &bonuses,
            enemy_conditions: &case.conditions,
            enemy_resistances: &resists,
            skills_by_name: &by_name,
            projectile_count: 1,
            of_total_damage: 0.0,
            scoped: &case.scoped,
            conversion_flat: case.conversion_flat,
            conversion_skill_damage_pct: case.conversion_skill_damage_pct,
        };
        compute_skill_damage(&input).expect("breakdown")
    }

    fn cond(active: &[&str]) -> ConditionMap {
        active.iter().map(|c| (c.to_string(), true)).collect()
    }

    #[test]
    fn element_break_needs_the_matching_enemy_toggle() {
        let off = breakdown(&Case {
            damage_type: "fire",
            stats: stats(&[("fire_break", 100.0)]),
            ..Default::default()
        });
        assert_eq!(off.hit_max, 100);
        let on = breakdown(&Case {
            damage_type: "fire",
            stats: stats(&[("fire_break", 100.0)]),
            conditions: cond(&["fire_break"]),
            ..Default::default()
        });
        assert_eq!(on.hit_max, 200);
    }

    #[test]
    fn critical_break_changes_only_the_matching_break_average() {
        for element in ELEMENTS {
            let key = format!("{element}_break");
            let mut case = Case {
                damage_type: element,
                stats: stats(&[
                    (&key, 100.0),
                    (&format!("{element}_break_crit_chance"), 15.0),
                    (&format!("{element}_break_crit_damage"), 25.0),
                ]),
                ..Default::default()
            };
            let off = breakdown(&case);
            assert_eq!((off.hit_max, off.avg_max), (100, 100));
            case.conditions.insert(key, true);
            let on = breakdown(&case);
            // 100 base + 100 Break + 100 Break × 15% chance × 25% damage.
            assert_eq!((on.hit_max, on.avg_max), (200, 203));
            case.stats.insert("elemental_break".into(), (50.0, 50.0));
            let both = breakdown(&case);
            // Universal Break does not gain typed Critical Break's bonus.
            assert_eq!((both.hit_max, both.avg_max), (250, 253));
            case.stats.remove("elemental_break");
            case.stats
                .insert("spell_crit_chance".into(), (100.0, 100.0));
            case.stats
                .insert("spell_crit_damage".into(), (100.0, 100.0));
            let ordinary_crit = breakdown(&case);
            assert_eq!(ordinary_crit.hit_max, 200);
            assert!(ordinary_crit.avg_max > on.avg_max);
            case.stats.remove(&format!("{element}_break_crit_chance"));
            case.scoped
                .insert(format!("{element}_break_crit_chance"), (100.0, 150.0));
            let always = breakdown(&case);
            // Independent ordinary crit and Critical Break multiply once.
            assert_eq!(always.avg_max, (225.0 * always.crit_multiplier_avg) as i64);
            case.damage_type = "physical";
            let physical = breakdown(&case);
            assert_eq!(physical.hit_max, 100);
        }
    }

    #[test]
    fn element_break_only_applies_to_its_own_element() {
        let cold = breakdown(&Case {
            damage_type: "cold",
            stats: stats(&[("fire_break", 100.0)]),
            conditions: cond(&["fire_break"]),
            ..Default::default()
        });
        assert_eq!(cold.hit_max, 100);
    }

    #[test]
    fn lightning_break_from_the_subtree_reaches_the_hit() {
        // Regression: lightning_break is skillScoped, so before E3 the subtree
        // value was dropped on the floor.
        let scoped_only = breakdown(&Case {
            scoped: stats(&[("lightning_break", 150.0)]),
            conditions: cond(&["lightning_break"]),
            ..Default::default()
        });
        assert_eq!(scoped_only.hit_max, 250);
    }

    #[test]
    fn enemy_damage_taken_increased_multiplies_the_hit() {
        let out = breakdown(&Case {
            scoped: stats(&[("enemy_damage_taken_increased", 25.0)]),
            ..Default::default()
        });
        assert_eq!(out.hit_max, 125);
    }

    #[test]
    fn subtree_multicast_applies_to_a_non_spell_skill() {
        let shared = breakdown(&Case {
            tags: tags(&["Active"]),
            stats: stats(&[("multicast_chance", 50.0)]),
            ..Default::default()
        });
        assert_eq!(shared.multicast_chance_pct, 0.0, "shared stays Spell-gated");
        let scoped = breakdown(&Case {
            tags: tags(&["Active"]),
            scoped: stats(&[("multicast_chance", 50.0)]),
            ..Default::default()
        });
        assert_eq!(scoped.multicast_chance_pct, 50.0);
        assert_eq!(scoped.avg_max, 150);
    }

    #[test]
    fn conversions_land_in_the_flat_and_skill_damage_slots() {
        let flat = breakdown(&Case {
            conversion_flat: 100.0,
            ..Default::default()
        });
        assert_eq!(flat.flat_max, 100.0);
        assert_eq!(flat.hit_max, 200);
        let pct = breakdown(&Case {
            conversion_skill_damage_pct: 50.0,
            ..Default::default()
        });
        assert_eq!(pct.skill_damage_max_pct, 50.0);
        assert_eq!(pct.hit_max, 150);
    }

    #[test]
    fn of_total_damage_multiplies_the_hit() {
        let base = hit_max_with(0.0);
        assert_eq!(base, 100);
        // 150% of total damage → 2.5× the hit.
        assert_eq!(hit_max_with(150.0), 250);
    }

    #[test]
    fn ranged_double_damage_preserves_spell_hit_and_average_bounds() {
        let result = breakdown(&Case {
            stats: StatMap::from([("double_damage_chance".into(), (0.0, 50.0))]),
            scoped: StatMap::from([("double_damage_chance".into(), (0.0, 50.0))]),
            ..Default::default()
        });
        assert_eq!((result.hit_min, result.hit_max), (100, 100));
        assert_eq!((result.avg_min, result.avg_max), (100, 200));
        assert_eq!(result.projectile_count, 1);
        assert_eq!(
            result
                .calculation()
                .iter()
                .find(|s| s.label() == "Double damage expectation")
                .unwrap()
                .value(),
            (1.0, 2.0)
        );
    }

    #[test]
    fn negative_of_total_damage_reduces_the_hit_and_stops_at_zero() {
        assert_eq!(hit_max_with(-50.0), 50);
        assert_eq!(hit_max_with(-100.0), 0);
        assert_eq!(hit_max_with(-150.0), 0);
    }

    #[test]
    fn archetype_orbital_applies_additive_and_more() {
        let s = stats(&[
            ("orbital_skill_damage", 30.0),
            ("orbital_skill_damage_more", 50.0),
        ]);
        let (add, more) = archetype_skill_damage(&s, &tags(&["Orbital"]));
        assert_eq!(add, (30.0, 30.0));
        assert_eq!(more, (1.5, 1.5));
    }

    #[test]
    fn archetype_ignored_when_tag_absent() {
        let s = stats(&[("orbital_skill_damage", 30.0), ("explosion_damage", 40.0)]);
        let (add, more) = archetype_skill_damage(&s, &tags(&["Spell"]));
        assert_eq!(add, (0.0, 0.0));
        assert_eq!(more, (1.0, 1.0));
    }

    #[test]
    fn archetype_spell_aoe_requires_both_tags() {
        let s = stats(&[("spell_aoe_damage", 20.0)]);
        assert_eq!(
            archetype_skill_damage(&s, &tags(&["Spell"])).0,
            (0.0, 0.0),
            "AoE damage must not apply to a non-AoE spell",
        );
        assert_eq!(
            archetype_skill_damage(&s, &tags(&["Spell", "Area of Effect"])).0,
            (20.0, 20.0),
        );
    }

    // Weakening Precision converts 75% of Frost Sunder's cold hit into damage
    // that can crit; without it an attack's elemental member never crits.
    #[test]
    fn crit_damage_portion_lets_an_attacks_elemental_member_crit() {
        let crit = stats(&[("crit_chance", 100.0), ("crit_damage", 100.0)]);
        let case = |scoped: StatMap| Case {
            damage_type: "cold",
            tags: tags(&["Attack", "Melee"]),
            attack_kind: Some(super::super::AttackKind::Attack),
            stats: crit.clone(),
            scoped,
            ..Default::default()
        };
        let none = breakdown(&case(StatMap::new()));
        assert_eq!(none.crit_multiplier_avg, 1.0, "no portion, no crit");
        assert_eq!(none.crit_chance, 0.0);

        let converted = breakdown(&case(stats(&[("crit_damage_portion", 75.0)])));
        // crit chance caps at 95%, so the doubling averages 1.95 on 75% of the hit.
        assert!(
            (converted.crit_multiplier_avg - 1.7125).abs() < 1e-9,
            "got {}",
            converted.crit_multiplier_avg
        );
        assert_eq!(converted.crit_chance, 100.0);
    }

    #[test]
    fn archetype_spell_damage_needs_the_spell_tag() {
        let s = stats(&[("spell_damage", 10.0), ("spell_damage_more", 25.0)]);
        assert_eq!(
            archetype_skill_damage(&s, &tags(&["Spell"])),
            ((10.0, 10.0), (1.25, 1.25))
        );
        assert_eq!(
            archetype_skill_damage(&s, &tags(&["Attack", "Melee"])),
            ((0.0, 0.0), (1.0, 1.0)),
        );
    }

    #[test]
    fn archetype_sentry_applies_additive_and_more() {
        let s = stats(&[("sentry_damage", 30.0), ("sentry_damage_more", 50.0)]);
        let (add, more) = archetype_skill_damage(&s, &tags(&["Sentry"]));
        assert_eq!(add, (30.0, 30.0));
        assert_eq!(more, (1.5, 1.5));
    }

    #[test]
    fn archetype_summon_is_additive_only() {
        let s = stats(&[("summon_damage", 25.0)]);
        let (add, more) = archetype_skill_damage(&s, &tags(&["Summon"]));
        assert_eq!(add, (25.0, 25.0));
        assert_eq!(more, (1.0, 1.0));
    }

    #[test]
    fn guardian_flat_damage_lands_in_the_flat_slot_for_guardian_skills() {
        let b = breakdown(&Case {
            tags: tags(&["Spell", "Guardian"]),
            stats: stats(&[("guardian_damage", 50.0)]),
            ..Default::default()
        });
        assert_eq!(b.flat_max, 50.0);
        assert_eq!(b.hit_max, 150);
        let plain = breakdown(&Case {
            stats: stats(&[("guardian_damage", 50.0)]),
            ..Default::default()
        });
        assert_eq!(
            plain.hit_max, 100,
            "flat must not leak to non-Guardian skills"
        );
    }

    #[test]
    fn archetype_explosion_is_additive_only() {
        let s = stats(&[("explosion_damage", 40.0)]);
        let (add, more) = archetype_skill_damage(&s, &tags(&["Explosion"]));
        assert_eq!(add, (40.0, 40.0));
        assert_eq!(more, (1.0, 1.0));
    }

    // LoadMonsterBreaks returns separate fractions; CalculateEndDamage adds
    // them before multiplying damage. 100 base × (1 + .5 + .5) = 200.
    #[test]
    fn element_break_and_elemental_break_add_before_multiplying_damage() {
        let both = breakdown(&Case {
            damage_type: "fire",
            stats: stats(&[("elemental_break", 50.0), ("fire_break", 50.0)]),
            conditions: cond(&["fire_break"]),
            ..Default::default()
        });
        assert_eq!(both.elemental_break_pct, 50.0);
        assert_eq!(both.hit_max, 200);
        let elemental_only = breakdown(&Case {
            damage_type: "fire",
            stats: stats(&[("elemental_break", 50.0)]),
            conditions: cond(&["fire_break"]),
            ..Default::default()
        });
        assert_eq!(elemental_only.hit_max, 150);
    }

    #[test]
    fn tag_skills_bonus_requires_the_matching_tag() {
        let s = stats(&[("projectile_skills", 4.0), ("sentry_skills", 3.0)]);
        let mut skill = Skill {
            name: "Test Throw".to_string(),
            tags: tags(&["Active", "Projectile"]),
            damage_type: Some("physical".to_string()),
            damage_formula: None,
            damage_scaling: DamageScaling::Full,
            damage_per_rank: None,
            bonus_sources: Vec::new(),
            attack_kind: None,
            attack_scaling: None,
        };
        assert_eq!(tag_skills_bonus(&s, &skill), (4.0, 4.0));
        skill.tags = tags(&["Active", "Projectile", "Sentry"]);
        assert_eq!(tag_skills_bonus(&s, &skill), (7.0, 7.0), "tag keys stack");
        skill.tags = tags(&["Active", "Spell"]);
        assert_eq!(tag_skills_bonus(&s, &skill), (0.0, 0.0));
    }

    // Magic Skill Damage keys off the damage type, not the tags: an attack
    // skill's elemental member counts like any other elemental hit.
    #[test]
    fn magic_skill_damage_follows_the_damage_type_not_the_tags() {
        for key in [
            "magic_skill_damage",
            "magic_skill_damage_more",
            "flat_magic_skill_damage",
        ] {
            let attack_tags = tags(&["Attack", "Melee"]);
            let cold_baseline = breakdown(&Case {
                damage_type: "cold",
                tags: attack_tags.clone(),
                ..Default::default()
            });
            let cold = breakdown(&Case {
                damage_type: "cold",
                tags: attack_tags.clone(),
                stats: stats(&[(key, 100.0)]),
                ..Default::default()
            });
            assert!(
                cold.hit_max > cold_baseline.hit_max,
                "{key} must reach an attack skill's elemental member"
            );

            let phys_baseline = breakdown(&Case {
                damage_type: "physical",
                tags: attack_tags.clone(),
                ..Default::default()
            });
            let phys = breakdown(&Case {
                damage_type: "physical",
                tags: attack_tags,
                stats: stats(&[(key, 100.0)]),
                ..Default::default()
            });
            assert_eq!(
                phys.hit_max, phys_baseline.hit_max,
                "{key} must stay off a physical hit"
            );
        }
    }

    // Crit belongs to the weapon swing; the elemental member an attack skill
    // adds on top of it never crits.
    #[test]
    fn attack_skills_do_not_crit_on_their_elemental_member() {
        let spell = breakdown(&Case {
            damage_type: "fire",
            stats: stats(&[("spell_crit_chance", 50.0), ("spell_crit_damage", 100.0)]),
            ..Default::default()
        });
        assert!(
            spell.crit_multiplier_avg > 1.0,
            "a plain spell must still crit, got {}",
            spell.crit_multiplier_avg
        );

        let attack = breakdown(&Case {
            damage_type: "fire",
            tags: tags(&["Attack", "Melee"]),
            attack_kind: Some(super::super::AttackKind::Attack),
            stats: stats(&[("crit_chance", 50.0), ("crit_damage", 100.0)]),
            ..Default::default()
        });
        assert_eq!(
            attack.crit_multiplier_avg, 1.0,
            "elemental member must not crit"
        );
        assert_eq!(attack.crit_chance, 0.0);
        assert_eq!(attack.avg_max, attack.hit_max);
    }

    #[test]
    fn flat_magic_skill_damage_is_a_flat_for_every_element_but_physical() {
        let fire = breakdown(&Case {
            damage_type: "fire",
            stats: stats(&[("flat_magic_skill_damage", 50.0)]),
            ..Default::default()
        });
        assert_eq!(fire.flat_max, 50.0);
        assert_eq!(fire.hit_max, 150);
        let physical = breakdown(&Case {
            damage_type: "physical",
            stats: stats(&[("flat_magic_skill_damage", 50.0)]),
            ..Default::default()
        });
        assert_eq!(physical.flat_max, 0.0);
        assert_eq!(physical.hit_max, 100);
    }

    #[test]
    fn flat_elemental_skill_damage_skips_physical() {
        let fire = breakdown(&Case {
            damage_type: "fire",
            stats: stats(&[("flat_elemental_skill_damage", 50.0)]),
            ..Default::default()
        });
        assert_eq!(fire.flat_max, 50.0);
        let physical = breakdown(&Case {
            damage_type: "physical",
            stats: stats(&[("flat_elemental_skill_damage", 50.0)]),
            ..Default::default()
        });
        assert_eq!(physical.flat_max, 0.0);
        assert_eq!(physical.hit_max, 100);
    }

    #[test]
    fn magic_skill_damage_shares_one_bracket_with_the_element_and_skips_physical() {
        let fire = breakdown(&Case {
            damage_type: "fire",
            stats: stats(&[("magic_skill_damage", 50.0), ("fire_skill_damage", 50.0)]),
            ..Default::default()
        });
        assert_eq!(fire.skill_damage_max_pct, 100.0, "one additive bracket");
        assert_eq!(fire.hit_max, 200);
        let physical = breakdown(&Case {
            damage_type: "physical",
            stats: stats(&[
                ("magic_skill_damage", 50.0),
                ("magic_skill_damage_more", 50.0),
            ]),
            ..Default::default()
        });
        assert_eq!(physical.skill_damage_max_pct, 0.0);
        assert_eq!(physical.hit_max, 100);
    }

    #[test]
    fn aggregated_ignore_all_counts_once_in_elemental_damage_and_caps_at_100() {
        use crate::calc::stats::{
            apply_stat_fan_outs, compute_final_stats, push_source, SourceContribution, SourceMap,
            SourceType,
        };
        for all in [10., 110.] {
            let mut sources = SourceMap::new();
            for (key, amount) in [("ignore_all_res", all), ("ignore_cold_res", 20.)] {
                push_source(
                    &mut sources,
                    key,
                    SourceContribution {
                        label: "test item".into(),
                        source_type: SourceType::Item,
                        value: (amount, amount),
                        forge: None,
                    },
                );
            }
            apply_stat_fan_outs(&mut sources);
            let stats = compute_final_stats(&sources);
            for damage_type in ["fire", "cold", "lightning", "poison", "arcane", "physical"] {
                let result = breakdown(&Case {
                    damage_type,
                    stats: stats.clone(),
                    ..Default::default()
                });
                let expected = match damage_type {
                    "physical" => 0.,
                    "cold" => (all + 20.).min(100.),
                    _ => all.min(100.),
                };
                assert_eq!(result.resistance_ignored_pct, expected, "{damage_type}");
            }
        }
    }

    #[test]
    fn item_implicit_enemy_resist_keys_count_as_pierce() {
        let cold = breakdown(&Case {
            damage_type: "cold",
            stats: stats(&[
                ("ignore_cold_res", 20.0),
                ("enemy_cold_resist", 30.0),
                ("enemy_all_resist", 10.0),
            ]),
            ..Default::default()
        });
        assert_eq!(cold.resistance_ignored_pct, 60.0);
        let physical = breakdown(&Case {
            damage_type: "physical",
            stats: stats(&[("enemy_all_resist", 10.0)]),
            ..Default::default()
        });
        assert_eq!(physical.resistance_ignored_pct, 0.0);
    }
    #[test]
    fn explanation_keeps_more_extra_crit_and_conversion_stages_reconcilable() {
        let d = breakdown(&Case {
            stats: stats(&[
                ("lightning_skill_damage", 50.0),
                ("lightning_skill_damage_more", 100.0),
                ("extra_damage_burning", 20.0),
                ("extra_damage_poisoned", 30.0),
                ("spell_crit_chance", 100.0),
                ("spell_crit_damage", 100.0),
                ("multicast_chance", 50.0),
            ]),
            conditions: cond(&["burning", "poisoned"]),
            conversion_flat: 20.0,
            ..Default::default()
        });
        let step = |label| {
            d.calculation()
                .iter()
                .find(|step| step.label() == label)
                .unwrap()
        };
        assert_eq!(step("More skill damage multiplier").value(), (2.0, 2.0));
        assert_eq!(step("Extra damage multiplier").value(), (1.5, 1.5));
        assert_eq!(step("Average critical multiplier").value(), (1.95, 1.95));
        assert_eq!(step("Hit before rounding").value(), (540.0, 540.0));
        assert_eq!(
            step("Average damage per cast").value(),
            (d.avg_min as f64, d.avg_max as f64)
        );
        assert!(d
            .calculation()
            .iter()
            .any(|step| step.stat_key() == Some("lightning_skill_damage_more")));
        assert!(serde_json::to_value(&d)
            .unwrap()
            .get("calculation")
            .is_none());
    }
}
