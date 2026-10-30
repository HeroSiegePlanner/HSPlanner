// Scorching Aura from LoadAura case 32 / LoadAuraScorching: one fire hit per enemy every
// ceil(60 · (1 − Rapid Burning%)) + 1 frames, whatever the cast speed.
use std::collections::HashMap;

use crate::calc::commands::{calc_build_performance, BuildPerformanceInput};
use crate::calc::planner::{evaluate, PlannerInput};
use crate::calc::types::CustomStat;

fn input(subskills: &[(&str, u32)], custom: &[(&str, f64)]) -> BuildPerformanceInput {
    BuildPerformanceInput {
        class_id: Some("pyromancer".into()),
        level: 50,
        skill_ranks: HashMap::from([("scorching_aura".into(), 10)]),
        subskill_ranks: subskills
            .iter()
            .map(|(id, rank)| (format!("scorching_aura:{id}"), *rank))
            .collect(),
        main_skill_id: Some("scorching_aura".into()),
        active_aura_id: Some("scorching_aura".into()),
        custom_stats: custom
            .iter()
            .map(|(key, value)| CustomStat {
                stat_key: key.to_string(),
                value: value.to_string(),
            })
            .collect(),
        ..Default::default()
    }
}

fn hit_and_rate(subskills: &[(&str, u32)], custom: &[(&str, f64)]) -> (f64, f64) {
    let perf = calc_build_performance(input(subskills, custom));
    let hit = perf.hit_damage().expect("Scorching Aura deals damage").1;
    let dps = perf.hit_dps_max.expect("Scorching Aura has hit DPS");
    (hit, dps / hit)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn pulses_61_frames_apart_whatever_the_cast_speed() {
    let (_, rate) = hit_and_rate(&[], &[]);
    assert!(close(rate, 60.0 / 61.0), "{rate}");
    let (_, fast) = hit_and_rate(&[], &[("faster_cast_rate", 200.0)]);
    assert!(close(fast, 60.0 / 61.0), "{fast}");
}

#[test]
fn rapid_burning_shortens_the_pulse_period() {
    for (rank, period) in [(1, 52.0), (3, 34.0), (5, 16.0)] {
        let (_, rate) = hit_and_rate(&[("rapid_burning", rank)], &[]);
        assert!(close(rate, 60.0 / period), "rank {rank}: {rate}");
    }
}

#[test]
fn damaging_aura_effectiveness_scales_the_pulse() {
    let (base, _) = hit_and_rate(&[], &[]);
    let (boosted, _) = hit_and_rate(&[], &[("damaging_aura_effectiveness", 50.0)]);
    assert!((boosted / base - 1.5).abs() < 0.01, "{base} -> {boosted}");
}

#[test]
fn burning_soul_adds_max_life_to_the_subtree_multiplier() {
    let (base, _) = hit_and_rate(&[], &[("life", 1000.0)]);
    let (soul, _) = hit_and_rate(&[("burning_soul", 1)], &[("life", 1000.0)]);
    let life = calc_build_performance(input(&[], &[("life", 1000.0)])).stats["life"].1;
    let expected = 1.0 + 0.06 * life / 100.0;
    assert!(
        (soul / base - expected).abs() < 0.01,
        "{base} -> {soul}, life {life}"
    );
}

#[test]
fn an_active_scorching_aura_adds_to_the_main_skills_dps() {
    let build = |aura: Option<&str>| BuildPerformanceInput {
        class_id: Some("pyromancer".into()),
        level: 50,
        skill_ranks: HashMap::from([("fireball".into(), 10), ("scorching_aura".into(), 10)]),
        main_skill_id: Some("fireball".into()),
        active_aura_id: aura.map(str::to_string),
        ..Default::default()
    };
    let combined = |aura| {
        evaluate(&PlannerInput {
            build: build(aura),
            active_skill_ids: vec!["fireball".into()],
            ..Default::default()
        })
        .current
        .combined_dps_max
        .unwrap()
    };
    let (without, with) = (combined(None), combined(Some("scorching_aura")));
    assert!(with > without, "{without} -> {with}");
}

#[test]
fn an_aura_only_build_reports_the_auras_dps() {
    let mut build = input(&[], &[]);
    build.main_skill_id = None;
    let planner = PlannerInput {
        build,
        ..Default::default()
    };
    let result = evaluate(&planner);
    assert!(result.current.combined_dps_max.is_some_and(|dps| dps > 0.0));
    assert_eq!(result.per_skill.len(), 1);
    assert_eq!(result.per_skill[0].skill_id, "scorching_aura");
    assert_eq!(
        crate::calc::planner::dps_skill_ids(&planner),
        vec!["scorching_aura".to_string()]
    );
}
