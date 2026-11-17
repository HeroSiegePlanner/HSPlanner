use super::{period, reaches};
use crate::skill_effects::Proc;
use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use};
use std::collections::HashMap;

const SKILL: &str = "scorching_aura";
const AURA: &str = "Pyromancer_Scorching_Aura_obj";
const RING: &str = "Shadow_Parent_obj";
const BURST: &str = "Pyromancer_Scorching_Searing_Burst_obj";
const PATCH: &str = "Pyromancer_Blazing_Trail_obj";
const SOUL: &str = "Pyromancer_Scorching_Harvester_obj";
const SOUL_BLAST: &str = "Explosion_Item_obj:Pyromancer_Explosion_spr";
const BLAST: &str = "Explosion_Item_obj";
const BALL: &str = "Pyromancer_Fire_Ball_Orbital_obj";
const DUMMY: [f32; 2] = [120., 0.];
/// Pulses a lucky roll is looked for in.
const TRIES: u64 = 101;

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("{SKILL}:{node}"), *rank))
        .collect();
    Mods::of(SKILL, &ranks)
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill(SKILL)
        .expect("scorching aura")
        .spawn;
    crate::skill_mods::apply_cast(SKILL, base, &mods(ranks), cast).into_owned()
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play_cast(spawns: &[Spawn], cast: u64) -> Timeline {
    let fx = crate::skill_sim::skill(SKILL).expect("scorching aura");
    let hitbox = crate::hitbox::skill(SKILL).expect("scorching aura hitboxes");
    replay_use(fx, hitbox, spawns, 1, cast)
}

fn pulse(ranks: &[(&str, u32)]) -> Timeline {
    play_cast(&spawns(ranks), 0)
}

fn find<'a>(spawns: &'a [Spawn], object: &str) -> Option<&'a Spawn> {
    spawns.iter().find(|spawn| spawn.object == object)
}

fn aura(spawns: &[Spawn]) -> &Spawn {
    find(spawns, AURA).expect("the pulse's aura")
}

fn made<'a>(procs: &'a [Proc], object: &str) -> Option<&'a Spawn> {
    procs
        .iter()
        .map(|proc| &proc.spawn)
        .find(|spawn| spawn.object == object)
}

/// The first cast among the first `TRIES` whose spawns pass `lucky`, with its number.
fn lucky(ranks: &[(&str, u32)], lucky: impl Fn(&[Spawn]) -> bool) -> (u64, Vec<Spawn>) {
    (0..TRIES)
        .map(|n| (n, spawns_of(ranks, n)))
        .find(|(_, spawns)| lucky(spawns))
        .expect("a cast that rolls it")
}

/// Casts among the first `TRIES` whose spawns pass `rolled`.
fn rolled(ranks: &[(&str, u32)], rolled: impl Fn(&[Spawn]) -> bool) -> usize {
    (0..TRIES).filter(|&n| rolled(&spawns_of(ranks, n))).count()
}

/// The first shape of every instance of `object`, with the frame it is drawn in.
fn births<'a>(timeline: &'a Timeline, object: &'a str) -> Vec<(usize, &'a Shape)> {
    timeline
        .frames
        .iter()
        .enumerate()
        .flat_map(|(frame, shapes)| shapes.iter().map(move |shape| (frame, shape)))
        .filter(|(_, shape)| shape.object.name == object && shape.age == 1)
        .collect()
}

fn near(a: f32, b: f32, within: f32) -> bool {
    (a - b).abs() < within
}

/// Ignition stacks the earlier pulses left the `cast`-th one, as its pulse proc holds them.
fn stacks_from(ranks: &[(&str, u32)], cast: u64) -> Option<u32> {
    let spawns = spawns_of(ranks, cast);
    let pulse = made(&aura(&spawns).procs, BLAST)?;
    pulse.stacks.map(|stacks| stacks.from)
}

#[test]
fn a_pulse_scorches_the_in_zone_dummy_once_under_a_shadow_lasting_its_period() {
    let base = spawns(&[]);
    let objects: Vec<&str> = base.iter().map(|s| s.object.as_str()).collect();
    assert_eq!(objects, [AURA, RING]);
    assert_eq!(
        (aura(&base).scale, aura(&base).life),
        (Some([2., 2.]), Some(6))
    );
    let shadow = find(&base, RING).unwrap();
    assert!(shadow.orbit.is_none());
    assert_eq!(shadow.scale, Some([1., 1.]));
    let timeline = pulse(&[]);
    // Parent Alarm_11 arms the aura 2 frames in; singleHit keeps it to one hit.
    assert_eq!(timeline.hits, [2]);
    assert_eq!(timeline.frames.len(), 61);
    let at_player = |shapes: &Vec<Shape>| {
        let shadows = shapes.iter().filter(|s| s.object.name == RING);
        shadows.filter(|s| s.pos == [0., 0.]).count()
    };
    assert!(timeline.frames.iter().all(|shapes| at_player(shapes) == 1));
}

#[test]
fn the_dummy_stands_inside_the_pulse_gate_that_a_160_px_one_misses() {
    let fx = crate::skill_sim::skill(SKILL).expect("scorching aura");
    assert_eq!(fx.dummy, DUMMY[0]);
    assert!(reaches(DUMMY[0], 0.));
    // The boxes 160 px apart are 137 px apart: out of the base 128, inside Scorched Lands' 147.
    assert!(!reaches(160., 0.));
    assert!(reaches(160., 0.15));
}

#[test]
fn scorched_lands_widens_the_aura_and_its_ring() {
    let lands = spawns(&[("scorched_lands", 5)]);
    assert!(near(aura(&lands).scale.unwrap()[0], 3.5, 1e-5));
    // The shadow's scale is the ring's 1 + area: circleRadius floor(128 × (1 + area)).
    let ring_scale = |rank| {
        find(&spawns(&[("scorched_lands", rank)]), RING)
            .unwrap()
            .scale
    };
    assert!(near(ring_scale(5).unwrap()[0], 1.75, 1e-5));
    assert!(near(ring_scale(1).unwrap()[1], 1.15, 1e-5));
}

#[test]
fn rapid_burning_shortens_the_pulse_period() {
    assert_eq!(period(0.), 61);
    for (rank, frames) in [(1, 52), (3, 34), (5, 16)] {
        let ring = find(&spawns(&[("rapid_burning", rank)]), RING)
            .cloned()
            .unwrap();
        assert_eq!(ring.life, Some(frames), "rank {rank}");
    }
}

#[test]
fn the_preview_pulses_at_the_aura_s_own_timer() {
    use crate::skill_mods::pulse_frames;
    assert_eq!(pulse_frames(SKILL, &mods(&[])), Some(61));
    assert_eq!(
        pulse_frames(SKILL, &mods(&[("rapid_burning", 5)])),
        Some(16)
    );
    assert_eq!(pulse_frames("fireball", &Mods::default()), None);
}

#[test]
fn burnt_feet_speeds_the_player_up_for_the_whole_pulse() {
    assert_eq!(
        pulse(&[("burnt_feet", 3)]).effects.buffed,
        Some(([0, 61], 45.))
    );
    let faster = pulse(&[("burnt_feet", 1), ("rapid_burning", 5)]);
    assert_eq!(faster.effects.buffed, Some(([0, 16], 15.)));
}

#[test]
fn searing_burst_scatters_four_bursts_a_rank_within_the_aura() {
    let bursts = |spawns: &[Spawn]| -> Vec<Spawn> {
        spawns
            .iter()
            .filter(|s| s.object == BURST)
            .cloned()
            .collect()
    };
    assert_eq!(bursts(&spawns(&[("searing_burst", 1)])).len(), 4);
    let three = bursts(&spawns(&[("searing_burst", 3)]));
    assert_eq!(three.len(), 12);
    for burst in &three {
        assert_eq!(
            (burst.scale, burst.life, burst.share),
            (Some([0.4, 0.4]), Some(24), Some(1.))
        );
        let [x, y] = burst.layout[0].offset;
        // irandom(floor(64 × scale)) px out: a whole number of px within 128.
        let out = x.hypot(y);
        assert!(out <= 128.01 && near(out, out.round(), 1e-3), "{out}");
    }
    let lucky = lucky(&[("searing_burst", 3)], |s| play_cast(s, 0).hits.len() > 1);
    assert!(play_cast(&lucky.1, lucky.0).hits.len() > 1);
}

#[test]
fn flaming_fissure_rolls_its_damage_value_as_the_pulse_chance() {
    let fissure = |spawns: &[Spawn]| made(&aura(spawns).procs, BURST).is_some();
    // L0x2e rolls LST(32, 4, 9), 10 % a rank, not the 3 % tooltip chance: about half at rank 5.
    let share = rolled(&[("flaming_fissure", 5)], fissure);
    assert!((35..=65).contains(&share), "{share}");
    let (cast, spawns) = lucky(&[("flaming_fissure", 1)], fissure);
    let burst = made(&aura(&spawns).procs, BURST).unwrap();
    assert!(burst.at_enemy);
    assert_eq!((burst.scale, burst.share), (Some([0.75, 0.75]), Some(2.)));
    let timeline = play_cast(&spawns, cast);
    // Made on the aura's hit, the burst steps from the next frame and arms 2 frames later.
    assert_eq!(timeline.hits, [2, 5]);
    assert_eq!(births(&timeline, BURST)[0].1.pos, DUMMY);
}

#[test]
fn ground_blaze_lights_a_patch_at_the_dummy_on_a_lucky_pulse() {
    let blaze = |spawns: &[Spawn]| made(&aura(spawns).procs, PATCH).is_some();
    let share = rolled(&[("ground_blaze", 5)], blaze);
    assert!((8..=35).contains(&share), "{share}");
    let (cast, spawns) = lucky(&[("ground_blaze", 2)], blaze);
    let patch = made(&aura(&spawns).procs, PATCH).unwrap();
    assert!(patch.at_enemy && near(patch.share.unwrap(), 0.24, 1e-5));
    // Alarm_0 resets the 0.7 maxScale the handler sets to image_xscale 1.
    assert_eq!(patch.grow.and_then(|grow| grow.max), Some(1.));
    let timeline = play_cast(&spawns, cast);
    assert_eq!(births(&timeline, PATCH)[0].1.pos, DUMMY);
    assert!(timeline.hits.len() > 2, "{:?}", timeline.hits);
}

#[test]
fn scorching_harvester_sends_a_soul_from_the_player_bursting_on_the_dummy() {
    let spawns = spawns(&[("scorching_harvester", 2)]);
    let soul = made(&aura(&spawns).procs, SOUL).expect("a soul");
    assert!(soul.at_player && soul.aim && soul.ability);
    assert_eq!(soul.speed, 4.);
    let blast = &soul.on_end[0];
    assert_eq!(blast.object, SOUL_BLAST);
    assert!(near(blast.share.unwrap(), 0.4, 1e-5));
    let timeline = play_cast(&spawns, 0);
    assert_eq!(births(&timeline, SOUL)[0].1.pos, [0., 0.]);
    let (frame, burst) = births(&timeline, SOUL_BLAST)[0];
    // 4 px a frame from the aura's hit until its centre is inside the dummy's 12 px.
    assert!(
        burst.pos[0] >= DUMMY[0] - 12. && frame > 20,
        "{frame} {:?}",
        burst.pos
    );
    assert_eq!(timeline.hits.len(), 2);
}

#[test]
fn ignite_the_flames_pulses_on_the_hit_that_finds_a_hundred_stacks() {
    let ranks = [("ignite_the_flames", 5)];
    let from = |cast| stacks_from(&ranks, cast);
    // The loop starts with the stacks full, so its first hit pulses, then a hundred more stack.
    assert_eq!(
        [from(0), from(1), from(100)],
        [Some(100), Some(0), Some(99)]
    );
    let spawns = spawns_of(&ranks, 0);
    let first = play_cast(&spawns, 0);
    let pulses = births(&first, BLAST);
    assert_eq!(pulses.len(), 1);
    assert_eq!((pulses[0].1.pos, pulses[0].1.scale), (DUMMY, [2.5, 2.5]));
    let pulse = made(&aura(&spawns).procs, BLAST).unwrap();
    // damage × (1 + value × stacks × 0.01), value = LST(32, 7, 9) × 0.01.
    assert!(near(pulse.share.unwrap(), 1.05, 1e-5));
    assert!(births(&play_cast(&spawns_of(&ranks, 1), 1), BLAST).is_empty());
}

#[test]
fn the_ignition_count_follows_the_stacks_each_pulse_lands() {
    let ranks = [("ignite_the_flames", 1), ("flaming_fissure", 5)];
    let mut expected = 100;
    for cast in 0..12 {
        assert_eq!(stacks_from(&ranks, cast), Some(expected), "cast {cast}");
        let stacked = play_cast(&spawns_of(&ranks, cast), cast).effects.stacks;
        expected = (expected + stacked) % 101;
    }
}

#[test]
fn release_the_chaos_spirals_balls_out_with_the_ignition_pulse() {
    let ranks = [("ignite_the_flames", 1), ("release_the_chaos", 1)];
    let spawns = spawns_of(&ranks, 0);
    let balls: Vec<&Spawn> = aura(&spawns)
        .procs
        .iter()
        .map(|proc| &proc.spawn)
        .filter(|spawn| spawn.object == BALL)
        .collect();
    assert_eq!(balls.len(), 3);
    for (ball, at) in balls.iter().zip([0., 120., 240.]) {
        let orbit = ball.orbit.expect("a spiral");
        assert_eq!(
            (orbit.radius, orbit.speed, orbit.spread, orbit.at),
            (16., 10., 3., at)
        );
        assert!(ball.ends_on_contact && ball.stacks.is_some());
        assert_eq!(ball.life, Some(135));
        // damage × (1 + 0.01 × rank + [346]), [346] = 40 % a rank.
        assert!(near(ball.share.unwrap(), 1.41, 1e-5));
        assert_eq!(ball.on_end[0].scale, Some([2., 2.]));
    }
    let timeline = play_cast(&spawns, 0);
    let born = births(&timeline, BALL);
    assert_eq!(born.len(), 3);
    // The first Step turns 10° and puts it 16 px out round the player.
    assert!(
        born.iter()
            .all(|(_, s)| near(s.pos[0].hypot(s.pos[1]), 16., 0.01))
    );
}

fn objects(procs: &[Proc]) -> Vec<String> {
    procs.iter().map(|p| p.spawn.object.clone()).collect()
}

#[test]
fn each_carrier_holds_every_effect_but_the_one_that_made_it() {
    // PC02's 341-344 handlers never write their effect back (L0x145-L0x157, L0x15b-L0x16c,
    // L0x1ad-L0x1ba, L0x1be-L0x1ec), and InheritModifiers leaves out only the maker's.
    let ranks = [
        ("flaming_fissure", 5),
        ("ground_blaze", 5),
        ("ignite_the_flames", 1),
        ("scorching_harvester", 1),
    ];
    let both = |s: &[Spawn]| {
        let procs = &aura(s).procs;
        made(procs, BURST).is_some() && made(procs, PATCH).is_some()
    };
    let (_, spawns) = lucky(&ranks, both);
    let procs = &aura(&spawns).procs;
    assert_eq!(objects(procs), [BURST, BLAST, PATCH, SOUL]);
    let of = |object| made(procs, object).unwrap();
    assert_eq!(objects(&of(BURST).procs), [BLAST, PATCH, SOUL]);
    assert_eq!(objects(&of(BLAST).procs), [BURST, PATCH, SOUL]);
    assert_eq!(objects(&of(PATCH).procs), [BURST, BLAST, SOUL]);
    // Harvester Step_0 L0x20 hands its explosion all the soul holds.
    assert!(of(SOUL).procs.is_empty());
    assert_eq!(objects(&of(SOUL).on_end[0].procs), [BURST, BLAST, PATCH]);
    let blaze = made(&of(BURST).procs, PATCH).unwrap();
    assert!(near(blaze.share.unwrap(), 2. * 0.6, 1e-5));
    assert_eq!(objects(&blaze.procs), [BLAST, SOUL]);
    let all = [procs, &of(PATCH).procs, &of(SOUL).on_end[0].procs];
    assert!(all.iter().all(|procs| procs.iter().all(|p| !p.once)));
}

#[test]
fn a_ground_blaze_patch_fires_its_effects_on_every_tick() {
    // Blazing_Trail_obj Create_0 L0x8-L0x9: singleHit with a tick_frequency, each tick a hit.
    let ranks = [("ground_blaze", 5), ("scorching_harvester", 1)];
    let blaze = |s: &[Spawn]| made(&aura(s).procs, PATCH).is_some();
    let (cast, spawns) = lucky(&ranks, blaze);
    let timeline = play_cast(&spawns, cast);
    // The aura's patch sends a soul each of its 6 ticks; the aura's own soul explodes into a
    // second patch, which holds nothing.
    assert_eq!(births(&timeline, PATCH).len(), 2);
    assert_eq!(births(&timeline, SOUL).len(), 7);
    assert_eq!(births(&timeline, SOUL_BLAST).len(), 7);
    assert_eq!(timeline.hits.len(), 1 + 6 + 7 + 6);
}

#[test]
fn a_ball_hands_its_explosion_all_it_holds() {
    // Orbital_obj Destroy_0 L0x18: InheritModifiers(explosion, ball) leaves nothing out.
    let ranks = [
        ("ignite_the_flames", 1),
        ("release_the_chaos", 1),
        ("scorching_harvester", 1),
    ];
    let spawns = spawns_of(&ranks, 0);
    let ball = made(&aura(&spawns).procs, BALL).expect("a ball");
    assert_eq!(objects(&ball.procs), [SOUL]);
    assert_eq!(objects(&ball.on_end[0].procs), [SOUL]);
}

#[test]
fn every_hit_of_an_object_holding_ignite_stacks() {
    let ranks = [
        ("ignite_the_flames", 1),
        ("flaming_fissure", 5),
        ("ground_blaze", 5),
        ("scorching_harvester", 3),
    ];
    let mut expected = 100;
    for cast in 0..12 {
        assert_eq!(stacks_from(&ranks, cast), Some(expected), "cast {cast}");
        let stacked = play_cast(&spawns_of(&ranks, cast), cast).effects.stacks;
        expected = (expected + stacked) % 101;
    }
    // A pulse rolling both lands 1 + 70 stacking hits: bursts, patch ticks and explosions.
    let both = |s: &[Spawn]| {
        let procs = &aura(s).procs;
        made(procs, BURST).is_some() && made(procs, PATCH).is_some()
    };
    let (cast, spawns) = lucky(&ranks, both);
    assert_eq!(play_cast(&spawns, cast).effects.stacks, 71);
}

#[test]
fn nodes_that_burn_break_or_add_damage_change_no_object() {
    let base = pulse(&[]);
    for node in [
        "scorching_aura_core",
        "sixt_degree_burns",
        "burning_necrosis",
        "scorching_agony",
        "burn_them_all",
        "burning_soul",
    ] {
        let with = spawns(&[(node, 1)]);
        assert_eq!(with.len(), 2, "{node}");
        assert!(aura(&with).procs.is_empty(), "{node}");
        let timeline = pulse(&[(node, 1)]);
        assert_eq!(timeline.hits, base.hits, "{node}");
        assert_eq!(timeline.frames.len(), base.frames.len(), "{node}");
    }
}

#[test]
fn only_the_nodes_the_game_reads_its_own_way_keep_their_damage_off_the_hit() {
    let owned = crate::skill_mods::parity_tests::owned("pyromancer", "scorching_aura");
    assert_eq!(
        owned,
        [
            "flaming_fissure",
            "ignite_the_flames",
            "ground_blaze",
            "scorching_harvester",
            "release_the_chaos"
        ]
    );
}

const CLASS: &str = "pyromancer";

/// The engine's damage-weighted hits per pulse.
fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_hits(CLASS, SKILL, ranks)
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let uncapped = super::parity::uncapped;
    let (sim, base) = (uncapped(&[]), engine(&[]));
    assert!(
        near(sim, 1., 1e-4) && near(base, 1., 1e-4),
        "sim {sim}, engine {base}"
    );
    let wrong = parity::misses(CLASS, SKILL, uncapped, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// Every rank of `a` with every rank of `b`, on top of `with`.
fn pairs(
    with: &[(&'static str, u32)],
    a: (&'static str, u32),
    b: (&'static str, u32),
) -> Vec<Vec<(&'static str, u32)>> {
    let ranks = |(node, max): (&'static str, u32)| (1..=max).map(move |rank| (node, rank));
    let rows = ranks(a).flat_map(|x| ranks(b).map(move |y| [x, y]));
    rows.map(|pair| with.iter().copied().chain(pair).collect())
        .collect()
}

/// Each of `together` counted by the sim, the pulse's rolls at their odds, and by the engine.
fn agree(together: &[Vec<(&str, u32)>]) {
    let wrong: Vec<String> = (together.iter())
        .filter_map(|ranks| {
            let (sim, engine) = (super::parity::uncapped(ranks), engine(ranks));
            let off = (sim - engine).abs() >= 0.01;
            off.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
        })
        .collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_procs_that_roll_each_other() {
    let (fissure, blaze, harvester) = (
        ("flaming_fissure", 5),
        ("ground_blaze", 5),
        ("scorching_harvester", 3),
    );
    // Every object a proc makes rolls the others, and Release the Chaos needs Ignite the Flames.
    let mut together = pairs(&[], fissure, harvester);
    together.extend(pairs(&[], fissure, blaze));
    together.extend(pairs(&[], fissure, ("ignite_the_flames", 5)));
    together.extend(pairs(&[], blaze, harvester));
    together.extend(pairs(
        &[],
        ("ignite_the_flames", 5),
        ("release_the_chaos", 3),
    ));
    together.extend(pairs(&[], ("ignite_the_flames", 5), harvester));
    together.extend(pairs(&[], ("ignite_the_flames", 5), blaze));
    for ignite in [1, 5] {
        let lit = [("ignite_the_flames", ignite)];
        for other in [fissure, harvester, blaze] {
            together.extend(pairs(&lit, ("release_the_chaos", 3), other));
        }
    }
    agree(&together);
}

#[test]
fn the_engine_follows_searing_burst_spread_by_scorched_lands() {
    agree(&pairs(&[], ("searing_burst", 3), ("scorched_lands", 5)));
}

/// The on-hit procs, each of whose objects rolls the others; Release the Chaos' balls are Ignite
/// the Flames' objects.
const PROCS: [&str; 5] = [
    "flaming_fissure",
    "ignite_the_flames",
    "ground_blaze",
    "scorching_harvester",
    "release_the_chaos",
];

#[test]
fn the_engine_chains_three_and_more_procs_as_the_sim_does() {
    let sets = parity::proc_sets(CLASS, SKILL, &PROCS);
    let wrong = parity::set_misses(&sets, super::parity::uncapped, engine);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
