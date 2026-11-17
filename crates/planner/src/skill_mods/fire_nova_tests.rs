use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use, uses};
use std::collections::HashMap;

const NOVA: &str = "Pyromancer_Firenova_obj";
const SPLIT: &str = "Pyromancer_Fire_Ball_Split_obj";
const ORBITAL: &str = "Pyromancer_Fire_Ball_Orbital_obj";
const BLAST: &str = "Explosion_Item_obj";
const METEOR: &str = "Pyromancer_Armageddon_obj";
const PATCH: &str = "Pyromancer_Blazing_Trail_obj";
const CONTROLLER: &str = "Pyromancer_Blazing_Trail_Controller_obj";
const DUMMY: [f32; 2] = [100., 0.];

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("fire_nova:{node}"), *rank))
        .collect();
    Mods::of("fire_nova", &ranks)
}

fn spawns_with(mods: &Mods, cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill("fire_nova")
        .expect("fire_nova")
        .spawn;
    crate::skill_mods::apply_cast("fire_nova", base, mods, cast).into_owned()
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    spawns_with(&mods(ranks), cast)
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play(spawns: &[Spawn]) -> Timeline {
    let fx = crate::skill_sim::skill("fire_nova").expect("fire_nova");
    let hitbox = crate::hitbox::skill("fire_nova").expect("fire_nova hitboxes");
    replay_use(fx, hitbox, spawns, 1, 0)
}

/// The spawns of the first cast among the first `tries` whose spawns pass `lucky`.
fn lucky(ranks: &[(&str, u32)], tries: u64, lucky: impl Fn(&[Spawn]) -> bool) -> Vec<Spawn> {
    (0..tries)
        .map(|n| spawns_of(ranks, n))
        .find(|spawns| lucky(spawns))
        .expect("a cast that rolls it")
}

fn makes(spawns: &[Spawn], object: &str) -> bool {
    spawns.iter().any(|spawn| spawn.object == object)
}

fn meteors(spawns: &[Spawn]) -> bool {
    spawns.iter().any(|spawn| !spawn.procs.is_empty())
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

fn frames_of(timeline: &Timeline, object: &str) -> Vec<usize> {
    births(timeline, object).iter().map(|(f, _)| *f).collect()
}

/// The shapes of `object` drawn in `frame`.
fn at_frame<'a>(timeline: &'a Timeline, object: &'a str, frame: usize) -> Vec<&'a Shape> {
    let shapes = timeline.frames[frame].iter();
    shapes.filter(|shape| shape.object.name == object).collect()
}

fn shares(timeline: &Timeline) -> Vec<f32> {
    timeline.effects.damage.iter().map(|d| d.share).collect()
}

fn near(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-2
}

/// GameMaker's lengthdir from the player.
fn out(length: f32, degrees: f32) -> [f32; 2] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [cos * length, -sin * length]
}

/// px a traveling nova has gone after `moves` moves: 8 px, then 0.9 of the last.
fn travelled(moves: i32) -> f32 {
    80. * (1. - 0.9f32.powi(moves))
}

#[test]
fn the_base_use_is_one_nova_on_the_player_hitting_the_dummy_once() {
    let base = spawns(&[]);
    assert_eq!(base.len(), 1);
    assert_eq!(base[0].grow.and_then(|grow| grow.max), Some(1.));
    let timeline = play(&base);
    let novas = births(&timeline, NOVA);
    assert_eq!(novas.len(), 1);
    assert_eq!((novas[0].0, novas[0].1.pos), (0, [0., 0.]));
    assert_eq!(shares(&timeline), [1.]);
    assert_eq!(uses("fire_nova", &mods(&[])).expect("uses").len(), 8);
}

#[test]
fn extended_and_traveling_flames_widen_the_nova_and_the_size_bonus_adds_to_it() {
    let max = |spawns: &[Spawn]| spawns[0].grow.and_then(|grow| grow.max).expect("a max");
    assert!((max(&spawns(&[("extended_flames", 5)])) - 1.75).abs() < 1e-5);
    let both = spawns(&[("extended_flames", 5), ("traveling_flames", 3)]);
    assert!((max(&both) - 2.2).abs() < 1e-5);
    let mut sized = mods(&[("extended_flames", 2)]);
    sized.size_bonus = 0.2;
    assert!((max(&spawns_with(&sized, 0)) - 1.5).abs() < 1e-5);
}

#[test]
fn traveling_flames_sends_the_nova_on_and_each_makes_the_next_nine_frames_later() {
    let timeline = play(&spawns(&[("traveling_flames", 3)]));
    let novas = births(&timeline, NOVA);
    assert_eq!(frames_of(&timeline, NOVA), [0, 9, 18, 27]);
    for (i, (_, nova)) in novas.iter().enumerate() {
        let spot = [travelled(8) * i as f32, 0.];
        assert!(near(nova.pos, spot), "{i}: {:?}", nova.pos);
    }
    let first = |frame| at_frame(&timeline, NOVA, frame)[0].pos[0];
    assert!((first(1) - 8.).abs() < 1e-4 && (first(2) - 15.2).abs() < 1e-4);
    // Each nova, made nearer the dummy, hits it once.
    assert_eq!(shares(&timeline), [1.; 4]);
}

#[test]
fn scorching_bolts_throws_piercing_balls_all_round_instead_of_the_nova() {
    let timeline = play(&spawns(&[("scorching_bolts", 1)]));
    assert!(births(&timeline, NOVA).is_empty());
    let balls = at_frame(&timeline, SPLIT, 1);
    assert_eq!(balls.len(), 5);
    for (i, ball) in balls.iter().enumerate() {
        let angle = 72. * i as f32;
        assert!(near(ball.pos, out(8.75, angle)), "{i}: {:?}", ball.pos);
        assert!((ball.angle - angle).abs() < 1e-3 && ball.scale == [1., 1.]);
    }
    // Only the ball thrown at the dummy hits it, once, and each explodes where it ends.
    assert_eq!(shares(&timeline), [1.]);
    assert_eq!(births(&timeline, BLAST).len(), 5);
    let wide = play(&spawns(&[("scorching_bolts", 2), ("extended_flames", 2)]));
    let balls = at_frame(&wide, SPLIT, 1);
    assert_eq!(balls.len(), 10);
    assert!(near(balls[0].pos, [10.5, 0.]) && (balls[0].scale[0] - 1.3).abs() < 1e-5);
}

#[test]
fn searing_tornado_alone_spirals_balls_out_that_explode_on_the_dummy() {
    let timeline = play(&spawns(&[("searing_tornado", 1)]));
    assert!(births(&timeline, NOVA).is_empty());
    let balls = at_frame(&timeline, ORBITAL, 1);
    assert_eq!(balls.len(), 2);
    assert!(near(balls[0].pos, out(48., 12.)) && near(balls[1].pos, out(48., 192.)));
    assert!((balls[0].angle - 52.).abs() < 1e-3);
    assert!(near(at_frame(&timeline, ORBITAL, 2)[0].pos, out(50., 24.)));
    let hits = shares(&timeline);
    assert!(
        !hits.is_empty() && hits.iter().all(|share| *share == 1.),
        "{hits:?}"
    );
    assert!(!births(&timeline, BLAST).is_empty());
}

#[test]
fn searing_tornado_leaves_scorching_bolts_flying_straight() {
    let bolts = play(&spawns(&[("scorching_bolts", 2)]));
    let both = play(&spawns(&[("scorching_bolts", 2), ("searing_tornado", 3)]));
    assert!(births(&both, ORBITAL).is_empty());
    let spots = |timeline: &Timeline| -> Vec<[f32; 2]> {
        at_frame(timeline, SPLIT, 5).iter().map(|s| s.pos).collect()
    };
    assert_eq!(spots(&both), spots(&bolts));
    assert_eq!((shares(&both), &both.hits), (shares(&bolts), &bolts.hits));
}

#[test]
fn armageddon_bolts_drop_four_meteors_on_the_dummy_each_carrier_hits() {
    let ranks = [("armageddon_bolts", 5)];
    let casts: Vec<bool> = (0..200).map(|n| meteors(&spawns_of(&ranks, n))).collect();
    assert!(casts.contains(&true) && casts.contains(&false));
    let timeline = play(&lucky(&ranks, 200, meteors));
    let falling = births(&timeline, METEOR);
    let heights: Vec<f32> = falling.iter().map(|(_, s)| s.height).collect();
    assert_eq!(heights, [320., 400., 480., 560.]);
    assert!(falling.iter().all(|(_, s)| s.pos == DUMMY));
    assert_eq!(births(&timeline, BLAST).len(), 4);
    assert_eq!(shares(&timeline), [1.; 5]);
    let ranks = [("armageddon_bolts", 5), ("traveling_flames", 1)];
    let chain = play(&lucky(&ranks, 200, meteors));
    assert_eq!(births(&chain, METEOR).len(), 8);
    assert_eq!(shares(&chain), [1.; 10]);
}

#[test]
fn mirror_flames_casts_the_nova_again_sixteen_frames_later_once() {
    let ranks = [("mirror_flames", 5)];
    let counts: Vec<usize> = (0..40).map(|n| spawns_of(&ranks, n).len()).collect();
    assert!(counts.contains(&1) && counts.contains(&2), "{counts:?}");
    assert!(counts.iter().all(|n| [1, 2].contains(n)));
    let mirrored = lucky(&ranks, 40, |s| s.len() == 2);
    assert_eq!((mirrored[0].frame, mirrored[1].frame), (0, 16));
    let timeline = play(&mirrored);
    assert_eq!(frames_of(&timeline, NOVA), [0, 16]);
    assert_eq!(shares(&timeline), [1., 1.]);
}

#[test]
fn molten_trail_walks_a_controller_out_laying_a_patch_every_44_px() {
    let ranks = [("molten_trail", 2)];
    let timeline = play(&lucky(&ranks, 40, |s| makes(s, CONTROLLER)));
    let patches = births(&timeline, PATCH);
    let spots: Vec<f32> = patches.iter().map(|(_, s)| s.pos[0]).collect();
    let every_44: Vec<f32> = (1..=10).map(|i| 44. * i as f32).collect();
    assert_eq!(spots, every_44);
    let frames: Vec<usize> = (0..10).map(|i| 12 + 11 * i).collect();
    assert_eq!(frames_of(&timeline, PATCH), frames);
    // The nova's hit, then the patches at 88 and 132 px, each ticking about once a second.
    assert_eq!(timeline.hits, [9, 25, 47, 88, 98, 148, 158]);
    assert_eq!(shares(&timeline), [1.; 7]);
}

#[test]
fn vacuum_flames_pulls_the_dummy_to_the_spot_48_px_along_the_aim_on_each_cast() {
    let timeline = play(&spawns(&[("vacuum_flames", 1)]));
    assert_eq!(timeline.effects.pulls, [([0, 60], [48., 0.])]);
    assert_eq!(shares(&timeline), [1.]);
    let bolts = play(&spawns(&[("vacuum_flames", 3), ("scorching_bolts", 1)]));
    assert_eq!(bolts.effects.pulls, [([0, 60], [48., 0.])]);
    // The copy's pull toward the same spot comes while the first lasts, so it extends it.
    let ranks = [("vacuum_flames", 1), ("mirror_flames", 5)];
    let mirrored = play(&lucky(&ranks, 40, |s| s.len() == 4));
    assert_eq!(mirrored.effects.pulls, [([0, 76], [48., 0.])]);
}

#[test]
fn damage_and_ailment_nodes_leave_the_cast_as_it_is() {
    let base = play(&spawns(&[]));
    for (node, rank) in [
        ("fire_nova_core", 1),
        ("boiling_point", 8),
        ("heat_haze", 5),
        ("burning_agony", 5),
        ("crushing_flames", 5),
        ("rapid_flames", 5),
        ("agonizing_flames", 5),
    ] {
        let cast = spawns(&[(node, rank)]);
        assert!(cast.len() == 1 && cast[0].procs.is_empty(), "{node}");
        let cast = play(&cast);
        assert_eq!((shares(&cast), &cast.hits), (shares(&base), &base.hits));
    }
}

#[test]
fn only_the_nodes_the_game_reads_its_own_way_keep_their_damage_off_the_hit() {
    let owned = crate::skill_mods::parity_tests::owned("pyromancer", "fire_nova");
    assert_eq!(owned, ["mirror_flames", "armageddon_bolts", "molten_trail"]);
}

const CLASS: &str = "pyromancer";

/// Damage-weighted hits a cast is expected to land, its rolls counted at their odds.
fn expected(ranks: &[(&str, u32)]) -> f32 {
    parity::expected_over_rolls(CLASS, "fire_nova", ranks, super::apply_rolled)
}

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_weighted(CLASS, "fire_nova", ranks)
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!(
        (sim - 1.).abs() < 1e-3 && (base - 1.).abs() < 1e-3,
        "sim {sim}, engine {base}"
    );
    let wrong = parity::misses(CLASS, "fire_nova", expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_transformations_the_sim_plays_together() {
    let together: [&[(&str, u32)]; 15] = [
        &[("scorching_bolts", 1), ("searing_tornado", 1)],
        &[("scorching_bolts", 3), ("searing_tornado", 3)],
        &[("scorching_bolts", 2), ("traveling_flames", 3)],
        &[("searing_tornado", 2), ("traveling_flames", 3)],
        &[("armageddon_bolts", 5), ("traveling_flames", 1)],
        &[("armageddon_bolts", 3), ("traveling_flames", 3)],
        &[("armageddon_bolts", 5), ("scorching_bolts", 2)],
        &[("armageddon_bolts", 5), ("searing_tornado", 2)],
        &[("mirror_flames", 5), ("traveling_flames", 3)],
        &[("mirror_flames", 2), ("scorching_bolts", 2)],
        &[("mirror_flames", 5), ("searing_tornado", 3)],
        &[("molten_trail", 2), ("traveling_flames", 3)],
        &[("molten_trail", 2), ("scorching_bolts", 2)],
        &[("molten_trail", 1), ("searing_tornado", 2)],
        &[("vacuum_flames", 3), ("searing_tornado", 2)],
    ];
    for ranks in together {
        let (sim, engine) = (expected(ranks), engine(ranks));
        assert!(
            (sim - engine).abs() < 0.01,
            "{ranks:?}: sim {sim}, engine {engine}"
        );
    }
}
