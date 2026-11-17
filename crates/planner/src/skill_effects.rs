//! What a simulated object does besides moving and hitting: orbits, procs, swells, frostbite,
//! mana drain, buffs and pulls, and the record of them one use leaves.
use crate::skill_motion::Accel;
use crate::skill_sim::Spawn;
use serde::Deserialize;

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Orbit {
    pub radius: f32,
    /// Degrees per frame, counter-clockwise; the angle turns before the position follows.
    pub speed: f32,
    pub count: u32,
    /// px below the player at which the circle is centred.
    pub below: f32,
    /// Age of its first turn; until then it stays where it was made.
    #[serde(default)]
    pub from: u32,
    /// Points outward along its turn, as an orbiting whip does.
    #[serde(default)]
    pub turns: bool,
    /// Degrees past the angle it is made at where its circling starts.
    #[serde(default)]
    pub at: f32,
    /// Forgets whom it hit after each full turn of its own travel, instead of on a tick.
    #[serde(default)]
    pub clears: bool,
    #[serde(default)]
    pub breakout: Option<Breakout>,
    #[serde(default)]
    pub lunge: Option<Lunge>,
    /// px its circle widens each frame of its age.
    #[serde(default)]
    pub spread: f32,
    /// Degrees past its turn an object that turns with the orbit faces.
    #[serde(default)]
    pub face: f32,
    /// The circle's centre charges along the aim, as a dashing player carries its orbs.
    #[serde(default)]
    pub ride: Option<Ride>,
    /// Its turn slows down frame by frame.
    #[serde(default)]
    pub slowdown: Option<Slowdown>,
}

/// Before each turn it slows by `rate` degrees a frame while it is faster than `floor`.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Slowdown {
    pub rate: f32,
    pub floor: f32,
}

/// A charge along the aim: `speed` px a frame at most, ramping up as `accel` says.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Ride {
    pub speed: f32,
    pub accel: Option<Accel>,
}

/// From `at` frames of age it leaves the circle and flies `speed` px a frame along its last turn.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Breakout {
    pub at: u32,
    pub speed: f32,
}

/// On a hit, with `chance`, it flies `speed` px a frame toward the enemy for `frames`, then back
/// toward its place on the circle until it is within `near` px of it.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Lunge {
    pub chance: f32,
    pub frames: u32,
    pub speed: f32,
    pub near: f32,
}

impl Orbit {
    /// One unturned object circling the player from its first frame, which others build on.
    pub(crate) const CIRCLE: Orbit = Orbit {
        radius: 0.,
        speed: 0.,
        count: 1,
        below: 0.,
        from: 0,
        turns: false,
        at: 0.,
        clears: false,
        breakout: None,
        lunge: None,
        spread: 0.,
        face: 0.,
        ride: None,
        slowdown: None,
    };

    /// Degrees it turns in the frame at `age`, its slowdown taken off first.
    pub(crate) fn speed_at(self, age: u32) -> f32 {
        let Some(slow) = self
            .slowdown
            .filter(|s| s.rate > 0. && self.speed > s.floor)
        else {
            return self.speed;
        };
        let slowing = ((self.speed - slow.floor) / slow.rate).ceil() as u32;
        self.speed - slow.rate * (age + 1).min(slowing) as f32
    }

    /// The angle an orbiting object is made with: its throw when it turns with the orbit.
    pub(crate) fn facing(self, angle: f32) -> f32 {
        if self.turns { angle } else { 0. }
    }

    /// Whether at `age` it has broken out of the circle.
    pub(crate) fn broken(self, age: u32) -> bool {
        self.breakout.is_some_and(|breakout| age >= breakout.at)
    }

    /// Its turn, position and angle after the frame at `age`: GameMaker's lengthdir around the
    /// player, recomputed every frame from its start, or its flight once broken out.
    pub(crate) fn step(
        self,
        age: u32,
        turn: f32,
        pos: [f32; 2],
        angle: f32,
    ) -> (f32, [f32; 2], f32) {
        if age < self.from {
            return (turn, pos, angle);
        }
        if let Some(breakout) = self.breakout.filter(|_| self.broken(age)) {
            let (sin, cos) = turn.to_radians().sin_cos();
            let flown = [pos[0] + cos * breakout.speed, pos[1] - sin * breakout.speed];
            return (turn, flown, angle);
        }
        let turn = turn + self.speed_at(age);
        let (sin, cos) = turn.to_radians().sin_cos();
        let radius = self.radius + self.spread * age as f32;
        let pos = [cos * radius, self.below - sin * radius];
        (turn, pos, if self.turns { turn + self.face } else { angle })
    }
}

#[derive(Deserialize, Clone)]
pub(crate) struct Proc {
    /// Share of hits, 0 to 1, that create `spawn` where the object is, or at the enemy hit.
    pub chance: f32,
    pub spawn: Spawn,
    /// Pulls the enemy hit toward the object as well.
    pub pull: Option<Pull>,
    /// Goes off on the first hit of its object only, as the game zeroes an effect it has used.
    #[serde(default)]
    pub once: bool,
}

impl Proc {
    /// The procs among `procs` that the `hits`-th hit of their object rolls, `piled` stacks of
    /// the use's stacking procs coming before it.
    pub(crate) fn of_hit(procs: &[Proc], hits: u32, piled: u32) -> impl Iterator<Item = &Proc> {
        procs.iter().filter(move |proc| {
            let full = proc.spawn.stacks.is_none_or(|stacks| stacks.full(piled));
            proc.rolls_on(hits) && full
        })
    }

    /// Whether the `hits`-th hit of its object rolls it.
    fn rolls_on(&self, hits: u32) -> bool {
        !self.once || hits == 1
    }
}

/// A buff the player stacks up, one per hit that rolls the proc making this, as Scorching
/// Hellflame's: the hit that finds `max` stacks makes it instead and clears them.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Stacks {
    pub max: u32,
    /// Stacks left by the uses before this one.
    pub from: u32,
}

impl Stacks {
    /// Whether the hit that comes after `piled` stacks of this use finds them full.
    fn full(self, piled: u32) -> bool {
        (self.from + piled) % (self.max + 1) == self.max
    }
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Swell {
    /// Frames between two rolls.
    pub every: u32,
    pub chance: f32,
    /// Added to the maximum scale on a successful roll.
    pub step: f32,
    /// Added to the share of the skill's hit its hits deal on a successful roll.
    pub damage: f32,
}

/// Rolled once per use: then every hit frostbites the enemy and deals `share` of itself again.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Frostbite {
    pub chance: f32,
    pub share: f32,
    /// Frames the enemy stays frostbitten after a hit.
    pub lasts: u32,
}

/// `share` of the player's maximum mana, first after `first` frames, then every `every`.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Drain {
    pub first: u32,
    pub every: u32,
    pub share: f32,
}

impl Drain {
    fn due(self, age: u32) -> bool {
        age >= self.first && (age - self.first).is_multiple_of(self.every.max(1))
    }
}

/// A buff the player gets for `frames` when the object is created.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Buff {
    pub frames: u32,
    pub percent: f32,
}

/// The enemy is pulled toward the object for `frames` when it is farther than `beyond` px.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Pull {
    pub frames: u32,
    pub beyond: f32,
    /// Pulls toward this point once, as its object is made, instead of on its hits: the caster's
    /// own pull, for an enemy it found in reach.
    #[serde(default)]
    pub at: Option<[f32; 2]>,
}

/// Hits that grow while their object lives: every `every` frames they deal `rate` more,
/// compounding, counted on from `from` frames of growth its creator already handed down.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Ramp {
    pub every: u32,
    pub rate: f32,
    pub from: u32,
}

impl Ramp {
    /// The growth of a hit after `steps` frames of the object's life.
    pub(crate) fn growth(self, steps: u32) -> f32 {
        let every = self.every.max(1);
        let grown = (self.from + steps) / every - self.from / every;
        (1. + self.rate).powi(grown as i32)
    }
}

/// The replay's dice: one seed per use, so a build always shows the same uses.
pub(crate) struct Dice(u64);

impl Dice {
    pub(crate) const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    /// The dice of the `n`-th use.
    pub(crate) fn of_use(n: u64) -> Self {
        Self(Self::SEED ^ n.wrapping_mul(0xD6E8_FEB8_6659_FD93) | 1)
    }

    /// The dice the `n`-th cast rolls before its objects exist, apart from those they roll.
    pub(crate) fn of_cast(n: u64) -> Self {
        Self::of_use(!n)
    }

    /// xorshift64*: an even draw from 0 up to 1.
    pub(crate) fn unit(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1 << 24) as f32
    }

    /// True for a share `chance` of the rolls.
    pub(crate) fn roll(&mut self, chance: f32) -> bool {
        self.unit() < chance
    }

    /// Whether an object made as its creator ends comes up: one with a chance rolls it each time.
    pub(crate) fn comes_up(&mut self, spawn: &Spawn) -> bool {
        spawn.chance.is_none_or(|chance| self.roll(chance))
    }
}

/// Placed `from` to `from + spread` px away from where it would be, in any direction.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Scatter {
    pub from: f32,
    pub spread: f32,
}

impl Scatter {
    /// GameMaker's lengthdir(from + random(spread), random(360)).
    pub(crate) fn offset(self, dice: &mut Dice) -> [f32; 2] {
        let distance = self.from + dice.unit() * self.spread;
        let (sin, cos) = (dice.unit() * 360.).to_radians().sin_cos();
        [cos * distance, -sin * distance]
    }
}

/// Damage the dummy takes in one frame, as a share of the skill's hit.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Damage {
    pub frame: u32,
    pub share: f32,
    /// A frostbite tick rather than a hit.
    pub frost: bool,
}

/// What one use leaves besides its shapes.
#[derive(Default)]
pub(crate) struct Effects {
    pub damage: Vec<Damage>,
    /// Frames, start inclusive and end exclusive, in which the dummy is frostbitten.
    pub frostbitten: Vec<[u32; 2]>,
    /// Frames the player is buffed and the buff's percent.
    pub buffed: Option<([u32; 2], f32)>,
    /// Frames the player loses a share of maximum mana.
    pub mana: Vec<(u32, f32)>,
    /// Frames the dummy is pulled and the point it is pulled toward.
    pub pulls: Vec<([u32; 2], [f32; 2])>,
    /// Hits that rolled a stacking proc, each piling a stack.
    pub stacks: u32,
}

impl Effects {
    /// The stacks piled before this hit, the `hits`-th of an object of `spawn`, which piles
    /// one more if it rolls a stacking proc.
    pub(crate) fn pile(&mut self, spawn: &Spawn, hits: u32) -> u32 {
        let before = self.stacks;
        let procs = spawn.procs.iter();
        let stacks = procs.filter(|proc| proc.spawn.stacks.is_some() && proc.rolls_on(hits));
        self.stacks += u32::from(stacks.count() > 0);
        before
    }

    pub(crate) fn frostbitten_at(&self, frame: u32) -> bool {
        self.frostbitten
            .iter()
            .any(|[from, to]| (*from..*to).contains(&frame))
    }

    /// One hit: its damage, grown by `grown` and raised on a dummy frostbitten before this frame,
    /// so the order of objects hitting in one frame does not matter, then the frostbite it leaves.
    pub(crate) fn hit(
        &mut self,
        frame: u32,
        spawn: &Spawn,
        bonus: f32,
        grown: f32,
        frostbites: bool,
    ) {
        let base = spawn.share.unwrap_or(1.) + bonus;
        let earlier = self
            .frostbitten
            .iter()
            .any(|[from, to]| *from < frame && frame < *to);
        let share = if earlier {
            base * grown * (1. + spawn.vs_frostbitten)
        } else {
            base * grown
        };
        self.damage.push(Damage {
            frame,
            share,
            frost: false,
        });
        let Some(frostbite) = spawn.frostbite.filter(|_| frostbites) else {
            return;
        };
        self.damage.push(Damage {
            frame,
            share: base * frostbite.share,
            frost: true,
        });
        let until = frame + frostbite.lasts;
        match self.frostbitten.last_mut() {
            Some([_, to]) if *to >= frame => *to = (*to).max(until),
            _ => self.frostbitten.push([frame, until]),
        }
    }

    /// A hit's pull toward `toward`; a pull toward a set point goes off only as its object is made.
    pub(crate) fn pull(&mut self, frame: u32, pull: Pull, toward: [f32; 2], dummy: [f32; 2]) {
        let far = (dummy[0] - toward[0]).hypot(dummy[1] - toward[1]) > pull.beyond;
        if pull.at.is_none() && far {
            self.pulled(frame, pull.frames, toward);
        }
    }

    /// A pull toward `toward`; one toward the same point while the last still lasts extends it.
    fn pulled(&mut self, frame: u32, frames: u32, toward: [f32; 2]) {
        let until = frame + frames;
        match self.pulls.last_mut() {
            Some(([_, to], point)) if *point == toward && *to >= frame => *to = (*to).max(until),
            _ => self.pulls.push(([frame, until], toward)),
        }
    }

    /// An object made: the buff it gives the player and its pull toward a set point.
    pub(crate) fn created(&mut self, frame: u32, spawn: &Spawn) {
        if let Some(buff) = spawn.buff {
            self.buffed
                .get_or_insert(([frame, frame + buff.frames], buff.percent));
        }
        if let Some(pull) = spawn.pull
            && let Some(toward) = pull.at
        {
            self.pulled(frame, pull.frames, toward);
        }
    }

    pub(crate) fn alive(&mut self, frame: u32, spawn: &Spawn, age: u32) {
        if let Some(drain) = spawn.drain.filter(|drain| drain.due(age)) {
            self.mana.push((frame, drain.share));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Dice, Effects, Frostbite};
    use crate::skill_sim::Spawn;

    #[test]
    fn the_dice_roll_their_chance() {
        let mut dice = Dice::of_use(0);
        assert!((0..1000).all(|_| dice.roll(1.)));
        assert!((0..1000).all(|_| !dice.roll(0.)));
        let share = (0..10_000).filter(|_| dice.roll(0.25)).count() as f32 / 10_000.;
        assert!((share - 0.25).abs() < 0.02, "{share}");
    }

    #[test]
    fn a_roll_is_a_unit_draw_under_its_chance() {
        let (mut draws, mut rolls) = (Dice::of_use(3), Dice::of_use(3));
        for _ in 0..1000 {
            let unit = draws.unit();
            assert!((0. ..1.).contains(&unit), "{unit}");
            assert_eq!(rolls.roll(0.3), unit < 0.3);
        }
    }

    #[test]
    fn a_scatter_lands_inside_its_ring_in_every_direction() {
        use super::Scatter;
        let ring = Scatter {
            from: 32.,
            spread: 180.,
        };
        let mut dice = Dice::of_use(0);
        let offsets: Vec<[f32; 2]> = (0..500).map(|_| ring.offset(&mut dice)).collect();
        for [x, y] in &offsets {
            let distance = x.hypot(*y);
            assert!((32. ..=212.).contains(&distance), "{distance}");
        }
        for quadrant in [[1., 1.], [1., -1.], [-1., 1.], [-1., -1.]] {
            assert!(
                offsets
                    .iter()
                    .any(|[x, y]| x * quadrant[0] > 0. && y * quadrant[1] > 0.)
            );
        }
    }

    #[test]
    fn a_frostbite_from_the_same_frame_gives_no_bonus_yet() {
        let spawn = Spawn {
            vs_frostbitten: 0.5,
            frostbite: Some(Frostbite {
                chance: 1.,
                share: 0.2,
                lasts: 60,
            }),
            ..Spawn::default()
        };
        let mut effects = Effects::default();
        effects.hit(10, &spawn, 0., 1., true);
        effects.hit(10, &spawn, 0., 1., true);
        let hits: Vec<f32> = effects
            .damage
            .iter()
            .filter(|d| !d.frost)
            .map(|d| d.share)
            .collect();
        assert_eq!(hits, [1., 1.]);
    }

    #[test]
    fn pulls_toward_one_point_merge_and_near_enemies_stay() {
        use super::Pull;
        let pull = Pull {
            frames: 60,
            beyond: 0.,
            at: None,
        };
        let mut effects = Effects::default();
        effects.pull(10, pull, [160., -30.], [160., 0.]);
        effects.pull(16, pull, [160., -30.], [160., 0.]);
        assert_eq!(effects.pulls, [([10, 76], [160., -30.])]);
        let far = Pull {
            frames: 60,
            beyond: 64.,
            at: None,
        };
        effects.pull(20, far, [100., -30.], [160., 0.]);
        assert_eq!(effects.pulls.len(), 2);
        effects.pull(30, far, [160., -30.], [160., 0.]);
        assert_eq!(effects.pulls.len(), 2);
    }

    #[test]
    fn a_pull_toward_a_set_point_goes_off_as_its_object_is_made_never_on_its_hits() {
        use super::Pull;
        let vacuum = Pull {
            frames: 60,
            beyond: 0.,
            at: Some([48., 0.]),
        };
        let spawn = Spawn {
            pull: Some(vacuum),
            ..Spawn::default()
        };
        let mut effects = Effects::default();
        effects.pull(5, vacuum, [0., 0.], [100., 0.]);
        assert!(effects.pulls.is_empty());
        effects.created(0, &spawn);
        assert_eq!(effects.pulls, [([0, 60], [48., 0.])]);
        let mut plain = Effects::default();
        plain.created(0, &Spawn::default());
        assert!(plain.pulls.is_empty());
    }

    #[test]
    fn a_ramp_grows_every_period_counted_on_from_its_creator() {
        use super::Ramp;
        let ramp = Ramp {
            every: 30,
            rate: 0.5,
            from: 0,
        };
        let growth: Vec<f32> = [0, 29, 30, 59, 60, 95].map(|s| ramp.growth(s)).into();
        assert_eq!(growth, [1., 1., 1.5, 1.5, 2.25, 3.375]);
        let handed = Ramp { from: 93, ..ramp };
        assert_eq!(
            [0, 26, 27, 57].map(|s| handed.growth(s)),
            [1., 1., 1.5, 2.25]
        );
    }

    #[test]
    fn a_grown_hit_leaves_its_frostbite_ungrown() {
        let spawn = Spawn {
            frostbite: Some(Frostbite {
                chance: 1.,
                share: 0.2,
                lasts: 60,
            }),
            ..Spawn::default()
        };
        let mut effects = Effects::default();
        effects.hit(10, &spawn, 0., 2., true);
        let shares: Vec<(f32, bool)> = effects.damage.iter().map(|d| (d.share, d.frost)).collect();
        assert_eq!(shares, [(2., false), (0.2, true)]);
    }

    #[test]
    fn an_orbit_waits_for_its_start_then_points_its_object_along_the_circle() {
        use super::Orbit;
        let orbit = Orbit {
            radius: 8.,
            speed: 12.,
            count: 3,
            below: 6.,
            from: 4,
            turns: true,
            ..Orbit::CIRCLE
        };
        let made = (90., [0., -10.], 90.);
        assert_eq!(orbit.step(3, made.0, made.1, made.2), made);
        let (turn, pos, angle) = orbit.step(4, made.0, made.1, made.2);
        assert_eq!((turn, angle), (102., 102.));
        let (sin, cos) = 102f32.to_radians().sin_cos();
        assert!((pos[0] - 8. * cos).abs() < 1e-5, "{pos:?}");
        assert!((pos[1] - (6. - 8. * sin)).abs() < 1e-5, "{pos:?}");
        assert_eq!(orbit.facing(90.), 90.);
    }

    #[test]
    fn an_unturned_orbit_circles_from_its_first_frame_and_keeps_its_object_unturned() {
        use super::Orbit;
        let orbit = Orbit {
            radius: 128.,
            speed: 4.,
            count: 1,
            below: 6.,
            ..Orbit::CIRCLE
        };
        let (turn, pos, angle) = orbit.step(0, 0., [0., 0.], 0.);
        assert_eq!((turn, angle), (4., 0.));
        assert!(
            (pos[0] - 128. * 4f32.to_radians().cos()).abs() < 1e-4,
            "{pos:?}"
        );
        assert_eq!(orbit.facing(120.), 0.);
    }

    #[test]
    fn a_spreading_orbit_widens_each_frame_and_faces_past_its_turn() {
        use super::Orbit;
        let orbit = Orbit {
            radius: 24.,
            speed: 4.,
            count: 1,
            below: 6.,
            from: 1,
            turns: true,
            spread: 1.5,
            face: 90.,
            ..Orbit::CIRCLE
        };
        let made = (30., [1., 2.], 30.);
        assert_eq!(orbit.step(0, made.0, made.1, made.2), made);
        let (turn, pos, angle) = orbit.step(2, 30., [0., 0.], 30.);
        assert_eq!((turn, angle), (34., 124.));
        let (sin, cos) = 34f32.to_radians().sin_cos();
        assert!((pos[0] - 27. * cos).abs() < 1e-4, "{pos:?}");
        assert!((pos[1] - (6. - 27. * sin)).abs() < 1e-4, "{pos:?}");
    }

    #[test]
    fn a_broken_out_orbit_flies_straight_along_its_last_turn() {
        use super::{Breakout, Orbit};
        let orbit = Orbit {
            radius: 48.,
            speed: 3.,
            count: 1,
            below: 6.,
            breakout: Some(Breakout { at: 10, speed: 6. }),
            ..Orbit::CIRCLE
        };
        let (turn, circling, _) = orbit.step(9, 87., [0., 0.], 0.);
        assert_eq!(turn, 90.);
        assert!(circling[0].abs() < 1e-4 && (circling[1] + 42.).abs() < 1e-4);
        assert!(!orbit.broken(9) && orbit.broken(10));
        let (turn, flown, angle) = orbit.step(10, turn, circling, 0.);
        assert_eq!((turn, angle), (90., 0.));
        assert!(
            flown[0].abs() < 1e-4 && (flown[1] + 48.).abs() < 1e-4,
            "{flown:?}"
        );
    }

    #[test]
    fn a_slowing_orbit_loses_its_rate_each_frame_until_its_floor() {
        use super::{Orbit, Slowdown};
        let orbit = Orbit {
            radius: 8.,
            speed: 2.05,
            slowdown: Some(Slowdown {
                rate: 0.015,
                floor: 2.,
            }),
            ..Orbit::CIRCLE
        };
        let speeds: Vec<f32> = (0..6).map(|age| orbit.speed_at(age)).collect();
        for (speed, expected) in speeds.iter().zip([2.035, 2.02, 2.005, 1.99, 1.99, 1.99]) {
            assert!((speed - expected).abs() < 1e-5, "{speeds:?}");
        }
        let (turn, _, _) = orbit.step(0, 30., [0., 0.], 0.);
        assert!((turn - 32.035).abs() < 1e-4, "{turn}");
        assert_eq!(Orbit::CIRCLE.speed_at(500), 0.);
    }

    #[test]
    fn a_drain_comes_first_after_its_delay_then_every_period() {
        use super::Drain;
        let drain = Drain {
            first: 30,
            every: 60,
            share: 0.03,
        };
        let due: Vec<u32> = (0..200).filter(|&age| drain.due(age)).collect();
        assert_eq!(due, [30, 90, 150]);
    }

    #[test]
    fn a_frostbitten_dummy_takes_the_bonus_from_the_next_hit_on() {
        let spawn = Spawn {
            vs_frostbitten: 0.5,
            frostbite: Some(Frostbite {
                chance: 1.,
                share: 0.2,
                lasts: 60,
            }),
            ..Spawn::default()
        };
        let mut effects = Effects::default();
        effects.hit(10, &spawn, 0., 1., true);
        effects.hit(30, &spawn, 0., 1., true);
        let shares: Vec<(f32, bool)> = effects.damage.iter().map(|d| (d.share, d.frost)).collect();
        assert_eq!(
            shares,
            [(1., false), (0.2, true), (1.5, false), (0.2, true)]
        );
        assert_eq!(effects.frostbitten, [[10, 90]]);
    }
}
