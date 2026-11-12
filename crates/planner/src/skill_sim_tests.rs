//! The simulation against the engine's dummy counts, and its generic features played with
//! hand-made spawns over the asteroid's objects.
use super::{FX, Fall, Placement, Shape, Spawn, Timeline, replay_use, simulate, skill};
use crate::hitbox;
use crate::skill_effects::{Frostbite, Proc, Ramp, Scatter, Stacks};
use crate::skill_motion::{Accel, Rebound, Trail};
use hsplanner_engine::calc::data;

/// The engine's base dummy hits of a skill, whatever its class; none when it counts none.
fn engine_hits(skill: &str) -> Option<f64> {
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
}

/// How a use's simulated hits miss the engine's base count; a random layout replays one case of
/// its average.
fn mismatch(got: f64, engine: Option<f64>, random: bool) -> Option<String> {
    let Some(expected) = engine else {
        return Some(format!("sim {got}, no engine count"));
    };
    let off = if random {
        (got - expected).abs() >= 0.5
    } else {
        got != expected
    };
    off.then(|| format!("sim {got}, engine {expected}"))
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
            let random = FX.skills[skill].random;
            mismatch(got, engine_hits(skill), random).map(|why| format!("{skill}: {why}"))
        })
        .collect();
    assert!(off.is_empty(), "{off:#?}");
}

#[test]
fn a_skill_the_engine_counts_no_hits_for_misses_its_parity() {
    assert!(mismatch(1., None, false).is_some());
    assert_eq!(mismatch(1., Some(1.), false), None);
    assert!(mismatch(2., Some(1.), false).is_some());
    // A random layout may miss the engine's average by under half a hit.
    assert_eq!(mismatch(2.6, Some(3.), true), None);
    assert_eq!(engine_hits("scorching_aura"), Some(1.));
}

#[test]
fn breath_of_fire_grows_up_to_its_growth_cap() {
    // Sep 16 Step_0 stops image_xscale at growthSizeMax (1.25), 10 frames in, before the fade.
    let uses = super::uses("breath_of_fire", &super::Mods::default()).expect("breath of fire");
    let timeline = &uses[0];
    let shapes = timeline.frames.iter().flatten();
    let widest = shapes
        .clone()
        .map(|shape| shape.scale[0])
        .fold(0., f32::max);
    assert!((widest - 1.25).abs() < 1e-3, "widest x scale {widest}");
    let capped = shapes.filter(|shape| (shape.scale[0] - 1.25).abs() < 1e-3);
    assert_eq!(capped.map(|shape| shape.age).min(), Some(11));
}

#[test]
#[ignore]
fn print_hits_per_projectile_count() {
    for skill in sorted_skills() {
        let counts: Vec<usize> = (1..=5)
            .map(|n| simulate(skill, n).map_or(0, |h| h.len()))
            .collect();
        eprintln!("{skill}: {counts:?} engine {:?}", engine_hits(skill));
    }
}

const ASTEROID: &str = "Exo_Asteroid_obj";
const BLAST: &str = "Explosion_Item_obj";

fn play_on(skill_id: &str, spawns: &[Spawn], n: u64) -> Timeline {
    let fx = skill(skill_id).expect("skill motion");
    let hitbox = hitbox::skill(skill_id).expect("skill hitboxes");
    replay_use(fx, hitbox, spawns, 1, n)
}

fn play(spawns: &[Spawn], n: u64) -> Timeline {
    play_on("asteroid", spawns, n)
}

fn blast() -> Spawn {
    Spawn {
        object: BLAST.into(),
        scale: Some([1.221, 1.221]),
        active_from: Some(4),
        life: Some(24),
        ..Spawn::default()
    }
}

/// An asteroid falling at the cursor, 320 px up, ending in `on_end`.
fn asteroid(on_end: Vec<Spawn>) -> Spawn {
    Spawn {
        object: ASTEROID.into(),
        target: true,
        ability: true,
        fall: Some(Fall {
            height: 320.,
            gravity: 0.25,
            ..Fall::default()
        }),
        scale: Some([0.66, 0.66]),
        on_end,
        ..Spawn::default()
    }
}

/// The first drawn shape of every instance of `object`, with the frame it was drawn in.
fn births<'a>(timeline: &'a Timeline, object: &'a str) -> Vec<(usize, &'a Shape)> {
    timeline
        .frames
        .iter()
        .enumerate()
        .flat_map(|(frame, shapes)| shapes.iter().map(move |shape| (frame, shape)))
        .filter(|(_, shape)| shape.object.name == object && shape.age == 1)
        .collect()
}

#[test]
fn a_spawn_with_no_chance_never_shows_and_a_sure_one_always_does() {
    let never = Spawn {
        chance: Some(0.),
        target: true,
        ..blast()
    };
    let sure = Spawn {
        chance: Some(1.),
        target: true,
        ..blast()
    };
    for n in 0..4 {
        assert!(play(std::slice::from_ref(&never), n).frames.is_empty());
        assert_eq!(play(std::slice::from_ref(&sure), n).hits.len(), 1);
    }
}

#[test]
fn scattered_instances_land_in_their_ring_and_move_between_uses() {
    let rain = Spawn {
        layout: (0..20)
            .map(|frame| Placement {
                frame,
                offset: [0., 0.],
            })
            .collect(),
        scatter: Some(Scatter {
            from: 32.,
            spread: 180.,
        }),
        ..blast()
    };
    let spots = |n| -> Vec<[f32; 2]> {
        births(&play(std::slice::from_ref(&rain), n), BLAST)
            .iter()
            .map(|(_, shape)| shape.pos)
            .collect()
    };
    let first = spots(0);
    assert_eq!(first.len(), 20);
    for [x, y] in &first {
        assert!((32. ..=212.).contains(&x.hypot(*y)), "{x}, {y}");
    }
    assert_ne!(first, spots(1));
}

#[test]
fn a_turned_child_flies_from_the_line_through_its_creator_offset_out() {
    let wave = Spawn {
        turn: Some(20.),
        offset: 16.,
        speed: 4.,
        ..blast()
    };
    let timeline = play(&[asteroid(vec![wave])], 0);
    let throw = 30f32.atan2(160.).to_degrees() + 20.;
    let (sin, cos) = throw.to_radians().sin_cos();
    let (_, start) = births(&timeline, BLAST)[0];
    assert!(
        (start.pos[0] - (160. + 16. * cos)).abs() < 1e-3,
        "{:?}",
        start.pos
    );
    assert!(
        (start.pos[1] - (-30. - 16. * sin)).abs() < 1e-3,
        "{:?}",
        start.pos
    );
    assert!((start.angle - throw).abs() < 1e-3);
    let next = timeline
        .frames
        .iter()
        .flatten()
        .find(|shape| shape.object.name == BLAST && shape.age == 2)
        .expect("a moving child");
    assert!((next.pos[0] - start.pos[0] - 4. * cos).abs() < 1e-3);
}

#[test]
fn a_childs_frame_delays_it_after_its_creator_makes_it() {
    let at_once = births(&play(&[asteroid(vec![blast()])], 0), BLAST)[0].0;
    let delayed = Spawn {
        frame: 5,
        ..blast()
    };
    let later = births(&play(&[asteroid(vec![delayed])], 0), BLAST)[0].0;
    assert_eq!((at_once, later), (52, 57));
}

#[test]
fn a_falling_object_drops_its_low_spawns_once_under_the_height() {
    // 320 - 0.25 * a(a+1)/2 first falls under 32 px at age 48.
    let falling = Spawn {
        low: 32.,
        on_low: vec![Spawn {
            life: Some(4),
            ..blast()
        }],
        ..asteroid(Vec::new())
    };
    let timeline = play(&[falling], 0);
    let born = births(&timeline, BLAST);
    assert_eq!(born.len(), 1);
    assert_eq!(born[0].0, 49);
    assert_eq!(born[0].1.pos, [160., -30.]);
}

#[test]
fn every_spawn_of_a_use_shares_one_frostbite_roll_and_hands_it_down() {
    let frost = Frostbite {
        chance: 0.5,
        share: 0.5,
        lasts: 60,
    };
    let cold = |on_end| Spawn {
        frostbite: Some(frost),
        ..asteroid(on_end)
    };
    let child = Spawn {
        frostbite: Some(frost),
        ..blast()
    };
    let frosts: Vec<usize> = (0..16)
        .map(|n| {
            let uses = [cold(vec![child.clone()]), cold(vec![child.clone()])];
            let timeline = play(&uses, n);
            timeline.effects.damage.iter().filter(|d| d.frost).count()
        })
        .collect();
    assert!(frosts.iter().all(|&n| n == 0 || n == 2), "{frosts:?}");
    assert!(frosts.contains(&0) && frosts.contains(&2), "{frosts:?}");
}

const WAVE: &str = "Exo_Tsunami_obj";

#[test]
fn an_accelerating_object_speeds_up_to_its_speed() {
    let wave = Spawn {
        object: WAVE.into(),
        speed: 4.5,
        accel: Some(Accel {
            from: 0.1,
            step: 0.1,
            rate: 0.05,
            steer: None,
        }),
        life: Some(40),
        ..Spawn::default()
    };
    let timeline = play_on("tsunami", &[wave], 0);
    let xs: Vec<f32> = timeline.frames.iter().flatten().map(|s| s.pos[0]).collect();
    let moves: Vec<f32> = xs.windows(2).map(|w| w[1] - w[0]).collect();
    assert!((moves[0] - 0.1).abs() < 1e-4, "{moves:?}");
    assert!((moves[1] - 0.205).abs() < 1e-4, "{moves:?}");
    assert!(
        moves[24..].iter().all(|m| (m - 4.5).abs() < 1e-4),
        "{moves:?}"
    );
}

#[test]
fn a_spawns_extra_projectiles_fan_out_with_the_volley() {
    let wave = Spawn {
        object: WAVE.into(),
        extra: 3,
        offset: 16.,
        life: Some(4),
        ..Spawn::default()
    };
    let timeline = play_on("tsunami", &[wave], 0);
    let mut angles: Vec<f32> = timeline.frames[0].iter().map(|s| s.angle).collect();
    angles.sort_by(f32::total_cmp);
    assert_eq!(angles, [-27., -9., 9., 27.]);
}

fn ramp() -> Ramp {
    Ramp {
        every: 30,
        rate: 1.,
        from: 0,
    }
}

#[test]
fn a_ramped_objects_hits_grow_while_it_lives() {
    let pool = Spawn {
        target: true,
        tick: 10.,
        life: Some(70),
        ramp: Some(ramp()),
        ..blast()
    };
    let timeline = play(&[pool], 0);
    assert!(timeline.hits.len() >= 5, "{:?}", timeline.hits);
    for (frame, damage) in timeline.hits.iter().zip(&timeline.effects.damage) {
        assert_eq!(
            damage.share,
            2f32.powi((frame / 30) as i32),
            "frame {frame}"
        );
    }
}

#[test]
fn what_a_ramped_object_makes_carries_its_growth() {
    let ripple = Spawn {
        share: Some(0.1),
        ..blast()
    };
    let pool = Spawn {
        target: true,
        life: Some(61),
        ramp: Some(ramp()),
        procs: vec![Proc {
            chance: 1.,
            spawn: ripple.clone(),
            pull: None,
            once: false,
        }],
        on_end: vec![ripple],
        ..blast()
    };
    let timeline = play(&[pool], 0);
    let shares: Vec<f32> = timeline.effects.damage.iter().map(|d| d.share).collect();
    assert_eq!(shares, [1., 0.1, 0.4]);
}

#[test]
fn a_child_made_at_the_player_starts_there_and_aims_at_the_dummy() {
    let trail = Spawn {
        at_player: true,
        aim: true,
        offset: 12.,
        speed: 5.,
        life: Some(4),
        ..blast()
    };
    let timeline = play(&[asteroid(vec![trail])], 0);
    let (_, start) = births(&timeline, BLAST)[0];
    assert_eq!((start.pos, start.angle), ([12., 0.], 0.));
}

/// A cosmic flare flying from the player at the dummy, 160 px away.
fn flare(rebound: Option<Rebound>) -> Spawn {
    Spawn {
        object: "Exo_Black_Hole_Cosmic_Flare_obj".into(),
        aim: true,
        speed: 8.,
        scale: Some([0.75, 0.75]),
        life: Some(90),
        rebound,
        ..Spawn::default()
    }
}

#[test]
fn a_rebounding_object_turns_back_through_the_dummy_and_hits_it_again() {
    // Looks at ages 0 and 16 come before the flare reaches the dummy at 160 px; the one at 32
    // turns it back through the dummy and the one at 48 out through it again.
    let rebound = Rebound {
        count: 4,
        every: 16,
        reach: 320.,
    };
    let once = play_on("black_hole", &[flare(None)], 0);
    let again = play_on("black_hole", &[flare(Some(rebound))], 0);
    assert_eq!(once.hits.len(), 1, "{:?}", once.hits);
    assert_eq!(again.hits.len(), 3, "{:?}", again.hits);
    let angles: Vec<f32> = again.frames.iter().flatten().map(|s| s.angle).collect();
    assert!(angles.contains(&180.), "never turned back");
}

#[test]
fn a_trail_leaves_its_spawn_behind_after_every_period_of_moves() {
    let drop = Spawn {
        life: Some(4),
        ..blast()
    };
    let trail = Spawn {
        ability: true,
        speed: 5.,
        life: Some(40),
        trail: Some(Box::new(Trail {
            every: 8,
            first: 0,
            gap: 0.,
            spawn: drop,
            next: None,
        })),
        ..blast()
    };
    let timeline = play(&[trail], 0);
    let born: Vec<(usize, f32)> = births(&timeline, BLAST)
        .iter()
        .map(|(frame, shape)| (*frame, shape.pos[0]))
        .collect();
    assert_eq!(born, [(0, 0.), (9, 40.), (17, 80.), (25, 120.), (33, 160.)]);
}

const FLARE: &str = "Exo_Solar_Flare_obj";

/// A solar flare thrown straight up from 24 px above the player, exploding where it ends.
fn thrown_flare() -> Spawn {
    let flare = Spawn {
        object: FLARE.into(),
        at_player: true,
        turn: Some(90.),
        offset: 24.,
        speed: 4.,
        ability: true,
        life: Some(600),
        on_end: vec![Spawn {
            life: Some(21),
            ..blast()
        }],
        ..Spawn::default()
    };
    Spawn {
        object: FLARE.into(),
        ability: true,
        life: Some(0),
        on_end: vec![flare],
        ..Spawn::default()
    }
}

#[test]
fn a_homing_flare_curves_onto_the_dummy_and_explodes_once_its_centre_meets_it() {
    let timeline = play_on("solar_flare", &[thrown_flare()], 0);
    let flight: Vec<&Shape> = timeline.frames[1..]
        .iter()
        .flatten()
        .filter(|shape| shape.object.name == FLARE)
        .collect();
    let first_move = flight.iter().find(|shape| shape.age == 2).expect("a move");
    assert!(first_move.pos[0].abs() < 1e-3 && (first_move.pos[1] + 28.).abs() < 1e-3);
    assert!(
        (first_move.angle - 88.25).abs() < 1e-3,
        "{}",
        first_move.angle
    );
    let last = flight.last().expect("a flare");
    let off = (last.pos[0] - timeline.dummy[0]).hypot(last.pos[1] - timeline.dummy[1]);
    assert!(off <= 12., "ended {off} px from the dummy");
    let blasts = births(&timeline, BLAST);
    assert_eq!(blasts.len(), 1);
    assert_eq!(blasts[0].1.pos, last.pos);
    assert_eq!(timeline.hits.len(), 1, "{:?}", timeline.hits);
}

#[test]
fn an_end_spawn_with_a_chance_rolls_it_each_time_its_creator_ends() {
    let ember = |chance: f32| Spawn {
        chance: Some(chance),
        life: Some(4),
        ..blast()
    };
    let embers = |chance: f32| {
        let bursts: Vec<Spawn> = (0..40)
            .map(|i| Spawn {
                frame: i,
                life: Some(1),
                on_end: vec![ember(chance)],
                ..blast()
            })
            .collect();
        let timeline = play(&bursts, 0);
        births(&timeline, BLAST).len() - 40
    };
    assert_eq!((embers(0.), embers(1.)), (0, 40));
    let some = embers(0.5);
    assert!((10..=30).contains(&some), "{some}");
}

#[test]
fn a_gap_trail_drops_its_spawn_where_its_ramping_mover_has_gone_the_gap() {
    let drop = Spawn {
        life: Some(4),
        ..blast()
    };
    let mover = Spawn {
        ability: true,
        speed: 13.,
        accel: Some(Accel {
            from: 10.,
            step: -0.025,
            rate: 0.025,
            steer: None,
        }),
        life: Some(20),
        trail: Some(Box::new(Trail {
            every: 0,
            first: 0,
            gap: 48.,
            spawn: drop,
            next: None,
        })),
        ..blast()
    };
    let timeline = play(&[mover], 0);
    let born: Vec<(usize, f32)> = births(&timeline, BLAST)
        .iter()
        .skip(1)
        .map(|(frame, shape)| (*frame, shape.pos[0]))
        .collect();
    let frames: Vec<usize> = born.iter().map(|(frame, _)| *frame).collect();
    assert_eq!(frames, [6, 11, 15, 19]);
    assert!((born[2].1 - 162.16).abs() < 0.01, "{born:?}");
}

#[test]
fn a_once_proc_goes_off_on_its_objects_first_hit_only() {
    let child = |share| Spawn {
        share: Some(share),
        ..blast()
    };
    let pool = Spawn {
        target: true,
        life: Some(100),
        tick: 10.,
        procs: vec![
            Proc {
                chance: 1.,
                spawn: child(0.5),
                pull: None,
                once: true,
            },
            Proc {
                chance: 1.,
                spawn: child(0.25),
                pull: None,
                once: false,
            },
        ],
        ..blast()
    };
    let timeline = play(&[pool], 0);
    let count = |share: f32| {
        let damage = timeline.effects.damage.iter();
        damage.filter(|d| d.share == share).count()
    };
    assert!(count(1.) > 3, "{:?}", timeline.hits);
    assert_eq!((count(0.5), count(0.25)), (1, count(1.)));
}

#[test]
fn a_cascade_keeps_at_most_the_cap_of_objects_alive() {
    let ember = Spawn {
        life: Some(3000),
        ..blast()
    };
    let pool = Spawn {
        target: true,
        life: Some(3000),
        tick: 1.,
        procs: vec![Proc {
            chance: 1.,
            spawn: ember,
            pull: None,
            once: false,
        }],
        ..blast()
    };
    let timeline = play(&[pool], 0);
    let most = timeline.frames.iter().map(Vec::len).max();
    assert_eq!(most, Some(super::MAX_LIVE));
}

/// A pool hitting every 11 frames whose hits pile stacks; its proc goes off on full stacks.
fn stacking_pool(stacks: Stacks, once: bool) -> Spawn {
    let pulse = Spawn {
        share: Some(0.5),
        stacks: Some(stacks),
        ..blast()
    };
    Spawn {
        target: true,
        life: Some(100),
        tick: 10.,
        procs: vec![Proc {
            chance: 1.,
            spawn: pulse,
            pull: None,
            once,
        }],
        ..blast()
    }
}

#[test]
fn a_stacking_proc_goes_off_on_each_hit_that_finds_the_stacks_full() {
    let pool = stacking_pool(Stacks { max: 2, from: 1 }, false);
    let timeline = play(&[pool], 0);
    let shares: Vec<f32> = timeline.effects.damage.iter().map(|d| d.share).collect();
    // One stack from earlier uses: the 2nd hit finds 2 stacks, then every 3rd hit after it.
    let hits = shares.iter().filter(|s| **s == 1.).count();
    assert!(hits >= 6, "{shares:?}");
    let full = (0..hits).filter(|hit| (1 + hit) % 3 == 2).count();
    assert_eq!(shares.iter().filter(|s| **s == 0.5).count(), full);
    let first = shares.iter().position(|s| *s == 0.5).expect("a pulse");
    assert_eq!(shares[..first], [1., 1.]);
    assert_eq!(timeline.effects.stacks, hits as u32);
}

#[test]
fn a_once_stacking_proc_piles_only_its_objects_first_hit() {
    let ready = play(&[stacking_pool(Stacks { max: 10, from: 10 }, true)], 0);
    let pulses = ready.effects.damage.iter().filter(|d| d.share == 0.5);
    assert_eq!((pulses.count(), ready.effects.stacks), (1, 1));
    let early = play(&[stacking_pool(Stacks { max: 10, from: 3 }, true)], 0);
    let pulses = early.effects.damage.iter().filter(|d| d.share == 0.5);
    assert_eq!((pulses.count(), early.effects.stacks), (0, 1));
}
