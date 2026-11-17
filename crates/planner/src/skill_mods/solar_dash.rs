//! Exo's Solar Dash: TalentsExo case 348 starts the charge with Charge_Controller_obj holding the
//! subskill values; skillsCharge moves the player through enemies, steering it toward the cursor,
//! drops a flame every 48 px and leaves the end-of-dash objects once the player cancels a loop
//! round the cursor; ProjectileCollision20Exo adds Photon Dash's flames. The preview's player
//! cannot move: an unseen mover dashes for it, drawn as the burning ball Draw_Player_Buff_obj
//! shows over the dashing player.
use super::Tree;
use super::charge::{self, BELOW, Stops};
use crate::skill_effects::{Buff, Orbit, Proc, Ride};
use crate::skill_homing::point_direction;
use crate::skill_sim::{Grow, Placement, Spawn};

const RESTORING: &str = "restoring_flames";
const PROTECTIVE: &str = "protective_cover";
const BURST: &str = "solar_burst";
const INTENSE: &str = "intense_flames";
const STAR: &str = "shooting_star";
const BURSTING: &str = "bursting_flames";
const CARNAGE: &str = "solar_carnage";
const GODDESS: &str = "the_sun_goddess";
const PHOTON: &str = "photon_dash";
/// Replayed uses, so Bursting Flames' roll on each flame shows its odds.
pub(super) const CASTS: u64 = 8;
/// TalentsExo L0x144, L0x17d, L0x181: 12 px a frame, halved by The Sun Goddess, tripled by Photon.
const DASH_SPEED: f32 = 12.;
const GODDESS_SPEED: f32 = 0.5;
const PHOTON_SPEED: f32 = 3.;
/// L0x194: the charge starts at 0.75 of its speed; skillsCharge stops the base dash only on walls.
const START: f32 = 0.75;
/// The mover runs `BELOW` px under the player: the centre of Mask_Player_spr and of the flames
/// (L0x1f0).
const MOVER: &str = "Draw_Player_Buff_obj";
/// L0x145, L0x186, L0x19e: flames tick every 0.5 s less Intense Flames' share, not under 0.1 s.
const FLAME_TICK: f32 = 30.;
const TICK_FLOOR: f32 = 6.;
/// L0x162, L0x168: Restoring Flames' and Protective Cover's buffs last 3 s.
const BUFF_FRAMES: u32 = 180;
/// End case L0x4cf-L0x4d1: Solar Burst's Explosion_Item_obj at 2.5, deadly from its 4-frame alarm.
const EXPLOSION: &str = "Explosion_Item_obj";
const EXPLOSION_SCALE: f32 = 2.5;
const EXPLOSION_ACTIVE: u32 = 4;
const EXPLOSION_LIFE: u32 = 24;
/// Exo_Solar_Dash_Fireball_obj (Create_0, Step_0): 8 px out, 8 px a frame, growing from 0.1 by
/// 0.05 a frame to 1, 1.5 s then shrinking away; the dash's own code never sets its scale.
const FIREBALL: &str = "Exo_Solar_Dash_Fireball_obj";
const BALL_OUT: f32 = 8.;
const BALL_SPEED: f32 = 8.;
const BALL_SCALE: f32 = 0.1;
const BALL_GROWTH: f32 = 0.05;
const BALL_LIFE: u32 = 90;
const BALL_FADE: u32 = 20;
/// skillsCharge L0x1ff-L0x216: two fireballs straight back at each flame, 16° wider each time.
const CARNAGE_SWEEP: f32 = 16.;
const BACK: f32 = 180.;
/// L0x1a8-L0x1ae, orb Step_0: 0.75 orbs circling (x, y + 6) at 120 px, the dash speed in ° a frame.
const ORB: &str = "Exo_Solar_Dash_Solar_Orb_obj";
const ORB_SCALE: f32 = 0.75;
const ORB_RADIUS: f32 = 120.;
/// ProjectileCollision20Exo L0x12a-L0x12c: Photon Dash's flame at the enemy hit is 0.6 big.
const PHOTON_SCALE: f32 = 0.6;

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    spawns.iter().flat_map(|dash| cast(dash, tree)).collect()
}

/// One cast: the charge with its end, its flames, Solar Carnage's fireballs and the orbs.
fn cast(base: &Spawn, tree: &Tree) -> Vec<Spawn> {
    let speed = dash_speed(tree);
    let dash = Dash { tree, base };
    let mover = dash.charge(speed);
    let path = charge::path(&mover);
    let drops = dash.drops(&mover);
    let flames = dash.flames(&path, &drops);
    let carnage = dash.carnage(&path, &drops);
    let orbs = dash.orbs(&mover, speed);
    let made = flames.into_iter().chain(carnage).chain(orbs);
    std::iter::once(mover).chain(made).collect()
}

/// L0x144-L0x185: 12 px a frame, transformed, then raised or cut by the speed nodes.
fn dash_speed(tree: &Tree) -> f32 {
    let mut speed = DASH_SPEED;
    if tree.learned(GODDESS) {
        speed *= GODDESS_SPEED;
    }
    if tree.learned(PHOTON) {
        speed *= PHOTON_SPEED;
    }
    speed * (1. + tree.stat("movement_speed") / 100.)
}

/// Where the dummy, and the cursor held on it, stand along the aim.
fn dummy() -> f32 {
    crate::skill_sim::skill("solar_dash").map_or(160., |fx| fx.dummy)
}

/// Frames the mover lives: one loop round the cursor, or until Photon Dash's next move would
/// land on the dummy (a move past it leaps over).
fn dash_frames(mover: &Spawn, photon: bool) -> u32 {
    let stops = Stops {
        contact: photon,
        distance: None,
    };
    charge::frames(mover, dummy(), stops)
}

/// The direction of `at` from where the player starts, the line a made object's turn adds to.
fn aim(at: [f32; 2]) -> f32 {
    point_direction([0., 0.], at)
}

/// The turn that puts what the dash leaves at the player, at `end`, `BELOW` px straight up from
/// the mover.
fn up(end: [f32; 2]) -> f32 {
    90. - aim([end[0], BELOW + end[1]])
}

/// What the dash keeps while it builds one use.
struct Dash<'a> {
    tree: &'a Tree<'a>,
    base: &'a Spawn,
}

impl Dash<'_> {
    /// The mover with the cast's speed, steered toward the cursor, its buff and its end, where the
    /// end case reads the heading it stops with.
    fn charge(&self, speed: f32) -> Spawn {
        let (speed, accel) = charge::ramp(START * speed, speed);
        let mut mover = Spawn {
            speed,
            accel: Some(charge::steered(accel, dummy())),
            buff: buff(self.tree),
            trail: None,
            ..self.base.clone()
        };
        let life = dash_frames(&mover, self.tree.learned(PHOTON));
        mover.life = Some(life);
        let (end, heading) = charge::path(&mover)[life.saturating_sub(1) as usize];
        let up = up(end);
        let aura = self.base.on_end.iter().map(|aura| Spawn {
            turn: Some(up),
            ..self.armed(aura, 1.)
        });
        let star = self.star(up, heading - aim(end)).into_iter();
        mover.on_end = aura.chain(self.burst(up)).chain(star).collect();
        mover
    }

    /// The moves after which skillsCharge drops a flame: once 48 px of its path past the last.
    fn drops(&self, mover: &Spawn) -> Vec<u32> {
        let life = mover.life.unwrap_or(0);
        let trail = self.base.trail.as_deref();
        trail.map_or(Vec::new(), |trail| {
            trail.drops(mover, life).map(|(moves, _)| moves).collect()
        })
    }

    /// The flames (L0x1f0-L0x1f8) at the player's centre, made as a trail drops them, each turned
    /// as the charge heads then (its direction, which Bursting Flames' ring starts from).
    fn flames(&self, path: &[([f32; 2], f32)], drops: &[u32]) -> Vec<Spawn> {
        let Some(trail) = self.base.trail.as_deref() else {
            return Vec::new();
        };
        let flame = self.flame(&trail.spawn);
        let placed = |&moves: &u32| {
            let ([x, y], heading) = path[moves as usize];
            let at = [x, BELOW + y];
            Spawn {
                layout: vec![Placement {
                    frame: moves + 1,
                    offset: at,
                }],
                on_end: self
                    .bursting(1., true, heading - aim(at))
                    .into_iter()
                    .collect(),
                ..flame.clone()
            }
        };
        drops.iter().map(placed).collect()
    }

    /// A flame: the cast's tick and Photon Dash's roll.
    fn flame(&self, flame: &Spawn) -> Spawn {
        let shorter = 1. - self.tree.own(INTENSE, "faster_cast_rate") / 100.;
        Spawn {
            tick: (FLAME_TICK * shorter).max(TICK_FLOOR),
            ..self.armed(flame, 1.)
        }
    }

    /// A damage object dealing `share` of the hit that carries the controller's values
    /// (InheritModifiers): with Photon Dash its hits leave flames.
    fn armed(&self, object: &Spawn, share: f32) -> Spawn {
        Spawn {
            share: Some(share),
            procs: self.photon(share).into_iter().collect(),
            ..object.clone()
        }
    }

    /// Photon Dash: a 0.6 flame at the enemy hit, floor(damage x (1 + the node's share)), ticking
    /// at the flame's own 0.5 s; it rolls the other entries, not its own.
    fn photon(&self, share: f32) -> Option<Proc> {
        if !self.tree.learned(PHOTON) {
            return None;
        }
        let flame = &self.base.trail.as_deref()?.spawn;
        let share = share * (1. + self.tree.own(PHOTON, "fire_skill_damage") / 100.);
        Some(Proc {
            chance: 1.,
            spawn: Spawn {
                at_enemy: true,
                scale: Some([PHOTON_SCALE; 2]),
                tick: FLAME_TICK,
                share: Some(share),
                on_end: self.bursting(share, false, 0.).into_iter().collect(),
                ..flame.clone()
            },
            pull: None,
            once: false,
        })
    }

    /// Bursting Flames (flame Destroy_0): rolled once a flame, its count of fireballs ringed out
    /// from the flame's direction, `from` past its aim, dealing the flame's damage.
    fn bursting(&self, share: f32, inherits: bool, from: f32) -> Option<Spawn> {
        let (chance, count) = self.tree.proc(BURSTING, "projectile_count")?;
        let ring = self.ring(count.round() as u32, share, inherits, from);
        Some(Spawn {
            chance: Some(chance),
            ..carrier(ring)
        })
    }

    /// Solar Burst: an explosion where the player stops, floor(damage x (1 + the node's share)).
    fn burst(&self, up: f32) -> Option<Spawn> {
        self.tree.learned(BURST).then(|| {
            let explosion = Spawn {
                object: EXPLOSION.into(),
                turn: Some(up),
                offset: BELOW,
                scale: Some([EXPLOSION_SCALE; 2]),
                active_from: Some(EXPLOSION_ACTIVE),
                life: Some(EXPLOSION_LIFE),
                ..Spawn::default()
            };
            let more = self.tree.own(BURST, "of_total_damage") / 100.;
            self.armed(&explosion, 1. + more)
        })
    }

    /// Shooting Star (end case L0x4de-L0x4ed): its count of fireballs ringed out from the heading
    /// the player stops with, `from` past its aim, each dealing the node's share of the damage.
    fn star(&self, up: f32, from: f32) -> Option<Spawn> {
        self.tree.learned(STAR).then(|| {
            let count = self.tree.own(STAR, "projectile_count").round() as u32;
            let share = self.tree.own(STAR, "of_total_damage") / 100.;
            Spawn {
                turn: Some(up),
                offset: BELOW,
                ..carrier(self.ring(count, share, true, from))
            }
        })
    }

    /// `count` fireballs 360/count° apart from `from` past the aim, made 8 px out.
    fn ring(&self, count: u32, share: f32, inherits: bool, from: f32) -> Vec<Spawn> {
        let count = count.max(1);
        let turn = |i: u32| from + 360. * i as f32 / count as f32;
        (0..count)
            .map(|i| self.fireball(turn(i), share, inherits))
            .collect()
    }

    fn fireball(&self, turn: f32, share: f32, inherits: bool) -> Spawn {
        let ball = Spawn {
            object: FIREBALL.into(),
            turn: Some(turn),
            offset: BALL_OUT,
            speed: BALL_SPEED,
            scale: Some([BALL_SCALE; 2]),
            grow: Some(Grow {
                step: BALL_GROWTH,
                max: Some(1.),
                ..Grow::default()
            }),
            life: Some(BALL_LIFE),
            fade: BALL_FADE,
            shrink: BALL_GROWTH,
            ..Spawn::default()
        };
        if inherits {
            self.armed(&ball, share)
        } else {
            Spawn {
                share: Some(share),
                ..ball
            }
        }
    }

    /// Solar Carnage: at each flame two fireballs from the player, straight back from its heading
    /// first, then 16° wider apart each time, dealing the damage.
    fn carnage(&self, path: &[([f32; 2], f32)], drops: &[u32]) -> Vec<Spawn> {
        if !self.tree.learned(CARNAGE) {
            return Vec::new();
        }
        let pair = |(k, &moves): (usize, &u32)| {
            let (at, heading) = path[moves as usize];
            let sweep = CARNAGE_SWEEP * k as f32;
            let from = heading - aim(at);
            let pair =
                [BACK + sweep, BACK - sweep].map(|turn| self.fireball(from + turn, 1., true));
            Spawn {
                layout: vec![Placement {
                    frame: moves,
                    offset: at,
                }],
                ..carrier(pair.into())
            }
        };
        drops.iter().enumerate().map(pair).collect()
    }

    /// The Sun Goddess: its count of orbs around the dashing player, dealing (1 + its share) of the
    /// damage, forgetting whom they hit each turn and gone with the dash.
    fn orbs(&self, mover: &Spawn, speed: f32) -> Option<Spawn> {
        let count = self.tree.own(GODDESS, "projectile_count").round() as u32;
        let more = self.tree.own(GODDESS, "fire_skill_damage") / 100.;
        self.tree.learned(GODDESS).then(|| Spawn {
            object: ORB.into(),
            scale: Some([ORB_SCALE; 2]),
            life: mover.life,
            share: Some(1. + more),
            orbit: Some(Orbit {
                radius: ORB_RADIUS,
                speed,
                count: count.max(1),
                below: BELOW,
                clears: true,
                ride: Some(Ride {
                    speed: mover.speed,
                    accel: mover.accel,
                }),
                ..Orbit::CIRCLE
            }),
            ..Spawn::default()
        })
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

/// Restoring Flames' life replenish, else Protective Cover's damage reduction, on the cast.
fn buff(tree: &Tree) -> Option<Buff> {
    let buffs = [
        (RESTORING, "life_replenish_pct"),
        (PROTECTIVE, "all_damage_taken_reduced_pct"),
    ];
    let (node, key) = buffs.into_iter().find(|(node, _)| tree.learned(node))?;
    Some(Buff {
        frames: BUFF_FRAMES,
        percent: tree.own(node, key),
    })
}

#[cfg(test)]
#[path = "solar_dash_tests.rs"]
mod tests;
