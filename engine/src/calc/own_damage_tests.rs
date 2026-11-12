// A node whose damage the game keeps to its own objects, or never reads, stays out of the skill's
// pools: the dummy hits carry it instead.
use std::collections::HashMap;

use super::BuildPerformance;
use crate::calc::commands::{calc_build_performance, BuildPerformanceInput};
use crate::calc::skills::SkillDamageBreakdown;

fn performance(class: &str, skill: &str, subskills: &[(&str, u32)]) -> BuildPerformance {
    calc_build_performance(BuildPerformanceInput {
        class_id: Some(class.into()),
        level: 100,
        skill_ranks: HashMap::from([(skill.into(), 20)]),
        subskill_ranks: subskills
            .iter()
            .map(|(id, rank)| (format!("{skill}:{id}"), *rank))
            .collect(),
        main_skill_id: Some(skill.into()),
        ..Default::default()
    })
}

fn damage(class: &str, skill: &str, subskills: &[(&str, u32)]) -> SkillDamageBreakdown {
    let perf = performance(class, skill, subskills);
    perf.damage.expect("elemental damage")
}

fn skill_pct(class: &str, skill: &str, subskills: &[(&str, u32)]) -> f64 {
    damage(class, skill, subskills).skill_damage_max_pct
}

fn of_total_pct(class: &str, skill: &str, subskills: &[(&str, u32)]) -> f64 {
    let sources = damage(class, skill, subskills).extra_damage_sources;
    sources
        .iter()
        .filter(|source| source.label == "Subtree")
        .map(|source| source.pct)
        .sum()
}

#[test]
fn too_hot_to_touch_leaves_the_volcanos_skill_damage() {
    let plain = skill_pct("pyromancer", "volcano", &[]);
    let hot = skill_pct("pyromancer", "volcano", &[("too_hot_to_touch", 5)]);
    assert_eq!(hot, plain);
}

#[test]
fn the_calculation_says_where_too_hot_to_touch_went() {
    let perf = performance("pyromancer", "volcano", &[("too_hot_to_touch", 5)]);
    let step = (perf.calculation().iter())
        .find(|step| step.label() == "Subskill damage on its own objects")
        .map(|step| step.value());
    assert_eq!(step, Some((200.0, 200.0)));
}

#[test]
fn celestial_collapse_leaves_the_moons_of_total_damage() {
    let collapse = of_total_pct("exo", "lunar_orbit", &[("celestial_collapse", 5)]);
    assert_eq!(collapse, 0.0);
}

#[test]
fn stacked_burn_still_raises_the_whole_slash() {
    let plain = skill_pct("pyromancer", "inferno_slash", &[]);
    let stacked = skill_pct("pyromancer", "inferno_slash", &[("stacked_burn", 5)]);
    assert!(stacked > plain, "{plain} -> {stacked}");
}
