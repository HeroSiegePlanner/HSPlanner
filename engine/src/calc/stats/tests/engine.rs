use super::super::*;
use crate::calc::types::{EquippedAffix, EquippedItem};

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
    let pick = data::data().skills_by_class.iter().find_map(|(class_id, skills)| {
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
    // Pick a real aura skill that contributes passive stats.
    let pick = data::data().skills_by_class.iter().find_map(|(class_id, skills)| {
        skills
            .iter()
            .find(|s| {
                s.kind == SkillKind::Aura
                    && s.passive_stats.as_ref().is_some_and(|p| {
                        p.base.as_ref().is_some_and(|b| !b.is_empty())
                            || p.per_rank.as_ref().is_some_and(|m| !m.is_empty())
                    })
            })
            .map(|s| (class_id.clone(), s.clone()))
    });
    let Some((class_id, aura)) = pick else {
        eprintln!("no aura with passive_stats in data; skipping");
        return;
    };

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
        (sum_contributions(at100.get("additive_physical_damage").unwrap()).0 - 100.0).abs()
            < 1e-9
    );
    assert!(
        (sum_contributions(at100.get("enhanced_damage").unwrap()).0 - 75.0).abs() < 1e-9
    );
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
    assert!(
        (sum_contributions(at92.get("enhanced_damage").unwrap()).0 - 69.0).abs() < 1e-9
    );
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
    apply_item_granted_passive_stats(
        &inv,
        Some(&extra),
        &on_cond,
        &mut on_attr,
        &mut on_stat,
    );
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
    let touched_off =
        apply_item_granted_conversions(&ranks, &stats, &HashMap::new(), &mut off);
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
    assert!((arcane.0 - 8.0).abs() < 1e-9, "arcane still gains it: {arcane:?}");
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
                data::game_config().attributes.first().map(|a| a.key.clone())
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
    let cold = stats.get("cold_skills").expect("picked element gets the ranks");
    assert!(cold.iter().any(|c| c.value == (4.0, 5.0)));
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
    let fire = stats.get("fire_skills").expect("picked element gets the ranks");
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
