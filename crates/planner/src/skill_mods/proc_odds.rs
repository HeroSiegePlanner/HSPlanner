//! Expected dummy hits of a use whose procs roll each other: each spawn at its creation odds, and
//! every kind of chancy proc, told apart by the object it makes, forced on and off and weighted by
//! its odds. A chain rolls each kind at most once, so a use is multilinear in the kinds' chances.
use super::parity_tests as parity;
use crate::skill_effects::Proc;
use crate::skill_motion::Trail;
use crate::skill_sim::Spawn;

/// Damage-weighted hits `spawns` of `skill` are expected to land in one use; only the top-level
/// spawns may roll their creation.
pub(crate) fn expected(skill: &str, spawns: &[Spawn]) -> f32 {
    let one = |spawn: &Spawn| {
        let sure = Spawn {
            chance: None,
            ..spawn.clone()
        };
        spawn.chance.unwrap_or(1.) * at_odds(skill, &sure)
    };
    spawns.iter().map(one).sum()
}

/// The corners of the kinds' chances, each made always or never, weighted by their odds.
fn at_odds(skill: &str, spawn: &Spawn) -> f32 {
    let mut kinds = Vec::new();
    chancy(spawn, &mut kinds);
    let corner = |mask: usize| {
        let on = |i: usize| mask >> i & 1 == 1;
        let odds = kinds.iter().enumerate();
        let weight: f32 = odds
            .map(|(i, (_, p))| if on(i) { *p } else { 1. - p })
            .product();
        let made = |object: &str| kinds.iter().position(|(o, _)| o == object).is_some_and(on);
        weight * parity::weighted(&parity::play(skill, &[forced(spawn, &made)], 1))
    };
    (0..1_usize << kinds.len()).map(corner).sum()
}

/// The objects chancy procs make anywhere in `spawn`, with their odds, in the order first seen.
fn chancy(spawn: &Spawn, kinds: &mut Vec<(String, f32)>) {
    for proc in &spawn.procs {
        let object = &proc.spawn.object;
        if proc.chance < 1. && !kinds.iter().any(|(o, _)| o == object) {
            kinds.push((object.clone(), proc.chance));
        }
        chancy(&proc.spawn, kinds);
    }
    let trail = spawn.trail.as_deref().map(|trail| &trail.spawn);
    for child in spawn.on_end.iter().chain(&spawn.on_low).chain(trail) {
        chancy(child, kinds);
    }
}

/// `spawn` with every chancy proc made always, when `made` says so for its object, or never.
fn forced(spawn: &Spawn, made: &dyn Fn(&str) -> bool) -> Spawn {
    let all = |spawns: &[Spawn]| spawns.iter().map(|s| forced(s, made)).collect();
    let proc = |proc: &Proc| Proc {
        chance: match proc.chance < 1. {
            true if made(&proc.spawn.object) => 1.,
            true => 0.,
            false => proc.chance,
        },
        spawn: forced(&proc.spawn, made),
        ..proc.clone()
    };
    Spawn {
        on_end: all(&spawn.on_end),
        on_low: all(&spawn.on_low),
        trail: (spawn.trail.as_deref()).map(|trail| {
            Box::new(Trail {
                spawn: forced(&trail.spawn, made),
                ..trail.clone()
            })
        }),
        procs: spawn.procs.iter().map(proc).collect(),
        ..spawn.clone()
    }
}
