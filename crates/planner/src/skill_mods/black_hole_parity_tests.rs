//! The engine's dummy hits held to the preview's Black Hole: every node alone at every rank and the
//! nodes whose objects work together, every roll at its odds.
use super::{
    EMPOWERED, GALAXY, HIDDEN, RADIATION, SINGULARITY, SUBZERO, SUPERMASSIVE, WATERMELTING,
};
use crate::skill_effects::Proc;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::Spawn;

const CLASS: &str = "exo";
const SKILL: &str = "black_hole";
const FLARE: &str = "Exo_Black_Hole_Cosmic_Flare_obj";

/// `spawn` with Radiation's flares rolled at `flare` and Singularity's bursts at `burst`, its
/// frostbite left out: the engine deals that as its frostbite ailment, not as hits.
fn rolled(spawn: &Spawn, flare: f32, burst: f32) -> Spawn {
    let procs = spawn.procs.iter().map(|proc| Proc {
        chance: if proc.spawn.object == FLARE {
            flare
        } else {
            burst
        },
        spawn: rolled(&proc.spawn, flare, burst),
        ..proc.clone()
    });
    Spawn {
        frostbite: None,
        procs: procs.collect(),
        ..spawn.clone()
    }
}

/// The odds of `node`'s roll, 0 when `ranks` leave it out.
fn odds(ranks: &[(&str, u32)], node: &str) -> f32 {
    let learned = ranks.iter().find(|(id, _)| *id == node);
    learned.map_or(0., |&learned| parity::cast_odds(CLASS, SKILL, &[learned]))
}

/// Damage-weighted hits of a use whose swells come up `lucky` and whose rolls come up as given.
fn play(ranks: &[(&str, u32)], lucky: bool, flare: f32, burst: f32) -> f32 {
    let made = parity::rewritten(CLASS, SKILL, ranks, 0, |spawns, tree| {
        super::apply_rolled(spawns, tree, lucky)
    });
    let made: Vec<Spawn> = made.iter().map(|s| rolled(s, flare, burst)).collect();
    parity::weighted(&parity::play(SKILL, &made, 1))
}

/// Damage-weighted hits a use is expected to land: each outcome of Galaxy's swells and of
/// Radiation's and Singularity's rolls weighted by its odds, as a use is linear in each.
fn expected(ranks: &[(&str, u32)]) -> f32 {
    let [swell, flare, burst] = [GALAXY, RADIATION, SINGULARITY].map(|node| odds(ranks, node));
    let outcomes = |odds: f32| [(0., 1. - odds), (1., odds)];
    let corners = outcomes(swell).into_iter().flat_map(|(lucky, a)| {
        outcomes(flare).into_iter().flat_map(move |(f, b)| {
            outcomes(burst)
                .into_iter()
                .map(move |(s, c)| ([lucky, f, s], a * b * c))
        })
    });
    corners
        .filter(|(_, weight)| *weight > 0.)
        .map(|([lucky, f, s], weight)| weight * play(ranks, lucky > 0., f, s))
        .sum()
}

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_hits(CLASS, SKILL, ranks)
}

/// Every rank of `a` with every rank of `b`.
fn pairs(a: (&'static str, u32), b: (&'static str, u32)) -> Vec<[(&'static str, u32); 2]> {
    let ranks = |(node, max): (&'static str, u32)| (1..=max).map(move |rank| (node, rank));
    ranks(a)
        .flat_map(|x| ranks(b).map(move |y| [x, y]))
        .collect()
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!(
        (sim - 12.).abs() < 1e-4 && (base - 12.).abs() < 1e-4,
        "sim {sim}, engine {base}"
    );
    let wrong = parity::misses(CLASS, SKILL, expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_nodes_the_sim_plays_together() {
    let (radiation, singularity) = ((RADIATION, 5), (SINGULARITY, 5));
    let (tick, life) = (("time_space", 5), ("mass_density", 5));
    let mut together = pairs(radiation, singularity);
    for proc in [radiation, singularity] {
        together.extend(pairs(tick, proc));
        // Not Galaxy: the game's flare and burst take the hole's swollen damage[2], the sim's not.
        for transformed in [HIDDEN, SUPERMASSIVE, EMPOWERED] {
            together.extend(pairs(proc, (transformed, 3)));
        }
    }
    let transformations = [GALAXY, HIDDEN, SUPERMASSIVE, EMPOWERED];
    for (i, transformed) in transformations.into_iter().enumerate() {
        together.extend(pairs(tick, (transformed, 3)));
        together.extend(pairs(life, (transformed, 3)));
        for other in &transformations[i + 1..] {
            together.extend(pairs((transformed, 3), (other, 3)));
        }
    }
    together.extend(pairs(tick, life));
    together.extend(pairs(("super_massive", 5), (HIDDEN, 3)));
    together.extend(pairs((WATERMELTING, 5), (SUBZERO, 5)));
    let wrong: Vec<String> = (together.iter())
        .filter_map(|ranks| {
            let (sim, engine) = (expected(ranks), engine(ranks));
            let off = (sim - engine).abs() >= 0.01;
            off.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
        })
        .collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn galaxy_swells_leave_the_hit_frames_so_a_use_is_linear_in_their_odds() {
    for rank in 1..=3 {
        let hits = |lucky| {
            let ranks = [(GALAXY, rank)];
            let made = parity::rewritten(CLASS, SKILL, &ranks, 0, |spawns, tree| {
                super::apply_rolled(spawns, tree, lucky)
            });
            parity::play(SKILL, &made, 1).hits
        };
        assert_eq!(hits(true), hits(false), "r{rank}");
    }
}
