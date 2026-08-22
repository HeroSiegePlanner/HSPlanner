use super::*;

// ---- classify_tree_node_line ----

#[test]
fn classify_stat_line_returns_stat_with_key() {
    match classify_tree_node_line("+10 to Maximum Life") {
        TreeLineClass::Stat(m) => {
            assert_eq!(m.key, "life");
            assert_eq!(m.value, 10.0);
        }
        _ => panic!("expected Stat"),
    }
}

// S10 trade-off phrasings carry the malus sign inside the number; the
// rules must not flip or drop it.
#[test]
fn s10_trade_off_lines_keep_negative_sign() {
    match classify_tree_node_line("-6% Reduced Movement Speed") {
        TreeLineClass::Stat(m) => {
            assert_eq!(m.key, "movement_speed");
            assert_eq!(m.value, -6.0);
        }
        _ => panic!("expected Stat"),
    }
    match classify_tree_node_line("-10% Damage Reduction") {
        TreeLineClass::Stat(m) => {
            assert_eq!(m.key, "physical_damage_reduction");
            assert_eq!(m.value, -10.0);
        }
        _ => panic!("expected Stat"),
    }
}

#[test]
fn classify_null_rule_line_is_recognized_no_stat() {
    assert!(matches!(
        classify_tree_node_line("Path to any Black Hole"),
        TreeLineClass::RecognizedNoStat
    ));
}

#[test]
fn classify_null_rule_conversion_line_is_meta() {
    // Rejected as a stat by a null rule but still a conversion the engine
    // applies — must classify as parsed (Meta), not silently dropped.
    assert!(matches!(
        classify_tree_node_line("12% of Resistances converted to Life"),
        TreeLineClass::Meta(ParsedMeta::Convert(_))
    ));
}

#[test]
fn classify_conversion_line_is_meta() {
    match classify_tree_node_line("20% of Physical Damage converted to Fire") {
        TreeLineClass::Meta(ParsedMeta::Convert(c)) => {
            assert_eq!(c.pct, 20.0);
        }
        _ => panic!("expected Meta(Convert)"),
    }
}

#[test]
fn classify_gibberish_is_unknown() {
    assert!(matches!(
        classify_tree_node_line("Totally not a real mod line"),
        TreeLineClass::Unknown
    ));
}

// Regression for rules the TS parser had but Rust initially missed
// (found by the one-shot TS↔Rust classification diff).
#[test]
fn rule_seconds_unit_and_crushing_blow_alternative() {
    let cases = [
        ("+0.1 s Cooldown Recovered", "cooldown_recovered_flat", 0.1),
        ("+2 s Stun Duration", "stun_duration", 2.0),
        ("+2 s Evasion Duration", "evasion_duration", 2.0),
        ("+3% Chance for a Crushing Blow", "crushing_blow_chance", 3.0),
        ("+2 Stun Duration", "stun_duration", 2.0),
    ];
    for (line, key, value) in cases {
        let parsed = parse_tree_node_mod(line)
            .unwrap_or_else(|| panic!("line must parse: {line}"));
        assert_eq!(parsed.key, key, "line: {line}");
        assert_eq!(parsed.value, value, "line: {line}");
    }
}

// Penalty prefix "+-N" mirrors JS Number(): strip '+', keep the negative.
#[test]
fn rule_penalty_prefix_parses_negative() {
    let parsed = parse_tree_node_mod("+-10% to All Resistances").expect("parses");
    assert_eq!(parsed.key, "all_resistances");
    assert_eq!(parsed.value, -10.0);
}

#[test]
fn classify_agrees_with_parse_tree_node_mod_on_stats() {
    let line = "+5 to Strength";
    let parsed = parse_tree_node_mod(line).expect("parses");
    match classify_tree_node_line(line) {
        TreeLineClass::Stat(m) => {
            assert_eq!(m.key, parsed.key);
            assert_eq!(m.value, parsed.value);
        }
        _ => panic!("expected Stat"),
    }
}

// ---- num ----

#[test]
fn num_parses_signed_and_unsigned() {
    assert_eq!(num("5"), 5.0);
    assert_eq!(num("+12"), 12.0);
    assert_eq!(num("-3"), -3.0);
    assert_eq!(num("0"), 0.0);
    assert_eq!(num("1.5"), 1.5);
    assert_eq!(num("+0.25"), 0.25);
}

#[test]
fn num_invalid_returns_nan() {
    assert!(num("abc").is_nan());
    assert!(num("").is_nan());
    assert!(num("--5").is_nan());
}

// ---- strip_weapon_context ----

#[test]
fn strip_weapon_context_removes_suffix() {
    let line = "+10 to Strength when wielding a sword";
    assert_eq!(strip_weapon_context(line), "+10 to Strength");
    let two = "+50% Critical Strike Damage while using a two handed melee weapon";
    assert_eq!(strip_weapon_context(two), "+50% Critical Strike Damage");
    let bow = "+15 to Dexterity while wielding a bow";
    assert_eq!(strip_weapon_context(bow), "+15 to Dexterity");
}

#[test]
fn strip_weapon_context_unchanged_when_no_suffix() {
    let line = "+10 to Strength";
    assert_eq!(strip_weapon_context(line), line);
    // Suffix in middle of string, not at end → no strip.
    let mid = "while using a sword you get nothing";
    assert_eq!(strip_weapon_context(mid), mid);
}

// ---- strip_self_condition ----

#[test]
fn strip_self_condition_crit_chance() {
    let (base, cond) = strip_self_condition(
        "+30% Critical Strike Damage when critical strike chance is below 40%",
    );
    assert_eq!(base, "+30% Critical Strike Damage");
    assert_eq!(cond, Some(SelfConditionKey::CritChanceBelow40));
}

#[test]
fn strip_self_condition_life_below_40() {
    let (b1, c1) = strip_self_condition("+20% Movement Speed while below 40% maximum life");
    assert_eq!(b1, "+20% Movement Speed");
    assert_eq!(c1, Some(SelfConditionKey::LifeBelow40));

    let (b2, c2) = strip_self_condition(
        "+10 to Strength when current life is below 40% of maximum life",
    );
    assert_eq!(b2, "+10 to Strength");
    assert_eq!(c2, Some(SelfConditionKey::LifeBelow40));
}

#[test]
fn strip_self_condition_no_match_returns_unchanged() {
    let (base, cond) = strip_self_condition("+5 to Strength");
    assert_eq!(base, "+5 to Strength");
    assert_eq!(cond, None);
}

// ---- SelfConditionKey ----

#[test]
fn self_condition_key_string_and_label() {
    assert_eq!(SelfConditionKey::CritChanceBelow40.as_str(), "crit_chance_below_40");
    assert_eq!(SelfConditionKey::LifeBelow40.as_str(), "life_below_40");
    assert_eq!(
        SelfConditionKey::CritChanceBelow40.label(),
        "Critical Strike Chance is below 40% (auto)"
    );
    assert_eq!(SELF_CONDITION_KEYS.len(), 2);
}

// ---- dispatchers ----

fn assert_mod(line: &str, key: &str, value: f64) {
    let actual = parse_tree_node_mod(line);
    assert_eq!(
        actual,
        Some(ParsedMod {
            key: key.to_string(),
            value,
            self_condition: None,
        }),
        "input: {line}"
    );
}

fn assert_mod_with_cond(line: &str, key: &str, value: f64, cond: SelfConditionKey) {
    let actual = parse_tree_node_mod(line);
    assert_eq!(
        actual,
        Some(ParsedMod {
            key: key.to_string(),
            value,
            self_condition: Some(cond),
        }),
        "input: {line}"
    );
}

#[test]
fn rule_life_and_mana() {
    assert_mod("+10 to Maximum Life", "life", 10.0);
    assert_mod("+250 to Maximum Mana", "mana", 250.0);
    assert_mod("5% Increased Maximum Life", "increased_life", 5.0);
    assert_mod("5% Increased Total Maximum Life", "increased_life_more", 5.0);
    assert_mod("8% Increased Maximum Mana", "increased_mana", 8.0);
    assert_mod("5% Increased Total Maximum Mana", "increased_mana_more", 5.0);
    assert_mod("12% Increased Mana", "increased_mana", 12.0);
}

#[test]
fn rule_attributes_flat() {
    assert_mod("+5 to Strength", "to_strength", 5.0);
    assert_mod("+5 Strength", "to_strength", 5.0);
    assert_mod("+7 to Dexterity", "to_dexterity", 7.0);
    assert_mod("+7 Dexterity", "to_dexterity", 7.0);
    assert_mod("+10 to Intelligence", "to_intelligence", 10.0);
    assert_mod("+3 to Energy", "to_energy", 3.0);
    assert_mod("+4 to Vitality", "to_vitality", 4.0);
    assert_mod("+2 to Armor", "to_armor", 2.0);
    assert_mod("+8 to All Attributes", "all_attributes", 8.0);
}

#[test]
fn rule_attributes_increased() {
    assert_mod("5% Increased All Attributes", "increased_all_attributes", 5.0);
    assert_mod("10% Increased Strength", "increased_strength", 10.0);
    assert_mod("10% Increased Total Strength", "increased_strength_more", 10.0);
    assert_mod("5% Increased Dexterity", "increased_dexterity", 5.0);
    assert_mod("5% Increased Total Dexterity", "increased_dexterity_more", 5.0);
    assert_mod("3% Increased Intelligence", "increased_intelligence", 3.0);
    assert_mod("4% Increased Energy", "increased_energy", 4.0);
    assert_mod("6% Increased Vitality", "increased_vitality", 6.0);
    assert_mod("2% Increased Armor", "increased_armor", 2.0);
}

#[test]
fn rule_defense_and_speed() {
    assert_mod("+50 to Defense", "defense", 50.0);
    assert_mod("10% Increased Movement Speed", "movement_speed", 10.0);
    assert_mod("10% Increased Total Movement Speed", "movement_speed_more", 10.0);
    assert_mod("15% Increased Attack Speed", "increased_attack_speed", 15.0);
    assert_mod(
        "15% Increased Total Attack Speed",
        "increased_attack_speed_more",
        15.0,
    );
    assert_mod("8% Increased Faster Cast Rate", "faster_cast_rate", 8.0);
    assert_mod(
        "8% Increased Total Faster Cast Rate",
        "faster_cast_rate_more",
        8.0,
    );
    assert_mod("5% Spell Haste", "skill_haste", 5.0);
    assert_mod("5% to Spell Haste", "skill_haste", 5.0);
}

#[test]
fn rule_weapon_conditional_lines_use_dedicated_keys() {
    assert_mod(
        "+8% to Faster Cast Rate while wielding a wand",
        "faster_cast_rate_with_wand",
        8.0,
    );
    assert_mod(
        "+5% Increased Total Faster Cast Rate while wielding a wand",
        "faster_cast_rate_more_with_wand",
        5.0,
    );
    assert_mod(
        "+8 to Maximum Damage when wielding a shield",
        "attack_damage_with_shield",
        8.0,
    );
    assert_mod("+8 to Maximum Damage", "attack_damage", 8.0);
    assert_mod(
        "+15% Damage Mitigation when using a Shield",
        "damage_mitigation_with_shield",
        15.0,
    );
    assert_mod(
        "+50 to Damage Returned when wielding a Shield",
        "damage_return_with_shield",
        50.0,
    );
    assert_mod(
        "+10% Increased Total Critical Strike Damage when using a Shield",
        "crit_damage_more_with_shield",
        10.0,
    );
    assert_mod(
        "+30% Increased Melee Attack Range when using a Shield",
        "melee_range_with_shield",
        30.0,
    );
    assert_mod(
        "+3% Increased Ailment Damage when using a Two Handed Weapon",
        "ailment_damage_all_with_two_handed",
        3.0,
    );
    assert_mod(
        "+20% Increased Total Ailment Damage when using a Two Handed Weapon",
        "ailment_damage_all_more_with_two_handed",
        20.0,
    );
    assert_mod(
        "+5% Increased Ailment Frequency when using a Two Handed Weapon",
        "increased_ailment_frequency_with_two_handed",
        5.0,
    );
    assert_mod(
        "+8% Increased Melee Attack Damage when using a Two Handed Melee Weapon",
        "damage_with_two_handed_melee",
        8.0,
    );
    assert_mod(
        "+50% to Enhanced Damage when using Bow",
        "enhanced_damage_with_bow",
        50.0,
    );
    assert_mod(
        "+50% to Enhanced Damage when using Gun",
        "enhanced_damage_with_gun",
        50.0,
    );
    assert_mod(
        "+50% to Enhanced Damage when using Throwing Weapon",
        "enhanced_damage_with_throwing",
        50.0,
    );
    assert_mod(
        "+50% to Enhanced Damage when using Axe",
        "damage_with_axe",
        50.0,
    );
    assert_mod(
        "5% Increased Total Spell Projectile Damage",
        "spell_projectile_damage_more",
        5.0,
    );
    assert_mod(
        "+8% Increased Spell Projectile Damage when wielding a Staff or a Cane",
        "two_handed_spell_projectile_damage",
        8.0,
    );
}

#[test]
fn rule_crit() {
    assert_mod("30% Increased Critical Strike Damage", "crit_damage", 30.0);
    assert_mod(
        "30% Increased Total Critical Strike Damage",
        "crit_damage_more",
        30.0,
    );
    assert_mod("25% Critical Damage", "crit_damage", 25.0);
    assert_mod("3% to Critical Strike Chance", "crit_chance", 3.0);
    assert_mod(
        "5% Chance to Critically Hit with Spells",
        "spell_crit_chance",
        5.0,
    );
}

#[test]
fn rule_signed_negative_values() {
    assert_mod("-10 to Maximum Life", "life", -10.0);
    assert_mod("-5 to Strength", "to_strength", -5.0);
    assert_mod("-3% Increased Mana", "increased_mana", -3.0);
}

#[test]
fn rule_with_self_condition() {
    assert_mod_with_cond(
        "30% Increased Critical Strike Damage when critical strike chance is below 40%",
        "crit_damage",
        30.0,
        SelfConditionKey::CritChanceBelow40,
    );
    assert_mod_with_cond(
        "20% Increased Movement Speed while below 40% maximum life",
        "movement_speed",
        20.0,
        SelfConditionKey::LifeBelow40,
    );
}

#[test]
fn rule_unmatched_returns_none() {
    assert_eq!(parse_tree_node_mod("This is not a known mod line"), None);
    assert_eq!(parse_tree_node_mod(""), None);
    // Wisdom is not in the attribute list — should miss.
    assert_eq!(parse_tree_node_mod("+10 to Wisdom"), None);
}

#[test]
fn dispatcher_caches_negative_results() {
    // Second call should hit the cache. We can't directly observe the cache
    // here, but we can confirm the result is stable across repeats.
    let line = "this will never match anything";
    assert_eq!(parse_tree_node_mod(line), None);
    assert_eq!(parse_tree_node_mod(line), None);
    assert_eq!(parse_tree_node_meta(line), None);
    assert_eq!(parse_tree_node_meta(line), None);
}

// ---- ELEMENTS / CONVERSION_TARGET_STATS smoke ----

#[test]
fn elements_const_matches_ts() {
    assert_eq!(ELEMENTS, &["arcane", "cold", "fire", "lightning", "poison"]);
}

#[test]
fn conversion_target_stats_has_expected_keys() {
    assert_eq!(
        CONVERSION_TARGET_STATS.get("magic skill damage").copied(),
        Some("magic_skill_damage")
    );
    assert_eq!(
        CONVERSION_TARGET_STATS.get("ranged physical damage").copied(),
        Some("ranged_physical_per_500_mana")
    );
    assert_eq!(CONVERSION_TARGET_STATS.get("unknown stat"), None);
}

// ---- element-parameterized rules ----

#[test]
fn rule_element_resistances() {
    assert_mod("+30% to Fire Resistance", "fire_resistance", 30.0);
    assert_mod("+25% to Cold Resistance", "cold_resistance", 25.0);
    assert_mod("+40% to Lightning Resistance", "lightning_resistance", 40.0);
    assert_mod("+15% to Poison Resistance", "poison_resistance", 15.0);
    assert_mod("+20% to Arcane Resistance", "arcane_resistance", 20.0);
    assert_mod("+5% to Maximum Fire Resistance", "max_fire_resistance", 5.0);
    assert_mod("+3% to Cold Absorb", "cold_absorption", 3.0);
}

#[test]
fn rule_element_skill_damage() {
    assert_mod("30% Increased Fire Skill Damage", "fire_skill_damage", 30.0);
    assert_mod(
        "30% Increased Total Fire Skill Damage",
        "fire_skill_damage_more",
        30.0,
    );
    assert_mod("25% Increased Lightning Skill Damage", "lightning_skill_damage", 25.0);
    assert_mod("+5 to Fire Skill Damage", "flat_fire_skill_damage", 5.0);
    assert_mod("+3 to Cold Skill Damage", "flat_cold_skill_damage", 3.0);
    assert_mod("+4 to Magic Skill Damage", "flat_magic_skill_damage", 4.0);
}

#[test]
fn rule_to_element_skills_flat() {
    assert_mod("+2 to Fire Skills", "fire_skills", 2.0);
    assert_mod("+3 to Cold Skills", "cold_skills", 3.0);
    assert_mod("+1 to Arcane Skills", "arcane_skills", 1.0);
}

#[test]
fn rule_all_resistances_variants() {
    assert_mod("+15% All Resistances", "all_resistances", 15.0);
    assert_mod("+15% to All Resistances", "all_resistances", 15.0);
    assert_mod("+20 to All Resistances", "all_resistances", 20.0);
    assert_mod("+20 to Total All Resistances", "all_resistances_more", 20.0);
    assert_mod("+20% to Total All Resistances", "all_resistances_more", 20.0);
    assert_mod("+5% to Maximum All Resistances", "max_all_resistances", 5.0);
}

// ---- null/fixed/cond rules ----

#[test]
fn null_rule_explicitly_rejects_socketable_slot() {
    // TS returns null → dispatcher aborts and caches None.
    assert_eq!(parse_tree_node_mod("1 Socketable Slot"), None);
}

#[test]
fn null_rule_path_to_black_hole_and_resistances_to_life() {
    assert_eq!(parse_tree_node_mod("Path to any Black Hole"), None);
    assert_eq!(parse_tree_node_mod("+0 Path to any Black Hole"), None);
    // Resistances → Life appears in BOTH RULES (null) and CONVERSION_RULES (meta).
    // parse_tree_node_mod aborts due to null rule. parse_tree_node_meta succeeds.
    assert_eq!(parse_tree_node_mod("50% of Resistances converted to Life"), None);
    let meta = parse_tree_node_meta("50% of Resistances converted to Life");
    assert!(matches!(meta, Some(ParsedMeta::Convert(_))));
}

#[test]
fn fixed_rule_flag_like_mods() {
    assert_mod(
        "You can now dual wield Two Handed Melee Weapons",
        "dual_wield_2h_melee",
        1.0,
    );
    assert_mod(
        "You can no longer dodge monster attacks but also cannot be stunned or frozen",
        "force_field_protection",
        1.0,
    );
    assert_mod(
        "Your skill weapon type restrictions are removed",
        "skill_restrictions_removed",
        1.0,
    );
}

#[test]
fn cond_rule_baked_in_self_condition() {
    // The cond_rule! macro sets self_condition in the build closure itself,
    // which the dispatcher preserves alongside the stripped outer condition.
    assert_mod_with_cond(
        "30% Increased Ranged Projectile Damage when Critical Strike Chance is below 40%",
        "ranged_projectile_damage",
        30.0,
        SelfConditionKey::CritChanceBelow40,
    );
    assert_mod_with_cond(
        "25% Increased Physical Damage while below 40% Maximum Life",
        "enhanced_damage",
        25.0,
        SelfConditionKey::LifeBelow40,
    );
}

// ---- meta dispatcher (CONVERSION_RULES + DISABLE_RULES) ----

fn assert_convert(
    line: &str,
    from_key: &str,
    from_kind: ConvertKind,
    to_key: &str,
    to_kind: ConvertKind,
    pct: f64,
) {
    match parse_tree_node_meta(line) {
        Some(ParsedMeta::Convert(c)) => {
            assert_eq!(c.from_key, from_key, "from_key mismatch on: {line}");
            assert_eq!(c.from_kind, from_kind, "from_kind mismatch on: {line}");
            assert_eq!(c.to_key, to_key, "to_key mismatch on: {line}");
            assert_eq!(c.to_kind, to_kind, "to_kind mismatch on: {line}");
            assert_eq!(c.pct, pct, "pct mismatch on: {line}");
        }
        other => panic!("expected Convert, got {other:?} for: {line}"),
    }
}

#[test]
fn meta_attribute_to_stat_conversion() {
    assert_convert(
        "10% of Strength converted to maximum life",
        "strength",
        ConvertKind::Attribute,
        "life",
        ConvertKind::Stat,
        10.0,
    );
    assert_convert(
        "5% of Dexterity is added as attack damage",
        "dexterity",
        ConvertKind::Attribute,
        "attack_damage",
        ConvertKind::Stat,
        5.0,
    );
}

#[test]
fn meta_element_conversion() {
    assert_convert(
        "20% of your Fire Resistance is converted to Increased Fire Skill Damage",
        "fire_resistance",
        ConvertKind::Stat,
        "fire_skill_damage",
        ConvertKind::Stat,
        20.0,
    );
    assert_convert(
        "30% of Physical Damage converted to fire",
        "additive_physical_damage",
        ConvertKind::Stat,
        "physical_to_fire",
        ConvertKind::Stat,
        30.0,
    );
}

#[test]
fn meta_weapon_specific_enhanced_damage() {
    // Weapon-gated enhanced damage is a flat conditional stat now, not a
    // conversion; folded into enhanced_damage when the weapon matches.
    assert_mod(
        "40% to Enhanced Damage when using Axe",
        "damage_with_axe",
        40.0,
    );
    assert_eq!(
        parse_tree_node_meta("50% to Enhanced Damage when using Bow"),
        None
    );
}

#[test]
fn meta_unsupported_attribute_target_returns_none() {
    // Attribute-conversion rule with target outside CONVERSION_TARGET_STATS
    // returns None from build → dispatcher skips that rule.
    assert_eq!(
        parse_tree_node_meta("10% of Strength converted to Made Up Stat"),
        None
    );
}

#[test]
fn meta_disable_life_replenish() {
    match parse_tree_node_meta("You cannot regenerate life from life replenish anymore") {
        Some(ParsedMeta::Disable(d)) => {
            assert_eq!(d.target, DisableTarget::LifeReplenish);
        }
        other => panic!("expected Disable, got {other:?}"),
    }
}

#[test]
fn meta_unmatched_returns_none() {
    assert_eq!(parse_tree_node_meta("Random text that matches no meta rule"), None);
    assert_eq!(parse_tree_node_meta(""), None);
}
