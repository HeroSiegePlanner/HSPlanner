use super::super::*;
use super::empty_input;
use crate::calc::types::EquippedItem;

#[test]
fn item_stat_conversions_reach_the_final_stats() {
    let allocated = HashMap::from([("intelligence".to_string(), 100)]);
    let inventory = HashMap::new();
    let ranks = HashMap::new();
    let flags = HashMap::new();
    let tree = HashSet::new();
    let sockets = HashMap::new();
    let custom = vec![CustomStat {
        stat_key: "intelligence_converted_to_magic_skill_dmg".into(),
        value: "10".into(),
    }];
    let input = empty_input(
        &allocated, &inventory, &ranks, &flags, &custom, &tree, &sockets, &flags, &ranks, &flags,
    );
    let result = compute_build_stats(&input);
    let intelligence = result.attributes["intelligence"];
    assert!(intelligence.0 >= 100.0);
    let magic = result.stats["magic_skill_damage"];
    let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(close(magic.0, intelligence.0 / 10.0) && close(magic.1, intelligence.1 / 10.0));
}

#[test]
fn fuel_count_reaches_the_pipeline_and_mania_keeps_area_conversions_unchanged() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let buffs = HashMap::from([("fuel_to_fire".into(), true)]);
    let tree = HashSet::from([381, 1141]);
    let sockets = HashMap::new();
    let conditions = HashMap::new();
    let subskills = HashMap::new();
    let custom = vec![
        CustomStat {
            stat_key: "area_of_effect".into(),
            value: "32".into(),
        },
        CustomStat {
            stat_key: "melee_attack_size_offset".into(),
            value: "0.5".into(),
        },
        CustomStat {
            stat_key: "melee_attack_size_offset".into(),
            value: "0.75".into(),
        },
        CustomStat {
            stat_key: "melee_projectile_size".into(),
            value: "0.5-0.75".into(),
        },
    ];
    let evaluate = |fuel, mania, count| {
        let ranks = HashMap::from([
            ("fuel_to_fire".into(), fuel),
            ("enraged_mania".into(), mania),
        ]);
        let counts = HashMap::from([("fuel_to_fire_consumed_objects".into(), count)]);
        let mut input = empty_input(
            &allocated,
            &inventory,
            &ranks,
            &buffs,
            &custom,
            &tree,
            &sockets,
            &conditions,
            &subskills,
            &conditions,
        );
        input.class_id = Some("butcher");
        input.stack_counts = &counts;
        compute_build_stats(&input)
    };
    let baseline = evaluate(0, 0, 0);
    for count in [0, 5] {
        let with_fuel = evaluate(1, 0, count);
        assert_eq!(
            with_fuel.stats["damage"].0
                - baseline.stats.get("damage").copied().unwrap_or_default().0,
            if count == 0 { 20.0 } else { 21.0 }
        );
    }
    assert!(
        baseline.stats["area_skill_damage"].0 > 0.0,
        "conversion must be exercised"
    );
    assert_eq!(baseline.stats["melee_attack_size_offset"], (1.0, 1.0));
    assert_eq!(baseline.stats["melee_projectile_size"], (0.5, 0.75));
    for (rank, size, attack_speed) in [(1, 9.0, 0.5), (3, 11.0, 1.5)] {
        let mania = evaluate(0, rank, 0);
        for key in [
            "area_of_effect",
            "area_skill_damage",
            "melee_projectile_size",
        ] {
            assert_eq!(mania.stats.get(key), baseline.stats.get(key), "{key}");
        }
        assert_eq!(mania.stats["melee_attack_size_offset"], (size, size));
        assert_eq!(
            mania.stats["increased_attack_speed"].0
                - baseline
                    .stats
                    .get("increased_attack_speed")
                    .copied()
                    .unwrap_or_default()
                    .0,
            attack_speed
        );
    }
}

#[test]
fn hunger_for_blood_floors_the_aggregated_global_bleeding_bonus() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let buffs = HashMap::new();
    let tree = HashSet::new();
    let sockets = HashMap::new();
    let conditions = HashMap::new();
    let subskills = HashMap::new();
    for (rank, other_source, expected) in [
        (1, 0.0, 1.0),
        (2, 0.0, 2.0),
        (3, 0.0, 3.0),
        (4, 0.0, 5.0),
        (1, 0.75, 2.0),
    ] {
        let ranks = HashMap::from([("hunger_for_blood".into(), rank)]);
        let custom = vec![CustomStat {
            stat_key: "extra_damage_bleeding".into(),
            value: other_source.to_string(),
        }];
        let mut input = empty_input(
            &allocated,
            &inventory,
            &ranks,
            &buffs,
            &custom,
            &tree,
            &sockets,
            &conditions,
            &subskills,
            &conditions,
        );
        input.class_id = Some("butcher");
        let result = compute_build_stats(&input);
        assert_eq!(
            result.stats.get("extra_damage_bleeding").copied(),
            Some((expected, expected)),
            "rank={rank}, other_source={other_source}"
        );
    }
}

#[test]
fn compute_build_stats_with_empty_input_does_not_panic() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let input = empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let result = compute_build_stats(&input);
    // Even with no class, default base stats from game-config populate
    // some entries.
    assert!(
        !result.stats.is_empty(),
        "default base stats should be present"
    );
    // Base attributes from game-config seed the attribute map.
    assert!(
        !result.attributes.is_empty(),
        "attributes should be seeded from defaults"
    );
}

#[test]
fn blade_strike_base_crit_never_enters_global_stats() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let buffs = HashMap::new();
    let custom = vec![CustomStat {
        stat_key: "crit_chance".into(),
        value: "10".into(),
    }];
    let tree = HashSet::new();
    let sockets = HashMap::new();
    let conditions = HashMap::new();
    let subskills = HashMap::new();
    let global_crit = |main, blade_rank| {
        let ranks = HashMap::from([
            ("flying_scimitar".into(), 10),
            ("blade_strike".into(), blade_rank),
        ]);
        let mut input = empty_input(
            &allocated,
            &inventory,
            &ranks,
            &buffs,
            &custom,
            &tree,
            &sockets,
            &conditions,
            &subskills,
            &conditions,
        );
        input.class_id = Some("nomad");
        input.main_skill_id = Some(main);
        input.level = 50;
        compute_build_stats(&input).stats["crit_chance"]
    };
    let baseline = global_crit("flying_scimitar", 0);
    for main in ["flying_scimitar", "blade_strike"] {
        for rank in [1, 5, 20] {
            assert_eq!(
                global_crit(main, rank),
                baseline,
                "Blade Strike rank {rank} must keep its own critical bonus out of global stats for {main}"
            );
        }
    }
}

#[test]
fn subskill_stats_reach_only_their_own_skill() {
    // charged_bolts/efficient_wiring grants the shared mana_cost_reduction. It
    // must not reach lightning_surge, which has its own subtree. (Subskill
    // LSSD such as electric_vains is skill-scoped subtree_damage now.)
    let ranks: HashMap<String, u32> =
        HashMap::from([("charged_bolts:efficient_wiring".to_string(), 3)]);
    let enemy = HashMap::new();

    let mut attrs = HashMap::new();
    let mut stats = HashMap::new();
    apply_subskill_aggregation(
        Some("stormweaver"),
        Some("lightning_surge"),
        &ranks,
        Some(&enemy),
        &mut attrs,
        &mut stats,
        "",
    );
    assert!(
        !stats.contains_key("mana_cost_reduction"),
        "another skill's subtree leaked into the stat map: {stats:?}"
    );

    let mut attrs = HashMap::new();
    let mut stats = HashMap::new();
    apply_subskill_aggregation(
        Some("stormweaver"),
        Some("charged_bolts"),
        &ranks,
        Some(&enemy),
        &mut attrs,
        &mut stats,
        "",
    );
    assert!(
        stats.contains_key("mana_cost_reduction"),
        "the owning skill lost its own subtree bonus"
    );
}

#[test]
fn skill_scoped_subskill_stats_bypass_the_shared_map() {
    // charged_bolts/world_ender: of_total_damage 50 per rank, rank 3 → 150.
    let ranks: HashMap<String, u32> = HashMap::from([("charged_bolts:world_ender".to_string(), 3)]);
    let enemy = HashMap::new();
    let mut attrs = HashMap::new();
    let mut stats = HashMap::new();
    let subtree = apply_subskill_aggregation(
        Some("stormweaver"),
        Some("charged_bolts"),
        &ranks,
        Some(&enemy),
        &mut attrs,
        &mut stats,
        "",
    );
    assert!(!stats.contains_key("of_total_damage"));
    assert_eq!(
        subtree["charged_bolts"].scoped.get("of_total_damage"),
        Some(&(150.0, 150.0))
    );
}

#[test]
fn side_skill_overrides_swap_the_main_skill_subtree() {
    // Both charged_bolts and lightning_surge grant the shared
    // mana_cost_reduction; the lightning_surge card must see its own value,
    // not the main skill's.
    let ranks: HashMap<String, u32> = HashMap::from([
        ("charged_bolts:efficient_wiring".to_string(), 3),
        ("lightning_surge:storms_prophet".to_string(), 1),
    ]);
    let enemy = HashMap::new();
    let mut attrs = HashMap::new();
    let mut sources = HashMap::new();
    let subtree = apply_subskill_aggregation(
        Some("stormweaver"),
        Some("charged_bolts"),
        &ranks,
        Some(&enemy),
        &mut attrs,
        &mut sources,
        "",
    );
    let main_bonus = subtree["charged_bolts"].shared["mana_cost_reduction"];
    let surge_bonus = subtree["lightning_surge"].shared["mana_cost_reduction"];
    assert_ne!(main_bonus, surge_bonus);

    // Pretend the finalized map is 100 flat + the main skill's subtree.
    let stats: HashMap<String, Ranged> = HashMap::from([(
        "mana_cost_reduction".to_string(),
        (100.0 + main_bonus.0, 100.0 + main_bonus.1),
    )]);
    let overrides =
        per_skill_stat_overrides(Some("stormweaver"), Some("charged_bolts"), &subtree, &stats);
    assert!(
        !overrides.contains_key("charged_bolts"),
        "the main skill needs no override"
    );
    assert_eq!(
        overrides["lightning_surge"]["mana_cost_reduction"],
        (100.0 + surge_bonus.0, 100.0 + surge_bonus.1)
    );
    // A skill without a subtree drops the main skill's bonus entirely.
    let plain = overrides
        .iter()
        .find(|(id, _)| !subtree.contains_key(*id))
        .map(|(_, ov)| ov["mana_cost_reduction"]);
    assert_eq!(plain, Some((100.0, 100.0)));
}

#[test]
fn class_scoped_all_skills_only_pays_out_for_that_class() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let mut inventory: Inventory = HashMap::new();
    inventory.insert(
        "charm_1".to_string(),
        crate::calc::types::EquippedItem {
            base_id: "charm_satanic_engineer_s_mini_drone".to_string(),
            ..Default::default()
        },
    );
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
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
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );

    let marksman = compute_build_stats(&BuildStatsInput {
        class_id: Some("marksman"),
        ..base_input
    });
    assert_eq!(
        marksman.stats.get("all_skills").copied(),
        Some((1.0, 2.0)),
        "the charm's +[1-2] All Skills (Marksman) must reach a marksman"
    );
    assert!(
        !marksman.stats.contains_key("all_skills_marksman"),
        "the class-scoped key folds into all_skills instead of lingering"
    );

    let amazon = compute_build_stats(&BuildStatsInput {
        class_id: Some("amazon"),
        ..base_input
    });
    assert_eq!(
        amazon
            .stats
            .get("all_skills")
            .copied()
            .unwrap_or((0.0, 0.0)),
        (0.0, 0.0),
        "another class gets nothing from it"
    );
}

#[test]
fn class_labelled_set_bonus_only_pays_out_for_that_class() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let mut inventory: Inventory = HashMap::new();
    for (slot, base_id) in [
        ("amulet", "amulet_satanic_engineer_s_master_pendant"),
        ("belt", "belt_satanic_engineer_s_toolbelt"),
        ("armor", "body_armor_satanic_engineer_s_plated_vest"),
        ("charm_1", "charm_satanic_engineer_s_mini_drone"),
    ] {
        inventory.insert(
            slot.to_string(),
            crate::calc::types::EquippedItem {
                base_id: base_id.to_string(),
                ..Default::default()
            },
        );
    }
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
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
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );

    // +5 from the set, +[1-2] from the charm, +[2-3] from the pendant;
    // the vest's unlabelled +1 stays global.
    let marksman = compute_build_stats(&BuildStatsInput {
        class_id: Some("marksman"),
        ..base_input
    });
    assert_eq!(
        marksman.stats.get("all_skills").copied(),
        Some((9.0, 11.0)),
        "the 4-set bonus and the charm both reach a marksman"
    );

    let amazon = compute_build_stats(&BuildStatsInput {
        class_id: Some("amazon"),
        ..base_input
    });
    assert_eq!(
        amazon.stats.get("all_skills").copied(),
        Some((1.0, 1.0)),
        "another class in the whole set keeps only the unlabelled vest implicit"
    );
}

#[test]
fn hell_difficulty_drops_every_elemental_resistance() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
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
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let baseline = compute_build_stats(&base_input);
    let hell = compute_build_stats(&BuildStatsInput {
        difficulty: Some("hell"),
        ..base_input
    });

    for key in [
        "fire_resistance",
        "cold_resistance",
        "lightning_resistance",
        "poison_resistance",
        "arcane_resistance",
    ] {
        let before = baseline.stats.get(key).copied().unwrap_or((0.0, 0.0)).1;
        let after = hell.stats.get(key).copied().unwrap_or((0.0, 0.0)).1;
        assert!(
            (before - after - 45.0).abs() < 1e-6,
            "{key} should drop by 45 on Hell (before {before}, after {after})"
        );
    }
}

#[test]
fn incarnation_nodes_contribute_to_build_stats() {
    // S10: patch sezonu podmienia całe drzewo — nody liczą się przez to
    // samo allocated_tree_nodes.
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
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
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let baseline = compute_build_stats(&base_input);

    // S10 incarnation node 0: "+25 to Maximum Life", "+5 to Strength".
    let alloc_nodes: HashSet<u32> = [0].into_iter().collect();
    let input = BuildStatsInput {
        allocated_tree_nodes: &alloc_nodes,
        ..base_input
    };
    let with_node = compute_build_stats(&input);

    let base_life = baseline.stats.get("life").copied().unwrap_or((0.0, 0.0));
    let node_life = with_node.stats.get("life").copied().unwrap_or((0.0, 0.0));
    assert!(
        node_life.0 >= base_life.0 + 25.0 - 1e-6,
        "incarnation node 0 should add +25 max life (base {}, got {})",
        base_life.0,
        node_life.0,
    );
    let base_str = baseline
        .attributes
        .get("strength")
        .copied()
        .unwrap_or((0.0, 0.0));
    let node_str = with_node
        .attributes
        .get("strength")
        .copied()
        .unwrap_or((0.0, 0.0));
    assert!(
        node_str.0 >= base_str.0 + 5.0 - 1e-6,
        "incarnation node 0 should add +5 strength (base {}, got {})",
        base_str.0,
        node_str.0,
    );

    // Incarnation node 1868: "+3% Magic Damage taken Reduced" — an S10
    // phrasing variant; proves the S10 rule additions feed build stats.
    let alloc_s10: HashSet<u32> = [1868].into_iter().collect();
    let input_s10 = BuildStatsInput {
        allocated_tree_nodes: &alloc_s10,
        ..base_input
    };
    let with_s10_node = compute_build_stats(&input_s10);
    let base_mdr = baseline
        .stats
        .get("magic_damage_reduction")
        .copied()
        .unwrap_or((0.0, 0.0));
    let node_mdr = with_s10_node
        .stats
        .get("magic_damage_reduction")
        .copied()
        .unwrap_or((0.0, 0.0));
    assert!(
        node_mdr.0 >= base_mdr.0 + 3.0 - 1e-6,
        "incarnation node 1868 should add +3% magic damage reduction (base {}, got {})",
        base_mdr.0,
        node_mdr.0,
    );
}

// Node 2083 ("+20 Physical Damage while wielding a Dagger") must feed additive
// physical damage only while a Dagger occupies the weapon slot.
#[test]
fn light_radius_nodes_feed_magic_skill_damage() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
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
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let baseline = compute_build_stats(&base_input);

    // 1695/1696/1697: "+1 to Light Radius". 1687: "+1 to Magic Skill Damage
    // per points in Light Radius". 1694: the "+4%" wording of the same note.
    let alloc_nodes: HashSet<u32> = [1695, 1696, 1697, 1687, 1694].into_iter().collect();
    let input = BuildStatsInput {
        allocated_tree_nodes: &alloc_nodes,
        ..base_input
    };
    let with_nodes = compute_build_stats(&input);

    let get = |c: &ComputedStats, k: &str| c.stats.get(k).copied().unwrap_or((0.0, 0.0)).0;
    assert_eq!(get(&with_nodes, "light_radius"), 3.0);
    assert_eq!(
        get(&with_nodes, "magic_skill_damage") - get(&baseline, "magic_skill_damage"),
        12.0,
    );
    assert_eq!(
        get(&with_nodes, "flat_magic_skill_damage") - get(&baseline, "flat_magic_skill_damage"),
        3.0,
    );
}

#[test]
fn light_in_the_dark_turns_light_radius_into_attributes() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
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
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );

    // 1545/1546/1549/1550/1551: "+1 to Light Radius". 1552 "Light In The Dark":
    // "+3 to Light Radius" and "+40% of your Light Radius Added as Increased
    // All Attributes" — 8 points, so +3.2% increased all attributes.
    let alloc_nodes: HashSet<u32> = [1545, 1546, 1549, 1550, 1551, 1552].into_iter().collect();
    let input = BuildStatsInput {
        allocated_tree_nodes: &alloc_nodes,
        ..base_input
    };
    let with_nodes = compute_build_stats(&input);

    let get = |k: &str| with_nodes.stats.get(k).copied().unwrap_or((0.0, 0.0)).0;
    assert_eq!(get("light_radius"), 8.0);
    assert!((get("increased_all_attributes") - 3.2).abs() < 1e-9);
}

#[test]
fn dagger_conditional_lines_require_dagger_weapon() {
    // Node 2083 istnieje w danych S10 (patch sezonu podmienia drzewo).
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let node_2083: HashSet<u32> = [2083].into_iter().collect();
    let no_nodes: HashSet<u32> = HashSet::new();

    let additive_phys = |inventory: &Inventory, nodes: &HashSet<u32>| -> f64 {
        let base_input = empty_input(
            &allocated,
            inventory,
            &skill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &player_conditions,
            &subskill_ranks,
            &enemy_conditions,
        );
        let input = BuildStatsInput {
            allocated_tree_nodes: nodes,
            ..base_input
        };
        compute_build_stats(&input)
            .stats
            .get("additive_physical_damage")
            .copied()
            .unwrap_or((0.0, 0.0))
            .0
    };

    assert!(
        data::get_item("base_dagger_kris").is_some_and(|b| b.base_type == "Dagger"),
        "test expects a Dagger base in item data"
    );
    let bare: Inventory = HashMap::new();
    let mut with_dagger: Inventory = HashMap::new();
    with_dagger.insert(
        "weapon".to_string(),
        EquippedItem {
            base_id: "base_dagger_kris".to_string(),
            ..Default::default()
        },
    );

    let delta_no_weapon = additive_phys(&bare, &node_2083) - additive_phys(&bare, &no_nodes);
    assert!(
        delta_no_weapon.abs() < 1e-6,
        "dagger line must stay inert without a Dagger (delta {delta_no_weapon})"
    );
    let delta_dagger =
        additive_phys(&with_dagger, &node_2083) - additive_phys(&with_dagger, &no_nodes);
    assert!(
        (delta_dagger - 20.0).abs() < 1e-6,
        "node 2083 must add +20 physical damage with a Dagger equipped (delta {delta_dagger})"
    );
}

// Weapon-gated tree lines (wand/shield/two-handed) must stay inert unless the
// matching weapon kind is equipped; base-tier wands are typed "Spell" in data.
#[test]
fn weapon_conditional_nodes_require_matching_weapon() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();

    let stat = |inventory: &Inventory, nodes: &HashSet<u32>, key: &str| -> f64 {
        let base_input = empty_input(
            &allocated,
            inventory,
            &skill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &player_conditions,
            &subskill_ranks,
            &enemy_conditions,
        );
        let input = BuildStatsInput {
            allocated_tree_nodes: nodes,
            ..base_input
        };
        compute_build_stats(&input)
            .stats
            .get(key)
            .copied()
            .unwrap_or((0.0, 0.0))
            .0
    };
    let delta = |item: Option<(&str, &str)>, node: u32, key: &str| -> f64 {
        let mut inventory: Inventory = HashMap::new();
        if let Some((slot, base_id)) = item {
            inventory.insert(
                slot.to_string(),
                EquippedItem {
                    base_id: base_id.to_string(),
                    ..Default::default()
                },
            );
        }
        let nodes: HashSet<u32> = [node].into_iter().collect();
        stat(&inventory, &nodes, key) - stat(&inventory, &HashSet::new(), key)
    };

    // Node 1265: +8% to Faster Cast Rate while wielding a wand.
    assert_eq!(delta(None, 1265, "faster_cast_rate"), 0.0);
    assert_eq!(
        delta(
            Some(("weapon", "base_spell_wand")),
            1265,
            "faster_cast_rate"
        ),
        8.0
    );
    // Node 1270: +5% Increased Total Faster Cast Rate while wielding a wand.
    // The diminishing-returns pass folds _more into the base key, so read it there.
    assert_eq!(delta(None, 1270, "faster_cast_rate"), 0.0);
    assert_eq!(
        delta(
            Some(("weapon", "base_spell_wand")),
            1270,
            "faster_cast_rate"
        ),
        5.0
    );
    // Node 622: +15% Damage Mitigation when using a Shield.
    assert_eq!(delta(None, 622, "damage_mitigation"), 0.0);
    assert_eq!(
        delta(
            Some(("offhand", "shield_angelic_st_hallgar_s_bloodforged_aegis")),
            622,
            "damage_mitigation",
        ),
        15.0
    );
    // Node 349: +20% Increased Total Ailment Damage and +5% Increased Ailment
    // Frequency, both gated on a two-handed weapon.
    assert_eq!(delta(None, 349, "ailment_damage_all_more"), 0.0);
    assert_eq!(delta(None, 349, "increased_ailment_frequency"), 0.0);
    assert_eq!(
        delta(
            Some(("weapon", "base_mace_ogre_maul")),
            349,
            "ailment_damage_all_more"
        ),
        20.0
    );
    assert_eq!(
        delta(
            Some(("weapon", "base_mace_ogre_maul")),
            349,
            "increased_ailment_frequency",
        ),
        5.0
    );
}

// Merc-granted auras arrive as input.granted_skill_ranks and must apply
// the granted skill's per-rank passive stats without any worn item.
#[test]
fn granted_skill_ranks_from_input_apply_passive_stats() {
    let granted = data::item_granted_skills().iter().find(|g| {
        g.passive_stats
            .as_ref()
            .and_then(|p| p.per_rank.as_ref())
            .is_some_and(|m| !m.is_empty())
    });
    let Some(granted) = granted else { return };
    let per_rank = granted
        .passive_stats
        .as_ref()
        .unwrap()
        .per_rank
        .as_ref()
        .unwrap();
    let (stat_key, per) = per_rank.iter().next().unwrap();

    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let mut extra: HashMap<String, Ranged> = HashMap::new();
    extra.insert(granted.name.clone(), (10.0, 20.0));
    let mut input = empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    input.granted_skill_ranks = Some(&extra);
    let result = compute_build_stats(&input);

    let expected = (per * 10.0, per * 20.0);
    let found = result
        .stat_sources
        .get(stat_key)
        .into_iter()
        .flatten()
        .chain(result.attribute_sources.get(stat_key).into_iter().flatten())
        .any(|s| s.label.starts_with(granted.name.as_str()) && s.value == expected);
    assert!(
        found,
        "expected {expected:?} from '{}' on '{stat_key}'",
        granted.name
    );
}

#[test]
fn compute_build_stats_class_seeds_attributes() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();

    let any_class = data::data().classes.keys().next().cloned();
    let Some(class_id) = any_class else {
        eprintln!("no classes; skipping");
        return;
    };
    let mut input = empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    input.class_id = Some(&class_id);
    input.level = 10;

    let result = compute_build_stats(&input);
    assert!(!result.attributes.is_empty());
    // Every base attribute key from game-config should be present.
    for attr in data::game_config().attributes.iter() {
        assert!(
            result.attributes.contains_key(&attr.key),
            "missing attribute {}",
            attr.key
        );
    }
}

#[test]
fn compute_build_stats_skips_crit_rerun_when_no_tree_nodes() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree: HashSet<u32> = HashSet::new(); // empty → no re-run
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let input = empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let result = compute_build_stats(&input);
    // Without tree nodes, the re-run is skipped → result equals core's
    // first pass. Test just confirms no panic + valid output.
    assert!(!result.stats.is_empty());
}

#[test]
fn compute_build_stats_custom_stat_appears_in_output() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats = vec![CustomStat {
        stat_key: "fire_skill_damage".to_string(),
        value: "60".to_string(),
    }];
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let input = empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let result = compute_build_stats(&input);
    let fire_skill = result
        .stats
        .get("fire_skill_damage")
        .expect("custom fire_skill_damage source should land in stats");
    assert_eq!(*fire_skill, (60.0, 60.0));
}

// Node 683 "Accurate Cleaver": both lines are axe-gated and fold into different
// targets. "Increased Damage" goes to attack_damage, which multiplies the whole
// hit -- enhanced_damage would only scale the weapon roll.
#[test]
fn accurate_cleaver_folds_both_lines_only_with_an_axe() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let player_conditions = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let node_683: HashSet<u32> = [683].into_iter().collect();
    let no_nodes: HashSet<u32> = HashSet::new();

    let stat = |inventory: &Inventory, nodes: &HashSet<u32>, key: &str| -> f64 {
        let base_input = empty_input(
            &allocated,
            inventory,
            &skill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &player_conditions,
            &subskill_ranks,
            &enemy_conditions,
        );
        let input = BuildStatsInput {
            allocated_tree_nodes: nodes,
            ..base_input
        };
        compute_build_stats(&input)
            .stats
            .get(key)
            .copied()
            .unwrap_or((0.0, 0.0))
            .0
    };

    assert!(
        data::get_item("base_melee_hand_axe").is_some_and(|b| b.base_type == "Axe"),
        "test expects an Axe base in item data"
    );
    let bare: Inventory = HashMap::new();
    let mut with_axe: Inventory = HashMap::new();
    with_axe.insert(
        "weapon".to_string(),
        EquippedItem {
            base_id: "base_melee_hand_axe".to_string(),
            ..Default::default()
        },
    );

    for (key, expected) in [("attack_damage", 30.0), ("attack_rating_pct", 20.0)] {
        let bare_delta = stat(&bare, &node_683, key) - stat(&bare, &no_nodes, key);
        assert!(
            bare_delta.abs() < 1e-6,
            "{key} must stay inert without an Axe (delta {bare_delta})"
        );
        let axe_delta = stat(&with_axe, &node_683, key) - stat(&with_axe, &no_nodes, key);
        assert!(
            (axe_delta - expected).abs() < 1e-6,
            "node 683 must add +{expected} {key} with an Axe (delta {axe_delta})"
        );
    }
}

// Node 773 "Soulburn Essence": +1% Increased Magic Skills Damage per 750 points
// in Mana. Reads the finalized mana total, so it runs after the multiplier pass.
#[test]
fn soulburn_essence_scales_magic_skill_damage_with_mana() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = vec![CustomStat {
        stat_key: "mana".to_string(),
        value: "3000".to_string(),
    }];
    let alloc_tree = HashSet::new();
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
        &alloc_tree,
        &tree_socketed,
        &player_conditions,
        &subskill_ranks,
        &enemy_conditions,
    );
    let nodes: HashSet<u32> = [773].into_iter().collect();
    let out = compute_build_stats(&BuildStatsInput {
        allocated_tree_nodes: &nodes,
        ..base_input
    });

    let mana = out.stats.get("mana").copied().unwrap_or((0.0, 0.0)).1;
    let expected = (mana / 750.0).floor();
    assert!(
        expected >= 1.0,
        "test needs enough mana to clear one step, got {mana}"
    );
    assert_eq!(
        out.stats
            .get("magic_skill_damage")
            .copied()
            .unwrap_or((0.0, 0.0))
            .1,
        expected,
        "node 773 should grant 1% per full 750 mana ({mana} mana)"
    );
}

#[test]
fn set_sail_blessing_adds_cold_skill_damage_when_toggled() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let allocated = HashMap::new();
    let mut inventory: Inventory = HashMap::new();
    inventory.insert(
        "armor".to_string(),
        crate::calc::types::EquippedItem {
            base_id: "body_armor_heroic_tundra_hunter_s_long_coat".to_string(),
            ..Default::default()
        },
    );
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();

    let off_conditions = HashMap::new();
    let off = compute_build_stats(&empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &off_conditions,
        &subskill_ranks,
        &enemy_conditions,
    ));
    assert_eq!(
        off.stats.get("cold_skill_damage").copied(),
        Some((25.0, 40.0)),
        "untoggled the coat only pays its own implicit"
    );

    let mut on_conditions = HashMap::new();
    on_conditions.insert("set_sail_buff".to_string(), true);
    let on = compute_build_stats(&empty_input(
        &allocated,
        &inventory,
        &skill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &on_conditions,
        &subskill_ranks,
        &enemy_conditions,
    ));
    assert_eq!(
        on.stats.get("cold_skill_damage").copied(),
        Some((75.0, 115.0)),
        "Set Sail level 20-30 adds 2.5% per level on top"
    );
    assert_eq!(
        on.stats.get("mana_replenish_pct").copied(),
        Some((45.0, 65.0)),
    );
}

// #158: a relic proc buff (Shiv casts Live by the Sword 3-30) is a Config toggle.
#[test]
fn shiv_live_by_the_sword_buff_applies_when_toggled() {
    let allocated = HashMap::new();
    let mut inventory: Inventory = HashMap::new();
    inventory.insert(
        "relic_1".to_string(),
        crate::calc::types::EquippedItem {
            base_id: "relic_relic_shiv".to_string(),
            ..Default::default()
        },
    );
    let skill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let subskill_ranks = HashMap::new();
    let enemy_conditions = HashMap::new();
    let speed = |toggled: bool| {
        let conditions = HashMap::from([("live_by_the_sword_buff".to_string(), toggled)]);
        let out = compute_build_stats(&empty_input(
            &allocated,
            &inventory,
            &skill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &conditions,
            &subskill_ranks,
            &enemy_conditions,
        ));
        out.stats
            .get("movement_speed")
            .copied()
            .unwrap_or((0.0, 0.0))
    };
    let (off, on) = (speed(false), speed(true));
    assert_eq!((on.0 - off.0, on.1 - off.1), (31.0, 58.0));
}

#[test]
fn amazon_weapon_gates_allow_only_matching_weapons() {
    for (skill, node, weapon, key, amount) in [
        (
            "noxious_strike",
            "corrosive_reach",
            "Polearm",
            "weapon_subtree_damage",
            12.0,
        ),
        (
            "rebound",
            "marksmanship",
            "Bow",
            "skill_attack_speed_more",
            18.0,
        ),
        (
            "astropes_gift",
            "master_of_javelin",
            "Throwing",
            "projectile_count",
            3.0,
        ),
    ] {
        let ranks = HashMap::from([(format!("{skill}:{node}"), 3)]);
        for equipped in [weapon, "Wand"] {
            let mut attrs = HashMap::new();
            let mut stats = HashMap::new();
            let result = apply_subskill_aggregation(
                Some("amazon"),
                Some(skill),
                &ranks,
                None,
                &mut attrs,
                &mut stats,
                equipped,
            );
            let got = result[skill]
                .scoped
                .get(key)
                .or_else(|| result[skill].shared.get(key))
                .copied()
                .unwrap_or((0.0, 0.0));
            let expected = if equipped == weapon { amount } else { 0.0 };
            assert_eq!(got, (expected, expected), "{skill}/{equipped}");
        }
    }
}

// Timed buffs (rule 12/35): stats scale by min(1, D·(1 + 0.005·haste)/cd).
#[test]
fn timed_buff_stats_scale_by_uptime_with_skill_haste() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let tree = HashSet::new();
    let sockets = HashMap::new();
    let conditions = HashMap::new();
    let subskills = HashMap::new();
    let stat = |class, skill: &str, key, rank, haste: &str| {
        let ranks = HashMap::from([(skill.to_string(), rank)]);
        let buffs = HashMap::from([(skill.to_string(), true)]);
        let custom = vec![CustomStat {
            stat_key: "skill_haste".into(),
            value: haste.into(),
        }];
        let mut input = empty_input(
            &allocated, &inventory, &ranks, &buffs, &custom, &tree, &sockets, &conditions,
            &subskills, &conditions,
        );
        input.class_id = Some(class);
        input.level = 50;
        compute_build_stats(&input).stats.get(key).copied().unwrap_or((0.0, 0.0)).1
    };
    let gain = |class, skill, key, haste| {
        stat(class, skill, key, 1, haste) - stat(class, skill, key, 0, haste)
    };
    // Live by the Sword: 30 % movement speed, 25 s of a 70 s cooldown.
    let lbts = gain("samurai", "live_by_the_sword", "movement_speed", "0");
    assert!((lbts - 30.0 * 25.0 / 70.0).abs() < 0.01, "{lbts}");
    let hasted = gain("samurai", "live_by_the_sword", "movement_speed", "100");
    assert!((hasted - 30.0 * 37.5 / 70.0).abs() < 0.01, "{hasted}");
    // Beacon: arcane 20 % for (3.8 + 0.2·L + 2) s of 10 s, capped at full uptime.
    let beacon = gain("marksman", "beacon", "arcane_skill_damage", "0");
    assert!((beacon - 20.0 * 0.6).abs() < 0.01, "{beacon}");
    let capped = gain("marksman", "beacon", "arcane_skill_damage", "200");
    assert!((capped - 20.0).abs() < 0.01, "{capped}");
}
