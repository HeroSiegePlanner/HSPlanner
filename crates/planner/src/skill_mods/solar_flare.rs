//! Exo's Solar Flare: TalentsExo case 346 casts a fan of flares, Sun's Orbit's circling ones, a
//! Grand Flare or Solar Orbs that ring flares out; Exo_Solar_Flare_obj homes on the enemy and
//! explodes where its centre meets it, and ProjectileCollision20Exo rolls Explosive Flares.
use super::Tree;
use crate::skill_effects::{Dice, Orbit, Proc};
use crate::skill_sim::{Placement, Spawn};

const FIREWORKS: &str = "solar_fireworks";
const EXPLOSIVE: &str = "explosive_flares";
const EVERBURNING: &str = "everburning_fire";
const BURST: &str = "burst_of_power";
const GRAND: &str = "grand_flare";
const SUNS_ORBIT: &str = "suns_orbit";
const SEARING: &str = "searing_flares";
const ORBS: &str = "solar_orbs";
/// Casts replayed, so the on-cast rolls of Burst of Power and Everburning Fire show.
pub(super) const CASTS: u64 = 8;
/// L0xa5: three flares a cast.
const FLARES: u32 = 3;
/// L0x119-L0x12a: 12° apart about the aim, each 0.25 px a frame faster than the one before.
const FAN: f32 = 12.;
const SPEED_STEP: f32 = 0.25;
/// Sun's Orbit (L0x120-L0x136, Step_0 L7-L0xd): circling (x, y + 6) from 24 px, the circle
/// widening 1.5 px a frame (Create_0 orbitRadiusSpeed), facing along it, for 3 s.
const ORBIT_BELOW: f32 = 6.;
const ORBIT_SPREAD: f32 = 1.5;
const ORBIT_FACE: f32 = 90.;
const ORBIT_LIFE: u32 = 180;
/// Grand Flare (L0x102-L0x10c, Create_0): 10 px out, 3 times as big, 4 px a frame for 2 s.
const GRAND_FLARE: &str = "Exo_Solar_Flare_Grand_Flare_obj";
const GRAND_OUT: f32 = 10.;
const GRAND_SCALE: f32 = 3.;
const GRAND_SPEED: f32 = 4.;
const GRAND_LIFE: u32 = 120;
/// L0xd2: flareTimerMax = max(FPS / 2 - the node's share / 2, FPS / 10).
const GRAND_EVERY: f32 = 30.;
const GRAND_EVERY_MIN: f32 = 6.;
/// Solar Orbs (L0xeb-L0xf7, Solar_Orb Alarm_0): 32 px out, shown at 0.75 from their second
/// frame, a ring every 1.25 s for the node's seconds.
const ORB: &str = "Exo_Solar_Flare_Solar_Orb_obj";
const ORB_OUT: f32 = 32.;
const ORB_SCALE: f32 = 0.75;
const ORB_EVERY: u32 = 75;
const FPS: f32 = 60.;
/// Grand_Flare and Solar_Orb Step_0: the first ring 0.25 s in, its flares 10 px out, 360/n
/// apart from the emitter's heading, 3 + 0.25 i px a frame.
const RING_FIRST: u32 = 15;
const RING_OUT: f32 = 10.;
const RING_SPEED: f32 = 3.;

/// What a cast rolls before its flares exist (L0xc2-L0xc8): Everburning Fire, then Burst of Power.
struct Rolls {
    burning: bool,
    burst: bool,
}

fn rolls(tree: &Tree, mut roll: impl FnMut(f32) -> bool) -> Rolls {
    let mut comes_up = |node: &str| tree.proc(node, "").is_some_and(|(chance, _)| roll(chance));
    let burning = comes_up(EVERBURNING);
    Rolls {
        burning,
        burst: comes_up(BURST),
    }
}

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    let mut dice = Dice::of_cast(tree.cast);
    cast(spawns, tree, &rolls(tree, |chance| dice.roll(chance)))
}

/// A cast whose Everburning Fire roll comes up `burning` and Burst of Power roll `burst`.
#[cfg(test)]
pub(super) fn apply_rolls(spawns: &[Spawn], tree: &Tree, burning: bool, burst: bool) -> Vec<Spawn> {
    cast(spawns, tree, &Rolls { burning, burst })
}

fn cast(spawns: &[Spawn], tree: &Tree, rolls: &Rolls) -> Vec<Spawn> {
    spawns
        .iter()
        .flat_map(|spawn| cast_of(spawn, tree, rolls))
        .collect()
}

/// L0xeb-L0x139: the cast makes Solar Orbs, else a Grand Flare, else its flares; the base cast
/// is an unseen flare ending at once whose flares each keep their own turn and speed.
fn cast_of(cast: &Spawn, tree: &Tree, rolls: &Rolls) -> Vec<Spawn> {
    let Some(first) = cast.on_end.first() else {
        return vec![cast.clone()];
    };
    let flare = armed(first, tree, rolls);
    let count = flare_count(tree, rolls);
    if tree.learned(ORBS) {
        return orbs(&flare, count, tree);
    }
    if tree.learned(GRAND) {
        return grand(&flare, count, tree);
    }
    let flares = if tree.learned(SUNS_ORBIT) {
        circling(&flare, count)
    } else {
        fanned(&flare, count)
    };
    vec![Spawn {
        on_end: flares,
        ..cast.clone()
    }]
}

/// L0xb8-L0xda: 3 flares, Solar Fireworks', Sun's Orbit's and Searing Flares' and a lucky Burst
/// of Power's; Solar's Edge's kill stacks never come on a dummy that never dies.
fn flare_count(tree: &Tree, rolls: &Rolls) -> u32 {
    let own = [FIREWORKS, SUNS_ORBIT, SEARING].map(|node| tree.own(node, "projectile_count"));
    let burst = tree
        .proc(BURST, "projectile_count")
        .filter(|_| rolls.burst)
        .map_or(0., |(_, more)| more);
    FLARES + (own.iter().sum::<f32>() + burst).round() as u32
}

/// The cast's flare, widened by the build's size bonus, its explosion dealing the cast's share.
fn armed(first: &Spawn, tree: &Tree, rolls: &Rolls) -> Spawn {
    let scale = first.scale.map_or(1., |[x, _]| x) + tree.mods.size_bonus;
    let share = burst_share(tree, rolls);
    Spawn {
        scale: Some([scale; 2]),
        on_end: first
            .on_end
            .iter()
            .map(|blast| exploding(blast, tree, rolls, share))
            .collect(),
        ..first.clone()
    }
}

/// L0xc7, L0xe1: a lucky Burst of Power adds its fire damage to the subtree's the hit holds.
fn burst_share(tree: &Tree, rolls: &Rolls) -> f32 {
    let bonus = tree
        .proc(BURST, "fire_skill_damage")
        .filter(|_| rolls.burst)
        .map_or(0., |(_, damage)| damage);
    let subtree = 100. + tree.stat("subtree_damage");
    (subtree + bonus) / subtree
}

/// ProjectileCollision20Exo L0x139-L0x14d: on a burning enemy a hit may set off one more
/// explosion where it is, dealing (1 + the node's share) of its damage and rolling no further;
/// only Everburning Fire's roll burns a dummy, so a cast that missed it sets none off.
fn exploding(blast: &Spawn, tree: &Tree, rolls: &Rolls, share: f32) -> Spawn {
    let proc = tree
        .proc(EXPLOSIVE, "of_total_damage")
        .filter(|_| tree.learned(EVERBURNING))
        .map(|(chance, more)| Proc {
            chance: if rolls.burning { chance } else { 0. },
            spawn: Spawn {
                share: Some(share * (1. + more / 100.)),
                ..blast.clone()
            },
            pull: None,
            once: false,
        });
    Spawn {
        share: Some(share),
        procs: proc.into_iter().collect(),
        ..blast.clone()
    }
}

/// L0x11c-L0x12a: the flares fanned 12° apart about the aim, each 0.25 px a frame faster.
fn fanned(flare: &Spawn, count: u32) -> Vec<Spawn> {
    let middle = (count - 1) as f32 / 2.;
    let fan = |i: u32| Spawn {
        turn: Some(FAN * (i as f32 - middle)),
        speed: flare.speed + SPEED_STEP * i as f32,
        ..flare.clone()
    };
    (0..count).map(fan).collect()
}

/// Sun's Orbit: flare i starts 24 px out at 360/n·i past the aim and from its first Step circles
/// at its speed in degrees a frame (Alarm_0 orbitSpeed = deltaSpeed); it touches like any flare.
fn circling(flare: &Spawn, count: u32) -> Vec<Spawn> {
    let circle = |i: u32| Spawn {
        turn: Some(360. * i as f32 / count as f32),
        speed: 0.,
        orbit: Some(Orbit {
            radius: flare.offset,
            speed: flare.speed + SPEED_STEP * i as f32,
            below: ORBIT_BELOW,
            from: 1,
            turns: true,
            spread: ORBIT_SPREAD,
            face: ORBIT_FACE,
            ..Orbit::CIRCLE
        }),
        life: Some(ORBIT_LIFE),
        ..flare.clone()
    };
    (0..count).map(circle).collect()
}

/// Grand Flare: one flare 3 times as big flying out along the aim, ringing the cast's flares
/// out 0.25 s in, then every flareTimerMax while it lives.
fn grand(flare: &Spawn, count: u32, tree: &Tree) -> Vec<Spawn> {
    let emitter = Spawn {
        object: GRAND_FLARE.into(),
        offset: GRAND_OUT,
        speed: GRAND_SPEED,
        ability: true,
        scale: Some([GRAND_SCALE; 2]),
        life: Some(GRAND_LIFE),
        ..Spawn::default()
    };
    let rings = ring_frames(grand_every(tree), GRAND_LIFE)
        .map(|frame| (frame, [GRAND_OUT + GRAND_SPEED * frame as f32, 0.]))
        .collect();
    vec![emitter, ring(flare, count, rings)]
}

/// The node's share comes off frames, not seconds, so every rank keeps 30 frames.
fn grand_every(tree: &Tree) -> u32 {
    let share = tree.own(GRAND, "faster_cast_rate") / 100.;
    (GRAND_EVERY - share / 2.).max(GRAND_EVERY_MIN).ceil() as u32
}

/// Solar Orbs: the node's count of suns 32 px out, 360/r apart from the aim, each ringing the
/// cast's flares out 0.25 s in, then every 1.25 s, for the node's seconds (L0xf7 destroyTimer).
fn orbs(flare: &Spawn, count: u32, tree: &Tree) -> Vec<Spawn> {
    let suns = tree.own(ORBS, "projectile_count").round().max(1.) as u32;
    let life = (tree.own(ORBS, "skill_duration") * FPS).round() as u32;
    let places: Vec<[f32; 2]> = (0..suns)
        .map(|j| {
            let (sin, cos) = (360. * j as f32 / suns as f32).to_radians().sin_cos();
            [cos * ORB_OUT, -sin * ORB_OUT]
        })
        .collect();
    let orb = Spawn {
        object: ORB.into(),
        ability: true,
        scale: Some([ORB_SCALE; 2]),
        life: Some(life),
        layout: places
            .iter()
            .map(|&offset| Placement { frame: 0, offset })
            .collect(),
        ..Spawn::default()
    };
    let rings = places
        .iter()
        .flat_map(|&place| ring_frames(ORB_EVERY, life).map(move |frame| (frame, place)))
        .collect();
    vec![orb, ring(flare, count, rings)]
}

/// The Steps an emitter rings flares out on: 0.25 s in, then every `every` frames it lives.
fn ring_frames(every: u32, life: u32) -> impl Iterator<Item = u32> {
    (RING_FIRST..=life).step_by(every.max(1) as usize)
}

/// Rings of `count` flares let out at each (Step, place): an unseen flare ending the frame
/// before, so its ring is made on that Step, 10 px out, turned 360/n apart from the line from
/// the player through it.
fn ring(flare: &Spawn, count: u32, rings: Vec<(u32, [f32; 2])>) -> Spawn {
    let ringed = |i: u32| Spawn {
        at_player: false,
        turn: Some(360. * i as f32 / count as f32),
        offset: RING_OUT,
        speed: RING_SPEED + SPEED_STEP * i as f32,
        ..flare.clone()
    };
    Spawn {
        object: flare.object.clone(),
        ability: true,
        scale: Some([0.; 2]),
        life: Some(0),
        layout: rings
            .into_iter()
            .map(|(frame, offset)| Placement {
                frame: frame - 1,
                offset,
            })
            .collect(),
        on_end: (0..count).map(ringed).collect(),
        ..Spawn::default()
    }
}

#[cfg(test)]
#[path = "solar_flare_parity_tests.rs"]
mod parity;
#[cfg(test)]
#[path = "solar_flare_tests.rs"]
mod tests;
