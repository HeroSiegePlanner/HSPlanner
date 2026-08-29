use super::super::*;
use crate::calc::types::EquippedItem;

// ---- tree contributions ----

#[test]
fn apply_tree_contributions_empty_set_returns_empty() {
    let alloc: HashSet<u32> = HashSet::new();
    let conds: HashMap<String, bool> = HashMap::new();
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    let agg = apply_tree_contributions(&alloc, &conds, &mut attrs, &mut stats);
    assert!(agg.conversions.is_empty());
    assert!(agg.disables.is_empty());
    assert!(attrs.is_empty());
    assert!(stats.is_empty());
}

#[test]
fn apply_tree_contributions_skips_jewelry_nodes() {
    // Pick a jewelry node id from data — its mod lines must NOT be parsed
    // here (jewelry contributions come from the socket function instead).
    let jewelry_id = data::tree_jewelry_ids().iter().next().copied();
    let Some(node_id) = jewelry_id else {
        eprintln!("no jewelry nodes in tree data; skipping");
        return;
    };
    let mut alloc = HashSet::new();
    alloc.insert(node_id);
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    let agg = apply_tree_contributions(&alloc, &HashMap::new(), &mut attrs, &mut stats);
    assert!(agg.conversions.is_empty());
    assert!(agg.disables.is_empty());
    assert!(attrs.is_empty());
    assert!(stats.is_empty());
}

#[test]
fn apply_tree_contributions_pushes_parseable_mod_line() {
    // Find any non-jewelry node whose first line parses successfully.
    let jewelry = data::tree_jewelry_ids();
    let pick = data::tree_nodes().iter().find_map(|(id_str, info)| {
        if info.kind == "jewelry" || info.lines.is_empty() {
            return None;
        }
        let id: u32 = id_str.parse().ok()?;
        if jewelry.contains(&id) {
            return None;
        }
        for line in &info.lines {
            if parse_tree_node_mod(line).is_some() {
                return Some(id);
            }
        }
        None
    });
    let Some(node_id) = pick else {
        eprintln!("no parseable tree node found in data; skipping");
        return;
    };
    let mut alloc = HashSet::new();
    alloc.insert(node_id);
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_tree_contributions(&alloc, &HashMap::new(), &mut attrs, &mut stats);
    let total_sources = attrs.values().map(|v| v.len()).sum::<usize>()
        + stats.values().map(|v| v.len()).sum::<usize>();
    assert!(
        total_sources >= 1,
        "expected at least one contribution from a parseable tree node"
    );
}

#[test]
fn spell_branch_node_gates_magic_skill_damage_to_spells() {
    let mut alloc = HashSet::new();
    alloc.insert(777); // Manahunger, a Spell-branch notable
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_tree_contributions(&alloc, &HashMap::new(), &mut attrs, &mut stats);
    assert!(stats.contains_key("spell_damage_more"));
    assert!(!stats.contains_key("magic_skill_damage_more"));
}

#[test]
fn apply_tree_jewelry_sockets_empty_alloc_is_noop() {
    let alloc: HashSet<u32> = HashSet::new();
    let socketed: HashMap<u32, TreeSocketContent> = HashMap::new();
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_tree_jewelry_sockets(&alloc, &socketed, &mut attrs, &mut stats);
    assert!(attrs.is_empty());
    assert!(stats.is_empty());
}

#[test]
fn apply_tree_jewelry_sockets_skips_non_jewelry_allocations() {
    // Allocating a non-jewelry node — even with a TreeSocketContent
    // wrongly placed at that id — must not push anything.
    let any_non_jewelry = data::tree_nodes().iter().find_map(|(id_str, info)| {
        if info.kind == "jewelry" {
            return None;
        }
        id_str.parse::<u32>().ok()
    });
    let Some(node_id) = any_non_jewelry else {
        eprintln!("no non-jewelry nodes; skipping");
        return;
    };
    let mut alloc = HashSet::new();
    alloc.insert(node_id);
    let mut socketed = HashMap::new();
    socketed.insert(
        node_id,
        TreeSocketContent::Item {
            id: "nonexistent_id".to_string(),
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_tree_jewelry_sockets(&alloc, &socketed, &mut attrs, &mut stats);
    assert!(attrs.is_empty());
    assert!(stats.is_empty());
}

// ---- base attributes ----

#[test]
fn apply_base_attributes_includes_default_base_when_present() {
    // game-config defines defaultBaseAttributes for every base attribute; without
    // a class only those defaults populate attr_sources.
    let mut attrs: SourceMap = HashMap::new();
    let allocated: HashMap<String, u32> = HashMap::new();
    apply_base_attributes(None, &allocated, &mut attrs);
    // At least one attribute should have a "Base character" source.
    let has_default = attrs.values().flatten().any(|c| c.label == "Base character");
    assert!(has_default, "expected 'Base character' source");
}

#[test]
fn apply_base_attributes_appends_allocated() {
    let mut attrs: SourceMap = HashMap::new();
    let mut allocated = HashMap::new();
    allocated.insert("strength".to_string(), 12_u32);
    apply_base_attributes(None, &allocated, &mut attrs);
    let str_sources = attrs.get("strength").expect("strength bucket missing");
    let alloc_entry = str_sources
        .iter()
        .find(|c| c.label == "Allocated points")
        .expect("missing allocated source");
    assert_eq!(alloc_entry.value, (12.0, 12.0));
    assert_eq!(alloc_entry.source_type, SourceType::Allocated);
}

#[test]
fn apply_base_attributes_with_real_class_id_succeeds() {
    // Pick any class — the function must not panic and must populate at
    // least one attribute bucket.
    let any_class = data::data().classes.keys().next().cloned();
    let Some(class_id) = any_class else {
        eprintln!("no classes in data; skipping");
        return;
    };
    let mut attrs: SourceMap = HashMap::new();
    apply_base_attributes(Some(&class_id), &HashMap::new(), &mut attrs);
    assert!(!attrs.is_empty());
}

// ---- set bonuses ----

#[test]
fn apply_set_bonuses_empty_inventory_is_noop() {
    let inv: Inventory = HashMap::new();
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_set_bonuses(&inv, &mut attrs, &mut stats);
    assert!(attrs.is_empty());
    assert!(stats.is_empty());
}

#[test]
fn apply_set_bonuses_applies_when_threshold_met() {
    // Set bonuses trigger on the base item's `setId`, not sets.json `items[].itemId`
    // (UI only) — so group by setId and pick a set covering a <=2-piece bonus.
    let mut by_set: HashMap<String, Vec<String>> = HashMap::new();
    for item in data::data().items.values() {
        if let Some(set_id) = item.set_id.as_deref() {
            by_set
                .entry(set_id.to_string())
                .or_default()
                .push(item.id.clone());
        }
    }
    let pick = by_set.iter().find_map(|(set_id, item_ids)| {
        if item_ids.len() < 2 {
            return None;
        }
        let set = data::get_set(set_id)?;
        let qualifying = set
            .bonuses
            .iter()
            .find(|b| b.pieces <= 2 && !b.stats.is_empty())?;
        Some((set_id.clone(), item_ids.clone(), qualifying.pieces))
    });
    let Some((set_id, item_ids, pieces)) = pick else {
        eprintln!("no setId with 2+ items + 2-piece bonus available; skipping");
        return;
    };

    let mut inv: Inventory = HashMap::new();
    for (i, item_id) in item_ids.iter().take(pieces as usize).enumerate() {
        inv.insert(
            format!("set_slot_{i}"),
            EquippedItem {
                base_id: item_id.clone(),
                ..Default::default()
            },
        );
    }
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_set_bonuses(&inv, &mut attrs, &mut stats);
    let total = attrs.values().map(|v| v.len()).sum::<usize>()
        + stats.values().map(|v| v.len()).sum::<usize>();
    assert!(
        total > 0,
        "expected at least one bonus source from set '{}' with {} items",
        set_id,
        pieces
    );
}

// ---- class baseline + per-level ----

#[test]
fn apply_class_baseline_seeds_defaults_only_without_class() {
    let mut stats: SourceMap = HashMap::new();
    apply_class_baseline(None, 1, false, &mut stats);
    // game-config has at least crit_chance / crit_damage / etc. defaults.
    let has_default = stats.values().flatten().any(|c| c.label == "Base character");
    assert!(has_default, "expected default base-stat sources");
}

#[test]
fn apply_class_baseline_suppresses_aps_when_weapon_provides_it() {
    let mut stats: SourceMap = HashMap::new();
    apply_class_baseline(None, 1, true, &mut stats);
    // The 'Base character' source for attacks_per_second must be absent.
    let has_default_aps = stats
        .get("attacks_per_second")
        .map(|list| list.iter().any(|c| c.label == "Base character"))
        .unwrap_or(false);
    assert!(!has_default_aps, "default APS should be suppressed");
}

#[test]
fn apply_class_baseline_per_level_multiplies() {
    // Find a class with at least one statsPerLevel entry.
    let class = data::data()
        .classes
        .values()
        .find(|c| c.stats_per_level.values().any(|&v| v != 0.0))
        .cloned();
    let Some(cls) = class else {
        eprintln!("no class with stats_per_level; skipping");
        return;
    };
    let (stat_key, &per_level) = cls
        .stats_per_level
        .iter()
        .find(|(_, &v)| v != 0.0)
        .unwrap();

    let mut stats: SourceMap = HashMap::new();
    apply_class_baseline(Some(&cls.id), 10, false, &mut stats);
    let expected = per_level * 10.0;
    let level_source = stats
        .get(stat_key)
        .and_then(|list| {
            list.iter().find(|c| {
                c.source_type == SourceType::Level
                    && (c.value.0 - expected).abs() < 1e-9
                    && (c.value.1 - expected).abs() < 1e-9
            })
        });
    assert!(
        level_source.is_some(),
        "expected per-level source on '{stat_key}' for class '{}'",
        cls.id
    );
}

// ---- custom stats ----

#[test]
fn apply_custom_stats_routes_via_stat_def() {
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    let customs = vec![
        CustomStat {
            stat_key: "crit_chance".to_string(),
            value: "15".to_string(),
        },
        CustomStat {
            stat_key: "fire_skill_damage".to_string(),
            value: "50-80".to_string(),
        },
        // Empty key skipped.
        CustomStat {
            stat_key: "".to_string(),
            value: "10".to_string(),
        },
        // Garbage value skipped.
        CustomStat {
            stat_key: "crit_chance".to_string(),
            value: "abc".to_string(),
        },
    ];
    apply_custom_stats(&customs, &mut attrs, &mut stats);

    let crit = stats.get("crit_chance").expect("crit_chance missing");
    assert_eq!(crit.len(), 1);
    assert_eq!(crit[0].value, (15.0, 15.0));
    assert_eq!(crit[0].source_type, SourceType::Custom);
    assert_eq!(crit[0].label, CUSTOM_SOURCE_LABEL);

    let fire = stats.get("fire_skill_damage").expect("fire_skill missing");
    assert_eq!(fire[0].value, (50.0, 80.0));
}

// ---- skill ranks ----

#[test]
fn apply_skill_ranks_no_class_id_is_noop() {
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_skill_ranks(
        None,
        &HashMap::new(),
        None,
        &HashMap::new(),
        &HashMap::new(),
        &mut attrs,
        &mut stats,
    );
    assert!(attrs.is_empty());
    assert!(stats.is_empty());
}

#[test]
fn apply_skill_ranks_with_unknown_class_is_noop() {
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_skill_ranks(
        Some("nonexistent_class"),
        &HashMap::new(),
        None,
        &HashMap::new(),
        &HashMap::new(),
        &mut attrs,
        &mut stats,
    );
    assert!(attrs.is_empty());
    assert!(stats.is_empty());
}

