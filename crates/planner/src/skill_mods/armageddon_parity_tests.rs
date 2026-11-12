//! The game's dummy hits for the engine's parity tests: every meteor's irandom spot enumerated at
//! its odds and every object counted apart, so the drawn preview's 256-object cap cuts nothing.
use super::{BOLT, Cast, FULL_TURN, MADNESS, SKILL, VOLLEYS, WARPED, dummy, pyro, side};
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Placement, Spawn};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

const CLASS: &str = "pyromancer";
/// Spots replayed in one use, to keep each timeline small.
const BATCH: usize = 2048;
/// px inside an object's reach where a solid round mask surely meets the dummy's 12 px disk.
const SURE_MARGIN: f32 = 3.;

/// Damage-weighted hits the game lands on the dummy in one use with `ranks`: every random spot
/// and roll at its odds and every object counted apart.
pub(super) fn uncapped(ranks: &[(&str, u32)]) -> f32 {
    with_cast(ranks, |cast| {
        let recast = cast.tree.proc(MADNESS, "").map_or(0., |(chance, _)| chance);
        let one = if cast.tree.learned(BOLT) {
            bolt(cast)
        } else if cast.tree.learned(WARPED) {
            warped(cast)
        } else {
            rain(cast)
        };
        one * (1. + recast)
    })
}

fn with_cast<T>(ranks: &[(&str, u32)], f: impl FnOnce(&Cast) -> T) -> T {
    let mods = parity::mods(SKILL, ranks);
    let tree = crate::skill_mods::Tree {
        nodes: parity::nodes(CLASS, SKILL),
        mods: &mods,
        cast: 0,
    };
    let base = &crate::skill_sim::skill(SKILL).expect("armageddon").spawn[0];
    f(&Cast::new(base, &tree))
}

/// The expectation of `f` over a meteor's Ball of Fire roll.
fn rolled(cast: &Cast, f: impl Fn(bool) -> f32) -> f32 {
    let odds = cast.ball.map_or(0., |(chance, ..)| chance);
    let plain = f(false);
    if odds > 0. {
        plain + odds * (f(true) - plain)
    } else {
        plain
    }
}

fn rain(cast: &Cast) -> f32 {
    let meteors = (VOLLEYS * cast.volley) as f32;
    meteors * rolled(cast, |lucky| landed(cast, lucky))
}

/// A meteor's explosion and Blazing Pulse's nova at its odds, over every spot it may land on.
fn landed(cast: &Cast, lucky: bool) -> f32 {
    let mut ends = cast.meteor(lucky).on_end.into_iter();
    let blast = ends.next().expect("an explosion");
    let nova = ends.next().map_or(0., |nova| {
        let odds = nova.chance.unwrap_or(1.);
        odds * spread(
            cast,
            &Spawn {
                chance: None,
                ..nova
            },
        )
    });
    spread(cast, &blast) + nova
}

/// Mean damage-weighted hits of `object` made where a meteor lands, over irandom(skillRadius) px
/// and irandom(360)° from the rain's centre: sure hits and misses by distance, the edge replayed.
fn spread(cast: &Cast, object: &Spawn) -> f32 {
    let (centre, radius) = (cast.centre(), cast.radius as u32);
    let key = format!(
        "{} {:?} {centre:?} {radius} {:?}",
        object.object, object.scale, object.share
    );
    cached(key, || {
        let (sure, reach) = bounds(object);
        let (mut sure_hits, mut edge) = (0_u32, Vec::new());
        let apart = from(centre, dummy());
        let near = (apart - reach).floor().max(0.) as u32;
        for k in near..=radius.min((apart + reach).ceil() as u32) {
            for heading in 0..=FULL_TURN {
                let [x, y] = pyro::lengthdir(k as f32, heading as f32);
                let spot = [centre[0] + x, centre[1] + y];
                let off = from(spot, dummy());
                if off <= sure {
                    sure_hits += 1;
                } else if off <= reach {
                    edge.push(spot);
                }
            }
        }
        let share = object.share.unwrap_or(1.);
        let edge_hits: f32 = edge.chunks(BATCH).map(|spots| placed(object, spots)).sum();
        let spots = (radius + 1) * (FULL_TURN + 1);
        (sure_hits as f32 * share + edge_hits) / spots as f32
    })
}

/// Damage-weighted hits of `object` made at each of `spots`.
fn placed(object: &Spawn, spots: &[[f32; 2]]) -> f32 {
    let layout = spots.iter().map(|&offset| Placement { frame: 0, offset });
    let spawn = Spawn {
        layout: layout.collect(),
        target: false,
        ..object.clone()
    };
    parity::weighted(&parity::play(SKILL, &[spawn], 1))
}

/// Distances from the dummy within which `object` surely hits it and beyond which it cannot: its
/// solid round mask at the widest it gets while it can hit.
fn bounds(object: &Spawn) -> (f32, f32) {
    let reach = reach_of(&object.object);
    let far = parity::play(SKILL, std::slice::from_ref(&away(object)), 1);
    let shapes = far.frames.iter().flatten().filter(|shape| shape.active);
    let widest = shapes.map(|shape| shape.scale[0]).fold(0., f32::max);
    let sure = (reach - SURE_MARGIN) * widest + 12. - SURE_MARGIN;
    (sure, reach * widest + 13.)
}

/// `object` alone far from the dummy.
fn away(object: &Spawn) -> Spawn {
    let far = [dummy()[0], dummy()[1] - 4000.];
    Spawn {
        layout: vec![Placement {
            frame: 0,
            offset: far,
        }],
        target: false,
        ..object.clone()
    }
}

fn reach_of(object: &str) -> f32 {
    hitbox(object).reach
}

fn hitbox(object: &str) -> &'static crate::hitbox::HitObject {
    let objects = &crate::hitbox::skill(SKILL).expect("hitboxes").objects;
    objects.iter().find(|o| o.name == object).expect(object)
}

fn warped(cast: &Cast) -> f32 {
    let meteors = (VOLLEYS * cast.volley) as f32;
    meteors * rolled(cast, |lucky| across(cast, lucky))
}

/// A horizontal meteor over its irandom(skillRadius) offsets across the aim, those whose box can
/// reach the dummy replayed with the nova they may stop in.
fn across(cast: &Cast, lucky: bool) -> f32 {
    let cursor = pyro::cursor(SKILL);
    let radius = cast.radius as u32;
    let (scale, _) = cast.ball(lucky);
    let box_half = half_height(super::HORIZONTAL) * scale + 13.;
    let target = from_line(cursor);
    let carriers: Vec<Spawn> = (0..=radius)
        .map(|k| cast.radius * 0.5 - k as f32)
        .filter(|off| (off - target).abs() <= box_half)
        .map(|off| {
            let at = side(cursor, off);
            Spawn {
                layout: vec![Placement {
                    frame: 0,
                    offset: at,
                }],
                ..super::carrier(vec![cast.horizontal(at, lucky)])
            }
        })
        .collect();
    parity::expected(SKILL, &carriers) / (radius + 1) as f32
}

/// px across the aim from the cursor point to the dummy.
fn from_line(cursor: [f32; 2]) -> f32 {
    let [x, y] = pyro::lengthdir(1., pyro::toward([0., 0.], cursor) + 90.);
    (dummy()[0] - cursor[0]) * x + (dummy()[1] - cursor[1]) * y
}

/// px a box mask reaches across its object's direction from its origin.
fn half_height(object: &str) -> f32 {
    let found = hitbox(object);
    let [_, top, _, bottom] = found.bounds.expect("a box mask");
    (found.origin[1] - top).max(bottom + 1. - found.origin[1])
}

fn bolt(cast: &Cast) -> f32 {
    if !cast.found() {
        return 0.;
    }
    rolled(cast, |lucky| {
        let meteor = Spawn {
            layout: vec![Placement {
                frame: 0,
                offset: dummy(),
            }],
            ..cast.meteor(lucky)
        };
        parity::expected(SKILL, &[meteor])
    })
}

/// Expectations already worked out, by what they depend on.
fn cached(key: String, work: impl FnOnce() -> f32) -> f32 {
    static KNOWN: LazyLock<Mutex<HashMap<String, f32>>> = LazyLock::new(Default::default);
    if let Some(known) = KNOWN.lock().expect("cache").get(&key) {
        return *known;
    }
    let value = work();
    KNOWN.lock().expect("cache").insert(key, value);
    value
}

fn from(pos: [f32; 2], to: [f32; 2]) -> f32 {
    (pos[0] - to[0]).hypot(pos[1] - to[1])
}

#[test]
fn a_spot_within_the_sure_distance_always_hits() {
    with_cast(&[("colossal_chunks", 5), ("blazing_pulse", 5)], |cast| {
        let ends = cast.meteor(true).on_end;
        for object in &ends {
            let object = Spawn {
                chance: None,
                ..object.clone()
            };
            let (sure, _) = bounds(&object);
            let spots: Vec<[f32; 2]> = (0..360)
                .map(|heading| {
                    let [x, y] = pyro::lengthdir(sure - 0.01, heading as f32);
                    [dummy()[0] + x, dummy()[1] + y]
                })
                .collect();
            let share = object.share.unwrap_or(1.);
            let hits = placed(&object, &spots);
            assert!(
                (hits / (360. * share) - 1.).abs() < 1e-4,
                "{}",
                object.object
            );
        }
    });
}
