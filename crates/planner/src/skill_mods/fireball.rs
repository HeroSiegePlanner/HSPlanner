//! Pyromancer's Fireball: TalentsPyromancer case 29 rolls the cast and makes its balls, the
//! ball's Destroy_0 leaves the explosion and splits, and ProjectileCollision02Pyromancer makes
//! the cast's Mark of Doom meteor and fireflies on each carrier's first hit.
use super::Tree;
use super::pyro::{self, sure};
use crate::hitbox;
use crate::skill_effects::{Dice, Orbit, Proc, Slowdown};
use crate::skill_sim::{self, Placement, Spawn};

const SKILL: &str = "fireball";
const MULTICAST: &str = "multicast";
const IGNITION: &str = "ignition_radius";
const SEARING: &str = "searing_speed";
const PARALYZING: &str = "paralyzing_impact";
const CRITICAL: &str = "critical_burn";
const DOOM: &str = "mark_of_doom";
const SPLITFIRE: &str = "splitfire";
const PATH: &str = "path_of_destruction";
const ORBITAL: &str = "orbital_fire";
const FIREFLIES: &str = "fireflies";
/// Casts replayed, so the on-cast rolls show their odds.
pub(super) const CASTS: u64 = 8;
/// L0x26f alarm[2] = 1 + 12 × volley: only the plain ball waits for it (Alarm_2 `activated`).
const VOLLEY_DELAY: u32 = 12;
/// Destroy_0 L0x1c: the explosion is (1 + explosionSize) × 0.8 big.
const EXPLOSION_SCALE: f32 = 0.8;
/// Destroy_0 L0x31-L0x3a: the Split_obj balls (pyro::split) 8 px out, (4 + count)° apart.
const SPLIT: &str = "Pyromancer_Fire_Ball_Split_obj";
const SPLIT_OUT: f32 = 8.;
const SPLIT_APART: f32 = 4.;
/// L0x27f-L0x283, Orbital_obj Step_0: 8 px out, +1 px and 10° a frame slowed to 2.
const ORBIT_RADIUS: f32 = 8.;
const ORBIT_GROWTH: f32 = 1.;
const ORBIT_SPEED: f32 = 10.;
const ORBIT_SLOWDOWN: Slowdown = Slowdown {
    rate: 0.015,
    floor: 2.,
};
/// L0x234: Orbital Fire's balls last 0.45 of the ball's 2 s.
const ORBITAL_TIME: f32 = 0.45;
/// Split_obj Step_0 L0xb: a patch per 32 × explosionSize px; Path's size never grows the patch.
const PATCH_GAP: f32 = 32.;
/// The 336 handler (L0x101-L0x110): Armageddon_obj at the enemy, yy 320.
const METEOR_HEIGHT: f32 = 320.;

/// What a cast rolls before its balls exist (L0x20f-L0x252), each roll its own draw.
struct Rolls {
    /// LSSD the lucky rolls add, in percent.
    subtree: f32,
    /// Critical Burn's m = 2 (L0x21a).
    double: bool,
    doom: bool,
    fireflies: bool,
    multicast: bool,
}

/// The cast's rolls, `roll` deciding each node's from its chance.
fn rolls(tree: &Tree, mut roll: impl FnMut(&str, f32) -> bool) -> Rolls {
    let mut lucky = |node: &str, effect: &str| {
        tree.proc(node, effect)
            .filter(|(chance, _)| roll(node, *chance))
            .map(|(_, value)| value)
    };
    let paralyzing = lucky(PARALYZING, "subtree_damage_on_proc");
    let double = lucky(CRITICAL, "").is_some();
    let doom = lucky(DOOM, "subtree_damage_on_proc");
    let fireflies = lucky(FIREFLIES, "subtree_damage_on_proc");
    let multicast = tree.own(MULTICAST, "multicast_chance") / 100.;
    Rolls {
        subtree: [paralyzing, doom, fireflies].into_iter().flatten().sum(),
        double,
        doom: doom.is_some(),
        fireflies: fireflies.is_some(),
        multicast: multicast > 0. && roll(MULTICAST, multicast),
    }
}

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    let mut dice = Dice::of_cast(tree.cast);
    volleys(spawns, tree, &rolls(tree, |_, chance| dice.roll(chance)))
}

/// A cast whose rolls come up as `lucky` says for each node.
#[cfg(test)]
pub(super) fn apply_rolled(
    spawns: &[Spawn],
    tree: &Tree,
    lucky: &dyn Fn(&str) -> bool,
) -> Vec<Spawn> {
    volleys(spawns, tree, &rolls(tree, |node, _| lucky(node)))
}

fn volleys(spawns: &[Spawn], tree: &Tree, rolls: &Rolls) -> Vec<Spawn> {
    spawns
        .iter()
        .flat_map(|base| Cast::new(base, tree, rolls).volleys())
        .collect()
}

/// One cast: its rolls, the base ball, its damage share and explosionSize.
struct Cast<'a> {
    tree: &'a Tree<'a>,
    rolls: &'a Rolls,
    base: &'a Spawn,
    /// floor(LTD × (1 + LSSD) × m) over the engine's hit, which holds the subtree's own LSSD.
    share: f32,
    /// explosionSize: 1 + Ignition Radius' and Path's area (L0x1f4, L0x20b, L0x22d).
    size: f32,
}

impl<'a> Cast<'a> {
    fn new(base: &'a Spawn, tree: &'a Tree<'a>, rolls: &'a Rolls) -> Self {
        let subtree = 100. + tree.stat("subtree_damage");
        let double = if rolls.double { 2. } else { 1. };
        let area = tree.own(IGNITION, "area_of_effect") + tree.own(PATH, "area_of_effect");
        Self {
            tree,
            rolls,
            base,
            share: (subtree + rolls.subtree) / subtree * double,
            size: 1. + area / 100.,
        }
    }

    /// L0x250-L0x25d: the volley and Multicast's; damage-object balls start at once.
    fn volleys(&self) -> Vec<Spawn> {
        let volley = self.volley();
        if !self.rolls.multicast {
            return volley;
        }
        let delay = if self.transformed() { 0 } else { VOLLEY_DELAY };
        let second = volley.iter().map(|spawn| Spawn {
            frame: spawn.frame + delay,
            ..spawn.clone()
        });
        volley.iter().cloned().chain(second).collect()
    }

    fn transformed(&self) -> bool {
        self.tree.learned(PATH) || self.tree.learned(ORBITAL)
    }

    /// L0x23f-L0x246: Path's piercing ball wins over Orbital Fire's, else the plain ball.
    fn volley(&self) -> Vec<Spawn> {
        if self.tree.learned(PATH) {
            self.path()
        } else if self.tree.learned(ORBITAL) {
            vec![self.orbital()]
        } else {
            vec![self.ball()]
        }
    }

    /// L0x23b: Searing Speed on the ball's px, or the orbit's degrees, a frame.
    fn faster(&self) -> f32 {
        1. + self.tree.own(SEARING, "projectile_speed_pct") / 100.
    }

    /// L0x233: 1 + Orbital Fire's 4 × rank balls.
    fn orbital_count(&self) -> u32 {
        1 + self.tree.own(ORBITAL, "primary_projectile_count").round() as u32
    }

    /// L0x1f7, L0x234, L0x23d: 2 s, or 0.45 of it, stretched by Orbital's and the build's duration.
    fn damage_ball_life(&self) -> u32 {
        let time = if self.tree.learned(ORBITAL) {
            ORBITAL_TIME
        } else {
            1.
        };
        let duration = self.tree.own(ORBITAL, "skill_duration") + self.tree.mods.duration_pct;
        let life = self.base.life.unwrap_or(0) as f32 * time * (1. + duration / 100.);
        life.ceil() as u32
    }

    /// The plain ball; its Destroy_0 leaves the explosion at the enemy hit, then the splits.
    fn ball(&self) -> Spawn {
        let base = self.base;
        let blast = base.on_end.first().map(|blast| Spawn {
            scale: Some([EXPLOSION_SCALE * (1. + self.size); 2]),
            share: Some(self.share),
            procs: self.flags(),
            ..blast.clone()
        });
        Spawn {
            speed: base.speed * self.faster(),
            on_end: blast.into_iter().chain(self.splits()).collect(),
            ..base.clone()
        }
    }

    /// Destroy_0 L0x30-L0x3d: 3 × rank splits fanned about the ball's direction.
    fn splits(&self) -> Vec<Spawn> {
        let count = self
            .tree
            .own(SPLITFIRE, "secondary_projectile_count")
            .round() as u32;
        let apart = SPLIT_APART + count as f32;
        let middle = (count as f32 - 1.) / 2.;
        (0..count)
            .map(|i| Spawn {
                turn: Some(apart * (i as f32 - middle)),
                ..self.split()
            })
            .collect()
    }

    /// A Split_obj: hits each enemy once with the cast's damage and flags, explodes where it ends.
    fn split(&self) -> Spawn {
        Spawn {
            offset: SPLIT_OUT,
            procs: self.flags(),
            ..pyro::split(self.share)
        }
    }

    /// Split_obj and Orbital_obj Destroy_0: an explosion where they end, 1 + explosionSize big.
    fn end_blast(&self, size: f32) -> Spawn {
        Spawn {
            share: Some(self.share),
            ..pyro::blast(1. + size)
        }
    }

    /// Orbital Fire's balls circle the player and explode on the first enemy they touch.
    fn orbital(&self) -> Spawn {
        let tree = self.tree;
        Spawn {
            object: pyro::ORBITAL.into(),
            orbit: Some(Orbit {
                radius: ORBIT_RADIUS,
                speed: ORBIT_SPEED * self.faster(),
                count: self.orbital_count(),
                turns: true,
                face: pyro::ORBITAL_FACE,
                spread: ORBIT_GROWTH + tree.own(ORBITAL, "area_of_effect") / 100.,
                slowdown: Some(ORBIT_SLOWDOWN),
                ..Orbit::CIRCLE
            }),
            speed: 0.,
            ability: false,
            ends_on_contact: true,
            active_from: None,
            life: Some(self.damage_ball_life()),
            share: Some(self.share),
            procs: self.flags(),
            on_end: vec![self.end_blast(self.size)],
            ..self.base.clone()
        }
    }

    /// Path of Destruction's piercing Split_obj balls, all along the aim (L0x262), with trails.
    fn path(&self) -> Vec<Spawn> {
        let ball = Spawn {
            object: SPLIT.into(),
            speed: self.base.speed * self.faster(),
            ability: false,
            ends_on_contact: false,
            active_from: None,
            life: Some(self.damage_ball_life()),
            share: Some(self.share),
            procs: self.flags(),
            on_end: vec![self.end_blast(self.size)],
            ..self.base.clone()
        };
        let one: Vec<Spawn> = std::iter::once(ball.clone())
            .chain(self.patches(&ball))
            .collect();
        (0..self.orbital_count())
            .flat_map(|_| one.iter().cloned())
            .collect()
    }

    /// Patches dropped before the ball's first hit still carry the flags that hit zeroes.
    fn patches(&self, ball: &Spawn) -> Vec<Spawn> {
        let hit = first_hit(ball).unwrap_or(u32::MAX);
        let (early, late): (Vec<_>, Vec<_>) = drops(ball, PATCH_GAP * self.size)
            .into_iter()
            .partition(|(age, _)| *age < hit);
        let patch = self.patch();
        [(early, self.flags()), (late, Vec::new())]
            .into_iter()
            .filter(|(drops, _)| !drops.is_empty())
            .map(|(drops, procs)| Spawn {
                layout: drops.into_iter().map(|(_, place)| place).collect(),
                procs,
                ..patch.clone()
            })
            .collect()
    }

    /// A Blazing_Trail_obj dealing floor(damage × 0.6 × rank).
    fn patch(&self) -> Spawn {
        let trail = self.tree.own(PATH, "trail_damage_percent") / 100.;
        pyro::patch(self.tree.mods, self.share * trail)
    }

    /// The cast's projEffect 336 and 338 (L0x279, L0x27b), used on each carrier's first hit.
    fn flags(&self) -> Vec<Proc> {
        let doom = self.rolls.doom.then(|| sure(self.meteor()));
        doom.into_iter().chain(self.fireflies()).collect()
    }

    /// The 336 handler runs first and leaves only itself out, so the meteor carries the fireflies.
    fn meteor(&self) -> Spawn {
        let blast = Spawn {
            share: Some(self.share),
            procs: self.fireflies(),
            ..pyro::blast(1.)
        };
        pyro::meteor(METEOR_HEIGHT, blast)
    }

    /// 4 × rank fireflies; Mark of Doom is already zeroed when the 338 handler copies the rest.
    fn fireflies(&self) -> Vec<Proc> {
        let count = self
            .tree
            .proc(FIREFLIES, "secondary_projectile_count")
            .filter(|_| self.rolls.fireflies)
            .map_or(0, |(_, count)| count.round() as u32);
        let round = 360. / count.max(1) as f32;
        (0..count)
            .map(|i| sure(pyro::firefly(round * i as f32, self.share)))
            .collect()
    }
}

/// The frame of the ball's first hit on the dummy, played alone.
fn first_hit(ball: &Spawn) -> Option<u32> {
    let fx = skill_sim::skill(SKILL)?;
    let hitbox = hitbox::skill(SKILL)?;
    let alone = Spawn {
        procs: Vec::new(),
        on_end: Vec::new(),
        ..ball.clone()
    };
    skill_sim::replay_use(fx, hitbox, &[alone], 1, 0)
        .hits
        .first()
        .copied()
}

/// Each drop's ball age and patch: where the ball is once more than `gap` px past the last.
fn drops(ball: &Spawn, gap: f32) -> Vec<(u32, Placement)> {
    let mut last = 0.;
    (1..ball.life.unwrap_or(0))
        .filter_map(|age| {
            let gone = ball.speed * age as f32;
            if gone - last <= gap {
                return None;
            }
            last = gone;
            let place = Placement {
                frame: ball.frame + age + 1,
                offset: [ball.offset + gone, 0.],
            };
            Some((age, place))
        })
        .collect()
}

#[cfg(test)]
#[path = "fireball_parity_tests.rs"]
mod parity;
#[cfg(test)]
#[path = "fireball_tests.rs"]
mod tests;
