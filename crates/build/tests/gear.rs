use hsplanner_build::{BuildSnapshot, gear, session::Draft};
use hsplanner_engine::calc::{data, types::SocketType};

#[test]
fn forge_removal_trims_the_extra_socket_and_preserves_other_rolls() {
    let base = data::data()
        .items
        .values()
        .find(|b| b.rarity == "common" && b.max_sockets.is_some_and(|n| (1..6).contains(&n)))
        .unwrap();
    let mut item = gear::make_item(&base.id).unwrap();
    let base_max = gear::max_sockets(&item);
    gear::set_forge(&mut item, Some("crystal_add_socket")).unwrap();
    gear::set_socket_count(&mut item, 99);
    assert_eq!(item.socket_count, base_max + 1);
    let gem = data::data().gems.values().next().unwrap();
    gear::set_socket(&mut item, 0, Some(&gem.id)).unwrap();
    item.socket_types[0] = SocketType::Rainbow;
    item.implicit_overrides.insert("strength".into(), 42.);
    gear::set_forge(&mut item, None).unwrap();
    assert_eq!(item.socket_count, base_max);
    assert_eq!(item.socketed.len(), base_max as usize);
    assert_eq!(item.socketed[0].as_deref(), Some(gem.id.as_str()));
    assert_eq!(item.socket_types[0], SocketType::Rainbow);
    assert_eq!(item.implicit_overrides["strength"], 42.);
    assert!(gear::set_socket(&mut item, base_max as usize, None).is_err());
}

#[test]
fn runewords_use_real_ordered_runes_and_do_not_create_invalid_unique_sockets() {
    let (base, word) = data::data()
        .items
        .values()
        .filter(|b| b.rarity == "common")
        .find_map(|base| {
            let item = gear::make_item(&base.id).unwrap();
            data::data()
                .runewords
                .iter()
                .find(|rw| {
                    rw.allowed_base_types.contains(&base.base_type)
                        && rw.runes.len() <= gear::max_sockets(&item) as usize
                })
                .map(|rw| (base, rw))
        })
        .unwrap();
    let mut item = gear::make_item(&base.id).unwrap();
    gear::apply_runeword(&mut item, &word.id).unwrap();
    assert_eq!(
        item.socketed,
        word.runes.iter().cloned().map(Some).collect::<Vec<_>>()
    );
    assert_eq!(item.socket_count as usize, word.runes.len());
    let unique = data::data()
        .items
        .values()
        .find(|b| b.rarity != "common" && b.max_sockets == Some(0))
        .unwrap();
    let mut item = gear::make_item(&unique.id).unwrap();
    gear::set_forge(&mut item, Some("crystal_add_socket")).unwrap();
    gear::set_socket_count(&mut item, 6);
    assert_eq!(item.socket_count, 0);
    assert!(gear::apply_runeword(&mut item, &word.id).is_err());
}

#[test]
fn losing_grip_removes_the_offhand_and_gear_preview_uses_the_same_rule() {
    let grip = data::data()
        .tree_nodes
        .iter()
        .find(|(_, node)| {
            node.lines
                .iter()
                .chain(node.note.iter())
                .any(|line| line.contains("Dual Wield") && line.contains("Melee Weapons"))
        })
        .map(|(id, _)| id.parse::<u32>().unwrap());
    // Locate by the same wording used by the reference's dual-wield rule.
    let grip = grip
        .or_else(|| {
            data::data()
                .tree_nodes
                .iter()
                .find(|(_, node)| {
                    node.lines.iter().chain(node.note.iter()).any(|line| {
                        let line = line.to_lowercase();
                        line.contains("dual wield")
                            && (line.contains("melee weapons")
                                || line.contains("swords, maces and axes"))
                    })
                })
                .map(|(id, _)| id.parse().unwrap())
        })
        .unwrap();
    let base = data::data()
        .items
        .values()
        .find(|b| b.two_handed == Some(true) && b.base_type == "Sword")
        .unwrap();
    let mut snapshot = BuildSnapshot {
        allocated_tree_nodes: vec![grip],
        ..Default::default()
    };
    let item = gear::make_item(&base.id).unwrap();
    gear::commit(&mut snapshot, "weapon", Some(item.clone()), false, true).unwrap();
    gear::commit(&mut snapshot, "offhand", Some(item.clone()), false, true).unwrap();
    assert!(snapshot.inventory.contains_key("offhand"));
    snapshot.set_tree_nodes(&[]);
    assert!(!snapshot.inventory.contains_key("offhand"));
    let before = serde_json::to_value(&snapshot).unwrap();
    assert!(gear::commit(&mut snapshot, "offhand", Some(item), false, true).is_err());
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), before);
}

#[test]
fn stash_copies_are_independent_deduplicated_and_bounded() {
    let base = data::data()
        .items
        .values()
        .find(|base| !gear::is_relic(base))
        .unwrap();
    let mut item = gear::make_item(&base.id).unwrap();
    let mut draft = Draft::default();
    gear::stash(&mut draft, &item);
    let id = draft.stash[0].id.clone();
    gear::stash(&mut draft, &item);
    assert_eq!(draft.stash.len(), 1);
    item.stars = Some(5);
    assert_eq!(draft.stash[0].item.stars, Some(0));
    for roll in 0..205 {
        item.implicit_overrides.insert("roll".into(), roll as f64);
        gear::stash(&mut draft, &item);
    }
    assert_eq!(draft.stash.len(), 200);
    assert!(!draft.stash.iter().any(|entry| entry.id == id));
}

#[test]
fn relics_are_never_stashed() {
    let relic = data::data()
        .items
        .values()
        .find(|base| gear::is_relic(base))
        .unwrap();
    let item = gear::make_item(&relic.id).unwrap();
    let mut draft = Draft::default();
    gear::stash(&mut draft, &item);
    assert!(draft.stash.is_empty());
}

#[test]
fn charm_capacity_respects_the_unlock_and_rectangular_items() {
    let units: Vec<_> = (1..=30).map(|n| (format!("charm_{n}"), 1, 1)).collect();
    assert!(gear::pack_charms(&units, true).is_empty());
    assert_eq!(gear::pack_charms(&units, false).len(), 1);
    let big: Vec<_> = (1..=11).map(|n| (format!("charm_{n}"), 1, 3)).collect();
    assert!(!gear::pack_charms(&big, true).is_empty());
    assert_eq!(
        gear::pack_charms(&[("invalid".into(), 4, 1)], true),
        ["invalid"]
    );
}

#[test]
fn crystal_roll_pins_a_value_and_survives_serialization() {
    let base = data::data()
        .items
        .values()
        .find(|b| b.rarity == "common" && b.slot == "armor")
        .unwrap();
    let mut item = gear::make_item(&base.id).unwrap();
    gear::set_forge(&mut item, Some("crystal_satanic_all_attributes")).unwrap();
    item.implicit_overrides.insert("strength".into(), 42.);
    assert_eq!(item.forged_mods[0].custom_value, None);
    for (requested, expected) in [(10., 10.), (18., 18.), (100., 25.)] {
        assert!(gear::set_forge_value(&mut item, 0, requested));
        let eq = &item.forged_mods[0];
        assert_eq!(eq.custom_value, Some(expected));
        assert_eq!(eq.affix_id, "crystal_satanic_all_attributes");
        assert!((0.0..=1.0).contains(&eq.roll));
        assert_eq!(item.implicit_overrides["strength"], 42.);
    }
    let saved = serde_json::to_string(&item).unwrap();
    let restored: hsplanner_engine::calc::types::EquippedItem =
        serde_json::from_str(&saved).unwrap();
    assert_eq!(restored.forged_mods[0].custom_value, Some(25.));
    assert!(!gear::set_forge_value(&mut item, 1, 15.));
    assert!(!gear::set_forge_value(&mut item, 0, f64::NAN));
}

#[test]
fn relic_tier_pins_every_ranged_stat_and_reads_back() {
    let mut item = gear::make_item("relic_relic_1000_kg").unwrap();
    assert_eq!(gear::relic_tier(&item), gear::RELIC_MAX_TIER);
    assert!(gear::set_relic_tier(&mut item, 1));
    assert_eq!(item.skill_bonus_overrides["Heavy Weight"], 2.);
    assert_eq!(gear::relic_tier(&item), 1);
    assert!(gear::set_relic_tier(&mut item, 10));
    assert_eq!(item.skill_bonus_overrides["Heavy Weight"], 50.);
    assert!(gear::set_relic_tier(&mut item, 5));
    assert_eq!(item.skill_bonus_overrides["Heavy Weight"], 23.);
    assert_eq!(gear::relic_tier(&item), 5);
    assert!(!gear::set_relic_tier(&mut item, 5));
    assert_eq!(gear::max_sockets(&item), 0);
}

#[test]
fn the_spoon_tier_scales_increased_mana_from_3_to_30() {
    let mut item = gear::make_item("relic_relic_the_spoon").unwrap();
    gear::set_relic_tier(&mut item, 1);
    assert_eq!(item.implicit_overrides["increased_mana"], 3.);
    gear::set_relic_tier(&mut item, 10);
    assert_eq!(item.implicit_overrides["increased_mana"], 30.);
}

#[test]
fn relic_tiers_preserve_fractional_stats_and_negative_scaling() {
    let mut sausage = gear::make_item("relic_relic_sausage").unwrap();
    gear::set_relic_tier(&mut sausage, 1);
    assert_eq!(sausage.implicit_overrides["increased_life"], 1.5);
    for tier in 1..=10 {
        gear::set_relic_tier(&mut sausage, tier);
        assert_eq!(gear::relic_tier(&sausage), tier);
    }
    let mut sword = gear::make_item("relic_relic_commander_s_sword").unwrap();
    gear::set_relic_tier(&mut sword, 1);
    assert_eq!(sword.implicit_overrides["movement_speed"], -2.);
    gear::set_relic_tier(&mut sword, 10);
    assert_eq!(sword.implicit_overrides["movement_speed"], -20.);
    assert_eq!(gear::relic_tier(&sword), 10);
}

#[test]
fn refreshed_relics_have_source_stats_without_neighbour_contamination() {
    let butterfly = data::get_item("relic_relic_butterfly_knife").unwrap();
    let stats = butterfly.implicit.as_ref().unwrap();
    assert_eq!(stats["increased_attack_speed"].as_ranged(), (1., 10.));
    assert_eq!(stats["crit_damage"].as_ranged(), (2., 20.));
    let globe = data::get_item("relic_relic_arcane_globe").unwrap();
    let stats = globe.implicit.as_ref().unwrap();
    assert_eq!(stats.len(), 2);
    assert_eq!(stats["arcane_skills"].as_ranged(), (1., 5.));
    assert!(!stats.contains_key("poison_skills"));
    let apple = data::get_item("relic_relic_apple").unwrap();
    assert!(apple.implicit.is_none());
    assert!(apple.unique_effects.as_ref().unwrap()[0].contains("Apple Blast"));
    let sock = data::get_item("relic_relic_mayo_s_old_sock").unwrap();
    assert_eq!(
        sock.implicit.as_ref().unwrap()["life"].as_ranged(),
        (10., 600.)
    );
    for id in ["snowball", "thief_s_glove", "soul_box"] {
        assert!(gear::make_item(&format!("relic_relic_{id}")).is_ok());
    }
    assert_eq!(
        data::get_item("relic_relic_devil_horn").unwrap().name,
        "Devil Horn"
    );
}
