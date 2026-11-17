//! The engine's dummy hits held to the game's as the preview plays Supernova: the second nova at
//! its odds, and procs whose objects roll the other procs at their joint odds.
use crate::skill_mods::parity_tests as parity;
use crate::skill_mods::proc_odds;

const CLASS: &str = "exo";
const SKILL: &str = "supernova";
/// The nova's on-hit procs, each of whose objects rolls the others. Three or more of them overflow
/// the preview's 256-object cap, so only pairs are held to the sim; proc_chain_tests and Scorching
/// Whip's sets cover longer chains.
const PROCS: [&str; 4] = [
    "chain_reactions",
    "particle_outflux",
    "connected_by_sun",
    "planetary_collision",
];

fn expected(ranks: &[(&str, u32)]) -> f32 {
    let mods = parity::mods(SKILL, ranks);
    let base = &crate::skill_sim::skill(SKILL).expect("supernova").spawn;
    proc_odds::expected(SKILL, &crate::skill_mods::apply(SKILL, base, &mods))
}

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_hits(CLASS, SKILL, ranks)
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
fn the_engine_follows_the_second_nova_and_the_procs_that_roll_each_other() {
    let mut together = Vec::new();
    for (i, a) in PROCS.iter().enumerate() {
        for b in &PROCS[i + 1..] {
            together.extend(pairs(a, b));
        }
        together.extend(pairs("chained_explosions", a));
    }
    let wrong: Vec<String> = (together.iter())
        .filter_map(|ranks| {
            let (sim, engine) = (expected(ranks), engine(ranks));
            let off = (sim - engine).abs() >= 0.01;
            off.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
        })
        .collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
