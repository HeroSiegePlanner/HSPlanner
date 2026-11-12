//! The moves of a skillsCharge charge, shared by the skills that move the player: the speed ramp,
//! the steering toward the cursor and the move it stops after.
use crate::skill_motion::{Accel, gone};
use crate::skill_sim::Spawn;
use crate::skill_steer::{Steer, travel};

/// skillsCharge L0x6d-L0x73: each move goes one px past the charge's speed, which then grows
/// 2.5 % a frame up to its cap.
const RAMP: f32 = 0.025;
const MOVE_EXTRA: f32 = 1.;
/// L0x29-L0x66: with no distance, before each move it turns toward the cursor by at most
/// playerEffect[23] = 15°.
const TURN_RATE: f32 = 15.;
/// A charge neither steered nor stopped leaves the view, within 20 s however slow.
const VIEW_END: f32 = 640.;
const MAX_MOVES: usize = 1200;
/// L0xc4-L0xf9: the player's 12 px circle, ceil(speed) ahead, touching the dummy's.
const PLAYER_RADIUS: f32 = 12.;
const DUMMY_RADIUS: f32 = 12.;
/// The player's collision circle sits 6 px under its origin (Mask_Player_spr origin 12, 6).
pub(super) const BELOW: f32 = 6.;

/// A charge from `start` px a frame up to `cap`: the mover's speed and its ramp, each move being
/// the last less one, times 1.025, plus one.
pub(super) fn ramp(start: f32, cap: f32) -> (f32, Accel) {
    let accel = Accel {
        from: start + MOVE_EXTRA,
        step: -RAMP,
        rate: RAMP,
        steer: None,
    };
    (cap + MOVE_EXTRA, accel)
}

/// `accel` steered toward the cursor, held on the dummy `dummy` px along the aim.
pub(super) fn steered(accel: Accel, dummy: f32) -> Accel {
    let cursor = Steer {
        rate: TURN_RATE,
        toward: [dummy, 0.],
    };
    Accel {
        steer: Some(cursor),
        ..accel
    }
}

/// What stops a charge before it leaves the view.
#[derive(Clone, Copy, Default)]
pub(super) struct Stops {
    /// Touching the dummy ends it before the move that would.
    pub contact: bool,
    /// px after which it ends, the move past them made (playerEffect[22], L0x286-L0x28a).
    pub distance: Option<f32>,
}

/// Where the mover is from where it starts and where it heads: before its first move, then after
/// each.
pub(super) fn path(mover: &Spawn) -> Vec<([f32; 2], f32)> {
    travel(mover.speed, mover.accel, 0.)
        .take(MAX_MOVES)
        .collect()
}

/// Frames the mover lives against a dummy `dummy` px out: it makes one move fewer, its last frame
/// being the stop.
pub(super) fn frames(mover: &Spawn, dummy: f32, stops: Stops) -> u32 {
    let path = path(mover);
    let gone: Vec<f32> = gone(mover.speed, mover.accel).take(MAX_MOVES).collect();
    let touches = |m: usize| {
        let ([x, y], heading) = path[m];
        let (sin, cos) = heading.to_radians().sin_cos();
        let ahead = (gone[m + 1] - gone[m]).ceil();
        let centre = [x + cos * ahead, y - sin * ahead + BELOW];
        (dummy - centre[0]).hypot(centre[1]) < PLAYER_RADIUS + DUMMY_RADIUS
    };
    let steer = mover.accel.and_then(|accel| accel.steer);
    let looped = steer
        .filter(|_| stops.distance.is_none())
        .and_then(|steer| second_pass(&path, steer.toward));
    let end = stops.distance.unwrap_or(if looped.is_some() {
        f32::INFINITY
    } else {
        VIEW_END
    });
    let stop = (0..path.len() - 1)
        .find(|&m| (stops.contact && touches(m)) || gone[m] >= end || Some(m) == looped);
    stop.unwrap_or(path.len() - 1) as u32 + 1
}

/// The charge ends at its second closest approach to the cursor: the player cancels back over it.
fn second_pass(path: &[([f32; 2], f32)], cursor: [f32; 2]) -> Option<usize> {
    let distance = |m: usize| {
        let [x, y] = path[m].0;
        (cursor[0] - x).hypot(cursor[1] - y)
    };
    (1..path.len() - 1)
        .filter(|&m| distance(m) < distance(m - 1) && distance(m) <= distance(m + 1))
        .nth(1)
}

#[cfg(test)]
mod tests {
    use super::{Stops, frames, path, ramp, steered};
    use crate::skill_sim::Spawn;

    fn mover(start: f32, cap: f32) -> Spawn {
        let (speed, accel) = ramp(start, cap);
        Spawn {
            speed,
            accel: Some(accel),
            ..Spawn::default()
        }
    }

    /// The same charge steered toward the cursor on the dummy 160 px out.
    fn steering(start: f32, cap: f32) -> Spawn {
        let charge = mover(start, cap);
        Spawn {
            accel: charge.accel.map(|accel| steered(accel, 160.)),
            ..charge
        }
    }

    #[test]
    fn a_ramp_moves_one_px_past_the_speed_and_caps_one_past_it() {
        let (speed, accel) = ramp(4., 14.);
        assert_eq!(speed, 15.);
        assert_eq!(accel.from, 5.);
        assert!((accel.next(5., speed) - 5.1).abs() < 1e-5);
    }

    #[test]
    fn a_charge_stops_short_of_the_dummy_it_would_touch() {
        // 21 moves bring it to 129.73 px; the next, 7.72 px, ceil 8, would touch the dummy.
        let contact = Stops {
            contact: true,
            distance: None,
        };
        assert_eq!(frames(&mover(4., 14.), 160., contact), 22);
        assert_eq!(frames(&steering(4., 14.), 160., contact), 22);
    }

    #[test]
    fn a_distance_ends_it_after_the_move_past_it_and_the_view_bounds_the_rest() {
        let charge = mover(4., 14.);
        let short = Stops {
            contact: false,
            distance: Some(180.),
        };
        assert_eq!(frames(&charge, 160., short), 29);
        assert_eq!(frames(&charge, 160., Stops::default()), 65);
    }

    #[test]
    fn a_steered_charge_loops_once_round_the_cursor_and_ends_back_over_it() {
        // At 13 px a frame the loop is a closed 24-frame circle: back where it passed, 2.2 px on.
        let dash = steering(9., 12.);
        assert_eq!(frames(&dash, 160., Stops::default()), 39);
        let [x, y] = path(&dash)[38].0;
        assert!((x - 162.16).abs() < 0.05 && y.abs() < 0.05, "{x} {y}");
        // Still speeding up, a wider loop brings it back 23 px off the cursor.
        let flight = steering(4., 14.);
        assert_eq!(frames(&flight, 160., Stops::default()), 50);
        let [x, y] = path(&flight)[49].0;
        assert!((x - 166.7).abs() < 0.1 && (y + 22.).abs() < 0.1, "{x} {y}");
    }
}
