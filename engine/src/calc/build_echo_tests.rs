use super::*;

fn performance(
    class: &str,
    skill: &str,
    rank: u32,
    nodes: &[u32],
    custom: &[(&str, &str)],
    projectiles: u32,
) -> BuildPerformance {
    compute_build_performance(&BuildPerformanceDeps {
        class_id: Some(class),
        level: 50,
        allocated_attrs: &HashMap::new(),
        inventory: &Inventory::new(),
        skill_ranks: &HashMap::from([(skill.into(), rank)]),
        subskill_ranks: &HashMap::new(),
        active_aura_id: None,
        active_buffs: &HashMap::new(),
        custom_stats: &custom
            .iter()
            .map(|(key, value)| CustomStat {
                stat_key: (*key).into(),
                value: (*value).into(),
            })
            .collect::<Vec<_>>(),
        allocated_tree_nodes: &nodes.iter().copied().collect(),
        tree_socketed: &HashMap::new(),
        main_skill_id: Some(skill),
        enemy_conditions: &HashMap::new(),
        player_conditions: &HashMap::new(),
        skill_projectiles: &HashMap::from([(skill.into(), projectiles)]),
        enemy_resistances: &HashMap::new(),
        proc_toggles: &HashMap::new(),
        kills_per_sec: 0.0,
        entity_rates: &HashMap::new(),
        stack_counts: &HashMap::new(),
        granted_skill_ranks: None,
        difficulty: None,
    })
}

fn trace(perf: &BuildPerformance, label: &str) -> Ranged {
    perf.calculation()
        .iter()
        .find(|step| step.label() == label)
        .unwrap_or_else(|| panic!("missing {label}"))
        .value()
}

#[test]
fn temporal_echo_counts_six_player_uses_and_preserves_primary_and_mana() {
    let base = performance("pyromancer", "fireball", 3, &[], &[], 1);
    let echo = performance("pyromancer", "fireball", 3, &[874], &[], 1);
    assert_eq!(base.avg_hit_dps_max, echo.avg_hit_dps_max);
    assert_eq!(
        base.damage.as_ref().unwrap().avg_max,
        echo.damage.as_ref().unwrap().avg_max
    );
    assert_eq!(
        serde_json::to_value(&base.skill_costs).unwrap(),
        serde_json::to_value(&echo.skill_costs).unwrap()
    );
    assert_eq!(trace(&echo, "Temporal Echo effective rank"), (4.0, 4.0));
    // Fireball repeat: all three skill synergies use rank four, including
    // the unlearned Comet/Meteor/Hydra. Intelligence remains an attribute.
    let intelligence = echo.attributes.get("intelligence").unwrap().1;
    let expected_hit =
        (4.0 + 18.0 * 4.0 * (1.0 + (4.0 * 37.5 + intelligence * 10.0) / 100.0)).ceil();
    let rate = echo.skill_costs["fireball"].cast_rate_max.unwrap();
    assert!(
        (echo.proc_dps_max - expected_hit * rate / 6.0).abs() < 1e-8,
        "actual={} expected={} hit={expected_hit} rate={rate}",
        echo.proc_dps_max,
        expected_hit * rate / 6.0
    );
    assert!(
        (echo.combined_dps_max.unwrap() - base.combined_dps_max.unwrap() - echo.proc_dps_max).abs()
            < 1e-8
    );
}

#[test]
fn temporal_echo_does_not_multicast_again_and_counts_projectiles_once() {
    let one = performance("pyromancer", "fireball", 10, &[874], &[], 1);
    let three = performance("pyromancer", "fireball", 10, &[874], &[], 3);
    assert!((three.proc_dps_max - one.proc_dps_max * 3.0).abs() < 1e-8);
    let multicast = performance(
        "pyromancer",
        "fireball",
        10,
        &[874],
        &[("multicast_chance", "100")],
        1,
    );
    assert_eq!(multicast.proc_dps_max, one.proc_dps_max);
    assert!(multicast.avg_hit_dps_max > one.avg_hit_dps_max);
    let ranked = performance(
        "pyromancer",
        "fireball",
        3,
        &[874],
        &[("all_skills", "2"), ("fire_skills", "1")],
        1,
    );
    assert_eq!(trace(&ranked, "Temporal Echo effective rank"), (9.0, 9.0));
    assert_eq!(
        trace(&ranked, "Temporal Echo maximum · Effective rank"),
        (9.0, 9.0)
    );
}

#[test]
fn temporal_echo_rejects_unlearned_non_spell_and_unverified_entity_paths() {
    for (class, skill, rank) in [
        ("pyromancer", "fireball", 0),
        ("amazon", "blender", 10),
        ("pyromancer", "hydra", 10),
    ] {
        let echo = performance(class, skill, rank, &[874], &[], 1);
        assert_eq!(echo.proc_dps_max, 0.0, "{skill}/{rank}");
        assert!(!echo
            .calculation()
            .iter()
            .any(|step| step.label() == "Temporal Echo direct DPS"));
    }
    let hydra = performance("pyromancer", "hydra", 10, &[874], &[], 1);
    assert!(hydra
        .calculation()
        .iter()
        .any(|step| step.label() == "Temporal Echo not yet modeled"));
}

#[test]
fn temporal_echo_replaces_burn_instead_of_adding_a_second_dot() {
    let custom = [("chance_inflict_burning", "100")];
    let base = performance("pyromancer", "fireball", 3, &[], &custom, 1);
    let echo = performance("pyromancer", "fireball", 3, &[874], &custom, 1);
    let primary_hit = base.damage.as_ref().unwrap().avg_max as f64;
    let echo_hit = trace(&echo, "Temporal Echo maximum · Average damage per cast").1;
    let fraction = data::game_config().ailment_base_fraction.as_ref().unwrap()["burning"];
    let expected = (6.0 * primary_hit + echo_hit) / 7.0 * fraction;
    assert!((echo.ailment_dps_max.unwrap() - expected).abs() < 1e-8);
    assert!(echo.ailment_dps_max.unwrap() < (primary_hit + echo_hit) * fraction);
    assert_eq!(base.avg_hit_dps_max, echo.avg_hit_dps_max);
    assert_eq!(
        echo.combined_dps_max.unwrap(),
        echo.avg_hit_dps_max.unwrap() + echo.proc_dps_max + echo.ailment_dps_max.unwrap()
    );
    let projectiles = performance("pyromancer", "fireball", 3, &[874], &custom, 3);
    assert!((projectiles.ailment_dps_max.unwrap() - expected).abs() < 1e-8);
    let multicast = performance(
        "pyromancer",
        "fireball",
        3,
        &[874],
        &[
            ("chance_inflict_burning", "100"),
            ("multicast_chance", "100"),
        ],
        1,
    );
    let expected_multicast = (12.0 * primary_hit + echo_hit) / 13.0 * fraction;
    assert!((multicast.ailment_dps_max.unwrap() - expected_multicast).abs() < 1e-8);
    assert_eq!(multicast.proc_dps_max, echo.proc_dps_max);
}
