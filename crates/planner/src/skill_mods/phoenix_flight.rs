//! Pyromancer's Phoenix Flight (Sep 16 exe): TalentsPyromancer case 22 (L0x93-L0x10e) starts a
//! skillsCharge charge with Charge_Controller_obj holding the subskill values and makes
//! Pyromancer_Phoenix_Flight_obj round the player for four of the nodes. skillsCharge steers the
//! flight toward the cursor, leaves a phoenix every 48 px, with Wings of the Phoenix's fan behind
//! it, and ends in the landing novas, Scorching Fury's balls and Rise from the Ashes' leap, which
//! skillsLeap plays out; ProjectileCollision02Pyromancer plants the seed, rolls the chains and
//! lays Burning Wings' patch. The preview's player cannot move: an unseen mover flies for it,
//! drawn as the phoenix.
use super::charge::{self, Stops};
use super::{Tree, pyro};
use crate::skill_effects::{Buff, Dice, Proc};
use crate::skill_homing::point_direction;
use crate::skill_sim::{self, Fall, Placement, Spawn};

const SKILL: &str = "phoenix_flight";
const SWIFT: &str = "swift_flight";
const FURY: &str = "scorching_fury";
const INFUSED: &str = "infused_flames";
const SEARING: &str = "searing_touch";
const SCREECH: &str = "burning_screech";
const COLLATERAL: &str = "collateral_burning";
const AFTERBURN: &str = "afterburn";
const FEATHERS: &str = "molten_feathers";
const MENTAL: &str = "mental_heat";
const RISE: &str = "rise_from_the_ashes";
const SEED: &str = "seed_of_fulmination";
const WINGS: &str = "wings_of_the_phoenix";
const BURNING: &str = "burning_wings";
/// Casts replayed, so Afterburn's roll and Searing Touch's chains show their odds.
pub(super) const CASTS: u64 = 8;
/// L0x96-L0x97, L0xd2-L0xd3: 4 px a frame growing to 14, both raised by Swift Flight.
const START: f32 = 4.;
const CAP: f32 = 14.;
/// skillsCharge L0xd7: these fly the charge through enemies; without them it stops on the dummy.
const THROUGH: [&str; 5] = [SEARING, MENTAL, COLLATERAL, SEED, WINGS];
/// L0xfa: the aura is made only with these, Wings of the Phoenix not among them.
const AURA_NODES: [&str; 4] = [COLLATERAL, SEARING, MENTAL, SEED];
/// L0xe5: Burning Wings sets playerEffect[22] = 180, a straight 180 px flight.
const BURNING_DISTANCE: f32 = 180.;
/// The unseen object: the flying player (drawn as the phoenix), its drops and the buffs.
const MOVER: &str = "Draw_Player_Buff_obj";
/// skillsCharge L0x2c9: maxScale = (1 + [71]) × 1.25, [71] = 1 + Burning Screech's area.
const NOVA_SCALE: f32 = 1.25;
/// L0x2b2: the n-th landing nova wakes 0.2 s × n later (alarm[0] = 1 + 12 × n).
const AFTERBURN_DELAY: u32 = 12;
/// L0x2b9: Scorching Fury's balls are made 12 − 32 × (novas − 1) px out.
const FURY_OUT: f32 = 12.;
const FURY_BACK: f32 = 32.;
const FULL_TURN: f32 = 360.;
/// Phoenix_Flight_obj Step_0 sits it at the player's (x, y + 6).
const AURA: &str = "Pyromancer_Phoenix_Flight_obj";
/// L0x231-L0x234: Wings of the Phoenix's balls 15° apart, centred straight behind, 8 px out.
const FAN_APART: f32 = 15.;
const BEHIND: f32 = 180.;
const FAN_OUT: f32 = 8.;
/// Fire_Ball_obj Create_0: 12 px a frame for 3 s, awake after alarm[2] = 1; Destroy_0's
/// explosion is (1 + explosionSize 1) × 0.8 big.
const BALL: &str = "Pyromancer_Fire_Ball_obj";
const BALL_SPEED: f32 = 12.;
const BALL_LIFE: u32 = 180;
const BALL_BLAST: f32 = 1.6;
/// L0xb2: Molten Feathers' 5 s buff, removed when the charge ends (L0x2e8); the 318 handler's
/// Infused Flames buff lasts 3 s.
const FEATHERS_FRAMES: u32 = 300;
const INFUSED_FRAMES: u32 = 180;
/// Seed_obj Create_0 and Destroy_0: 1.25 big, 0.75 s on the enemy, then a 2 big explosion.
const SEED_OBJ: &str = "Pyromancer_Phoenix_Flight_Seed_obj";
const SEED_SCALE: f32 = 1.25;
const SEED_LIFE: u32 = 45;
const SEED_BLAST: f32 = 2.;
/// Chain_Lightning_Parent_obj: moved onto the enemy hit, it hits it from frame 2 and, with no
/// other enemy to jump to, is gone at its first jump, 13 frames in.
const CHAIN: &str = "Pyromancer_Searing_Chains_obj";
const CHAIN_LIFE: u32 = 13;
/// skillsCharge L0x2cd-L0x2d6 and skillsLeap: from 64 px up, a 128 px rise under gravity
/// 0.35 + 0.425, from the frame after the charge.
const LEAP_FROM: f32 = 64.;
const LEAP_RISE: f32 = 128.;
const LEAP_GRAVITY: f32 = 0.775;
const LEAP_MAX: u32 = 600;
/// skillsLeap L0x1e9-L0x1f8: a nova each time a 0.25 s timer runs out, the first at once, growing
/// to 0.75 + its height / 100.
const LEAP_NOVA_EVERY: u32 = 16;
const LEAP_NOVA_SCALE: f32 = 0.75;
/// skillsLeap L0x230-L0x23f: the landing's Explosion_Item_obj with Viking_Odins_Fury_spr, 2 big,
/// fading 0.02 a frame.
const SMASH: &str = "Explosion_Item_obj:Viking_Odins_Fury_spr";
const SMASH_SCALE: f32 = 2.;
const SMASH_ACTIVE: u32 = 4;
const SMASH_FADE: u32 = 48;

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    flown(spawns, tree, afterburn(tree))
}

/// A cast whose Afterburn roll comes up `lucky`.
#[cfg(test)]
pub(super) fn apply_rolled(spawns: &[Spawn], tree: &Tree, lucky: bool) -> Vec<Spawn> {
    flown(spawns, tree, lucky && tree.learned(AFTERBURN))
}

fn flown(spawns: &[Spawn], tree: &Tree, afterburn: bool) -> Vec<Spawn> {
    let [base] = spawns else {
        return spawns.to_vec();
    };
    let flight = Flight {
        tree,
        base,
        afterburn,
    };
    let mover = flight.mover();
    let drops = flight.drops(&mover);
    let aura = flight.aura(&mover);
    std::iter::once(mover).chain(drops).chain(aura).collect()
}

/// The direction of `at` from where the player starts, the line a made object's turn adds to.
fn aim(at: [f32; 2]) -> f32 {
    point_direction([0., 0.], at)
}

/// L0xb5-L0xb6: the cast rolls Afterburn's second nova, irandom(99) < 8 × rank.
fn afterburn(tree: &Tree) -> bool {
    let mut dice = Dice::of_cast(tree.cast);
    tree.proc(AFTERBURN, "")
        .is_some_and(|(chance, _)| dice.roll(chance))
}

/// One cast: the base charge and what the nodes make of it.
struct Flight<'a> {
    tree: &'a Tree<'a>,
    base: &'a Spawn,
    afterburn: bool,
}

impl Flight<'_> {
    fn any(&self, nodes: &[&str]) -> bool {
        nodes.iter().any(|node| self.tree.learned(node))
    }

    /// The charge: its ramp, its steering toward the cursor unless Burning Wings gives it a
    /// distance, its stop, Molten Feathers and the landing.
    fn mover(&self) -> Spawn {
        let faster = 1. + self.tree.own(SWIFT, "movement_speed") / 100.;
        let (speed, accel) = charge::ramp(START * faster, CAP * faster);
        let dummy = skill_sim::skill(SKILL).map_or(0., |fx| fx.dummy);
        let distance = self.tree.learned(BURNING).then_some(BURNING_DISTANCE);
        let steered = distance.is_none().then(|| charge::steered(accel, dummy));
        let mut mover = Spawn {
            speed,
            accel: steered.or(Some(accel)),
            trail: None,
            ..self.base.clone()
        };
        let stops = Stops {
            contact: !self.any(&THROUGH),
            distance,
        };
        let life = charge::frames(&mover, dummy, stops);
        mover.life = Some(life);
        mover.buff = self.feathers(life);
        let end = charge::path(&mover)[life.saturating_sub(1) as usize].0;
        mover.on_end = self.landing(aim(end));
        mover
    }

    /// Molten Feathers: 15 % × rank less damage taken while the charge lasts, 5 s at most.
    fn feathers(&self, life: u32) -> Option<Buff> {
        self.tree.learned(FEATHERS).then(|| Buff {
            frames: life.min(FEATHERS_FRAMES),
            percent: self.tree.own(FEATHERS, "all_damage_taken_reduced_pct"),
        })
    }

    /// What skillsCharge leaves each 48 px of the path (L0x154, L0x226-L0x23b), made as a trail
    /// drops it: the phoenix turned with the flight, then Wings of the Phoenix's 3 × rank balls.
    fn drops(&self, mover: &Spawn) -> Vec<Spawn> {
        let Some(trail) = self.base.trail.as_deref() else {
            return Vec::new();
        };
        let path = charge::path(mover);
        let life = mover.life.unwrap_or(0);
        let dropped = trail.drops(mover, life).map(|(moves, _)| {
            let (at, heading) = path[moves as usize];
            Spawn {
                layout: vec![Placement {
                    frame: moves + 1,
                    offset: at,
                }],
                ..self.drop(&trail.spawn, heading - aim(at))
            }
        });
        dropped.collect()
    }

    /// One drop, `heading` past its aim: the phoenix, then the fan straight behind.
    fn drop(&self, phoenix: &Spawn, heading: f32) -> Spawn {
        let count = self.tree.own(WINGS, "projectile_count").round() as u32;
        let middle = (count as f32 - 1.) / 2.;
        let fan = (0..count).map(|i| self.ball(heading + BEHIND + FAN_APART * (i as f32 - middle)));
        let phoenix = Spawn {
            turn: Some(heading),
            ..phoenix.clone()
        };
        carrier(std::iter::once(phoenix).chain(fan).collect())
    }

    /// A fan's Fire_Ball_obj with the cast's hit, exploding on the enemy it touches.
    fn ball(&self, turn: f32) -> Spawn {
        let blast = Spawn {
            at_enemy: true,
            procs: self.carried(1.),
            ..pyro::blast(BALL_BLAST)
        };
        Spawn {
            object: BALL.into(),
            turn: Some(turn),
            offset: FAN_OUT,
            speed: BALL_SPEED,
            scale: Some([pyro::BALL_START, 1.]),
            grow: Some(pyro::ball_growth()),
            active_from: Some(1),
            life: Some(BALL_LIFE),
            ability: true,
            ends_on_contact: true,
            on_end: vec![blast],
            ..Spawn::default()
        }
    }

    /// The end block (skillsCharge L0x2a8-L0x2e4) where the player is, `aim` from its start: a
    /// nova, two with Afterburn, each with Scorching Fury's ring, then Rise from the Ashes' leap.
    fn landing(&self, aim: f32) -> Vec<Spawn> {
        let count = 1 + u32::from(self.afterburn);
        let novas = (0..count).map(|i| Spawn {
            frame: AFTERBURN_DELAY * i,
            ..self.nova()
        });
        let rings = (0..count).flat_map(|_| self.ring(count, aim));
        novas.chain(rings).chain(self.leap()).collect()
    }

    /// A landing nova: no size bonus reaches it, as it only inherits the controller's effects.
    fn nova(&self) -> Spawn {
        let screech = self.tree.own(SCREECH, "area_of_effect") / 100.;
        Spawn {
            procs: self.carried(1.),
            ..pyro::firenova(NOVA_SCALE * (2. + screech))
        }
    }

    /// What the controller's effects make on a carrier's hit, in push order: Infused Flames'
    /// buff (318, used once) and Burning Wings' patch at the enemy with the carrier's damage (324).
    fn carried(&self, share: f32) -> Vec<Proc> {
        let percent = self.tree.own(INFUSED, "fire_skill_damage");
        let buff = (percent > 0.).then(|| pyro::sure(buffing(INFUSED_FRAMES, percent)));
        let patch = self.tree.learned(BURNING).then(|| Proc {
            chance: 1.,
            spawn: Spawn {
                at_enemy: true,
                ..pyro::patch(self.tree.mods, share)
            },
            pull: None,
            once: false,
        });
        buff.into_iter().chain(patch).collect()
    }

    /// Scorching Fury: 3 × rank Split_obj balls 360 / n° apart from 0°, not from the heading,
    /// each 25 % × rank of the hit.
    fn ring(&self, novas: u32, aim: f32) -> Vec<Spawn> {
        let count = self.tree.own(FURY, "projectile_count").round() as u32;
        let share = self.tree.own(FURY, "of_total_damage") / 100.;
        let out = FURY_OUT - FURY_BACK * (novas - 1) as f32;
        let apart = FULL_TURN / count.max(1) as f32;
        (0..count)
            .map(|j| Spawn {
                turn: Some(apart * j as f32 - aim),
                offset: out,
                procs: self.carried(share),
                ..pyro::split(share)
            })
            .collect()
    }

    /// Rise from the Ashes: a nova each 16 frames of the leap, as many as its rank, drawn at the
    /// player's height, then the smash where it lands; neither inherits the controller.
    fn leap(&self) -> Vec<Spawn> {
        if !self.tree.learned(RISE) {
            return Vec::new();
        }
        let count = self.tree.own(RISE, "projectile_count").round() as u32;
        let landing = leap_landing();
        let novas = (0..count)
            .map(|k| k * LEAP_NOVA_EVERY)
            .filter(|&frame| frame < landing)
            .map(|frame| {
                let height = leap_height(frame);
                Spawn {
                    frame: 1 + frame,
                    fall: Some(Fall {
                        height,
                        ..Fall::default()
                    }),
                    ..pyro::firenova(LEAP_NOVA_SCALE + height / 100.)
                }
            });
        let smash = Spawn {
            frame: 1 + landing,
            ..smash()
        };
        novas.chain(std::iter::once(smash)).collect()
    }

    /// Pyromancer_Phoenix_Flight_obj (L0x104-L0x10e): round the flying player, 1 + Seed's 15 % ×
    /// rank big plus the build's size, dealing Collateral Burning's and Searing Touch's shares;
    /// its hit plants the seed (322), then rolls the chain (319).
    fn aura(&self, mover: &Spawn) -> Option<Spawn> {
        if !self.any(&AURA_NODES) {
            return None;
        }
        let tree = self.tree;
        let searing = tree
            .proc(SEARING, "of_total_damage")
            .map_or(0., |(_, part)| part);
        let share = (tree.own(COLLATERAL, "fire_skill_damage") + searing) / 100.;
        let size = 1. + tree.own(SEED, "area_of_effect") / 100. + tree.mods.size_bonus;
        let seed = tree.learned(SEED).then(|| pyro::sure(self.seed()));
        Some(Spawn {
            object: AURA.into(),
            layout: vec![Placement {
                frame: 0,
                offset: [0., charge::BELOW],
            }],
            speed: mover.speed,
            accel: mover.accel,
            scale: Some([size; 2]),
            life: mover.life,
            share: Some(share),
            procs: seed.into_iter().chain(self.chain(share)).collect(),
            ..Spawn::default()
        })
    }

    /// The seed on the enemy hit (the 322 handler), exploding 0.75 s later with floor(damage ×
    /// 15 % × rank); the explosion carries the aura's chain roll.
    fn seed(&self) -> Spawn {
        let share = self.tree.own(SEED, "area_of_effect") / 100.;
        let blast = Spawn {
            share: Some(share),
            procs: self.chain(share).into_iter().collect(),
            ..pyro::blast(SEED_BLAST)
        };
        Spawn {
            object: SEED_OBJ.into(),
            at_enemy: true,
            ability: true,
            scale: Some([SEED_SCALE; 2]),
            life: Some(SEED_LIFE),
            on_end: vec![blast],
            ..Spawn::default()
        }
    }

    /// Searing Touch's roll (the 319 handler): a chain on the enemy hit with the carrier's damage.
    fn chain(&self, share: f32) -> Option<Proc> {
        let (chance, _) = self.tree.proc(SEARING, "of_total_damage")?;
        let chain = Spawn {
            object: CHAIN.into(),
            at_enemy: true,
            life: Some(CHAIN_LIFE),
            share: Some(share),
            ..Spawn::default()
        };
        Some(Proc {
            chance,
            spawn: chain,
            pull: None,
            once: false,
        })
    }
}

/// The player's height `frame` frames into the leap (playerJumpGravity).
fn leap_height(frame: u32) -> f32 {
    let rise = (2. * LEAP_GRAVITY * LEAP_RISE).sqrt();
    let n = frame as f32;
    LEAP_FROM + n * rise - LEAP_GRAVITY * n * (n + 1.) / 2.
}

/// The frame of the leap the player lands on.
fn leap_landing() -> u32 {
    (1..LEAP_MAX)
        .find(|&frame| leap_height(frame) <= 0.)
        .unwrap_or(LEAP_MAX)
}

/// The landing's smash, the cast's hit fading away.
fn smash() -> Spawn {
    Spawn {
        object: SMASH.into(),
        scale: Some([SMASH_SCALE; 2]),
        active_from: Some(SMASH_ACTIVE),
        life: Some(0),
        fade: SMASH_FADE,
        ..Spawn::default()
    }
}

/// An unseen object ending at once, so what it makes keeps its own turn from where it is.
fn carrier(on_end: Vec<Spawn>) -> Spawn {
    Spawn {
        object: MOVER.into(),
        ability: true,
        scale: Some([0.; 2]),
        life: Some(0),
        on_end,
        ..Spawn::default()
    }
}

/// An unseen object giving the player `percent` for `frames` as it is made.
fn buffing(frames: u32, percent: f32) -> Spawn {
    Spawn {
        buff: Some(Buff { frames, percent }),
        ..carrier(Vec::new())
    }
}

#[cfg(test)]
#[path = "phoenix_flight_tests.rs"]
mod tests;
