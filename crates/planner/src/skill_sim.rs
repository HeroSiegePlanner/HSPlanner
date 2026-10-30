//! Frame-by-frame skill simulation against a training dummy (data/skill-fx.json).
use crate::hitbox::{self, HitObject, Mask, SkillHitbox};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

/// The dummy's collision circle, as the dps cards count it.
const DUMMY_RADIUS: i32 = 12;
/// Cursor skills appear 6 px above the cursor, which sits 24 px above the dummy.
const TARGET_ABOVE: f32 = 30.;
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
    /// The engine counts an average over random layouts; `spawn` replays one of them.
    #[cfg(test)]
    #[serde(default)]
    pub random: bool,
    pub spawn: Vec<Spawn>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Spawn {
    pub object: String,
    /// Frames after the use at which it is created.
    pub frame: u32,
    /// Created at the cursor instead of in front of the player.
    pub target: bool,
    /// Turned toward the dummy from where it is created.
    pub aim: bool,
    /// One instance per entry instead of one per projectile.
    pub layout: Vec<Placement>,
    /// Created at the enemy its parent touched.
    pub at_enemy: bool,
    /// Distance in front of the player along the aim.
    pub offset: f32,
    /// px per frame along the aim.
    pub speed: f32,
    pub fall: Option<Fall>,
    /// Bounces off the first enemy it touches.
    pub bounce: Option<Bounce>,
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

struct Instance {
    spawn: &'static Spawn,
    object: &'static HitObject,
    pos: [f32; 2],
    /// Degrees, counter-clockwise with y down, as GameMaker turns sprites.
    angle: f32,
    scale: [f32; 2],
    speed: f32,
    /// Height above the ground, its speed and the gravity pulling it.
    fall: Option<[f32; 3]>,
    bounced: bool,
    age: u32,
    end: u32,
    clock: HitClock,
}

struct Frame {
    hit: bool,
    /// Able to hit in this frame.
    active: bool,
    /// Gone after this frame; true when it ended on touching the dummy.
    ended: Option<bool>,
}

/// One object as the preview draws it in one frame.
#[derive(Clone, Copy)]
pub(crate) struct Shape {
    pub object: &'static HitObject,
    pub pos: [f32; 2],
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
}

impl Instance {
    fn new(
        spawn: &'static Spawn,
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
            angle,
            scale: spawn.scale.unwrap_or(object.scale.map(|s| s as f32)),
            speed: spawn.speed,
            fall,
            bounced: false,
            age: 0,
            end,
            clock: HitClock::default(),
        })
    }

    fn advance(&mut self) {
        let (sin, cos) = self.angle.to_radians().sin_cos();
        self.pos = [
            self.pos[0] + cos * self.speed,
            self.pos[1] - sin * self.speed,
        ];
        let gravity_from = if self.spawn.ability { 1 } else { ACTIVATION };
        if self.age >= gravity_from {
            if let Some(fall) = self.fall.as_mut() {
                fall[1] -= fall[2];
                fall[0] += fall[1];
            }
        }
        let spawn = self.spawn;
        let fading = fade_from(spawn).is_some_and(|from| self.age >= from);
        if fading && spawn.shrink > 0. {
            self.scale = self.scale.map(|s| (s - spawn.shrink).max(0.));
        } else if let Some(grow) = spawn.grow.filter(|_| !fading) {
            let next = |s: f32| {
                let s = s + grow.step + grow.max.map_or(0., |m| (m - s) * grow.lerp);
                grow.max.map_or(s, |m| s.min(m))
            };
            self.scale = if grow.x_only {
                [next(self.scale[0]), self.scale[1]]
            } else {
                self.scale.map(next)
            };
        }
    }

    fn step(&mut self, dummy: [f32; 2]) -> Frame {
        if self.age > 0 {
            self.advance();
        }
        let spawn = self.spawn;
        let landed = self.fall.is_some_and(|[height, ..]| height <= 0.);
        let active = self.age >= spawn.active_from.unwrap_or(ACTIVATION)
            && spawn.deadly.is_none_or(|deadly| self.age <= deadly + 1);
        let looks = (active && (!spawn.ability || spawn.ends_on_contact))
            || (spawn.bounce.is_some() && !self.bounced);
        let overlap = looks && overlaps(self.object, self.scale, self.pos, self.angle, dummy);
        if overlap && !self.bounced {
            if let (Some(bounce), Some(fall)) = (spawn.bounce, self.fall.as_mut()) {
                self.bounced = true;
                self.speed *= bounce.speed_mult;
                fall[1] = bounce.z_speed;
                fall[2] = bounce.gravity;
            }
        }
        let hit = active && !spawn.ability && self.clock.frame(overlap, spawn.tick);
        self.age += 1;
        let ended = if spawn.ends_on_contact && active && overlap {
            Some(true)
        } else {
            (landed || self.age >= self.end).then_some(false)
        };
        Frame {
            hit,
            active: active && !spawn.ability,
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
        runs.chunks_exact(2)
            .any(|run| run[0] <= col && col < run[1])
    })
}

fn known(spawns: &[Spawn], hitbox: &SkillHitbox) -> bool {
    spawns.iter().all(|spawn| {
        hitbox.objects.iter().any(|o| o.name == spawn.object) && known(&spawn.on_end, hitbox)
    })
}

/// Frames in which one use with `projectiles` projectiles hits the dummy.
#[cfg(test)]
pub(crate) fn simulate(skill_id: &str, projectiles: u32) -> Option<Vec<u32>> {
    run(skill_id, projectiles).map(|timeline| timeline.hits)
}

static TIMELINES: LazyLock<Mutex<HashMap<String, Arc<Timeline>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The base use of a skill with motion data, simulated once and kept for the preview.
pub(crate) fn timeline(skill_id: &str) -> Option<Arc<Timeline>> {
    let mut cache = TIMELINES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(timeline) = cache.get(skill_id) {
        return Some(timeline.clone());
    }
    let timeline = Arc::new(run(skill_id, 1)?);
    cache.insert(skill_id.to_string(), timeline.clone());
    Some(timeline)
}

fn run(skill_id: &str, projectiles: u32) -> Option<Timeline> {
    let fx = skill(skill_id)?;
    let hitbox = hitbox::skill(skill_id)?;
    if !known(&fx.spawn, hitbox) {
        return None;
    }
    let dummy = [fx.dummy, 0.];
    let n = projectiles.max(1);
    let mut pending = Vec::new();
    for spawn in &fx.spawn {
        let base = if spawn.target {
            [fx.dummy, -TARGET_ABOVE]
        } else {
            [0., 0.]
        };
        for (i, place) in spawn.layout.iter().enumerate() {
            let at = [base[0] + place.offset[0], base[1] + place.offset[1]];
            let instance = Instance::new(spawn, hitbox, at, 0., i as u32);
            pending.extend(instance.map(|instance| (place.frame, instance)));
        }
        for i in (0..n).filter(|_| spawn.layout.is_empty()) {
            let angle = if spawn.aim {
                (base[1] - dummy[1]).atan2(dummy[0] - base[0]).to_degrees()
            } else {
                fx.fan * (i as f32 - (n - 1) as f32 / 2.)
            };
            let (sin, cos) = angle.to_radians().sin_cos();
            let at = [base[0] + cos * spawn.offset, base[1] - sin * spawn.offset];
            let instance = Instance::new(spawn, hitbox, at, angle, i);
            pending.extend(instance.map(|instance| (spawn.frame, instance)));
        }
    }
    let mut live = Vec::new();
    let mut hits = Vec::new();
    let mut frames = Vec::new();
    for frame in 0..MAX_FRAMES {
        let (due, later): (Vec<_>, Vec<_>) = pending.into_iter().partition(|(at, _)| *at <= frame);
        pending = later;
        live.extend(due.into_iter().map(|(_, instance)| instance));
        if live.is_empty() && pending.is_empty() {
            break;
        }
        let mut born = Vec::new();
        let mut shapes = Vec::with_capacity(live.len());
        live.retain_mut(|instance| {
            let step = instance.step(dummy);
            shapes.push(instance.shape(step.active));
            if step.hit {
                hits.push(frame);
            }
            let Some(by_contact) = step.ended else {
                return true;
            };
            for child in &instance.spawn.on_end {
                let at = if child.at_enemy && by_contact {
                    dummy
                } else {
                    instance.pos
                };
                born.extend(Instance::new(child, hitbox, at, 0., 0));
            }
            false
        });
        frames.push(shapes);
        live.extend(born);
    }
    Some(Timeline {
        dummy,
        frames,
        hits,
    })
}

#[cfg(test)]
mod tests {
    use super::{FX, simulate};
    use hsplanner_engine::calc::data;

    /// The engine's base dummy hits of a skill, whatever its class.
    fn engine_hits(skill: &str) -> f64 {
        let classes = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/skills"))
            .expect("class skill files");
        classes
            .filter_map(|entry| Some(entry.ok()?.path().file_stem()?.to_str()?.to_owned()))
            .find_map(|class| {
                data::get_skills_by_class(&class)
                    .iter()
                    .find(|s| s.id == skill)
                    .and_then(|s| s.dummy_hits.as_ref())
                    .map(|d| d.base)
            })
            .unwrap_or(f64::NAN)
    }

    fn sorted_skills() -> Vec<&'static String> {
        let mut skills: Vec<_> = FX.skills.keys().collect();
        skills.sort();
        skills
    }

    #[test]
    fn simulated_hits_match_the_engine_dummy_counts() {
        let off: Vec<String> = sorted_skills()
            .into_iter()
            .filter_map(|skill| {
                let got = simulate(skill, 1).expect(skill).len() as f64;
                let engine = engine_hits(skill);
                // A random layout replays one case of the engine's average.
                let off = if FX.skills[skill].random {
                    (got - engine).abs() >= 0.5
                } else {
                    got != engine
                };
                off.then(|| format!("{skill}: sim {got}, engine {engine}"))
            })
            .collect();
        assert!(off.is_empty(), "{off:#?}");
    }

    #[test]
    fn breath_of_fire_grows_until_its_fade_starts() {
        let timeline = super::timeline("breath_of_fire").expect("breath of fire");
        let widest = timeline
            .frames
            .iter()
            .flatten()
            .map(|shape| shape.scale[0])
            .fold(0., f32::max);
        assert!((widest - 1.55).abs() < 1e-3, "widest x scale {widest}");
    }

    #[test]
    #[ignore]
    fn print_hits_per_projectile_count() {
        for skill in sorted_skills() {
            let counts: Vec<usize> = (1..=5)
                .map(|n| simulate(skill, n).map_or(0, |h| h.len()))
                .collect();
            eprintln!("{skill}: {counts:?} engine {}", engine_hits(skill));
        }
    }
}
