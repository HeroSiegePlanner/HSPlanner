//! The engine's dummy hits held to the game's as Breath of Fire's procs roll each other: the
//! cast's rolls at their joint odds and Hellflame's stacks at every count a cast may find.
use super::{BURST, CHAIN, COMBUSTION, DOUBLE, HELLFLAME, HELLFLAME_STACKS, SKILL, cast, rolls};
use crate::skill_mods::parity_tests as parity;

const CLASS: &str = "pyromancer";
/// The cast's own rolls (case 30 L0x2bb-L0x2c8).
const ROLLED: [&str; 3] = [COMBUSTION, DOUBLE, BURST];
/// The procs whose objects carry the others (InheritModifiers in PC02 329-334), and Double
/// Breeze, whose second breath keeps Burst of Heat.
const PROCS: [&str; 5] = [COMBUSTION, DOUBLE, BURST, HELLFLAME, CHAIN];

/// Damage-weighted hits a cast is expected to land: its rolls at their joint odds, and the
/// Hellflame stacks it finds any count from none to ten alike, as a long run of casts leaves them.
pub(super) fn expected(ranks: &[(&str, u32)]) -> f32 {
    let odds: Vec<(&str, f32)> = (ROLLED.iter())
        .filter_map(|&node| {
            let &learned = ranks.iter().find(|(id, _)| *id == node)?;
            Some((node, parity::cast_odds(CLASS, SKILL, &[learned])))
        })
        .collect();
    let counts = if ranks.iter().any(|(id, _)| *id == HELLFLAME) {
        HELLFLAME_STACKS + 1
    } else {
        1
    };
    let corner = |mask: usize| {
        let on = |i: usize| mask >> i & 1 == 1;
        let weight: f32 = (odds.iter().enumerate())
            .map(|(i, &(_, p))| if on(i) { p } else { 1. - p })
            .product();
        let lucky = |node: &str| odds.iter().position(|&(id, _)| id == node).is_some_and(on);
        let found = |from: u32| {
            let made = parity::rewritten(CLASS, SKILL, ranks, 0, |spawns, tree| {
                cast(spawns, tree, &rolls(tree, |node, _| lucky(node)), from)
            });
            parity::expected(SKILL, &made)
        };
        weight * (0..counts).map(found).sum::<f32>() / counts as f32
    };
    (0..1 << odds.len()).map(corner).sum()
}

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_weighted(CLASS, SKILL, ranks)
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
fn the_engine_follows_the_procs_that_roll_each_other() {
    let mut together = Vec::new();
    for (i, a) in PROCS.iter().enumerate() {
        for b in &PROCS[i + 1..] {
            together.extend(pairs(a, b));
        }
    }
    agree(&together);
}

#[test]
fn the_engine_chains_three_and_more_procs_as_the_sim_does() {
    let sets = parity::proc_sets(CLASS, SKILL, &PROCS);
    let wrong = parity::set_misses(&sets, expected, engine);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_chains_on_every_breath_of_the_volley() {
    agree(&[
        vec![(COMBUSTION, 5), (HELLFLAME, 3), ("hydras_breeze", 1)],
        vec![(BURST, 5), (CHAIN, 1), ("molten_orb", 1)],
        vec![
            (COMBUSTION, 5),
            (HELLFLAME, 3),
            (CHAIN, 1),
            ("molten_orb", 1),
        ],
        vec![(DOUBLE, 5), (BURST, 5), (CHAIN, 3), ("hydras_breeze", 1)],
        vec![
            (COMBUSTION, 5),
            (BURST, 5),
            (HELLFLAME, 3),
            ("hydras_breeze", 1),
        ],
        vec![(BURST, 3), (HELLFLAME, 2), (CHAIN, 1), ("molten_orb", 2)],
    ]);
}
