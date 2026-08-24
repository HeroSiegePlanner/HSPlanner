use super::*;

#[test]
fn parse_custom_stats_batch_mirrors_custom_stat_parser() {
    let out = parse_custom_stats(vec![
        "100".to_string(),
        "50-80".to_string(),
        "not a number".to_string(),
    ]);
    assert_eq!(out[0], Some([100.0, 100.0]));
    assert_eq!(out[1], Some([50.0, 80.0]));
    assert_eq!(out[2], None);
}

#[test]
fn display_values_batch_matches_affix_math() {
    let affix = Affix {
        id: "t".into(),
        stat_key: Some("life".into()),
        value_min: Some(10.0),
        value_max: Some(20.0),
        ..Default::default()
    };
    let input = DisplayValuesInput {
        affixes: vec![AffixValueReq {
            affix: affix.clone(),
            roll: 0.5,
            stars: None,
        }],
        scaled: vec![ScaledValueReq {
            value: [10.0, 20.0],
            stat_key: "life".into(),
            stars: Some(0),
        }],
    };
    let out = display_values_impl(&input);
    assert_eq!(
        out.affixes[0].value,
        super::super::affix::rolled_affix_value(&affix, 0.5)
    );
    assert_eq!(out.affixes[0].range_min, 10.0);
    assert_eq!(out.affixes[0].range_max, 20.0);
    assert_eq!(out.scaled[0], [10.0, 20.0]);
}

#[test]
fn classify_tree_nodes_partitions_every_node_line() {
    let map = classify_tree_nodes_impl();
    assert!(!map.is_empty(), "tree data should yield nodes");
    let nodes = super::super::data::tree_nodes();
    for (id, cls) in &map {
        let node = nodes.get(id).expect("classified id exists in data");
        assert!(cls.parsed.len() + cls.unsupported.len() <= node.lines.len());
        for line in cls.parsed.iter().chain(cls.unsupported.iter()) {
            assert!(node.lines.contains(line), "line must come from the node");
        }
    }
}

#[test]
fn subskill_aggregation_unknown_skill_returns_empty() {
    let input = SubskillAggregationInput {
        class_id: "no_such_class".to_string(),
        skill_id: "no_such_skill".to_string(),
        subskill_ranks: HashMap::from([("no_such_skill:sub".to_string(), 3)]),
        enemy_conditions: HashMap::new(),
        season: None,
    };
    let out = subskill_aggregation_impl(&input);
    assert!(out.stats.is_empty());
    assert!(out.proc_stats.is_empty());
    assert!(out.applied_states.is_empty());
}

// Proves the item-ranking batch returns one finite score per base id.
#[test]
fn rank_slot_items_scores_every_base() {
    let perf: BuildPerformanceInput =
        serde_json::from_str("{}").expect("all fields default");
    let base_ids: Vec<String> = crate::calc::data::data()
        .items
        .keys()
        .take(3)
        .cloned()
        .collect();
    assert!(!base_ids.is_empty(), "items present");
    let out = rank_slot_items(RankSlotItemsInput {
        perf,
        slot: "armor".to_string(),
        base_ids: base_ids.clone(),
        active_skill_ids: Vec::new(),
    });
    assert_eq!(out.len(), base_ids.len());
    assert!(out.values().all(|v| v.is_finite() && *v >= 0.0));
}

// Every S10 stat line must either parse into a stat key or sit on the exemption
// list below; a new pattern on either side fails the test.
#[test]
fn s10_incarnation_node_lines_parse_coverage() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    // ponytail: unmodeled S10 mechanics — drop an entry once the engine learns it.
    const EXEMPT_PATTERNS: &[&str] = &[
        "+# to Level of Struck Skills",
        "+## Increased Damage with Leap skills",
        "+## to Level of Struck Skills",
        "+#### Lightning Damage dealt by odin",
        "+###% Living Carcass Explosion Damage",
        "+##% Arcana Destruction Damage",
        "+##% Avalanche of Boulders Damage",
        "+##% Chance for Critical Arcane Break on hit",
        "+##% Chance for Critical Cold Break on hit",
        "+##% Chance for Critical Fire Break",
        "+##% Chance for Critical Lightning Break on hit",
        "+##% Chance for Critical Poison Break",
        "+##% Chance on hit to unleash multiple piercing daggers flying in a cone dealing damage.",
        "+##% Critical Arcane Break Damage",
        "+##% Critical Cold Break Damage",
        "+##% Critical Fire Break Damage",
        "+##% Critical Lightning Break Damage",
        "+##% Critical Poison Break Damage",
        "+##% Heart of Fire Damage",
        "+##% Increased Damage with Leap skills",
        "+##% Increased Melee Projectile Critical Damage",
        "+##% Increased Struck Skill effectiveness",
        "+##% Storm Turbulence Damage",
        "+##% Vile Pustules Damage",
        "+##% Wallbanger Damage",
        "+##% of Maximum life dealt as Arcane Damage",
        "+#% Chance to unleash Arcana Destruction on hit",
        "+#% Chance to unleash Avalanche of Boulders on hit",
        "+#% Chance to unleash Heart of Fire on hit",
        "+#% Chance to unleash Storm Turbulence on hit",
        "+#% Chance to unleash Vile Pustules on hit",
        "+#% Increased Melee Projectile Damage",
        "-##% Increased Melee Projectile Damage",
    ];

    let map = classify_tree_nodes_impl();
    let mut parsed = 0usize;
    let mut unsupported: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for c in map.values() {
        parsed += c.parsed.len();
        for line in &c.unsupported {
            let pattern: String = line
                .chars()
                .map(|ch| if ch.is_ascii_digit() { '#' } else { ch })
                .collect();
            unsupported.insert(pattern);
        }
    }
    eprintln!(
        "incarnation coverage: parsed={parsed} unsupported_patterns={}",
        unsupported.len()
    );
    // Mass-regression floor: a stray null_rule! silently reclassifies stat lines
    // as no-stat without touching the pattern sets. 2234 parse as of S10 launch.
    assert!(
        parsed >= 2200,
        "incarnation parsed-line count dropped to {parsed} (expected >= 2200)"
    );
    let exempt: std::collections::BTreeSet<String> =
        EXEMPT_PATTERNS.iter().map(|s| s.to_string()).collect();
    let new_unsupported: Vec<_> = unsupported.difference(&exempt).collect();
    let stale_exempt: Vec<_> = exempt.difference(&unsupported).collect();
    assert!(
        new_unsupported.is_empty(),
        "new incarnation stat lines the parser does not understand: {new_unsupported:#?}"
    );
    assert!(
        stale_exempt.is_empty(),
        "exempt patterns that now parse — remove them from EXEMPT_PATTERNS: {stale_exempt:#?}"
    );
}

mod projectile_dto_tests {
    use super::*;

    #[test]
    fn skill_damage_input_accepts_fractional_projectile_count() {
        let input: SkillDamageInput = serde_json::from_value(serde_json::json!({
            "skill": { "name": "Test" },
            "allocatedRank": 1.0,
            "projectileCount": 13.04
        }))
        .expect("fractional projectileCount must deserialize");
        assert_eq!(input.projectile_count.max(0.0) as u32, 13);
    }

    #[test]
    fn weapon_damage_input_accepts_fractional_projectile_count() {
        let input: WeaponDamageInput = serde_json::from_value(serde_json::json!({
            "projectileCount": 2.6
        }))
        .expect("fractional projectileCount must deserialize");
        assert_eq!(input.projectile_count.map(|p| p.max(0.0) as u32), Some(2));
    }
}
