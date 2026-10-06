use super::super::*;
use crate::calc::types::{EquippedAffix, EquippedItem};

#[test]
fn fuel_to_fire_consumed_count_scales_only_its_active_learned_payload() {
    for count in [None, Some(0), Some(5), Some(u32::MAX)] {
        for form in ["holy_form", "unholy_form"] {
            for learned in [false, true] {
                for active in [false, true] {
                    let ranks = HashMap::from([
                        ("fuel_to_fire".into(), u32::from(learned)),
                        (form.into(), 1),
                    ]);
                    let buffs =
                        HashMap::from([("fuel_to_fire".into(), active), (form.into(), true)]);
                    let counts = count
                        .map(|n| HashMap::from([("fuel_to_fire_consumed_objects".into(), n)]))
                        .unwrap_or_default();
                    let mut attrs = SourceMap::new();
                    let mut stats = SourceMap::new();
                    for (key, value) in [("all_skills", (0.0, 2.0)), ("damage", (7.5, 7.5))] {
                        push_source(
                            &mut stats,
                            key,
                            SourceContribution {
                                label: "Other source".into(),
                                source_type: SourceType::Custom,
                                value,
                                forge: None,
                            },
                        );
                    }
                    apply_skill_ranks(
                        Some("butcher"),
                        &ranks,
                        None,
                        &buffs,
                        &counts,
                        &Inventory::new(),
                        &mut attrs,
                        &mut stats,
                    );
                    let fuel = stats
                        .get("damage")
                        .into_iter()
                        .flatten()
                        .filter(|s| s.label.starts_with("Fuel to Fire"))
                        .fold((0.0, 0.0), |sum, s| (sum.0 + s.value.0, sum.1 + s.value.1));
                    let factor = 1.0 + f64::from(count.unwrap_or(0)) * 0.01;
                    let expected = if learned && active {
                        (20.0 * factor, 23.0 * factor)
                    } else {
                        (0.0, 0.0)
                    };
                    assert!(
                        (fuel.0 - expected.0).abs() < 1e-6 && (fuel.1 - expected.1).abs() < 1e-6,
                        "count={count:?}, form={form}, learned={learned}, active={active}: {fuel:?} != {expected:?}"
                    );
                    let other = stats["damage"]
                        .iter()
                        .find(|s| s.label == "Other source")
                        .unwrap();
                    assert_eq!(other.value, (7.5, 7.5));
                }
            }
        }
    }
}

#[test]
fn enraged_mania_grants_size_and_attack_speed_without_global_area() {
    for learned in [false, true] {
        let ranks = HashMap::from([("enraged_mania".into(), u32::from(learned))]);
        let mut attrs = SourceMap::new();
        let mut stats = SourceMap::new();
        for (key, value) in [
            ("all_skills", (0.0, 2.0)),
            ("area_of_effect", (12.5, 12.5)),
            ("melee_projectile_size", (0.5, 0.75)),
        ] {
            push_source(
                &mut stats,
                key,
                SourceContribution {
                    label: "Other source".into(),
                    source_type: SourceType::Custom,
                    value,
                    forge: None,
                },
            );
        }
        apply_skill_ranks(
            Some("butcher"),
            &ranks,
            None,
            &HashMap::new(),
            &HashMap::new(),
            &Inventory::new(),
            &mut attrs,
            &mut stats,
        );
        let totals = compute_final_stats(&stats);
        assert_eq!(totals["area_of_effect"], (12.5, 12.5));
        assert_eq!(totals["melee_projectile_size"], (0.5, 0.75));
        assert_eq!(
            totals
                .get("melee_attack_size_offset")
                .copied()
                .unwrap_or_default(),
            if learned { (8.0, 10.0) } else { (0.0, 0.0) }
        );
        assert_eq!(
            totals
                .get("increased_attack_speed")
                .copied()
                .unwrap_or_default(),
            if learned { (0.5, 1.5) } else { (0.0, 0.0) }
        );
    }
}

#[test]
fn melee_attack_size_offset_floors_aggregate_without_rounding_other_size_stats() {
    for (skill, item, custom, expected) in [
        ((8.0, 10.0), (0.75, 1.5), (0.5, 0.75), (9.0, 12.0)),
        ((0.0, 0.0), (-1.75, -0.25), (0.5, 0.5), (-2.0, 0.0)),
    ] {
        let mut sources = SourceMap::new();
        for (source_type, value) in [
            (SourceType::Skill, skill),
            (SourceType::Item, item),
            (SourceType::Custom, custom),
        ] {
            push_source(
                &mut sources,
                "melee_attack_size_offset",
                SourceContribution {
                    label: "Test contribution".into(),
                    source_type,
                    value,
                    forge: None,
                },
            );
        }
        let raw_total = sum_contributions(&sources["melee_attack_size_offset"]);
        let mut stats = compute_final_stats(&sources);
        stats.insert("area_of_effect".into(), (12.5, 16.75));
        stats.insert("melee_projectile_size".into(), (0.5, 0.75));
        apply_multipliers_pass(&mut stats);
        finalize_melee_attack_size_offset(&mut stats);
        assert_eq!(stats["melee_attack_size_offset"], expected);
        assert_eq!(stats["area_of_effect"], (12.5, 16.75));
        assert_eq!(stats["melee_projectile_size"], (0.5, 0.75));
        assert_eq!(
            sum_contributions(&sources["melee_attack_size_offset"]),
            raw_total
        );
    }
}

#[test]
fn bleeding_global_floor_preserves_later_legacy_subskill_fractions() {
    let mut sources = SourceMap::new();
    for (source_type, value) in [
        (SourceType::Skill, (1.25, 3.75)),
        (SourceType::Item, (0.75, 0.5)),
        (SourceType::Subskill, (17.5, 19.25)),
    ] {
        push_source(
            &mut sources,
            "extra_damage_bleeding",
            SourceContribution {
                label: "Test contribution".into(),
                source_type,
                value,
                forge: None,
            },
        );
    }
    let mut stats = compute_final_stats(&sources);
    apply_multipliers_pass(&mut stats);
    finalize_extra_damage_bleeding(&sources, &mut stats);
    assert_eq!(stats["extra_damage_bleeding"], (19.5, 23.25));
    assert_eq!(
        sum_contributions(&sources["extra_damage_bleeding"]),
        (19.5, 23.5)
    );
}

#[test]
fn awakening_fury_requires_learned_active_buff_and_ignores_forms() {
    for form in [
        None,
        Some("holy_form"),
        Some("unholy_form"),
        Some("insatiable_hunger"),
    ] {
        for learned in [false, true] {
            for active in [false, true] {
                let mut ranks = HashMap::from([("awakening_fury".into(), u32::from(learned))]);
                let mut buffs = HashMap::from([("awakening_fury".into(), active)]);
                if let Some(form) = form {
                    ranks.insert(form.into(), 1);
                    buffs.insert(form.into(), true);
                    if form == "insatiable_hunger" {
                        ranks.insert("unholy_form".into(), 1);
                    }
                }
                let mut attrs = SourceMap::new();
                let mut stats = SourceMap::new();
                for (key, value) in [("all_skills", (0.0, 2.0)), ("attack_damage", (7.5, 7.5))] {
                    push_source(
                        &mut stats,
                        key,
                        SourceContribution {
                            label: "Test base".into(),
                            source_type: SourceType::Custom,
                            value,
                            forge: None,
                        },
                    );
                }
                apply_skill_ranks(
                    Some("butcher"),
                    &ranks,
                    None,
                    &buffs,
                    &HashMap::new(),
                    &Inventory::new(),
                    &mut attrs,
                    &mut stats,
                );
                let fury = stats
                    .values()
                    .flatten()
                    .filter(|source| source.label.starts_with("Awakening Fury"))
                    .fold((0.0, 0.0), |sum, source| {
                        (sum.0 + source.value.0, sum.1 + source.value.1)
                    });
                let expected = if learned && active {
                    (20.0, 34.0)
                } else {
                    (0.0, 0.0)
                };
                assert_eq!(
                    fury, expected,
                    "{form:?}, learned={learned}, active={active}"
                );
                assert_eq!(
                    compute_final_stats(&stats)["attack_damage"],
                    (7.5 + expected.0, 7.5 + expected.1)
                );
            }
        }
    }
}

#[test]
fn audited_butcher_talents_return_zero_at_nonpositive_effective_rank() {
    for (id, key, rank_one) in [
        ("ending_fate", "life_replenish_more", 50.0),
        ("awakening_fury", "attack_damage", 20.0),
        ("hunger_for_blood", "extra_damage_bleeding", 1.25),
        ("fuel_to_fire", "damage", 20.0),
        ("enraged_mania", "melee_attack_size_offset", 8.0),
        ("enraged_mania", "increased_attack_speed", 0.5),
    ] {
        for (bonus, expected) in [((-1.0, -1.0), (0.0, 0.0)), ((-2.0, 0.0), (0.0, rank_one))] {
            let ranks = HashMap::from([(id.into(), 1)]);
            let buffs = HashMap::from([(id.into(), true)]);
            let mut attrs = SourceMap::new();
            let mut stats = SourceMap::new();
            push_source(
                &mut stats,
                "all_skills",
                SourceContribution {
                    label: "Custom rank reduction".into(),
                    source_type: SourceType::Custom,
                    value: bonus,
                    forge: None,
                },
            );
            apply_skill_ranks(
                Some("butcher"),
                &ranks,
                None,
                &buffs,
                &HashMap::new(),
                &Inventory::new(),
                &mut attrs,
                &mut stats,
            );
            let totals = compute_final_stats(&stats);
            assert_eq!(
                totals.get(key).copied().unwrap_or((0.0, 0.0)),
                expected,
                "{id}, bonus={bonus:?}"
            );
        }
    }
}

#[test]
fn holy_form_healing_scales_existing_stats_in_the_more_pool() {
    let ranks = HashMap::from([("holy_form".into(), 10)]);
    let buffs = HashMap::from([("holy_form".into(), true)]);
    let mut attrs = SourceMap::new();
    let mut stats = SourceMap::new();
    for (key, value) in [
        ("all_skills", (0.0, 2.0)),
        ("life_replenish", (100.0, 100.0)),
        ("life_steal", (200.0, 200.0)),
    ] {
        stats.insert(
            key.into(),
            vec![SourceContribution {
                label: "Test base".into(),
                source_type: SourceType::Custom,
                value,
                forge: None,
            }],
        );
    }
    apply_skill_ranks(
        Some("butcher"),
        &ranks,
        None,
        &buffs,
        &HashMap::new(),
        &Inventory::new(),
        &mut attrs,
        &mut stats,
    );
    assert_eq!(
        sum_ranged_from_map(&stats, "life_replenish"),
        (102.0, 102.0)
    );
    assert_eq!(sum_ranged_from_map(&stats, "life_steal"), (200.0, 200.0));
    assert_eq!(
        sum_ranged_from_map(&stats, "life_replenish_more"),
        (92.0, 108.0)
    );
    assert_eq!(sum_ranged_from_map(&stats, "life_steal_more"), (65.0, 75.0));
    let mut final_stats = compute_final_stats(&stats);
    apply_multipliers_pass(&mut final_stats);
    let actual = final_stats["life_replenish"];
    assert!((actual.0 - 195.84).abs() < 1e-9);
    assert!((actual.1 - 212.16).abs() < 1e-9);
    assert_eq!(final_stats["life_steal"], (330.0, 350.0));
}

#[test]
fn insatiable_hunger_replaces_form_payloads_with_the_actual_runtime_selectors() {
    for ordinary_toggles in [false, true] {
        for holy_learned in [false, true] {
            for unholy_learned in [false, true] {
                let ranks = HashMap::from([
                    ("holy_form".into(), if holy_learned { 5 } else { 0 }),
                    ("unholy_form".into(), if unholy_learned { 7 } else { 0 }),
                    ("insatiable_hunger".into(), 3),
                ]);
                let buffs = HashMap::from([
                    ("holy_form".into(), ordinary_toggles),
                    ("unholy_form".into(), ordinary_toggles),
                    ("insatiable_hunger".into(), true),
                ]);
                let mut attrs = SourceMap::new();
                let mut stats = SourceMap::new();
                for (key, value) in [
                    ("all_skills", (0.0, 2.0)),
                    ("life_replenish", (100.0, 100.0)),
                    ("life_steal", (100.0, 100.0)),
                    ("life_steal_more", (20.0, 20.0)),
                ] {
                    stats.insert(
                        key.into(),
                        vec![SourceContribution {
                            label: "Test base".into(),
                            source_type: SourceType::Custom,
                            value,
                            forge: None,
                        }],
                    );
                }
                apply_skill_ranks(
                    Some("butcher"),
                    &ranks,
                    None,
                    &buffs,
                    &HashMap::new(),
                    &Inventory::new(),
                    &mut attrs,
                    &mut stats,
                );
                assert!(!stats.contains_key("effectiveness"));
                let exact_total =
                    |key: &str| sum_contributions(stats.get(key).map(Vec::as_slice).unwrap_or(&[]));
                assert_eq!(
                    exact_total("damage"),
                    if unholy_learned {
                        (29.435, 32.64)
                    } else if holy_learned && ordinary_toggles {
                        (-50.0, -50.0)
                    } else {
                        (0.0, 0.0)
                    }
                );
                assert_eq!(
                    exact_total("life_replenish_more"),
                    if holy_learned && unholy_learned {
                        (40.6, 51.0)
                    } else if holy_learned && ordinary_toggles {
                        (52.0, 68.0)
                    } else {
                        (0.0, 0.0)
                    }
                );
                assert_eq!(
                    exact_total("life_steal_more"),
                    if holy_learned && unholy_learned {
                        (-31.0, -30.75)
                    } else if holy_learned && ordinary_toggles {
                        (60.0, 70.0)
                    } else {
                        (20.0, 20.0)
                    }
                );
                let flat_regen = if unholy_learned || holy_learned && ordinary_toggles {
                    102.0
                } else {
                    100.0
                };
                assert_eq!(
                    sum_ranged_from_map(&stats, "life_replenish"),
                    (flat_regen, flat_regen)
                );
                if !unholy_learned {
                    assert!(stats
                        .values()
                        .flatten()
                        .all(|source| !source.label.starts_with("Insatiable Hunger")));
                }
                assert_eq!(sum_ranged_from_map(&stats, "life_steal"), (100.0, 100.0));
                let mut final_stats = compute_final_stats(&stats);
                apply_multipliers_pass(&mut final_stats);
                let expected = if holy_learned && unholy_learned {
                    (69.0, 69.25)
                } else if holy_learned && ordinary_toggles {
                    (160.0, 170.0)
                } else {
                    (120.0, 120.0)
                };
                let actual = final_stats["life_steal"];
                assert!((actual.0 - expected.0).abs() < 1e-9);
                assert!((actual.1 - expected.1).abs() < 1e-9);
                let actual = final_stats["life_replenish"];
                let expected = if holy_learned && unholy_learned {
                    (143.412, 154.02)
                } else if holy_learned && ordinary_toggles {
                    (155.04, 171.36)
                } else {
                    (flat_regen, flat_regen)
                };
                assert!((actual.0 - expected.0).abs() < 1e-9);
                assert!((actual.1 - expected.1).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn inactive_or_unlearned_hunger_preserves_the_ordinary_form() {
    for (hunger_rank, hunger_toggle) in [(3, false), (0, false), (0, true)] {
        let ranks = HashMap::from([
            ("holy_form".into(), 1),
            ("unholy_form".into(), 0),
            ("insatiable_hunger".into(), hunger_rank),
        ]);
        let buffs = HashMap::from([
            ("holy_form".into(), true),
            ("unholy_form".into(), true),
            ("insatiable_hunger".into(), hunger_toggle),
        ]);
        let mut attrs = SourceMap::new();
        let mut stats = SourceMap::from([(
            "all_skills".into(),
            vec![SourceContribution {
                label: "Test ranks".into(),
                source_type: SourceType::Custom,
                value: (2.0, 2.0),
                forge: None,
            }],
        )]);
        apply_skill_ranks(
            Some("butcher"),
            &ranks,
            None,
            &buffs,
            &HashMap::new(),
            &Inventory::new(),
            &mut attrs,
            &mut stats,
        );
        let totals = compute_final_stats(&stats);
        assert_eq!(totals["damage"], (-50.0, -50.0));
        assert_eq!(totals["life_replenish"], (2.0, 2.0));
        assert_eq!(totals["life_replenish_more"], (36.0, 36.0));
        assert_eq!(totals["life_steal_more"], (30.0, 30.0));
        assert!(!totals.contains_key("effectiveness"));
        assert!(stats
            .values()
            .flatten()
            .all(|source| !source.label.starts_with("Insatiable Hunger")));
    }
}

#[test]
fn hunger_payload_zero_gate_handles_negative_rank_bonuses_and_mixed_endpoints() {
    for mixed in [false, true] {
        for holy_active in [false, true] {
            let ranks = HashMap::from([
                ("holy_form".into(), 5),
                ("unholy_form".into(), 1),
                ("insatiable_hunger".into(), 3),
                ("sacrilegious_scorn".into(), 10),
                ("spiritual_duality".into(), 10),
            ]);
            let buffs = HashMap::from([
                ("holy_form".into(), holy_active),
                ("insatiable_hunger".into(), true),
            ]);
            let mut attrs = SourceMap::new();
            let mut stats = SourceMap::from([(
                "all_skills".into(),
                vec![SourceContribution {
                    label: "Negative custom ranks".into(),
                    source_type: SourceType::Custom,
                    value: (-1.0, if mixed { 0.0 } else { -1.0 }),
                    forge: None,
                }],
            )]);
            apply_skill_ranks(
                Some("butcher"),
                &ranks,
                None,
                &buffs,
                &HashMap::new(),
                &Inventory::new(),
                &mut attrs,
                &mut stats,
            );
            let totals = compute_final_stats(&stats);
            let holy_low = |value: f64| if holy_active { value } else { 0.0 };
            for (key, low, high) in [
                ("damage", holy_low(-50.0), 20.3),
                ("life_replenish_more", holy_low(44.0), 40.6),
                ("life_steal_more", holy_low(35.0), -50.75),
                ("life_replenish", holy_low(2.0), 2.0),
                ("increased_attack_speed", holy_low(24.0), 24.5),
                ("crit_damage", 0.0, 10.0),
                ("life_steal", holy_low(13.0), 14.0),
                ("attack_damage", 0.0, 15.0),
            ] {
                let high = if mixed { high } else { low };
                let actual = totals.get(key).copied().unwrap_or((0.0, 0.0));
                assert_eq!(
                    actual,
                    (low.min(high), low.max(high)),
                    "{key}, mixed={mixed}, holy_active={holy_active}"
                );
            }
            assert!(!totals.contains_key("effectiveness"));
        }
    }
}

#[test]
fn hunger_rank_range_includes_the_last_ordinary_form_value_before_activation() {
    let ranks = HashMap::from([
        ("holy_form".into(), 20),
        ("unholy_form".into(), 1),
        ("insatiable_hunger".into(), 3),
    ]);
    let buffs = HashMap::from([
        ("holy_form".into(), true),
        ("insatiable_hunger".into(), true),
    ]);
    let mut attrs = SourceMap::new();
    let mut stats = SourceMap::from([(
        "all_skills".into(),
        vec![SourceContribution {
            label: "Crossing ranks".into(),
            source_type: SourceType::Custom,
            value: (-2.0, 1.0),
            forge: None,
        }],
    )]);
    apply_skill_ranks(
        Some("butcher"),
        &ranks,
        None,
        &buffs,
        &HashMap::new(),
        &Inventory::new(),
        &mut attrs,
        &mut stats,
    );
    let totals = compute_final_stats(&stats);
    assert_eq!(totals["life_replenish_more"], (116.725, 164.0));
    assert_eq!(totals["life_steal_more"], (-50.875, 110.0));
    assert_eq!(totals["damage"], (-50.0, 21.876));
    assert_eq!(totals["life_replenish"], (2.0, 2.0));
}

#[test]
fn ending_fate_regeneration_requires_a_learned_active_proc_and_survives_form_changes() {
    for form in [
        None,
        Some("holy_form"),
        Some("unholy_form"),
        Some("insatiable_hunger"),
    ] {
        for learned in [false, true] {
            for active in [false, true] {
                let mut ranks = HashMap::from([("ending_fate".into(), u32::from(learned))]);
                let mut buffs = HashMap::from([("ending_fate".into(), active)]);
                if let Some(id) = form {
                    ranks.insert(id.into(), 1);
                    buffs.insert(id.into(), true);
                }
                let mut attrs = SourceMap::new();
                let mut stats = SourceMap::new();
                for (key, value) in [
                    ("all_skills", (0.0, 2.0)),
                    ("life_replenish", (100.0, 100.0)),
                    ("life_replenish_more", (20.0, 20.0)),
                ] {
                    stats.insert(
                        key.into(),
                        vec![SourceContribution {
                            label: "Test base".into(),
                            source_type: SourceType::Custom,
                            value,
                            forge: None,
                        }],
                    );
                }
                apply_skill_ranks(
                    Some("butcher"),
                    &ranks,
                    None,
                    &buffs,
                    &HashMap::new(),
                    &Inventory::new(),
                    &mut attrs,
                    &mut stats,
                );
                let ending_sources = |key: &str| {
                    stats
                        .get(key)
                        .into_iter()
                        .flatten()
                        .filter(|source| source.label.starts_with("Ending Fate"))
                        .fold((0.0, 0.0), |sum, source| {
                            (sum.0 + source.value.0, sum.1 + source.value.1)
                        })
                };
                let enabled = learned && active;
                let expected = if enabled { (50.0, 66.0) } else { (0.0, 0.0) };
                assert_eq!(
                    ending_sources("life_replenish_more"),
                    expected,
                    "{form:?}, learned={learned}, active={active}"
                );
                assert_eq!(ending_sources("life_replenish"), (0.0, 0.0));
                assert_eq!(ending_sources("attack_damage"), (0.0, 0.0));
                stats.retain(|key, _| {
                    matches!(key.as_str(), "life_replenish" | "life_replenish_more")
                });
                for sources in stats.values_mut() {
                    sources.retain(|source| {
                        source.label == "Test base" || source.label.starts_with("Ending Fate")
                    });
                }
                let mut final_stats = compute_final_stats(&stats);
                apply_multipliers_pass(&mut final_stats);
                let expected_regen = if enabled {
                    (170.0, 186.0)
                } else {
                    (120.0, 120.0)
                };
                let actual = final_stats["life_replenish"];
                assert!((actual.0 - expected_regen.0).abs() < 1e-9);
                assert!((actual.1 - expected_regen.1).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn butcher_form_passives_require_the_matching_active_form() {
    for (form, holy, unholy, hunger_unholy_learned) in [
        (None, false, false, false),
        (Some("holy_form"), true, false, false),
        (Some("unholy_form"), false, true, false),
        (Some("insatiable_hunger"), true, true, true),
        (Some("insatiable_hunger"), false, false, false),
    ] {
        for learned in [false, true] {
            let mut ranks = HashMap::from([
                ("sacrilegious_scorn".into(), 10),
                ("spiritual_duality".into(), 10),
            ]);
            let mut buffs = HashMap::new();
            if let Some(id) = form {
                ranks.insert(id.into(), u32::from(learned));
                buffs.insert(id.into(), true);
            }
            if hunger_unholy_learned {
                ranks.insert("unholy_form".into(), 1);
            }
            let mut attrs = SourceMap::new();
            let mut stats = SourceMap::new();
            stats.insert(
                "all_skills".into(),
                vec![SourceContribution {
                    label: "Rank range".into(),
                    source_type: SourceType::Custom,
                    value: (0.0, 2.0),
                    forge: None,
                }],
            );
            apply_skill_ranks(
                Some("butcher"),
                &ranks,
                None,
                &buffs,
                &HashMap::new(),
                &Inventory::new(),
                &mut attrs,
                &mut stats,
            );
            for (key, value, enabled) in [
                ("increased_attack_speed", (24.5, 25.5), holy),
                ("crit_damage", (10.0, 12.0), unholy),
                ("life_steal", (14.0, 16.0), holy),
                ("attack_damage", (15.0, 18.0), unholy),
            ] {
                let total = stats
                    .get(key)
                    .into_iter()
                    .flatten()
                    .filter(|source| {
                        source.label.starts_with("Sacrilegious Scorn")
                            || source.label.starts_with("Spiritual Duality")
                    })
                    .fold((0.0, 0.0), |sum, source| {
                        (sum.0 + source.value.0, sum.1 + source.value.1)
                    });
                let expected = if enabled && learned {
                    value
                } else {
                    (0.0, 0.0)
                };
                assert_eq!(total, expected, "{form:?}, learned={learned}, {key}");
            }
        }
    }
}

#[test]
fn defense_affixes_use_their_own_stats_and_star_rules() {
    for (id, key, plain, starred) in [
        (
            "2_10_of_target_defense_ignored_t5_armorcrushing",
            "defense_ignored",
            30.,
            30.,
        ),
        (
            "1_4_increased_defense_t5_kings_guard",
            "defense_pct",
            20.,
            23.,
        ),
        (
            "10_25_defense_vs_missiles_t5_missileguard",
            "defense_vs_missiles",
            150.,
            180.,
        ),
    ] {
        for (stars, expected) in [(0, plain), (5, starred)] {
            let inventory = Inventory::from([(
                "weapon".into(),
                EquippedItem {
                    base_id: "base_mace_cudgel".into(),
                    stars: Some(stars),
                    affixes: vec![EquippedAffix {
                        affix_id: id.into(),
                        tier: 5,
                        roll: 1.,
                        custom_value: None,
                    }],
                    ..Default::default()
                },
            )]);
            let mut attrs = SourceMap::new();
            let mut stats = SourceMap::new();
            apply_inventory(&inventory, &mut attrs, &mut stats);
            assert_eq!(
                sum_ranged_from_map(&stats, key),
                (expected, expected),
                "{id} at {stars} stars"
            );
            assert_eq!(
                sum_ranged_from_map(&stats, "defense"),
                (0., 0.),
                "{id} must not grant flat Defense"
            );
            let mut totals = compute_final_stats(&stats);
            totals.insert("defense".into(), (100., 100.));
            apply_multipliers_pass(&mut totals);
            let defense = if key == "defense_pct" {
                100. + expected
            } else {
                100.
            };
            assert_eq!(totals["defense"], (defense, defense));
        }
    }
}

#[test]
fn rainbow_sockets_floor_each_bonus_before_summing() {
    for (base_id, socket_id, types, count, expected) in [
        (
            "body_armor_angelic_st_jupe_s_plate_of_command",
            "rune_ist",
            vec![SocketType::Rainbow; 6],
            6,
            132.,
        ),
        (
            "body_armor_angelic_st_jupe_s_plate_of_command",
            "rune_ist",
            vec![SocketType::Normal; 6],
            6,
            90.,
        ),
        (
            "body_armor_angelic_st_jupe_s_plate_of_command",
            "rune_ist",
            vec![SocketType::Rainbow, SocketType::Normal],
            2,
            37.,
        ),
        (
            "charm_heroic_tablet_of_awakening",
            "rune_ist",
            vec![],
            4,
            81.,
        ),
        (
            "body_armor_heroic_gem_king_s_garb",
            "gem_pristine_topaz",
            vec![SocketType::Rainbow; 6],
            6,
            132.,
        ),
    ] {
        let inventory = Inventory::from([(
            "body".into(),
            EquippedItem {
                base_id: base_id.into(),
                socket_count: count,
                socketed: vec![Some(socket_id.into()); count as usize],
                socket_types: types,
                ..Default::default()
            },
        )]);
        let mut attrs = SourceMap::new();
        let mut stats = SourceMap::new();
        apply_inventory(&inventory, &mut attrs, &mut stats);
        let socket_total: f64 = stats["magic_find"]
            .iter()
            .filter(|source| source.source_type == SourceType::Socket)
            .map(|source| {
                assert_eq!(source.value.0, source.value.1);
                source.value.0
            })
            .sum();
        assert_eq!(socket_total, expected, "{base_id}: {socket_id}");
    }
}

#[test]
fn auroras_might_grants_lunar_aura_and_preserves_rune_bonuses() {
    let inventory = Inventory::from([(
        "weapon".into(),
        EquippedItem {
            base_id: "base_mace_cudgel".into(),
            socket_count: 3,
            socketed: vec![
                Some("rune_ymn".into()),
                Some("rune_co".into()),
                Some("rune_qi".into()),
            ],
            ..Default::default()
        },
    )]);
    let mut attrs = SourceMap::new();
    let mut stats = SourceMap::new();
    apply_inventory(&inventory, &mut attrs, &mut stats);
    let ranks =
        apply_item_granted_passive_stats(&inventory, None, &HashMap::new(), &mut attrs, &mut stats);
    assert_eq!(ranks.get("lunar aura"), Some(&(12., 28.)));
    assert_eq!(sum_ranged_from_map(&stats, "attack_rating_pct"), (24., 56.));
    assert_eq!(sum_ranged_from_map(&stats, "all_resistances"), (24., 56.));
    assert_eq!(
        sum_ranged_from_map(&stats, "additive_arcane_damage"),
        (440., 600.)
    );
    assert_eq!(
        sum_ranged_from_map(&stats, "additive_cold_damage"),
        (280., 280.)
    );
    assert_eq!(sum_ranged_from_map(&stats, "life_steal"), (7., 7.));
    assert_eq!(sum_ranged_from_map(&stats, "attack_damage"), (20., 20.));
    assert_eq!(sum_ranged_from_map(&attrs, "dexterity"), (10., 10.));
    assert_eq!(sum_ranged_from_map(&attrs, "energy"), (0., 0.));
}

#[test]
fn source_socket_prohibition_ignores_legacy_gems_but_keeps_common_sockets() {
    let gem = data::data()
        .gems
        .values()
        .find(|gem| !gem.stats.is_empty())
        .unwrap();
    let common = data::get_item("helmet_normal_cap").unwrap();
    let forbidden = data::get_item("helmet_heroic_the_colossal_avenger").unwrap();
    assert_eq!(forbidden.max_sockets, Some(0));
    for (base, expected_socket_source) in [(forbidden, false), (common, true)] {
        let inventory = Inventory::from([(
            "helmet".to_string(),
            EquippedItem {
                base_id: base.id.clone(),
                socket_count: 1,
                socketed: vec![Some(gem.id.clone())],
                forged_mods: vec![EquippedAffix {
                    affix_id: "crystal_add_socket".into(),
                    tier: 1,
                    roll: 1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )]);
        let mut attrs = SourceMap::new();
        let mut stats = SourceMap::new();
        apply_inventory(&inventory, &mut attrs, &mut stats);
        let has_socket = attrs
            .values()
            .chain(stats.values())
            .flatten()
            .any(|source| matches!(source.source_type, SourceType::Socket));
        assert_eq!(has_socket, expected_socket_source, "{}", base.id);
    }
}

#[test]
fn apply_increased_all_attributes_applies_to_each() {
    let cfg = data::game_config();
    // Seed an attribute with a known flat value.
    let first_attr = cfg.attributes.first().expect("no attrs").key.clone();
    let mut attrs: SourceMap = HashMap::new();
    push_source(
        &mut attrs,
        &first_attr,
        SourceContribution {
            label: "seed".to_string(),
            source_type: SourceType::Item,
            value: (10.0, 10.0),
            forge: None,
        },
    );
    // Seed `increased_all_attributes` percent source.
    let mut stats: SourceMap = HashMap::new();
    push_source(
        &mut stats,
        "increased_all_attributes",
        SourceContribution {
            label: "tree".to_string(),
            source_type: SourceType::Tree,
            value: (20.0, 20.0),
            forge: None,
        },
    );

    apply_increased_all_attributes(&mut attrs, &stats);

    // 10 * 20% = 2 (floor)
    let bonus_present = attrs
        .get(&first_attr)
        .map(|list| list.iter().any(|c| c.value == (2.0, 2.0)))
        .unwrap_or(false);
    assert!(bonus_present, "expected +2 bonus from 20% of 10");
}

#[test]
fn apply_increased_per_attribute_compounds_add_and_more() {
    let cfg = data::game_config();
    let first_attr = cfg.attributes.first().expect("no attrs").key.clone();

    let mut attrs: SourceMap = HashMap::new();
    push_source(
        &mut attrs,
        &first_attr,
        SourceContribution {
            label: "seed".to_string(),
            source_type: SourceType::Item,
            value: (100.0, 100.0),
            forge: None,
        },
    );
    let mut stats: SourceMap = HashMap::new();
    push_source(
        &mut stats,
        &format!("increased_{first_attr}"),
        SourceContribution {
            label: "ring".to_string(),
            source_type: SourceType::Item,
            value: (50.0, 50.0),
            forge: None,
        },
    );
    push_source(
        &mut stats,
        &format!("increased_{first_attr}_more"),
        SourceContribution {
            label: "tree total".to_string(),
            source_type: SourceType::Tree,
            value: (30.0, 30.0),
            forge: None,
        },
    );

    apply_increased_per_attribute(&mut attrs, &stats);

    // 100 * 1.5 * 1.3 = 195 → bonus = 195 - 100 = 95
    let bonus_present = attrs
        .get(&first_attr)
        .map(|list| list.iter().any(|c| c.value == (95.0, 95.0)))
        .unwrap_or(false);
    assert!(bonus_present, "expected +95 compounded bonus");
}

#[test]
fn apply_attribute_divided_stats_floors_per_divisor() {
    let mut attributes: HashMap<String, Ranged> = HashMap::new();
    // game-config divides vitality by 8 → life_replenish.
    attributes.insert("vitality".to_string(), (50.0, 50.0));
    let mut stats: SourceMap = HashMap::new();
    apply_attribute_divided_stats(&attributes, &mut stats);
    // 50 / 8 = 6.25 → floor 6.
    let life_replenish = stats
        .get("life_replenish")
        .map(|list| list.iter().any(|c| c.value == (6.0, 6.0)))
        .unwrap_or(false);
    assert!(life_replenish, "expected life_replenish from vitality÷8");
}

#[test]
fn apply_tree_disables_zeros_life_replenish() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("life_replenish".to_string(), (50.0, 50.0));
    stats.insert("life_replenish_pct".to_string(), (10.0, 10.0));
    let mut disables = HashSet::new();
    disables.insert(DisableTarget::LifeReplenish);
    apply_tree_disables(&disables, &mut stats);
    assert_eq!(stats.get("life_replenish").copied(), Some((0.0, 0.0)));
    assert_eq!(stats.get("life_replenish_pct").copied(), Some((0.0, 0.0)));
}

#[test]
fn apply_skill_ranks_passive_pushes_when_allocated() {
    // Find any class+skill combo with passive_stats.base set.
    let pick = data::data()
        .skills_by_class
        .iter()
        .find_map(|(class_id, skills)| {
            for s in skills.iter() {
                let has_passive = s
                    .passive_stats
                    .as_ref()
                    .is_some_and(|p| p.base.as_ref().is_some_and(|b| !b.is_empty()));
                if has_passive
                    && s.kind != SkillKind::Aura
                    && s.kind != SkillKind::Buff
                    && !s
                        .tags
                        .as_ref()
                        .is_some_and(|t| t.iter().any(|x| x == "Buff"))
                {
                    return Some((class_id.clone(), s.clone()));
                }
            }
            None
        });
    let Some((class_id, skill)) = pick else {
        eprintln!("no class/skill with passive_stats.base in data; skipping");
        return;
    };

    let mut ranks = HashMap::new();
    ranks.insert(skill.id.clone(), 3_u32);
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_skill_ranks(
        Some(&class_id),
        &ranks,
        None,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &mut attrs,
        &mut stats,
    );
    let total = attrs.values().map(|v| v.len()).sum::<usize>()
        + stats.values().map(|v| v.len()).sum::<usize>();
    assert!(
        total > 0,
        "expected passive contributions for skill '{}'",
        skill.id
    );
}

#[test]
fn buffing_aura_effectiveness_scales_active_aura_passive() {
    let class_id = "plague_doctor";
    let aura = data::get_skills_by_class(class_id)
        .iter()
        .find(|skill| skill.id == "lifeblood_aura")
        .unwrap()
        .clone();

    let mut ranks = HashMap::new();
    ranks.insert(aura.id.clone(), 5_u32);

    // Baseline: no buffing_aura_effectiveness in the stat map.
    let mut attrs0: SourceMap = HashMap::new();
    let mut stats0: SourceMap = HashMap::new();
    apply_skill_ranks(
        Some(&class_id),
        &ranks,
        Some(&aura.id),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &mut attrs0,
        &mut stats0,
    );
    let Some(key) = stats0.keys().next().cloned() else {
        eprintln!("aura contributed no non-attribute stat; skipping");
        return;
    };
    let base_val = stats0.get(&key).unwrap()[0].value;

    // With +100% buffing aura effectiveness the contribution doubles.
    let mut attrs1: SourceMap = HashMap::new();
    let mut stats1: SourceMap = HashMap::new();
    stats1.insert(
        "buffing_aura_effectiveness".to_string(),
        vec![SourceContribution {
            label: "test".to_string(),
            source_type: SourceType::Item,
            value: (100.0, 100.0),
            forge: None,
        }],
    );
    apply_skill_ranks(
        Some(&class_id),
        &ranks,
        Some(&aura.id),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &mut attrs1,
        &mut stats1,
    );
    let scaled_val = stats1.get(&key).unwrap()[0].value;

    assert!(
        (scaled_val.0 - base_val.0 * 2.0).abs() < 0.02
            && (scaled_val.1 - base_val.1 * 2.0).abs() < 0.02,
        "expected doubled aura contribution for '{}' key '{}': base={:?} scaled={:?}",
        aura.id,
        key,
        base_val,
        scaled_val,
    );
}

#[test]
fn damage_per_resist_scales_enhanced_damage_from_summed_resistances() {
    let seed = |m: &mut SourceMap, key: &str, v: f64| {
        m.entry(key.to_string())
            .or_default()
            .push(SourceContribution {
                label: "test".to_string(),
                source_type: SourceType::Item,
                value: (v, v),
                forge: None,
            });
    };
    // Positive: (fire 100 + cold 50) * 0.4 = +60 enhanced_damage.
    let mut pos: SourceMap = HashMap::new();
    seed(&mut pos, "fire_resistance", 100.0);
    seed(&mut pos, "cold_resistance", 50.0);
    seed(&mut pos, "damage_per_resist_point", 0.4);
    apply_damage_per_resist(&mut pos);
    let ed = sum_ranged_from_map(&pos, "enhanced_damage");
    assert!((ed.0 - 60.0).abs() < 1e-9, "positive: got {ed:?}");

    // Negative resist reduces damage: -300 * 0.4 = -120 (faithful, no clamp).
    let mut neg: SourceMap = HashMap::new();
    seed(&mut neg, "fire_resistance", -300.0);
    seed(&mut neg, "damage_per_resist_point", 0.4);
    apply_damage_per_resist(&mut neg);
    let edn = sum_ranged_from_map(&neg, "enhanced_damage");
    assert!((edn.0 + 120.0).abs() < 1e-9, "negative: got {edn:?}");
}

#[test]
fn stats_based_on_level_scale_with_character_level() {
    let seed = |m: &mut SourceMap, key: &str, v: f64| {
        m.entry(key.to_string())
            .or_default()
            .push(SourceContribution {
                label: "test".to_string(),
                source_type: SourceType::Item,
                value: (v, v),
                forge: None,
            });
    };
    // Level 100: full value (mana 525, life 475).
    let mut at100: SourceMap = HashMap::new();
    seed(&mut at100, "mana_based_on_level", 525.0);
    seed(&mut at100, "life_based_on_level", 475.0);
    seed(&mut at100, "damage_based_on_level", 100.0);
    seed(&mut at100, "enhanced_damage_based_on_level", 75.0);
    seed(&mut at100, "strength_based_on_level", 40.0);
    let mut attr100: SourceMap = HashMap::new();
    apply_stats_based_on_level(100, &mut attr100, &mut at100);
    assert!((sum_contributions(at100.get("mana").unwrap()).0 - 525.0).abs() < 1e-9);
    assert!((sum_contributions(at100.get("life").unwrap()).0 - 475.0).abs() < 1e-9);
    assert!(
        (sum_contributions(at100.get("additive_physical_damage").unwrap()).0 - 100.0).abs() < 1e-9
    );
    assert!((sum_contributions(at100.get("enhanced_damage").unwrap()).0 - 75.0).abs() < 1e-9);
    assert!(
        (sum_contributions(attr100.get("strength").unwrap()).0 - 40.0).abs() < 1e-9,
        "strength attr routed"
    );

    // Level 92: 525*0.92=483, 475*0.92=437, 100*0.92=92.
    let mut at92: SourceMap = HashMap::new();
    seed(&mut at92, "mana_based_on_level", 525.0);
    seed(&mut at92, "life_based_on_level", 475.0);
    seed(&mut at92, "damage_based_on_level", 100.0);
    seed(&mut at92, "enhanced_damage_based_on_level", 75.0);
    let mut attr92: SourceMap = HashMap::new();
    apply_stats_based_on_level(92, &mut attr92, &mut at92);
    assert!((sum_contributions(at92.get("mana").unwrap()).0 - 483.0).abs() < 1e-9);
    assert!((sum_contributions(at92.get("life").unwrap()).0 - 437.0).abs() < 1e-9);
    assert!(
        (sum_contributions(at92.get("additive_physical_damage").unwrap()).0 - 92.0).abs() < 1e-9
    );
    assert!((sum_contributions(at92.get("enhanced_damage").unwrap()).0 - 69.0).abs() < 1e-9);
}

#[test]
fn item_granted_conditional_blessing_gated_by_toggle() {
    let granted = data::item_granted_skills()
        .iter()
        .find(|g| g.condition.is_some() && g.passive_stats.is_some());
    let Some(granted) = granted else {
        eprintln!("no conditional granted skill in data; skipping");
        return;
    };
    let cond = granted.condition.clone().unwrap();
    let inv: Inventory = HashMap::new();
    let mut extra = HashMap::new();
    extra.insert(granted.name.clone(), (1.0, 1.0));

    // Toggle OFF → blessing not applied.
    let mut off_attr: SourceMap = HashMap::new();
    let mut off_stat: SourceMap = HashMap::new();
    apply_item_granted_passive_stats(
        &inv,
        Some(&extra),
        &HashMap::new(),
        &mut off_attr,
        &mut off_stat,
    );
    let off = off_attr.values().map(|v| v.len()).sum::<usize>()
        + off_stat.values().map(|v| v.len()).sum::<usize>();
    assert_eq!(off, 0, "blessing must not apply while toggle is off");

    // Toggle ON → blessing applied.
    let mut on_cond = HashMap::new();
    on_cond.insert(cond, true);
    let mut on_attr: SourceMap = HashMap::new();
    let mut on_stat: SourceMap = HashMap::new();
    apply_item_granted_passive_stats(&inv, Some(&extra), &on_cond, &mut on_attr, &mut on_stat);
    let on = on_attr.values().map(|v| v.len()).sum::<usize>()
        + on_stat.values().map(|v| v.len()).sum::<usize>();
    assert!(on > 0, "blessing must apply while toggle is on");
}

#[test]
fn radiant_power_converts_mana_with_base_pct() {
    // Tooltip: every 500 max mana → 3.75% [+0.25% per level] damage at
    // rank 1, i.e. (0.7 + 0.05 × rank)% of mana as magic skill damage.
    let mut ranks: HashMap<String, Ranged> = HashMap::new();
    ranks.insert(normalize_skill_name("Radiant Power"), (5.0, 15.0));
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("mana".into(), (10_000.0, 10_000.0));

    // Toggle OFF → the buff converts nothing.
    let mut off: SourceMap = HashMap::new();
    let touched_off = apply_item_granted_conversions(&ranks, &stats, &HashMap::new(), &mut off);
    assert!(touched_off.is_empty(), "buff must be gated by its toggle");

    // Toggle ON → (0.7 + 0.05 × rank)% of mana.
    let mut on_cond = HashMap::new();
    on_cond.insert("radiant_power_buff".to_string(), true);
    let mut on: SourceMap = HashMap::new();
    let touched = apply_item_granted_conversions(&ranks, &stats, &on_cond, &mut on);

    assert!(touched.contains("magic_skill_damage"));
    let got = sum_contributions(on.get("magic_skill_damage").unwrap());
    assert!((got.0 - 95.0).abs() < 1e-9, "rank 5: {got:?}");
    assert!((got.1 - 145.0).abs() < 1e-9, "rank 15: {got:?}");
}

#[test]
fn absolute_zero_grants_cold_break_from_rank_one() {
    // Tooltip: 0% [+2% per level] Cold Break at rank 1 → 2 × (rank - 1).
    let inv: Inventory = HashMap::new();
    let mut extra: HashMap<String, Ranged> = HashMap::new();
    extra.insert("Absolute Zero".into(), (10.0, 20.0));
    let mut attr_sources: SourceMap = HashMap::new();
    let mut stat_sources: SourceMap = HashMap::new();

    apply_item_granted_passive_stats(
        &inv,
        Some(&extra),
        &HashMap::new(),
        &mut attr_sources,
        &mut stat_sources,
    );

    let got = sum_contributions(stat_sources.get("cold_break").expect("cold_break missing"));
    assert!((got.0 - 18.0).abs() < 1e-9, "rank 10: {got:?}");
    assert!((got.1 - 38.0).abs() < 1e-9, "rank 20: {got:?}");
}

#[test]
fn trampling_force_converts_movement_speed_into_both_damage_stats() {
    // Tooltip: 50% [+15% per level] of movement speed at rank 1, added as
    // attack damage and as magic skill damage. Passive — no toggle.
    let mut ranks: HashMap<String, Ranged> = HashMap::new();
    ranks.insert(normalize_skill_name("Trampling Force"), (1.0, 3.0));
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("movement_speed".into(), (200.0, 200.0));
    let mut sources: SourceMap = HashMap::new();

    apply_item_granted_conversions(&ranks, &stats, &HashMap::new(), &mut sources);

    for key in ["attack_damage", "magic_skill_damage"] {
        let got = sum_contributions(sources.get(key).unwrap_or_else(|| panic!("{key} missing")));
        assert!((got.0 - 100.0).abs() < 1e-9, "{key} rank 1: {got:?}");
        assert!((got.1 - 160.0).abs() < 1e-9, "{key} rank 3: {got:?}");
    }
}

#[test]
fn celestial_might_moves_every_elemental_increase_into_arcane() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let mut ranks: HashMap<String, Ranged> = HashMap::new();
    ranks.insert(normalize_skill_name("Celestial Might"), (1.0, 1.0));
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    for key in [
        "fire_skill_damage",
        "cold_skill_damage",
        "lightning_skill_damage",
        "poison_skill_damage",
    ] {
        stats.insert(key.into(), (40.0, 40.0));
    }
    // Flat elemental damage is a different stat and must survive untouched.
    stats.insert("flat_fire_skill_damage".into(), (25.0, 25.0));
    let mut sources: SourceMap = HashMap::new();

    let touched = apply_item_granted_conversions(&ranks, &stats, &HashMap::new(), &mut sources);

    let arcane = sum_contributions(sources.get("arcane_skill_damage").expect("arcane missing"));
    assert!(
        (arcane.0 - 160.0).abs() < 1e-9,
        "four schools at 40% each land as 160% arcane: {arcane:?}"
    );
    for key in [
        "fire_skill_damage",
        "cold_skill_damage",
        "lightning_skill_damage",
        "poison_skill_damage",
    ] {
        let net = sum_contributions(sources.get(key).unwrap_or_else(|| panic!("{key} missing")));
        assert!(
            (net.0 + 40.0).abs() < 1e-9,
            "{key} must be cancelled out, got {net:?}"
        );
        assert!(touched.contains(key), "{key} must be re-summed");
    }
    assert!(
        !sources.contains_key("flat_fire_skill_damage"),
        "flat elemental damage is not an increase and stays put"
    );
}

#[test]
fn replaces_conversion_never_drives_its_source_negative() {
    // A school whose only bonus is a `_more` multiplier: the converted figure
    // includes it, so taking it all off the additive key would leave -8%.
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let mut ranks: HashMap<String, Ranged> = HashMap::new();
    ranks.insert(normalize_skill_name("Celestial Might"), (1.0, 1.0));
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("lightning_skill_damage".into(), (0.0, 0.0));
    stats.insert("lightning_skill_damage_more".into(), (8.0, 8.0));
    let mut sources: SourceMap = HashMap::new();

    apply_item_granted_conversions(&ranks, &stats, &HashMap::new(), &mut sources);

    let additive = sources
        .get("lightning_skill_damage")
        .map(|l| sum_contributions(l))
        .unwrap_or((0.0, 0.0));
    assert!(
        additive.0.abs() < 1e-9 && additive.1.abs() < 1e-9,
        "an empty additive side stays at zero, got {additive:?}"
    );
    let more = sources
        .get("lightning_skill_damage_more")
        .map(|l| sum_contributions(l))
        .unwrap_or((0.0, 0.0));
    assert!(
        (more.0 + 8.0).abs() < 1e-9,
        "the multiplier is what gets cancelled, got {more:?}"
    );
    let arcane = sum_contributions(sources.get("arcane_skill_damage").expect("arcane missing"));
    assert!(
        (arcane.0 - 8.0).abs() < 1e-9,
        "arcane still gains it: {arcane:?}"
    );
}

#[test]
fn a_portion_convert_leaves_its_source_alone() {
    // Fallen God's Bloodlust converts "a portion of" attack speed: the source
    // keeps its full value, unlike a replaces conversion.
    let mut ranks: HashMap<String, Ranged> = HashMap::new();
    ranks.insert(normalize_skill_name("Fallen God's Bloodlust"), (1.0, 1.0));
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("increased_attack_speed".into(), (100.0, 100.0));
    let mut sources: SourceMap = HashMap::new();

    apply_item_granted_conversions(&ranks, &stats, &HashMap::new(), &mut sources);

    assert!(sources.contains_key("faster_cast_rate"));
    assert!(
        !sources.contains_key("increased_attack_speed"),
        "no replaces flag means nothing is taken away"
    );
}

#[test]
fn apply_inventory_implicit_overrides_replace_base_implicits() {
    // Pick any item with non-empty implicit, override one of its keys
    // with a scalar value, and verify the override appears in the source.
    let item_with_implicit = data::data()
        .items
        .values()
        .find(|i| {
            i.implicit
                .as_ref()
                .is_some_and(|m| !m.is_empty() && !m.contains_key("enhanced_defense"))
        })
        .cloned();
    let Some(base) = item_with_implicit else {
        eprintln!("no item with non-ED implicit in data; skipping");
        return;
    };
    let (override_key, _) = base.implicit.as_ref().unwrap().iter().next().unwrap();
    let override_key = override_key.clone();
    let mut overrides = std::collections::HashMap::new();
    overrides.insert(override_key.clone(), 999.0_f64);

    let mut inv: Inventory = HashMap::new();
    inv.insert(
        base.slot.clone(),
        EquippedItem {
            base_id: base.id.clone(),
            implicit_overrides: overrides,
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_inventory(&inv, &mut attrs, &mut stats);

    // Override value should appear somewhere — either in attr_sources (if
    // the stat modifies an attribute) or in stat_sources.
    let in_stats = stats
        .get(&override_key)
        .map(|v| v.iter().any(|c| c.value == (999.0, 999.0)))
        .unwrap_or(false);
    let in_attrs = if let Some(def) = stat_def(&override_key) {
        if let Some(target) = def.modifies_attribute.as_deref() {
            let key = if target == "all" {
                data::game_config()
                    .attributes
                    .first()
                    .map(|a| a.key.clone())
            } else {
                Some(target.to_string())
            };
            key.and_then(|k| attrs.get(&k).cloned())
                .map(|v| v.iter().any(|c| c.value == (999.0, 999.0)))
                .unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    };
    assert!(in_stats || in_attrs, "override value not found anywhere");
}

#[test]
fn random_skill_element_lands_on_the_picked_element_skills() {
    let mut inv: Inventory = HashMap::new();
    inv.insert(
        "boots".into(),
        EquippedItem {
            base_id: "s10_phantoms_step".into(),
            random_skill_element: Some("cold".into()),
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_inventory(&inv, &mut attrs, &mut stats);
    let cold = stats
        .get("cold_skills")
        .expect("picked element gets the ranks");
    assert!(cold.iter().any(|c| c.value == (3.0, 5.0)));
    assert!(!stats.contains_key("random_skill_element"));
}

#[test]
fn all_skills_class_lands_on_the_picked_class() {
    let mut inv: Inventory = HashMap::new();
    inv.insert(
        "charm_1".into(),
        EquippedItem {
            base_id: "charm_heroic_torch_of_shadow".into(),
            all_skills_class_id: Some("jotunn".into()),
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_inventory(&inv, &mut attrs, &mut stats);
    let picked = stats
        .get("all_skills_jotunn")
        .expect("the roll lands on the picked class");
    assert!(picked.iter().any(|c| c.value == (1.0, 3.0)));
    assert!(!stats.contains_key("all_skills"));
    assert!(!stats.contains_key("all_skills_class"));
}

#[test]
fn random_element_affix_stars_match_the_selected_skill_bonus() {
    for element in ["physical", "fire", "cold", "lightning", "poison", "arcane"] {
        for (stars, expected) in [(0, 1.), (1, 1.), (2, 1.), (3, 2.), (4, 2.), (5, 3.)] {
            let inventory = Inventory::from([(
                "weapon".into(),
                EquippedItem {
                    base_id: "base_flask_achemists_flask".into(),
                    stars: Some(stars),
                    random_skill_element: Some(element.into()),
                    affixes: vec![EquippedAffix {
                        affix_id: "1_to_random_skill_element_t1_devoted".into(),
                        tier: 1,
                        roll: 1.,
                        custom_value: None,
                    }],
                    ..Default::default()
                },
            )]);
            let mut attrs = SourceMap::new();
            let mut stats = SourceMap::new();
            apply_inventory(&inventory, &mut attrs, &mut stats);
            assert_eq!(
                sum_ranged_from_map(&stats, &format!("{element}_skills")),
                (expected, expected),
                "{element} at {stars} stars"
            );
            assert!(!stats.contains_key("random_skill_element"));
        }
    }
}

#[test]
fn resolved_implicit_pins_replace_the_roll_once_even_with_stars() {
    for (base_id, slot, raw_key, resolved_key) in [
        (
            "s10_phantoms_step",
            "boots",
            "random_skill_element",
            "fire_skills",
        ),
        (
            "charm_heroic_torch_of_shadow",
            "charm_1",
            "all_skills_class",
            "all_skills_jotunn",
        ),
    ] {
        for overrides in [
            HashMap::from([(raw_key.into(), 4.)]),
            HashMap::from([(resolved_key.into(), 4.)]),
            HashMap::from([(raw_key.into(), 4.), (resolved_key.into(), 7.)]),
        ] {
            let inv = Inventory::from([(
                slot.into(),
                EquippedItem {
                    base_id: base_id.into(),
                    stars: Some(5),
                    random_skill_element: Some("fire".into()),
                    all_skills_class_id: Some("jotunn".into()),
                    implicit_overrides: overrides,
                    ..Default::default()
                },
            )]);
            let mut attrs = SourceMap::new();
            let mut stats = SourceMap::new();
            apply_inventory(&inv, &mut attrs, &mut stats);
            assert_eq!(
                sum_ranged_from_map(&stats, resolved_key),
                (4., 4.),
                "{base_id}"
            );
        }
    }
}

#[test]
fn all_skills_class_is_inert_until_a_class_is_picked() {
    let mut inv: Inventory = HashMap::new();
    inv.insert(
        "charm_1".into(),
        EquippedItem {
            base_id: "charm_heroic_torch_of_shadow".into(),
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_inventory(&inv, &mut attrs, &mut stats);
    assert!(!stats.contains_key("all_skills"));
    assert!(!stats.contains_key("all_skills_class"));
}

#[test]
fn a_rolled_random_element_affix_lands_on_the_picked_element_skills() {
    let mut inv: Inventory = HashMap::new();
    inv.insert(
        "helmet".into(),
        EquippedItem {
            base_id: "helmet_normal_cap".into(),
            affixes: vec![EquippedAffix {
                affix_id: "1_to_random_skill_element_t5_kindred".into(),
                tier: 5,
                roll: 1.0,
                custom_value: None,
            }],
            random_skill_element: Some("fire".into()),
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_inventory(&inv, &mut attrs, &mut stats);
    let fire = stats
        .get("fire_skills")
        .expect("picked element gets the ranks");
    assert!(fire.iter().any(|c| c.value == (5.0, 5.0)));
    assert!(!stats.contains_key("random_skill_element"));
}

#[test]
fn a_rolled_random_element_affix_is_inert_until_an_element_is_picked() {
    let mut inv: Inventory = HashMap::new();
    inv.insert(
        "helmet".into(),
        EquippedItem {
            base_id: "helmet_normal_cap".into(),
            affixes: vec![EquippedAffix {
                affix_id: "1_to_random_skill_element_t5_kindred".into(),
                tier: 5,
                roll: 1.0,
                custom_value: None,
            }],
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_inventory(&inv, &mut attrs, &mut stats);
    assert!(!stats.contains_key("fire_skills"));
    assert!(!stats.contains_key("random_skill_element"));
}

#[test]
fn random_skill_element_is_inert_until_an_element_is_picked() {
    let mut inv: Inventory = HashMap::new();
    inv.insert(
        "boots".into(),
        EquippedItem {
            base_id: "s10_phantoms_step".into(),
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_inventory(&inv, &mut attrs, &mut stats);
    assert!(!stats.contains_key("cold_skills"));
    assert!(!stats.contains_key("random_skill_element"));
}

// ---- charm star scaling ----

// Only common charms (Small/Large/Grand) take stars — unique charms never do.
#[test]
fn stars_scale_common_charms_only() {
    const AFFIX_ID: &str = "15_30_to_life_t1_bear";
    const UNIQUE_CHARM: &str = "charm_angelic_air_melon";

    let charm = |base_id: &str, stars: u32| {
        let mut inv: Inventory = HashMap::new();
        inv.insert(
            "charm_1".to_string(),
            EquippedItem {
                base_id: base_id.to_string(),
                stars: Some(stars),
                affixes: vec![EquippedAffix {
                    affix_id: AFFIX_ID.to_string(),
                    roll: 1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_inventory(&inv, &mut attrs, &mut stats);
        sum_ranged_from_map(&stats, "life")
    };

    let plain = charm("charms_normal_small_charm", 0);
    assert!(
        charm("charms_normal_small_charm", 5).0 > plain.0,
        "a common charm's affix scales with stars"
    );
    assert_eq!(
        charm(UNIQUE_CHARM, 5),
        charm(UNIQUE_CHARM, 0),
        "a unique charm ignores stars entirely"
    );
}

#[test]
fn rakhuls_ritual_band_mirrors_the_other_ring() {
    let mut inv: Inventory = HashMap::new();
    inv.insert(
        "ring_1".into(),
        EquippedItem {
            base_id: "ring_heroic_rakhul_s_ritual_band".into(),
            ..Default::default()
        },
    );
    inv.insert(
        "ring_2".into(),
        EquippedItem {
            base_id: "ring_heroic_signet_of_corruption".into(),
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_inventory(&inv, &mut attrs, &mut stats);
    let poison = stats.get("poison_skills").expect("mirrored implicit lands");
    assert_eq!(poison.len(), 2, "counted once per ring");
    assert!(poison.iter().any(|c| c.label.ends_with("(mirrored)")));
}

#[test]
fn two_ritual_bands_mirror_nothing() {
    let mut inv: Inventory = HashMap::new();
    for slot in ["ring_1", "ring_2"] {
        inv.insert(
            slot.into(),
            EquippedItem {
                base_id: "ring_heroic_rakhul_s_ritual_band".into(),
                ..Default::default()
            },
        );
    }
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_inventory(&inv, &mut attrs, &mut stats);
    assert!(stats.is_empty() && attrs.is_empty());
}

#[test]
fn relic_slot_applies_pinned_ranged_implicit() {
    let inventory = Inventory::from([(
        "relic_1".to_string(),
        EquippedItem {
            base_id: "relic_relic_the_spoon".into(),
            implicit_overrides: HashMap::from([("increased_mana".to_string(), 15.0)]),
            ..Default::default()
        },
    )]);
    let mut attrs = SourceMap::new();
    let mut stats = SourceMap::new();
    apply_inventory(&inventory, &mut attrs, &mut stats);
    let mana: Vec<_> = stats["increased_mana"].iter().map(|s| s.value).collect();
    assert_eq!(mana, vec![(15.0, 15.0)]);
}

#[test]
fn ignore_all_resistance_adds_to_exactly_five_elements_with_sources() {
    let mut sources = SourceMap::new();
    for (key, value, label) in [
        ("ignore_all_res", (10., 20.), "all resistance item"),
        ("ignore_fire_res", (5., 5.), "fire item"),
    ] {
        push_source(
            &mut sources,
            key,
            SourceContribution {
                label: label.into(),
                source_type: SourceType::Item,
                value,
                forge: None,
            },
        );
    }
    apply_stat_fan_outs(&mut sources);
    let totals = compute_final_stats(&sources);
    for element in ["fire", "cold", "lightning", "poison", "arcane"] {
        let key = format!("ignore_{element}_res");
        let expected = if element == "fire" {
            (15., 25.)
        } else {
            (10., 20.)
        };
        assert_eq!(totals.get(&key), Some(&expected), "{key}");
        assert!(sources[&key]
            .iter()
            .any(|s| s.label == "all resistance item"));
    }
    assert_eq!(totals["ignore_all_res"], (10., 20.));
    for element in ["physical", "magic", "explosion"] {
        assert!(!totals.contains_key(&format!("ignore_{element}_res")));
    }
}

#[test]
fn legacy_ignore_pin_replaces_base_once_and_stacks_with_nju_rune() {
    let item: EquippedItem = serde_json::from_str(
        r#"{
        "baseId":"body_armor_angelic_st_jupe_s_plate_of_command",
        "implicitOverrides":{"enemy_all_resist":23},
        "socketCount":1,"socketed":["rune_nju"]
    }"#,
    )
    .unwrap();
    let inventory = Inventory::from([("armor".into(), item)]);
    let mut attrs = SourceMap::new();
    let mut sources = SourceMap::new();
    apply_inventory(&inventory, &mut attrs, &mut sources);
    apply_stat_fan_outs(&mut sources);
    let totals = compute_final_stats(&sources);
    assert_eq!(totals["ignore_all_res"], (26., 26.));
    assert!(!totals.contains_key("enemy_all_resist"));
    for element in ["fire", "cold", "lightning", "poison", "arcane"] {
        let key = format!("ignore_{element}_res");
        assert_eq!(totals[&key], (26., 26.));
        assert_eq!(sources[&key].len(), 2, "one item pin and one rune");
    }
}
