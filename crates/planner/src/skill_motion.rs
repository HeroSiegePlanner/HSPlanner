//! How a simulated object moves besides flying straight: its speed ramp, the turns that send it
//! back at the enemy, its lunges off an orbit, and what it leaves along its way.
use crate::skill_effects::{Dice, Lunge, Orbit, Ride};
use crate::skill_homing::Seeker;
use crate::skill_sim::Spawn;
use crate::skill_steer::{Steer, Steering, travel};
use serde::Deserialize;

/// GameMaker's deltaSpeed ramp: the object starts at `from` px a frame and after each move its
/// speed grows by `rate` of itself plus `step`, until it reaches its spawn's speed; a negative
/// `rate` slows it down instead, to its spawn's speed at the least.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Accel {
    pub from: f32,
    pub step: f32,
    pub rate: f32,
    /// Turns it toward a point before each move, as a charge with no distance turns.
    #[serde(default)]
    pub steer: Option<Steer>,
}

impl Accel {
    /// The speed of the next move after one at `speed`, capped at `max`, or kept above it.
    pub(crate) fn next(self, speed: f32, max: f32) -> f32 {
        let next = speed * (1. + self.rate) + self.step;
        if self.rate < 0. {
            return next.max(max);
        }
        if speed >= max {
            return speed;
        }
        next.min(max)
    }
}

/// Bounces between enemies: every `every` frames, from its first, it turns to the nearest enemy
/// within `reach` px and forgets whom it hit, `count` times; a lone enemy is its own next one.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Rebound {
    pub count: u32,
    pub every: u32,
    pub reach: f32,
}

/// What steers an object besides its spawn: its turn around the player and the degrees turned
/// since it last forgot whom it hit, its rebounds left with the frames until it looks for an
/// enemy again, a lunge off its orbit under way, the homing of its own object and the hits it
/// has landed.
pub(crate) struct Course {
    /// Degrees around the player of an orbiting object.
    turn: f32,
    turned: f32,
    left: u32,
    wait: u32,
    lunge: Option<Lunging>,
    seeker: Option<Seeker>,
    steering: Option<Steering>,
    /// Frames it has homed through, the age of its object.
    age: u32,
    hits: u32,
}

const FULL_TURN: f32 = 360.;

/// A lunge: frames left flying toward the enemy hit and the direction, then the way back.
#[derive(Clone, Copy)]
enum Lunging {
    Out { left: u32, toward: f32 },
    Back,
}

/// GameMaker's point_direction from `from` to `to`.
fn toward(from: [f32; 2], to: [f32; 2]) -> f32 {
    (from[1] - to[1]).atan2(to[0] - from[0]).to_degrees()
}

fn moved(pos: [f32; 2], degrees: f32, speed: f32) -> [f32; 2] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [pos[0] + cos * speed, pos[1] - sin * speed]
}

impl Course {
    /// The course of an object made at `angle`: an orbiting one circles from there past its `at`.
    pub(crate) fn new(spawn: &Spawn, angle: f32) -> Self {
        Self {
            turn: spawn.orbit.map_or(0., |orbit| angle + orbit.at),
            turned: 0.,
            left: spawn.rebound.map_or(0, |rebound| rebound.count),
            wait: 0,
            lunge: None,
            seeker: Seeker::of(spawn),
            steering: Steering::of(spawn),
            age: 0,
            hits: 0,
        }
    }

    /// Hits its object has landed so far.
    pub(crate) fn hits(&self) -> u32 {
        self.hits
    }

    /// Its own Step after moving to `pos`, once a frame: a homing object's angle turned toward the
    /// enemy, ending it once its centre meets it, from its first Step (Alarm_0 sets the target);
    /// before that it takes the direction its object starts from, if any.
    pub(crate) fn home(
        &mut self,
        pos: [f32; 2],
        enemy: [f32; 2],
        angle: f32,
        end: &mut u32,
    ) -> f32 {
        let age = self.age;
        self.age += 1;
        if let Some(steering) = self.steering.as_mut() {
            return steering.turn(pos, angle);
        }
        let Some(seeker) = self.seeker else {
            return angle;
        };
        if age == 0 {
            return seeker.start().unwrap_or(angle);
        }
        let (turned, meets) = seeker.step(age, pos, enemy, angle);
        if meets {
            *end = (*end).min(age + 1);
        }
        turned
    }

    /// The orbit's step at `age`, moving the object on the circle or off it lunging; true when an
    /// orbit that clears completes a full turn of its own, so it forgets whom it hit before the
    /// frame's hit check (Exo_Lunar_Orbit_obj Step_0 L0x17-L0x33).
    pub(crate) fn orbit(
        &mut self,
        orbit: Orbit,
        age: u32,
        pos: &mut [f32; 2],
        angle: &mut f32,
    ) -> bool {
        let (turn, slot, facing) = orbit.step(age, self.turn, *pos, *angle);
        let slot = match orbit.ride {
            Some(ride) if age >= orbit.from && !orbit.broken(age) => ride.carry(slot, age),
            _ => slot,
        };
        self.turn = turn;
        *angle = facing;
        *pos = match (self.lunge, orbit.lunge) {
            (Some(state), Some(lunge)) if !orbit.broken(age) => {
                self.lunged(state, lunge, *pos, slot)
            }
            _ => slot,
        };
        if !orbit.clears || age < orbit.from || orbit.broken(age) {
            return false;
        }
        self.turned += orbit.speed_at(age);
        let full = self.turned >= FULL_TURN;
        if full {
            self.turned = 0.;
        }
        full
    }

    /// Exo_Lunar_Orbit_obj Step_0 L0x1b-L0x23: out along the lunge, then toward its moving place
    /// on the circle, rejoining it once near.
    fn lunged(&mut self, state: Lunging, lunge: Lunge, pos: [f32; 2], slot: [f32; 2]) -> [f32; 2] {
        let heading = match state {
            Lunging::Out { left, toward } => {
                let rest = (left > 1).then_some(Lunging::Out {
                    left: left - 1,
                    toward,
                });
                self.lunge = rest.or(Some(Lunging::Back));
                toward
            }
            Lunging::Back if (slot[0] - pos[0]).hypot(slot[1] - pos[1]) < lunge.near => {
                self.lunge = None;
                return pos;
            }
            Lunging::Back => toward(pos, slot),
        };
        moved(pos, heading, lunge.speed)
    }

    /// After its hit check at `pos`: a hit may send it lunging at the enemy; true when it turns to
    /// the enemy to rebound.
    pub(crate) fn react(
        &mut self,
        spawn: &Spawn,
        pos: [f32; 2],
        enemy: [f32; 2],
        hit: bool,
        dice: &mut Dice,
    ) -> bool {
        self.hits += u32::from(hit);
        let lunge = spawn.orbit.and_then(|orbit| orbit.lunge);
        if let Some(lunge) = lunge.filter(|_| hit && self.lunge.is_none())
            && dice.roll(lunge.chance)
        {
            self.lunge = Some(Lunging::Out {
                left: lunge.frames,
                toward: toward(pos, enemy),
            });
        }
        self.turns(spawn.rebound, pos, enemy)
    }

    /// Whether the object at `pos` turns to the enemy at `enemy` in this frame.
    pub(crate) fn turns(
        &mut self,
        rebound: Option<Rebound>,
        pos: [f32; 2],
        enemy: [f32; 2],
    ) -> bool {
        let Some(rebound) = rebound.filter(|_| self.left > 0) else {
            return false;
        };
        if self.wait > 0 {
            self.wait -= 1;
            return false;
        }
        self.wait = rebound.every.saturating_sub(1);
        let turns = (enemy[0] - pos[0]).hypot(enemy[1] - pos[1]) < rebound.reach;
        self.left -= u32::from(turns);
        turns
    }
}

/// px gone after each move, from none: `speed` px a frame, or ramping up to it as `accel` says.
pub(crate) fn gone(speed: f32, accel: Option<Accel>) -> impl Iterator<Item = f32> {
    let mut step = accel.map_or(speed, |accel| accel.from);
    std::iter::successors(Some(0.), move |gone| {
        let next = gone + step;
        if let Some(accel) = accel {
            step = accel.next(step, speed);
        }
        Some(next)
    })
}

impl Ride {
    /// `at` moved as the charge moves in `moves` moves, along the aim or steered.
    pub(crate) fn carry(self, at: [f32; 2], moves: u32) -> [f32; 2] {
        let gone = travel(self.speed, self.accel, 0.).nth(moves as usize);
        let [x, y] = gone.map_or([0., 0.], |(to, _)| to);
        [at[0] + x, at[1] + y]
    }
}

/// Leaves `spawn` where it is after every `every` moves, as a trail drops flames on its way, or,
/// with a `gap`, once it is that many px past its last drop, as skillsCharge drops Solar Dash's.
#[derive(Deserialize, Clone)]
pub(crate) struct Trail {
    pub every: u32,
    /// Moves before its first drop, when it comes sooner than `every`; 0 waits a full period.
    #[serde(default)]
    pub first: u32,
    #[serde(default)]
    pub gap: f32,
    pub spawn: Spawn,
    /// A further trail on its own schedule; when two are due on one move, the first drops.
    #[serde(default)]
    pub next: Option<Box<Trail>>,
}

impl Trail {
    /// What `mover`'s trails drop after `moves` moves, if they drop anything then.
    pub(crate) fn dropped(mover: &Spawn, moves: u32) -> Option<&Spawn> {
        let mut trails =
            std::iter::successors(mover.trail.as_deref(), |trail| trail.next.as_deref());
        trails
            .find(|trail| trail.due(mover, moves))
            .map(|trail| &trail.spawn)
    }

    /// Whether `mover` drops its spawn after `moves` moves; a trail by moves needs no walk.
    pub(crate) fn due(&self, mover: &Spawn, moves: u32) -> bool {
        if self.gap <= 0. {
            let (first, every) = self.schedule();
            return moves >= first && (moves - first).is_multiple_of(every);
        }
        self.drops(mover, moves)
            .last()
            .is_some_and(|(at, _)| at == moves)
    }

    /// The first move that drops by moves and the moves between drops.
    fn schedule(&self) -> (u32, u32) {
        let every = self.every.max(1);
        (if self.first == 0 { every } else { self.first }, every)
    }

    /// The moves, up to `until`, after which `mover` drops its spawn, and the px it has gone then.
    /// A charge checks its trail before moving and stops instead, so its last frame drops nothing.
    pub(crate) fn drops(&self, mover: &Spawn, until: u32) -> impl Iterator<Item = (u32, f32)> {
        let ((first, every), gap) = (self.schedule(), self.gap);
        let stops = mover
            .life
            .filter(|_| gap > 0.)
            .map(|life| life.saturating_sub(1));
        let until = stops.map_or(until, |stops| until.min(stops.saturating_sub(1)));
        let mut last = 0.;
        let path = gone(mover.speed, mover.accel).take(until as usize + 1);
        path.zip(0..).skip(1).filter_map(move |(at, moves)| {
            let due = if gap > 0. {
                at - last >= gap
            } else {
                moves >= first && (moves - first).is_multiple_of(every)
            };
            last = if due { at } else { last };
            due.then_some((moves, at))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Accel;

    #[test]
    fn a_ramp_compounds_its_rate_and_step_up_to_the_cap() {
        let ramp = Accel {
            from: 0.1,
            step: 0.1,
            rate: 0.05,
            steer: None,
        };
        let speeds: Vec<f32> = std::iter::successors(Some(ramp.from), |&v| Some(ramp.next(v, 4.5)))
            .take(30)
            .collect();
        for (k, speed) in speeds.iter().take(24).enumerate() {
            let expected = 2.1 * 1.05f32.powi(k as i32) - 2.;
            assert!(
                (speed - expected).abs() < 1e-4,
                "{k}: {speed} vs {expected}"
            );
        }
        assert!(speeds[24..].iter().all(|&v| v == 4.5), "{speeds:?}");
    }

    #[test]
    fn a_ramp_already_past_its_cap_keeps_its_speed() {
        let ramp = Accel {
            from: 3.,
            step: 0.1,
            rate: 0.05,
            steer: None,
        };
        assert_eq!(ramp.next(3., 2.), 3.);
    }

    #[test]
    fn a_negative_rate_slows_it_down_to_its_spawns_speed() {
        let slowing = Accel {
            from: 8.,
            step: 0.,
            rate: -0.1,
            steer: None,
        };
        assert!((slowing.next(8., 0.) - 7.2).abs() < 1e-6);
        assert_eq!(slowing.next(1., 2.), 2.);
        let gone: Vec<f32> = super::gone(0., Some(slowing)).take(4).collect();
        for (at, want) in gone.iter().zip([0., 8., 15.2, 21.68]) {
            assert!((at - want).abs() < 1e-4, "{gone:?}");
        }
    }

    fn rebounding(rebound: Option<super::Rebound>) -> super::Course {
        let spawn = crate::skill_sim::Spawn {
            rebound,
            ..crate::skill_sim::Spawn::default()
        };
        super::Course::new(&spawn, 0.)
    }

    #[test]
    fn a_rebound_looks_on_the_first_frame_then_every_period_until_its_turns_run_out() {
        use super::Rebound;
        let rebound = Some(Rebound {
            count: 3,
            every: 16,
            reach: 320.,
        });
        let mut rebounds = rebounding(rebound);
        let turns: Vec<u32> = (0..80)
            .filter(|_| rebounds.turns(rebound, [0., 0.], [116., 0.]))
            .collect();
        assert_eq!(turns, [0, 16, 32]);
    }

    #[test]
    fn a_rebound_skips_an_enemy_out_of_reach_but_waits_its_period_again() {
        use super::Rebound;
        let rebound = Some(Rebound {
            count: 2,
            every: 4,
            reach: 100.,
        });
        let mut rebounds = rebounding(rebound);
        let far: Vec<bool> = (0..5)
            .map(|_| rebounds.turns(rebound, [0., 0.], [150., 0.]))
            .collect();
        assert_eq!(far, [false; 5]);
        let near: Vec<u32> = (5..20)
            .filter(|_| rebounds.turns(rebound, [0., 0.], [50., 0.]))
            .collect();
        assert_eq!(near, [8, 12]);
        assert!(!rebounding(None).turns(None, [0., 0.], [0., 0.]));
    }

    /// A 48 px orbit at 3° a frame that lunges 40 frames at 3 px a frame and rejoins within 8 px.
    fn lunging(chance: f32) -> crate::skill_sim::Spawn {
        use crate::skill_effects::{Lunge, Orbit};
        let orbit = Orbit {
            radius: 48.,
            speed: 3.,
            count: 1,
            below: 6.,
            lunge: Some(Lunge {
                chance,
                frames: 40,
                speed: 3.,
                near: 8.,
            }),
            ..Orbit::CIRCLE
        };
        crate::skill_sim::Spawn {
            orbit: Some(orbit),
            ..crate::skill_sim::Spawn::default()
        }
    }

    fn near(a: [f32; 2], b: [f32; 2], within: f32) -> bool {
        (a[0] - b[0]).hypot(a[1] - b[1]) < within
    }

    /// Where the orbit's step at `age` takes an unturned object from `pos`.
    fn stepped(
        course: &mut super::Course,
        orbit: crate::skill_effects::Orbit,
        age: u32,
        mut pos: [f32; 2],
    ) -> [f32; 2] {
        course.orbit(orbit, age, &mut pos, &mut 0.);
        pos
    }

    #[test]
    fn a_clearing_orbit_forgets_whom_it_hit_after_each_full_turn_of_its_own() {
        use crate::skill_effects::Orbit;
        let orbit = Orbit {
            radius: 96.,
            speed: 6.,
            below: 6.,
            clears: true,
            ..Orbit::CIRCLE
        };
        let spawn = crate::skill_sim::Spawn {
            orbit: Some(orbit),
            ..crate::skill_sim::Spawn::default()
        };
        let mut course = super::Course::new(&spawn, 0.);
        let mut pos = [0., 0.];
        let forgets: Vec<u32> = (0..200)
            .filter(|&age| course.orbit(orbit, age, &mut pos, &mut 0.))
            .collect();
        assert_eq!(forgets, [59, 119, 179]);
    }

    #[test]
    fn a_lunge_flies_at_the_enemy_hit_then_back_to_its_place_on_the_circle() {
        use crate::skill_effects::Dice;
        let spawn = lunging(1.);
        let orbit = spawn.orbit.expect("an orbit");
        let mut course = super::Course::new(&spawn, 30.);
        let start = stepped(&mut course, orbit, 0, [0., 0.]);
        let (sin, cos) = 33f32.to_radians().sin_cos();
        assert!(near(start, [48. * cos, 6. - 48. * sin], 1e-3), "{start:?}");
        let enemy = [72., 0.];
        assert!(!course.react(&spawn, start, enemy, true, &mut Dice::of_use(0)));
        let (sin, cos) = super::toward(start, enemy).to_radians().sin_cos();
        let mut pos = start;
        for age in 1..=40 {
            pos = stepped(&mut course, orbit, age, pos);
        }
        assert!(near(
            pos,
            [start[0] + 120. * cos, start[1] - 120. * sin],
            1e-2
        ));
        let back = (41..2000).find(|&age| {
            let next = stepped(&mut course, orbit, age, pos);
            let rejoined = next == pos;
            pos = next;
            rejoined
        });
        let back = back.expect("the moon comes back");
        let slot = |age: u32| {
            let (sin, cos) = (30. + 3. * (age + 1) as f32).to_radians().sin_cos();
            [48. * cos, 6. - 48. * sin]
        };
        assert!(near(pos, slot(back), 8.), "{pos:?} at {back}");
        let circling = stepped(&mut course, orbit, back + 1, pos);
        assert!(near(circling, slot(back + 1), 1e-3));
    }

    #[test]
    fn only_a_lucky_hit_sends_it_lunging_and_never_while_it_lunges() {
        use crate::skill_effects::Dice;
        let mut dice = Dice::of_use(0);
        let mut second_frame = |spawn: &crate::skill_sim::Spawn, hit: bool| {
            let orbit = spawn.orbit.expect("an orbit");
            let mut course = super::Course::new(spawn, 0.);
            let pos = stepped(&mut course, orbit, 0, [0., 0.]);
            course.react(spawn, pos, [72., 0.], hit, &mut dice);
            stepped(&mut course, orbit, 1, pos)
        };
        let (sin, cos) = 6f32.to_radians().sin_cos();
        let slot = [48. * cos, 6. - 48. * sin];
        assert!(near(second_frame(&lunging(0.), true), slot, 1e-4));
        assert!(near(second_frame(&lunging(1.), false), slot, 1e-4));
        assert!(!near(second_frame(&lunging(1.), true), slot, 1.));
        let spawn = lunging(1.);
        let orbit = spawn.orbit.expect("an orbit");
        let mut course = super::Course::new(&spawn, 0.);
        let pos = stepped(&mut course, orbit, 0, [0., 0.]);
        course.react(&spawn, pos, [72., 0.], true, &mut dice);
        let out = stepped(&mut course, orbit, 1, pos);
        course.react(&spawn, out, [-72., 0.], true, &mut dice);
        let further = stepped(&mut course, orbit, 2, out);
        let first = super::toward(pos, [72., 0.]);
        assert!((super::toward(out, further) - first).abs() < 1e-3);
    }

    #[test]
    fn a_trail_leaves_its_spawn_after_every_period_of_moves() {
        use super::Trail;
        let trail = Trail {
            every: 8,
            first: 0,
            gap: 0.,
            spawn: crate::skill_sim::Spawn::default(),
            next: None,
        };
        let mover = crate::skill_sim::Spawn::default();
        let due: Vec<u32> = (0..40).filter(|&moves| trail.due(&mover, moves)).collect();
        assert_eq!(due, [8, 16, 24, 32]);
    }

    #[test]
    fn a_trail_with_a_first_drop_leaves_it_then_one_every_period() {
        use super::Trail;
        let trail = Trail {
            every: 26,
            first: 6,
            gap: 0.,
            spawn: crate::skill_sim::Spawn::default(),
            next: None,
        };
        let mover = crate::skill_sim::Spawn::default();
        let due: Vec<u32> = (0..100).filter(|&moves| trail.due(&mover, moves)).collect();
        assert_eq!(due, [6, 32, 58, 84]);
    }

    #[test]
    fn a_chained_trail_leaves_its_own_spawn_on_its_own_schedule() {
        use super::Trail;
        use crate::skill_sim::Spawn;
        let named = |object: &str| Spawn {
            object: object.into(),
            ..Spawn::default()
        };
        let once = |first: u32, object: &str, next: Option<Trail>| Trail {
            every: u32::MAX,
            first,
            gap: 0.,
            spawn: named(object),
            next: next.map(Box::new),
        };
        let mover = Spawn {
            trail: Some(Box::new(once(53, "shot", Some(once(62, "burst", None))))),
            ..Spawn::default()
        };
        let drops: Vec<(u32, &str)> = (0..200)
            .filter_map(|moves| Trail::dropped(&mover, moves).map(|s| (moves, s.object.as_str())))
            .collect();
        assert_eq!(drops, [(53, "shot"), (62, "burst")]);
    }

    /// Solar Dash's charge: 10 px on the first move, each next one 2.5 % more less 0.025, up to 13.
    fn charging(life: Option<u32>) -> crate::skill_sim::Spawn {
        crate::skill_sim::Spawn {
            speed: 13.,
            accel: Some(Accel {
                from: 10.,
                step: -0.025,
                rate: 0.025,
                steer: None,
            }),
            life,
            ..crate::skill_sim::Spawn::default()
        }
    }

    #[test]
    fn a_ramping_mover_goes_its_first_speed_then_the_ramp_up_to_its_cap() {
        let gone: Vec<f32> = super::gone(13., charging(None).accel).take(15).collect();
        assert_eq!(gone[..3], [0., 10., 20.225]);
        assert!((gone[12] - 136.16).abs() < 0.01, "{gone:?}");
        assert!(
            gone.windows(2).skip(12).all(|w| w[1] - w[0] == 13.),
            "{gone:?}"
        );
    }

    #[test]
    fn a_gap_trail_drops_once_its_mover_is_the_gap_past_its_last_drop() {
        use super::Trail;
        let trail = Trail {
            every: 0,
            first: 0,
            gap: 48.,
            spawn: crate::skill_sim::Spawn::default(),
            next: None,
        };
        let mover = charging(Some(40));
        let drops: Vec<(u32, f32)> = trail.drops(&mover, 23).collect();
        let moves: Vec<u32> = drops.iter().map(|(moves, _)| *moves).collect();
        assert_eq!(moves, [5, 10, 14, 18, 22]);
        for ((_, at), want) in drops.iter().zip([52.31, 110.83, 162.16, 214.16, 266.16]) {
            assert!((at - want).abs() < 0.01, "{drops:?}");
        }
        let due: Vec<u32> = (0..40).filter(|&moves| trail.due(&mover, moves)).collect();
        assert_eq!(due[..5], moves);
    }

    #[test]
    fn a_charge_drops_nothing_in_the_frame_it_stops() {
        use super::Trail;
        let trail = Trail {
            every: 0,
            first: 0,
            gap: 48.,
            spawn: crate::skill_sim::Spawn::default(),
            next: None,
        };
        // A mover living 15 frames stops after 14 moves, where a flame was due.
        assert!(trail.due(&charging(Some(16)), 14));
        assert!(!trail.due(&charging(Some(15)), 14));
        assert_eq!(trail.drops(&charging(Some(15)), 40).count(), 2);
    }

    #[test]
    fn a_riding_orbit_circles_a_centre_that_dashes_along_the_aim() {
        use crate::skill_effects::{Orbit, Ride};
        let ride = Ride {
            speed: 13.,
            accel: charging(None).accel,
        };
        let orbit = Orbit {
            radius: 120.,
            speed: 6.,
            below: 6.,
            ride: Some(ride),
            ..Orbit::CIRCLE
        };
        let spawn = crate::skill_sim::Spawn {
            orbit: Some(orbit),
            ..crate::skill_sim::Spawn::default()
        };
        let mut course = super::Course::new(&spawn, 0.);
        let at = |age: u32, course: &mut super::Course| stepped(course, orbit, age, [0., 0.]);
        let first = at(0, &mut course);
        let (sin, cos) = 6f32.to_radians().sin_cos();
        assert!(
            near(first, [120. * cos, 6. - 120. * sin], 1e-3),
            "{first:?}"
        );
        let later = (1..=12)
            .map(|age| at(age, &mut course))
            .last()
            .expect("a turn");
        let (sin, cos) = 78f32.to_radians().sin_cos();
        assert!(
            near(later, [136.16 + 120. * cos, 6. - 120. * sin], 0.02),
            "{later:?}"
        );
    }
}
