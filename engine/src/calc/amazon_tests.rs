use super::*;

#[test]
fn amazon_neutral_elemental_hits_match_executable_coefficients() {
    // Supplied YYC: Amazon constructor + inherited zero intercept from s_TalentInfo.
    // Independently fixed neutral outputs at ranks 1/10/20; LoadTalentDamage ceil.
    for (id, expected) in [
        ("noxious_strike", [25, 151, 291]),
        ("caustic_spearhead", [13, 125, 250]),
        ("leaping_ambush", [28, 280, 560]),
        ("death_from_above", [22, 220, 440]),
        ("toxic_remains", [16, 155, 310]),
        ("envenom", [24, 240, 480]),
        ("astropes_gift", [18, 90, 170]),
        ("rebound", [8, 75, 149]),
        ("spearnage", [15, 150, 300]),
        ("storm_dash", [13, 125, 250]),
        ("thunder_fury", [13, 125, 250]),
    ] {
        let spec = data::get_skills_by_class("amazon")
            .iter()
            .find(|s| s.id == id)
            .unwrap();
        let skill = skill_spec_to_calc_skill(spec);
        for (rank, expected) in [1.0, 10.0, 20.0].into_iter().zip(expected) {
            let hit = compute_skill_damage(&SkillInput {
                skill: &skill,
                allocated_rank: rank,
                attributes: &HashMap::new(),
                stats: &HashMap::new(),
                skill_ranks_by_name: &HashMap::new(),
                skills_by_name: &HashMap::new(),
                item_skill_bonuses: &HashMap::new(),
                enemy_conditions: &HashMap::new(),
                enemy_resistances: &HashMap::new(),
                scoped: &HashMap::new(),
                projectile_count: 1,
                of_total_damage: 0.0,
                conversion_flat: 0.0,
                conversion_skill_damage_pct: 0.0,
            })
            .unwrap();
            assert_eq!(
                (hit.hit_min, hit.hit_max),
                (expected, expected),
                "{id} rank{rank}"
            );
        }
    }
}

#[test]
fn amazon_transformations_retain_damage_instead_of_subtracting_their_percentage() {
    for (skill, node, pct, max_rank) in [
        ("leaping_ambush", "master_of_agility", 30.0, 3),
        ("storm_dash", "agility_over_power", 15.0, 5),
    ] {
        let base = perf("amazon", skill, 10, &[], &[]).damage.unwrap().hit_max;
        for rank in 1..=max_rank {
            let changed = perf("amazon", skill, 10, &[(node, rank)], &[]);
            assert_eq!(
                changed.damage.unwrap().hit_max,
                (base as f64 * pct * rank as f64 / 100.0).floor() as i64,
                "{skill} r{rank}"
            );
        }
    }
}

#[test]
fn amazon_secondary_payloads_do_not_inflate_primary_hits() {
    for (skill, node) in [
        ("caustic_spearhead", "venomous_touch"),
        ("caustic_spearhead", "venomous_burst"),
        ("rebound", "rebomb"),
        ("rebound", "kickback_projectile"),
        ("astropes_gift", "thundering_storm"),
        ("leaping_ambush", "arc_of_spears"),
    ] {
        let base = perf("amazon", skill, 10, &[], &[]);
        let changed = perf("amazon", skill, 10, &[(node, 1)], &[]);
        assert_eq!(
            base.damage.as_ref().unwrap().hit_max,
            changed.damage.as_ref().unwrap().hit_max,
            "{skill}:{node}"
        );
        assert!(changed
            .calculation()
            .iter()
            .any(|s| s.label().starts_with("Model note ·")));
    }
}

#[test]
fn amazon_storm_orbs_use_cooldown_and_not_unconditional_extra_hits() {
    let result = perf("amazon", "storm_dash", 10, &[("storm_orbs", 1)], &[]);
    assert_eq!(result.damage.as_ref().unwrap().projectile_count, 1);
    assert!((result.skill_costs["storm_dash"].cast_rate_max.unwrap() - 1.0 / 2.25).abs() < 1e-9);
}

#[test]
fn amazon_death_waves_are_separate_from_projectiles_and_rehit_interval() {
    let base = perf("amazon", "death_from_above", 10, &[], &[]);
    let waves = perf(
        "amazon",
        "death_from_above",
        10,
        &[("storm_of_spears", 2)],
        &[],
    );
    assert_eq!(waves.damage.as_ref().unwrap().projectile_count, 1);
    assert_eq!(waves.hits_per_cast, Some((9.0, 9.0)));
    assert!((waves.avg_hit_dps_max.unwrap() / base.avg_hit_dps_max.unwrap() - 9.0).abs() < 1e-9);
}

#[test]
fn amazon_powerstasis_doubles_only_expected_elemental_damage() {
    let base = perf("amazon", "astropes_gift", 10, &[], &[]);
    let changed = perf("amazon", "astropes_gift", 10, &[("powerstasis", 5)], &[]);
    let a = base.attack_damage.unwrap();
    let b = changed.attack_damage.unwrap();
    assert_eq!(a.physical_avg_max, b.physical_avg_max);
    assert_eq!(a.poison_hit_max, b.poison_hit_max);
    assert!((b.poison_avg_max as f64 - a.poison_avg_max as f64 * 1.25).abs() <= 1.0);
}

#[test]
fn amazon_weapon_requirements_do_not_grant_bonuses_when_unarmed() {
    for (skill, node) in [
        ("noxious_strike", "corrosive_reach"),
        ("rebound", "marksmanship"),
        ("astropes_gift", "master_of_javelin"),
    ] {
        let base = perf("amazon", skill, 10, &[], &[]);
        let changed = perf("amazon", skill, 10, &[(node, 3)], &[]);
        assert_eq!(base.avg_hit_dps_max, changed.avg_hit_dps_max, "{skill}");
    }
}

#[test]
fn amazon_envenova_keeps_its_retained_damage_in_own_proc_scope() {
    let spec = data::get_skills_by_class("amazon")
        .iter()
        .find(|s| s.id == "envenom")
        .unwrap();
    let owner = crate::calc::stats::skill_spec_to_subskill_owner(spec);
    let ranks = HashMap::from([(subskill_key("envenom", "envenova"), 2)]);
    let agg = crate::calc::subskill::aggregate_subskill_stats(&owner, &ranks, None);
    assert_eq!(agg.stats.get("retained_damage_percent"), Some(&70.0));
    assert!(!agg.stats.contains_key("of_total_damage"));
    let scoped = agg.stats.into_iter().map(|(k, v)| (k, (v, v))).collect();
    let skill = skill_spec_to_calc_skill(spec);
    let damage = compute_skill_damage(&SkillInput {
        skill: &skill,
        allocated_rank: 10.0,
        attributes: &HashMap::new(),
        stats: &HashMap::new(),
        skill_ranks_by_name: &HashMap::new(),
        skills_by_name: &HashMap::new(),
        item_skill_bonuses: &HashMap::new(),
        enemy_conditions: &HashMap::new(),
        enemy_resistances: &HashMap::new(),
        scoped: &scoped,
        projectile_count: 1,
        of_total_damage: 0.0,
        conversion_flat: 0.0,
        conversion_skill_damage_pct: 0.0,
    })
    .unwrap();
    assert_eq!(damage.hit_max, 168); // 24*10*0.70, not 24*10*1.70.
}

#[test]
fn amazon_speed_conversion_is_own_multiplier_not_flat_damage() {
    let result = perf_with_stats(
        "amazon",
        "death_from_above",
        10,
        &[("need_for_speed", 2)],
        &[("movement_speed", "100")],
        1.0,
    );
    let d = result.damage.unwrap();
    let generic = d
        .calculation()
        .iter()
        .find(|s| s.label() == "Damage after generic helper rounding")
        .unwrap()
        .value()
        .1;
    let speed = result.stats["movement_speed"].1;
    assert_eq!(
        d.hit_max,
        (generic * (1.0 + speed * 0.20 / 100.0)).floor() as i64
    );
}

#[test]
fn amazon_third_strike_bonus_changes_average_but_not_every_hit() {
    let base = perf("amazon", "noxious_strike", 10, &[], &[]);
    let third = perf("amazon", "noxious_strike", 10, &[("noxian_slash", 5)], &[]);
    let a = base.attack_damage.unwrap();
    let b = third.attack_damage.unwrap();
    assert_eq!(
        (a.physical_hit_max, a.poison_hit_max),
        (b.physical_hit_max, b.poison_hit_max)
    );
    assert!(b.physical_avg_max > a.physical_avg_max);
    assert!(b.poison_avg_max > a.poison_avg_max);
    assert_eq!(a.projectile_count, b.projectile_count);
}
