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

mod combined_dps_mid_tests {
    use crate::calc::build::BuildPerformance;
    use crate::calc::commands::performance::combined_dps_mid;

    fn perf(avg: f64, exec: f64, proc_dps: f64, ailment: Option<f64>) -> BuildPerformance {
        BuildPerformance {
            avg_hit_dps_min: Some(avg),
            avg_hit_dps_max: Some(avg),
            execute_mult: exec,
            proc_dps_min: proc_dps,
            proc_dps_max: proc_dps,
            ailment_dps_min: ailment,
            ailment_dps_max: ailment,
            ..Default::default()
        }
    }

    #[test]
    fn multi_skill_sums_executed_avg_and_counts_proc_once() {
        let ids = vec!["a".to_string(), "b".to_string()];
        // a: 100 avg x2.0 exec + proc 10 x2.0; b: 50 avg x1.0 + ailment 5 x1.0
        let dps = combined_dps_mid(&ids, |main| match main {
            Some("a") => perf(100.0, 2.0, 10.0, None),
            Some("b") => perf(50.0, 1.0, 999.0, Some(5.0)),
            _ => panic!("unexpected main {main:?}"),
        });
        // 100*2 + 50*1 + 10*2 (primary proc only) + 5*1 = 275
        assert!((dps - 275.0).abs() < 1e-9, "got {dps}");
    }

    #[test]
    fn single_skill_uses_combined_dps_midpoint() {
        let ids = vec!["a".to_string()];
        let dps = combined_dps_mid(&ids, |main| {
            assert_eq!(main, Some("a"));
            BuildPerformance {
                combined_dps_min: Some(100.0),
                combined_dps_max: Some(200.0),
                ..Default::default()
            }
        });
        assert!((dps - 150.0).abs() < 1e-9, "got {dps}");
    }

    #[test]
    fn single_skill_falls_back_to_hit_dps_then_zero() {
        let ids = vec!["a".to_string()];
        let hit_only = combined_dps_mid(&ids, |_| BuildPerformance {
            hit_dps_min: Some(10.0),
            hit_dps_max: Some(30.0),
            ..Default::default()
        });
        assert!((hit_only - 20.0).abs() < 1e-9, "got {hit_only}");
        let empty = combined_dps_mid(&[], |main| {
            assert_eq!(main, None);
            BuildPerformance::default()
        });
        assert_eq!(empty, 0.0);
    }
}
