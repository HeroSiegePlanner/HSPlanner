//! Pyromancer's Volcano: TalentsPyromancer case 36 sets the volcano up from the subtree, its Step_0
//! throws Pyromancer_Volcano_Fragment_obj, cracks the earth and sends lava trails, Alarm_0 gives
//! Yellowstone its look and Alarm_1 chains Trail of Volcanoes' next volcano; each fragment's Step_0
//! bounces it and its Destroy_0 explodes it and raises Yellowstone's small volcano (Sep 16 exe).
use super::{Tree, pyro};
use crate::skill_effects::Dice;
use crate::skill_motion::Trail;
use crate::skill_sim::{self, Fall, Grow, Placement, Spawn};
use std::collections::BTreeMap;

const SKILL: &str = "volcano";
const ERUPTION: &str = "eruption";
const EXTENDED: &str = "extended_lava";
const DANGER: &str = "danger_zone";
const SEARING: &str = "searing_mountain";
const COLOSSAL: &str = "colossal_fragment";
const CRACKLING: &str = "crackling_earth";
const CRUSHING: &str = "crushing_fragment";
const TOO_HOT: &str = "too_hot_to_touch";
const YELLOWSTONE: &str = "yellowstone";
const LAVA: &str = "lava_trail";
const I_AM: &str = "i_am_the_volcano_now";
const CHAIN: &str = "trail_of_volcanoes";
/// Nodes acting through the fragments, which the base use leaves out: they miss a dummy under it.
const THROWERS: [&str; 6] = [ERUPTION, DANGER, COLOSSAL, CRUSHING, YELLOWSTONE, I_AM];
/// Casts replayed, so the cast's rolls show their odds.
pub(super) const CASTS: u64 = 8;
/// GameMaker's epsilon when it compares reals.
const EPSILON: f32 = 1e-5;
/// A destroyTimer that runs out starts the fade 2 frames later (Alarm_10).
const FADE_DELAY: u32 = 2;
const FULL_TURN: u32 = 360;
/// Create_0: 0.1 big; m_fadeIn adds 0.05 a frame to maxScale, m_fadeDestroy takes 0.05 × maxScale.
const START: f32 = 0.1;
const GROWTH: f32 = 0.05;
const SHRINK: f32 = 0.05;
/// Create_0 and m_fireProjectile L0x97-L0x98: shootCooldown is shootFrequency (0.25 s), and after
/// a Yellowstone shot max(1.5 s × (1 − yellowStone), 0.25 s); Step_0 shoots once it is not over 0.
const SHOT_EVERY: f32 = 15.;
const YELLOWSTONE_EVERY: f32 = 90.;
/// Case 36 L0x532-L0x533 and Alarm_0: Yellowstone adds 1.25 to maxScale, and once yellowStone
/// (0.25 a rank) reads as true, over 0.5, a quarter of the growth past 1 stays.
const YELLOWSTONE_SIZE: f32 = 1.25;
const YELLOWSTONE_KEEP: f32 = 0.25;
const TRUE_OVER: f32 = 0.5;
const YELLOWSTONE_LOOK: &str = "Pyromancer_Volcano_obj:Pyromancer_Volcano_Yellowstone_spr";
/// Create_0: cracklingEarthTimer and lavaTrailTimer start at 0.25 s; m_cracklingEarth then waits
/// 0.75 s × (1 − its rate) (case 36 L0x51f-L0x521), m_lavaTrail 1 s.
const FIRST_EVENT: u32 = 15;
const CRACK_EVERY: f32 = 45.;
const LAVA_EVERY: u32 = 60;
/// m_cracklingEarth: a burst irandom(128) + 80 px out, 0.4 big.
const BURST_NEAR: f32 = 80.;
const BURST_SPREAD: u32 = 128;
const BURST_SCALE: f32 = 0.4;
/// m_lavaTrail L0xab: its controllers walk 5 px a frame.
const LAVA_SPEED: f32 = 5.;
/// Alarm_1 (alarm[1] 0.5 s): the next volcano 96 px out at direction − 40 + irandom(80).
const CHAIN_DELAY: u32 = 30;
const CHAIN_STEP: f32 = 96.;
const CHAIN_TURN: f32 = 40.;
const CHAIN_SPREAD: u32 = 80;
/// m_iAmVolcano: x = host.x, y = host.y + 6 every Step.
const HOST_BELOW: f32 = 6.;
/// Fragment Create_0 and Alarm_0: 2 px × (1 + Danger Zone) a frame from yy 32 at zSpeed 4; Step_0
/// grows it 0.1 × maxScale a frame to 0.5, +1 for Yellowstone, +0.75 colossal (L0x79, L0x7f).
const FRAGMENT: &str = "Pyromancer_Volcano_Fragment_obj";
const FRAGMENT_SPEED: f32 = 2.;
const FRAGMENT_FALL: Fall = Fall {
    height: 32.,
    height_step: 0.,
    z_speed: 4.,
    gravity: 0.15,
};
const FRAGMENT_START: f32 = 0.1;
const FRAGMENT_GROWTH: f32 = 0.1;
const FRAGMENT_SIZE: f32 = 0.5;
const YELLOWSTONE_FRAGMENT: f32 = 1.;
const COLOSSAL_FRAGMENT: f32 = 0.75;
/// m_fireProjectile L0x57-L0x5e: the spot 128 px out and the reach of the enemy nearest it.
const AIM_REACH: f32 = 128.;
/// Fragment Step_0 L0x13-L0x2b: 3 bounces at 0.75 of its speed from yy 8, zSpeed 2 + bounces left,
/// each leaving Viking_Odins_Fury_spr 0.5 big with half its damage, fading 0.02 a frame.
const BOUNCES: u32 = 3;
const BOUNCE_SLOWDOWN: f32 = 0.75;
const BOUNCE_HEIGHT: f32 = 8.;
const BOUNCE_LIFT: f32 = 2.;
const BOUNCE_BLAST: &str = "Explosion_Item_obj:Viking_Odins_Fury_spr";
const BOUNCE_SCALE: f32 = 0.5;
const BOUNCE_SHARE: f32 = 0.5;
const BOUNCE_FADE: u32 = 48;
/// Fragment Destroy_0: an explosion dealing 1.5 × the fragment's damage.
const BLAST_SHARE: f32 = 1.5;
/// The volcano's Step making its objects of one moment: unseen, it ends at once.
const STEP: &str = "Pyromancer_Volcano_obj_Step_0";
/// A trail that drops once.
const ONCE: u32 = u32::MAX;

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    let [volcano] = spawns else {
        return spawns.to_vec();
    };
    let cast = Cast::new(volcano, tree);
    let mut dice = Dice::of_cast(tree.cast);
    cast.volcanoes(&mut dice)
}

/// One volcano: where it stands, how big it grows, its ticks' share and what its Step makes.
#[derive(Clone, Copy)]
struct Mount {
    at: [f32; 2],
    max: f32,
    share: f32,
    /// Fragments a shot and the cooldown after each shot.
    fragments: u32,
    cooldown: f32,
    /// Whether its fragments' explosions deal damage: Alarm_1 copies no fragmentDamage.
    harms: bool,
    /// Yellowstone's: its look, and fragments that raise volcanoes.
    yellowstone: bool,
    cracks: bool,
    trails: u32,
}

/// A fragment's way: its direction, speed and size.
#[derive(Clone, Copy)]
struct Flight {
    heading: f32,
    speed: f32,
    size: f32,
}

/// One cast: what case 36 reads from the subtree for the volcano and all it makes.
struct Cast<'a> {
    tree: &'a Tree<'a>,
    base: &'a Spawn,
    /// destroyTimer stretched by Extended Lava (L0x54f) and the build's duration (Alarm_11).
    life: u32,
    /// Too Hot to Touch's multiplier on damage, which fragmentDamage leaves out (L0x55a-L0x55b).
    share: f32,
    throws: bool,
    fragments: u32,
    speed: f32,
    colossal: f32,
    crushing: f32,
    yellowstone: f32,
    crack: Option<u32>,
    trails: u32,
    chain: u32,
    on_player: bool,
    dummy: [f32; 2],
}

impl<'a> Cast<'a> {
    fn new(base: &'a Spawn, tree: &'a Tree<'a>) -> Self {
        let part = |node: &str, key: &str| tree.own(node, key) / 100.;
        let chance = |node: &str| tree.proc(node, "").map_or(0., |(chance, _)| chance);
        let stretch =
            (1. + part(EXTENDED, "skill_duration")) * (1. + tree.mods.duration_pct / 100.);
        let life = base.life.unwrap_or(0) as f32 * stretch;
        let crack = tree.learned(CRACKLING).then(|| {
            let every = CRACK_EVERY * (1. - part(CRACKLING, "faster_cast_rate"));
            (every - EPSILON).ceil().max(1.) as u32
        });
        Self {
            tree,
            base,
            life: (life - EPSILON).ceil() as u32,
            share: 1. + part(TOO_HOT, "fire_skill_damage"),
            throws: THROWERS.iter().any(|node| tree.learned(node)),
            fragments: 1 + tree.own(ERUPTION, "projectile_count").round() as u32,
            speed: FRAGMENT_SPEED * (1. + part(DANGER, "area_of_effect")),
            colossal: chance(COLOSSAL),
            crushing: chance(CRUSHING),
            yellowstone: part(YELLOWSTONE, "faster_cast_rate"),
            crack,
            trails: tree.own(LAVA, "trail_count").round() as u32,
            chain: tree.own(CHAIN, "projectile_count").round() as u32,
            on_player: tree.learned(I_AM),
            dummy: [skill_sim::skill(SKILL).map_or(0., |fx| fx.dummy), 0.],
        }
    }

    fn bonus(&self) -> f32 {
        self.tree.mods.size_bonus
    }

    /// The cast's volcano and Trail of Volcanoes' chain.
    fn volcanoes(&self, dice: &mut Dice) -> Vec<Spawn> {
        let main = self.main();
        let first = self.volcano(&main, dice);
        let mut placed = vec![self.placed(first, main.at, 0)];
        for (link, at) in (1..).zip(self.links(main.at, dice)) {
            let volcano = self.volcano(&self.chained(at), dice);
            placed.push(self.placed(volcano, at, CHAIN_DELAY * link));
        }
        placed
    }

    /// Alarm_1's chain from `from`: each volcano 96 px on at the last direction − 40 + irandom(80),
    /// starting from the aim.
    fn links(&self, from: [f32; 2], dice: &mut Dice) -> Vec<[f32; 2]> {
        let mut heading = pyro::toward([0., 0.], pyro::cursor(SKILL));
        let mut at = from;
        (0..self.chain)
            .map(|_| {
                heading += pyro::irandom(dice, CHAIN_SPREAD) - CHAIN_TURN;
                let [dx, dy] = pyro::lengthdir(CHAIN_STEP, heading);
                at = [at[0] + dx, at[1] + dy];
                at
            })
            .collect()
    }

    /// Case 36's volcano: Searing Mountain's and Yellowstone's maxScale (L0x51a, L0x533) as
    /// Alarm_0 leaves it, then the build's size bonus (Alarm_11).
    fn main(&self) -> Mount {
        let yellowstone = self.yellowstone > 0.;
        let mut max = 1. + self.tree.own(SEARING, "projectile_size") / 100.;
        if yellowstone {
            max += YELLOWSTONE_SIZE;
        }
        if self.yellowstone > TRUE_OVER {
            max = 1. + (max - 1.) * YELLOWSTONE_KEEP;
        }
        let cooldown = if yellowstone {
            (YELLOWSTONE_EVERY * (1. - self.yellowstone)).max(SHOT_EVERY)
        } else {
            SHOT_EVERY
        };
        Mount {
            at: if self.on_player {
                [0., HOST_BELOW]
            } else {
                pyro::cursor(SKILL)
            },
            max: max + self.bonus(),
            share: self.share,
            fragments: self.fragments,
            cooldown,
            harms: true,
            yellowstone,
            cracks: self.crack.is_some(),
            trails: self.trails,
        }
    }

    /// Alarm_1's volcano: damage, fragments, cracks and trails copied, but no fragmentDamage.
    fn chained(&self, at: [f32; 2]) -> Mount {
        Mount {
            share: self.share,
            fragments: self.fragments,
            harms: false,
            cracks: self.crack.is_some(),
            trails: self.trails,
            ..self.raised(at)
        }
    }

    /// Destroy_0's volcano: the fragment's damage, Create_0's maxScale, cooldown and fragment.
    fn raised(&self, at: [f32; 2]) -> Mount {
        Mount {
            at,
            max: 1. + self.bonus(),
            share: 1.,
            fragments: 1,
            cooldown: SHOT_EVERY,
            harms: true,
            yellowstone: false,
            cracks: false,
            trails: 0,
        }
    }

    /// The base volcano with a mount's look, growth, share and what its Step makes.
    fn volcano(&self, mount: &Mount, dice: &mut Dice) -> Spawn {
        Spawn {
            trail: self.steps(mount, dice),
            ..self.body(mount)
        }
    }

    /// The volcano itself: the mount's look, growth and share.
    fn body(&self, mount: &Mount) -> Spawn {
        let object = if mount.yellowstone {
            YELLOWSTONE_LOOK.into()
        } else {
            self.base.object.clone()
        };
        Spawn {
            object,
            life: Some(self.life),
            grow: self.base.grow.map(|grow| Grow {
                max: Some(mount.max),
                ..grow
            }),
            shrink: SHRINK * mount.max,
            share: (mount.share > 1.).then_some(mount.share),
            ..self.base.clone()
        }
    }

    /// A volcano the cast places `frame` frames in: the one on the cursor stays as the base has it.
    fn placed(&self, volcano: Spawn, at: [f32; 2], frame: u32) -> Spawn {
        if frame == 0 && !self.on_player {
            return volcano;
        }
        let origin = if self.on_player {
            [0., 0.]
        } else {
            pyro::cursor(SKILL)
        };
        Spawn {
            target: !self.on_player,
            layout: vec![Placement {
                frame,
                offset: [at[0] - origin[0], at[1] - origin[1]],
            }],
            ..volcano
        }
    }

    /// What its Step makes, one unseen carrier per age, chained in age order.
    fn steps(&self, mount: &Mount, dice: &mut Dice) -> Option<Box<Trail>> {
        let mut made: BTreeMap<u32, Vec<Spawn>> = BTreeMap::new();
        if self.throws {
            for age in self.shots(mount) {
                let shot = self.shot(mount, dice);
                made.entry(age).or_default().extend(shot);
            }
        }
        if let Some(every) = self.crack.filter(|_| mount.cracks) {
            for age in self.every(every) {
                let heading = pyro::irandom(dice, FULL_TURN);
                let burst = self.burst(mount, heading, pyro::irandom(dice, BURST_SPREAD));
                made.entry(age).or_default().push(burst);
            }
        }
        if mount.trails > 0 {
            for age in self.every(LAVA_EVERY) {
                let trails: Vec<Spawn> = (0..mount.trails)
                    .map(|_| self.controller(mount, pyro::irandom(dice, FULL_TURN)))
                    .collect();
                made.entry(age).or_default().extend(trails);
            }
        }
        made.into_iter().rev().fold(None, |next, (age, items)| {
            Some(Box::new(Trail {
                every: ONCE,
                first: age - 1,
                gap: 0.,
                spawn: carrier(items),
                next,
            }))
        })
    }

    /// Ages of a timer started at 0.25 s and reset to `period` while the volcano stands.
    fn every(&self, period: u32) -> Vec<u32> {
        let end = self.life + FADE_DELAY + self.base.fade;
        (0..)
            .map(|k| FIRST_EVENT + k * period)
            .take_while(|&age| age < end)
            .collect()
    }

    /// Step_0's shots once grown: after Create_0's cooldown, then each shot's, until the fade.
    fn shots(&self, mount: &Mount) -> Vec<u32> {
        let grown = ((mount.max - START) / GROWTH - EPSILON).ceil().max(0.) as u32;
        let first = grown + countdown(SHOT_EVERY);
        let period = countdown(mount.cooldown);
        let fade = self.life + FADE_DELAY;
        (0..)
            .map(|k| first + k * period)
            .take_while(|&age| age < fade)
            .collect()
    }

    /// m_fireProjectile: the shot's crushing roll (L0x4f), then for each fragment its colossal
    /// roll (L0x54) and its direction.
    fn shot(&self, mount: &Mount, dice: &mut Dice) -> Vec<Spawn> {
        let bounces = if dice.roll(self.crushing) { BOUNCES } else { 0 };
        (0..mount.fragments)
            .map(|_| {
                let colossal = dice.roll(self.colossal);
                let heading = self.aimed(mount.at, pyro::irandom(dice, FULL_TURN));
                self.fragment(mount, heading, colossal, bounces, dice)
            })
            .collect()
    }

    /// L0x57-L0x5e: the drawn heading, or at the enemy nearest the spot 128 px out that way if it
    /// stands within 128 px of it.
    fn aimed(&self, at: [f32; 2], heading: f32) -> f32 {
        let [dx, dy] = pyro::lengthdir(AIM_REACH, heading);
        let spot = [at[0] + dx, at[1] + dy];
        let near = (self.dummy[0] - spot[0]).hypot(self.dummy[1] - spot[1]) < AIM_REACH;
        if near {
            pyro::toward(at, self.dummy)
        } else {
            heading
        }
    }

    fn fragment(
        &self,
        mount: &Mount,
        heading: f32,
        colossal: bool,
        bounces: u32,
        dice: &mut Dice,
    ) -> Spawn {
        let more = |on: bool, size: f32| if on { size } else { 0. };
        let size = FRAGMENT_SIZE
            + more(mount.yellowstone, YELLOWSTONE_FRAGMENT)
            + more(colossal, COLOSSAL_FRAGMENT);
        let flight = Flight {
            heading,
            speed: self.speed,
            size,
        };
        Spawn {
            scale: Some([FRAGMENT_START; 2]),
            grow: Some(Grow {
                step: FRAGMENT_GROWTH * size,
                max: Some(size),
                ..Grow::default()
            }),
            ..self.flight(mount, mount.at, flight, FRAGMENT_FALL, bounces, dice)
        }
    }

    /// A fragment flying from `from` until it lands; it never hits (Player_Ability_Parent_obj).
    fn flight(
        &self,
        mount: &Mount,
        from: [f32; 2],
        flight: Flight,
        fall: Fall,
        bounces: u32,
        dice: &mut Dice,
    ) -> Spawn {
        let gone = flight.speed * pyro::landing(fall) as f32;
        let [dx, dy] = pyro::lengthdir(gone, flight.heading);
        let landed = [from[0] + dx, from[1] + dy];
        Spawn {
            object: FRAGMENT.into(),
            ability: true,
            turn: Some(flight.heading - pyro::toward([0., 0.], from)),
            speed: flight.speed,
            fall: Some(fall),
            scale: Some([flight.size; 2]),
            on_end: self.landed(mount, landed, flight, bounces, dice),
            ..Spawn::default()
        }
    }

    /// Step_0 L0x11-L0x2d where it lands: a bounce while it has bounces left, else Destroy_0.
    fn landed(
        &self,
        mount: &Mount,
        at: [f32; 2],
        flight: Flight,
        bounces: u32,
        dice: &mut Dice,
    ) -> Vec<Spawn> {
        if bounces == 0 {
            return self.destroyed(mount, at, dice);
        }
        let hop = Flight {
            speed: flight.speed * BOUNCE_SLOWDOWN,
            ..flight
        };
        let fall = Fall {
            height: BOUNCE_HEIGHT,
            z_speed: BOUNCE_LIFT + bounces as f32,
            ..FRAGMENT_FALL
        };
        let blast = Spawn {
            object: BOUNCE_BLAST.into(),
            life: Some(0),
            fade: BOUNCE_FADE,
            ..self.blast(mount, BOUNCE_SHARE, BOUNCE_SCALE)
        };
        let hopped = self.flight(mount, at, hop, fall, bounces - 1, dice);
        vec![blast, hopped]
    }

    /// Destroy_0: its explosion, then Yellowstone's volcano where it landed (L0x1a-L0x2c).
    fn destroyed(&self, mount: &Mount, at: [f32; 2], dice: &mut Dice) -> Vec<Spawn> {
        let blast = self.blast(mount, BLAST_SHARE, 1.);
        let raised = mount
            .yellowstone
            .then(|| self.volcano(&self.raised(at), dice));
        std::iter::once(blast).chain(raised).collect()
    }

    /// An explosion `scale` big with `share` of the fragment's damage, none without fragmentDamage.
    fn blast(&self, mount: &Mount, share: f32, scale: f32) -> Spawn {
        let blast = pyro::blast(scale);
        if mount.harms {
            Spawn {
                share: Some(share),
                ..blast
            }
        } else {
            Spawn {
                ability: true,
                ..blast
            }
        }
    }

    /// m_cracklingEarth L0xbf-L0xc5: a burst at its drawn heading and 80 + `spread` px out, with the
    /// volcano's damage.
    fn burst(&self, mount: &Mount, heading: f32, spread: f32) -> Spawn {
        Spawn {
            turn: Some(heading - pyro::toward([0., 0.], mount.at)),
            offset: BURST_NEAR + spread,
            ..pyro::burst(BURST_SCALE + self.bonus(), mount.share)
        }
    }

    /// m_lavaTrail L0xa3-L0xad: a controller at its drawn heading with the volcano's damage.
    fn controller(&self, mount: &Mount, heading: f32) -> Spawn {
        Spawn {
            turn: Some(heading - pyro::toward([0., 0.], mount.at)),
            ..pyro::controller(self.tree.mods, LAVA_SPEED, mount.share)
        }
    }
}

/// The unseen object making `items` where the volcano stands.
fn carrier(items: Vec<Spawn>) -> Spawn {
    Spawn {
        object: STEP.into(),
        ability: true,
        life: Some(0),
        on_end: items,
        ..Spawn::default()
    }
}

/// Frames until a cooldown that Step_0 counts down from `cooldown` lets it act again.
fn countdown(cooldown: f32) -> u32 {
    (cooldown - EPSILON).ceil() as u32 + 1
}

#[cfg(test)]
#[path = "volcano_parity_tests.rs"]
mod parity;
#[cfg(test)]
#[path = "volcano_tests.rs"]
mod tests;
