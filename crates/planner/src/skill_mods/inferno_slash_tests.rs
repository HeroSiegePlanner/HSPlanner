use crate::skill_effects::Buff;
use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use, uses};
use std::collections::HashMap;

const SKILL: &str = "inferno_slash";
const WAVE: &str = "Projectile_Player_obj";
const SWING: &str = "Visual_Effect_obj";
const PATCH: &str = "Pyromancer_Blazing_Trail_obj";
const FLY: &str = "Pyromancer_Fire_Ball_Fly_obj";
const BLAST: &str = "Explosion_Item_obj";
/// The base wave touches the in-zone dummy 16 moves into its flight.
const BASE_HIT: u32 = 17;

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("{SKILL}:{node}"), *rank))
        .collect();
    Mods::of(SKILL, &ranks)
}

fn spawns_with(mods: &Mods, cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill(SKILL).expect("inferno slash").spawn;
    crate::skill_mods::apply_cast(SKILL, base, mods, cast).into_owned()
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    spawns_with(&mods(ranks), cast)
}

/// A build whose full hit is a quarter fire, and whose weapon bonus is 80 % of the rest.
fn quarter_fire(ranks: &[(&str, u32)]) -> Mods {
    Mods {
        elemental: 0.25,
        weapon_bonus: 0.8,
        ..mods(ranks)
    }
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play(spawns: &[Spawn]) -> Timeline {
    let fx = crate::skill_sim::skill(SKILL).expect("inferno slash");
    let hitbox = crate::hitbox::skill(SKILL).expect("inferno slash hitboxes");
    replay_use(fx, hitbox, spawns, 1, 0)
}

/// The spawns of the first cast among the first `tries` whose spawns pass `lucky`.
fn lucky(ranks: &[(&str, u32)], tries: u64, lucky: impl Fn(&[Spawn]) -> bool) -> Vec<Spawn> {
    (0..tries)
        .map(|n| spawns_of(ranks, n))
        .find(|spawns| lucky(spawns))
        .expect("a cast that rolls it")
}

/// Whether some spawn, or anything it makes, is an `object`.
fn makes(spawns: &[Spawn], object: &str) -> bool {
    spawns.iter().any(|spawn| {
        spawn.object == object
            || makes(&spawn.on_end, object)
            || (spawn.procs.iter()).any(|proc| makes(std::slice::from_ref(&proc.spawn), object))
    })
}

/// Whether some spawn, or anything it makes, gives the player a buff.
fn buffs(spawns: &[Spawn]) -> bool {
    spawns.iter().any(|spawn| {
        spawn.buff.is_some()
            || buffs(&spawn.on_end)
            || (spawn.procs.iter()).any(|proc| buffs(std::slice::from_ref(&proc.spawn)))
    })
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

/// Every shape of `object` drawn in `frame`.
fn shapes_at<'a>(timeline: &'a Timeline, object: &'a str, frame: usize) -> Vec<&'a Shape> {
    let shapes = timeline.frames[frame].iter();
    shapes.filter(|shape| shape.object.name == object).collect()
}

fn shares(timeline: &Timeline) -> Vec<f32> {
    timeline.effects.damage.iter().map(|d| d.share).collect()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// Degrees of a point seen from the player, counter-clockwise with y down.
fn bearing(pos: [f32; 2]) -> f32 {
    (-pos[1]).atan2(pos[0]).to_degrees()
}

#[test]
fn the_base_use_is_the_swing_and_one_wave_hitting_once_mid_flight() {
    let base = spawns(&[]);
    assert_eq!(base.len(), 2);
    let timeline = play(&base);
    assert_eq!(timeline.hits, [BASE_HIT]);
    assert_eq!(shares(&timeline), [1.]);
    let waves = births(&timeline, WAVE);
    assert_eq!(waves.len(), 1);
    // Created inactive at the player, it moves 5 px a frame from 2 frames after the attack.
    assert_eq!((waves[0].0, waves[0].1.pos), (1, [0., 0.]));
    assert_eq!(shapes_at(&timeline, WAVE, 2)[0].pos, [5., 0.]);
    assert_eq!(births(&timeline, SWING).len(), 1);
    assert_eq!(uses(SKILL, &mods(&[])).expect("uses").len(), 8);
}

#[test]
fn infernal_reach_speeds_the_wave_up_so_it_hits_sooner() {
    let timeline = play(&spawns(&[("infernal_reach", 5)]));
    // projEffect[197] = 1 + 15 % × rank sets deltaSpeed 5 × 1.75 = 8.75 px a frame.
    assert_eq!(shapes_at(&timeline, WAVE, 2)[0].pos, [8.75, 0.]);
    assert_eq!(timeline.hits, [10]);
}

#[test]
fn igniting_wind_fires_a_second_salvo_woken_0_15_s_after_the_first() {
    let ranks = [("igniting_wind", 5)];
    let rolled = lucky(&ranks, 64, |spawns| spawns.len() == 3);
    let timeline = play(&rolled);
    assert_eq!(timeline.hits, [BASE_HIT, BASE_HIT + 9]);
    let plain = lucky(&ranks, 64, |spawns| spawns.len() == 2);
    assert_eq!(play(&plain).hits, [BASE_HIT]);
}

#[test]
fn infernal_cleave_sends_nine_waves_45_degrees_apart_one_hitting_with_its_fraction() {
    let timeline = play(&spawns(&[("infernal_cleave", 3)]));
    let waves = shapes_at(&timeline, WAVE, 2);
    let mut bearings: Vec<i32> = waves
        .iter()
        .map(|w| bearing(w.pos).round() as i32)
        .collect();
    bearings.sort_unstable();
    assert_eq!(bearings, [-180, -135, -90, -45, 0, 45, 90, 135, 180]);
    // L0x17f-L0x181: the fire part × (1 + 30 % × rank) / 8 waves.
    assert_eq!(timeline.hits, [BASE_HIT]);
    assert!(
        near(shares(&timeline)[0], 1.9 / 8.),
        "{:?}",
        shares(&timeline)
    );
}

#[test]
fn searing_fork_waves_share_one_hit_and_each_wave_that_touches_forks_two_that_hit() {
    for (rank, waves, hits) in [(1, 2, 5), (2, 3, 7), (3, 4, 9)] {
        let timeline = play(&spawns(&[("searing_fork", rank)]));
        assert_eq!(shapes_at(&timeline, WAVE, 2).len(), waves, "rank {rank}");
        assert_eq!(timeline.hits.len(), hits, "rank {rank}");
        // The salvo's waves share one pack: the first to touch hits, each fork a frame after
        // its wave touches; the tilted waves touch a frame sooner than the straight one.
        let first = timeline.hits[0];
        assert_eq!(first, BASE_HIT - 1, "rank {rank}");
        assert!(
            timeline.hits[1..]
                .iter()
                .all(|&f| f > first && f <= first + 2),
            "rank {rank}"
        );
        assert!(shares(&timeline).iter().all(|&s| s == 1.), "rank {rank}");
    }
}

#[test]
fn forks_and_forked_waves_end_with_the_waves_animation() {
    let timeline = play(&spawns(&[("searing_fork", 3)]));
    // Other_7: the 8-frame crescent ends on frame 25; forks copy its image_index.
    let last = (timeline.frames.iter().enumerate())
        .filter(|(_, shapes)| shapes.iter().any(|shape| shape.object.name == WAVE))
        .map(|(frame, _)| frame)
        .max();
    assert_eq!(last, Some(25));
    // The wave landing the salvo's hit is gone 3 frames after it forks (alarm[0] = 3).
    let hit = timeline.hits[0] as usize;
    let straight = |frame: usize| {
        let shapes = shapes_at(&timeline, WAVE, frame);
        shapes
            .iter()
            .filter(|shape| shape.age == frame as u32)
            .count()
    };
    assert_eq!((straight(hit + 2), straight(hit + 3)), (1, 0));
}

#[test]
fn searing_fork_with_infernal_cleave_divides_the_fire_part_by_the_fork_count() {
    let timeline = play(&spawns(&[("infernal_cleave", 1), ("searing_fork", 2)]));
    assert_eq!(timeline.hits.len(), 7);
    assert!(shares(&timeline).iter().all(|&s| near(s, 1.3 / 2.)));
}

#[test]
fn molten_ground_rolls_once_per_attack_and_leaves_a_patch_ticking_on_the_dummy() {
    let ranks = [("molten_ground", 5)];
    let rolled = lucky(&ranks, 64, |spawns| makes(spawns, PATCH));
    let timeline = play(&rolled);
    let patches = births(&timeline, PATCH);
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].1.pos, timeline.dummy);
    // [325] = 20 % × rank of the wave's fire damage, a tick every 0.5 s for 2.5 s.
    assert_eq!(timeline.hits.len(), 7);
    assert_eq!(shares(&timeline), [1.; 7]);
    let plain = lucky(&ranks, 64, |spawns| !makes(spawns, PATCH));
    assert_eq!(play(&plain).hits, [BASE_HIT]);
}

#[test]
fn flies_of_agony_sends_three_flies_a_rank_from_the_player_on_each_carriers_first_hit() {
    let timeline = play(&spawns(&[("flies_of_agony", 3)]));
    let flies = births(&timeline, FLY);
    assert_eq!(flies.len(), 9);
    assert!(
        flies
            .iter()
            .all(|(frame, _)| *frame == BASE_HIT as usize + 1)
    );
    assert_eq!(births(&timeline, BLAST).len(), 9);
    // Each fly's blast deals floor(d3 × 35 % × rank), besides the wave's own hit.
    let shares = shares(&timeline);
    assert_eq!(shares.len(), 10);
    assert_eq!(shares.iter().filter(|&&s| near(s, 1.05)).count(), 9);
}

#[test]
fn flies_of_agony_rides_the_molten_patch_which_carries_the_waves_other_modifiers() {
    let ranks = [("molten_ground", 1), ("flies_of_agony", 1)];
    let rolled = lucky(&ranks, 256, |spawns| makes(spawns, PATCH));
    let timeline = play(&rolled);
    // The wave's flies and, on the patch's first tick, the patch's own.
    assert_eq!(births(&timeline, FLY).len(), 6);
    assert_eq!(births(&timeline, BLAST).len(), 6);
}

#[test]
fn the_waves_flies_keep_its_molten_ground_and_each_blast_lays_a_patch() {
    // No handler zeroes 325, 327 leaves out only itself, and Fly_obj hands 325 to its blast.
    let ranks = [("molten_ground", 1), ("flies_of_agony", 1)];
    let rolled = lucky(&ranks, 256, |spawns| makes(spawns, PATCH));
    let timeline = play(&rolled);
    assert_eq!(births(&timeline, PATCH).len(), 4);
    let shares = shares(&timeline);
    let count = |share: f32| shares.iter().filter(|&&s| near(s, share)).count();
    // The wave, its patch's 6 ticks, 3 + 3 fly blasts and the blasts' 3 patches' 18 ticks.
    assert_eq!((count(1.), count(0.2), count(0.35)), (1, 6, 3));
    assert_eq!((count(0.07), shares.len()), (21, 31));
}

#[test]
fn stacked_burn_and_accelerated_fumes_show_their_buffs() {
    let ranks = [("stacked_burn", 5)];
    let rolled = lucky(&ranks, 64, buffs);
    let effects = play(&rolled).effects;
    // The 326 handler: +3 × rank for 5 s from the wave's hit.
    assert_eq!(effects.buffed, Some(([BASE_HIT, BASE_HIT + 300], 15.)));
    let fumes = lucky(&[("accelerated_fumes", 5)], 512, buffs);
    // L0x159: BuffAdd 308 as the attack goes off, +8 % × rank attack speed for 2 s.
    assert_eq!(play(&fumes).effects.buffed, Some(([0, 120], 40.)));
    let fumes_buff = fumes.iter().find_map(|spawn| spawn.buff);
    assert!(matches!(fumes_buff, Some(Buff { frames: 120, .. })));
}

#[test]
fn nodes_without_a_visible_effect_leave_the_use_alone() {
    let base = play(&spawns(&[]));
    for node in [
        "scattering_flame",
        "piercing_flame",
        "scorching_slash",
        "nerve_damage",
        "third_degree_burns",
        "fearful_flames",
    ] {
        let timeline = play(&spawns(&[(node, 3)]));
        assert_eq!(timeline.hits, base.hits, "{node}");
        assert_eq!(shares(&timeline), shares(&base), "{node}");
    }
}

#[test]
fn only_the_nodes_the_game_reads_its_own_way_keep_their_damage_off_the_hit() {
    let owned = crate::skill_mods::parity_tests::owned("pyromancer", "inferno_slash");
    assert_eq!(
        owned,
        [
            "molten_ground",
            "igniting_wind",
            "infernal_cleave",
            "flies_of_agony"
        ]
    );
}

#[test]
fn objects_copying_only_d3_deal_the_fire_parts_share_of_the_full_hit() {
    // attackClasses L0xce/L0xd2: physical and fire are apart in damage[]; the 325 and 327
    // handlers copy damage[3] alone.
    let molten = quarter_fire(&[("molten_ground", 5)]);
    let rolled = (0..64)
        .map(|n| spawns_with(&molten, n))
        .find(|spawns| makes(spawns, PATCH))
        .expect("a cast that rolls it");
    let shares = shares(&play(&rolled));
    assert_eq!(shares[0], 1.);
    assert!(shares[1..].iter().all(|&s| near(s, 0.25)), "{shares:?}");
    let flies = shares_of(&quarter_fire(&[("flies_of_agony", 3)]));
    assert_eq!(flies.iter().filter(|&&s| near(s, 1.05 * 0.25)).count(), 9);
}

#[test]
fn infernal_cleave_scales_the_fire_part_and_the_weapon_bonus_but_not_the_base_weapon() {
    // L0x17f-L0x181: [1020] and [993] × (1 + 30 % × rank) / 8; the weapon's own 100 % stays.
    let cleave = 1.9 / 8.;
    let wave = 0.25 * cleave + 0.75 * (1. - 0.8 * (1. - cleave));
    let shares = shares_of(&quarter_fire(&[
        ("infernal_cleave", 3),
        ("flies_of_agony", 1),
    ]));
    assert!(shares.iter().any(|&s| near(s, wave)), "{shares:?}");
    let fly = 0.35 * 0.25 * cleave;
    assert_eq!(shares.iter().filter(|&&s| near(s, fly)).count(), 3);
}

fn shares_of(mods: &Mods) -> Vec<f32> {
    shares(&play(&spawns_with(mods, 0)))
}

const CLASS: &str = "pyromancer";
const MOLTEN: &str = "molten_ground";
const FLIES: &str = "flies_of_agony";
const IGNITING: &str = "igniting_wind";
/// Objects a parity play may hold at once, well under the 256 past which the sim drops what procs
/// make.
const UNCUT: usize = 192;

/// Damage-weighted hits the game lands in one use, each of the attack's rolls counted at its own
/// node's odds, against the full hit of a build without a weapon (all fire). Infernal Cleave's
/// 1 + 30 % × rank is left out: the engine adds its fire damage to every hit instead.
fn expected(ranks: &[(&str, u32)]) -> f32 {
    let odds: Vec<(&str, f32)> = (super::ROLLED.iter())
        .filter_map(|&node| {
            let &learned = ranks.iter().find(|(id, _)| *id == node)?;
            Some((node, parity::cast_odds(CLASS, SKILL, &[learned])))
        })
        .collect();
    let corner = |mask: usize| {
        let on = |i: usize| mask >> i & 1 == 1;
        let weight: f32 = (odds.iter().enumerate())
            .map(|(i, &(_, p))| if on(i) { p } else { 1. - p })
            .product();
        let lucky = |node: &str| odds.iter().position(|&(id, _)| id == node).is_some_and(on);
        let made = parity::rewritten(CLASS, SKILL, ranks, 0, |spawns, tree| {
            super::apply_rolled(spawns, tree, &lucky)
        });
        let timeline = play(&made);
        let peak = timeline.frames.iter().map(Vec::len).max().unwrap_or(0);
        assert!(peak < UNCUT, "{ranks:?}: {peak} objects at once");
        weight * parity::weighted(&timeline)
    };
    let hits: f32 = (0..1 << odds.len()).map(corner).sum();
    let cleave = ranks.iter().find(|(node, _)| *node == "infernal_cleave");
    hits / (1. + cleave.map_or(0., |&(_, rank)| 0.3 * rank as f32))
}

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_hits(CLASS, SKILL, ranks)
}

/// Each of `together` landed by the sim and counted by the engine within 0.01.
fn agree<R: AsRef<[(&'static str, u32)]>>(together: &[R]) {
    let wrong: Vec<String> = (together.iter().map(AsRef::as_ref))
        .filter_map(|ranks| {
            let (sim, engine) = (expected(ranks), engine(ranks));
            let off = (sim - engine).abs() >= 0.01;
            off.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
        })
        .collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// Every rank of `a` with every rank of `b`.
fn pairs(a: &'static str, b: &'static str) -> Vec<Vec<(&'static str, u32)>> {
    let max = |id: &str| {
        let node = parity::nodes(CLASS, SKILL).iter().find(|n| n.id == id);
        node.map_or(0, |n| n.max_rank)
    };
    let ranks = (1..=max(a)).flat_map(|ra| (1..=max(b)).map(move |rb| (ra, rb)));
    ranks.map(|(ra, rb)| vec![(a, ra), (b, rb)]).collect()
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!(near(sim, 1.) && near(base, 1.), "sim {sim}, engine {base}");
    let wrong = parity::misses(CLASS, SKILL, expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_infernal_cleave_with_searing_fork() {
    // L0x169 overwrites the cleave's 8 with the fork's count before L0x180 divides by it.
    for cleave in 1..=3 {
        for fork in 1..=3 {
            agree(&[&[("infernal_cleave", cleave), ("searing_fork", fork)]]);
        }
    }
}

#[test]
fn the_engine_follows_what_every_fork_and_salvo_carries() {
    // Forks and Igniting Wind's salvo are whole carriers: each makes its own patch and flies.
    for fork in 1..=3 {
        agree(&[
            [("searing_fork", fork), (MOLTEN, 5)],
            [("searing_fork", fork), (FLIES, 3)],
            [("searing_fork", fork), (IGNITING, 5)],
        ]);
    }
    agree(&[pairs(IGNITING, MOLTEN), pairs(IGNITING, FLIES)].concat());
}

#[test]
fn the_engine_follows_the_procs_that_roll_each_other() {
    // Every fly blast lays a patch; a patch sends flies on its first tick only (327 is zeroed).
    agree(&pairs(MOLTEN, FLIES));
}

#[test]
fn the_engine_chains_the_procs_on_every_carrier_and_salvo() {
    agree(&[
        vec![(MOLTEN, 5), (FLIES, 3), (IGNITING, 5)],
        vec![(MOLTEN, 1), (FLIES, 1), (IGNITING, 1)],
        vec![(MOLTEN, 5), (FLIES, 3), ("infernal_cleave", 3)],
        vec![(MOLTEN, 5), (FLIES, 1), ("searing_fork", 1)],
        vec![(MOLTEN, 2), (FLIES, 1), ("searing_fork", 1), (IGNITING, 4)],
        vec![
            (MOLTEN, 3),
            (FLIES, 2),
            (IGNITING, 3),
            ("infernal_cleave", 2),
        ],
    ]);
}
