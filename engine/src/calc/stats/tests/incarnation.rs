use super::super::*;
use super::empty_input;

#[test]
fn vital_power_increases_final_life_from_final_strength() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::from([("strength".to_string(), 1000)]);
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats = Vec::new();
    let no_nodes = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let base_input = empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &no_nodes,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let baseline = compute_build_stats(&base_input);
    let nodes = HashSet::from([742]); // Vital Power: 5% of Strength as increased Maximum Life.
    let with_node = compute_build_stats(&BuildStatsInput {
        allocated_tree_nodes: &nodes,
        ..base_input
    });
    let strength = with_node.attributes["strength"].0;
    let bonus_pct = strength * 0.05;
    assert_eq!(
        with_node.stats["increased_life"].0
            - baseline.stats.get("increased_life").map_or(0.0, |v| v.0),
        bonus_pct
    );
    assert_eq!(
        with_node.stats["life"].0,
        (baseline.stats["life"].0 * (1.0 + bonus_pct / 100.0)).floor()
    );
}

#[test]
fn spiritual_fulfilment_applies_life_multiplier_to_converted_mana() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats = Vec::new();
    let no_nodes = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let base_input = empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &no_nodes,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let baseline = compute_build_stats(&base_input);
    let nodes = HashSet::from([11, 943]); // +10% life and +10% of mana as life.
    let with_nodes = compute_build_stats(&BuildStatsInput {
        allocated_tree_nodes: &nodes,
        ..base_input
    });
    let converted = baseline.stats["mana"].0 * 0.1;
    let expected = ((baseline.stats["life"].0 + converted) * 1.1).floor();
    assert_eq!(with_nodes.stats["life"].0, expected);
}

#[test]
fn mana_from_life_reaches_mana_threshold_notes() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats = vec![CustomStat {
        stat_key: "life".to_string(),
        value: "5000".to_string(),
    }];
    let no_nodes = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let base_input = empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &no_nodes,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let baseline = compute_build_stats(&base_input);
    let nodes = HashSet::from([950, 1188]); // Life → mana; damage per 500 mana.
    let with_nodes = compute_build_stats(&BuildStatsInput {
        allocated_tree_nodes: &nodes,
        ..base_input
    });
    let expected_mana = (baseline.stats["mana"].0 + baseline.stats["life"].0 * 0.1).floor();
    assert_eq!(with_nodes.stats["mana"].0, expected_mana);
    assert_eq!(
        with_nodes.stats["ranged_projectile_damage"].0
            - baseline
                .stats
                .get("ranged_projectile_damage")
                .map_or(0.0, |v| v.0),
        (expected_mana / 500.0).floor()
    );
}

#[test]
fn strength_to_life_uses_final_strength_and_stacks_per_node() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::from([("strength".to_string(), 103)]);
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats = Vec::new();
    let no_nodes = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let base_input = empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &no_nodes,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let baseline = compute_build_stats(&base_input);
    let nodes = HashSet::from([11, 738, 739, 742]); // Two flat notes, +10% life, Vital Power.
    let with_nodes = compute_build_stats(&BuildStatsInput {
        allocated_tree_nodes: &nodes,
        ..base_input
    });
    let strength = with_nodes.attributes["strength"].0;
    let per_node = (strength / 5.0).floor();
    let bonus = per_node * 2.0;
    let expected = ((baseline.stats["life"].0 + bonus) * (1.1 + strength * 0.05 / 100.0)).floor();
    assert_eq!(with_nodes.stats["life"].0, expected);
    let sources: Vec<_> = with_nodes.stat_sources["life"]
        .iter()
        .filter(|source| source.label.contains("Strength To Life"))
        .collect();
    assert_eq!(sources.len(), 2);
    assert!(sources
        .iter()
        .all(|source| source.value == (per_node, per_node)));
}

#[test]
fn powerfunneled_lifespan_reads_weapon_damage_after_enhanced_damage() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats = Vec::new();
    let nodes = HashSet::from([1558]);
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let weapon_id = "base_melee_giant_axe";
    let weapon = crate::calc::data::get_item(weapon_id).unwrap();
    let weapon_min = weapon.damage_min.unwrap();
    let weapon_max = weapon.damage_max.unwrap();

    for added_ed in [0.0, 50.0] {
        let affixes = if added_ed == 0.0 {
            Vec::new()
        } else {
            vec![crate::calc::types::EquippedAffix {
                affix_id: "25_50_enhanced_damage_t1_soldier_s".to_string(),
                custom_value: Some(added_ed),
                ..Default::default()
            }]
        };
        let inventory = HashMap::from([(
            "weapon".to_string(),
            crate::calc::types::EquippedItem {
                base_id: weapon_id.to_string(),
                affixes,
                ..Default::default()
            },
        )]);
        let input = empty_input(
            &allocated,
            &inventory,
            &skill_ranks,
            &active_buffs,
            &custom_stats,
            &nodes,
            &tree_socketed,
            &player_conditions,
            &subskill_ranks,
            &enemy_conditions,
        );
        let result = compute_build_stats(&input);
        let ed = result.stats["enhanced_damage"].0;
        let expected = (
            weapon_min * (1.0 + ed / 100.0) * 0.075,
            weapon_max * (1.0 + ed / 100.0) * 0.075,
        );
        let actual = result
            .stats
            .get("increased_life")
            .copied()
            .unwrap_or_default();
        assert!(
            (actual.0 - expected.0).abs() < 1e-9,
            "ED +{added_ed}%: {actual:?}"
        );
        assert!(
            (actual.1 - expected.1).abs() < 1e-9,
            "ED +{added_ed}%: {actual:?}"
        );
        let raw_life = sum_contributions(&result.stat_sources["life"]).0;
        assert_eq!(
            result.stats["life"].0,
            (raw_life * (1.0 + expected.0 / 100.0)).floor()
        );
        assert_eq!(
            result.stats["life"].1,
            (raw_life * (1.0 + expected.1 / 100.0)).floor()
        );
    }
}

fn audit_build(nodes: &[u32], items: &[(&str, &str)], custom: &[(&str, f64)]) -> ComputedStats {
    audit_build_with_conditions(nodes, items, custom, &[], &[])
}

#[test]
fn critical_break_nodes_keep_both_parameters_for_their_own_element() {
    for (node, element) in [
        (1768, "cold"),
        (1782, "fire"),
        (1796, "arcane"),
        (1810, "poison"),
        (1824, "lightning"),
    ] {
        let out = audit_build(&[node], &[], &[]);
        assert_eq!(
            out.stats[&format!("{element}_break_crit_chance")],
            (15.0, 15.0)
        );
        assert_eq!(
            out.stats[&format!("{element}_break_crit_damage")],
            (25.0, 25.0)
        );
        assert_eq!(
            out.stats.get("crit_chance"),
            audit_build(&[], &[], &[]).stats.get("crit_chance")
        );
    }
}

fn audit_build_with_conditions(
    nodes: &[u32],
    items: &[(&str, &str)],
    custom: &[(&str, f64)],
    conditions: &[(&str, bool)],
    stacks: &[(&str, u32)],
) -> ComputedStats {
    let allocated = HashMap::new();
    let inventory = items
        .iter()
        .map(|(slot, id)| {
            (
                slot.to_string(),
                crate::calc::types::EquippedItem {
                    base_id: id.to_string(),
                    ..Default::default()
                },
            )
        })
        .collect();
    let ranks = HashMap::new();
    let conditions = conditions
        .iter()
        .map(|(key, value)| (key.to_string(), *value))
        .collect();
    let stacks = stacks
        .iter()
        .map(|(key, value)| (key.to_string(), *value))
        .collect();
    let custom = custom
        .iter()
        .map(|(key, value)| CustomStat {
            stat_key: key.to_string(),
            value: value.to_string(),
        })
        .collect::<Vec<_>>();
    let nodes = nodes.iter().copied().collect();
    let sockets = HashMap::new();
    let input = empty_input(
        &allocated,
        &inventory,
        &ranks,
        &conditions,
        &custom,
        &nodes,
        &sockets,
        &conditions,
        &ranks,
        &conditions,
    );
    compute_build_stats(&BuildStatsInput {
        stack_counts: &stacks,
        ..input
    })
}

#[test]
fn agile_wizard_note_disables_both_life_replenish_sources() {
    let result = audit_build(
        &[800],
        &[],
        &[("life_replenish", 100.0), ("life_replenish_pct", 5.0)],
    );
    assert_eq!(result.stats["life_replenish"], (0.0, 0.0));
    assert_eq!(result.stats["life_replenish_pct"], (0.0, 0.0));
}

#[test]
fn regeneration_restrictions_cover_percentage_sources() {
    for node in [649, 1327] {
        let resource = if node == 649 { "life" } else { "mana" };
        let flat = format!("{resource}_replenish");
        let pct = format!("{resource}_replenish_pct");
        let result = audit_build(&[node], &[], &[(&flat, 100.0), (&pct, 5.0)]);
        assert_eq!(result.stats[&flat], (0.0, 0.0), "node {node}");
        assert_eq!(result.stats[&pct], (0.0, 0.0), "node {node}");
    }
}

#[test]
fn manafury_disables_only_replenish_and_preserves_other_mana_recovery() {
    let recovery = [
        ("mana_replenish", 100.0),
        ("mana_replenish_pct", 5.0),
        ("mana_replenish_more", 50.0),
        ("mana_steal", 12.0),
        ("mana_steal_rate", 20.0),
        ("mana_per_kill", 30.0),
        ("damage_recouped_as_mana", 40.0),
        ("life_replenish_mana_on_ranged_hit", 15.0),
    ];
    let base = audit_build(&[], &[], &recovery);
    let enabled = audit_build(&[1327], &[], &recovery);
    assert_eq!(enabled.stats["mana_replenish"], (0.0, 0.0));
    assert_eq!(enabled.stats["mana_replenish_pct"], (0.0, 0.0));
    for (key, _) in &recovery[3..] {
        assert_eq!(enabled.stats[*key], base.stats[*key], "{key}");
    }
}

#[test]
fn shield_melee_bonus_requires_a_shield_and_stays_melee_scoped() {
    let shield = data::data()
        .items
        .values()
        .find(|item| item.base_type == "Shield")
        .unwrap();
    let off = audit_build(&[617], &[], &[]);
    let on = audit_build(&[617], &[("offhand", &shield.id)], &[]);
    assert_eq!(
        off.stats.get("melee_damage").copied().unwrap_or_default(),
        (0.0, 0.0)
    );
    assert_eq!(
        on.stats.get("melee_damage").copied().unwrap_or_default(),
        (8.0, 8.0)
    );
}

#[test]
fn staff_damage_reduction_requires_a_staff() {
    let staff = data::data()
        .items
        .values()
        .find(|item| item.base_type == "Staff")
        .unwrap();
    let base = audit_build(&[], &[("weapon", &staff.id)], &[]);
    let on = audit_build(&[1448], &[("weapon", &staff.id)], &[]);
    let off = audit_build(&[1448], &[], &[]);
    assert_eq!(
        on.stats
            .get("physical_damage_reduction")
            .map_or(0.0, |v| v.0)
            - base
                .stats
                .get("physical_damage_reduction")
                .map_or(0.0, |v| v.0),
        3.0
    );
    assert_eq!(
        off.stats
            .get("physical_damage_reduction")
            .copied()
            .unwrap_or_default(),
        (0.0, 0.0)
    );
}

#[test]
fn celerity_mana_recovery_uses_the_percentage_regen_key() {
    let result = audit_build(&[416], &[], &[]);
    assert_eq!(
        result
            .stats
            .get("mana_replenish_pct")
            .copied()
            .unwrap_or_default(),
        (1.0, 1.0)
    );
}

#[test]
fn divine_essence_amplifies_pearlescent_dreams_aura_contribution() {
    let items = [("boots", "boots_heroic_pearlescent_dream")];
    let base = audit_build(&[], &items, &[]);
    let damaging = audit_build(&[245], &items, &[]);
    let buffing = audit_build(&[1562], &items, &[]);
    for key in ["attack_damage", "magic_skill_damage"] {
        let aura = |result: &ComputedStats| {
            result.stat_sources[key]
                .iter()
                .find(|source| source.label.starts_with("Holy Aura ("))
                .unwrap()
                .value
        };
        let baseline = aura(&base);
        assert!(baseline.0 > 0.0);
        assert_eq!(aura(&damaging), (baseline.0 * 1.25, baseline.1 * 1.25));
        assert_eq!(aura(&buffing), baseline);
        // The item's own Magic Skill Damage affix is not an aura output.
        assert_eq!(damaging.stats[key].0 - base.stats[key].0, baseline.0 * 0.25);
    }
}

#[test]
fn externally_granted_mixed_aura_scales_each_stat_with_its_effectiveness() {
    let extra = HashMap::from([("Lunar Aura".to_string(), (10.0, 20.0))]);
    let mut attrs = SourceMap::new();
    let mut stats = SourceMap::new();
    apply_tree_contributions(
        &HashSet::from([245, 1562]),
        &HashMap::new(),
        &mut attrs,
        &mut stats,
    );
    apply_item_granted_passive_stats(
        &Inventory::new(),
        Some(&extra),
        &HashMap::new(),
        &mut attrs,
        &mut stats,
    );
    assert_eq!(
        sum_contributions(&stats["additive_arcane_damage"]),
        (125.0, 250.0)
    );
    for key in ["all_resistances", "attack_rating_pct"] {
        assert_eq!(sum_contributions(&stats[key]), (20.6, 41.2));
    }
}

#[test]
fn class_damage_aura_uses_damaging_effectiveness_only_while_active() {
    let ranks = HashMap::from([("high_voltage_aura".to_string(), 4)]);
    for active in [false, true] {
        let mut attrs = SourceMap::new();
        let mut stats = SourceMap::new();
        apply_tree_contributions(
            &HashSet::from([245, 1562]),
            &HashMap::new(),
            &mut attrs,
            &mut stats,
        );
        apply_skill_ranks(
            Some("stormweaver"),
            &ranks,
            active.then_some("high_voltage_aura"),
            &HashMap::new(),
            &Inventory::new(),
            &mut attrs,
            &mut stats,
        );
        assert_eq!(
            stats
                .get("lightning_skill_damage")
                .map(|s| sum_contributions(s))
                .unwrap_or_default(),
            if active { (16.25, 16.25) } else { (0.0, 0.0) }
        );
    }
}

#[test]
fn resistance_conversions_sum_elements_and_respect_sign_and_caps() {
    let custom = [
        ("fire_resistance", 100.0),
        ("cold_resistance", 90.0),
        ("lightning_resistance", -50.0),
        ("poison_resistance", -20.0),
        ("arcane_resistance", 10.0),
    ];
    for (node, target, expected) in [
        (1152, "life", 15.6),
        (641, "increased_life", 14.0),
        (643, "damage", 2.8),
    ] {
        let result = audit_build(&[node], &[], &custom);
        let source = result
            .stat_sources
            .get(target)
            .unwrap()
            .iter()
            .find(|s| s.label.contains(&format!("#{node}:")))
            .unwrap_or_else(|| panic!("node {node}: {:?}", result.stat_sources.get(target)));
        assert!(
            (source.value.0 - expected).abs() < 1e-9,
            "{node}: {:?}",
            source.value
        );
    }
    // Feast also lowers Total All Resistances: derive its source from the
    // actual final resistances, then compare only the positive cap excess.
    let feast = audit_build(
        &[175],
        &[],
        &[("fire_resistance", 150.0), ("cold_resistance", 120.0)],
    );
    let sum: f64 = ["fire", "cold", "lightning", "poison", "arcane"]
        .iter()
        .map(|element| {
            let key = format!("{element}_resistance");
            let cap =
                crate::calc::defense::effective_cap(&key, &feast.stats_combined).unwrap_or(75.0);
            (feast.stats_combined[&key].0 - cap).max(0.0)
        })
        .sum();
    let source = feast.stat_sources["life"]
        .iter()
        .find(|s| s.label.contains("#175:"))
        .unwrap();
    assert!((source.value.0 - sum * 0.2).abs() < 1e-9);
    for node in [641, 643] {
        let positive = audit_build(&[node], &[], &[("all_resistances", 100.0)]);
        assert!(!positive
            .stat_sources
            .values()
            .flatten()
            .any(|s| s.label.contains(&format!("#{node}:"))));
    }
}

#[test]
fn compound_life_conditions_apply_every_bonus_only_in_the_matching_state() {
    let value = |r: &ComputedStats, key: &str| r.stats.get(key).map_or(0.0, |v| v.0);
    for (node, condition, amount, defense_sign) in [
        (1050, "full_life", 25.0, -1.0),
        (1056, "life_below_40", 20.0, 1.0),
    ] {
        let off = audit_build(&[node], &[], &[]);
        let on = audit_build_with_conditions(&[node], &[], &[], &[(condition, true)], &[]);
        for key in ["increased_attack_speed", "damage"] {
            assert_eq!(value(&on, key) - value(&off, key), amount, "{node}: {key}");
        }
        for key in [
            "physical_damage_reduction",
            "fire_resistance",
            "cold_resistance",
        ] {
            assert_eq!(
                value(&on, key) - value(&off, key),
                amount * defense_sign,
                "{node}: {key}"
            );
        }
    }
    let off = audit_build(&[964, 1045], &[], &[]);
    assert_eq!(value(&off, "damage"), 0.0);
    assert_eq!(value(&off, "increased_attack_speed"), 0.0);
    let full = audit_build_with_conditions(&[964, 1045], &[], &[], &[("full_life", true)], &[]);
    assert_eq!(value(&full, "increased_attack_speed"), 8.0);
    assert_eq!(value(&full, "damage"), 0.0);
    let low = audit_build_with_conditions(&[964, 1045], &[], &[], &[("life_below_40", true)], &[]);
    assert_eq!(value(&low, "damage"), 40.0);
    assert_eq!(value(&low, "damage_taken_increased"), 40.0);
}

#[test]
fn incarnation_combat_stacks_use_configured_counts_and_caps() {
    for (node, stack, stat, rate, cap) in [
        (1072, "wizardry", "faster_cast_rate", 1.0, 50),
        (1076, "mage_guard", "physical_damage_reduction", 1.0, 25),
        (854, "surging_storm", "damage", 5.0, 10),
    ] {
        for count in [0, 3, 100] {
            let r = audit_build_with_conditions(&[node], &[], &[], &[], &[(stack, count)]);
            assert_eq!(
                r.stats.get(stat).map_or(0.0, |v| v.0),
                f64::from(count.min(cap)) * rate,
                "{node}, {count}"
            );
            if node == 854 {
                assert_eq!(
                    r.stats.get("damage_taken_increased").map_or(0.0, |v| v.0),
                    f64::from(count.min(cap)) * rate
                );
            }
        }
    }
}

#[test]
fn unarmed_wargod_adds_strength_to_weapon_and_disables_strength_damage_only_unarmed() {
    let custom = [("to_strength", 100.0)];
    let unarmed = audit_build(&[391], &[], &custom);
    assert_eq!(
        unarmed.stats["str_to_unarmed_damage"].0,
        unarmed.attributes["strength"].0 * 0.2
    );
    assert_eq!(
        unarmed
            .stats
            .get("enhanced_damage")
            .copied()
            .unwrap_or_default(),
        (0.0, 0.0)
    );
    assert_eq!(unarmed.stats["attack_damage"].0, 10.0);
    let armed = audit_build(&[391], &[("weapon", "base_melee_giant_axe")], &custom);
    assert!(!armed.stats.contains_key("str_to_unarmed_damage"));
    assert!(armed.stats["enhanced_damage"].0 > 0.0);
    assert_eq!(
        armed
            .stats
            .get("attack_damage")
            .copied()
            .unwrap_or_default(),
        (0.0, 0.0)
    );
}

#[test]
fn phasing_mitigation_requires_the_phasing_condition() {
    let off = audit_build(&[221], &[], &[]);
    let on = audit_build_with_conditions(&[221], &[], &[], &[("phasing", true)], &[]);
    assert_eq!(
        off.stats
            .get("damage_mitigation")
            .copied()
            .unwrap_or_default(),
        (0.0, 0.0)
    );
    assert_eq!(on.stats["damage_mitigation"], (15.0, 15.0));
    assert_eq!(on.stats["movement_speed"], off.stats["movement_speed"]);
}

#[test]
fn immovable_object_removes_attack_dodge() {
    let r = audit_build(
        &[578],
        &[],
        &[
            ("dodge_chance", 30.0),
            ("dodge_physical_damage_chance", 15.0),
        ],
    );
    assert_eq!(r.stats["dodge_chance"], (0.0, 0.0));
    assert_eq!(r.stats["dodge_physical_damage_chance"], (0.0, 0.0));
}

#[test]
fn far_reaching_carnage_keeps_the_user_approved_melee_bonus() {
    let r = audit_build(&[441], &[], &[]);
    assert_eq!(r.stats["melee_damage"], (40.0, 40.0));
    assert!(!r.stats.contains_key("damage_far"));
}

#[test]
fn energetic_carnage_converts_final_energy_without_a_mana_threshold() {
    for mana in [0.0, 5000.0] {
        let r = audit_build(&[1193], &[], &[("to_energy", 1000.0), ("mana", mana)]);
        assert_eq!(
            r.stats["flat_ranged_physical_damage"].0,
            r.attributes["energy"].0 * 0.25
        );
        assert!(!r.stats.contains_key("ranged_physical_per_500_mana"));
    }
}

#[test]
fn area_damage_does_not_inflate_radius() {
    let r = audit_build(&[381, 1161], &[], &[]);
    assert_eq!(r.stats["area_of_effect"], (10.0, 10.0));
    assert_eq!(r.stats["area_skill_damage"], (13.0, 13.0));
}

#[test]
fn physical_element_conversions_keep_percentages_until_the_skill_hit_exists() {
    let nodes: Vec<u32> = (1080..1110).collect();
    for physical in [0.0, 10000.0] {
        let result = audit_build(&nodes, &[], &[("additive_physical_damage", physical)]);
        for element in crate::calc::skills::ELEMENTS {
            assert_eq!(
                result.stats[&format!("physical_to_{element}")],
                (75.0, 75.0)
            );
        }
    }
}

#[test]
fn dual_wield_permission_nodes_keep_their_total_speed_penalties() {
    for (node, key) in [(731, "increased_attack_speed"), (1275, "faster_cast_rate")] {
        for increased in [0.0, 100.0] {
            let custom = [(key, increased)];
            let baseline = audit_build(&[], &[], &custom);
            let changed = audit_build(&[node], &[], &custom);
            let speed = |result: &ComputedStats| {
                1.0 + result.stats.get(key).copied().unwrap_or_default().1 / 100.0
            };
            assert!((speed(&changed) / speed(&baseline) - 0.75).abs() < 1e-9);
        }
    }
}

#[test]
fn wizards_wrath_switches_cast_rate_and_global_damage_on_overheat() {
    for (heated, expected_cast, expected_damage) in [(false, 50.0, 0.0), (true, -50.0, 50.0)] {
        let result = audit_build_with_conditions(&[992], &[], &[], &[("overheated", heated)], &[]);
        // The diminishing-return pass folds Total Cast Rate into the final
        // cast-rate value and clears its multiplier bucket.
        assert_eq!(
            result.stats["faster_cast_rate"],
            (expected_cast, expected_cast)
        );
        assert_eq!(
            result.stats.get("damage").copied().unwrap_or_default(),
            (expected_damage, expected_damage)
        );
        assert!(!result.stats.contains_key("enhanced_damage_more"));
        let without_node =
            audit_build_with_conditions(&[], &[], &[], &[("overheated", heated)], &[]);
        assert!(!without_node.stats.contains_key("faster_cast_rate_more"));
    }
}

#[test]
fn mind_over_matter_adds_only_the_mana_available_for_damage_diversion() {
    let result = audit_build(&[662], &[], &[("life", 1000.0), ("mana", 100.0)]);
    assert_eq!(result.stats["damage_drained_from_mana"], (25.0, 25.0));
    let life = result.stats["life"].1;
    let mana = result.stats["mana"].1;
    let ehp = crate::calc::defense::compute_ehp(&result.stats);
    let physical = ehp
        .entries
        .iter()
        .find(|entry| entry.damage_type == "physical")
        .unwrap();
    assert!((physical.ehp.unwrap() - (life + mana.min(life / 3.0))).abs() < 1e-8);
}

#[test]
fn tree_defense_scales_equipped_armor_and_feeds_hulking_colossus() {
    let armor = data::data()
        .items
        .values()
        .find(|item| item.defense_min.is_some_and(|value| value > 0.0) && item.slot != "weapon")
        .unwrap();
    let inventory = [(armor.slot.as_str(), armor.id.as_str())];
    let base = audit_build(&[], &inventory, &[("defense", 1000.0)]);
    let result = audit_build(&[56, 58], &inventory, &[("defense", 1000.0)]);
    let defense = (base.stats["defense"].0 * 1.13).floor();
    assert_eq!(result.stats["defense"].0, defense);
    assert_eq!(result.stats["defense_pct"], (13.0, 13.0));
    assert!((result.stats["life"].0 - (base.stats["life"].0 + defense * 0.025)).abs() < 1e-8);
    let breakdown = compute_stat_breakdown(&result.stat_sources, "defense", None);
    assert!((breakdown.combined.0 - base.stats["defense"].0 * 1.13).abs() < 1e-8);
}

#[test]
fn enhanced_damage_tree_branches_follow_weapon_type_and_reach_powerfunnel() {
    for (kind, expected) in [
        ("Sword", (4.0, 8.0)),
        ("Bow", (30.0, 8.0)),
        ("Flask", (30.0, 8.0)),
        ("Staff", (4.0, 8.0)),
        ("Wand", (4.0, 8.0)),
    ] {
        let weapon = data::data()
            .items
            .values()
            .find(|item| {
                item.slot == "weapon" && weapon_kind_of(item) == kind && item.rarity == "common"
            })
            .unwrap();
        let items = [("weapon", weapon.id.as_str())];
        let base = audit_build(&[], &items, &[]);
        let result = audit_build(&[1573, 1576, 1610, 1603, 1558], &items, &[]);
        for (key, delta) in [
            ("enhanced_damage", expected.0),
            ("enhanced_damage_more", expected.1),
        ] {
            let before = base.stats.get(key).copied().unwrap_or_default();
            let after = result.stats.get(key).copied().unwrap_or_default();
            assert_eq!(
                (after.0 - before.0, after.1 - before.1),
                (delta, delta),
                "{kind}: {key}"
            );
        }
        let ed = result
            .stats
            .get("enhanced_damage")
            .copied()
            .unwrap_or_default()
            .0;
        let more = result
            .stats
            .get("enhanced_damage_more")
            .copied()
            .unwrap_or_default()
            .0;
        let flat = result
            .stats
            .get("additive_physical_damage")
            .copied()
            .unwrap_or_default()
            .0;
        let attack = result
            .stats
            .get("attack_damage")
            .copied()
            .unwrap_or_default()
            .0;
        let expected_life_pct =
            (weapon.damage_min.unwrap() * (1.0 + ed / 100.0) * (1.0 + more / 100.0) + flat)
                * (1.0 + attack / 100.0)
                * 0.075;
        let added_life_pct = result.stats["increased_life"].0
            - base
                .stats
                .get("increased_life")
                .copied()
                .unwrap_or_default()
                .0;
        assert!((added_life_pct - expected_life_pct).abs() < 1e-8, "{kind}");
    }
}

#[test]
fn spacial_conversion_preserves_radius_and_benefits_non_spell_area_skills() {
    let result = audit_build(
        &[1141],
        &[],
        &[("area_of_effect", 40.0), ("area_of_effect_more", 50.0)],
    );
    // (1 + 40%) × (1 + 50%) - 1 = 110%, copied without consuming radius.
    assert_eq!(result.stats_combined["area_of_effect"], (110.0, 110.0));
    assert_eq!(result.stats["area_skill_damage"], (110.0, 110.0));
    for tags in [
        vec!["Melee", "Area of Effect"],
        vec!["Spell", "Area of Effect"],
    ] {
        let tags = tags.into_iter().map(str::to_string).collect::<Vec<_>>();
        assert_eq!(
            crate::calc::affix_tags::sum_for(
                crate::calc::types::AffixEffect::Damage,
                &tags,
                &result.stats,
            ),
            (110.0, 110.0)
        );
    }
    assert!(!result.stats.contains_key("spell_aoe_damage"));
}

#[test]
fn charge_damage_reads_final_strength_and_vitality() {
    let r = audit_build(
        &[367, 370, 2042, 2046],
        &[],
        &[("to_strength", 1000.0), ("to_vitality", 1000.0)],
    );
    let expected = r.attributes["strength"].0 * 0.2 + r.attributes["vitality"].0 * 0.35;
    assert!((r.stats["charging_damage"].0 - expected).abs() < 1e-9);
}

#[test]
fn swinging_axes_requires_both_weapons_to_be_axes() {
    let axe = "base_melee_giant_axe";
    let sword = data::data()
        .items
        .values()
        .find(|base| base.slot == "weapon" && weapon_kind_of(base) == "Sword")
        .unwrap();
    for offhand in [None, Some(axe), Some(sword.id.as_str())] {
        let mut items = vec![("weapon", axe)];
        if let Some(id) = offhand {
            items.push(("offhand", id));
        }
        let base = audit_build(&[], &items, &[]);
        let r = audit_build(&[684], &items, &[]);
        let enabled = offhand == Some(axe);
        for (key, bonus) in [("additive_physical_damage", 100.0), ("attack_radius", 80.0)] {
            let delta =
                r.stats.get(key).map_or(0.0, |v| v.0) - base.stats.get(key).map_or(0.0, |v| v.0);
            assert_eq!(
                delta,
                if enabled { bonus } else { 0.0 },
                "{offhand:?}, {key}"
            );
        }
    }
}

#[test]
fn quillboar_trigger_chance_is_not_extra_return_damage() {
    let r = audit_build(&[236], &[], &[]);
    assert_eq!(r.stats["damage_return_more"], (15.0, 15.0));
    assert_eq!(r.stats["quillboar_chance"], (50.0, 50.0));
}

#[test]
fn mana_replenish_percentage_scales_recovery_instead_of_adding_flat_mana() {
    let custom = [("mana_replenish", 100.0), ("mana_replenish_pct", 2.0)];
    let base = audit_build(&[], &[], &custom);
    let r = audit_build(&[22], &[], &custom);
    assert_eq!(
        r.stats["mana_replenish"].0,
        base.stats["mana_replenish"].0 * 1.25
    );
    assert_eq!(r.stats["mana_replenish_pct"], (2.5, 2.5));
}

#[test]
fn recovery_totals_are_not_multiplied_twice_in_display_or_sustain_stats() {
    for resource in ["life", "mana"] {
        let flat = format!("{resource}_replenish");
        let pct = format!("{resource}_replenish_pct");
        let more = format!("{resource}_replenish_more");
        let r = audit_build(&[], &[], &[(&flat, 100.0), (&pct, 2.0), (&more, 50.0)]);
        assert_eq!(r.stats_combined[&flat], r.stats[&flat]);
        assert_eq!(r.stats[&pct], (3.0, 3.0));
        let node = if resource == "life" { 800 } else { 1327 };
        let disabled = audit_build(&[node], &[], &[(&flat, 100.0), (&more, 50.0)]);
        assert_eq!(disabled.stats_combined[&flat], (0.0, 0.0));
    }
}

#[test]
fn resistance_limiter_sets_fifty_percent_even_with_extra_maximum_resistances() {
    let r = audit_build(&[418, 556], &[], &[]);
    for element in ["fire", "cold", "lightning", "poison", "arcane"] {
        assert_eq!(
            crate::calc::defense::effective_cap(&format!("{element}_resistance"), &r.stats),
            Some(50.0)
        );
        let entry = r
            .ehp
            .entries
            .iter()
            .find(|e| e.damage_type == element)
            .unwrap();
        assert_eq!(entry.multiplier, 0.5);
    }
    let feast = audit_build(&[418, 175], &[], &[("all_resistances", 100.0)]);
    let mut combined = feast.stats.clone();
    combined.extend(feast.stats_combined.clone());
    let excess: f64 = ["fire", "cold", "lightning", "poison", "arcane"]
        .iter()
        .map(|element| (combined[&format!("{element}_resistance")].0 - 50.0).max(0.0))
        .sum();
    let from_feast = feast.stat_sources["life"]
        .iter()
        .find(|s| s.label.contains("#175:"))
        .unwrap();
    assert!((from_feast.value.0 - excess * 0.2).abs() < 1e-9);
}

#[test]
fn delayed_recovery_compensates_tick_interval_in_its_per_second_rate() {
    for (node, resource, multiplier) in [(719, "mana", 1.5), (951, "life", 4.0)] {
        let flat = format!("{resource}_replenish");
        let custom = [(&*flat, 100.0)];
        let base = audit_build(&[], &[], &custom);
        let r = audit_build(&[node], &[], &custom);
        assert_eq!(
            r.stats[&format!("{resource}_replenish_interval")],
            (2.0, 2.0)
        );
        assert_eq!(r.stats[&flat].0, base.stats[&flat].0 * multiplier);
        assert_eq!(r.stats_combined[&flat], r.stats[&flat]);
    }
}

#[test]
fn citadel_scales_recovery_with_configured_colossus_stacks() {
    let custom = [("life_replenish", 100.0)];
    let base = audit_build(&[], &[], &custom);
    for count in [0, 1, 4, 5, 99] {
        let r = audit_build_with_conditions(
            &[579, 580, 581, 582, 583],
            &[],
            &custom,
            &[],
            &[("colossus", count)],
        );
        assert_eq!(
            r.stats["life_replenish"].0,
            base.stats["life_replenish"].0 * (1.0 + 2.5 * f64::from(count.min(5)))
        );
        assert_eq!(
            r.stats_combined
                .get("life_replenish")
                .copied()
                .unwrap_or(r.stats["life_replenish"]),
            r.stats["life_replenish"]
        );
    }
}

#[test]
fn ramping_pulse_counts_radius_stacks_without_inflating_damage_conversions() {
    let baseline = audit_build(&[381, 1141], &[], &[]);
    for count in [0, 1, 4, 10, 99] {
        let result = audit_build_with_conditions(
            &[381, 1141, 1160],
            &[],
            &[],
            &[],
            &[("ramping_pulse", count)],
        );
        let expected = 5.0 * f64::from(count.min(10));
        assert_eq!(
            result
                .stats
                .get("ramping_pulse_radius")
                .copied()
                .unwrap_or_default(),
            (expected, expected)
        );
        // The game applies this buff when loading skill size, after the global
        // radius getters. It must not feed Spacial Conversion a second source.
        for key in ["area_of_effect", "area_skill_damage"] {
            assert_eq!(result.stats.get(key), baseline.stats.get(key));
        }
    }
    assert_eq!(
        audit_build(&[1160], &[], &[]).stats["ramping_pulse_radius"],
        (50.0, 50.0)
    );
    // More trigger chance affects uptime, not the bonus of an existing stack.
    let higher_chance = audit_build_with_conditions(
        &[1160],
        &[],
        &[("ramping_pulse_chance", 100.0)],
        &[],
        &[("ramping_pulse", 2)],
    );
    assert_eq!(higher_chance.stats["ramping_pulse_radius"], (10.0, 10.0));
    let capacity_only = audit_build_with_conditions(
        &[],
        &[],
        &[("ramping_pulse_max_stacks", 10.0)],
        &[],
        &[("ramping_pulse", 10)],
    );
    assert!(!capacity_only.stats.contains_key("ramping_pulse_radius"));
}

#[test]
fn agitation_scales_movement_speed_with_up_to_ten_stacks() {
    let base = audit_build(&[71, 74], &[], &[]);
    for count in [0, 1, 4, 10, 99] {
        let result = audit_build_with_conditions(
            &[71, 74, 608],
            &[],
            &[],
            &[],
            &[("agitation", count)],
        );
        assert_eq!(result.stats["max_agitation_stacks"], (10.0, 10.0));
        let extra = 10.0 * f64::from(count.min(10));
        // Agitation joins increased movement speed before the existing +5%
        // total multiplier, and never changes attack/cast speed.
        let expected = base.stats["movement_speed"].0 + extra * 1.05;
        assert!((result.stats["movement_speed"].0 - expected).abs() < 1e-9);
        assert!((result.stats["movement_speed"].1 - expected).abs() < 1e-9);
        for key in ["increased_attack_speed", "faster_cast_rate"] {
            assert_eq!(result.stats.get(key), base.stats.get(key));
        }
    }
    assert_eq!(
        audit_build(&[608], &[], &[]).stats["movement_speed"],
        (100.0, 100.0)
    );
    let inactive = audit_build_with_conditions(&[71, 74], &[], &[], &[], &[("agitation", 10)]);
    assert_eq!(inactive.stats["movement_speed"], base.stats["movement_speed"]);
}

#[test]
fn colossus_grants_total_damage_per_stack_and_has_one_base_stack() {
    for nodes in [&[579][..], &[579, 580, 581, 582][..]] {
        let rate = nodes.len() as f64;
        let cap = nodes.len() as u32 + 1;
        for count in [0, 1, cap, 99] {
            let result = audit_build_with_conditions(nodes, &[], &[], &[], &[("colossus", count)]);
            assert_eq!(
                result.stats["max_colossus_stacks"],
                (cap as f64, cap as f64)
            );
            let expected = rate * count.min(cap) as f64;
            assert_eq!(
                result.stats.get("damage").copied().unwrap_or_default(),
                (expected, expected)
            );
        }
        let default = audit_build(nodes, &[], &[]);
        assert_eq!(
            default.stats["damage"],
            (rate * cap as f64, rate * cap as f64)
        );
    }
    // Extra capacity alone cannot grant the on-hit buff: the game requires a
    // positive Colossus damage value before creating it.
    let dormant = audit_build_with_conditions(
        &[583],
        &[],
        &[("max_colossus_stacks", 4.0)],
        &[],
        &[("colossus", 3)],
    );
    assert_eq!(
        dormant.stats["life_replenish"],
        audit_build(&[], &[], &[]).stats["life_replenish"]
    );
}

#[test]
fn combat_mitigation_stacks_reach_ehp_and_respect_base_capacity() {
    let base = audit_build(&[], &[], &[]);
    for (nodes, cap, rate) in [
        (&[467][..], 1_u32, 1.0),
        (&[467, 468, 469, 470, 471][..], 3, 4.0),
    ] {
        for count in [0, 1, cap, 99] {
            let result =
                audit_build_with_conditions(nodes, &[], &[], &[], &[("combat_mitigation", count)]);
            let reduction = rate * count.min(cap) as f64;
            assert_eq!(
                result.stats["max_combat_mitigation_stacks"],
                (cap as f64, cap as f64)
            );
            assert_eq!(
                result
                    .stats
                    .get("damage_mitigation")
                    .copied()
                    .unwrap_or_default(),
                (reduction, reduction)
            );
            for (before, after) in base.ehp.entries.iter().zip(&result.ehp.entries) {
                let ratio = after.ehp.unwrap() / before.ehp.unwrap();
                assert!((ratio - 1.0 / (1.0 - reduction / 100.0)).abs() < 1e-9);
            }
        }
    }
    assert_eq!(
        audit_build(&[471], &[], &[]).ehp.entries[0].ehp,
        base.ehp.entries[0].ehp
    );
}

#[test]
fn recovery_breakdowns_explain_the_same_multipliers_as_final_stats() {
    let r = audit_build(&[22, 719], &[], &[("mana_replenish_pct", 2.0)]);
    for key in ["mana_replenish", "mana_replenish_pct"] {
        let breakdown = compute_stat_breakdown(&r.stat_sources, key, None);
        assert_eq!(breakdown.increased_sum, (25.0, 25.0));
        assert_eq!(breakdown.more_sum, (50.0, 50.0));
        assert_eq!(breakdown.combined, r.stats[key]);
    }
}

#[test]
fn flask_reduction_only_applies_while_regenerating_from_a_flask() {
    let off = audit_build(&[396], &[], &[]);
    let on = audit_build_with_conditions(&[396], &[], &[], &[("flask_regeneration", true)], &[]);
    assert_eq!(
        off.stats
            .get("all_damage_taken_reduced_pct")
            .copied()
            .unwrap_or_default(),
        (0.0, 0.0)
    );
    assert_eq!(on.stats["all_damage_taken_reduced_pct"], (25.0, 25.0));
    for (before, after) in off.ehp.entries.iter().zip(on.ehp.entries.iter()) {
        assert!((after.ehp.unwrap() / before.ehp.unwrap() - 1.0 / 0.75).abs() < 1e-9);
    }
}

#[test]
fn shield_damage_bounds_keep_flat_percent_and_equipment_conditions_separate() {
    let shield = data::data()
        .items
        .values()
        .find(|item| item.base_type == "Shield")
        .unwrap();
    for (node, key, value) in [
        (1504, "maximum_damage_flat", 8.0),
        (1505, "maximum_damage_flat", 8.0),
        (1506, "maximum_damage_flat", 8.0),
        (1507, "maximum_damage_flat", 8.0),
        (1508, "maximum_damage_pct", 4.0),
        (1509, "minimum_damage_flat", 8.0),
        (1510, "minimum_damage_flat", 8.0),
        (1511, "minimum_damage_flat", 8.0),
        (1512, "minimum_damage_flat", 8.0),
        (1513, "minimum_damage_pct", 4.0),
    ] {
        let off = audit_build(&[node], &[], &[]);
        assert_eq!(off.stats.get(key).copied().unwrap_or_default(), (0.0, 0.0));
        let base = audit_build(&[], &[("offhand", &shield.id)], &[]);
        let on = audit_build(&[node], &[("offhand", &shield.id)], &[]);
        let original = base.stats.get(key).copied().unwrap_or_default();
        assert_eq!(
            on.stats[key],
            (original.0 + value, original.1 + value),
            "node {node}"
        );
        assert_eq!(
            on.stats.get("attack_damage"),
            base.stats.get("attack_damage"),
            "flat bounds are not attack percentages"
        );
    }
    let on = audit_build(
        &(1504..=1513).collect::<Vec<_>>(),
        &[("offhand", &shield.id)],
        &[],
    );
    assert_eq!(on.stats["minimum_damage_flat"], (32.0, 32.0));
    assert_eq!(on.stats["maximum_damage_flat"], (32.0, 32.0));
    assert_eq!(on.stats["minimum_damage_pct"], (4.0, 4.0));
    assert_eq!(on.stats["maximum_damage_pct"], (4.0, 4.0));
}
