//! Pyromancer's Fire Nova: TalentsPyromancer case 35 rolls the cast, pulls for Vacuum Flames,
//! then makes the nova or Scorching Bolts' and Searing Tornado's balls instead, Mirror Flames'
//! Universal_Double_Cast_obj and Molten Trail's controller; Firenova Step_0 sends Traveling
//! Flames' novas on and ProjectileCollision02Pyromancer drops Armageddon Bolts' meteors (Sep 16
//! exe).
use super::{Tree, pyro};
use crate::skill_effects::{Breakout, Dice, Orbit, Proc, Pull};
use crate::skill_motion::{Accel, Trail};
use crate::skill_sim::{self, Grow, Spawn};

const SKILL: &str = "fire_nova";
const EXTENDED: &str = "extended_flames";
const MIRROR: &str = "mirror_flames";
const ARMAGEDDON: &str = "armageddon_bolts";
const MOLTEN: &str = "molten_trail";
const SCORCHING: &str = "scorching_bolts";
const TORNADO: &str = "searing_tornado";
const TRAVELING: &str = "traveling_flames";
const VACUUM: &str = "vacuum_flames";
/// Casts replayed, so the on-cast rolls show their odds.
pub(super) const CASTS: u64 = 8;
/// L0x4b9-L0x4bc, Firenova Step_0 L0x4 and L0x17-L0x2b: a traveling nova starts at 8 px a frame,
/// keeps 0.9 of it each frame and makes the next once travelingFlamesTimer (0.15 s) runs out.
const TRAVEL_SPEED: f32 = 8.;
const TRAVEL_RATE: f32 = -0.1;
const TRAVEL_MOVES: u32 = 8;
/// PC02 L0x170-L0x17c: 4 Armageddon_obj at the enemy from yy 320 + 80 × i; 350 is never zeroed.
const METEORS: u32 = 4;
const METEOR_HEIGHT: f32 = 320.;
const METEOR_STEP: f32 = 80.;
/// L0x4c1-L0x4d5, Split_obj Create_0: balls 360/n° apart at 7 × (1 + speed) px a frame, 3 s.
const SPLIT: &str = "Pyromancer_Fire_Ball_Split_obj";
const BOLT_SPEED: f32 = 7.;
const SPLIT_LIFE: f32 = 180.;
/// L0x4d8-L0x4de, Orbital_obj Create_0 and Step_0: balls circling the host from 48 px out, +2 px
/// and 12° a frame, facing 40° past their turn, for 2.25 s or until they touch an enemy.
const ORBITAL: &str = "Pyromancer_Fire_Ball_Orbital_obj";
const ORBIT_RADIUS: f32 = 48.;
const ORBIT_GROWTH: f32 = 2.;
const ORBIT_SPEED: f32 = 12.;
const ORBIT_FACE: f32 = 40.;
const ORBITAL_LIFE: f32 = 135.;
/// Split_obj and Orbital_obj Destroy_0: an explosion 1 + explosionSize (1) big.
const BALL_BLAST: f32 = 2.;
/// L0x496-L0x49f: TalentPullMonster for 1 s on enemies within floor(320 × scale × 0.5) of the
/// spot 48 px along the aim.
const VACUUM_OBJ: &str = "TalentPullMonster";
const VACUUM_OUT: f32 = 48.;
const VACUUM_RADIUS: f32 = 160.;
const PULL_FRAMES: u32 = 60;
/// L0x4f0-L0x4f7: Molten Trail's controller walks 4 px a frame.
const TRAIL_SPEED: f32 = 4.;

/// What a cast rolls before its objects exist (L0x471-L0x479), each roll its own draw; a copy
/// rolls Mirror Flames too, but L0x4e6 keeps it from casting again.
struct Rolls {
    mirror: bool,
    meteors: bool,
    trail: bool,
}

impl Rolls {
    fn of(tree: &Tree, roll: &mut impl FnMut(f32) -> bool) -> Self {
        let mut lucky = |node: &str| tree.proc(node, "").is_some_and(|(chance, _)| roll(chance));
        Self {
            mirror: lucky(MIRROR),
            meteors: lucky(ARMAGEDDON),
            trail: lucky(MOLTEN),
        }
    }
}

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    let mut dice = Dice::of_cast(tree.cast);
    cast(spawns, tree, &mut |chance| dice.roll(chance))
}

/// A cast whose every roll comes up `lucky`.
#[cfg(test)]
pub(super) fn apply_rolled(spawns: &[Spawn], tree: &Tree, lucky: bool) -> Vec<Spawn> {
    cast(spawns, tree, &mut |_| lucky)
}

fn cast(spawns: &[Spawn], tree: &Tree, roll: &mut impl FnMut(f32) -> bool) -> Vec<Spawn> {
    let [nova] = spawns else {
        return spawns.to_vec();
    };
    let cast = Cast::new(nova, tree);
    let first = Rolls::of(tree, roll);
    let copy = first.mirror.then(|| Rolls::of(tree, roll));
    let copied = copy.iter().flat_map(|rolls| cast.spawns(rolls));
    let delayed = copied.map(|spawn| Spawn {
        frame: spawn.frame + pyro::DOUBLE_CAST_DELAY,
        ..spawn
    });
    cast.spawns(&first).into_iter().chain(delayed).collect()
}

/// One cast: the base nova and its scale, 1 × (1 + Extended's and Traveling's area) (L0x464,
/// L0x48b, L0x494).
struct Cast<'a> {
    tree: &'a Tree<'a>,
    base: &'a Spawn,
    scale: f32,
}

impl<'a> Cast<'a> {
    fn new(base: &'a Spawn, tree: &'a Tree<'a>) -> Self {
        let area = tree.own(EXTENDED, "area_of_effect") + tree.own(TRAVELING, "area_of_effect");
        Self {
            tree,
            base,
            scale: 1. + area / 100.,
        }
    }

    /// The pull, then the nova or the balls (L0x4ab), then Molten Trail's controller.
    fn spawns(&self, rolls: &Rolls) -> Vec<Spawn> {
        let meteors = if rolls.meteors { meteors() } else { Vec::new() };
        let main = if self.tree.learned(SCORCHING) {
            self.bolts(&meteors)
        } else if self.tree.learned(TORNADO) {
            self.tornado(&meteors)
        } else {
            self.nova(&meteors)
        };
        let vacuum = self.tree.learned(VACUUM).then(|| self.vacuum()).flatten();
        let trail = rolls.trail.then(|| self.trail());
        vacuum.into_iter().chain([main]).chain(trail).collect()
    }

    /// L0x4ac-L0x4bc: the nova growing to the scale plus the build's size bonus (Alarm_11);
    /// Traveling Flames sends it on along the aim with as many novas to make as its rank.
    fn nova(&self, meteors: &[Proc]) -> Spawn {
        let grow = Grow {
            max: Some(self.scale + self.tree.mods.size_bonus),
            ..self.base.grow.unwrap_or_default()
        };
        let nova = Spawn {
            grow: Some(grow),
            procs: meteors.to_vec(),
            ..self.base.clone()
        };
        let count = self.tree.own(TRAVELING, "projectile_count").round() as u32;
        if count == 0 {
            nova
        } else {
            traveling(nova, count)
        }
    }

    /// L0x4c1-L0x4d5: 5 × rank Split_obj balls at 360/n × i° (no aim term; the sim aims at 0°),
    /// flying straight from their first move, piercing, then exploding where they end.
    fn bolts(&self, meteors: &[Proc]) -> Spawn {
        let faster = 1. + self.tree.own(SCORCHING, "projectile_speed_pct") / 100.;
        let orbit = Orbit {
            count: self.tree.own(SCORCHING, "projectile_count").round() as u32,
            from: 1,
            turns: true,
            breakout: Some(Breakout {
                at: 1,
                speed: BOLT_SPEED * faster,
            }),
            ..Orbit::CIRCLE
        };
        self.ball(SPLIT, orbit, SPLIT_LIFE, meteors)
    }

    /// L0x4d8-L0x4de: without Scorching Bolts, 2 × rank Orbital_obj balls circling the player;
    /// Split_obj reads none of the orbit fields, so with it the balls fly straight.
    fn tornado(&self, meteors: &[Proc]) -> Spawn {
        let orbit = Orbit {
            radius: ORBIT_RADIUS - ORBIT_GROWTH,
            speed: ORBIT_SPEED,
            count: self.tree.own(TORNADO, "projectile_count").round() as u32,
            from: 1,
            turns: true,
            spread: ORBIT_GROWTH,
            face: ORBIT_FACE,
            ..Orbit::CIRCLE
        };
        Spawn {
            ends_on_contact: true,
            ..self.ball(ORBITAL, orbit, ORBITAL_LIFE, meteors)
        }
    }

    /// A ball `object` as big as the scale plus the size bonus, lasting `life` stretched by the
    /// build (Alarm_11); its explosion inherits the meteors.
    fn ball(&self, object: &str, orbit: Orbit, life: f32, meteors: &[Proc]) -> Spawn {
        let blast = Spawn {
            procs: meteors.to_vec(),
            ..pyro::blast(BALL_BLAST)
        };
        let life = life * (1. + self.tree.mods.duration_pct / 100.);
        Spawn {
            object: object.into(),
            orbit: Some(orbit),
            scale: Some([self.scale + self.tree.mods.size_bonus; 2]),
            life: Some(life.round() as u32),
            procs: meteors.to_vec(),
            on_end: vec![blast],
            ..Spawn::default()
        }
    }

    /// An unseen object on the spot pulling the dummy toward it as the cast goes off, if the
    /// cast's collision_circle_list finds the dummy there (the aim runs along +x).
    fn vacuum(&self) -> Option<Spawn> {
        let spot = [VACUUM_OUT, 0.];
        let dummy = skill_sim::skill(SKILL).map_or(0., |fx| fx.dummy);
        let reach = (VACUUM_RADIUS * self.scale).floor();
        let found = (dummy - spot[0]).hypot(spot[1]) <= reach;
        found.then(|| Spawn {
            object: VACUUM_OBJ.into(),
            offset: VACUUM_OUT,
            ability: true,
            life: Some(0),
            pull: Some(Pull {
                frames: PULL_FRAMES,
                beyond: 0.,
                at: Some(spot),
            }),
            ..Spawn::default()
        })
    }

    /// Molten Trail's unseen controller walking out from the player along the aim.
    fn trail(&self) -> Spawn {
        pyro::controller(self.tree.mods, TRAIL_SPEED, 1.)
    }
}

/// A traveling nova with `left` novas still to make, each where it is after 8 moves; the copy
/// keeps the damage, the scale and the projEffects (Step_0 L0x1d-L0x2b).
fn traveling(nova: Spawn, left: u32) -> Spawn {
    let next = left.checked_sub(1).map(|rest| {
        Box::new(Trail {
            every: u32::MAX,
            first: TRAVEL_MOVES,
            gap: 0.,
            spawn: traveling(nova.clone(), rest),
            next: None,
        })
    });
    Spawn {
        speed: 0.,
        accel: Some(Accel {
            from: TRAVEL_SPEED,
            step: 0.,
            rate: TRAVEL_RATE,
            steer: None,
        }),
        trail: next,
        ..nova
    }
}

/// Armageddon Bolts' meteors, made on each enemy a carrier hits.
fn meteors() -> Vec<Proc> {
    let meteor = |i: u32| Proc {
        chance: 1.,
        spawn: pyro::meteor(METEOR_HEIGHT + METEOR_STEP * i as f32, pyro::blast(1.)),
        pull: None,
        once: false,
    };
    (0..METEORS).map(meteor).collect()
}

#[cfg(test)]
#[path = "fire_nova_tests.rs"]
mod tests;
