//! The engine's dummy hits held to the game's as Hydra's procs roll each other: the attack's
//! rolls at their joint odds and Scorching Blast's novas at theirs, the burst shot's included.
use super::{
    BURSTFIRE, CALAMITY, CLONING, ESSENCE, ESSENCE_EVERY, SCORCHING_BLAST, SHOOT_FREQUENCY, SKILL,
};
use crate::skill_mods::parity_tests as parity;
use crate::skill_motion::Trail;
use crate::skill_sim::Spawn;

const CLASS: &str = "pyromancer";
/// The attack's own rolls (Step_0 L0x5f, L0x62).
const ROLLED: [&str; 2] = [BURSTFIRE, CLONING];
/// The procs whose objects carry the others: the comets roll the nova (InheritModifiers), and the
/// burst and clone shots, fresh shots of the hydra, roll both.
const PROCS: [&str; 4] = [SCORCHING_BLAST, CALAMITY, BURSTFIRE, CLONING];
/// Burning Essence's auras the preview's one-attack hydra pulses.
const WINDOW_AURAS: f32 = 2.;

/// Auras a grown hydra pulses in one attack period: an attack every ceil(60 × shootFrequency) + 1
/// frames, an aura every 20.
pub(super) fn auras_per_attack() -> f32 {
    let attack = (crate::projectile::FPS as f32 * SHOOT_FREQUENCY).ceil() + 1.;
    attack / ESSENCE_EVERY as f32
}

/// Damage-weighted hits of a hydra's attack, its rolls at their joint odds.
pub(super) fn expected(ranks: &[(&str, u32)]) -> f32 {
    let plain: Vec<(&str, u32)> = (ranks.iter().copied())
        .filter(|(node, _)| *node != ESSENCE)
        .collect();
    if plain.len() == ranks.len() {
        return at_odds(ranks);
    }
    // The engine counts the auras of a whole attack period, which the one-attack preview cuts.
    let without = at_odds(&plain);
    without + (at_odds(ranks) - without) * auras_per_attack() / WINDOW_AURAS
}

/// Every way the attack's rolls come up, each node's at its own odds, over a turn of casts.
fn at_odds(ranks: &[(&str, u32)]) -> f32 {
    let odds_of = |node: &'static str| {
        let &learned = ranks.iter().find(|(id, _)| *id == node)?;
        Some((node, parity::cast_odds(CLASS, SKILL, &[learned])))
    };
    let odds: Vec<(&str, f32)> = ROLLED.iter().filter_map(|&node| odds_of(node)).collect();
    let nova = odds_of(SCORCHING_BLAST).map_or(0., |(_, odds)| odds);
    let corner = |mask: usize, cast: u64| {
        let on = |i: usize| mask >> i & 1 == 1;
        let weight: f32 = (odds.iter().enumerate())
            .map(|(i, &(_, p))| if on(i) { p } else { 1. - p })
            .product();
        let lucky = |node: &str| odds.iter().position(|&(id, _)| id == node).is_some_and(on);
        let made = parity::rewritten(CLASS, SKILL, ranks, cast, |spawns, tree| {
            super::apply_rolled(spawns, tree, &lucky)
        });
        weight * novas_at_odds(&made, nova)
    };
    let turn = crate::skill_mods::casts(SKILL, 1);
    let all = (0..turn).flat_map(|cast| (0..1 << odds.len()).map(move |mask| (mask, cast)));
    all.map(|(mask, cast)| corner(mask, cast)).sum::<f32>() / turn as f32
}

/// Damage-weighted hits of `spawns`, every nova made always and never, weighted by `odds`.
fn novas_at_odds(spawns: &[Spawn], odds: f32) -> f32 {
    let at = |chance: f32| {
        let forced: Vec<Spawn> = spawns.iter().map(|spawn| rolled(spawn, chance)).collect();
        parity::weighted(&parity::play(SKILL, &forced, 1))
    };
    if odds == 0. {
        return parity::weighted(&parity::play(SKILL, spawns, 1));
    }
    let miss = at(0.);
    miss + odds * (at(1.) - miss)
}

/// parity::rolled, which leaves out the burst shot riding the shot trail's `next`.
fn rolled(spawn: &Spawn, chance: f32) -> Spawn {
    Spawn {
        trail: (spawn.trail.as_deref()).map(|trail| Box::new(shots(trail, chance))),
        ..parity::rolled(spawn, chance)
    }
}

fn shots(trail: &Trail, chance: f32) -> Trail {
    Trail {
        spawn: parity::rolled(&trail.spawn, chance),
        next: (trail.next.as_deref()).map(|next| Box::new(shots(next, chance))),
        ..trail.clone()
    }
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
fn the_burst_shots_nova_counts_at_its_odds() {
    let ranks = [(SCORCHING_BLAST, 5), (BURSTFIRE, 5)];
    let burst = parity::rewritten(CLASS, SKILL, &ranks, 0, |spawns, tree| {
        super::apply_rolled(spawns, tree, &|_| true)
    });
    // Ball and burst ball, each with a nova a fifth of the time.
    assert!((novas_at_odds(&burst, 0.2) - 2.4).abs() < 1e-4);
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
fn the_engine_follows_the_procs_on_the_krakens_breaths_and_the_auras() {
    agree(&[
        vec![(SCORCHING_BLAST, 5), (CALAMITY, 3), ("scorching_kraken", 1)],
        vec![(BURSTFIRE, 5), (CALAMITY, 3), ("scorching_kraken", 2)],
        vec![
            (BURSTFIRE, 5),
            (SCORCHING_BLAST, 5),
            ("scorching_kraken", 1),
        ],
        vec![(CLONING, 5), (CALAMITY, 3), ("scorching_kraken", 1)],
        vec![(SCORCHING_BLAST, 5), (CALAMITY, 3), (ESSENCE, 5)],
        vec![(SCORCHING_BLAST, 1), (CALAMITY, 1), (ESSENCE, 1)],
        vec![(BURSTFIRE, 5), (CALAMITY, 3), (ESSENCE, 5)],
    ]);
}
