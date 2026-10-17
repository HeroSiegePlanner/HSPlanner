use app_lib::calc::commands::{calc_build_performance, calc_build_stats, BuildPerformanceInput};
use app_lib::calc::types::{CustomStat, EquippedItem};

fn build(node_rank: u32) -> BuildPerformanceInput {
    BuildPerformanceInput {
        class_id: Some("butcher".into()),
        level: 50,
        main_skill_id: Some("brutalizing_slash".into()),
        skill_ranks: [
            ("brutalizing_slash".into(), 10),
            ("slicing_throw".into(), 10),
        ]
        .into_iter()
        .collect(),
        subskill_ranks: [("brutalizing_slash:bucthering_skill".into(), node_rank)]
            .into_iter()
            .collect(),
        inventory: [(
            "weapon".into(),
            EquippedItem {
                base_id: "base_dagger_kris".into(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        custom_stats: vec![CustomStat {
            stat_key: "crit_chance".into(),
            value: "10".into(),
        }],
        ..Default::default()
    }
}

#[test]
fn bucthering_skill_multiplies_only_brutalizing_intrinsic_attack_rating() {
    let result = calc_build_performance(build(5)).attack_damage.unwrap();
    assert!(result.physical_hit_max > 0);
    assert_eq!(result.attack_rating_pct_min, 75.0);
    assert_eq!(result.attack_rating_pct_max, 75.0);
}

#[test]
fn bucthering_skill_never_enters_global_attack_rating_or_crit_pool() {
    let baseline = calc_build_stats(build(0));
    let learned = calc_build_stats(build(5));
    let global_pools = |stats: &app_lib::calc::stats::ComputedStats| {
        ["attack_rating_pct", "crit_chance"]
            .map(|key| stats.stats.get(key).copied().unwrap_or_default())
    };
    assert_eq!(global_pools(&learned), global_pools(&baseline));
}

#[test]
fn bucthering_skill_does_not_change_another_primary_skill() {
    let other = |rank| {
        let mut input = build(rank);
        input.main_skill_id = Some("slicing_throw".into());
        calc_build_performance(input).attack_damage.unwrap()
    };
    let baseline = other(0);
    let learned = other(5);
    assert_eq!(
        learned.attack_rating_pct_max,
        baseline.attack_rating_pct_max
    );
    assert_eq!(learned.crit_chance, baseline.crit_chance);
    assert_eq!(learned.physical_hit_max, baseline.physical_hit_max);
}

#[test]
fn with_two_hands_stores_an_expected_cast_repeat_without_increasing_primary_damage() {
    let baseline = calc_build_performance(build(0));
    for rank in 1..=5 {
        let mut input = build(0);
        input.subskill_ranks = [("brutalizing_slash:with_2_hands".into(), rank)]
            .into_iter()
            .collect();
        let changed = calc_build_performance(input);
        let a = baseline.attack_damage.as_ref().unwrap();
        let b = changed.attack_damage.as_ref().unwrap();
        assert_eq!(b.physical_hit_min, a.physical_hit_min);
        assert_eq!(b.physical_hit_max, a.physical_hit_max);
        assert_eq!(b.physical_avg_max, a.physical_avg_max);
        // The dps_cards dummy model counts the expected extra strike (+0.04r hits).
        assert!(changed.avg_hit_dps_max > baseline.avg_hit_dps_max);
        let count = changed
            .calculation()
            .iter()
            .find(|step| step.label() == "Subtree · secondary projectile count")
            .expect("the attack-time roll retains a separate expected spawn count");
        assert!(
            (count.value().0 - [0.04, 0.08, 0.12, 0.16, 0.20][rank as usize - 1]).abs() < 1e-12
        );
        assert!(!changed.stats.contains_key("secondary_projectile_count"));
        assert!(changed.calculation().iter().any(|step| {
            step.label() == "Model note · With 2 Hands"
                && step.expression().contains("excluded from displayed DPS")
        }));
    }
}

#[test]
fn for_the_weak_keeps_fractional_strength_size_offset_out_of_damage_and_generic_range() {
    let mut input = build(0);
    input.inventory.clear();
    input.custom_stats.extend([
        CustomStat {
            stat_key: "to_strength".into(),
            value: "98-298".into(),
        },
        CustomStat {
            stat_key: "skill_restrictions_removed".into(),
            value: "100".into(),
        },
        CustomStat {
            stat_key: "melee_attack_size_offset".into(),
            value: "1.75".into(),
        },
    ]);
    let baseline = calc_build_performance(input.clone());
    assert_eq!(baseline.attributes["strength"], (100.0, 300.0));
    for (rank, expected) in [(1, (0.025, 0.075)), (5, (0.125, 0.375))] {
        input.subskill_ranks = [("brutalizing_slash:for_the_weak".into(), rank)]
            .into_iter()
            .collect();
        let changed = calc_build_performance(input.clone());
        let step = changed
            .attack_damage
            .as_ref()
            .unwrap()
            .calculation()
            .iter()
            .find(|step| step.label() == "Local melee size offset from Strength")
            .expect("show the separate eligible-melee scale offset");
        for (actual, expected) in [(step.value().0, expected.0), (step.value().1, expected.1)] {
            assert!((actual - expected).abs() < 1e-12);
        }
        assert_eq!(changed.stats["melee_attack_size_offset"], (1.0, 1.0));
        assert_eq!(changed.stats, baseline.stats);
        let a = baseline.attack_damage.as_ref().unwrap();
        let b = changed.attack_damage.as_ref().unwrap();
        assert_eq!(b.physical_hit_max, a.physical_hit_max);
        assert_eq!(b.physical_avg_max, a.physical_avg_max);
        assert_eq!(changed.avg_hit_dps_max, baseline.avg_hit_dps_max);
        assert!(!changed
            .stats
            .contains_key("skill_melee_size_offset_per_100_strength"));
    }
}
