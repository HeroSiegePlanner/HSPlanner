use std::collections::HashMap;

use super::{
    AttrMap, BonusSource, ConditionMap, ELEMENTS, ItemSkillBonuses, Ranged, ResistMap, Skill,
    SkillDamageBreakdown, SkillRanks, StatMap, collect_extra_damage, crit_factors, r_max, r_min, rg,
};

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
    let eff_min = input.allocated_rank + r_min(all_skills) + elem_min + item.0;
    let eff_max = input.allocated_rank + r_max(all_skills) + elem_max + item.1;

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
        let i_min = ((eff_min as i64).max(1).min(n) - 1) as usize;
        let i_max = ((eff_max as i64).max(1).min(n) - 1) as usize;
        (t[i_min].min.max(0.0), t[i_max].max.max(0.0))
    };

    let mut flat_min = 0.0;
    let mut flat_max = 0.0;
    for k in ["flat_skill_damage", "flat_elemental_skill_damage"] {
        let v = rg(input.stats, k);
        flat_min += r_min(v);
        flat_max += r_max(v);
    }
    if let Some(k) = keys {
        let v = rg(input.stats, k.flat_skill_damage);
        flat_min += r_min(v);
        flat_max += r_max(v);
    }

    let mut synergy_min = 0.0;
    let mut synergy_max = 0.0;
    for bs in &s.bonus_sources {
        match bs {
            BonusSource::AttributePoint { source, value } => {
                let v = input.attributes.get(source).copied().unwrap_or((0.0, 0.0));
                synergy_min += r_min(v) * value;
                synergy_max += r_max(v) * value;
            }
            BonusSource::SkillLevel { source, value } => {
                let br = *input.skill_ranks_by_name.get(source).unwrap_or(&0.0);
                if br <= 0.0 {
                    continue;
                }
                if let Some(s2) = input.skills_by_name.get(source) {
                    let all = rg(input.stats, "all_skills");
                    let (el_min, el_max): Ranged =
                        match s2.damage_type.as_deref().and_then(element_keys) {
                            Some(k) => {
                                let e = rg(input.stats, k.skills);
                                (r_min(e), r_max(e))
                            }
                            None => (0.0, 0.0),
                        };
                    let it = input
                        .item_skill_bonuses
                        .get(source)
                        .copied()
                        .unwrap_or((0.0, 0.0));
                    synergy_min += (br + r_min(all) + el_min + it.0) * value;
                    synergy_max += (br + r_max(all) + el_max + it.1) * value;
                } else {
                    synergy_min += br * value;
                    synergy_max += br * value;
                }
            }
        }
    }

    let magic = rg(input.stats, "magic_skill_damage");
    let elem = keys
        .map(|k| rg(input.stats, k.skill_damage))
        .unwrap_or((0.0, 0.0));
    let skill_dmg_min = r_min(magic) + r_min(elem);
    let skill_dmg_max = r_max(magic) + r_max(elem);

    let magic_more = rg(input.stats, "magic_skill_damage_more");
    let elem_more = keys
        .map(|k| rg(input.stats, k.skill_damage_more))
        .unwrap_or((0.0, 0.0));
    let skill_more_min = (1.0 + r_min(magic_more) / 100.0) * (1.0 + r_min(elem_more) / 100.0);
    let skill_more_max = (1.0 + r_max(magic_more) / 100.0) * (1.0 + r_max(elem_more) / 100.0);

    let (extra_mult, extra_sources) =
        collect_extra_damage(input.stats, input.enemy_conditions);
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

    let is_elemental = s
        .damage_type
        .as_deref()
        .is_some_and(|dt| ELEMENTS.contains(&dt));
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

    let lightning_break_pct = if s.damage_type.as_deref() == Some("lightning")
        && *input
            .enemy_conditions
            .get("lightning_break")
            .unwrap_or(&false)
    {
        r_max(rg(input.stats, "lightning_break")).max(0.0)
    } else {
        0.0
    };
    let lightning_break_mult = 1.0 + lightning_break_pct / 100.0;

    let hit_min = (base_min + flat_min)
        * (1.0 + synergy_min / 100.0)
        * (1.0 + skill_dmg_min / 100.0)
        * skill_more_min
        * extra_mult
        * elemental_break_mult
        * lightning_break_mult
        * resistance_mult;
    let hit_max = (base_max + flat_max)
        * (1.0 + synergy_max / 100.0)
        * (1.0 + skill_dmg_max / 100.0)
        * skill_more_max
        * extra_mult
        * elemental_break_mult
        * lightning_break_mult
        * resistance_mult;

    let crit_min_f = hit_min * crit.on_crit_mult;
    let crit_max_f = hit_max * crit.on_crit_mult;
    let multicast_chance = if is_spell {
        r_max(rg(input.stats, "multicast_chance")).max(0.0)
    } else {
        0.0
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
