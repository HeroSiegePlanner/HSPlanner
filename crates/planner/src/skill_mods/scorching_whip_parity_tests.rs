//! The engine's dummy hits held to the game's as the preview plays Scorching Whip: the echo and
//! Fire Starter at their odds, and procs whose objects roll the other procs at their joint odds.
use crate::skill_mods::parity_tests as parity;
use crate::skill_mods::proc_odds;

const CLASS: &str = "exo";
const SKILL: &str = "scorching_whip";
const HARVESTER: &str = "solar_harvester";
const WHIRL: &str = "whirlwhipper";
const FIRE_STARTER: &str = "fire_starter";
/// The whip's on-hit procs, each of whose objects rolls the others (Celestial Aid from whips only).
const PROCS: [&str; 4] = [
    "punishing_flames",
    "celestial_aid",
    "blasting_whiplash",
    "flaming_whip",
];

fn expected(ranks: &[(&str, u32)]) -> f32 {
    let mods = parity::mods(SKILL, ranks);
    let base = &crate::skill_sim::skill(SKILL)
        .expect("scorching whip")
        .spawn;
    proc_odds::expected(SKILL, &crate::skill_mods::apply(SKILL, base, &mods))
}

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_hits(CLASS, SKILL, ranks)
}

fn agree(together: &[Vec<(&str, u32)>]) {
    let wrong: Vec<String> = (together.iter())
        .filter_map(|ranks| {
            let (sim, engine) = (expected(ranks), engine(ranks));
            let off = (sim - engine).abs() >= 0.01;
            off.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
        })
        .collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// Every rank of `a` with every rank of `b`.
fn pairs(a: &'static str, b: &'static str) -> Vec<Vec<(&'static str, u32)>> {
    let max = |id: &str| {
        let node = parity::nodes(CLASS, SKILL).iter().find(|n| n.id == id);
        node.map_or(0, |n| n.max_rank)
    };
    let ranks = (1..=max(a)).flat_map(|ra| (1..=max(b)).map(move |rb| (ra, rb)));
    ranks.map(|(ra, rb)| vec![(a, ra), (b, rb)]).collect()
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!((sim - base).abs() < 0.01, "sim {sim}, engine {base}");
    let wrong = parity::misses(CLASS, SKILL, expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_transformations_and_the_echo_the_sim_plays_together() {
    let mut together = pairs(HARVESTER, WHIRL);
    for node in [HARVESTER, WHIRL].into_iter().chain(PROCS) {
        together.extend(pairs("double_whip", node));
    }
    for shared in pairs(HARVESTER, WHIRL) {
        together.extend([1, 5].map(|rank| [shared.as_slice(), &[("double_whip", rank)]].concat()));
    }
    for node in PROCS.into_iter().chain([FIRE_STARTER]) {
        together.extend(pairs(HARVESTER, node));
        together.extend(pairs(WHIRL, node));
    }
    agree(&together);
}

#[test]
fn the_engine_follows_the_procs_that_roll_each_other() {
    let mut together = Vec::new();
    for (i, a) in PROCS.iter().enumerate() {
        for b in &PROCS[i + 1..] {
            together.extend(pairs(a, b));
        }
        together.extend(pairs(FIRE_STARTER, a));
    }
    agree(&together);
}

#[test]
fn the_engine_chains_three_and_more_procs_as_the_sim_does() {
    let procs = [PROCS.as_slice(), &[FIRE_STARTER]].concat();
    let sets = parity::proc_sets(CLASS, SKILL, &procs);
    let wrong = parity::set_misses(&sets, expected, engine);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
