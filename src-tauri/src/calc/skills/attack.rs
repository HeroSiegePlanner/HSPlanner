use super::{
    AttackSkillDamageBreakdown, ConditionMap, DamageFormula, ItemSkillBonuses, Skill,
    SkillDamageBreakdown, StatMap, Weapon, collect_extra_damage, crit_factors, damage::element_keys,
    r_max, r_min, rg,
};

pub struct AttackSkillInput<'a> {
    pub skill: &'a Skill,
    pub allocated_rank: f64,
    pub stats: &'a StatMap,
    pub item_skill_bonuses: &'a ItemSkillBonuses,
    pub enemy_conditions: &'a ConditionMap,
    pub weapon: Option<&'a Weapon>,
    /// Shared with caller's `damage` to avoid double-computing the element side.
    pub poison_breakdown: Option<&'a SkillDamageBreakdown>,
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

    let (extra_mult, _extra_sources) =
        collect_extra_damage(input.stats, input.enemy_conditions);

    let crit = crit_factors(input.stats, false);

    let base_min = w_min
        * (1.0 + r_min(ed) / 100.0)
        * (1.0 + r_min(ed_more) / 100.0)
        + r_min(add_phys)
        + skill_flat_min;
    let base_max = w_max
        * (1.0 + r_max(ed) / 100.0)
        * (1.0 + r_max(ed_more) / 100.0)
        + r_max(add_phys)
        + skill_flat_max;

    let total_atk_min = r_min(atk) + skill_wdp_min;
    let total_atk_max = r_max(atk) + skill_wdp_max;

    let phys_hit_min = base_min * (1.0 + total_atk_min / 100.0) * extra_mult;
    let phys_hit_max = base_max * (1.0 + total_atk_max / 100.0) * extra_mult;
    let phys_avg_min = phys_hit_min * crit.avg_mult;
    let phys_avg_max = phys_hit_max * crit.avg_mult;

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
