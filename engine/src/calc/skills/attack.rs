use std::collections::HashMap;

use super::{
    AttackSkillDamageBreakdown, AttrMap, CRUSHING_BLOW_DEFAULT, ConditionMap, DamageFormula,
    ItemSkillBonuses, Skill, SkillDamageBreakdown, SkillRanks, StatMap, Weapon,
    collect_extra_damage, crit_factors, damage::bonus_source_synergy_pct, damage::element_keys,
    deadly_blow_mult, r_max, r_min, rg,
};

pub struct AttackSkillInput<'a> {
    pub skill: &'a Skill,
    pub allocated_rank: f64,
    pub attributes: &'a AttrMap,
    pub stats: &'a StatMap,
    pub skill_ranks_by_name: &'a SkillRanks,
    pub skills_by_name: &'a HashMap<String, Skill>,
    pub item_skill_bonuses: &'a ItemSkillBonuses,
    pub enemy_conditions: &'a ConditionMap,
    pub weapon: Option<&'a Weapon>,
    /// Shared with caller's `damage` to avoid double-computing the element side.
    pub poison_breakdown: Option<&'a SkillDamageBreakdown>,
    /// The owning skill's `skillScoped` subtree values.
    pub scoped: &'a StatMap,
    pub projectile_count: u32,
    /// Resolved in `build.rs`; attack skills have no elemental breakdown to land it in.
    pub conversion_flat: f64,
}

#[inline]
fn total_pct(input: &AttackSkillInput<'_>, key: &str) -> f64 {
    r_max(rg(input.stats, key)) + r_max(rg(input.scoped, key))
}

/// Clamp to >= 0: linear formulas can extrapolate negative at low ranks,
/// but a skill never grants negative damage/AR.
#[inline]
fn formula_at_clamped_opt(f: Option<&DamageFormula>, rank: f64) -> f64 {
    f.map(|x| (x.base + x.per_level * rank).max(0.0))
        .unwrap_or(0.0)
}

pub fn compute_attack_skill_damage(
    input: &AttackSkillInput<'_>,
) -> Option<AttackSkillDamageBreakdown> {
    let s = input.skill;
    if input.allocated_rank == 0.0 {
        return None;
    }
    let scaling = s.attack_scaling.as_ref()?;

    let all_skills = rg(input.stats, "all_skills");
    let elem_bonus = s
        .damage_type
        .as_deref()
        .and_then(element_keys)
        .map(|k| rg(input.stats, k.skills))
        .unwrap_or((0.0, 0.0));
    let item = input
        .item_skill_bonuses
        .get(&s.name)
        .copied()
        .unwrap_or((0.0, 0.0));
    let eff_min = input.allocated_rank + r_min(all_skills) + r_min(elem_bonus) + item.0;
    let eff_max = input.allocated_rank + r_max(all_skills) + r_max(elem_bonus) + item.1;

    let skill_wdp_min = formula_at_clamped_opt(scaling.weapon_damage_pct.as_ref(), eff_min);
    let skill_wdp_max = formula_at_clamped_opt(scaling.weapon_damage_pct.as_ref(), eff_max);
    let skill_flat_min = formula_at_clamped_opt(scaling.flat_physical_min.as_ref(), eff_min);
    let skill_flat_max = formula_at_clamped_opt(scaling.flat_physical_max.as_ref(), eff_max);
    let skill_arp_min = formula_at_clamped_opt(scaling.attack_rating_pct.as_ref(), eff_min);
    let skill_arp_max = formula_at_clamped_opt(scaling.attack_rating_pct.as_ref(), eff_max);

    // Unarmed baseline is 2-6 physical for every character.
    let (w_min, w_max) = input
        .weapon
        .map(|w| (w.damage_min, w.damage_max))
        .unwrap_or((2.0, 6.0));
    let ed = rg(input.stats, "enhanced_damage");
    let ed_more = rg(input.stats, "enhanced_damage_more");
    let add_phys = rg(input.stats, "additive_physical_damage");
    let atk = rg(input.stats, "attack_damage");
    // Synergies like Flail Mastery grant attack_damage% per source rank; they
    // stack additively with the gear/strength attack-damage stat.
    let (synergy_min, synergy_max) = bonus_source_synergy_pct(
        s,
        input.attributes,
        input.stats,
        input.skill_ranks_by_name,
        input.skills_by_name,
        input.item_skill_bonuses,
        true,
    );

    let (extra_mult, extra_sources) =
        collect_extra_damage(input.stats, input.enemy_conditions, Some("physical"));

    let crit = crit_factors(input.stats, false);

    let base_min = w_min
        * (1.0 + r_min(ed) / 100.0)
        * (1.0 + r_min(ed_more) / 100.0)
        + r_min(add_phys)
        + skill_flat_min
        + input.conversion_flat;
    let base_max = w_max
        * (1.0 + r_max(ed) / 100.0)
        * (1.0 + r_max(ed_more) / 100.0)
        + r_max(add_phys)
        + skill_flat_max
        + input.conversion_flat;

    // S8: the skill's own attack damage % is a standalone multiplier on top of
    // the gear/strength attack-damage modifier, not additive with it.
    let atk_mult_min = 1.0 + (r_min(atk) + synergy_min) / 100.0;
    let atk_mult_max = 1.0 + (r_max(atk) + synergy_max) / 100.0;
    let has_wdp = scaling.weapon_damage_pct.is_some();
    let skill_mult_min = if has_wdp { skill_wdp_min / 100.0 } else { 1.0 };
    let skill_mult_max = if has_wdp { skill_wdp_max / 100.0 } else { 1.0 };

    // Same crush/deadly stage weapon.rs applies, so a skill swing and a plain
    // swing agree.
    let crushing_raw = r_max(rg(input.stats, "crushing_blow_modifier"));
    let crushing_blow_modifier = if crushing_raw == 0.0 {
        CRUSHING_BLOW_DEFAULT
    } else {
        crushing_raw
    };
    let armor_break_pct = total_pct(input, "armor_break");
    let crush_armor_mult = crushing_blow_modifier + armor_break_pct / 100.0;

    let deadly_blow_chance =
        total_pct(input, "deadly_blow_chance") + total_pct(input, "deadly_blow");
    let deadly_mult = deadly_blow_mult(
        deadly_blow_chance,
        total_pct(input, "deadly_blow_effectiveness"),
    );

    let phys_hit_min = base_min * atk_mult_min * skill_mult_min * crush_armor_mult
        * deadly_mult
        * extra_mult;
    let phys_hit_max = base_max * atk_mult_max * skill_mult_max * crush_armor_mult
        * deadly_mult
        * extra_mult;
    // Mirrors the spell path: hit stays per-projectile, the average folds the
    // projectile fan-in. The poison breakdown already multiplied its own avg.
    let projectiles = input.projectile_count.max(1);
    let phys_avg_min = phys_hit_min * crit.avg_mult * projectiles as f64;
    let phys_avg_max = phys_hit_max * crit.avg_mult * projectiles as f64;

    let (poison_hit_min, poison_hit_max, poison_avg_min, poison_avg_max) = input
        .poison_breakdown
        .map(|p| (p.hit_min, p.hit_max, p.avg_min, p.avg_max))
        .unwrap_or((0, 0, 0, 0));

    let phys_hit_min_i = phys_hit_min.floor() as i64;
    let phys_hit_max_i = phys_hit_max.floor() as i64;
    let phys_avg_min_i = phys_avg_min.floor() as i64;
    let phys_avg_max_i = phys_avg_max.floor() as i64;

    let combined_hit_min = phys_hit_min_i + poison_hit_min;
    let combined_hit_max = phys_hit_max_i + poison_hit_max;
    let combined_avg_min = phys_avg_min_i + poison_avg_min;
    let combined_avg_max = phys_avg_max_i + poison_avg_max;

    let ias = rg(input.stats, "increased_attack_speed");
    let ias_more = rg(input.stats, "increased_attack_speed_more");
    let base_aps = r_max(rg(input.stats, "attacks_per_second"));
    let aps_min = base_aps * (1.0 + r_min(ias) / 100.0) * (1.0 + r_min(ias_more) / 100.0);
    let aps_max = base_aps * (1.0 + r_max(ias) / 100.0) * (1.0 + r_max(ias_more) / 100.0);

    let dps_min = (combined_avg_min as f64) * aps_min;
    let dps_max = (combined_avg_max as f64) * aps_max;

    Some(AttackSkillDamageBreakdown {
        effective_rank_min: eff_min,
        effective_rank_max: eff_max,
        weapon_damage_pct_min: skill_wdp_min,
        weapon_damage_pct_max: skill_wdp_max,
        skill_flat_phys_min: skill_flat_min,
        skill_flat_phys_max: skill_flat_max,
        attack_rating_pct_min: skill_arp_min,
        attack_rating_pct_max: skill_arp_max,
        synergy_min_pct: synergy_min,
        synergy_max_pct: synergy_max,
        projectile_count: projectiles,
        weapon_damage_min: w_min,
        weapon_damage_max: w_max,
        enhanced_damage_min_pct: r_min(ed),
        enhanced_damage_max_pct: r_max(ed),
        additive_physical_min: r_min(add_phys),
        additive_physical_max: r_max(add_phys),
        attack_damage_min_pct: r_min(atk),
        attack_damage_max_pct: r_max(atk),
        crushing_blow_modifier,
        armor_break_pct,
        deadly_blow_chance,
        crit_chance: crit.chance,
        crit_damage_pct: crit.damage_pct,
        crit_multiplier_avg: crit.avg_mult,
        extra_damage_sources: extra_sources,
        physical_hit_min: phys_hit_min_i,
        physical_hit_max: phys_hit_max_i,
        physical_avg_min: phys_avg_min_i,
        physical_avg_max: phys_avg_max_i,
        poison_hit_min,
        poison_hit_max,
        poison_avg_min,
        poison_avg_max,
        combined_hit_min,
        combined_hit_max,
        combined_avg_min,
        combined_avg_max,
        attacks_per_second_min: aps_min,
        attacks_per_second_max: aps_max,
        dps_min,
        dps_max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::skills::{AttackSkillScaling, DamageFormula};

    fn skill() -> Skill {
        Skill {
            name: "Test Swing".into(),
            tags: vec!["Attack".into()],
            damage_type: Some("physical".into()),
            damage_formula: None,
            damage_per_rank: None,
            bonus_sources: Vec::new(),
            attack_kind: Some(crate::calc::skills::AttackKind::Attack),
            attack_scaling: Some(AttackSkillScaling {
                // 100% weapon damage = a neutral skill multiplier, so these
                // tests isolate the other stages.
                weapon_damage_pct: Some(DamageFormula {
                    base: 100.0,
                    per_level: 0.0,
                }),
                ..Default::default()
            }),
        }
    }

    fn stats(pairs: &[(&str, f64)]) -> StatMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), (*v, *v)))
            .collect()
    }

    fn breakdown_for(
        s: &Skill,
        shared: &StatMap,
        scoped: &StatMap,
        skill_ranks: &SkillRanks,
        projectile_count: u32,
        conversion_flat: f64,
    ) -> AttackSkillDamageBreakdown {
        let bonuses = ItemSkillBonuses::new();
        let conds = ConditionMap::new();
        let attrs = AttrMap::new();
        let skills_by_name = HashMap::new();
        let weapon = Weapon {
            name: "test".into(),
            damage_min: 100.0,
            damage_max: 100.0,
        };
        let input = AttackSkillInput {
            skill: s,
            allocated_rank: 1.0,
            attributes: &attrs,
            stats: shared,
            skill_ranks_by_name: skill_ranks,
            skills_by_name: &skills_by_name,
            item_skill_bonuses: &bonuses,
            enemy_conditions: &conds,
            weapon: Some(&weapon),
            poison_breakdown: None,
            scoped,
            projectile_count,
            conversion_flat,
        };
        compute_attack_skill_damage(&input).expect("attack breakdown")
    }

    fn hit_max(shared: &StatMap, scoped: &StatMap) -> i64 {
        breakdown_for(&skill(), shared, scoped, &SkillRanks::new(), 1, 0.0).combined_hit_max
    }

    fn hit_max_with_conversion(conversion_flat: f64) -> i64 {
        breakdown_for(
            &skill(),
            &StatMap::new(),
            &StatMap::new(),
            &SkillRanks::new(),
            1,
            conversion_flat,
        )
        .combined_hit_max
    }

    #[test]
    fn crushing_blow_default_matches_the_weapon_panel() {
        // weapon 100 * crushing 1.5 = 150, the same stage weapon.rs applies.
        assert_eq!(hit_max(&StatMap::new(), &StatMap::new()), 150);
    }

    #[test]
    fn armor_break_raises_attack_skill_damage() {
        // crush_armor_mult = 1.5 + 100/100 = 2.5
        assert_eq!(hit_max(&stats(&[("armor_break", 100.0)]), &StatMap::new()), 250);
    }

    #[test]
    fn armor_break_from_the_subtree_counts_like_a_shared_one() {
        assert_eq!(
            hit_max(&StatMap::new(), &stats(&[("armor_break", 100.0)])),
            hit_max(&stats(&[("armor_break", 100.0)]), &StatMap::new()),
        );
    }

    #[test]
    fn deadly_blow_effectiveness_scales_the_proc_bonus() {
        // chance 100%: S8 deadly = 1.35 -> 150 * 1.35 = 202.5, floored
        let plain = hit_max(&stats(&[("deadly_blow_chance", 100.0)]), &StatMap::new());
        assert_eq!(plain, 202);
        // +50% effectiveness: on-proc 1.35 * 1.5 = 2.025 -> 150 * 2.025 = 303.75
        let boosted = hit_max(
            &stats(&[
                ("deadly_blow_chance", 100.0),
                ("deadly_blow_effectiveness", 50.0),
            ]),
            &StatMap::new(),
        );
        assert_eq!(boosted, 303);
    }

    #[test]
    fn conversion_flat_lands_in_the_swing_before_the_multipliers() {
        // weapon 100 + 40 converted, then the same crushing 1.5 stage.
        assert_eq!(hit_max_with_conversion(0.0), 150);
        assert_eq!(hit_max_with_conversion(40.0), 210);
    }

    #[test]
    fn subtree_deadly_blow_adds_to_the_chance() {
        assert_eq!(
            hit_max(&StatMap::new(), &stats(&[("deadly_blow", 100.0)])),
            202,
        );
    }

    #[test]
    fn projectiles_multiply_the_average_but_not_the_hit() {
        let plain = breakdown_for(&skill(), &StatMap::new(), &StatMap::new(), &SkillRanks::new(), 1, 0.0);
        let fanned = breakdown_for(&skill(), &StatMap::new(), &StatMap::new(), &SkillRanks::new(), 3, 0.0);
        assert_eq!(fanned.combined_hit_max, plain.combined_hit_max);
        assert_eq!(fanned.combined_avg_max, plain.combined_avg_max * 3);
        assert!((fanned.dps_max - plain.dps_max * 3.0).abs() < 1e-6);
    }

    #[test]
    fn bonus_source_synergy_raises_attack_damage() {
        // Flail Mastery rank 10 at 4%/level = +40% attack damage:
        // 100 * 1.4 * 1.5 crushing = 210.
        let mut s = skill();
        s.bonus_sources = vec![crate::calc::skills::BonusSource::SkillLevel {
            source: "flail mastery".into(),
            stat: "attack_damage".into(),
            value: 4.0,
        }];
        let ranks: SkillRanks = [("flail mastery".to_string(), 10.0)].into_iter().collect();
        let out = breakdown_for(&s, &StatMap::new(), &StatMap::new(), &ranks, 1, 0.0);
        assert_eq!(out.combined_hit_max, 210);
        assert_eq!(out.synergy_max_pct, 40.0);
    }

    #[test]
    fn skill_weapon_damage_pct_is_a_standalone_multiplier() {
        // S8: 200% attack damage and 300% skill tooltip multiply, not add:
        // 100 * 3.0 * (1 + 2.0) * 1.5 = 1350 (additive would give 900).
        let s = skill();
        let mut s2 = s.clone();
        s2.attack_scaling = Some(AttackSkillScaling {
            weapon_damage_pct: Some(DamageFormula {
                base: 300.0,
                per_level: 0.0,
            }),
            ..Default::default()
        });
        let shared = stats(&[("attack_damage", 200.0)]);
        let out = breakdown_for(&s2, &shared, &StatMap::new(), &SkillRanks::new(), 1, 0.0);
        assert_eq!(out.combined_hit_max, 1350);
    }
}
