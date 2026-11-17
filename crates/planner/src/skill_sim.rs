//! Frame-by-frame skill simulation against a training dummy (data/skill-fx.json).
use crate::hitbox::{self, HitObject, Mask, SkillHitbox};
use crate::skill_effects::{
    Buff, Dice, Drain, Effects, Frostbite, Orbit, Proc, Pull, Ramp, Scatter, Stacks, Swell,
};
use crate::skill_mods::{self, Mods};
use crate::skill_motion::{Accel, Course, Rebound, Trail};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

/// The dummy's collision circle, as the dps cards count it.
const DUMMY_RADIUS: i32 = 12;
/// Cursor skills appear 6 px above the cursor, which sits 24 px above the dummy.
pub(crate) const TARGET_ABOVE: f32 = 30.;
/// After a contact a damage object checks collision again only every 3 frames.
const CHECK_EVERY: u32 = 3;
/// Damage objects turn deadly and start falling 2 frames after creation (parent Alarm_11).
const ACTIVATION: u32 = 2;
/// A destroyTimer that runs out sets alarm[10] = 1; Alarm_10 starts the fade a frame later.
const FADE_DELAY: u32 = 2;
/// Life of an object whose end the game leaves to its animation.
const ANIMATION_LIFE: u32 = 30;
/// Safety stop for objects that end only on landing or contact.
const MAX_FRAMES: u32 = 3600;
/// Live objects of a use at most: what procs make past it is left out, to keep cascades drawable.
const MAX_LIVE: usize = 256;

#[derive(Deserialize)]
struct SkillFxFile {
    skills: HashMap<String, SkillFx>,
}

#[derive(Deserialize)]
pub(crate) struct SkillFx {
    /// Distance from the player at which the engine's dummyHits count holds.
    pub dummy: f32,
    /// Degrees between the projectiles of one volley, centred on the aim.
    #[serde(default)]
    pub fan: f32,
    /// Degrees a volley of more than one turns past its centre.
    #[serde(default)]
    pub skew: f32,
    /// The engine counts an average over random layouts; `spawn` replays one of them.
    #[cfg(test)]
    #[serde(default)]
    pub random: bool,
    pub spawn: Vec<Spawn>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Spawn {
    pub object: String,
    /// Frames after the use, or after its creator makes it, at which it is created.
    pub frame: u32,
    /// Rolled once per use, or each time its creator ends: made, with all it makes, if it comes up.
    pub chance: Option<f32>,
    /// Created at a random distance and direction from where it would be.
    pub scatter: Option<Scatter>,
    /// Created at the cursor instead of in front of the player.
    pub target: bool,
    /// Turned toward the dummy from where it is created.
    pub aim: bool,
    /// One instance per entry instead of one per projectile.
    pub layout: Vec<Placement>,
    /// Projectiles it adds to the use's volley, fanned out with them.
    pub extra: u32,
    /// Created at the enemy its parent touched.
    pub at_enemy: bool,
    /// Made by another: created at the player instead of where its creator is.
    pub at_player: bool,
    /// Distance in front of the player along the aim; made by another, out from its creator
    /// along its own direction.
    pub offset: f32,
    /// Made by another: flies this many degrees from the line from the player through its creator.
    pub turn: Option<f32>,
    /// px per frame along the aim.
    pub speed: f32,
    /// Starts slower and speeds up to `speed`.
    pub accel: Option<Accel>,
    pub fall: Option<Fall>,
    /// Bounces off the first enemy it touches.
    pub bounce: Option<Bounce>,
    pub rebound: Option<Rebound>,
    pub trail: Option<Box<Trail>>,
    pub scale: Option<[f32; 2]>,
    pub grow: Option<Grow>,
    pub life: Option<u32>,
    /// Frames it lives on after `life`, losing `shrink` scale per frame.
    pub fade: u32,
    pub shrink: f32,
    pub active_from: Option<u32>,
    /// deadlyFrames, counted down from activation: the last frame it can hit.
    pub deadly: Option<u32>,
    /// Frames between hit-map clears; 0 hits each target once.
    pub tick: f32,
    /// A Player_Ability_Parent_obj: it hits only through what it creates.
    pub ability: bool,
    pub ends_on_contact: bool,
    pub on_end: Vec<Spawn>,
    /// Height under which a falling object creates `on_low`, once.
    pub low: f32,
    pub on_low: Vec<Spawn>,
    /// Circles the player instead of flying: one instance per `count`, evenly spread.
    pub orbit: Option<Orbit>,
    /// Chance rolls on each of its hits that create another object.
    pub procs: Vec<Proc>,
    /// Chance rolls that raise its maximum scale while it lives.
    pub swell: Option<Swell>,
    /// Share of the skill's hit each of its hits deals; 1 when unset.
    pub share: Option<f32>,
    /// Grows its hits, and the hits of all it creates, while it lives.
    pub ramp: Option<Ramp>,
    /// Rolled once per use; then each of its hits frostbites the enemy.
    pub frostbite: Option<Frostbite>,
    /// Extra share of its hits on a frostbitten enemy.
    pub vs_frostbitten: f32,
    /// Mana it takes from the player while it lives.
    pub drain: Option<Drain>,
    /// A buff the player gets when it is created.
    pub buff: Option<Buff>,
    /// Pulls the enemy each of its hits touches.
    pub pull: Option<Pull>,
    /// Made by a proc only on the hit that finds the player's stacks full.
    pub stacks: Option<Stacks>,
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Placement {
    pub frame: u32,
    /// px from where the spawn is created, y down.
    pub offset: [f32; 2],
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Fall {
    pub height: f32,
    /// Added to the height of each further projectile.
    pub height_step: f32,
    pub z_speed: f32,
    pub gravity: f32,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Bounce {
    pub z_speed: f32,
    pub speed_mult: f32,
    pub gravity: f32,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Grow {
    /// Scale added per frame.
    pub step: f32,
    /// Share of the gap to `max` closed per frame.
    pub lerp: f32,
    pub max: Option<f32>,
    pub x_only: bool,
}

static FX: LazyLock<SkillFxFile> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/skill-fx.json"))
        .expect("embedded skill motion must be valid")
});

pub(crate) fn skill(id: &str) -> Option<&'static SkillFx> {
    FX.skills.get(id)
}

/// One target's hit map as the game's damage objects keep it (METODA rule 20, `tick_hits`).
#[derive(Default)]
struct HitClock {
    tick: f32,
    cooldown: u32,
    hit_map: bool,
}

impl HitClock {
    /// One active frame; true when it lands a new hit.
    fn frame(&mut self, overlap: bool, tick_frames: f32) -> bool {
        self.cooldown = self.cooldown.saturating_sub(1);
        let mut hit = false;
        if self.cooldown == 0 && overlap {
            self.cooldown = CHECK_EVERY;
            hit = !self.hit_map;
            self.hit_map = true;
        }
        if tick_frames > 0. {
            if self.tick > 0. {
                self.tick -= 1.;
                if self.tick <= 0. {
                    self.hit_map = false;
                }
            } else {
                self.tick = tick_frames;
            }
        }
        hit
    }
}

struct Instance<'a> {
    spawn: &'a Spawn,
    object: &'static HitObject,
    pos: [f32; 2],
    /// Degrees, counter-clockwise with y down, as GameMaker turns sprites.
    angle: f32,
    scale: [f32; 2],
    speed: f32,
    /// Height above the ground, its speed and the gravity pulling it.
    fall: Option<[f32; 3]>,
    bounced: bool,
    /// The scale it grows toward, raised by a swell.
    max: Option<f32>,
    /// Share of the skill's hit a swell added to its hits.
    bonus: f32,
    /// Growth its creator's hits had when it made it.
    carry: f32,
    /// Whether this use's frostbite roll came up.
    frostbites: bool,
    /// Whether it already fell under its `low` height.
    lowered: bool,
    course: Course,
    age: u32,
    end: u32,
    clock: HitClock,
}

struct Frame {
    hit: bool,
    /// Able to hit in this frame.
    active: bool,
    /// Fell under its `low` height in this frame.
    low: bool,
    /// Gone after this frame; true when it ended on touching the dummy.
    ended: Option<bool>,
}

/// One object as the preview draws it in one frame.
#[derive(Clone, Copy)]
pub(crate) struct Shape {
    pub object: &'static HitObject,
    pub pos: [f32; 2],
    /// Degrees it is turned, counter-clockwise with y down.
    pub angle: f32,
    /// Height above the ground of a falling object.
    pub height: f32,
    pub scale: [f32; 2],
    pub active: bool,
    /// Frames since it was created.
    pub age: u32,
    /// Share of its fade-out already gone: 0 until `life` ends, 1 when it is gone.
    pub fade: f32,
}

/// One use replayed frame by frame: what is on screen and when the dummy is hit.
pub(crate) struct Timeline {
    pub dummy: [f32; 2],
    pub frames: Vec<Vec<Shape>>,
    pub hits: Vec<u32>,
    pub effects: Effects,
}

impl<'a> Instance<'a> {
    fn new(
        spawn: &'a Spawn,
        hitbox: &'static SkillHitbox,
        pos: [f32; 2],
        angle: f32,
        index: u32,
    ) -> Option<Self> {
        let object = hitbox.objects.iter().find(|o| o.name == spawn.object)?;
        let fall = spawn.fall.map(|f| {
            [
                f.height + f.height_step * index as f32,
                f.z_speed,
                f.gravity,
            ]
        });
        let end = match (spawn.life, fall) {
            (Some(_), _) => fade_from(spawn).unwrap_or(0) + spawn.fade,
            (None, Some(_)) => MAX_FRAMES,
            (None, None) => ANIMATION_LIFE.max(spawn.deadly.unwrap_or(0)),
        };
        Some(Self {
            spawn,
            object,
            pos,
            angle: spawn.orbit.map_or(angle, |orbit| orbit.facing(angle)),
            scale: spawn.scale.unwrap_or(object.scale.map(|s| s as f32)),
            speed: spawn.accel.map_or(spawn.speed, |accel| accel.from),
            fall,
            bounced: false,
            max: spawn.grow.and_then(|grow| grow.max),
            bonus: 0.,
            carry: 1.,
            frostbites: false,
            lowered: false,
            course: Course::new(spawn, angle),
            age: 0,
            end,
            clock: HitClock::default(),
        })
    }

    /// GameMaker's lengthdir around the player, recomputed every frame.
    fn circle(&mut self, orbit: Orbit) {
        let forgets = self
            .course
            .orbit(orbit, self.age, &mut self.pos, &mut self.angle);
        self.clock.hit_map &= !forgets;
    }

    /// The growth of its hits in this step: what its creator handed down and its own ramp.
    fn grown(&self) -> f32 {
        let steps = self.age.saturating_sub(1);
        self.carry * self.spawn.ramp.map_or(1., |ramp| ramp.growth(steps))
    }

    fn advance(&mut self) {
        let (sin, cos) = self.angle.to_radians().sin_cos();
        self.pos = [
            self.pos[0] + cos * self.speed,
            self.pos[1] - sin * self.speed,
        ];
        if let Some(accel) = self.spawn.accel {
            self.speed = accel.next(self.speed, self.spawn.speed);
        }
        let gravity_from = if self.spawn.ability { 1 } else { ACTIVATION };
        if self.age >= gravity_from
            && let Some(fall) = self.fall.as_mut()
        {
            fall[1] -= fall[2];
            fall[0] += fall[1];
        }
        let spawn = self.spawn;
        let fading = fade_from(spawn).is_some_and(|from| self.age >= from);
        if fading && spawn.shrink > 0. {
            self.scale = self.scale.map(|s| (s - spawn.shrink).max(0.));
        } else if let Some(grow) = spawn.grow.filter(|_| !fading) {
            let max = self.max;
            let next = |s: f32| {
                let s = s + grow.step + max.map_or(0., |m| (m - s) * grow.lerp);
                max.map_or(s, |m| s.min(m))
            };
            self.scale = if grow.x_only {
                [next(self.scale[0]), self.scale[1]]
            } else {
                self.scale.map(next)
            };
        }
    }

    fn step(&mut self, dummy: [f32; 2], dice: &mut Dice) -> Frame {
        if let Some(orbit) = self.spawn.orbit {
            self.circle(orbit);
        }
        if self.age > 0 {
            self.advance();
        }
        self.angle = self.course.home(self.pos, dummy, self.angle, &mut self.end);
        if let (Some(swell), Some(max)) = (self.spawn.swell, self.max.as_mut())
            && self.age > 0
            && self.age.is_multiple_of(swell.every.max(1))
            && dice.roll(swell.chance)
        {
            *max += swell.step;
            self.bonus += swell.damage;
        }
        let spawn = self.spawn;
        let landed = self.fall.is_some_and(|[height, ..]| height <= 0.);
        let low = !self.lowered
            && !spawn.on_low.is_empty()
            && self.fall.is_some_and(|[height, ..]| height < spawn.low);
        self.lowered |= low;
        let active = self.age >= spawn.active_from.unwrap_or(ACTIVATION)
            && spawn.deadly.is_none_or(|deadly| self.age <= deadly + 1);
        let looks = (active && (!spawn.ability || spawn.ends_on_contact))
            || (spawn.bounce.is_some() && !self.bounced);
        let overlap = looks && overlaps(self.object, self.scale, self.pos, self.angle, dummy);
        if overlap
            && !self.bounced
            && let (Some(bounce), Some(fall)) = (spawn.bounce, self.fall.as_mut())
        {
            self.bounced = true;
            self.speed *= bounce.speed_mult;
            fall[1] = bounce.z_speed;
            fall[2] = bounce.gravity;
        }
        let hit = active && !spawn.ability && self.clock.frame(overlap, spawn.tick);
        if self.course.react(spawn, self.pos, dummy, hit, dice) {
            self.angle = aim(self.pos, dummy);
            self.clock.hit_map = false;
        }
        self.age += 1;
        let ended = if spawn.ends_on_contact && active && overlap {
            Some(true)
        } else {
            (landed || self.age >= self.end).then_some(false)
        };
        Frame {
            hit,
            active: active && !spawn.ability,
            low,
            ended,
        }
    }

    fn shape(&self, active: bool) -> Shape {
        let fade = match fade_from(self.spawn) {
            Some(from) if self.spawn.fade > 0 => {
                self.age.saturating_sub(from) as f32 / self.spawn.fade as f32
            }
            _ => 0.,
        };
        Shape {
            object: self.object,
            pos: self.pos,
            angle: self.angle,
            height: self.fall.map_or(0., |[height, ..]| height.max(0.)),
            scale: self.scale,
            active,
            age: self.age,
            fade,
        }
    }
}

/// The frame its fade starts: a destroyTimer with a fade runs `FADE_DELAY` frames past `life`;
/// an object that ends with its animation goes at once.
fn fade_from(spawn: &Spawn) -> Option<u32> {
    let delay = if spawn.fade > 0 { FADE_DELAY } else { 0 };
    spawn.life.map(|life| life + delay)
}

/// Whether a pixel of the dummy's circle falls on the object's collision mask.
fn overlaps(
    object: &HitObject,
    scale: [f32; 2],
    pos: [f32; 2],
    angle: f32,
    dummy: [f32; 2],
) -> bool {
    if scale[0] <= 0. || scale[1] <= 0. {
        return false;
    }
    let reach = object.reach * scale[0].max(scale[1]) + DUMMY_RADIUS as f32 + 1.;
    if object.reach > 0. && (dummy[0] - pos[0]).hypot(dummy[1] - pos[1]) > reach {
        return false;
    }
    let (sin, cos) = angle.to_radians().sin_cos();
    let mask = object.mask.as_deref().and_then(hitbox::mask);
    let r = DUMMY_RADIUS;
    (-r..=r).any(|j| {
        (-r..=r).any(|i| {
            if i * i + j * j > r * r {
                return false;
            }
            let (wx, wy) = (dummy[0] + i as f32 - pos[0], dummy[1] + j as f32 - pos[1]);
            // Undo GameMaker's turn (counter-clockwise, y down), then the scale.
            let x = (wx * cos - wy * sin) / scale[0] + object.origin[0];
            let y = (wx * sin + wy * cos) / scale[1] + object.origin[1];
            match (mask, object.bounds) {
                (Some(mask), _) => on_mask(mask, x - object.at[0], y - object.at[1]),
                (None, Some([x0, y0, x1, y1])) => {
                    (x0..x1 + 1.).contains(&x) && (y0..y1 + 1.).contains(&y)
                }
                (None, None) => false,
            }
        })
    })
}

fn on_mask(mask: &Mask, x: f32, y: f32) -> bool {
    if x < 0. || y < 0. {
        return false;
    }
    let col = x as u16;
    mask.rows.get(y as usize).is_some_and(|runs| {
        runs.as_chunks::<2>()
            .0
            .iter()
            .any(|&[start, end]| start <= col && col < end)
    })
}

fn known(spawns: &[Spawn], hitbox: &SkillHitbox) -> bool {
    spawns.iter().all(|spawn| {
        hitbox.objects.iter().any(|o| o.name == spawn.object)
            && known(&spawn.on_end, hitbox)
            && known(&spawn.on_low, hitbox)
            && spawn
                .trail
                .iter()
                .all(|t| known(std::slice::from_ref(&t.spawn), hitbox))
            && spawn
                .procs
                .iter()
                .all(|proc| known(std::slice::from_ref(&proc.spawn), hitbox))
    })
}

/// GameMaker's point_direction from `from` to `to`.
fn aim(from: [f32; 2], to: [f32; 2]) -> f32 {
    (from[1] - to[1]).atan2(to[0] - from[0]).to_degrees()
}

/// An object another creates: at the enemy it hit or where the creator is, turned to the dummy
/// when it aims or from the creator's throw when it turns, with the use's frostbite roll and the
/// growth of the creator's hits.
fn child<'a>(
    spawn: &'a Spawn,
    hitbox: &'static SkillHitbox,
    creator: &Instance,
    dummy: [f32; 2],
    on_enemy: bool,
) -> Option<Instance<'a>> {
    let at = if spawn.at_enemy && on_enemy {
        dummy
    } else if spawn.at_player {
        [0., 0.]
    } else {
        creator.pos
    };
    let angle = match spawn.turn {
        Some(turn) => aim([0., 0.], creator.pos) + turn,
        None if spawn.aim => aim(at, dummy),
        None => 0.,
    };
    let (sin, cos) = angle.to_radians().sin_cos();
    let at = [at[0] + cos * spawn.offset, at[1] - sin * spawn.offset];
    let mut instance = Instance::new(spawn, hitbox, at, angle, 0)?;
    instance.frostbites = creator.frostbites;
    instance.carry = creator.grown();
    Some(instance)
}

/// Frames in which one use with `projectiles` projectiles hits the dummy.
#[cfg(test)]
pub(crate) fn simulate(skill_id: &str, projectiles: u32) -> Option<Vec<u32>> {
    run(skill_id, &Mods::default(), projectiles).map(|timeline| timeline.hits)
}

/// Uses replayed with other dice when chance plays a part in a use.
const CHANCY_USES: u64 = 8;

/// The modifiers uses were simulated with, and the uses.
type Simulated = (Mods, Arc<[Timeline]>);

/// The last uses simulated per skill.
static TIMELINES: LazyLock<Mutex<HashMap<String, Simulated>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Uses of a skill with motion data under the build's modifiers, kept until they change: one, or
/// several rolled with other dice when chance plays a part.
pub(crate) fn uses(skill_id: &str, mods: &Mods) -> Option<Arc<[Timeline]>> {
    let mut cache = TIMELINES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((cached, uses)) = cache.get(skill_id)
        && cached == mods
    {
        return Some(uses.clone());
    }
    let spawns = skill_mods::apply(skill_id, &skill(skill_id)?.spawn, mods);
    let count = skill_mods::casts(skill_id, if chancy(&spawns) { CHANCY_USES } else { 1 });
    let uses: Arc<[Timeline]> = (0..count)
        .map(|n| play(skill_id, mods, 1, n))
        .collect::<Option<_>>()?;
    cache.insert(skill_id.to_string(), (mods.clone(), uses.clone()));
    Some(uses)
}

/// Whether chance plays a part in a use: procs, swells, scatters, frostbite or creation rolls.
fn chancy(spawns: &[Spawn]) -> bool {
    spawns.iter().any(|spawn| {
        !spawn.procs.is_empty()
            || spawn.swell.is_some()
            || spawn.frostbite.is_some()
            || spawn.chance.is_some()
            || spawn.scatter.is_some()
            || chancy(&spawn.on_end)
            || chancy(&spawn.on_low)
    })
}

#[cfg(test)]
pub(crate) fn run(skill_id: &str, mods: &Mods, projectiles: u32) -> Option<Timeline> {
    play(skill_id, mods, projectiles, 0)
}

/// The `n`-th use, rolled with its own dice.
fn play(skill_id: &str, mods: &Mods, projectiles: u32, n: u64) -> Option<Timeline> {
    let fx = skill(skill_id)?;
    let hitbox = hitbox::skill(skill_id)?;
    let spawns = skill_mods::apply_cast(skill_id, &fx.spawn, mods, n);
    known(&spawns, hitbox).then(|| replay_use(fx, hitbox, &spawns, projectiles, n))
}

/// The `n`-th use of these spawns against the skill's dummy.
pub(crate) fn replay_use(
    fx: &SkillFx,
    hitbox: &'static SkillHitbox,
    spawns: &[Spawn],
    projectiles: u32,
    n: u64,
) -> Timeline {
    let mut dice = Dice::of_use(n);
    let pending = place(spawns, hitbox, fx, projectiles.max(1), &mut dice);
    replay(pending, hitbox, [fx.dummy, 0.], &mut dice)
}

/// Where a scattered spawn lands instead of `at`.
fn scattered(at: [f32; 2], spawn: &Spawn, dice: &mut Dice) -> [f32; 2] {
    spawn.scatter.map_or(at, |scatter| {
        let [x, y] = scatter.offset(dice);
        [at[0] + x, at[1] + y]
    })
}

/// Every instance of the spawns and the frame it is created on; the use rolls frostbite once.
fn place<'a>(
    spawns: &'a [Spawn],
    hitbox: &'static SkillHitbox,
    fx: &SkillFx,
    n: u32,
    dice: &mut Dice,
) -> Vec<(u32, Instance<'a>)> {
    let dummy = [fx.dummy, 0.];
    let mut pending = Vec::new();
    let frostbites = spawns
        .iter()
        .find_map(|spawn| spawn.frostbite)
        .is_some_and(|f| dice.roll(f.chance));
    for spawn in spawns {
        if spawn.chance.is_some_and(|chance| !dice.roll(chance)) {
            continue;
        }
        let created = |instance: Option<Instance<'a>>, frame: u32| {
            instance.map(|mut instance| {
                instance.frostbites = frostbites;
                (frame, instance)
            })
        };
        let base = if spawn.target {
            [fx.dummy, -TARGET_ABOVE]
        } else {
            [0., 0.]
        };
        for (i, place) in spawn.layout.iter().enumerate() {
            let at = [base[0] + place.offset[0], base[1] + place.offset[1]];
            let at = scattered(at, spawn, dice);
            pending.extend(created(
                Instance::new(spawn, hitbox, at, 0., i as u32),
                place.frame,
            ));
        }
        let count = spawn
            .orbit
            .map_or(n + spawn.extra, |orbit| orbit.count.max(1));
        for i in (0..count).filter(|_| spawn.layout.is_empty()) {
            let angle = if spawn.orbit.is_some() {
                360. * i as f32 / count as f32
            } else if spawn.aim {
                aim(base, dummy)
            } else {
                fx.fanned(i, count)
            };
            let (sin, cos) = angle.to_radians().sin_cos();
            let at = [base[0] + cos * spawn.offset, base[1] - sin * spawn.offset];
            let at = scattered(at, spawn, dice);
            pending.extend(created(
                Instance::new(spawn, hitbox, at, angle, i),
                spawn.frame,
            ));
        }
    }
    pending
}

/// The instances played frame by frame against the dummy.
fn replay(
    mut pending: Vec<(u32, Instance)>,
    hitbox: &'static SkillHitbox,
    dummy: [f32; 2],
    dice: &mut Dice,
) -> Timeline {
    let (mut live, mut frames) = (Vec::new(), Vec::new());
    let mut stage = Stage {
        hitbox,
        dummy,
        dice,
        effects: Effects::default(),
        hits: Vec::new(),
    };
    for frame in 0..MAX_FRAMES {
        let (due, later): (Vec<_>, Vec<_>) = pending.into_iter().partition(|(at, _)| *at <= frame);
        pending = later;
        for (_, instance) in due {
            stage.effects.created(frame, instance.spawn);
            live.push(instance);
        }
        if live.is_empty() && pending.is_empty() {
            break;
        }
        let mut born = Vec::new();
        let mut shapes = Vec::with_capacity(live.len());
        live.retain_mut(|instance| {
            let step = instance.step(dummy, stage.dice);
            shapes.push(instance.shape(step.active));
            stage.effects.alive(frame, instance.spawn, instance.age);
            stage.leaves(frame, instance, &step, &mut born);
            step.ended.is_none()
        });
        frames.push(shapes);
        let room = MAX_LIVE.saturating_sub(live.len() + pending.len());
        for instance in born.into_iter().take(room) {
            match instance.spawn.frame {
                0 => {
                    stage.effects.created(frame, instance.spawn);
                    live.push(instance);
                }
                delay => pending.push((frame + 1 + delay, instance)),
            }
        }
    }
    Timeline {
        dummy,
        frames,
        hits: stage.hits,
        effects: stage.effects,
    }
}

/// What the replay keeps while instances step: the hitboxes, the dummy, the dice and the record.
struct Stage<'d> {
    hitbox: &'static SkillHitbox,
    dummy: [f32; 2],
    dice: &'d mut Dice,
    effects: Effects,
    hits: Vec<u32>,
}

impl Stage<'_> {
    /// What one step of an instance leaves: its hit with its pull and procs, then what it creates
    /// on falling low and on ending.
    fn leaves<'a>(
        &mut self,
        frame: u32,
        instance: &Instance<'a>,
        step: &Frame,
        born: &mut Vec<Instance<'a>>,
    ) {
        let (spawn, dummy) = (instance.spawn, self.dummy);
        if step.hit {
            self.hits.push(frame);
            let grown = instance.grown();
            self.effects
                .hit(frame, spawn, instance.bonus, grown, instance.frostbites);
            if let Some(pull) = spawn.pull {
                self.effects.pull(frame, pull, instance.pos, dummy);
            }
            let piled = self.effects.pile(spawn, instance.course.hits());
            for proc in Proc::of_hit(&spawn.procs, instance.course.hits(), piled) {
                if !self.dice.roll(proc.chance) {
                    continue;
                }
                born.extend(child(&proc.spawn, self.hitbox, instance, dummy, true));
                if let Some(pull) = proc.pull {
                    self.effects.pull(frame, pull, instance.pos, dummy);
                }
            }
        }
        let moves = instance.age.saturating_sub(1);
        if let Some(drop) = Trail::dropped(spawn, moves) {
            born.extend(child(drop, self.hitbox, instance, dummy, false));
        }
        let low = spawn.on_low.iter().filter(|_| step.low);
        born.extend(low.filter_map(|low| child(low, self.hitbox, instance, dummy, false)));
        if let Some(by_contact) = step.ended {
            let ends = spawn.on_end.iter().filter(|end| self.dice.comes_up(end));
            born.extend(
                ends.filter_map(|end| child(end, self.hitbox, instance, dummy, by_contact)),
            );
        }
    }
}

#[cfg(test)]
#[path = "skill_sim_tests.rs"]
mod tests;
