//! Objects that home on their own: each Step they turn toward their target at a growing rate
//! (gml_Script_LoadHomingStep) and end once their centre stands on it (position_meeting).
use crate::skill_sim::Spawn;

/// The dummy's collision circle: a centre inside it meets the dummy.
const DUMMY_RADIUS: f32 = 12.;
/// GameMaker's epsilon when it compares reals.
const EPSILON: f32 = 1e-5;

/// The direction turns toward the target by at most the turn rate, which starts at `turn` and
/// grows by `gain` a Step; from `snap` on the object faces its target outright.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Homing {
    pub turn: f32,
    pub gain: f32,
    pub snap: f32,
    /// The direction it starts from when nothing sets one, whatever it is made with.
    pub start: Option<f32>,
}

/// Objects whose own Step homes and ends on touching: Exo_Solar_Flare_obj Create_0 (turnRate 1.75)
/// and Step_0 L0x13-L0x16 (0.025 more a Step, straight at it from 5) and L0x1f-L0x22; Fireflies'
/// Pyromancer_Fire_Ball_Fly_obj the same from 3.5 (Create_0, Step_0 L5-L7), its direction left
/// at GameMaker's 0; Scorching Harvester's soul from 2 (Create_0, Step_0 L8-L0x12).
const HOMERS: [(&str, Homing); 3] = [
    (
        "Exo_Solar_Flare_obj",
        Homing {
            turn: 1.75,
            gain: 0.025,
            snap: 5.,
            start: None,
        },
    ),
    (
        "Pyromancer_Fire_Ball_Fly_obj",
        Homing {
            turn: 3.5,
            gain: 0.025,
            snap: 5.,
            start: Some(0.),
        },
    ),
    (
        "Pyromancer_Scorching_Harvester_obj",
        Homing {
            turn: 2.,
            gain: 0.025,
            snap: 5.,
            start: None,
        },
    ),
];

/// What an instance does on its own: homes unless it circles (isOrbital skips the homing) and
/// ends when its centre meets the enemy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Seeker {
    homing: Option<Homing>,
}

impl Seeker {
    /// The seeker of an instance of `spawn`, when its object homes on its own.
    pub(crate) fn of(spawn: &Spawn) -> Option<Self> {
        let (_, homing) = HOMERS.iter().find(|(object, _)| *object == spawn.object)?;
        Some(Self {
            homing: spawn.orbit.is_none().then_some(*homing),
        })
    }

    /// The direction it starts from instead of the one it is made with.
    pub(crate) fn start(self) -> Option<f32> {
        self.homing.and_then(|homing| homing.start)
    }

    /// The Step at `age` (the first being 1) at `pos`: its angle turned toward `enemy`, and
    /// whether its centre meets it.
    pub(crate) fn step(self, age: u32, pos: [f32; 2], enemy: [f32; 2], angle: f32) -> (f32, bool) {
        let turned = self
            .homing
            .map_or(angle, |homing| homing.steer(age, pos, enemy, angle));
        let meets = (enemy[0] - pos[0]).hypot(enemy[1] - pos[1]) <= DUMMY_RADIUS;
        (turned, meets)
    }
}

impl Homing {
    /// The direction after the Step at `age`: turned toward `enemy` by at most its rate.
    pub(crate) fn steer(self, age: u32, pos: [f32; 2], enemy: [f32; 2], angle: f32) -> f32 {
        let toward = point_direction(pos, enemy);
        let rate = self.turn + self.gain * age.saturating_sub(1) as f32;
        if rate >= self.snap - EPSILON {
            return toward;
        }
        let difference = angle_difference(toward, angle);
        angle + difference.abs().min(rate).copysign(difference)
    }
}

/// GameMaker's point_direction: degrees counter-clockwise with y down.
pub(crate) fn point_direction(from: [f32; 2], to: [f32; 2]) -> f32 {
    (from[1] - to[1]).atan2(to[0] - from[0]).to_degrees()
}

/// GameMaker's angle_difference: the shortest turn from `from` to `to`, in [-180, 180).
pub(crate) fn angle_difference(to: f32, from: f32) -> f32 {
    (to - from + 180.).rem_euclid(360.) - 180.
}

#[cfg(test)]
mod tests {
    use super::{Homing, Seeker, angle_difference};
    use crate::skill_effects::Orbit;
    use crate::skill_sim::Spawn;

    const FLARE: Homing = Homing {
        turn: 1.75,
        gain: 0.025,
        snap: 5.,
        start: None,
    };

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn the_turn_is_capped_by_a_rate_that_grows_each_step() {
        let mut angle = 90.;
        let mut turns = Vec::new();
        for age in 1..=4 {
            let next = FLARE.steer(age, [0., 0.], [100., 0.], angle);
            turns.push(angle - next);
            angle = next;
        }
        for (turn, expected) in turns.iter().zip([1.75, 1.775, 1.8, 1.825]) {
            assert!(near(*turn, expected), "{turns:?}");
        }
    }

    #[test]
    fn a_turn_smaller_than_the_rate_faces_the_target_at_once_either_way() {
        assert!(near(FLARE.steer(1, [0., 0.], [100., 0.], 1.), 0.));
        assert!(near(FLARE.steer(1, [0., 0.], [100., 0.], -1.), 0.));
        assert!(near(FLARE.steer(1, [0., 0.], [100., 0.], -30.), -28.25));
        assert!(near(FLARE.steer(1, [0., 0.], [0., -100.], 350.), 351.75));
    }

    #[test]
    fn from_the_snap_rate_on_it_faces_the_target_outright() {
        let capped = FLARE.steer(130, [0., 0.], [100., 0.], 90.);
        assert!(near(capped, 90. - 4.975), "{capped}");
        assert!(near(FLARE.steer(131, [0., 0.], [100., 0.], 90.), 0.));
    }

    #[test]
    fn angle_differences_take_the_shorter_way_round() {
        assert!(near(angle_difference(10., 350.), 20.));
        assert!(near(angle_difference(350., 10.), -20.));
        assert!(near(angle_difference(0., 720.), 0.));
    }

    fn spawn(object: &str, orbit: Option<Orbit>) -> Spawn {
        Spawn {
            object: object.into(),
            orbit,
            ..Spawn::default()
        }
    }

    #[test]
    fn a_firefly_starts_heading_right_whatever_it_is_made_with_then_homes() {
        use crate::skill_motion::Course;
        let fly = spawn("Pyromancer_Fire_Ball_Fly_obj", None);
        let mut course = Course::new(&fly, 135.);
        let mut end = 600;
        assert_eq!(course.home([0., -40.], [160., 0.], 135., &mut end), 0.);
        let turned = course.home([6., -40.], [160., 0.], 0., &mut end);
        assert!(near(turned, -3.5), "{turned}");
        let flare = spawn("Exo_Solar_Flare_obj", None);
        assert_eq!(
            Course::new(&flare, 12.).home([24., 0.], [160., 0.], 12., &mut end),
            12.
        );
    }

    #[test]
    fn only_a_flare_seeks_and_an_orbiting_one_only_touches() {
        assert!(Seeker::of(&spawn("Explosion_Item_obj", None)).is_none());
        let flare = Seeker::of(&spawn("Exo_Solar_Flare_obj", None)).expect("a seeker");
        let (turned, meets) = flare.step(1, [0., 0.], [100., 0.], 90.);
        assert!(near(turned, 88.25) && !meets);
        let circling = Seeker::of(&spawn("Exo_Solar_Flare_obj", Some(Orbit::CIRCLE)));
        let circling = circling.expect("a seeker");
        assert_eq!(circling.step(1, [100., 11.], [100., 0.], 90.), (90., true));
        assert!(!circling.step(1, [100., 13.], [100., 0.], 90.).1);
    }
}
