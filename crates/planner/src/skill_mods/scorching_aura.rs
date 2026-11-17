//! Pyromancer's Scorching Aura (Sep 16 exe). While its buff lasts (LoadAura case 32), skillsAura
//! counts playerEffect[52] down and at zero runs LoadAuraScorching, which restarts it. A pulse
//! that finds an enemy in reach (L0xf) makes one Pyromancer_Scorching_Aura_obj at the player and
//! Searing Burst's bursts (L0x60-L0x81). The aura's hit runs ProjectileCollision02Pyromancer's
//! 341-344, none of which zeroes its effect: every object they make holds all its carrier's but
//! its maker's (InheritModifiers) and runs them on every hit. A preview use is one pulse, under
//! the player's shadow that draws the aura's glow and flame ring.
use super::{Mods, Tree, pyro};
use crate::skill_effects::{Buff, Dice, Orbit, Proc, Stacks};
use crate::skill_sim::{self, Placement, Spawn};

const SKILL: &str = "scorching_aura";
const LANDS: &str = "scorched_lands";
const FISSURE: &str = "flaming_fissure";
const RAPID: &str = "rapid_burning";
const FEET: &str = "burnt_feet";
const IGNITE: &str = "ignite_the_flames";
const BLAZE: &str = "ground_blaze";
const BURSTS: &str = "searing_burst";
const HARVESTER: &str = "scorching_harvester";
const CHAOS: &str = "release_the_chaos";
/// Pulses replayed: one ignition cycle against one dummy, a hundred stacks and the pulsing hit.
pub(super) const CASTS: u64 = 101;
const PERCENT: f32 = 100.;
/// L6, L0xc, L0x8d: the timer restarts at 60 × (1 - pulse speed) frames; skillsAura pulses on
/// the Step after it runs out.
const PULSE_FRAMES: f64 = 60.;
/// L8-L0xf: a pulse needs an enemy's box under 128 × (1 + area) px from the player's.
const REACH: f32 = 128.;
/// Mask_Circle_24x_spr's box about the dummy's origin: left, top, right, bottom.
const DUMMY_BOX: [f32; 4] = [-12., -12., 11., 11.];
/// L0x13, L0x51: the aura is 2 × (1 + area) big.
const AURA_SCALE: f32 = 2.;
/// L0x76-L0x77: each burst irandom(360)° and irandom(floor(128 × scale × 0.5)) px from the player.
const FULL_TURN: u32 = 360;
const BURST_SPREAD: f32 = 64.;
/// Searing_Burst_obj Create_0: 0.4 big; PC02 L0x147-L0x14c: the fissure's 0.75 one deals
/// damage × (1 + projEffect[341]), which L0x2f sets to true.
const BURST_SCALE: f32 = 0.4;
const FISSURE_SCALE: f32 = 0.75;
const FISSURE_DAMAGE: f32 = 2.;
/// Harvester_obj Create_0: deltaSpeed 4 for 10 s; Step_0 L0x12-L0x1d: an Explosion_Item_obj with
/// Pyromancer_Explosion_spr where its centre meets the enemy.
const SOUL: &str = "Pyromancer_Scorching_Harvester_obj";
const SOUL_SPEED: f32 = 4.;
const SOUL_LIFE: u32 = 600;
const SOUL_BLAST: &str = "Explosion_Item_obj:Pyromancer_Explosion_spr";
/// PC02 L0x1bf-L0x1c9: the hit finding 100 stacks pulses an Explosion_Item_obj 2.5 big dealing
/// damage × (1 + value × stacks × 0.01), the buff's value being LST(32, 7, 9) × 0.01.
const STACKS: u32 = 100;
const PULSE_SCALE: f32 = 2.5;
/// PC02 L0x1d6-L0x1dd: Release the Chaos' balls 360 / n° apart round the player, 16 px out,
/// growing 3 px and turning 10° a frame; Orbital_obj Create_0: 2.25 s, explosionSize 1.
const CHAOS_RADIUS: f32 = 16.;
const CHAOS_GROWTH: f32 = 3.;
const CHAOS_SPEED: f32 = 10.;
const CHAOS_LIFE: f32 = 135.;
const CHAOS_BLAST: f32 = 2.;

/// What draws a pulse's rolls: the tree, the pulse's number and the aura's scale.
type Roller = dyn Fn(&Tree, u64, f32) -> Rolls;

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    pulse(spawns, tree, &Rolls::of)
}

/// A pulse whose every pulse, this one and the earlier ones, rolls Flaming Fissure `fissure` and
/// Ground Blaze `blaze`, and no Searing Burst spots.
#[cfg(test)]
pub(super) fn apply_rolled(
    spawns: &[Spawn],
    tree: &Tree,
    fissure: bool,
    blaze: bool,
) -> Vec<Spawn> {
    let forced = move |tree: &Tree, _: u64, _: f32| Rolls {
        fissure: fissure && tree.learned(FISSURE),
        blaze: (tree.proc(BLAZE, "of_total_damage"))
            .filter(|_| blaze)
            .map(|(_, value)| value / PERCENT),
        spots: Vec::new(),
    };
    pulse(spawns, tree, &forced)
}

fn pulse(spawns: &[Spawn], tree: &Tree, rolled: &Roller) -> Vec<Spawn> {
    let [aura, ring] = spawns else {
        return spawns.to_vec();
    };
    let pulse = Pulse::new(tree, rolled);
    let ring = pulse.ring(ring);
    let dummy = skill_sim::skill(SKILL).map_or(0., |fx| fx.dummy);
    if !reaches(dummy, pulse.area) {
        return vec![ring];
    }
    [pulse.aura(aura), ring]
        .into_iter()
        .chain(pulse.bursts())
        .collect()
}

/// L0xf: distance_to_object(Enemy_Parent_obj) < 128 × (1 + area), for a dummy `dummy` px out.
fn reaches(dummy: f32, area: f32) -> bool {
    pyro::player_gap(DUMMY_BOX, [dummy, 0.]) < REACH * (1. + area)
}

/// Frames from one pulse to the next with Rapid Burning's pulse speed.
pub(super) fn pulse_frames(tree: &Tree) -> u32 {
    period(tree.own(RAPID, "aura_pulse_speed"))
}

/// Frames from one pulse to the next at `speed` % pulse speed: ceil(60 × (1 - speed)) + 1.
fn period(speed: f32) -> u32 {
    let frames = PULSE_FRAMES * (1. - f64::from(speed) / f64::from(PERCENT));
    (frames.ceil().max(0.) + 1.) as u32
}

/// What a pulse rolls, in the game's order: Flaming Fissure (L0x2e), Ground Blaze (L0x32), then
/// each burst's spot (L0x76-L0x77).
struct Rolls {
    fissure: bool,
    /// Ground Blaze's share of its carrier's damage, when the pulse rolled it.
    blaze: Option<f32>,
    spots: Vec<[f32; 2]>,
}

impl Rolls {
    /// L0x2e rolls irandom(99) < LST(32, 4, 9), the value the tooltip calls Flaming Fissure's
    /// damage; Ground Blaze rolls its chance, LST(32, 8, 10).
    fn of(tree: &Tree, cast: u64, scale: f32) -> Self {
        let mut dice = Dice::of_cast(cast);
        let fissure = tree
            .proc(FISSURE, "fire_skill_damage")
            .is_some_and(|(_, value)| dice.roll(value / PERCENT));
        let blaze = tree
            .proc(BLAZE, "of_total_damage")
            .filter(|(chance, _)| dice.roll(*chance))
            .map(|(_, value)| value / PERCENT);
        let count = tree.own(BURSTS, "projectile_count").round() as u32;
        let spread = (BURST_SPREAD * scale).floor() as u32;
        let spots = (0..count)
            .map(|_| {
                let heading = pyro::irandom(&mut dice, FULL_TURN);
                pyro::lengthdir(pyro::irandom(&mut dice, spread), heading)
            })
            .collect();
        Self {
            fissure,
            blaze,
            spots,
        }
    }
}

/// The cast's on-hit effects in push order (L0x53-L0x5e: 341, 342, 343, 344).
#[derive(Clone, Copy)]
enum Effect {
    Fissure,
    Ignite,
    Blaze,
    Harvest,
}

/// The effects a carrier holds, a bit each.
#[derive(Clone, Copy)]
struct Held(u8);

impl Held {
    /// What the aura holds: the pulse's rolls and the effects always pushed (L0x53-L0x5e).
    fn of(tree: &Tree, rolls: &Rolls) -> Self {
        let effects = [
            (Effect::Fissure, rolls.fissure),
            (Effect::Ignite, tree.learned(IGNITE)),
            (Effect::Blaze, rolls.blaze.is_some()),
            (Effect::Harvest, harvest(tree) > 0.),
        ];
        let held = effects.into_iter().filter(|(_, on)| *on);
        Self(held.fold(0, |bits, (effect, _)| bits | bit(effect)))
    }

    fn has(self, effect: Effect) -> bool {
        self.0 & bit(effect) != 0
    }

    /// What an object `effect` makes holds: InheritModifiers leaves out its maker's alone (PC02
    /// L0x154, L0x169, L0x1b7, L0x1cf, L0x1e4).
    fn without(self, effect: Effect) -> Self {
        Self(self.0 & !bit(effect))
    }
}

fn bit(effect: Effect) -> u8 {
    1 << effect as u8
}

/// Scorching Harvester's share of its carrier's damage (PC02 L0x163).
fn harvest(tree: &Tree) -> f32 {
    tree.own(HARVESTER, "fire_skill_damage") / PERCENT
}

/// A proc made on every hit of its carrier, as no handler of 341-344 zeroes its effect.
fn every(spawn: Spawn) -> Proc {
    Proc {
        chance: 1.,
        spawn,
        pull: None,
        once: false,
    }
}

/// What LoadAuraScorching reads for one pulse (L6-L0x51) and what it rolled.
struct Pulse<'a> {
    tree: &'a Tree<'a>,
    /// Scorched Lands' radius share.
    area: f32,
    scale: f32,
    period: u32,
    rolls: Rolls,
    /// Ignition stacks the earlier pulses left.
    from: u32,
}

impl<'a> Pulse<'a> {
    fn new(tree: &'a Tree<'a>, rolled: &Roller) -> Self {
        let area = tree.own(LANDS, "area_of_effect") / PERCENT;
        let scale = AURA_SCALE * (1. + area);
        Self {
            tree,
            area,
            scale,
            period: pulse_frames(tree),
            rolls: rolled(tree, tree.cast, scale),
            from: stacks_before(tree, scale, rolled),
        }
    }

    fn bonus(&self) -> f32 {
        self.tree.mods.size_bonus
    }

    /// The pulse's aura at the player; Step_0 keeps it on its host.
    fn aura(&self, base: &Spawn) -> Spawn {
        Spawn {
            scale: Some([self.scale; 2]),
            procs: self.carried(Held::of(self.tree, &self.rolls), 1.),
            ..base.clone()
        }
    }

    /// The player's shadow for the pulse's period, with Burnt Feet's buff: it draws the aura's
    /// glow and bends the ring round its reach (LoadAuraTimer), which its scale widens.
    fn ring(&self, base: &Spawn) -> Spawn {
        let buff = self.tree.learned(FEET).then(|| Buff {
            frames: self.period,
            percent: self.tree.own(FEET, "movement_speed"),
        });
        Spawn {
            scale: Some([1. + self.area; 2]),
            life: Some(self.period),
            buff,
            ..base.clone()
        }
    }

    /// Searing Burst's bursts (L0x74-L0x81), each with the aura's damage.
    fn bursts(&self) -> impl Iterator<Item = Spawn> + '_ {
        self.rolls.spots.iter().map(|&offset| Spawn {
            layout: vec![Placement { frame: 0, offset }],
            ..pyro::burst(BURST_SCALE + self.bonus(), 1.)
        })
    }

    /// The procs of a carrier holding `held` and dealing `share` of the hit, each made on its
    /// every hit and holding the rest.
    fn carried(&self, held: Held, share: f32) -> Vec<Proc> {
        let made = |effect| held.has(effect).then(|| held.without(effect));
        let fissure = made(Effect::Fissure).map(|rest| self.fissure(rest, share));
        let ignition = made(Effect::Ignite).map(|rest| self.ignition(rest, share));
        let blaze = (made(Effect::Blaze).zip(self.rolls.blaze))
            .map(|(rest, part)| self.blaze(rest, share * part));
        let soul = made(Effect::Harvest).map(|rest| self.soul(rest, share));
        let spawns = fissure.into_iter().chain(ignition.into_iter().flatten());
        spawns.chain(blaze).chain(soul).map(every).collect()
    }

    /// Flaming Fissure's burst under the enemy hit (PC02 L0x145-L0x154).
    fn fissure(&self, held: Held, share: f32) -> Spawn {
        let share = share * FISSURE_DAMAGE;
        Spawn {
            at_enemy: true,
            procs: self.carried(held, share),
            ..pyro::burst(FISSURE_SCALE + self.bonus(), share)
        }
    }

    /// Ignite the Flames (PC02 L0x1be-L0x1e8): every hit stacks the buff, or, on finding a hundred,
    /// pulses at the enemy and lets Release the Chaos' balls out.
    fn ignition(&self, held: Held, share: f32) -> Vec<Spawn> {
        let value = self.tree.own(IGNITE, "fire_skill_damage") / PERCENT;
        let boost = 1. + value * STACKS as f32 / PERCENT;
        let stacks = Some(Stacks {
            max: STACKS,
            from: self.from,
        });
        let pulse = Spawn {
            at_enemy: true,
            share: Some(share * boost),
            procs: self.carried(held, share * boost),
            stacks,
            ..pyro::blast(PULSE_SCALE)
        };
        let chaos = self.tree.own(CHAOS, "fire_skill_damage") / PERCENT;
        let balls = self.balls(held, share * (boost + chaos), stacks);
        std::iter::once(pulse).chain(balls).collect()
    }

    /// Release the Chaos' Fire_Ball_Orbital_obj balls spiralling out round the player; each ends on
    /// the first enemy it touches in an explosion 1 + explosionSize big, which Destroy_0 L0x18
    /// hands all the ball holds.
    fn balls(&self, held: Held, share: f32, stacks: Option<Stacks>) -> Vec<Spawn> {
        let count = self.tree.own(CHAOS, "projectile_count").round() as u32;
        if count == 0 {
            return Vec::new();
        }
        let life = CHAOS_LIFE * (1. + self.tree.mods.duration_pct / PERCENT);
        let procs = self.carried(held, share);
        let blast = Spawn {
            share: Some(share),
            procs: procs.clone(),
            ..pyro::blast(CHAOS_BLAST)
        };
        let ball = |i: u32| Spawn {
            object: pyro::ORBITAL.into(),
            orbit: Some(Orbit {
                radius: CHAOS_RADIUS,
                speed: CHAOS_SPEED,
                at: FULL_TURN as f32 / count as f32 * i as f32,
                spread: CHAOS_GROWTH,
                turns: true,
                face: pyro::ORBITAL_FACE,
                ..Orbit::CIRCLE
            }),
            scale: Some([pyro::BALL_START, 1.]),
            grow: Some(pyro::ball_growth()),
            life: Some(life.ceil() as u32),
            ends_on_contact: true,
            share: Some(share),
            procs: procs.clone(),
            on_end: vec![blast.clone()],
            stacks,
            ..Spawn::default()
        };
        (0..count).map(ball).collect()
    }

    /// Ground Blaze's Blazing_Trail_obj at the enemy (PC02 L0x1ad-L0x1b7), floor(damage × [343])
    /// a tick; Alarm_0 resets the 0.7 maxScale the handler sets to image_xscale 1.
    fn blaze(&self, held: Held, share: f32) -> Spawn {
        Spawn {
            at_enemy: true,
            procs: self.carried(held, share),
            ..pyro::patch(self.tree.mods, share)
        }
    }

    /// Scorching Harvester's soul (PC02 L0x15b-L0x169): from the player toward the enemy hit,
    /// homing until its centre meets it, where its explosion deals floor(damage × [344]) and gets
    /// all the soul holds (Harvester Step_0 L0x20).
    fn soul(&self, held: Held, share: f32) -> Spawn {
        let share = share * harvest(self.tree);
        let blast = Spawn {
            object: SOUL_BLAST.into(),
            share: Some(share),
            procs: self.carried(held, share),
            ..pyro::blast(1.)
        };
        Spawn {
            object: SOUL.into(),
            at_player: true,
            aim: true,
            speed: SOUL_SPEED,
            ability: true,
            life: Some(SOUL_LIFE),
            on_end: vec![blast],
            ..Spawn::default()
        }
    }
}

/// Ignition stacks the earlier pulses left, the loop starting full so that its first hit pulses:
/// every hit of an object holding 342 stacks once (PC02 L0x1e8).
fn stacks_before(tree: &Tree, scale: f32, rolled: &Roller) -> u32 {
    if !tree.learned(IGNITE) {
        return 0;
    }
    let ticks = patch_ticks(tree.mods);
    let hits: u32 = (0..tree.cast)
        .map(|n| 1 + stacking(Held::of(tree, &rolled(tree, n, scale)), ticks))
        .sum();
    (STACKS + hits) % (STACKS + 1)
}

/// The hits one hit of an object holding `held` sets off beyond itself, each holding 342 too: a
/// burst's and a soul's explosion's one, a patch's `ticks`.
fn stacking(held: Held, ticks: u32) -> u32 {
    let made = [
        (Effect::Fissure, 1),
        (Effect::Blaze, ticks),
        (Effect::Harvest, 1),
    ];
    let made = made.into_iter().filter(|(effect, _)| held.has(*effect));
    made.map(|(effect, hits)| hits * (1 + stacking(held.without(effect), ticks)))
        .sum()
}

/// Hits a Ground Blaze patch lands on the dummy it is lit on.
fn patch_ticks(mods: &Mods) -> u32 {
    let (Some(fx), Some(hitbox)) = (skill_sim::skill(SKILL), crate::hitbox::skill(SKILL)) else {
        return 0;
    };
    let patch = Spawn {
        layout: vec![Placement {
            frame: 0,
            offset: [fx.dummy, 0.],
        }],
        ..pyro::patch(mods, 1.)
    };
    skill_sim::replay_use(fx, hitbox, &[patch], 1, 0).hits.len() as u32
}

#[cfg(test)]
#[path = "scorching_aura_parity_tests.rs"]
mod parity;

#[cfg(test)]
#[path = "scorching_aura_tests.rs"]
mod tests;
