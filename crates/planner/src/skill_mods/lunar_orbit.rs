//! Exo's Lunar Orbit: TalentsExo case 358 casts the moons, or Crescent Moon's controller that
//! summons them, on the rings of the player's counter playerEffect[91]; Exo_Lunar_Orbit_obj
//! circles and lunges (Step_0), breaks out (Alarm_10) and bursts into shrapnel (Destroy_0), and
//! ProjectileCollision20Exo rolls the procs on each hit.
use super::Tree;
use crate::skill_effects::{Breakout, Dice, Drain, Lunge, Orbit, Proc, Ramp};
use crate::skill_motion::Accel;
use crate::skill_sim::{Fall, Grow, Spawn};

const FULL_MOON: &str = "full_moon";
const EXTRASOLAR: &str = "extrasolar_moons";
const MAGNETIC: &str = "magnetic_pull";
const LOSS: &str = "loss_of_orbit";
const COLLAPSE: &str = "celestial_collapse";
const TIDALWAVES: &str = "tidalwaves";
const CRESCENT: &str = "crescent_moon";
const ARCHITECT: &str = "lunar_architect";
const BREAKOUT: &str = "orbit_breakout";
const NEBULA: &str = "devastating_nebula";
/// Case 358 L0x3fb-L0x44a: the rings in counter order, px and degrees a frame; the sixth (case 5
/// and default) only Crescent Moon's counter reaches.
const RINGS: [(f32, f32); 6] = [
    (32., 3.),
    (48., 3.),
    (128., 5.),
    (96., 6.),
    (64., 4.),
    (160., 8.),
];
/// L0x447-L0x44a: a cast's counter wraps after the fifth ring.
const CAST_RINGS: u64 = 5;
/// The counter lives on the player between casts: the preview starts on the 48 px ring, whose 3
/// hits are the engine's 0/3/0/6/4 average of 2.6 rounded.
const FIRST_RING: u64 = 1;
/// Uses replayed: two turns of the five rings.
pub(super) const CASTS: u64 = 10;
/// L0x36d, L0x386, L0x3b2: Lunar Architect's 3 moons plus both nodes' counts; Crescent Moon's
/// counter wraps after the same 3 plus the counts (skillProjectiles, L0x3d3).
const MOONS: f32 = 3.;
/// The controller's Create_0 and Step_0: a moon 0.1 s after the cast, then one every second less
/// the node's share (skillFrequency, L0x3ae).
const SUMMON_FIRST: u32 = 6;
const SUMMON_EVERY: f32 = 60.;
/// Exo_Lunar_Orbit_obj: Create_0's 0.75 scale, Step_0 L0x2d-L0x2e circling (x, y + 6), L0x43-L0x49
/// shrinking 0.05 a frame once it fades.
const MOON_SCALE: f32 = 0.75;
const BELOW: f32 = 6.;
const SHRINK: f32 = 0.05;
/// ProjectileCollision20Exo 868: a lunge lasts 2 s over orbitSpeed at orbitSpeed px a frame;
/// Step_0 L0x20 rejoins the circle within 8 px.
const LUNGE_PX: f32 = 120.;
const LUNGE_NEAR: f32 = 8.;
/// Alarm_10 reached 2 frames after the destroyTimer: the moon flies 3 s at orbitSpeed x (2 + the
/// node's speed).
const ALARM_DELAY: u32 = 2;
const BREAKOUT_LIFE: u32 = 180;
const BREAKOUT_SPEED: f32 = 2.;
/// Destroy_0: 5 shrapnel 72 degrees apart from 8 px out; Shrapnel Create_0: 4 px a frame, thrown
/// up at 4 from 16 px under 0.175 gravity.
const SHRAPNEL: &str = "Exo_Lunar_Orbit_Shrapnel_obj";
const SHRAPNEL_COUNT: u32 = 5;
const SHRAPNEL_OUT: f32 = 8.;
const SHRAPNEL_SPEED: f32 = 4.;
const SHRAPNEL_FALL: Fall = Fall {
    height: 16.,
    height_step: 0.,
    z_speed: 4.,
    gravity: 0.175,
};
/// ProjectileCollision20Exo 869 (L0xda-L0xe5): a 0.75 tsunami from the player at the enemy hit;
/// Exo_Tsunami_obj Create_0 and Step_0 speed it from 0.1 up to 4 px a frame for its animation.
const TSUNAMI: &str = "Exo_Tsunami_obj";
const TSUNAMI_SCALE: f32 = 0.75;
const TSUNAMI_SPEED: f32 = 4.;
const TSUNAMI_ACCEL: Accel = Accel {
    from: 0.1,
    step: 0.1,
    rate: 0.05,
    steer: None,
};
const TSUNAMI_LIFE: u32 = 93;
/// ProjectileCollision20Exo 870 (L0xf3-L0xfb) at the enemy hit; Exo_Lunar_Orbit_Nebula_obj
/// Create_0, Step_0: 0.1 growing 0.05 a frame to 1.66, ticking every 0.5 s for 5 s, then its
/// alpha fades 0.075 a frame.
const NEBULA_OBJ: &str = "Exo_Lunar_Orbit_Nebula_obj";
const NEBULA_SCALE: f32 = 0.1;
const NEBULA_GROWTH: f32 = 0.05;
const NEBULA_MAX: f32 = 1.66;
const NEBULA_TICK: f32 = 30.;
const NEBULA_LIFE: u32 = 300;
const NEBULA_FADE: u32 = 14;

/// A moon's on-hit procs, in the order case 358 fills projEffect 869 and 870.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Tidal,
    Nebula,
}

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    let collapse = collapse(tree, |chance| Dice::of_cast(tree.cast).roll(chance));
    cast(spawns, tree, collapse)
}

/// A cast whose collapse roll comes up `lucky`.
#[cfg(test)]
pub(super) fn apply_rolled(spawns: &[Spawn], tree: &Tree, lucky: bool) -> Vec<Spawn> {
    cast(spawns, tree, collapse(tree, |_| lucky))
}

fn cast(spawns: &[Spawn], tree: &Tree, collapse: Option<f32>) -> Vec<Spawn> {
    let one = |base: &Spawn| {
        if tree.learned(CRESCENT) {
            summons(base, tree)
        } else {
            moons(base, tree, collapse)
        }
    };
    spawns.iter().flat_map(one).collect()
}

/// The cast's moons (L0x3e1-L0x44a): one, or Lunar Architect's spread evenly from the aim, on
/// consecutive rings from where the counter stopped, with the cast's collapse roll.
fn moons(base: &Spawn, tree: &Tree, collapse: Option<f32>) -> Vec<Spawn> {
    let count = moon_count(tree);
    let first = FIRST_RING + tree.cast * u64::from(count);
    let cast_moon = |i: u32| {
        let ring = ((first + u64::from(i)) % CAST_RINGS) as usize;
        let at = 360. * i as f32 / count as f32;
        let moon = moon(base, tree, ring, at, &[Kind::Tidal, Kind::Nebula]);
        Spawn {
            on_end: collapse
                .map(|share| shrapnel(tree, share))
                .unwrap_or_default(),
            ..broken_out(moon, tree)
        }
    };
    (0..count).map(cast_moon).collect()
}

/// L0x36f, L0x3b1-L0x3b3: one moon, or Lunar Architect's 3 plus both nodes' counts.
fn moon_count(tree: &Tree) -> u32 {
    if !tree.learned(ARCHITECT) {
        return 1;
    }
    let more = tree.own(EXTRASOLAR, "projectile_count") + tree.own(ARCHITECT, "projectile_count");
    (MOONS + more).round() as u32
}

/// Crescent Moon (L0x39c-L0x3de): the cast makes only the controller, which summons a moon at
/// the player's aim every skillFrequency, each paying its mana, on the counter's ring wrapping
/// after skillProjectiles; a use summons for one moon's life, as the engine prices it.
fn summons(base: &Spawn, tree: &Tree) -> Vec<Spawn> {
    let every = SUMMON_EVERY * (1. - tree.own(CRESCENT, "faster_cast_rate") / 100.);
    let frames: Vec<u32> = (SUMMON_FIRST..moon_life(base, tree))
        .step_by(every.ceil().max(1.) as usize)
        .collect();
    let rings = crescent_rings(tree);
    let first = FIRST_RING + tree.cast * frames.len() as u64;
    let drain = Drain {
        first: 1,
        every: u32::MAX,
        share: tree.own(CRESCENT, "mana_drain_pct") / 100.,
    };
    let summon = |(i, frame): (usize, &u32)| {
        let ring = ((first + i as u64) % rings).min(RINGS.len() as u64 - 1) as usize;
        Spawn {
            frame: *frame,
            drain: Some(drain),
            ..moon(base, tree, ring, 0., &[Kind::Tidal])
        }
    };
    frames.iter().enumerate().map(summon).collect()
}

/// skillProjectiles: 3 plus Extrasolar Moons' count, plus Lunar Architect's when learned.
fn crescent_rings(tree: &Tree) -> u64 {
    let architect = if tree.learned(ARCHITECT) {
        tree.own(ARCHITECT, "projectile_count")
    } else {
        0.
    };
    (MOONS + tree.own(EXTRASOLAR, "projectile_count") + architect).round() as u64
}

/// One moon on `ring`, circling from `at` degrees past the aim, carrying the procs of `kinds`:
/// Magnetic Pull's speed (L0x384), its hit map cleared per turn of its own (Step_0 L0x31-L0x33),
/// Full Moon's life and the build's size, shrinking 0.05 a frame at the end.
fn moon(base: &Spawn, tree: &Tree, ring: usize, at: f32, kinds: &[Kind]) -> Spawn {
    let (radius, degrees) = RINGS[ring];
    let speed = degrees * (1. + tree.own(MAGNETIC, "projectile_speed_pct") / 100.);
    let scale = MOON_SCALE + tree.mods.size_bonus;
    Spawn {
        orbit: Some(Orbit {
            radius,
            speed,
            below: BELOW,
            at,
            clears: true,
            lunge: lunge(tree, speed),
            ..Orbit::CIRCLE
        }),
        life: Some(moon_life(base, tree)),
        scale: Some([scale; 2]),
        fade: (scale / SHRINK).ceil() as u32,
        shrink: SHRINK,
        procs: procs(tree, kinds, 1.),
        ..base.clone()
    }
}

/// destroyTimer: 5 s x (1 + Full Moon's duration) (L0x36c, L0x3bd), stretched by the build's
/// Skill Duration.
fn moon_life(base: &Spawn, tree: &Tree) -> u32 {
    let duration = tree.own(FULL_MOON, "skill_duration") + tree.mods.duration_pct;
    (base.life.unwrap_or(0) as f32 * (1. + duration / 100.)).round() as u32
}

/// Loss of Orbit (ProjectileCollision20Exo 868, Step_0 L0x1a-L0x29): a hit may send the moon at
/// the enemy and back to its place on the circle.
fn lunge(tree: &Tree, speed: f32) -> Option<Lunge> {
    let (chance, _) = tree.proc(LOSS, "of_total_damage")?;
    Some(Lunge {
        chance,
        frames: (LUNGE_PX / speed).ceil() as u32,
        speed,
        near: LUNGE_NEAR,
    })
}

/// Orbit Breakout (Alarm_10): when its destroyTimer runs out the moon flies on along its turn for
/// 3 s, its damage x (1 + the node's damage), then fades.
fn broken_out(moon: Spawn, tree: &Tree) -> Spawn {
    let Some(orbit) = moon.orbit.filter(|_| tree.learned(BREAKOUT)) else {
        return moon;
    };
    let at = moon.life.unwrap_or(0) + ALARM_DELAY;
    let speed = BREAKOUT_SPEED + tree.own(BREAKOUT, "projectile_speed_pct") / 100.;
    Spawn {
        orbit: Some(Orbit {
            breakout: Some(Breakout {
                at,
                speed: orbit.speed * speed,
            }),
            ..orbit
        }),
        life: Some(at + BREAKOUT_LIFE),
        ramp: Some(Ramp {
            every: at,
            rate: tree.own(BREAKOUT, "cold_skill_damage") / 100.,
            from: 0,
        }),
        ..moon
    }
}

/// Celestial Collapse (L0x389-L0x38a): rolled once a cast by `roll`; then every moon of the cast
/// bursts into shrapnel dealing the node's share of its damage.
fn collapse(tree: &Tree, roll: impl FnOnce(f32) -> bool) -> Option<f32> {
    let (chance, share) = tree.proc(COLLAPSE, "of_total_damage")?;
    roll(chance).then_some(share / 100.)
}

/// Destroy_0: the shrapnel inherit all the moon's procs (InheritModifiers with no exception).
fn shrapnel(tree: &Tree, share: f32) -> Vec<Spawn> {
    let piece = |i: u32| Spawn {
        object: SHRAPNEL.into(),
        turn: Some(360. * i as f32 / SHRAPNEL_COUNT as f32),
        offset: SHRAPNEL_OUT,
        speed: SHRAPNEL_SPEED,
        fall: Some(SHRAPNEL_FALL),
        share: Some(share),
        procs: procs(tree, &[Kind::Tidal, Kind::Nebula], share),
        ..Spawn::default()
    };
    (0..SHRAPNEL_COUNT).map(piece).collect()
}

/// The procs of `kinds` an object dealing `share` of the hit rolls on each hit; what a proc
/// creates rolls the rest (InheritModifiers leaves out only its own).
fn procs(tree: &Tree, kinds: &[Kind], share: f32) -> Vec<Proc> {
    let proc = |&kind: &Kind| {
        let rest: Vec<Kind> = kinds.iter().copied().filter(|k| *k != kind).collect();
        let (chance, spawn) = match kind {
            Kind::Tidal => {
                let (chance, part) = tree.proc(TIDALWAVES, "of_total_damage")?;
                (chance, tsunami(tree, share * part / 100., &rest))
            }
            Kind::Nebula => {
                let (chance, more) = tree.proc(NEBULA, "of_total_damage")?;
                (chance, nebula(tree, share * (1. + more / 100.), &rest))
            }
        };
        Some(Proc {
            chance,
            spawn,
            pull: None,
            once: false,
        })
    };
    kinds.iter().filter_map(proc).collect()
}

/// Tidalwaves: floor(damage x tidalwaves) for the tsunami.
fn tsunami(tree: &Tree, share: f32, rest: &[Kind]) -> Spawn {
    Spawn {
        object: TSUNAMI.into(),
        at_player: true,
        aim: true,
        speed: TSUNAMI_SPEED,
        accel: Some(TSUNAMI_ACCEL),
        scale: Some([TSUNAMI_SCALE; 2]),
        life: Some(TSUNAMI_LIFE),
        share: Some(share),
        procs: procs(tree, rest, share),
        ..Spawn::default()
    }
}

/// Devastating Nebula: floor(damage x (1 + nebula)) a tick; its slow shows nothing.
fn nebula(tree: &Tree, share: f32, rest: &[Kind]) -> Spawn {
    Spawn {
        object: NEBULA_OBJ.into(),
        at_enemy: true,
        scale: Some([NEBULA_SCALE; 2]),
        grow: Some(Grow {
            step: NEBULA_GROWTH,
            max: Some(NEBULA_MAX),
            ..Grow::default()
        }),
        tick: NEBULA_TICK,
        life: Some(NEBULA_LIFE),
        fade: NEBULA_FADE,
        share: Some(share),
        procs: procs(tree, rest, share),
        ..Spawn::default()
    }
}

#[cfg(test)]
#[path = "lunar_orbit_parity_tests.rs"]
mod parity;
#[cfg(test)]
#[path = "lunar_orbit_tests.rs"]
mod tests;
