//! Pyromancer's Blazing Trail (Sep 16 exe): TalentsPyromancer case 20 (L0x1e-L0x82) adds Flaming
//! Steps' buff and makes one unseen Blazing_Trail_Controller_obj at the player, or with Circle of
//! Fire 5 × rank of them walking out 360 / n° apart. A controller's Step_0 leaves a patch, a comet
//! or a mine at (x, y + 6) once that is over floor(42 × skillScale) px from its last drop, so the
//! preview's standing player leaves nothing. A patch's Alarm_0 rolls Forking Flames, then Crackling
//! Heat; its Step_0 forks every 0.25 s and pulses every second.
use super::{Tree, pyro};
use crate::skill_effects::{Buff, Dice};
use crate::skill_motion::Trail;
use crate::skill_sim::{self, Fall, Placement, Spawn};

const SKILL: &str = "blazing_trail";
const TIMED: &str = "timed_fire";
const EXPANDED: &str = "expanded_flames";
const FLAMING: &str = "flaming_steps";
const CRACKLING: &str = "crackling_heat";
const FUELED: &str = "fueled_flames";
const FORKING: &str = "forking_flames";
const CIRCLE: &str = "circle_of_fire";
const COMETS: &str = "trail_of_comets";
const FIELD: &str = "detonation_field";
/// Casts replayed, so the patches' rolls show their odds.
pub(super) const CASTS: u64 = 8;
/// L0x1f, L0x50, L0x59: skillTimer 5 s, 2 s with Circle of Fire, stretched by Timed Fire.
const TIMER: f32 = 300.;
const CIRCLE_TIMER: f32 = 120.;
/// L0x20, L0x5b: skillTickRate 0.5 s less Fueled Flames' share.
const TICK: f32 = 30.;
/// L0x3c: BuffAdd 307 for GetTalentInfo(20, 25), 5 s.
const BUFF_FRAMES: u32 = 300;
/// L0x7c-L0x7f: rays 360 / n° apart from 0°, walking deltaSpeed 5 px a frame.
const WALK: f32 = 5.;
const FULL_TURN: f32 = 360.;
/// Controller Step_0 L0xd-L0x29: a drop at (x, y + 6) once that is over floor(42 × skillScale)
/// px from the last drop's (x, y).
const BELOW: f32 = 6.;
const GAP: f32 = 42.;
/// Unseen: case 20's loop making the controllers, and what a forking patch's Step leaves.
const CAST: &str = "TalentsPyromancer_20";
const FORK_STEP: &str = "Pyromancer_Blazing_Trail_obj_Step_0";
/// Blazing_Trail_obj Step_0 L0x3e-L0x58: each forkTimer (0.25 s) the next of 10 forks, sprite_width
/// (48 × scale) on; its carrier drops a frame early to make it on time.
const FORK_EVERY: u32 = 15;
const FORK_DROP: u32 = FORK_EVERY - 2;
const FORKS: u32 = 10;
const SPRITE_WIDTH: f32 = 48.;
/// Alarm_0 L0xf: the fork heads irandom(180) + point_direction(patch, player) - 90.
const FORK_SPREAD: u32 = 180;
const FORK_TURN: f32 = 90.;
/// Create_0 and Step_0 L0x25-L0x39: pulseTimer 1 s, then a Firenova with maxScale 1.
const PULSE_EVERY: u32 = 60;
const PULSE_MAX: f32 = 1.;
/// Trail_of_Comets_obj Create_0: from yy 320 under gravityPower 0.25; Destroy_0 L0x2-L0xe: a 3.5
/// big explosion taking Pyromancer_Explosion_spr as sprite and mask.
const COMET: &str = "Pyromancer_Trail_of_Comets_obj";
const COMET_HEIGHT: f32 = 320.;
const COMET_GRAVITY: f32 = 0.25;
const COMET_BLAST: &str = "Explosion_Item_obj:Pyromancer_Explosion_spr";
const COMET_BLAST_SCALE: f32 = 3.5;
/// Blazing_Detonation_Field_obj: 10 s, or 2 s from the first Step whose distance_to_object finds an
/// enemy under 48 px (Step_0 L0x17-L0x1d); Destroy_0 L0xb: a 4 big explosion.
const MINE: &str = "Pyromancer_Blazing_Detonation_Field_obj";
const MINE_LIFE: u32 = 601;
const ARMED_LIFE: u32 = 122;
const MINE_REACH: f32 = 48.;
/// Half the 32 px mine's and the 24 px dummy's boxes.
const MINE_HALF: f32 = 16.;
const DUMMY_HALF: f32 = 12.;
const MINE_BLAST: f32 = 4.;

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    let [controller] = spawns else {
        return spawns.to_vec();
    };
    let cast = Cast::new(tree);
    if cast.rays == 0 {
        let standing = Spawn {
            life: Some(cast.timer + 1),
            buff: cast.buff(),
            ..controller.clone()
        };
        return vec![standing];
    }
    let mut dice = Dice::of_cast(tree.cast);
    let walkers = (0..cast.rays)
        .map(|i| cast.walker(controller, i, &mut dice))
        .collect();
    vec![Spawn {
        object: CAST.into(),
        ability: true,
        layout: vec![Placement {
            frame: 0,
            offset: [0., BELOW],
        }],
        scale: Some([0.; 2]),
        life: Some(0),
        buff: cast.buff(),
        on_end: walkers,
        ..Spawn::default()
    }]
}

/// Controller Step_0 for one walking `heading`: the moves after which it drops, over its
/// `timer` Steps, each checking before its move.
fn drops(heading: f32, scale: f32, timer: u32) -> Vec<u32> {
    let gap = (GAP * scale).floor();
    let mut spot = [0., 0.];
    (1..timer)
        .filter(|&moves| {
            let at = pyro::lengthdir(WALK * moves as f32, heading);
            let due = (at[0] - spot[0]).hypot(at[1] + BELOW - spot[1]) > gap;
            if due {
                spot = at;
            }
            due
        })
        .collect()
}

/// What a controller leaves (Step_0 L0xe-L0x13): a comet with Trail of Comets, else a mine with
/// Detonation Field, else a patch.
#[derive(Clone, Copy)]
enum Leaves {
    Patch,
    Comet,
    Mine,
}

/// What case 20 reads from the subtree for every controller (L0x33-L0x56).
struct Cast<'a> {
    tree: &'a Tree<'a>,
    /// skillScale: 1 + Expanded Flames' area.
    scale: f32,
    /// skillTickRate in frames.
    tick: f32,
    /// skillTimer in frames.
    timer: u32,
    rays: u32,
    /// Forking Flames' chance and what its forks' damage is multiplied by.
    fork: Option<(f32, f32)>,
    crackle: f32,
    leaves: Leaves,
}

impl<'a> Cast<'a> {
    fn new(tree: &'a Tree<'a>) -> Self {
        let part = |node: &str, key: &str| tree.own(node, key) / 100.;
        let rays = tree.own(CIRCLE, "trail_count").round() as u32;
        let timer = if rays > 0 { CIRCLE_TIMER } else { TIMER };
        let leaves = match (tree.learned(COMETS), tree.learned(FIELD)) {
            (true, _) => Leaves::Comet,
            (false, true) => Leaves::Mine,
            _ => Leaves::Patch,
        };
        let fork = tree.proc(FORKING, "fire_skill_damage");
        Self {
            tree,
            scale: 1. + part(EXPANDED, "area_of_effect"),
            tick: TICK * (1. - part(FUELED, "faster_cast_rate")),
            timer: (timer * (1. + part(TIMED, "skill_duration"))).round() as u32,
            rays,
            fork: fork.map(|(chance, damage)| (chance, 1. + damage / 100.)),
            crackle: tree.proc(CRACKLING, "").map_or(0., |(chance, _)| chance),
            leaves,
        }
    }

    fn bonus(&self) -> f32 {
        self.tree.mods.size_bonus
    }

    /// Flaming Steps' movement speed for 5 s.
    fn buff(&self) -> Option<Buff> {
        self.tree.learned(FLAMING).then(|| Buff {
            frames: BUFF_FRAMES,
            percent: self.tree.own(FLAMING, "movement_speed"),
        })
    }

    /// Controller `i` (L0x7c-L0x7f) walking from (0, 6), so its drops sit where Step_0 makes them.
    fn walker(&self, controller: &Spawn, i: u32, dice: &mut Dice) -> Spawn {
        let heading = FULL_TURN / self.rays as f32 * i as f32;
        let start = [0., BELOW];
        let drops: Vec<(u32, Spawn)> = drops(heading, self.scale, self.timer)
            .into_iter()
            .map(|moves| {
                let [x, y] = pyro::lengthdir(WALK * moves as f32, heading);
                (moves, self.leave([x, y + BELOW], dice))
            })
            .collect();
        Spawn {
            turn: Some(heading - pyro::toward([0., 0.], start)),
            speed: WALK,
            life: Some(self.timer + 1),
            trail: pyro::once_each(drops.into_iter()),
            ..controller.clone()
        }
    }

    fn leave(&self, at: [f32; 2], dice: &mut Dice) -> Spawn {
        match self.leaves {
            Leaves::Patch => self.patch(at, self.scale + self.bonus(), dice),
            Leaves::Comet => self.comet(at, dice),
            Leaves::Mine => self.mine(at),
        }
    }

    /// A patch at `at` growing to `max`; Alarm_0 rolls Forking Flames (L4-L0xf), whose fork and
    /// its own damage grow by the node, then Crackling Heat (L0x1b).
    fn patch(&self, at: [f32; 2], max: f32, dice: &mut Dice) -> Spawn {
        let fork = self.fork.filter(|&(chance, _)| dice.roll(chance));
        let heading = fork.map(|_| {
            let back = pyro::toward(at, [0., 0.]);
            back - FORK_TURN + pyro::irandom(dice, FORK_SPREAD)
        });
        let crackles = dice.roll(self.crackle);
        let share = fork.map_or(1., |(_, damage)| damage);
        let forks = heading.and_then(|heading| self.forks(at, max, share, heading, dice));
        self.sized(max, share, crackles, forks)
    }

    /// The forks of a patch at `at` growing to `max` (Step_0 L0x43-L0x56): each sprite_width past
    /// its parent, as big as the parent then, which Alarm_11 grows again.
    fn fork_spots(&self, at: [f32; 2], max: f32, heading: f32) -> Vec<([f32; 2], f32)> {
        let next = |&(parent, parent_max): &([f32; 2], f32)| {
            let grown = pyro::patch_scale(FORK_EVERY, parent_max, self.bonus());
            let [x, y] = pyro::lengthdir(SPRITE_WIDTH * grown, heading);
            Some(([parent[0] + x, parent[1] + y], grown + self.bonus()))
        };
        let spots = std::iter::successors(Some((at, max)), next).skip(1);
        spots.take(FORKS as usize).collect()
    }

    /// Each fork made by a carrier its parent drops; a fork rolls Crackling Heat, never forking.
    fn forks(
        &self,
        at: [f32; 2],
        max: f32,
        share: f32,
        heading: f32,
        dice: &mut Dice,
    ) -> Option<Box<Trail>> {
        let spots = self.fork_spots(at, max, heading);
        let crackles: Vec<bool> = spots.iter().map(|_| dice.roll(self.crackle)).collect();
        let parents: Vec<[f32; 2]> = std::iter::once(at)
            .chain(spots.iter().map(|(spot, _)| *spot))
            .collect();
        let links: Vec<_> = parents.into_iter().zip(spots).zip(crackles).collect();
        links
            .into_iter()
            .rev()
            .fold(None, |next, ((parent, (spot, max)), crackles)| {
                let carrier = Spawn {
                    object: FORK_STEP.into(),
                    ability: true,
                    turn: Some(heading - pyro::toward([0., 0.], parent)),
                    offset: (spot[0] - parent[0]).hypot(spot[1] - parent[1]),
                    scale: Some([0.; 2]),
                    life: Some(0),
                    on_end: vec![self.sized(max, share, crackles, next)],
                    ..Spawn::default()
                };
                pyro::once_each(std::iter::once((FORK_DROP, carrier)))
            })
    }

    /// A patch with its pulses, if it crackles, and its next fork.
    fn sized(&self, max: f32, share: f32, crackles: bool, fork: Option<Box<Trail>>) -> Spawn {
        let pulses = |next| Trail {
            every: PULSE_EVERY,
            first: PULSE_EVERY - 1,
            gap: 0.,
            spawn: self.pulse(share),
            next,
        };
        let trail = if crackles {
            Some(Box::new(pulses(fork)))
        } else {
            fork
        };
        Spawn {
            trail,
            ..pyro::sized_patch(self.tree.mods, share, max, self.tick)
        }
    }

    /// Crackling Heat's Firenova on its patch with the patch's damage; Alarm_11 adds the size bonus.
    fn pulse(&self, share: f32) -> Spawn {
        Spawn {
            share: Some(share),
            ..pyro::firenova(PULSE_MAX + self.bonus())
        }
    }

    /// A comet falling where its controller is, then its explosion and its patch, as big as the
    /// comet that Alarm_11 grew by the size bonus (Destroy_0 L0x2-L0x2e; its shrapnel is drawn
    /// only and left out).
    fn comet(&self, at: [f32; 2], dice: &mut Dice) -> Spawn {
        let blast = Spawn {
            object: COMET_BLAST.into(),
            ..pyro::blast(COMET_BLAST_SCALE)
        };
        let patch = self.patch(at, self.scale + 2. * self.bonus(), dice);
        Spawn {
            object: COMET.into(),
            ability: true,
            fall: Some(Fall {
                height: COMET_HEIGHT,
                gravity: COMET_GRAVITY,
                ..Fall::default()
            }),
            on_end: vec![blast, patch],
            ..Spawn::default()
        }
    }

    /// A mine at `at`, `skillScale` big: armed on its first Step when its box is within 48 px of
    /// the dummy's, which never moves, it explodes 2 s later, else when its 10 s run out.
    fn mine(&self, at: [f32; 2]) -> Spawn {
        let dummy = skill_sim::skill(SKILL).map_or(0., |fx| fx.dummy);
        let half = MINE_HALF * self.scale + DUMMY_HALF;
        let gap = |d: f32| (d.abs() - half).max(0.);
        let armed = gap(at[0] - dummy).hypot(gap(at[1])) < MINE_REACH;
        Spawn {
            object: MINE.into(),
            ability: true,
            scale: Some([self.scale; 2]),
            life: Some(if armed { ARMED_LIFE } else { MINE_LIFE }),
            on_end: vec![pyro::blast(MINE_BLAST)],
            ..Spawn::default()
        }
    }
}

#[cfg(test)]
#[path = "blazing_trail_parity_tests.rs"]
mod parity;

#[cfg(test)]
#[path = "blazing_trail_tests.rs"]
mod tests;
