//! The engine's dummy hits held to the game's as Fireball's procs roll each other: the cast's
//! rolls at their joint odds.
use super::{CRITICAL, DOOM, FIREFLIES, MULTICAST, PARALYZING, SKILL};
use crate::skill_mods::parity_tests as parity;

const CLASS: &str = "pyromancer";
/// The cast's own rolls (case 29 L0x20f-L0x252).
const ROLLED: [&str; 5] = [PARALYZING, CRITICAL, DOOM, FIREFLIES, MULTICAST];

/// Damage-weighted hits a cast is expected to land, every way its rolls come up at their joint
/// odds.
pub(super) fn expected(ranks: &[(&str, u32)]) -> f32 {
    let odds: Vec<(&str, f32)> = (ROLLED.iter())
        .filter_map(|&node| {
            let &learned = ranks.iter().find(|(id, _)| *id == node)?;
            Some((node, parity::cast_odds(CLASS, SKILL, &[learned])))
        })
        .collect();
    let corner = |mask: usize| {
        let on = |i: usize| mask >> i & 1 == 1;
        let weight: f32 = (odds.iter().enumerate())
            .map(|(i, &(_, p))| if on(i) { p } else { 1. - p })
            .product();
        let lucky = |node: &str| odds.iter().position(|&(id, _)| id == node).is_some_and(on);
        let made = parity::rewritten(CLASS, SKILL, ranks, 0, |spawns, tree| {
            super::apply_rolled(spawns, tree, &lucky)
        });
        weight * parity::expected(SKILL, &made)
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
    // Mark of Doom's meteor carries the fireflies (PC02 336 then 338); no other proc makes objects.
    agree(&pairs(DOOM, FIREFLIES));
}

#[test]
fn the_engine_follows_the_meteors_fireflies_with_the_casts_other_rolls() {
    agree(&[
        vec![(DOOM, 5), (FIREFLIES, 3), (CRITICAL, 5)],
        vec![(DOOM, 3), (FIREFLIES, 2), (MULTICAST, 5)],
        vec![(DOOM, 1), (FIREFLIES, 1), (CRITICAL, 1), (MULTICAST, 1)],
    ]);
}
