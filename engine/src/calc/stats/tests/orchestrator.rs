use super::super::*;
use super::empty_input;
use crate::calc::types::EquippedItem;


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
    assert!(!result.stats.is_empty(), "default base stats should be present");
    // Base attributes from game-config seed the attribute map.
    assert!(!result.attributes.is_empty(), "attributes should be seeded from defaults");
}

#[test]
fn subskill_stats_reach_only_their_own_skill() {
    // charged_bolts/electric_vains grants lightning_skill_damage. It must
    // not buff lightning_surge, which has its own subtree.
    let ranks: HashMap<String, u32> =
        HashMap::from([("charged_bolts:electric_vains".to_string(), 5)]);
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
    );
    assert!(
        !stats.contains_key("lightning_skill_damage"),
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
    );
    assert!(
        stats.contains_key("lightning_skill_damage"),
        "the owning skill lost its own subtree bonus"
    );
}

#[test]
fn skill_scoped_subskill_stats_bypass_the_shared_map() {
    // charged_bolts/world_ender: of_total_damage 50 per rank, rank 3 → 150.
    let ranks: HashMap<String, u32> =
        HashMap::from([("charged_bolts:world_ender".to_string(), 3)]);
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
    );
    assert!(!stats.contains_key("of_total_damage"));
    assert_eq!(
        subtree["charged_bolts"].scoped.get("of_total_damage"),
        Some(&(150.0, 150.0))
    );
}

#[test]
fn side_skill_overrides_swap_the_main_skill_subtree() {
    // Both charged_bolts and lightning_surge grant lightning_skill_damage; the
    // lightning_surge card must see its own value, not the main skill's.
    let ranks: HashMap<String, u32> = HashMap::from([
        ("charged_bolts:electric_vains".to_string(), 5),
        ("lightning_surge:amplified_storm".to_string(), 2),
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
    );
    let main_bonus = subtree["charged_bolts"].shared["lightning_skill_damage"];
    let surge_bonus = subtree["lightning_surge"].shared["lightning_skill_damage"];
    assert_ne!(main_bonus, surge_bonus);

    // Pretend the finalized map is 100 flat + the main skill's subtree.
    let stats: HashMap<String, Ranged> = HashMap::from([(
        "lightning_skill_damage".to_string(),
        (100.0 + main_bonus.0, 100.0 + main_bonus.1),
    )]);
    let overrides =
        per_skill_stat_overrides(Some("stormweaver"), Some("charged_bolts"), &subtree, &stats);
    assert!(
        !overrides.contains_key("charged_bolts"),
        "the main skill needs no override"
    );
    assert_eq!(
        overrides["lightning_surge"]["lightning_skill_damage"],
        (100.0 + surge_bonus.0, 100.0 + surge_bonus.1)
    );
    // A skill without a subtree drops the main skill's bonus entirely.
    let plain = overrides
        .iter()
        .find(|(id, _)| !subtree.contains_key(*id))
        .map(|(_, ov)| ov["lightning_skill_damage"]);
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
        amazon.stats.get("all_skills").copied().unwrap_or((0.0, 0.0)),
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

    // 4-set "+5 to All Skills (Marksman)" + the charm's +[1-2] + the vest's
    // unlabelled +1 implicit, which stays global because the item data says so.
    let marksman = compute_build_stats(&BuildStatsInput {
        class_id: Some("marksman"),
        ..base_input
    });
    assert_eq!(
        marksman.stats.get("all_skills").copied(),
        Some((7.0, 8.0)),
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

    let delta_no_weapon =
        additive_phys(&bare, &node_2083) - additive_phys(&bare, &no_nodes);
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
        delta(Some(("weapon", "base_spell_wand")), 1265, "faster_cast_rate"),
        8.0
    );
    // Node 1270: +5% Increased Total Faster Cast Rate while wielding a wand.
    // The diminishing-returns pass folds _more into the base key, so read it there.
    assert_eq!(delta(None, 1270, "faster_cast_rate"), 0.0);
    assert_eq!(
        delta(Some(("weapon", "base_spell_wand")), 1270, "faster_cast_rate"),
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
        delta(Some(("weapon", "base_mace_ogre_maul")), 349, "ailment_damage_all_more"),
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
    assert!(expected >= 1.0, "test needs enough mana to clear one step, got {mana}");
    assert_eq!(
        out.stats.get("magic_skill_damage").copied().unwrap_or((0.0, 0.0)).1,
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
