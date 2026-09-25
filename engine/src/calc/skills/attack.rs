use super::calculation::{number, range, scalar, stat_inputs, CalculationStep};
use std::collections::HashMap;

use super::{
    collect_extra_damage, damage::bonus_source_synergy_pct, damage::element_keys, deadly_blow_mult,
    r_max, r_min, rg, AttackSkillDamageBreakdown, AttrMap, ConditionMap, DamageFormula,
    ExtraSource, ItemSkillBonuses, Skill, SkillDamageBreakdown, SkillRanks, StatMap, Weapon,
    CRUSHING_BLOW_DEFAULT,
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

fn converted_attack_elements(
    input: &AttackSkillInput<'_>,
    hit: (f64, f64),
    average: (f64, f64),
) -> (Vec<super::ConvertedAttackDamage>, Vec<CalculationStep>) {
    let mut elements = Vec::new();
    let mut calculation = Vec::new();
    for element in super::ELEMENTS {
        let key = format!("physical_to_{element}");
        let shared = rg(input.stats, &key);
        let scoped = rg(input.scoped, &key);
        let pct = (
            (shared.0 + scoped.0).max(0.0),
            (shared.1 + scoped.1).max(0.0),
        );
        if pct == (0.0, 0.0) {
            continue;
        }
        let keys = element_keys(element).expect("element keys");
        // The copied physical hit already includes generic/tag/skill bonuses.
        // Only the destination's elemental percentages are added here; neither
        // flat spell damage nor another application of the skill coefficient.
        let element_damage = rg(input.stats, keys.skill_damage);
        let magic_damage = rg(input.stats, "magic_skill_damage");
        let element_more = rg(input.stats, keys.skill_damage_more);
        let magic_more = rg(input.stats, "magic_skill_damage_more");
        let element_multiplier = (
            (1.0 + (element_damage.0 + magic_damage.0) / 100.0).max(0.0)
                * (1.0 + element_more.0 / 100.0).max(0.0)
                * (1.0 + magic_more.0 / 100.0).max(0.0),
            (1.0 + (element_damage.1 + magic_damage.1) / 100.0).max(0.0)
                * (1.0 + element_more.1 / 100.0).max(0.0)
                * (1.0 + magic_more.1 / 100.0).max(0.0),
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
        let target_mult =
            resistance_mult * (1.0 + (elemental_break + element_break) / 100.0);
        let (critical_break_average, critical_break_step) = super::critical_break_average_multiplier(
            element, element_break, elemental_break, input.stats, input.scoped,
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
                key.as_str(),
                keys.skill_damage,
                keys.skill_damage_more,
                "magic_skill_damage",
                "magic_skill_damage_more",
                keys.ignore_res,
                "elemental_break",
                "elemental_break_on_strike",
                break_key.as_str(),
            ],
        );
        calculation.push(CalculationStep::new(
            format!("Physical converted to {element}"),
            format!("floor({} physical skill hit × {}% / 100 × {} elemental damage multiplier × {} resistance × [1 + ({}% Elemental Break + {}% {element} Break) / 100]); physical damage is preserved",
                range(hit), range(pct), range(element_multiplier), number(resistance_mult),
                number(elemental_break), number(element_break)),
            (component.hit_min as f64, component.hit_max as f64),
        ));
        calculation.push(CalculationStep::new(
            format!("Average converted {element} damage"),
            "The physical average already includes critical chance, double damage and projectile count; apply conversion, target modifiers and the independent Critical Break adjustment once",
            (component.avg_min as f64, component.avg_max as f64),
        ));
        calculation.extend(critical_break_step);
        elements.push(component);
    }
    (elements, calculation)
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
    let tag_bonus = crate::calc::rank::tag_skills_sum(input.stats, &s.tags);
    let (eff_min, eff_max) = crate::calc::rank::effective_rank_range_for(
        s,
        input.allocated_rank,
        input.stats,
        input.item_skill_bonuses,
    );

    let skill_wdp_min = formula_at_clamped_opt(scaling.weapon_damage_pct.as_ref(), eff_min);
    let skill_wdp_max = formula_at_clamped_opt(scaling.weapon_damage_pct.as_ref(), eff_max);
    let skill_flat_min = formula_at_clamped_opt(scaling.flat_physical_min.as_ref(), eff_min);
    let skill_flat_max = formula_at_clamped_opt(scaling.flat_physical_max.as_ref(), eff_max);
    let rating_bonus = total_pct(input, "attack_rating_pct");
    let skill_arp_min =
        formula_at_clamped_opt(scaling.attack_rating_pct.as_ref(), eff_min) + rating_bonus;
    let skill_arp_max =
        formula_at_clamped_opt(scaling.attack_rating_pct.as_ref(), eff_max) + rating_bonus;

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

    let (mut extra_mult, mut extra_sources) =
        collect_extra_damage(input.stats, input.enemy_conditions, Some("physical"));
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

    let crit = super::crit_factors_with_tags(input.stats, false, &s.tags);
    // LoadTalentDamage's physical weapon-coefficient branch ignores its flat
    // pool, including Leap stat437. See ../leap-evidence.md. Other tagged flat
    // sources retain their existing behavior pending their own source audit.
    let tagged_flat = crate::calc::affix_tags::keys_for(
        crate::calc::types::AffixEffect::FlatDamage,
        &s.tags,
    )
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
    // Amazon's own subtree scales the added weapon coefficient, before the
    // neutral weapon hit returns. Element-only bonuses never enter this pool.
    let subtree = rg(input.scoped, "subtree_damage");
    let weapon_subtree = rg(input.scoped, "weapon_subtree_damage");
    let subtree_factor = (
        (1.0 + (subtree.0 + weapon_subtree.0) / 100.0).max(0.0),
        (1.0 + (subtree.1 + weapon_subtree.1) / 100.0).max(0.0),
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
    let (skill_mult_min, skill_mult_max) = if weapon_bonus_scaling {
        (
            (1.0 + (skill_wdp_min - 100.0 + synergy_min) * tagged_factor.0 * subtree_factor.0
                / 100.0)
                .max(0.0),
            (1.0 + (skill_wdp_max - 100.0 + synergy_max) * tagged_factor.1 * subtree_factor.1
                / 100.0)
                .max(0.0),
        )
    } else if has_wdp {
        (skill_wdp_min / 100.0, skill_wdp_max / 100.0)
    } else {
        (1.0, 1.0)
    };
    let third_attack = rg(input.scoped, "third_attack_subtree_damage");
    let expected_skill_mult = if weapon_bonus_scaling {
        (
            (1.0 + (skill_wdp_min - 100.0 + synergy_min)
                * tagged_factor.0
                * (subtree_factor.0 + third_attack.0 / 300.0)
                / 100.0)
                .max(0.0),
            (1.0 + (skill_wdp_max - 100.0 + synergy_max)
                * tagged_factor.1
                * (subtree_factor.1 + third_attack.1 / 300.0)
                / 100.0)
                .max(0.0),
        )
    } else {
        (skill_mult_min, skill_mult_max)
    };

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
    let phys_avg_min = physical_expected.0
        * crit.avg_mult
        * double_damage.0
        * (projectiles as f64 + extra_projectiles.0.max(0.0));
    let phys_avg_max = physical_expected.1
        * crit.avg_mult
        * double_damage.1
        * (projectiles as f64 + extra_projectiles.1.max(0.0));

    let (poison_hit_min, poison_hit_max, poison_avg_min, poison_avg_max) = input
        .poison_breakdown
        .map(|p| (p.hit_min, p.hit_max, p.avg_min, p.avg_max))
        .unwrap_or((0, 0, 0, 0));

    let phys_hit_min_i = phys_hit_min.floor() as i64;
    let phys_hit_max_i = phys_hit_max.floor() as i64;
    let phys_avg_min_i = phys_avg_min.floor() as i64;
    let phys_avg_max_i = phys_avg_max.floor() as i64;

    let (converted_elements, conversion_steps) = converted_attack_elements(
        input,
        (phys_hit_min, phys_hit_max),
        (phys_avg_min, phys_avg_max),
    );
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

    let ias = rg(input.stats, "increased_attack_speed");
    let ias_more = rg(input.stats, "increased_attack_speed_more");
    let base_aps = r_max(rg(input.stats, "attacks_per_second"));
    let aps_min = base_aps * (1.0 + r_min(ias) / 100.0) * (1.0 + r_min(ias_more) / 100.0);
    let aps_max = base_aps * (1.0 + r_max(ias) / 100.0) * (1.0 + r_max(ias_more) / 100.0);

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
        format!(
            "{} allocated + {} all skills + {} element + {} tagged + {} item ranks",
            number(input.allocated_rank),
            range(all_skills),
            range(elem_bonus),
            range(tag_bonus),
            range(item)
        ),
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
            "crushing_blow_modifier",
            "armor_break",
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
    if s.tags.iter().any(|tag| tag == "Leap")
        && rg(input.stats, "flat_leap_damage") != (0.0, 0.0)
    {
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
        calculation.push(CalculationStep::new("Attack rating bonus %", "Skill rank formula + shared and subtree attack-rating bonuses; accuracy is not inferred without enemy evasion", (skill_arp_min, skill_arp_max)));
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
    calculation.push(CalculationStep::new(
        "Skill weapon multiplier",
        if weapon_bonus_scaling {
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
            "{} crushing (default {}) + {}% armor break / 100",
            number(crushing_blow_modifier),
            number(CRUSHING_BLOW_DEFAULT),
            number(armor_break_pct)
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
            "{} base × {} attack × {} skill × {} crush/armor × {} deadly × {} extra",
            range((base_min, base_max)),
            range((atk_mult_min, atk_mult_max)),
            range((skill_mult_min, skill_mult_max)),
            number(crush_armor_mult),
            number(deadly_mult),
            number(extra_mult)
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
        format!(
            "(1 + {}% critical damage / 100) × (1 + {}% more / 100)",
            number(crit.damage_pct),
            number(r_max(rg(input.stats, "crit_damage_more")))
        ),
        scalar(crit.on_crit_mult),
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
    calculation.push(CalculationStep::new("Double damage expectation", "1 + clamp(double damage chance, 0, 100) / 100; one contact, independent of critical and extra damage", double_damage));
    if third_attack != (0.0, 0.0) {
        calculation.push(CalculationStep::new("Expected weapon multiplier", format!("Own subtree gains {}% / 3 on average from the third attack; neutral weapon damage stays outside the bonus", range(third_attack)), expected_skill_mult));
    }
    calculation.push(CalculationStep::new(
        "Average physical damage",
        format!(
            "floor({} expected physical hit × {} average crit × {} double damage expectation × ({} ordinary + {} expected additional projectiles))",
            range(physical_expected),
            number(crit.avg_mult),
            range(double_damage),
            projectiles,
            range(extra_projectiles)
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
                "{} base × (1 + {}% increased / 100) × (1 + {}% more / 100)",
                number(base_aps),
                range(ias),
                range(ias_more)
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
    fn conversion_uses_the_skill_hit_and_destination_bonus_without_consuming_physical() {
        let mut s = skill();
        s.attack_scaling
            .as_mut()
            .unwrap()
            .weapon_damage_pct
            .as_mut()
            .unwrap()
            .base = 300.0;
        let shared = stats(&[
            ("crushing_blow_modifier", 1.0),
            ("physical_to_fire", 10.0),
            ("fire_skill_damage", 100.0),
            ("flat_fire_skill_damage", 99999.0),
        ]);
        let d = breakdown_for(&s, &shared, &StatMap::new(), &SkillRanks::new(), 1, 0.0);
        assert_eq!(d.physical_hit_max, 300);
        assert_eq!(d.converted_elements[0].hit_max, 60);
        assert_eq!(d.combined_hit_max, 360);
    }

    #[test]
    fn conversions_above_one_hundred_percent_stack_and_average_only_once() {
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
        assert_eq!(d.physical_hit_max, base.physical_hit_max);
        assert_eq!(d.physical_avg_max, base.physical_avg_max);
        assert_eq!(d.combined_hit_min, base.combined_hit_min * 3);
        assert_eq!(d.combined_hit_max, base.combined_hit_max * 4);
        assert_eq!(d.combined_avg_min, base.combined_avg_min * 3);
        assert_eq!(d.combined_avg_max, base.combined_avg_max * 4);
    }

    #[test]
    fn each_converted_element_uses_its_own_bonuses_resistance_and_break() {
        let s = skill();
        let empty = StatMap::new();
        let ranks = SkillRanks::new();
        let skills = HashMap::new();
        let shared = stats(&[
            ("physical_to_fire", 10.0),
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
        let (components, _) = converted_attack_elements(&input, (300.0, 300.0), (900.0, 900.0));
        let fire = components.iter().find(|c| c.element == "fire").unwrap();
        // 300 × 10% × (1 + 50% fire + 50% magic) × 2 more × .75 res × (1 + 1 + 1 Break).
        assert_eq!(fire.hit_max, 270);
        assert_eq!(fire.avg_max, 810);
        let mut critical_stats = shared.clone();
        critical_stats.insert("fire_break_crit_chance".into(), (15.0, 100.0));
        critical_stats.insert("fire_break_crit_damage".into(), (25.0, 25.0));
        let (critical, _) = converted_attack_elements(
            &AttackSkillInput { stats: &critical_stats, ..input },
            (300.0, 300.0), (900.0, 900.0),
        );
        let critical_fire = critical.iter().find(|c| c.element == "fire").unwrap();
        assert_eq!((critical_fire.hit_min, critical_fire.hit_max), (270, 270));
        assert_eq!((critical_fire.avg_min, critical_fire.avg_max), (820, 877));
        assert_eq!(
            components
                .iter()
                .find(|c| c.element == "cold")
                .unwrap()
                .hit_max,
            0
        );
    }

    #[test]
    fn weapon_damage_bounds_follow_enhanced_damage_and_feed_conversion_once() {
        let shared = stats(&[
            ("enhanced_damage", 100.0),
            ("minimum_damage_flat", 8.0),
            ("maximum_damage_flat", 32.0),
            ("minimum_damage_pct", 50.0),
            ("maximum_damage_pct", 100.0),
            ("crushing_blow_modifier", 1.0),
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
        assert_eq!(d.physical_hit_min, 312); // (100 * 2 + 8) * 1.5
        assert_eq!(d.physical_hit_max, 464); // (100 * 2 + 32) * 2
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
        // 100 weapon × 1.5 crushing × (1 + (40 + 10)/100) × 1.2.
        assert_eq!(result.physical_hit_max, 270);
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
    fn crushing_blow_default_matches_the_weapon_panel() {
        // weapon 100 * crushing 1.5 = 150, the same stage weapon.rs applies.
        assert_eq!(hit_max(&StatMap::new(), &StatMap::new()), 150);
    }

    #[test]
    fn armor_break_raises_attack_skill_damage() {
        // crush_armor_mult = 1.5 + 100/100 = 2.5
        assert_eq!(
            hit_max(&stats(&[("armor_break", 100.0)]), &StatMap::new()),
            250
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
        // Rank one: weapon 100 * (1 + 1.20 * (1 + .08 + .04)) * crush 1.5.
        // The poison-only bonus must not change this physical coefficient.
        let scoped = stats(&[
            ("subtree_damage", 8.0),
            ("weapon_subtree_damage", 4.0),
            ("elemental_subtree_damage", 500.0),
        ]);
        let out = breakdown_for(&s, &StatMap::new(), &scoped, &SkillRanks::new(), 1, 0.0);
        assert_eq!(out.physical_hit_max, 351);
        let extra = stats(&[("expected_additional_projectiles", 0.25)]);
        let out = breakdown_for(&s, &StatMap::new(), &extra, &SkillRanks::new(), 1, 0.0);
        assert_eq!(out.physical_hit_max, 330);
        assert_eq!(out.physical_avg_max, 412);
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
            (stats(&[]), 0.0, 1.9, 285),
            (stats(&[("orbital_skill_damage", 100.0)]), 0.0, 2.8, 420),
            (stats(&[]), 10.0, 2.2, 330),
            (stats(&[("orbital_skill_damage", 100.0)]), 10.0, 3.4, 510),
            (
                stats(&[
                    ("orbital_skill_damage", 100.0),
                    ("physical_skill_damage", 50.0),
                ]),
                0.0,
                2.8,
                630,
            ),
            (stats(&[("attack_damage", 100.0)]), 10.0, 2.2, 660),
            (
                stats(&[("orbital_skill_damage_more", 100.0)]),
                0.0,
                2.8,
                420,
            ),
            (stats(&[("spell_damage", 100.0)]), 0.0, 1.9, 285),
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
            // The usual crushing factor is 1.5; three projectiles change only
            // the volley average, never this coefficient or individual hit.
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
        assert_eq!(out.physical_hit_min, 318); // 100 × (1 + 0.9 × 1.25) × 1.5.
        assert_eq!(out.physical_hit_max, 420); // 100 × (1 + 0.9 × 2) × 1.5.
    }
    #[test]
    fn physical_of_total_damage_applies_signed_adjustment_before_projectiles() {
        for (pct, expected_hit) in [
            (0.0, 150),
            (100.0, 300),
            (-50.0, 75),
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
            (150, 150)
        );
        assert_eq!(
            (result.physical_avg_min, result.physical_avg_max),
            (450, 900)
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
            (0.0, 150, 100),
            (100.0, 300, 200),
            (-50.0, 75, 50),
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
