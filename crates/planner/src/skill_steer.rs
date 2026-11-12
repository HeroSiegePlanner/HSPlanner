//! Objects turned toward a point before each move by at most a rate, as skillsCharge steers a
//! charge with no distance toward the cursor (L0x29-L0x66).
use crate::skill_homing::{angle_difference, point_direction};
use crate::skill_motion::Accel;
use crate::skill_sim::Spawn;
use serde::Deserialize;

const FULL_TURN: f32 = 360.;

/// Turns toward the point `toward` px from where it starts by at most `rate` degrees a frame.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Steer {
    pub rate: f32,
    pub toward: [f32; 2],
}

impl Steer {
    /// L0x59-L0x63: the heading at `pos` of an object that started at `start`, turned by
    /// sign(d) × min(|d|, rate), d = angle_difference(direction to the point, heading), kept
    /// within a turn.
    pub(crate) fn turn(self, start: [f32; 2], pos: [f32; 2], heading: f32) -> f32 {
        let point = [start[0] + self.toward[0], start[1] + self.toward[1]];
        let difference = angle_difference(point_direction(pos, point), heading);
        let turned = heading + difference.abs().min(self.rate).copysign(difference);
        if turned > FULL_TURN {
            turned - FULL_TURN
        } else if turned < 0. {
            turned + FULL_TURN
        } else {
            turned
        }
    }
}

/// An instance's steering: its turn and where it started, taken at its first Step.
#[derive(Clone, Copy)]
pub(crate) struct Steering {
    steer: Steer,
    start: Option<[f32; 2]>,
}

impl Steering {
    /// The steering of an instance of `spawn`, when its ramp steers.
    pub(crate) fn of(spawn: &Spawn) -> Option<Self> {
        let steer = spawn.accel?.steer?;
        Some(Self { steer, start: None })
    }

    /// Its heading for the next move from `pos`.
    pub(crate) fn turn(&mut self, pos: [f32; 2], heading: f32) -> f32 {
        let start = *self.start.get_or_insert(pos);
        self.steer.turn(start, pos, heading)
    }
}

/// Where an object made at (0, 0) heading `angle` is, and where it heads, before its first move
/// and after each: `speed` px a frame, ramping and steering as `accel` says.
pub(crate) fn travel(
    speed: f32,
    accel: Option<Accel>,
    angle: f32,
) -> impl Iterator<Item = ([f32; 2], f32)> {
    let steer = accel.and_then(|accel| accel.steer);
    let start = [0., 0.];
    let turn =
        move |pos: [f32; 2], heading: f32| steer.map_or(heading, |s| s.turn(start, pos, heading));
    let mut step = accel.map_or(speed, |accel| accel.from);
    std::iter::successors(Some((start, turn(start, angle))), move |&(pos, heading)| {
        let (sin, cos) = heading.to_radians().sin_cos();
        let next = [pos[0] + cos * step, pos[1] - sin * step];
        if let Some(accel) = accel {
            step = accel.next(step, speed);
        }
        Some((next, turn(next, heading)))
    })
}

#[cfg(test)]
mod tests {
    use super::{Steer, travel};
    use crate::skill_motion::Accel;
    use crate::skill_sim::{Placement, Spawn, replay_use};

    const CURSOR: Steer = Steer {
        rate: 15.,
        toward: [160., 0.],
    };

    fn near(a: f32, b: f32, within: f32) -> bool {
        (a - b).abs() < within
    }

    /// Solar Dash's charge: 10 px, each next move 2.5 % more less 0.025, up to 13, steered.
    fn charge() -> Accel {
        Accel {
            from: 10.,
            step: -0.025,
            rate: 0.025,
            steer: Some(CURSOR),
        }
    }

    #[test]
    fn a_steer_turns_toward_the_point_by_at_most_its_rate() {
        let start = [0., 0.];
        assert!(near(CURSOR.turn(start, start, 90.), 75., 1e-4));
        assert!(near(CURSOR.turn(start, start, 10.), 0., 1e-4));
        assert!(near(CURSOR.turn(start, [0., 50.], 0.), 15., 1e-4));
    }

    #[test]
    fn a_point_straight_behind_turns_it_clockwise_kept_within_a_turn() {
        // angle_difference(180, 0) is -180: GameMaker's sign sends it the negative way.
        assert!(near(CURSOR.turn([0., 0.], [170., 0.], 0.), 345., 1e-4));
        assert!(near(CURSOR.turn([0., 0.], [170., 0.], 355.), 340., 1e-4));
    }

    #[test]
    fn a_capped_charge_steered_past_its_point_circles_back_through_it() {
        let path: Vec<([f32; 2], f32)> = travel(13., Some(charge()), 0.).take(40).collect();
        let [x, y] = path[14].0;
        assert!(near(x, 162.16, 0.01) && y == 0., "{:?}", path[14]);
        // 24 turns of 15° on a 13 px step: a circle 99.3 px across, back where it passed.
        let widest = path.iter().map(|(at, _)| at[1]).fold(0., f32::max);
        assert!(near(widest, 98.745, 0.01), "{widest}");
        let [x, y] = path[38].0;
        assert!(near(x, 162.16, 0.05) && near(y, 0., 0.05), "{:?}", path[38]);
        assert!(near(path[37].1, 0., 1e-3), "{}", path[37].1);
    }

    #[test]
    fn without_a_steer_it_travels_straight_along_its_angle() {
        let straight = Accel {
            steer: None,
            ..charge()
        };
        let path: Vec<([f32; 2], f32)> = travel(13., Some(straight), 0.).take(4).collect();
        let xs: Vec<f32> = path.iter().map(|(at, _)| at[0]).collect();
        assert_eq!(xs, [0., 10., 20.225, 30.680626]);
        assert!(
            path.iter()
                .all(|(at, heading)| at[1] == 0. && *heading == 0.)
        );
    }

    #[test]
    fn a_steered_instance_follows_its_travel_in_the_replay() {
        let mover = Spawn {
            object: "Draw_Player_Buff_obj".into(),
            ability: true,
            layout: vec![Placement {
                frame: 0,
                offset: [0., 6.],
            }],
            speed: 13.,
            accel: Some(charge()),
            life: Some(39),
            ..Spawn::default()
        };
        let fx = crate::skill_sim::skill("solar_dash").expect("solar dash");
        let hitbox = crate::hitbox::skill("solar_dash").expect("solar dash hitboxes");
        let timeline = replay_use(fx, hitbox, &[mover], 1, 0);
        let shapes = timeline.frames.iter().flatten();
        let moved: Vec<[f32; 2]> = shapes.map(|shape| shape.pos).collect();
        // Up to its return over the point, where a second loop would start on a 180° tie.
        let path = travel(13., Some(charge()), 0.).take(39);
        assert_eq!(moved.len(), 39);
        for (at, (want, _)) in moved.iter().zip(path) {
            assert!(
                near(at[0], want[0], 1e-3) && near(at[1], 6. + want[1], 1e-3),
                "{at:?} {want:?}"
            );
        }
    }

    #[test]
    fn an_orbit_riding_a_steered_charge_follows_its_loop() {
        let ride = crate::skill_effects::Ride {
            speed: 13.,
            accel: Some(charge()),
        };
        let carried = ride.carry([0., 0.], 26);
        let (want, _) = travel(13., Some(charge()), 0.).nth(26).expect("a move");
        assert!(near(carried[0], want[0], 1e-4) && near(carried[1], want[1], 1e-4));
    }
}
