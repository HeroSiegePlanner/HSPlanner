use std::collections::HashMap;

use super::{
    AttrMap, BonusSource, ConditionMap, ELEMENTS, ExtraSource, ItemSkillBonuses, Ranged, ResistMap,
    Skill, SkillDamageBreakdown, SkillRanks, StatMap, collect_extra_damage, crit_factors, r_max,
    r_min, rg,
};
use crate::calc::affix_tags;
use crate::calc::types::AffixEffect;

pub(super) struct ElementKeys {
    pub skills: &'static str,
    pub skill_damage: &'static str,
    pub skill_damage_more: &'static str,
    pub flat_skill_damage: &'static str,
    pub ignore_res: &'static str,
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
) -> Ranged {
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
                synergy_min += r_min(v) * value;
                synergy_max += r_max(v) * value;
            }
            BonusSource::SkillLevel { source, value, .. } => {
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
                    synergy_min += (br + r_min(all) + el_min + tg_min + it.0) * value;
                    synergy_max += (br + r_max(all) + el_max + tg_max + it.1) * value;
                } else {
                    synergy_min += br * value;
                    synergy_max += br * value;
                }
            }
        }
    }
    (synergy_min, synergy_max)
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
    let s = input.skill;
    if input.allocated_rank == 0.0 {
        return None;
    }
    let has_formula = s.damage_formula.is_some();
    let has_table = s
        .damage_per_rank
        .as_ref()
        .is_some_and(|t| !t.is_empty());
    if !has_formula && !has_table {
        return None;
    }

    let keys = s.damage_type.as_deref().and_then(element_keys);

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
    let eff_min = input.allocated_rank + r_min(all_skills) + elem_min + tag_min + item.0;
    let eff_max = input.allocated_rank + r_max(all_skills) + elem_max + tag_max + item.1;

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
                if is_max { d.max } else { d.min }
            };
            if r <= n {
                field((r - 1) as usize).max(0.0)
            } else {
                let last = field((n - 1) as usize);
                let prev = if n >= 2 { field((n - 2) as usize) } else { last };
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
    let tag_flat = affix_tags::sum_for(AffixEffect::FlatDamage, &s.tags, input.stats);
    flat_min += tag_flat.0;
    flat_max += tag_flat.1;
    flat_min += input.conversion_flat;
    flat_max += input.conversion_flat;

    let (synergy_min, synergy_max) = bonus_source_synergy_pct(
        s,
        input.attributes,
        input.stats,
        input.skill_ranks_by_name,
        input.skills_by_name,
        input.item_skill_bonuses,
        false,
    );

    let magic = if is_elemental {
        rg(input.stats, "magic_skill_damage")
    } else {
        (0.0, 0.0)
    };
    let elem = keys
        .map(|k| rg(input.stats, k.skill_damage))
        .unwrap_or((0.0, 0.0));
    let (arch_add, arch_more) = archetype_skill_damage(input.stats, &s.tags);
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

    let (mut extra_mult, mut extra_sources) =
        collect_extra_damage(input.stats, input.enemy_conditions, s.damage_type.as_deref());
    if input.of_total_damage > 0.0 {
        extra_sources.push(ExtraSource {
            label: "Subtree",
            pct: input.of_total_damage,
        });
        extra_mult *= 1.0 + input.of_total_damage / 100.0;
    }
    let extra_pct = (extra_mult - 1.0) * 100.0;

    let is_spell = s.tags.iter().any(|t| t == "Spell");
    let crit = crit_factors(input.stats, is_spell);

    let enemy_res_pct = s
        .damage_type
        .as_deref()
        .and_then(|dt| input.enemy_resistances.get(dt).copied())
        .unwrap_or(0.0);
    let raw_ignore = keys
        .map(|k| r_max(rg(input.stats, k.ignore_res)))
        .unwrap_or(0.0);
    let ignore_res_pct = raw_ignore.clamp(0.0, 100.0);
    let eff_res_pct = enemy_res_pct * (1.0 - ignore_res_pct / 100.0);
    let resistance_mult = 1.0 - eff_res_pct / 100.0;

    let elemental_break_pct = if is_elemental {
        let base = r_max(rg(input.stats, "elemental_break"));
        let on = if is_spell {
            r_max(rg(input.stats, "elemental_break_on_spell"))
        } else {
            r_max(rg(input.stats, "elemental_break_on_strike"))
        };
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
    let element_break_mult = 1.0 + element_break_pct / 100.0;

    let damage_taken_pct = r_max(rg(input.scoped, "enemy_damage_taken_increased")).max(0.0);
    let damage_taken_mult = 1.0 + damage_taken_pct / 100.0;

    let hit_min = (base_min + flat_min)
        * (1.0 + synergy_min / 100.0)
        * (1.0 + skill_dmg_min / 100.0)
        * skill_more_min
        * extra_mult
        * damage_taken_mult
        * elemental_break_mult
        * element_break_mult
        * resistance_mult;
    let hit_max = (base_max + flat_max)
        * (1.0 + synergy_max / 100.0)
        * (1.0 + skill_dmg_max / 100.0)
        * skill_more_max
        * extra_mult
        * damage_taken_mult
        * elemental_break_mult
        * element_break_mult
        * resistance_mult;

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
    let multicast_chance = if affix_tags::is_entity(&s.tags) {
        0.0
    } else {
        (shared_multicast + r_max(rg(input.scoped, "multicast_chance"))).max(0.0)
    };
    let multicast_mult = 1.0 + multicast_chance / 100.0;
    let projectiles = input.projectile_count.max(1);
    let avg_min_f = hit_min * crit.avg_mult * multicast_mult * projectiles as f64;
    let avg_max_f = hit_max * crit.avg_mult * multicast_mult * projectiles as f64;

    Some(SkillDamageBreakdown {
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

    struct Case {
        damage_type: &'static str,
        tags: Vec<String>,
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
        assert_eq!(plain.hit_max, 100, "flat must not leak to non-Guardian skills");
    }

    #[test]
    fn archetype_explosion_is_additive_only() {
        let s = stats(&[("explosion_damage", 40.0)]);
        let (add, more) = archetype_skill_damage(&s, &tags(&["Explosion"]));
        assert_eq!(add, (40.0, 40.0));
        assert_eq!(more, (1.0, 1.0));
    }

    // `fire_break` and `elemental_break` are separate stages — neither may leak
    // into the other's slot. 100 base x 1.5 x 1.5 = 225.
    #[test]
    fn verify_element_break_and_elemental_break_multiply_without_reuse() {
        let both = breakdown(&Case {
            damage_type: "fire",
            stats: stats(&[("elemental_break", 50.0), ("fire_break", 50.0)]),
            conditions: cond(&["fire_break"]),
            ..Default::default()
        });
        assert_eq!(both.elemental_break_pct, 50.0);
        assert_eq!(both.hit_max, 225);
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
}
