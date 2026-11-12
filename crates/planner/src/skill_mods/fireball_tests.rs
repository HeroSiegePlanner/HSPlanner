use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use, uses};
use std::collections::HashMap;

const BALL: &str = "Pyromancer_Fire_Ball_obj";
const BLAST: &str = "Explosion_Item_obj";
const SPLIT: &str = "Pyromancer_Fire_Ball_Split_obj";
const ORBITAL: &str = "Pyromancer_Fire_Ball_Orbital_obj";
const FLY: &str = "Pyromancer_Fire_Ball_Fly_obj";
const PATCH: &str = "Pyromancer_Blazing_Trail_obj";
const METEOR: &str = "Pyromancer_Armageddon_obj";
const DUMMY: [f32; 2] = [160., 0.];

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("fireball:{node}"), *rank))
        .collect();
    Mods::of("fireball", &ranks)
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill("fireball").expect("fireball").spawn;
    crate::skill_mods::apply_cast("fireball", base, &mods(ranks), cast).into_owned()
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play(spawns: &[Spawn]) -> Timeline {
    let fx = crate::skill_sim::skill("fireball").expect("fireball");
    let hitbox = crate::hitbox::skill("fireball").expect("fireball hitboxes");
    replay_use(fx, hitbox, spawns, 1, 0)
}

/// The first cast among the first `tries` whose spawns pass `lucky`.
fn cast_where(ranks: &[(&str, u32)], tries: u64, lucky: impl Fn(&[Spawn]) -> bool) -> u64 {
    (0..tries)
        .find(|&n| lucky(&spawns_of(ranks, n)))
        .expect("a cast that rolls it")
}

/// The first drawn shape of every instance of `object`, with the frame it is drawn in.
fn births<'a>(timeline: &'a Timeline, object: &'a str) -> Vec<(usize, &'a Shape)> {
    timeline
        .frames
        .iter()
        .enumerate()
        .flat_map(|(frame, shapes)| shapes.iter().map(move |shape| (frame, shape)))
        .filter(|(_, shape)| shape.object.name == object && shape.age == 1)
        .collect()
}

fn shapes<'a>(timeline: &'a Timeline, object: &'a str) -> impl Iterator<Item = &'a Shape> {
    let all = timeline.frames.iter().flatten();
    all.filter(move |shape| shape.object.name == object)
}

fn shares(timeline: &Timeline) -> Vec<f32> {
    timeline.effects.damage.iter().map(|d| d.share).collect()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn from(pos: [f32; 2], to: [f32; 2]) -> f32 {
    (pos[0] - to[0]).hypot(pos[1] - to[1])
}

/// The explosion the ball ends in.
fn blast(spawns: &[Spawn]) -> &Spawn {
    &spawns[0].on_end[0]
}

#[test]
fn the_base_cast_keeps_one_ball_exploding_once_on_the_dummy() {
    let base = spawns(&[]);
    assert_eq!(base.len(), 1);
    assert_eq!((base[0].object.as_str(), base[0].speed), (BALL, 10.));
    assert_eq!(base[0].on_end.len(), 1);
    assert_eq!(blast(&base).scale, Some([1.6, 1.6]));
    assert!(blast(&base).procs.is_empty());
    let timeline = play(&base);
    assert_eq!(shares(&timeline), [1.]);
    assert_eq!(uses("fireball", &mods(&[])).expect("uses").len(), 8);
}

#[test]
fn multicast_may_send_a_second_ball_12_frames_after_the_first() {
    let ranks = [("multicast", 5)];
    let counts: Vec<usize> = (0..40).map(|n| spawns_of(&ranks, n).len()).collect();
    assert!(counts.iter().all(|n| [1, 2].contains(n)), "{counts:?}");
    assert!(counts.contains(&1));
    let lucky = spawns_of(&ranks, cast_where(&ranks, 40, |s| s.len() == 2));
    assert_eq!((lucky[0].frame, lucky[1].frame), (0, 12));
    let timeline = play(&lucky);
    let blasts: Vec<usize> = births(&timeline, BLAST).iter().map(|(f, _)| *f).collect();
    assert_eq!(blasts.len(), 2);
    assert_eq!(blasts[1] - blasts[0], 12);
    assert_eq!(timeline.hits.len(), 2);
}

#[test]
fn a_multicast_of_damage_objects_starts_with_the_first_volley() {
    let ranks = [("multicast", 5), ("orbital_fire", 1)];
    let lucky = spawns_of(&ranks, cast_where(&ranks, 40, |s| s.len() == 2));
    assert!(lucky.iter().all(|s| s.frame == 0 && s.object == ORBITAL));
}

#[test]
fn ignition_radius_widens_the_explosion_by_a_fifth_of_its_size_a_rank() {
    let wide = spawns(&[("ignition_radius", 5)]);
    assert_eq!(blast(&wide).scale, Some([2.4, 2.4]));
    let timeline = play(&wide);
    let widest = shapes(&timeline, BLAST)
        .map(|s| s.scale[0])
        .fold(0., f32::max);
    assert!(near(widest, 2.4), "{widest}");
    assert_eq!(timeline.hits.len(), 1);
}

#[test]
fn plain_damage_and_ailment_nodes_leave_the_cast_as_it_is() {
    let base = play(&spawns(&[]));
    for node in ["inferno_ball", "gas_fumes", "soulburn", "burn_baby_burn"] {
        let cast = play(&spawns(&[(node, 5)]));
        assert_eq!(shares(&cast), shares(&base), "{node}");
        assert_eq!(cast.hits, base.hits, "{node}");
    }
}

#[test]
fn every_fireball_node_keeps_its_damage_on_the_hit() {
    assert!(crate::skill_mods::parity_tests::owned("pyromancer", "fireball").is_empty());
}

#[test]
fn searing_speed_hurries_the_ball_and_its_explosion() {
    let fast = spawns(&[("searing_speed", 5)]);
    assert!(near(fast[0].speed, 17.5), "{}", fast[0].speed);
    let base = play(&spawns(&[]));
    let quick = play(&fast);
    assert!(
        quick.hits[0] < base.hits[0],
        "{:?} {:?}",
        quick.hits,
        base.hits
    );
}

#[test]
fn paralyzing_impact_may_add_its_damage_to_the_casts_subtree() {
    let ranks = [("paralyzing_impact", 5), ("inferno_ball", 5)];
    let rolled: Vec<f32> = (0..60)
        .map(|n| blast(&spawns_of(&ranks, n)).share.expect("a share"))
        .collect();
    assert!(
        rolled.iter().all(|s| near(*s, 1.) || near(*s, 2.)),
        "{rolled:?}"
    );
    assert!(rolled.iter().any(|s| near(*s, 1.)) && rolled.iter().any(|s| near(*s, 2.)));
}

#[test]
fn critical_burn_may_double_the_casts_damage() {
    let ranks = [("critical_burn", 5)];
    let rolled: Vec<f32> = (0..40)
        .map(|n| blast(&spawns_of(&ranks, n)).share.expect("a share"))
        .collect();
    assert!(
        rolled.iter().all(|s| near(*s, 1.) || near(*s, 2.)),
        "{rolled:?}"
    );
    assert!(rolled.iter().any(|s| near(*s, 2.)));
}

#[test]
fn mark_of_doom_drops_an_armageddon_meteor_on_the_enemy_its_explosion_hits() {
    let ranks = [("mark_of_doom", 5)];
    let doomed = |s: &[Spawn]| !blast(s).procs.is_empty();
    let lucky = spawns_of(&ranks, cast_where(&ranks, 400, doomed));
    assert!(near(blast(&lucky).share.expect("a share"), 3.5));
    assert!(blast(&lucky).procs.iter().all(|proc| proc.once));
    let timeline = play(&lucky);
    let meteors = births(&timeline, METEOR);
    assert_eq!(meteors.len(), 1);
    assert!(from(meteors[0].1.pos, DUMMY) < 1., "{:?}", meteors[0].1.pos);
    let blasts = births(&timeline, BLAST);
    assert_eq!(blasts.len(), 2);
    assert!(
        blasts[1].0 > blasts[0].0 + 50,
        "{} {}",
        blasts[0].0,
        blasts[1].0
    );
    assert!(from(blasts[1].1.pos, DUMMY) < 1.);
    assert_eq!(shares(&timeline), [3.5, 3.5]);
}

#[test]
fn fireflies_ring_the_player_fly_right_then_home_and_explode_on_the_enemy() {
    let ranks = [("fireflies", 3)];
    let swarm = |s: &[Spawn]| !blast(s).procs.is_empty();
    let lucky = spawns_of(&ranks, cast_where(&ranks, 40, swarm));
    assert_eq!(blast(&lucky).procs.len(), 12);
    let timeline = play(&lucky);
    let flies = births(&timeline, FLY);
    assert_eq!(flies.len(), 12);
    let first = flies[0].0;
    for (frame, fly) in &flies {
        assert_eq!(*frame, first);
        assert!(near(from(fly.pos, [0., 0.]), 8.), "{:?}", fly.pos);
    }
    let moved: Vec<&Shape> = shapes(&timeline, FLY).filter(|s| s.age == 2).collect();
    assert!(
        moved.iter().all(|s| s.angle.abs() <= 3.6),
        "{:?}",
        moved.len()
    );
    let blasts = births(&timeline, BLAST);
    assert_eq!(blasts.len(), 13);
    assert!(blasts[1..].iter().all(|(_, b)| from(b.pos, DUMMY) <= 12.));
    assert_eq!(shares(&timeline), [2.8; 13]);
}

#[test]
fn a_meteor_inherits_the_fireflies_and_the_fireflies_leave_the_meteor_out() {
    let ranks = [("mark_of_doom", 5), ("fireflies", 3)];
    let both = |s: &[Spawn]| blast(s).procs.len() == 13;
    let lucky = spawns_of(&ranks, cast_where(&ranks, 2000, both));
    let timeline = play(&lucky);
    assert_eq!(births(&timeline, METEOR).len(), 1);
    assert_eq!(births(&timeline, FLY).len(), 24);
    assert_eq!(timeline.hits.len(), 1 + 12 + 1 + 12);
}

#[test]
fn splitfire_fans_three_splits_a_rank_out_of_the_ball_each_hitting_once() {
    let few = spawns(&[("splitfire", 1)]);
    let turns: Vec<f32> = few[0].on_end[1..].iter().filter_map(|s| s.turn).collect();
    assert_eq!(turns, [-7., 0., 7.]);
    let ranks = [("splitfire", 3)];
    let many = spawns(&ranks);
    let splits = &many[0].on_end[1..];
    assert_eq!(splits.len(), 9);
    assert_eq!((splits[0].turn, splits[8].turn), (Some(-52.), Some(52.)));
    let timeline = play(&many);
    let ball_end = shapes(&timeline, BALL).last().expect("the ball").pos;
    let born = births(&timeline, SPLIT);
    assert_eq!(born.len(), 9);
    for (_, split) in &born {
        assert!(near(from(split.pos, ball_end), 8.), "{:?}", split.pos);
    }
    assert_eq!(shares(&timeline), [1.; 10]);
}

#[test]
fn path_of_destruction_pierces_and_burns_a_trail_every_32_px_of_explosion_size() {
    let path = spawns(&[("path_of_destruction", 3)]);
    assert_eq!(path[0].object, SPLIT);
    assert!(!path[0].ability && !path[0].ends_on_contact);
    let patches: Vec<f32> = path[1..]
        .iter()
        .flat_map(|s| s.layout.iter().map(|p| p.offset[0]))
        .collect();
    assert_eq!(patches[..3], [86., 156., 226.]);
    let timeline = play(&path);
    let burning = births(&timeline, PATCH)
        .into_iter()
        .filter(|(_, patch)| from(patch.pos, DUMMY) < 35.);
    let burning: Vec<f32> = burning.map(|(_, patch)| patch.pos[0]).collect();
    assert_eq!(burning, [156.]);
    let on_dummy: Vec<f32> = shares(&timeline);
    assert_eq!(on_dummy[0], 1.);
    assert_eq!(on_dummy[1..], [1.8; 6]);
    let ball = shapes(&timeline, SPLIT).last().expect("the ball");
    assert!(ball.pos[0] > 1000., "{:?}", ball.pos);
}

#[test]
fn a_patch_dropped_before_the_balls_hit_carries_the_fireflies_once() {
    let ranks = [
        ("path_of_destruction", 1),
        ("ignition_radius", 2),
        ("fireflies", 3),
    ];
    let swarm = |s: &[Spawn]| !s[0].procs.is_empty();
    let lucky = spawns_of(&ranks, cast_where(&ranks, 40, swarm));
    let early = lucky[1..]
        .iter()
        .find(|s| !s.procs.is_empty())
        .expect("an early patch");
    assert_eq!(early.layout.last().map(|p| p.offset[0]), Some(136.));
    let timeline = play(&lucky);
    let patch_hits = shares(&timeline).iter().filter(|s| near(**s, 1.68)).count();
    assert!(patch_hits > 1, "{patch_hits}");
    assert_eq!(births(&timeline, FLY).len(), 24);
}

#[test]
fn orbital_fire_circles_balls_out_from_the_player_until_they_touch_or_time_out() {
    let ring = spawns(&[("orbital_fire", 3)]);
    let orbit = ring[0].orbit.expect("an orbit");
    assert_eq!((orbit.count, orbit.radius, orbit.face), (13, 8., 40.));
    assert!(near(orbit.spread, 1.9) && near(orbit.speed, 10.));
    assert_eq!(ring[0].life, Some(84));
    let timeline = play(&ring);
    let balls = births(&timeline, ORBITAL);
    assert_eq!(balls.len(), 13);
    assert!(balls.iter().all(|(_, b)| near(from(b.pos, [0., 0.]), 8.)));
    let oldest = shapes(&timeline, ORBITAL).map(|s| s.age).max();
    assert_eq!(oldest, Some(84));
    // The ball and its explosion hit where a ball touches; one ball timing out nearby blasts too.
    let blasts = births(&timeline, BLAST);
    let touched = blasts.iter().filter(|(frame, _)| *frame < 84).count();
    assert_eq!((touched, timeline.hits.len()), (6, 2 * 6 + 1));
    let small = play(&spawns(&[("orbital_fire", 1)]));
    assert!(small.hits.is_empty(), "{:?}", small.hits);
}

const CLASS: &str = "pyromancer";

use super::parity::expected;

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_weighted(CLASS, "fireball", ranks)
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!(near(sim, 1.) && near(base, 1.), "sim {sim}, engine {base}");
    let wrong = parity::misses(CLASS, "fireball", expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_transformations_the_sim_plays_together() {
    let together: [&[(&str, u32)]; 5] = [
        &[("path_of_destruction", 1), ("orbital_fire", 1)],
        &[("path_of_destruction", 3), ("orbital_fire", 2)],
        &[("splitfire", 3), ("path_of_destruction", 1)],
        &[("splitfire", 2), ("orbital_fire", 3)],
        &[("fireflies", 3), ("path_of_destruction", 3)],
    ];
    for ranks in together {
        let (sim, engine) = (expected(ranks), engine(ranks));
        assert!(
            (sim - engine).abs() < 0.01,
            "{ranks:?}: sim {sim}, engine {engine}"
        );
    }
}

#[test]
fn extra_balls_leave_six_degrees_off_centre() {
    // Case 29 L0x25f centres the volley on the extra balls only: 12° × (i − (count − 2) / 2).
    let three = parity::play("fireball", &spawns(&[]), 3);
    let mut angles: Vec<f32> = births(&three, BALL).iter().map(|(_, s)| s.angle).collect();
    angles.sort_by(f32::total_cmp);
    assert_eq!(angles, [-6., 6., 18.]);
    let one = parity::play("fireball", &spawns(&[]), 1);
    assert_eq!(births(&one, BALL)[0].1.angle, 0.);
}

#[test]
fn the_engines_extra_balls_follow_the_sims_off_centre_volley() {
    let base = spawns(&[]);
    let hits: Vec<f32> = (1..=7)
        .map(|count| parity::weighted(&parity::play("fireball", &base, count)))
        .collect();
    assert_eq!(hits, [1., 1., 2., 1., 2., 1., 2.]);
    for (count, sim) in (1..=7).zip(hits) {
        let engine = parity::engine_volley_hits(CLASS, "fireball", &[], Some(count));
        assert!(
            near(sim, engine),
            "{count} balls: sim {sim}, engine {engine}"
        );
    }
}
