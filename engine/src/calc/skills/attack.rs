use super::calculation::{number, range, scalar, stat_inputs, CalculationStep};
use std::collections::HashMap;

use super::{
    collect_extra_damage_with_scoped, damage::bonus_source_synergy_pct, damage::element_keys,
    deadly_blow_mult, r_max, r_min, rg, AttackSkillDamageBreakdown, AttrMap, ConditionMap,
    DamageFormula, ExtraSource, ItemSkillBonuses, Skill, SkillDamageBreakdown, SkillRanks, StatMap,
    Weapon,
};

pub struct AttackSkillInput<'a> {
    /// Optional cast cadence for persistent attack-damage skills.
    pub action_rate_override: Option<(f64, f64)>,
    pub skill: &'a Skill,
    pub allocated_rank: f64,
    pub attributes: &'a AttrMap,
    pub stats: &'a StatMap,
    pub skill_ranks_by_name: &'a SkillRanks,
    pub skills_by_name: &'a HashMap<String, Skill>,
    pub item_skill_bonuses: &'a ItemSkillBonuses,
    pub enemy_conditions: &'a ConditionMap,
    pub enemy_resistances: &'a HashMap<String, f64>,
    pub weapon: Option<&'a Weapon>,
    /// Shared with caller's `damage` to avoid double-computing the element side.
    pub poison_breakdown: Option<&'a SkillDamageBreakdown>,
    /// The owning skill's `skillScoped` subtree values.
    pub scoped: &'a StatMap,
    pub projectile_count: u32,
    /// Aggregate damage adjustment from the owning subtree. Elemental damage
    /// already consumes its own copy, so this changes only the physical member.
    pub of_total_damage: f64,
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

const CONVERSION_ORDER: [&str; 5] = ["arcane", "cold", "fire", "lightning", "poison"];

/// The largest physical conversion total wins; ties go arcane → cold → fire → lightning → poison.
fn physical_conversion(input: &AttackSkillInput<'_>) -> Option<(&'static str, (f64, f64))> {
    let mut best: Option<(&'static str, (f64, f64))> = None;
    for element in CONVERSION_ORDER {
        let key = format!("physical_to_{element}");
        let shared = rg(input.stats, &key);
        let scoped = rg(input.scoped, &key);
        let pct = (
            (shared.0 + scoped.0).max(0.0),
            (shared.1 + scoped.1).max(0.0),
        );
        if pct.1 > best.map_or(0.0, |(_, top)| top.1) {
            best = Some((element, pct));
        }
    }
    best
}

fn converted_attack_elements(
    input: &AttackSkillInput<'_>,
    conversion: Option<(&'static str, (f64, f64))>,
    hit: (f64, f64),
    average: (f64, f64),
) -> (Vec<super::ConvertedAttackDamage>, Vec<CalculationStep>) {
    let mut calculation = Vec::new();
    stat_inputs(
        &mut calculation,
        input.stats,
        CONVERSION_ORDER.map(|element| format!("physical_to_{element}")),
    );
    let Some((element, pct)) = conversion else {
        return (Vec::new(), calculation);
    };
    let keys = element_keys(element).expect("element keys");
    // Only the element's own Skill Damage scales the converted share.
    let shared_damage = rg(input.stats, keys.skill_damage);
    let scoped_damage = rg(input.scoped, keys.skill_damage);
    let skill_damage = (
        shared_damage.0 + scoped_damage.0,
        shared_damage.1 + scoped_damage.1,
    );
    let element_multiplier = (
        (1.0 + skill_damage.0 / 100.0).max(0.0),
        (1.0 + skill_damage.1 / 100.0).max(0.0),
    );
    let resistance = input.enemy_resistances.get(element).copied().unwrap_or(0.0);
    let ignored = (total_pct(input, keys.ignore_res)
        + total_pct(input, keys.legacy_ignore_res)
        + total_pct(input, "enemy_all_resist"))
    .clamp(0.0, 100.0);
    let resistance_mult = (1.0 - resistance * (1.0 - ignored / 100.0) / 100.0).max(0.0);
    let elemental_break = (total_pct(input, "elemental_break")
        + total_pct(input, "elemental_break_on_strike"))
    .max(0.0);
    let break_key = format!("{element}_break");
    let element_break = if input
        .enemy_conditions
        .get(&break_key)
        .copied()
        .unwrap_or(false)
    {
        total_pct(input, &break_key).max(0.0)
    } else {
        0.0
    };
    // As on an elemental hit, the subtree's damage taken joins the elemental break sum.
    let damage_taken = r_max(rg(input.scoped, "enemy_damage_taken_increased")).max(0.0);
    let target_mult =
        resistance_mult * (1.0 + (elemental_break + element_break + damage_taken) / 100.0);
    let (critical_break_average, critical_break_step) = super::critical_break_average_multiplier(
        element,
        element_break,
        elemental_break,
        input.stats,
        input.scoped,
    );
    let scale = (
        pct.0 / 100.0 * element_multiplier.0 * target_mult,
        pct.1 / 100.0 * element_multiplier.1 * target_mult,
    );
    let component = super::ConvertedAttackDamage {
        element: element.to_string(),
        conversion_pct: pct,
        hit_min: (hit.0 * scale.0).floor() as i64,
        hit_max: (hit.1 * scale.1).floor() as i64,
        avg_min: (average.0 * scale.0 * critical_break_average.0).floor() as i64,
        avg_max: (average.1 * scale.1 * critical_break_average.1).floor() as i64,
    };
    stat_inputs(
        &mut calculation,
        input.stats,
        [
            keys.skill_damage,
            keys.ignore_res,
            "elemental_break",
            "elemental_break_on_strike",
            break_key.as_str(),
        ],
    );
    calculation.push(CalculationStep::new(
        "Physical conversion",
        format!(
            "{}% to {element}, the largest physical conversion; ties go arcane → cold → fire → lightning → poison and the other conversions are ignored",
            range(pct)
        ),
        pct,
    ));
    calculation.push(CalculationStep::new(
        format!("Physical converted to {element}"),
        format!("floor({} physical hit before the physical break × {}% / 100 × (1 + {}% {element} Skill Damage / 100) × {} resistance × [1 + ({}% Elemental Break + {}% {element} Break + {}% damage taken) / 100]); the rest stays physical",
            range(hit), range(pct), range(skill_damage), number(resistance_mult),
            number(elemental_break), number(element_break), number(damage_taken)),
        (component.hit_min as f64, component.hit_max as f64),
    ));
    calculation.push(CalculationStep::new(
        format!("Average converted {element} damage"),
        "The physical average already includes critical chance, double damage and projectile count; apply conversion, target modifiers and the independent Critical Break adjustment once",
        (component.avg_min as f64, component.avg_max as f64),
    ));
    calculation.extend(critical_break_step);
    (vec![component], calculation)
}

pub fn compute_attack_skill_damage(
    input: &AttackSkillInput<'_>,
) -> Option<AttackSkillDamageBreakdown> {
    compute_attack_skill_damage_with_rank(input, None)
}

pub(crate) fn compute_attack_skill_damage_at_rank(
    input: &AttackSkillInput<'_>,
    effective_rank: f64,
) -> Option<AttackSkillDamageBreakdown> {
    compute_attack_skill_damage_with_rank(input, Some(effective_rank))
}

fn compute_attack_skill_damage_with_rank(
    input: &AttackSkillInput<'_>,
    fixed_rank: Option<f64>,
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
    let tag_bonus = crate::calc::rank::tag_skills_sum(input.stats, &s.tags);
    let (eff_min, eff_max) = fixed_rank.map(|rank| (rank, rank)).unwrap_or_else(|| {
        crate::calc::rank::effective_rank_range_for(
            s,
            input.allocated_rank,
            input.stats,
            input.item_skill_bonuses,
        )
    });

    let skill_wdp_min = formula_at_clamped_opt(scaling.weapon_damage_pct.as_ref(), eff_min);
    let skill_wdp_max = formula_at_clamped_opt(scaling.weapon_damage_pct.as_ref(), eff_max);
    let skill_flat_min = formula_at_clamped_opt(scaling.flat_physical_min.as_ref(), eff_min);
    let skill_flat_max = formula_at_clamped_opt(scaling.flat_physical_max.as_ref(), eff_max);
    let rating_bonus = total_pct(input, "attack_rating_pct");
    let rating_multiplier = rg(input.scoped, "skill_attack_rating_multiplier_pct");
    let skill_arp_min = formula_at_clamped_opt(scaling.attack_rating_pct.as_ref(), eff_min)
        * (1.0 + rating_multiplier.0 / 100.0)
        + rating_bonus;
    let skill_arp_max = formula_at_clamped_opt(scaling.attack_rating_pct.as_ref(), eff_max)
        * (1.0 + rating_multiplier.1 / 100.0)
        + rating_bonus;

    // Unarmed baseline is 2-6 physical for every character.
    let (mut w_min, mut w_max) = input
        .weapon
        .map(|w| (w.damage_min, w.damage_max))
        .unwrap_or((2.0, 6.0));
    if input.weapon.is_none() {
        let unarmed = rg(input.stats, "str_to_unarmed_damage");
        w_min += unarmed.0;
        w_max += unarmed.1;
    }
    let ed = rg(input.stats, "enhanced_damage");
    let ed_more = rg(input.stats, "enhanced_damage_more");
    let add_phys = rg(input.stats, "additive_physical_damage");
    let ranged_flat = if s.tags.iter().any(|tag| tag == "Ranged") {
        rg(input.stats, "flat_ranged_physical_damage")
    } else {
        (0.0, 0.0)
    };
    let atk = rg(input.stats, "attack_damage");
    let atk_more = rg(input.stats, "attack_damage_more");
    // Most attack synergies share the attack-damage stage. Skills with an
    // explicit weapon-bonus model add them to the skill's bonus instead.
    let ((synergy_min, synergy_max), synergy_steps) = bonus_source_synergy_pct(
        s,
        input.attributes,
        input.stats,
        input.skill_ranks_by_name,
        input.skills_by_name,
        input.item_skill_bonuses,
        true,
        None,
    );

    let (mut extra_mult, mut extra_sources) = collect_extra_damage_with_scoped(
        input.stats,
        input.scoped,
        input.enemy_conditions,
        Some("physical"),
    );

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

    let global_crit = super::crit_factors_with_tags(input.stats, false, &s.tags);
    let subtree_crit_chance = rg(input.scoped, "skill_crit_chance");
    let skill_crit_chance = (
        formula_at_clamped_opt(scaling.critical_chance.as_ref(), eff_min) + subtree_crit_chance.0,
        formula_at_clamped_opt(scaling.critical_chance.as_ref(), eff_max) + subtree_crit_chance.1,
    );
    let critical_damage_bonus = rg(input.scoped, "skill_crit_damage_multiplier_pct");
    let critical_damage_per_vitality = rg(
        input.scoped,
        "skill_crit_damage_multiplier_per_100_vitality",
    );
    let vitality = rg(input.attributes, "vitality");
    let critical_damage_multiplier = (
        1.0 + critical_damage_bonus.0 / 100.0
            + vitality.0 * critical_damage_per_vitality.0 / 10000.0,
        1.0 + critical_damage_bonus.1 / 100.0
            + vitality.1 * critical_damage_per_vitality.1 / 10000.0,
    );
    let subtree_crit_damage = rg(input.scoped, "skill_crit_damage");
    let skill_crit_damage = (
        formula_at_clamped_opt(scaling.critical_damage.as_ref(), eff_min)
            * critical_damage_multiplier.0
            + subtree_crit_damage.0,
        formula_at_clamped_opt(scaling.critical_damage.as_ref(), eff_max)
            * critical_damage_multiplier.1
            + subtree_crit_damage.1,
    );
    let crit_min = global_crit.with_skill_bonuses(skill_crit_chance.0, skill_crit_damage.0);
    let crit = global_crit.with_skill_bonuses(skill_crit_chance.1, skill_crit_damage.1);
    // LoadTalentDamage's physical weapon-coefficient branch ignores its flat
    // pool, including Leap stat437. See ../leap-evidence.md. Other tagged flat
    // sources retain their existing behavior pending their own source audit.
    let tagged_flat =
        crate::calc::affix_tags::keys_for(crate::calc::types::AffixEffect::FlatDamage, &s.tags)
            .into_iter()
            .filter(|key| *key != "flat_leap_damage")
            .fold((0.0, 0.0), |total, key| {
                let value = rg(input.stats, key);
                (total.0 + value.0, total.1 + value.1)
            });

    let (bound_flat, bound_multiplier) = super::damage_bound_bonuses(input.stats);
    let base_min = w_min * (1.0 + r_min(ed) / 100.0) * (1.0 + r_min(ed_more) / 100.0)
        + r_min(add_phys)
        + ranged_flat.0
        + skill_flat_min
        + tagged_flat.0
        + input.conversion_flat
        + bound_flat.0;
    let base_min = base_min * bound_multiplier.0;
    let base_max = w_max * (1.0 + r_max(ed) / 100.0) * (1.0 + r_max(ed_more) / 100.0)
        + r_max(add_phys)
        + ranged_flat.1
        + skill_flat_max
        + tagged_flat.1
        + input.conversion_flat
        + bound_flat.1;
    let base_max = base_max * bound_multiplier.1;

    use crate::calc::{affix_tags, types::AffixEffect};
    let tagged = affix_tags::sum_for(AffixEffect::Damage, &s.tags, input.stats);
    let tagged_more = affix_tags::more_for(AffixEffect::DamageMore, &s.tags, input.stats);
    let tagged_factor = (
        (1.0 + tagged.0 / 100.0).max(0.0) * tagged_more.0,
        (1.0 + tagged.1 / 100.0).max(0.0) * tagged_more.1,
    );
    let has_wdp = scaling.weapon_damage_pct.is_some();
    let weapon_bonus_scaling = scaling.weapon_bonus_scaling && has_wdp;
    let subtree = rg(input.scoped, "subtree_damage");
    let weapon_subtree = rg(input.scoped, "weapon_subtree_damage");
    let subtree_per_dexterity = rg(input.scoped, "subtree_damage_per_100_dexterity");
    let dexterity = rg(input.attributes, "dexterity");
    let dexterity_subtree = (
        (dexterity.0 * 0.01) * (subtree_per_dexterity.0 * 0.01),
        (dexterity.1 * 0.01) * (subtree_per_dexterity.1 * 0.01),
    );
    let subtree_factor = (
        (1.0 + (subtree.0 + weapon_subtree.0) / 100.0 + dexterity_subtree.0).max(0.0),
        (1.0 + (subtree.1 + weapon_subtree.1) / 100.0 + dexterity_subtree.1).max(0.0),
    );
    let attack_synergy = if weapon_bonus_scaling {
        (0.0, 0.0)
    } else {
        (synergy_min, synergy_max)
    };
    let atk_mult_min =
        (1.0 + (r_min(atk) + attack_synergy.0) / 100.0) * (1.0 + r_min(atk_more) / 100.0);
    let atk_mult_max =
        (1.0 + (r_max(atk) + attack_synergy.1) / 100.0) * (1.0 + r_max(atk_more) / 100.0);
    // The stored percentage includes neutral weapon damage. In this model the
    // bonus and its synergies receive tag scaling before the neutral 100% returns.
    let weapon_multiplier = |wdp: f64, synergy: f64, tag: f64, subtree: f64| {
        let bonus = (wdp - 100.0 + synergy) * tag / 100.0;
        if scaling.weapon_subtree_scales_full_hit {
            ((1.0 + bonus) * subtree).max(0.0)
        } else {
            (1.0 + (wdp - 100.0 + synergy) * tag * subtree / 100.0).max(0.0)
        }
    };
    let (skill_mult_min, skill_mult_max) = if weapon_bonus_scaling {
        (
            weapon_multiplier(
                skill_wdp_min,
                synergy_min,
                tagged_factor.0,
                subtree_factor.0,
            ),
            weapon_multiplier(
                skill_wdp_max,
                synergy_max,
                tagged_factor.1,
                subtree_factor.1,
            ),
        )
    } else if has_wdp {
        (skill_wdp_min / 100.0, skill_wdp_max / 100.0)
    } else {
        (1.0, 1.0)
    };
    let third_attack = rg(input.scoped, "third_attack_subtree_damage");
    let proc_subtree = rg(input.scoped, "subtree_damage_on_proc");
    let expected_skill_mult = if weapon_bonus_scaling {
        (
            weapon_multiplier(
                skill_wdp_min,
                synergy_min,
                tagged_factor.0,
                subtree_factor.0 + proc_subtree.0 / 100.0 + third_attack.0 / 300.0,
            ),
            weapon_multiplier(
                skill_wdp_max,
                synergy_max,
                tagged_factor.1,
                subtree_factor.1 + proc_subtree.1 / 100.0 + third_attack.1 / 300.0,
            ),
        )
    } else {
        (skill_mult_min, skill_mult_max)
    };
    let third_skill_mult = if weapon_bonus_scaling {
        (
            weapon_multiplier(
                skill_wdp_min,
                synergy_min,
                tagged_factor.0,
                subtree_factor.0 + (proc_subtree.0 + third_attack.0) / 100.0,
            ),
            weapon_multiplier(
                skill_wdp_max,
                synergy_max,
                tagged_factor.1,
                subtree_factor.1 + (proc_subtree.1 + third_attack.1) / 100.0,
            ),
        )
    } else {
        (skill_mult_min, skill_mult_max)
    };

    // Same crush/deadly stage weapon.rs applies, so a skill swing and a plain
    // swing agree.
    let armor_break_pct = total_pct(input, "armor_break");
    let crushing_blow_pct = total_pct(input, "crushing_blow_armor_break");
    let crush_armor_mult = 1.0 + (armor_break_pct + crushing_blow_pct) / 100.0;
    // The physical break lands after conversion: converted damage copies the hit before it.
    let unbroken = |value: f64| {
        if crush_armor_mult > 0.0 {
            value / crush_armor_mult
        } else {
            0.0
        }
    };

    let deadly_blow_chance =
        total_pct(input, "deadly_blow_chance") + total_pct(input, "deadly_blow");
    let deadly_mult = deadly_blow_mult(
        deadly_blow_chance,
        total_pct(input, "deadly_blow_effectiveness"),
    );

    let physical_skill_mult = {
        let physical = rg(input.stats, "physical_skill_damage");
        let more = rg(input.stats, "physical_skill_damage_more");
        let (tagged, tagged_more) = if weapon_bonus_scaling {
            ((0.0, 0.0), (1.0, 1.0))
        } else {
            (tagged, tagged_more)
        };
        (
            (1.0 + (physical.0 + tagged.0) / 100.0).max(0.0)
                * (1.0 + more.0 / 100.0).max(0.0)
                * tagged_more.0,
            (1.0 + (physical.1 + tagged.1) / 100.0).max(0.0)
                * (1.0 + more.1 / 100.0).max(0.0)
                * tagged_more.1,
        )
    };
    let phys_hit_min = base_min
        * atk_mult_min
        * skill_mult_min
        * crush_armor_mult
        * deadly_mult
        * extra_mult
        * physical_skill_mult.0;
    let phys_hit_max = base_max
        * atk_mult_max
        * skill_mult_max
        * crush_armor_mult
        * deadly_mult
        * extra_mult
        * physical_skill_mult.1;
    // Mirrors the spell path: hit stays per-projectile, the average folds the
    // projectile fan-in. The poison breakdown already multiplied its own avg.
    let projectiles = input.projectile_count.max(1);
    let double_damage = super::double_damage_factor(input.stats, input.scoped);
    let extra_projectiles = rg(input.scoped, "expected_additional_projectiles");
    let physical_expected = (
        base_min
            * atk_mult_min
            * expected_skill_mult.0
            * crush_armor_mult
            * deadly_mult
            * extra_mult
            * physical_skill_mult.0,
        base_max
            * atk_mult_max
            * expected_skill_mult.1
            * crush_armor_mult
            * deadly_mult
            * extra_mult
            * physical_skill_mult.1,
    );
    let third_crit_damage = rg(input.scoped, "third_attack_skill_crit_damage");
    let third_crit_extra = (
        base_min
            * atk_mult_min
            * third_skill_mult.0
            * crush_armor_mult
            * deadly_mult
            * extra_mult
            * physical_skill_mult.0
            * (crit_min.chance.clamp(0.0, 95.0) / 100.0)
            * third_crit_damage.0
            / 300.0,
        base_max
            * atk_mult_max
            * third_skill_mult.1
            * crush_armor_mult
            * deadly_mult
            * extra_mult
            * physical_skill_mult.1
            * (crit.chance.clamp(0.0, 95.0) / 100.0)
            * third_crit_damage.1
            / 300.0,
    );

    let phys_avg_min = (physical_expected.0 * crit_min.avg_mult + third_crit_extra.0)
        * double_damage.0
        * (projectiles as f64 + extra_projectiles.0.max(0.0));
    let phys_avg_max = (physical_expected.1 * crit.avg_mult + third_crit_extra.1)
        * double_damage.1
        * (projectiles as f64 + extra_projectiles.1.max(0.0));

    let (poison_hit_min, poison_hit_max, poison_avg_min, poison_avg_max) = input
        .poison_breakdown
        .map(|p| (p.hit_min, p.hit_max, p.avg_min, p.avg_max))
        .unwrap_or((0, 0, 0, 0));

    let conversion = physical_conversion(input);
    let (converted_elements, conversion_steps) = converted_attack_elements(
        input,
        conversion,
        (unbroken(phys_hit_min), unbroken(phys_hit_max)),
        (unbroken(phys_avg_min), unbroken(phys_avg_max)),
    );
    // The converted share leaves the physical hit; the game sets no 100% cap.
    let kept = conversion.map_or((1.0, 1.0), |(_, pct)| {
        (1.0 - pct.0 / 100.0, 1.0 - pct.1 / 100.0)
    });
    let phys_hit_min = phys_hit_min * kept.0;
    let phys_hit_max = phys_hit_max * kept.1;
    let phys_avg_min = phys_avg_min * kept.0;
    let phys_avg_max = phys_avg_max * kept.1;
    let phys_hit_min_i = phys_hit_min.floor() as i64;
    let phys_hit_max_i = phys_hit_max.floor() as i64;
    let phys_avg_min_i = phys_avg_min.floor() as i64;
    let phys_avg_max_i = phys_avg_max.floor() as i64;
    let converted_hit = converted_elements
        .iter()
        .fold((0, 0), |sum, c| (sum.0 + c.hit_min, sum.1 + c.hit_max));
    let converted_avg = converted_elements
        .iter()
        .fold((0, 0), |sum, c| (sum.0 + c.avg_min, sum.1 + c.avg_max));
    let combined_hit_min = phys_hit_min_i + poison_hit_min + converted_hit.0;
    let combined_hit_max = phys_hit_max_i + poison_hit_max + converted_hit.1;
    let combined_avg_min = phys_avg_min_i + poison_avg_min + converted_avg.0;
    let combined_avg_max = phys_avg_max_i + poison_avg_max + converted_avg.1;

    let (aps_min, aps_max) = super::attacks_per_second(input.stats);

    let (aps_min, aps_max) = input.action_rate_override.unwrap_or((aps_min, aps_max));

    let dps_min = (combined_avg_min as f64) * aps_min;
    let dps_max = (combined_avg_max as f64) * aps_max;

    let mut calculation = Vec::new();
    stat_inputs(&mut calculation, input.stats, ["all_skills"]);
    if let Some(keys) = s.damage_type.as_deref().and_then(element_keys) {
        stat_inputs(&mut calculation, input.stats, [keys.skills]);
    }
    calculation.push(CalculationStep::new(
        "Effective attack rank",
        match fixed_rank {
            Some(rank) => format!(
                "{} explicit triggered rank; all/element/tag/item skill ranks are not added again",
                number(rank),
            ),
            None => format!(
                "{} allocated + {} all skills + {} element + {} tagged + {} item ranks",
                number(input.allocated_rank),
                range(all_skills),
                range(elem_bonus),
                range(tag_bonus),
                range(item)
            ),
        },
        (eff_min, eff_max),
    ));
    calculation.push(CalculationStep::new(
        "Weapon damage",
        input
            .weapon
            .map(|w| w.name.clone())
            .unwrap_or_else(|| "Unarmed baseline".into()),
        (w_min, w_max),
    ));
    for (label, formula, result) in [
        (
            "Skill weapon damage %",
            scaling.weapon_damage_pct.as_ref(),
            (skill_wdp_min, skill_wdp_max),
        ),
        (
            "Skill flat physical minimum",
            scaling.flat_physical_min.as_ref(),
            scalar(skill_flat_min),
        ),
        (
            "Skill flat physical maximum",
            scaling.flat_physical_max.as_ref(),
            scalar(skill_flat_max),
        ),
    ] {
        if let Some(formula) = formula {
            calculation.push(CalculationStep::new(
                label,
                format!(
                    "max(0, {} + {} × effective rank)",
                    number(formula.base),
                    number(formula.per_level)
                ),
                result,
            ));
        }
    }
    stat_inputs(
        &mut calculation,
        input.stats,
        [
            "enhanced_damage",
            "enhanced_damage_more",
            "additive_physical_damage",
            "minimum_damage_flat",
            "maximum_damage_flat",
            "minimum_damage_pct",
            "maximum_damage_pct",
            "attack_damage",
            "attack_damage_more",
            "armor_break",
            "crushing_blow_armor_break",
            "deadly_blow_chance",
            "deadly_blow",
            "deadly_blow_effectiveness",
            "crit_chance",
            "crit_damage",
            "crit_damage_more",
            "attacks_per_second",
            "increased_attack_speed",
            "increased_attack_speed_more",
        ],
    );
    calculation.push(CalculationStep::new(
        "Converted flat physical damage",
        "Conversion applies to physical only when no elemental member consumes it",
        scalar(input.conversion_flat),
    ));
    if s.tags.iter().any(|tag| tag == "Leap") && rg(input.stats, "flat_leap_damage") != (0.0, 0.0) {
        calculation.push(CalculationStep::new(
            "Flat Leap damage on the weapon component",
            "The physical weapon coefficient does not consume flat Leap damage",
            scalar(0.0),
        ));
    }
    calculation.push(CalculationStep::new("Physical base", format!("({} weapon × (1 + {}% enhanced / 100) × (1 + {}% more / 100) + {} additive + {} ranged flat + {} tagged flat + {} skill flat + {} converted + {} bound flat) × {} bound multipliers", range((w_min, w_max)), range(ed), range(ed_more), range(add_phys), range(ranged_flat), range(tagged_flat), range((skill_flat_min, skill_flat_max)), number(input.conversion_flat), range(bound_flat), range(bound_multiplier)), (base_min, base_max)));
    calculation.extend(synergy_steps);
    if weapon_bonus_scaling {
        stat_inputs(
            &mut calculation,
            input.stats,
            affix_tags::keys_for(AffixEffect::Damage, &s.tags)
                .into_iter()
                .chain(affix_tags::keys_for(AffixEffect::DamageMore, &s.tags)),
        );
        calculation.push(CalculationStep::new(
            "Weapon bonus tag multiplier",
            format!(
                "(1 + {}% matching tag damage / 100) × {} matching tag more; scales the weapon bonus before adding the base weapon",
                range(tagged),
                range(tagged_more)
            ),
            tagged_factor,
        ));
    }
    {
        stat_inputs(
            &mut calculation,
            input.stats,
            [
                "physical_skill_damage",
                "physical_skill_damage_more",
                "attack_rating_pct",
            ],
        );
        calculation.push(CalculationStep::new(
            "Physical skill multiplier",
            if weapon_bonus_scaling {
                "(1 + physical skill damage% / 100) × physical more; matching tags already scale the weapon bonus"
            } else {
                "(1 + (physical + matching tagged skill damage)% / 100) × physical more × tagged more"
            },
            physical_skill_mult,
        ));
        calculation.push(CalculationStep::new("Attack rating bonus %", format!("Skill rank formula × (1 + {}% intrinsic skill multiplier / 100) + shared and subtree attack-rating bonuses; accuracy is not inferred without enemy evasion", range(rating_multiplier)), (skill_arp_min, skill_arp_max)));
    }

    calculation.push(CalculationStep::new(
        "Attack damage multiplier",
        format!(
            "(1 + ({} attack + {} synergy)% / 100) × (1 + {}% more / 100)",
            range(atk),
            range(attack_synergy),
            range(atk_more)
        ),
        (atk_mult_min, atk_mult_max),
    ));
    if subtree_per_dexterity != (0.0, 0.0) {
        calculation.push(CalculationStep::new(
            "Skill damage from Dexterity",
            format!(
                "{} Dexterity / 100 × {}% per 100 Dexterity / 100; added to the own subtree multiplier",
                range(dexterity),
                range(subtree_per_dexterity)
            ),
            dexterity_subtree,
        ));
    }
    calculation.push(CalculationStep::new(
        "Skill weapon multiplier",
        if weapon_bonus_scaling && scaling.weapon_subtree_scales_full_hit {
            format!(
                "max(0, (1 + ({}% weapon damage − 100 + {}% synergy) × {} matching tag multiplier / 100) × {} own subtree multiplier)",
                range((skill_wdp_min, skill_wdp_max)),
                range((synergy_min, synergy_max)),
                range(tagged_factor),
                range(subtree_factor)
            )
        } else if weapon_bonus_scaling {
            format!(
                "max(0, 1 + ({}% weapon damage − 100 + {}% synergy) × {} matching tag multiplier × {} own subtree multiplier / 100)",
                range((skill_wdp_min, skill_wdp_max)),
                range((synergy_min, synergy_max)),
                range(tagged_factor),
                range(subtree_factor)
            )
        } else if has_wdp {
            format!(
                "{}% weapon damage / 100",
                range((skill_wdp_min, skill_wdp_max))
            )
        } else {
            "No weapon scaling formula: ×1".into()
        },
        (skill_mult_min, skill_mult_max),
    ));
    calculation.push(CalculationStep::new(
        "Crushing blow + armor break",
        format!(
            "1 + ({}% Armor Break + {}% Crushing Blow stacks) / 100; physical damage only, conversions copy the hit before it",
            number(armor_break_pct),
            number(crushing_blow_pct)
        ),
        scalar(crush_armor_mult),
    ));
    calculation.push(CalculationStep::new(
        "Deadly blow multiplier",
        format!(
            "1 + clamp({}%, 0, 100) / 100 × (1.35 × (1 + {}% effectiveness / 100) − 1)",
            number(deadly_blow_chance),
            number(total_pct(input, "deadly_blow_effectiveness"))
        ),
        scalar(deadly_mult),
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
        format!(
            "(1 + sum of applicable build extra damage % / 100) × max(0, 1 + {}% subtree / 100)",
            number(input.of_total_damage)
        ),
        scalar(extra_mult),
    ));
    calculation.push(CalculationStep::new(
        "Physical hit before rounding",
        format!(
            "{} base × {} attack × {} skill × {} crush/armor × {} deadly × {} extra × {} kept after conversion",
            range((base_min, base_max)),
            range((atk_mult_min, atk_mult_max)),
            range((skill_mult_min, skill_mult_max)),
            number(crush_armor_mult),
            number(deadly_mult),
            number(extra_mult),
            range(kept)
        ),
        (phys_hit_min, phys_hit_max),
    ));
    calculation.push(CalculationStep::new(
        "Physical hit",
        "Floor physical hit before rounding; one projectile",
        (phys_hit_min.floor(), phys_hit_max.floor()),
    ));
    calculation.push(CalculationStep::new(
        "Critical damage multiplier",
        if scaling.critical_damage.is_some() || subtree_crit_damage != (0.0, 0.0) {
            format!(
                "{} global critical multiplier + {}% from this skill / 100; intrinsic critical damage uses {}× from its subtree, then adds {} local critical points",
                number(global_crit.on_crit_mult),
                range(skill_crit_damage),
                range(critical_damage_multiplier),
                range(subtree_crit_damage)
            )
        } else {
            format!(
                "(1 + {}% critical damage / 100) × (1 + {}% more / 100)",
                number(crit.damage_pct),
                number(r_max(rg(input.stats, "crit_damage_more")))
            )
        },
        (crit_min.on_crit_mult, crit.on_crit_mult),
    ));
    if scaling.critical_chance.is_some() || subtree_crit_chance != (0.0, 0.0) {
        calculation.push(CalculationStep::new(
            "Skill critical chance",
            format!(
                "{}% global + {}% from this skill, after global stat scaling; capped at 95% for the critical roll",
                number(global_crit.chance),
                range(skill_crit_chance),
            ),
            (crit_min.chance, crit.chance),
        ));
    }
    calculation.push(CalculationStep::new(
        "Average critical multiplier",
        format!(
            "1 + clamp({}%, 0, 95) / 100 × ({} critical multiplier − 1)",
            range((crit_min.chance, crit.chance)),
            range((crit_min.on_crit_mult, crit.on_crit_mult))
        ),
        (crit_min.avg_mult, crit.avg_mult),
    ));
    calculation.push(CalculationStep::new("Double damage expectation", "1 + clamp(double damage chance, 0, 100) / 100; one contact, independent of critical and extra damage", double_damage));
    if third_attack != (0.0, 0.0) || proc_subtree != (0.0, 0.0) {
        calculation.push(CalculationStep::new("Expected weapon multiplier", format!("Own subtree gains {}% chance-weighted damage and {}% / 3 from every third attack, at this skill's weapon stage; long-run average", range(proc_subtree), range(third_attack)), expected_skill_mult));
    }
    if third_crit_damage != (0.0, 0.0) {
        calculation.push(CalculationStep::new(
            "Third attack critical contribution",
            format!("Third attack uses {} weapon multiplier × critical chance × {} local critical damage / 100 / 3; damage and critical bonuses stay on the same hit, outside global critical more", range(third_skill_mult), range(third_crit_damage)),
            third_crit_extra,
        ));
    }
    let daisy_size = rg(input.scoped, "daisy_size_scaling");
    if daisy_size != (0.0, 0.0) {
        calculation.push(CalculationStep::new(
            "Daisy size on every third attack",
            format!("0.75 + (1 + missing life / maximum life) × {} / 100; scale from full life to the zero-life limit for Daisy only, without adding target contacts", range(daisy_size)),
            (0.75 + daisy_size.0 / 100.0, 0.75 + 2.0 * daisy_size.1 / 100.0),
        ));
    }
    let melee_size_from_strength = rg(input.scoped, "skill_melee_size_offset_per_100_strength");
    if melee_size_from_strength != (0.0, 0.0) {
        let strength = rg(input.attributes, "strength");
        calculation.push(CalculationStep::new(
            "Local melee size offset from Strength",
            format!("{} Strength × {}% per 100 Strength / 10000; additive scale offset for eligible melee projectiles, separate from generic range and target contacts", range(strength), range(melee_size_from_strength)),
            (
                (strength.0 * 0.01) * (melee_size_from_strength.0 * 0.01),
                (strength.1 * 0.01) * (melee_size_from_strength.1 * 0.01),
            ),
        ));
    }
    let life_recovery = rg(input.scoped, "skill_life_recovery_from_max_life_pct");
    if life_recovery != (0.0, 0.0) {
        let life = rg(input.stats, "life");
        calculation.push(CalculationStep::new(
            "Life recovery before blocking and caps",
            format!("floor({} maximum life × ({}% × 0.01)) with no recovery block, before limiting recovery to missing life; a positive recovery-block scalar reduces this floored amount before a second floor, and a block of 100 or more prevents recovery. No passive-regeneration modifiers or contact count; no recovery rate", range(life), range(life_recovery)),
            ((life.0 * (life_recovery.0 * 0.01)).floor(), (life.1 * (life_recovery.1 * 0.01)).floor()),
        ));
    }
    let mana_recovery = rg(input.scoped, "skill_mana_recovery_from_max_mana_pct");
    if mana_recovery != (0.0, 0.0) {
        let mana = rg(input.stats, "mana");
        let amount = (
            (mana.0 * (mana_recovery.0 * 0.01)).floor(),
            (mana.1 * (mana_recovery.1 * 0.01)).floor(),
        );
        calculation.push(CalculationStep::new(
            "Mana recovery controller base amount",
            format!("floor({} maximum mana × ({}% × 0.01)); controller input before limiting each delivered portion to missing mana, without passive-regeneration modifiers or contact count", range(mana), range(mana_recovery)),
            amount,
        ));
        calculation.push(CalculationStep::new(
            "Mana recovery controller portion",
            format!("{} controller base amount / 8; nominal two-second controller requires no existing mana controller. Exact tick count, delivered total and recovery per second remain unverified", range(amount)),
            (amount.0 / 8.0, amount.1 / 8.0),
        ));
    }
    calculation.push(CalculationStep::new(
        "Average physical damage",
        format!(
            "floor(({} expected physical hit × {} average crit + {} third-attack critical contribution) × {} double damage expectation × ({} ordinary + {} expected additional projectiles) × {} kept after conversion)",
            range(physical_expected),
            range((crit_min.avg_mult, crit.avg_mult)),
            range(third_crit_extra),
            range(double_damage),
            projectiles,
            range(extra_projectiles),
            range(kept)
        ),
        (phys_avg_min.floor(), phys_avg_max.floor()),
    ));
    calculation.extend(conversion_steps);
    calculation.push(CalculationStep::new(
        "Combined hit damage",
        format!(
            "{} physical + {} elemental (see elemental calculation) + {} converted elements",
            range((phys_hit_min_i as f64, phys_hit_max_i as f64)),
            range((poison_hit_min as f64, poison_hit_max as f64)),
            range((converted_hit.0 as f64, converted_hit.1 as f64))
        ),
        (combined_hit_min as f64, combined_hit_max as f64),
    ));
    calculation.push(CalculationStep::new(
        "Average damage per swing",
        format!(
            "{} average physical + {} average elemental + {} average converted elements",
            range((phys_avg_min_i as f64, phys_avg_max_i as f64)),
            range((poison_avg_min as f64, poison_avg_max as f64)),
            range((converted_avg.0 as f64, converted_avg.1 as f64))
        ),
        (combined_avg_min as f64, combined_avg_max as f64),
    ));
    calculation.push(CalculationStep::new(
        if input.action_rate_override.is_some() {
            "Configured actions per second"
        } else {
            "Attacks per second"
        },
        if input.action_rate_override.is_some() {
            "Action cadence supplied by the entity, cooldown or persistent-skill model".into()
        } else {
            format!(
                "{} base + min(base, 0.75) × {}% increased / 100, × (1 + {}% more / 100)",
                number(r_max(rg(input.stats, "attacks_per_second"))),
                range(rg(input.stats, "increased_attack_speed")),
                range(rg(input.stats, "increased_attack_speed_more"))
            )
        },
        (aps_min, aps_max),
    ));
    calculation.push(CalculationStep::new(
        "Attack DPS before entities / repeated hits",
        format!(
            "{} average per swing × {} attacks per second",
            range((combined_avg_min as f64, combined_avg_max as f64)),
            range((aps_min, aps_max))
        ),
        (dps_min, dps_max),
    ));

    Some(AttackSkillDamageBreakdown {
        calculation,
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
        converted_elements,
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
            damage_scaling: Default::default(),
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
            action_rate_override: None,
            skill: s,
            allocated_rank: 1.0,
            attributes: &attrs,
            stats: shared,
            skill_ranks_by_name: skill_ranks,
            skills_by_name: &skills_by_name,
            item_skill_bonuses: &bonuses,
            enemy_conditions: &conds,
            enemy_resistances: &HashMap::new(),
            weapon: Some(&weapon),
            poison_breakdown: None,
            scoped,
            projectile_count,
            of_total_damage: r_max(rg(scoped, "of_total_damage")),
            conversion_flat,
        };
        compute_attack_skill_damage(&input).expect("attack breakdown")
    }

    fn hit_max(shared: &StatMap, scoped: &StatMap) -> i64 {
        breakdown_for(&skill(), shared, scoped, &SkillRanks::new(), 1, 0.0).combined_hit_max
    }

    #[test]
    fn crushing_blow_adds_to_armor_break_and_sundering_strikes() {
        // Sundering Strikes reaches the planner as subtree armor_break.
        let crushed = hit_max(
            &stats(&[("armor_break", 20.0), ("crushing_blow_armor_break", 50.0)]),
            &stats(&[("armor_break", 30.0)]),
        );
        assert_eq!(
            crushed,
            hit_max(&stats(&[("armor_break", 100.0)]), &StatMap::new())
        );
    }

    fn converted(shared: &[(&str, f64)], scoped: &[(&str, f64)]) -> AttackSkillDamageBreakdown {
        breakdown_for(
            &skill(),
            &stats(shared),
            &stats(scoped),
            &SkillRanks::new(),
            1,
            0.0,
        )
    }

    fn element_and_hits(d: &AttackSkillDamageBreakdown) -> (&str, i64, i64) {
        assert_eq!(d.converted_elements.len(), 1);
        let c = &d.converted_elements[0];
        (c.element.as_str(), d.physical_hit_max, c.hit_max)
    }

    #[test]
    fn physical_converts_only_to_the_element_with_the_largest_total() {
        let d = converted(
            &[("physical_to_cold", 50.0), ("physical_to_fire", 70.0)],
            &[],
        );
        assert_eq!(element_and_hits(&d), ("fire", 30, 70));
    }

    #[test]
    fn tied_conversions_follow_arcane_cold_fire_lightning_poison() {
        for (first, second) in [
            ("arcane", "cold"),
            ("cold", "fire"),
            ("fire", "lightning"),
            ("lightning", "poison"),
            ("arcane", "poison"),
        ] {
            let a = format!("physical_to_{first}");
            let b = format!("physical_to_{second}");
            let d = converted(&[(b.as_str(), 50.0), (a.as_str(), 50.0)], &[]);
            assert_eq!(element_and_hits(&d), (first, 50, 50));
        }
    }

    #[test]
    fn the_losing_conversion_is_ignored_and_its_share_stays_physical() {
        // Cold Impact + Fire Impact: 50% cold, the fire 50% does nothing.
        let impacts = [("physical_to_cold", 50.0), ("physical_to_fire", 50.0)];
        let d = converted(&impacts, &[]);
        assert_eq!(element_and_hits(&d), ("cold", 50, 50));
        assert_eq!(d.combined_hit_max, 100);
        // +20% fire from items: fire reaches 70% and Cold Impact stops working.
        let d = converted(
            &[("physical_to_cold", 50.0), ("physical_to_fire", 70.0)],
            &[],
        );
        assert_eq!(element_and_hits(&d), ("fire", 30, 70));
    }

    #[test]
    fn sources_of_the_same_element_add_up_before_the_comparison() {
        let d = converted(
            &[("physical_to_cold", 50.0), ("physical_to_fire", 60.0)],
            &[("physical_to_cold", 20.0)],
        );
        assert_eq!(element_and_hits(&d), ("cold", 30, 70));
    }

    #[test]
    fn conversion_keeps_the_rest_physical_and_scales_by_the_elements_skill_damage() {
        // 1000 physical, Cold Impact and +40% Cold Skill Damage: 500 physical + 500 × 1.4 cold.
        let mut s = skill();
        s.attack_scaling
            .as_mut()
            .unwrap()
            .weapon_damage_pct
            .as_mut()
            .unwrap()
            .base = 1000.0;
        let shared = stats(&[
            ("physical_to_cold", 50.0),
            ("cold_skill_damage", 40.0),
            ("magic_skill_damage", 100.0),
            ("cold_skill_damage_more", 100.0),
        ]);
        let d = breakdown_for(&s, &shared, &StatMap::new(), &SkillRanks::new(), 1, 0.0);
        assert_eq!(element_and_hits(&d), ("cold", 500, 700));
        assert_eq!(d.combined_hit_max, 1200);
    }

    #[test]
    fn converted_damage_takes_the_elements_resistance_and_breaks() {
        let s = skill();
        let empty = StatMap::new();
        let ranks = SkillRanks::new();
        let skills = HashMap::new();
        let weapon = Weapon {
            name: "test".into(),
            damage_min: 100.0,
            damage_max: 100.0,
        };
        let shared = stats(&[
            ("physical_to_cold", 50.0),
            ("elemental_break_on_strike", 20.0),
            ("cold_break", 30.0),
        ]);
        let scoped = stats(&[("enemy_damage_taken_increased", 10.0)]);
        let resistance = HashMap::from([("cold".into(), 50.0)]);
        let hit = |conditions: &ConditionMap| {
            compute_attack_skill_damage(&AttackSkillInput {
                action_rate_override: None,
                skill: &s,
                allocated_rank: 1.0,
                attributes: &empty,
                stats: &shared,
                skill_ranks_by_name: &ranks,
                skills_by_name: &skills,
                item_skill_bonuses: &empty,
                enemy_conditions: conditions,
                enemy_resistances: &resistance,
                weapon: Some(&weapon),
                poison_breakdown: None,
                scoped: &scoped,
                projectile_count: 1,
                of_total_damage: 0.0,
                conversion_flat: 0.0,
            })
            .unwrap()
        };
        // 100 × 50% × (1 − 50% cold res) × [1 + (20% + 30% Cold Break + 10% taken) / 100].
        let broken = hit(&ConditionMap::from([("cold_break".into(), true)]));
        assert_eq!(element_and_hits(&broken), ("cold", 50, 40));
        let open = hit(&ConditionMap::new());
        assert_eq!(element_and_hits(&open), ("cold", 50, 32));
    }

    #[test]
    fn crushing_blow_and_armor_break_reach_only_the_kept_physical() {
        let conversion = [("physical_to_cold", 50.0)];
        assert_eq!(
            element_and_hits(&converted(&conversion, &[])),
            ("cold", 50, 50)
        );
        let breaks = [
            ("physical_to_cold", 50.0),
            ("armor_break", 50.0),
            ("crushing_blow_armor_break", 50.0),
        ];
        // 100 × 50% kept × (1 + (50% + 50%) / 100); the cold half takes neither.
        assert_eq!(
            element_and_hits(&converted(&breaks, &[])),
            ("cold", 100, 50)
        );
    }

    #[test]
    fn intrinsic_critical_chance_is_local_and_capped_after_global_chance() {
        let mut s = skill();
        s.attack_scaling.as_mut().unwrap().critical_chance = Some(DamageFormula {
            base: 50.0,
            per_level: 0.0,
        });
        for (global, expected_chance, expected_average) in [(20., 70., 1.7), (70., 120., 1.95)] {
            let shared = stats(&[("crit_chance", global), ("crit_damage", 100.)]);
            let plain = breakdown_for(
                &skill(),
                &shared,
                &StatMap::new(),
                &SkillRanks::new(),
                1,
                0.,
            );
            let blade = breakdown_for(&s, &shared, &StatMap::new(), &SkillRanks::new(), 1, 0.);
            assert_eq!(blade.crit_chance, expected_chance);
            assert!((blade.crit_multiplier_avg - expected_average).abs() < 1e-12);
            assert_eq!(blade.physical_hit_min, plain.physical_hit_min);
            assert_eq!(blade.physical_hit_max, plain.physical_hit_max);
            assert_eq!(plain.crit_chance, global);
            assert_eq!(shared["crit_chance"], (global, global));
        }
    }

    #[test]
    fn intrinsic_critical_chance_uses_both_effective_rank_endpoints() {
        let mut s = skill();
        s.attack_scaling.as_mut().unwrap().critical_chance = Some(DamageFormula {
            base: 10.0,
            per_level: 10.0,
        });
        let mut shared = stats(&[("crit_chance", 20.), ("crit_damage", 100.)]);
        shared.insert("all_skills".into(), (0., 2.));
        let damage = breakdown_for(&s, &shared, &StatMap::new(), &SkillRanks::new(), 1, 0.);
        assert_eq!(
            (damage.effective_rank_min, damage.effective_rank_max),
            (1., 3.)
        );
        assert_eq!(
            damage.physical_avg_min,
            (damage.physical_hit_min as f64 * 1.4).floor() as i64
        );
        assert_eq!(
            damage.physical_avg_max,
            (damage.physical_hit_max as f64 * 1.6).floor() as i64
        );
        let step = damage
            .calculation()
            .iter()
            .find(|step| step.label() == "Average critical multiplier")
            .unwrap();
        assert!((step.value().0 - 1.4).abs() < 1e-12);
        assert!((step.value().1 - 1.6).abs() < 1e-12);

        let scoped = StatMap::from([("skill_crit_chance".into(), (5., 25.))]);
        let local = breakdown_for(&s, &shared, &scoped, &SkillRanks::new(), 1, 0.);
        assert_eq!(local.physical_hit_min, damage.physical_hit_min);
        assert_eq!(local.physical_hit_max, damage.physical_hit_max);
        let step = local
            .calculation()
            .iter()
            .find(|step| step.label() == "Average critical multiplier")
            .unwrap();
        assert!((step.value().0 - 1.45).abs() < 1e-12);
        assert!((step.value().1 - 1.85).abs() < 1e-12);
    }

    #[test]
    fn local_critical_damage_adds_after_global_and_intrinsic_multipliers() {
        let mut s = skill();
        s.attack_scaling.as_mut().unwrap().critical_damage = Some(DamageFormula {
            base: 10.,
            per_level: 0.,
        });
        let shared = stats(&[
            ("crit_chance", 50.),
            ("crit_damage", 100.),
            ("crit_damage_more", 50.),
        ]);
        let scoped = StatMap::from([
            ("skill_crit_damage_multiplier_pct".into(), (100., 100.)),
            ("skill_crit_damage".into(), (30., 90.)),
        ]);
        let out = breakdown_for(&s, &shared, &scoped, &SkillRanks::new(), 1, 0.);
        assert_eq!((out.physical_hit_min, out.physical_hit_max), (100, 100));
        // 100 × (1 + 50% × (3.5..4.1 − 1)); 255 floors to 254 in f64.
        assert_eq!((out.physical_avg_min, out.physical_avg_max), (225, 254));
        let step = out
            .calculation()
            .iter()
            .find(|step| step.label() == "Critical damage multiplier")
            .unwrap();
        assert!((step.value().0 - 3.5).abs() < 1e-9);
        assert!((step.value().1 - 4.1).abs() < 1e-9);
        assert_eq!(out.crit_damage_pct, 210.);
        assert_eq!(shared["crit_damage"], (100., 100.));
    }

    #[test]
    fn intrinsic_critical_damage_uses_rank_range_after_global_multipliers() {
        let mut blade = skill();
        let scaling = blade.attack_scaling.as_mut().unwrap();
        scaling.critical_chance = Some(DamageFormula {
            base: 50.,
            per_level: 0.,
        });
        scaling.critical_damage = Some(DamageFormula {
            base: 67.5,
            per_level: 22.5,
        });
        let mut shared = stats(&[
            ("crit_chance", 20.),
            ("crit_damage", 100.),
            ("crit_damage_more", 50.),
        ]);
        shared.insert("all_skills".into(), (0., 2.));
        let ordinary = breakdown_for(
            &skill(),
            &shared,
            &StatMap::new(),
            &SkillRanks::new(),
            1,
            0.,
        );
        let damage = breakdown_for(&blade, &shared, &StatMap::new(), &SkillRanks::new(), 1, 0.);
        let expected = (1. + 0.7 * (3.9 - 1.), 1. + 0.7 * (4.35 - 1.));
        assert_eq!(damage.physical_hit_min, ordinary.physical_hit_min);
        assert_eq!(damage.physical_hit_max, ordinary.physical_hit_max);
        assert_eq!(
            damage.physical_avg_min,
            (damage.physical_hit_min as f64 * expected.0).floor() as i64
        );
        assert_eq!(
            damage.physical_avg_max,
            (damage.physical_hit_max as f64 * expected.1).floor() as i64
        );
        assert_eq!(shared["crit_damage"], (100., 100.));
        let step = damage
            .calculation()
            .iter()
            .find(|s| s.label() == "Critical damage multiplier")
            .unwrap();
        assert!((step.value().0 - 3.9).abs() < 1e-12);
        assert!((step.value().1 - 4.35).abs() < 1e-12);
        assert_eq!(ordinary.crit_damage_pct, 100.);
    }

    #[test]
    fn conversion_uses_the_skill_hit_and_destination_bonus_and_keeps_the_rest_physical() {
        let mut s = skill();
        s.attack_scaling
            .as_mut()
            .unwrap()
            .weapon_damage_pct
            .as_mut()
            .unwrap()
            .base = 300.0;
        let shared = stats(&[
            ("physical_to_fire", 10.0),
            ("fire_skill_damage", 100.0),
            ("flat_fire_skill_damage", 99999.0),
        ]);
        let d = breakdown_for(&s, &shared, &StatMap::new(), &SkillRanks::new(), 1, 0.0);
        assert_eq!(d.physical_hit_max, 270);
        assert_eq!(d.converted_elements[0].hit_max, 60);
        assert_eq!(d.combined_hit_max, 330);
    }

    #[test]
    fn conversion_past_one_hundred_percent_turns_the_kept_physical_negative() {
        // The game has no cap: 150% leaves 1 − 1.5 of the physical hit.
        let shared = StatMap::from([
            ("physical_to_fire".into(), (100.0, 150.0)),
            ("physical_to_cold".into(), (100.0, 150.0)),
            ("crit_chance".into(), (50.0, 50.0)),
            ("crit_damage".into(), (100.0, 100.0)),
            ("double_damage_chance".into(), (100.0, 100.0)),
        ]);
        let base = breakdown_for(
            &skill(),
            &shared
                .iter()
                .filter(|(k, _)| !k.starts_with("physical_to_"))
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            &StatMap::new(),
            &SkillRanks::new(),
            3,
            0.0,
        );
        let d = breakdown_for(
            &skill(),
            &shared,
            &StatMap::new(),
            &SkillRanks::new(),
            3,
            0.0,
        );
        assert_eq!(d.converted_elements.len(), 1);
        assert_eq!(d.converted_elements[0].element, "cold");
        assert_eq!((d.physical_hit_min, d.physical_hit_max), (0, -50));
        assert_eq!((d.physical_avg_min, d.physical_avg_max), (0, -450));
        assert_eq!(d.converted_elements[0].avg_max, 1350);
        assert_eq!(d.combined_hit_min, base.combined_hit_min);
        assert_eq!(d.combined_hit_max, base.combined_hit_max);
        assert_eq!(d.combined_avg_min, base.combined_avg_min);
        assert_eq!(d.combined_avg_max, base.combined_avg_max);
    }

    #[test]
    fn converted_element_uses_its_own_skill_damage_resistance_and_break() {
        let s = skill();
        let empty = StatMap::new();
        let ranks = SkillRanks::new();
        let skills = HashMap::new();
        let shared = stats(&[
            ("physical_to_fire", 20.0),
            ("physical_to_cold", 10.0),
            ("fire_skill_damage", 50.0),
            ("magic_skill_damage", 50.0),
            ("fire_skill_damage_more", 100.0),
            ("ignore_fire_res", 50.0),
            ("elemental_break_on_strike", 100.0),
            ("fire_break", 100.0),
        ]);
        let resistance = HashMap::from([("fire".into(), 50.0), ("cold".into(), 100.0)]);
        let conditions = ConditionMap::from([("fire_break".into(), true)]);
        let input = AttackSkillInput {
            action_rate_override: None,
            skill: &s,
            allocated_rank: 1.0,
            attributes: &empty,
            stats: &shared,
            skill_ranks_by_name: &ranks,
            skills_by_name: &skills,
            item_skill_bonuses: &empty,
            enemy_conditions: &conditions,
            enemy_resistances: &resistance,
            weapon: None,
            poison_breakdown: None,
            scoped: &empty,
            projectile_count: 1,
            of_total_damage: 0.0,
            conversion_flat: 0.0,
        };
        let conversion = physical_conversion(&input);
        let (components, _) =
            converted_attack_elements(&input, conversion, (300.0, 300.0), (900.0, 900.0));
        assert_eq!(components.len(), 1);
        let fire = &components[0];
        // 300 × 20% × (1 + 50% fire) × .75 res × (1 + 1 + 1 Break); magic and more stay out.
        assert_eq!(fire.element, "fire");
        assert_eq!(fire.hit_max, 202);
        assert_eq!(fire.avg_max, 607);
        let mut critical_stats = shared.clone();
        critical_stats.insert("fire_break_crit_chance".into(), (15.0, 100.0));
        critical_stats.insert("fire_break_crit_damage".into(), (25.0, 25.0));
        let (critical, _) = converted_attack_elements(
            &AttackSkillInput {
                stats: &critical_stats,
                ..input
            },
            conversion,
            (300.0, 300.0),
            (900.0, 900.0),
        );
        let critical_fire = &critical[0];
        assert_eq!((critical_fire.hit_min, critical_fire.hit_max), (202, 202));
        assert_eq!((critical_fire.avg_min, critical_fire.avg_max), (615, 658));
    }

    #[test]
    fn weapon_damage_bounds_follow_enhanced_damage_and_feed_conversion_once() {
        let shared = stats(&[
            ("enhanced_damage", 100.0),
            ("minimum_damage_flat", 8.0),
            ("maximum_damage_flat", 32.0),
            ("minimum_damage_pct", 50.0),
            ("maximum_damage_pct", 100.0),
            ("physical_to_fire", 100.0),
        ]);
        let d = breakdown_for(
            &skill(),
            &shared,
            &StatMap::new(),
            &SkillRanks::new(),
            1,
            0.0,
        );
        // (100 * 2 + 8) * 1.5 and (100 * 2 + 32) * 2, all of it fire.
        assert_eq!((d.physical_hit_min, d.physical_hit_max), (0, 0));
        assert_eq!(d.converted_elements[0].hit_min, 312);
        assert_eq!(d.converted_elements[0].hit_max, 464);
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
    fn incarnation_melee_projectile_and_leap_modifiers_reach_only_matching_attacks() {
        let mut s = skill();
        let shared = stats(&[
            ("melee_projectile_damage", 8.0),
            ("melee_projectile_crit_damage", 30.0),
            ("flat_leap_damage", 12.0),
            ("leap_damage", 15.0),
        ]);
        let empty = StatMap::new();
        let ranks = SkillRanks::new();
        s.tags = vec!["Melee".into(), "Projectile".into()];
        let melee = breakdown_for(&s, &shared, &empty, &ranks, 1, 0.0);
        let base = breakdown_for(&s, &empty, &empty, &ranks, 1, 0.0);
        assert!(melee.combined_hit_max > base.combined_hit_max);
        let crit = super::super::crit_factors_with_tags(&shared, false, &s.tags);
        assert_eq!(crit.damage_pct, 30.0);
        s.tags = vec!["Ranged".into(), "Projectile".into()];
        let ranged = breakdown_for(&s, &shared, &empty, &ranks, 1, 0.0);
        assert_eq!(ranged.combined_hit_max, base.combined_hit_max);
        assert_eq!(
            super::super::crit_factors_with_tags(&shared, false, &s.tags).damage_pct,
            0.0
        );
        s.tags = vec!["Leap".into()];
        let leap = breakdown_for(&s, &shared, &empty, &ranks, 1, 0.0);
        assert!(leap.combined_hit_max > base.combined_hit_max);
    }

    #[test]
    fn energetic_carnage_adds_physical_only_to_ranged_attacks() {
        let mut s = skill();
        let shared = stats(&[("flat_ranged_physical_damage", 250.0)]);
        let empty = StatMap::new();
        let ranks = SkillRanks::new();
        let base = breakdown_for(&s, &empty, &empty, &ranks, 1, 0.0);
        assert_eq!(
            breakdown_for(&s, &shared, &empty, &ranks, 1, 0.0).physical_hit_max,
            base.physical_hit_max
        );
        s.tags.push("Ranged".into());
        let ranged = breakdown_for(&s, &shared, &empty, &ranks, 1, 0.0);
        assert_eq!(
            ranged.physical_hit_max,
            (base.physical_hit_max as f64 * 3.5) as i64
        );
    }

    #[test]
    fn double_damage_roll_changes_average_not_contact_damage_or_count() {
        let base = breakdown_for(
            &skill(),
            &StatMap::new(),
            &StatMap::new(),
            &SkillRanks::new(),
            3,
            0.0,
        );
        let doubled = breakdown_for(
            &skill(),
            &StatMap::new(),
            &stats(&[("double_damage_chance", 100.0)]),
            &SkillRanks::new(),
            3,
            0.0,
        );
        assert_eq!(doubled.physical_hit_max, base.physical_hit_max);
        assert_eq!(doubled.projectile_count, base.projectile_count);
        assert_eq!(doubled.physical_avg_max, 2 * base.physical_avg_max);
        assert_eq!(doubled.attacks_per_second_max, base.attacks_per_second_max);
    }

    #[test]
    fn physical_skill_damage_and_matching_tags_apply_to_all_attacks() {
        let mut s = skill();
        s.tags.push("Projectile".into());
        s.tags.push("Ranged".into());
        let shared = stats(&[
            ("physical_skill_damage", 40.0),
            ("physical_skill_damage_more", 20.0),
            ("ranged_projectile_damage", 10.0),
            ("spell_damage", 500.0),
        ]);
        let result = breakdown_for(&s, &shared, &StatMap::new(), &SkillRanks::new(), 1, 0.0);
        // 100 weapon × (1 + (40 + 10)/100) × 1.2 = 180, floored from 179.99… in f64.
        assert_eq!(result.physical_hit_max, 179);
    }

    #[test]
    fn attack_rank_and_rating_use_shared_skill_bonuses() {
        let mut s = skill();
        s.tags.push("Projectile".into());
        let shared = stats(&[("projectile_skills", 3.0), ("attack_rating_pct", 25.0)]);
        let result = breakdown_for(
            &s,
            &shared,
            &stats(&[("attack_rating_pct", 10.0)]),
            &SkillRanks::new(),
            1,
            0.0,
        );
        assert_eq!(result.effective_rank_min, 4.0);
        assert_eq!(result.effective_rank_max, 4.0);
        assert_eq!(result.attack_rating_pct_max, 35.0);
    }

    #[test]
    fn intrinsic_attack_rating_multiplier_uses_rank_endpoints_only() {
        let mut s = skill();
        s.attack_scaling.as_mut().unwrap().attack_rating_pct = Some(DamageFormula {
            base: 10.,
            per_level: 10.,
        });
        let mut shared = stats(&[("attack_rating_pct", 25.)]);
        shared.insert("all_skills".into(), (0., 2.));
        let scoped = StatMap::from([
            ("attack_rating_pct".into(), (10., 10.)),
            ("skill_attack_rating_multiplier_pct".into(), (10., 50.)),
        ]);
        let plain = breakdown_for(&s, &shared, &StatMap::new(), &SkillRanks::new(), 1, 0.);
        let local = breakdown_for(&s, &shared, &scoped, &SkillRanks::new(), 1, 0.);
        assert_eq!(
            (local.effective_rank_min, local.effective_rank_max),
            (1., 3.)
        );
        assert_eq!(
            (local.attack_rating_pct_min, local.attack_rating_pct_max),
            (57., 95.)
        );
        assert_eq!(local.physical_hit_min, plain.physical_hit_min);
        assert_eq!(local.physical_avg_max, plain.physical_avg_max);
        s.attack_scaling.as_mut().unwrap().attack_rating_pct = None;
        let absent = breakdown_for(&s, &shared, &scoped, &SkillRanks::new(), 1, 0.);
        assert_eq!(
            (absent.attack_rating_pct_min, absent.attack_rating_pct_max),
            (35., 35.)
        );
    }

    #[test]
    fn physical_break_stage_matches_the_weapon_panel() {
        // weapon 100 with no break = 100, the same stage weapon.rs applies.
        assert_eq!(hit_max(&StatMap::new(), &StatMap::new()), 100);
    }

    #[test]
    fn armor_break_raises_attack_skill_damage() {
        // crush_armor_mult = 1 + 100/100 = 2
        assert_eq!(
            hit_max(&stats(&[("armor_break", 100.0)]), &StatMap::new()),
            200
        );
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
        // chance 100%: S8 deadly = 1.35 -> 100 * 1.35 = 135
        let plain = hit_max(&stats(&[("deadly_blow_chance", 100.0)]), &StatMap::new());
        assert_eq!(plain, 135);
        // +50% effectiveness: on-proc 1.35 * 1.5 = 2.025 -> 100 * 2.025 = 202.5
        let boosted = hit_max(
            &stats(&[
                ("deadly_blow_chance", 100.0),
                ("deadly_blow_effectiveness", 50.0),
            ]),
            &StatMap::new(),
        );
        assert_eq!(boosted, 202);
    }

    #[test]
    fn conversion_flat_lands_in_the_swing_before_the_multipliers() {
        // weapon 100 + 40 converted, then the same physical break stage.
        assert_eq!(hit_max_with_conversion(0.0), 100);
        assert_eq!(hit_max_with_conversion(40.0), 140);
    }

    #[test]
    fn subtree_deadly_blow_adds_to_the_chance() {
        assert_eq!(
            hit_max(&StatMap::new(), &stats(&[("deadly_blow", 100.0)])),
            135,
        );
    }

    #[test]
    fn projectiles_multiply_the_average_but_not_the_hit() {
        let plain = breakdown_for(
            &skill(),
            &StatMap::new(),
            &StatMap::new(),
            &SkillRanks::new(),
            1,
            0.0,
        );
        let fanned = breakdown_for(
            &skill(),
            &StatMap::new(),
            &StatMap::new(),
            &SkillRanks::new(),
            3,
            0.0,
        );
        assert_eq!(fanned.combined_hit_max, plain.combined_hit_max);
        assert_eq!(fanned.combined_avg_max, plain.combined_avg_max * 3);
        assert!((fanned.dps_max - plain.dps_max * 3.0).abs() < 1e-6);
    }

    #[test]
    fn bonus_source_synergy_raises_attack_damage() {
        // Flail Mastery rank 10 at 4%/level = +40% attack damage:
        // 100 * 1.4 = 140.
        let mut s = skill();
        s.bonus_sources = vec![crate::calc::skills::BonusSource::SkillLevel {
            source: "flail mastery".into(),
            stat: "attack_damage".into(),
            value: 4.0,
        }];
        let ranks: SkillRanks = [("flail mastery".to_string(), 10.0)].into_iter().collect();
        let out = breakdown_for(&s, &StatMap::new(), &StatMap::new(), &ranks, 1, 0.0);
        assert_eq!(out.combined_hit_max, 140);
        assert_eq!(out.synergy_max_pct, 40.0);
    }

    #[test]
    fn skill_weapon_damage_pct_is_a_standalone_multiplier() {
        // S8: 200% attack damage and 300% skill tooltip multiply, not add:
        // 100 * 3.0 * (1 + 2.0) = 900 (additive would give 600).
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
        assert_eq!(out.combined_hit_max, 900);
    }

    #[test]
    fn amazon_subtree_scales_weapon_bonus_and_keeps_elemental_pool_separate() {
        let mut s = skill();
        s.attack_scaling = Some(AttackSkillScaling {
            weapon_bonus_scaling: true,
            weapon_damage_pct: Some(DamageFormula {
                base: 185.0,
                per_level: 35.0,
            }),
            ..Default::default()
        });
        // Rank one: weapon 100 * (1 + 1.20 * (1 + .08 + .04)).
        // The poison-only bonus must not change this physical coefficient.
        let scoped = stats(&[
            ("subtree_damage", 8.0),
            ("weapon_subtree_damage", 4.0),
            ("elemental_subtree_damage", 500.0),
        ]);
        let out = breakdown_for(&s, &StatMap::new(), &scoped, &SkillRanks::new(), 1, 0.0);
        assert_eq!(out.physical_hit_max, 234);
        let extra = stats(&[("expected_additional_projectiles", 0.25)]);
        let out = breakdown_for(&s, &StatMap::new(), &extra, &SkillRanks::new(), 1, 0.0);
        assert_eq!(out.physical_hit_max, 220);
        assert_eq!(out.physical_avg_max, 275);
    }

    #[test]
    fn third_attack_damage_and_local_critical_bonus_remain_correlated() {
        let mut s = skill();
        s.attack_scaling = Some(AttackSkillScaling {
            weapon_bonus_scaling: true,
            weapon_damage_pct: Some(DamageFormula {
                base: 200.,
                per_level: 0.,
            }),
            ..Default::default()
        });
        let shared = stats(&[
            ("crit_chance", 50.),
            ("crit_damage", 100.),
            ("crit_damage_more", 50.),
        ]);
        let scoped = StatMap::from([
            ("third_attack_subtree_damage".into(), (50., 100.)),
            ("third_attack_skill_crit_damage".into(), (100., 200.)),
        ]);
        let out = breakdown_for(&s, &shared, &scoped, &SkillRanks::new(), 1, 0.);
        assert_eq!((out.physical_hit_min, out.physical_hit_max), (200, 200));
        assert_eq!((out.physical_avg_min, out.physical_avg_max), (475, 566));
        assert_eq!(out.crit_damage_pct, 100.);
        let mut no_crit = shared.clone();
        no_crit.insert("crit_chance".into(), (0., 0.));
        let out = breakdown_for(&s, &no_crit, &scoped, &SkillRanks::new(), 1, 0.);
        assert_eq!((out.physical_avg_min, out.physical_avg_max), (216, 233));
    }

    #[test]
    fn chance_weighted_subtree_damage_changes_only_expected_weapon_bonus() {
        let mut s = skill();
        s.attack_scaling = Some(AttackSkillScaling {
            weapon_bonus_scaling: true,
            weapon_damage_pct: Some(DamageFormula {
                base: 200.,
                per_level: 0.,
            }),
            ..Default::default()
        });
        let scoped = stats(&[("subtree_damage", 50.), ("subtree_damage_on_proc", 20.)]);
        let out = breakdown_for(&s, &StatMap::new(), &scoped, &SkillRanks::new(), 1, 0.);
        assert_eq!(out.physical_hit_max, 250);
        assert_eq!(out.physical_avg_max, 270);
    }
    #[test]
    fn whole_hit_subtree_retains_neutral_weapon_damage_and_both_bounds() {
        let mut s = skill();
        s.tags.push("Orbital".into());
        s.attack_scaling = Some(AttackSkillScaling {
            weapon_bonus_scaling: true,
            weapon_subtree_scales_full_hit: true,
            weapon_damage_pct: Some(DamageFormula {
                base: 200.0,
                per_level: 0.0,
            }),
            ..Default::default()
        });
        let shared = stats(&[("orbital_skill_damage", 100.0)]);
        let scoped = StatMap::from([("subtree_damage".into(), (0.0, 50.0))]);
        let whole = breakdown_for(&s, &shared, &scoped, &SkillRanks::new(), 1, 0.0);
        assert_eq!((whole.physical_hit_min, whole.physical_hit_max), (300, 450));
        s.attack_scaling
            .as_mut()
            .unwrap()
            .weapon_subtree_scales_full_hit = false;
        let bonus = breakdown_for(&s, &shared, &scoped, &SkillRanks::new(), 1, 0.0);
        assert_eq!((bonus.physical_hit_min, bonus.physical_hit_max), (300, 400));
    }

    #[test]
    fn weapon_bonus_scaling_keeps_base_weapon_outside_synergy_and_tags() {
        let mut s = skill();
        s.tags.push("Orbital".into());
        s.attack_scaling = Some(AttackSkillScaling {
            weapon_bonus_scaling: true,
            weapon_damage_pct: Some(DamageFormula {
                base: 175.0,
                per_level: 15.0,
            }),
            ..Default::default()
        });
        s.bonus_sources = vec![crate::calc::skills::BonusSource::SkillLevel {
            source: "test synergy".into(),
            stat: "attack_damage".into(),
            value: 3.0,
        }];
        for (shared, synergy_rank, expected_skill_mult, expected_hit) in [
            (stats(&[]), 0.0, 1.9, 190),
            (stats(&[("orbital_skill_damage", 100.0)]), 0.0, 2.8, 280),
            (stats(&[]), 10.0, 2.2, 220),
            (stats(&[("orbital_skill_damage", 100.0)]), 10.0, 3.4, 340),
            (
                stats(&[
                    ("orbital_skill_damage", 100.0),
                    ("physical_skill_damage", 50.0),
                ]),
                0.0,
                2.8,
                420,
            ),
            (stats(&[("attack_damage", 100.0)]), 10.0, 2.2, 440),
            (
                stats(&[("orbital_skill_damage_more", 100.0)]),
                0.0,
                2.8,
                280,
            ),
            (stats(&[("spell_damage", 100.0)]), 0.0, 1.9, 190),
        ] {
            let ranks = HashMap::from([("test synergy".into(), synergy_rank)]);
            let out = breakdown_for(&s, &shared, &StatMap::new(), &ranks, 3, 0.0);
            let skill_mult = out
                .calculation()
                .iter()
                .find(|step| step.label() == "Skill weapon multiplier")
                .unwrap()
                .value();
            assert!((skill_mult.0 - expected_skill_mult).abs() < 1e-9);
            assert!((skill_mult.1 - expected_skill_mult).abs() < 1e-9);
            // Three projectiles change only the volley average, never this
            // coefficient or individual hit.
            assert!((out.physical_hit_max - expected_hit).abs() <= 1);
            assert!((out.physical_avg_max - expected_hit * 3).abs() <= 1);
            assert_eq!(out.projectile_count, 3);
        }
    }

    #[test]
    fn weapon_bonus_scaling_preserves_the_tag_bonus_range() {
        let mut s = skill();
        s.tags.push("Orbital".into());
        s.attack_scaling.as_mut().unwrap().weapon_bonus_scaling = true;
        s.attack_scaling.as_mut().unwrap().weapon_damage_pct = Some(DamageFormula {
            base: 190.0,
            per_level: 0.0,
        });
        let shared = HashMap::from([("orbital_skill_damage".into(), (25.0, 100.0))]);
        let out = breakdown_for(&s, &shared, &StatMap::new(), &SkillRanks::new(), 1, 0.0);
        assert_eq!(out.physical_hit_min, 212); // 100 × (1 + 0.9 × 1.25).
        assert_eq!(out.physical_hit_max, 280); // 100 × (1 + 0.9 × 2).
    }
    #[test]
    fn physical_of_total_damage_applies_signed_adjustment_before_projectiles() {
        for (pct, expected_hit) in [
            (0.0, 100),
            (100.0, 200),
            (-50.0, 50),
            (-100.0, 0),
            (-150.0, 0),
        ] {
            let scoped = stats(&[("of_total_damage", pct)]);
            let result = breakdown_for(
                &skill(),
                &StatMap::new(),
                &scoped,
                &SkillRanks::new(),
                3,
                0.0,
            );
            assert_eq!(result.physical_hit_max, expected_hit, "subtree {pct}%");
            assert_eq!(result.combined_avg_max, expected_hit * 3, "subtree {pct}%");
        }
    }

    #[test]
    fn ranged_double_damage_changes_each_average_endpoint_independently() {
        let shared = StatMap::from([("double_damage_chance".into(), (0.0, 100.0))]);
        let result = breakdown_for(
            &skill(),
            &shared,
            &StatMap::new(),
            &SkillRanks::new(),
            3,
            0.0,
        );
        assert_eq!(
            (result.physical_hit_min, result.physical_hit_max),
            (100, 100)
        );
        assert_eq!(
            (result.physical_avg_min, result.physical_avg_max),
            (300, 600)
        );
        assert_eq!(result.projectile_count, 3);
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
    fn hybrid_of_total_damage_applies_once_to_each_damage_member() {
        let mut skill = skill();
        skill.damage_type = Some("fire".into());
        skill.damage_formula = Some(DamageFormula {
            base: 100.0,
            per_level: 0.0,
        });
        let empty = StatMap::new();
        let ranks = SkillRanks::new();
        let by_name = HashMap::new();
        let conds = ConditionMap::new();
        let weapon = Weapon {
            name: "test".into(),
            damage_min: 100.0,
            damage_max: 100.0,
        };
        for (pct, expected_physical, expected_elemental) in [
            (0.0, 100, 100),
            (100.0, 200, 200),
            (-50.0, 50, 50),
            (-100.0, 0, 0),
            (-150.0, 0, 0),
        ] {
            let element =
                crate::calc::skills::compute_skill_damage(&crate::calc::skills::SkillInput {
                    skill: &skill,
                    allocated_rank: 1.0,
                    attributes: &empty,
                    stats: &empty,
                    skill_ranks_by_name: &ranks,
                    item_skill_bonuses: &empty,
                    enemy_conditions: &conds,
                    enemy_resistances: &HashMap::new(),
                    skills_by_name: &by_name,
                    projectile_count: 3,
                    of_total_damage: pct,
                    scoped: &empty,
                    conversion_flat: 0.0,
                    conversion_skill_damage_pct: 0.0,
                })
                .unwrap();
            let result = compute_attack_skill_damage(&AttackSkillInput {
                action_rate_override: Some((1.0, 1.0)),
                skill: &skill,
                allocated_rank: 1.0,
                attributes: &empty,
                stats: &empty,
                skill_ranks_by_name: &ranks,
                skills_by_name: &by_name,
                item_skill_bonuses: &empty,
                enemy_conditions: &conds,
                enemy_resistances: &HashMap::new(),
                weapon: Some(&weapon),
                poison_breakdown: Some(&element),
                scoped: &empty,
                projectile_count: 3,
                of_total_damage: pct,
                conversion_flat: 0.0,
            })
            .unwrap();
            assert_eq!(result.physical_hit_max, expected_physical, "subtree {pct}%");
            assert_eq!(result.poison_hit_max, expected_elemental, "subtree {pct}%");
            assert_eq!(
                result.combined_avg_max,
                (expected_physical + expected_elemental) * 3,
                "subtree {pct}%"
            );
            assert_eq!(result.dps_max, result.combined_avg_max as f64);
        }
    }

    #[test]
    fn explanation_includes_weapon_more_deadly_and_projectile_averaging() {
        let d = breakdown_for(
            &skill(),
            &stats(&[
                ("enhanced_damage_more", 100.0),
                ("attack_damage_more", 50.0),
                ("deadly_blow_chance", 100.0),
                ("deadly_blow_effectiveness", 50.0),
                ("crit_chance", 50.0),
                ("crit_damage", 100.0),
                ("attacks_per_second", 2.0),
            ]),
            &StatMap::new(),
            &SkillRanks::new(),
            3,
            40.0,
        );
        let step = |label| {
            d.calculation()
                .iter()
                .find(|step| step.label() == label)
                .unwrap()
        };
        assert_eq!(step("Physical base").value(), (240.0, 240.0));
        assert_eq!(step("Attack damage multiplier").value(), (1.5, 1.5));
        assert!((step("Deadly blow multiplier").value().0 - 2.025).abs() < 1e-12);
        assert_eq!(
            step("Average damage per swing").value(),
            (d.combined_avg_min as f64, d.combined_avg_max as f64)
        );
        assert_eq!(
            step("Attack DPS before entities / repeated hits").value(),
            (d.dps_min, d.dps_max)
        );
    }
}
