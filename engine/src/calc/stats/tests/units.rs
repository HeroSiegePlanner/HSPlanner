use super::super::*;
use super::contrib;

// ---- augment application ----

// Every augment in augments.json must flow into stat aggregation: tier stats
// are added exactly, and out-of-range levels clamp to the last tier.
#[test]
fn augment_stats_apply_and_level_clamps() {
    use crate::calc::types::{AugmentRef, EquippedItem};

    let aug = data::get_augment("artful_dodger").expect("artful_dodger in augments.json");
    let last_tier = aug.levels.last().expect("levels present");
    let (key, &expected) = last_tier.stats.iter().next().expect("tier has stats");

    let base_id = data::data()
        .items
        .keys()
        .next()
        .expect("items present")
        .clone();
    let sum_for = |aug_ref: Option<AugmentRef>| {
        let mut inv: Inventory = HashMap::new();
        inv.insert(
            "armor".to_string(),
            EquippedItem {
                base_id: base_id.clone(),
                augment: aug_ref,
                ..Default::default()
            },
        );
        let mut attr: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_inventory(&inv, &mut attr, &mut stats);
        sum_contributions(stats.get(key).map(|v| v.as_slice()).unwrap_or(&[]))
    };

    let max_level = aug.levels.len() as u32;
    let without = sum_for(None);
    let with_max = sum_for(Some(AugmentRef {
        id: "artful_dodger".into(),
        level: max_level,
    }));
    assert_eq!(with_max.0 - without.0, expected);
    assert_eq!(with_max.1 - without.1, expected);

    let with_overflow = sum_for(Some(AugmentRef {
        id: "artful_dodger".into(),
        level: 99,
    }));
    assert_eq!(with_overflow, with_max);
}

// ---- is_zero ----

#[test]
fn is_zero_recognises_zero_and_nonzero() {
    assert!(is_zero((0.0, 0.0)));
    assert!(!is_zero((1.0, 1.0)));
    assert!(!is_zero((0.0, 1.0)));
    assert!(!is_zero((-1.0, 0.0)));
}

// ---- push_source ----

#[test]
fn push_source_skips_zero_and_appends() {
    let mut map: SourceMap = HashMap::new();
    push_source(&mut map, "life", contrib((0.0, 0.0)));
    assert!(map.is_empty(), "zero value should be skipped");
    push_source(&mut map, "life", contrib((10.0, 20.0)));
    assert_eq!(map.get("life").map(|v| v.len()), Some(1));
    push_source(&mut map, "life", contrib((5.0, 5.0)));
    assert_eq!(map.get("life").map(|v| v.len()), Some(2));
}

// ---- sum_contributions ----

#[test]
fn sum_contributions_adds_endpoints() {
    let list = vec![
        contrib((10.0, 20.0)),
        contrib((5.0, 5.0)),
        contrib((-2.0, -1.0)),
    ];
    assert_eq!(sum_contributions(&list), (13.0, 24.0));
    assert_eq!(sum_contributions(&[]), (0.0, 0.0));
}

#[test]
fn sum_ranged_from_map_floors_endpoints() {
    let mut map: SourceMap = HashMap::new();
    push_source(&mut map, "life", contrib((10.5, 20.7)));
    push_source(&mut map, "life", contrib((5.5, 5.5)));
    // sum = (16.0, 26.2); floored → (16, 26).
    assert_eq!(sum_ranged_from_map(&map, "life"), (16.0, 26.0));
    assert_eq!(sum_ranged_from_map(&map, "missing"), (0.0, 0.0));
}

// ---- stats_combined_map ----

#[test]
fn stats_combined_only_emits_keys_with_more_twin() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("faster_cast_rate".into(), (10.0, 10.0));
    stats.insert("faster_cast_rate_more".into(), (50.0, 50.0));
    stats.insert("life".into(), (100.0, 100.0));
    let out = stats_combined_map(&stats);
    // (1.10 * 1.50 - 1) * 100 = 65
    assert_eq!(out.get("faster_cast_rate"), Some(&(65.0, 65.0)));
    assert!(!out.contains_key("life"));
    assert!(!out.contains_key("faster_cast_rate_more"));
    assert_eq!(out.len(), 1);
}

#[test]
fn stats_combined_more_without_base_uses_zero_additive() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("mana_replenish_more".into(), (25.0, 30.0));
    let out = stats_combined_map(&stats);
    assert_eq!(out.get("mana_replenish"), Some(&(25.0, 30.0)));
}

#[test]
fn stats_combined_matches_combine_additive_and_more() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("enhanced_damage".into(), (12.5, 20.0));
    stats.insert("enhanced_damage_more".into(), (33.0, 40.0));
    let out = stats_combined_map(&stats);
    assert_eq!(
        out.get("enhanced_damage"),
        Some(&combine_additive_and_more((12.5, 20.0), (33.0, 40.0)))
    );
}

// ---- compute_item_effective_defense ----

#[test]
fn effective_defense_none_when_base_missing() {
    assert_eq!(compute_item_effective_defense(None, None, None), None);
    assert_eq!(compute_item_effective_defense(Some(10.0), None, None), None);
    assert_eq!(compute_item_effective_defense(None, Some(10.0), None), None);
}

#[test]
fn effective_defense_no_enhancement_floors_to_base() {
    assert_eq!(
        compute_item_effective_defense(Some(10.0), Some(20.0), None),
        Some((10.0, 20.0))
    );
}

#[test]
fn effective_defense_applies_enhanced_percent() {
    // base 10..20, +50% min .. +100% max → 15..40
    assert_eq!(
        compute_item_effective_defense(Some(10.0), Some(20.0), Some((50.0, 100.0))),
        Some((15.0, 40.0))
    );
    // fractional product (10 * 1.33 = 13.3) gets floor'd
    assert_eq!(
        compute_item_effective_defense(Some(10.0), Some(10.0), Some((33.0, 33.0))),
        Some((13.0, 13.0))
    );
}

// ---- combine_additive_and_more ----

#[test]
fn combine_additive_only() {
    assert_eq!(
        combine_additive_and_more((50.0, 50.0), (0.0, 0.0)),
        (50.0, 50.0)
    );
}

#[test]
fn combine_more_only() {
    assert_eq!(
        combine_additive_and_more((0.0, 0.0), (30.0, 30.0)),
        (30.0, 30.0)
    );
}

#[test]
fn combine_additive_and_more_compounds() {
    // 50% additive × 30% more → (1.5)(1.3)-1 = 95%
    let v = combine_additive_and_more((50.0, 50.0), (30.0, 30.0));
    assert!((v.0 - 95.0).abs() < 1e-9, "got {v:?}");
    assert!((v.1 - 95.0).abs() < 1e-9, "got {v:?}");
}

#[test]
fn combine_ranged_endpoints_independent() {
    // min: (1.5)(1.3)-1 = 95   max: (2.0)(1.5)-1 = 200
    let v = combine_additive_and_more((50.0, 100.0), (30.0, 50.0));
    assert!((v.0 - 95.0).abs() < 1e-9);
    assert!((v.1 - 200.0).abs() < 1e-9);
}

// ---- apply_multiplier ----

#[test]
fn apply_multiplier_no_flat_is_noop() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    apply_multiplier(&mut stats, "life", Some("increased_life"), None, true);
    assert!(stats.is_empty());
}

#[test]
fn apply_multiplier_zero_pct_and_more_skips() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("life".to_string(), (100.0, 100.0));
    apply_multiplier(
        &mut stats,
        "life",
        Some("increased_life"),
        Some("increased_life_more"),
        true,
    );
    assert_eq!(stats.get("life"), Some(&(100.0, 100.0)));
}

#[test]
fn apply_multiplier_additive_floors() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("life".to_string(), (100.0, 100.0));
    stats.insert("increased_life".to_string(), (50.0, 50.0));
    apply_multiplier(&mut stats, "life", Some("increased_life"), None, true);
    assert_eq!(stats.get("life"), Some(&(150.0, 150.0)));
}

#[test]
fn apply_multiplier_additive_plus_more_compounds() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("life".to_string(), (100.0, 100.0));
    stats.insert("increased_life".to_string(), (50.0, 50.0));
    stats.insert("increased_life_more".to_string(), (30.0, 30.0));
    apply_multiplier(
        &mut stats,
        "life",
        Some("increased_life"),
        Some("increased_life_more"),
        true,
    );
    // 100 * 1.5 * 1.3 = 195
    assert_eq!(stats.get("life"), Some(&(195.0, 195.0)));
}

#[test]
fn apply_multiplier_floor_false_preserves_fractional() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("mana_replenish".to_string(), (3.0, 3.0));
    stats.insert("mana_replenish_more".to_string(), (15.0, 15.0));
    apply_multiplier(
        &mut stats,
        "mana_replenish",
        None,
        Some("mana_replenish_more"),
        false,
    );
    // 3 * 1.15 = 3.45 — no floor
    let v = stats.get("mana_replenish").copied().unwrap();
    assert!((v.0 - 3.45).abs() < 1e-9);
    assert!((v.1 - 3.45).abs() < 1e-9);
}

#[test]
fn multipliers_pass_scales_ailment_durations() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("frostbite_duration".to_string(), (6.0, 6.0));
    stats.insert("frostbite_duration_pct".to_string(), (8.0, 8.0));
    stats.insert("shadowburn_duration".to_string(), (7.0, 7.0));
    stats.insert("shadowburn_duration_pct".to_string(), (50.0, 50.0));
    // no pct sources for stasis — must stay untouched
    stats.insert("stasis_duration".to_string(), (5.0, 5.0));
    apply_multipliers_pass(&mut stats);
    // (5 + 1s) * 1.08 = 6.48 — fractional seconds survive (no floor)
    let f = stats.get("frostbite_duration").copied().unwrap();
    assert!((f.0 - 6.48).abs() < 1e-9 && (f.1 - 6.48).abs() < 1e-9);
    // 7 * 1.5 = 10.5
    let s = stats.get("shadowburn_duration").copied().unwrap();
    assert!((s.0 - 10.5).abs() < 1e-9 && (s.1 - 10.5).abs() < 1e-9);
    assert_eq!(stats.get("stasis_duration"), Some(&(5.0, 5.0)));
}

// ---- STAT_FAN_OUTS ----

#[test]
fn stat_fan_outs_present() {
    assert_eq!(STAT_FAN_OUTS.len(), 2);
    let (all_res_key, all_res_targets) = STAT_FAN_OUTS[0];
    assert_eq!(all_res_key, "all_resistances");
    assert_eq!(all_res_targets.len(), 5);
    let (max_key, max_targets) = STAT_FAN_OUTS[1];
    assert_eq!(max_key, "max_all_resistances");
    assert_eq!(max_targets.len(), 5);
}

// ---- stat_def ----

#[test]
fn stat_def_exact_lookup_resolves_known_key() {
    let def = stat_def("all_skills");
    assert!(def.is_some(), "all_skills should be defined");
    assert_eq!(def.unwrap().key, "all_skills");
}

#[test]
fn stat_def_falls_back_to_base_for_more_suffix() {
    // Pick any key that exists in game-config and verify _more falls back
    // to the same base def.
    if let Some(base) = stat_def("all_skills") {
        let synthesised = stat_def("all_skills_more");
        assert!(synthesised.is_some());
        assert_eq!(synthesised.unwrap().key, base.key);
    }
}

#[test]
fn stat_def_unknown_key_returns_none() {
    assert!(stat_def("definitely_not_a_stat").is_none());
    assert!(stat_def("definitely_not_a_stat_more").is_none());
}

// ---- apply_contribution ----

#[test]
fn apply_contribution_zero_value_is_noop() {
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_contribution(
        &mut attrs,
        &mut stats,
        "life",
        (0.0, 0.0),
        "src".to_string(),
        SourceType::Item,
        None,
    );
    assert!(attrs.is_empty());
    assert!(stats.is_empty());
}

#[test]
fn apply_contribution_routes_modifies_attribute_to_attrs() {
    // `to_strength` has modifiesAttribute = 'strength' in game-config.
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_contribution(
        &mut attrs,
        &mut stats,
        "to_strength",
        (5.0, 5.0),
        "ring".to_string(),
        SourceType::Item,
        None,
    );
    assert!(
        attrs.contains_key("strength"),
        "should route to strength bucket"
    );
    assert!(stats.is_empty());
}

#[test]
fn apply_contribution_normal_key_routes_to_stats() {
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_contribution(
        &mut attrs,
        &mut stats,
        "fire_resistance",
        (30.0, 30.0),
        "ring".to_string(),
        SourceType::Item,
        None,
    );
    assert!(attrs.is_empty());
    assert!(stats.contains_key("fire_resistance"));
    let entries = stats.get("fire_resistance").unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source_type, SourceType::Item);
    assert_eq!(entries[0].value, (30.0, 30.0));
}

// ---- apply_inventory ----

use crate::calc::types::EquippedItem;
use crate::calc::data;

#[test]
fn apply_inventory_empty_is_noop() {
    let inv: Inventory = HashMap::new();
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    let weapon_aps = apply_inventory(&inv, &mut attrs, &mut stats);
    assert!(!weapon_aps);
    assert!(attrs.is_empty());
    assert!(stats.is_empty());
}

#[test]
fn apply_inventory_unknown_base_is_skipped() {
    let mut inv: Inventory = HashMap::new();
    inv.insert(
        "weapon".to_string(),
        EquippedItem {
            base_id: "definitely_not_a_real_item".to_string(),
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    let weapon_aps = apply_inventory(&inv, &mut attrs, &mut stats);
    assert!(!weapon_aps);
    assert!(attrs.is_empty());
    assert!(stats.is_empty());
}

#[test]
fn apply_inventory_pushes_defense_source_for_armor_with_defense_range() {
    // Needs an armor item with defense but no enhanced_defense implicit, else
    // scaling makes the floor(base) assertion below diverge from the pushed value.
    let any_def_item = data::data()
        .items
        .values()
        .find(|i| {
            i.slot == "armor"
                && i.defense_min.map(|v| v > 0.0).unwrap_or(false)
                && i.defense_max.map(|v| v > 0.0).unwrap_or(false)
                && !i
                    .implicit
                    .as_ref()
                    .map(|m| m.contains_key("enhanced_defense"))
                    .unwrap_or(false)
        })
        .cloned();
    let Some(item_base) = any_def_item else {
        eprintln!("no defense-armor item in data; skipping");
        return;
    };

    let mut inv: Inventory = HashMap::new();
    inv.insert(
        "armor".to_string(),
        EquippedItem {
            base_id: item_base.id.clone(),
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    apply_inventory(&inv, &mut attrs, &mut stats);

    let def_entries = stats.get("defense").expect("expected defense source");
    assert_eq!(def_entries.len(), 1);
    // The pushed range matches base.defense_min..defense_max (no enhanced_defense).
    let (lo, hi) = def_entries[0].value;
    assert_eq!(lo, item_base.defense_min.unwrap().floor());
    assert_eq!(hi, item_base.defense_max.unwrap().floor());
    assert_eq!(def_entries[0].source_type, SourceType::Item);
}

#[test]
fn apply_inventory_weapon_slot_with_weapon_item_flags_attack_speed() {
    let any_weapon = data::data()
        .items
        .values()
        .find(|i| {
            i.slot == "weapon"
                && (i.attack_speed.is_some()
                    || i.implicit
                        .as_ref()
                        .is_some_and(|m| m.contains_key("attacks_per_second")))
        })
        .cloned();
    let Some(weapon_base) = any_weapon else {
        eprintln!("no weapon with attack_speed in data; skipping");
        return;
    };

    let mut inv: Inventory = HashMap::new();
    inv.insert(
        "weapon".to_string(),
        EquippedItem {
            base_id: weapon_base.id.clone(),
            ..Default::default()
        },
    );
    let mut attrs: SourceMap = HashMap::new();
    let mut stats: SourceMap = HashMap::new();
    let weapon_aps = apply_inventory(&inv, &mut attrs, &mut stats);
    assert!(
        weapon_aps,
        "weapon item with APS should flip weapon_has_attack_speed"
    );
}


// ---- diminishing returns ----

fn dr(threshold: f64, power: f64, cap: Option<f64>) -> crate::calc::types::DiminishDef {
    crate::calc::types::DiminishDef { threshold, power, cap }
}

// User-verified: 564 raw skill haste lands exactly on the 200 hard cap.
#[test]
fn diminished_value_matches_game_example() {
    let def = dr(100.0, 0.75, Some(200.0));
    let eff = diminished_value(564.0, &def);
    assert!((eff - 200.0).abs() < 0.05, "564 raw skill haste -> {eff}, expected ~200");
}

#[test]
fn diminished_value_below_threshold_is_identity() {
    let def = dr(350.0, 0.8, None);
    assert_eq!(diminished_value(300.0, &def), 300.0);
    assert_eq!(diminished_value(-20.0, &def), -20.0);
    assert_eq!(diminished_value(350.0, &def), 350.0);
}

#[test]
fn diminished_value_applies_hard_cap() {
    let def = dr(200.0, 0.86, Some(600.0));
    assert_eq!(diminished_value(5000.0, &def), 600.0);
}

#[test]
fn apply_diminishing_returns_folds_more_twin_and_zeroes_it() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("faster_cast_rate".to_string(), (300.0, 300.0));
    stats.insert("faster_cast_rate_more".to_string(), (50.0, 50.0));
    let mut defs = HashMap::new();
    defs.insert("faster_cast_rate".to_string(), dr(350.0, 0.8, None));
    let raw = apply_diminishing_returns(&mut stats, &defs);
    // combined raw = (1+3)*(1+0.5)-1 = 500% -> 350 + 150^0.8 ~= 405.06
    let eff = stats["faster_cast_rate"];
    assert!((eff.1 - 405.06).abs() < 0.05, "500% raw FCR -> {}, expected ~405.06", eff.1);
    assert_eq!(stats["faster_cast_rate_more"], (0.0, 0.0));
    // pre-diminish total exposed for the UI's "eff (raw)" display
    assert_eq!(raw["faster_cast_rate"], (500.0, 500.0));
}

#[test]
fn apply_diminishing_returns_skips_unconfigured_and_absent_keys() {
    let mut stats: HashMap<String, Ranged> = HashMap::new();
    stats.insert("crit_chance".to_string(), (900.0, 900.0));
    let mut defs = HashMap::new();
    defs.insert("gold_find".to_string(), dr(200.0, 0.75, None));
    apply_diminishing_returns(&mut stats, &defs);
    assert_eq!(stats["crit_chance"], (900.0, 900.0));
    assert!(!stats.contains_key("gold_find"));
}

// S10 config carries the four original curves plus FCR, deadly blow and skill haste.
#[test]
fn diminishing_returns_config_has_s10_curves() {
    let s10 = crate::calc::data::data_for("s10").game_config.diminishing_returns.as_ref().unwrap();
    assert_eq!(s10.len(), 7);
    let db = &s10["deadly_blow"];
    assert_eq!((db.threshold, db.power, db.cap), (50.0, 0.75, Some(100.0)));
}
