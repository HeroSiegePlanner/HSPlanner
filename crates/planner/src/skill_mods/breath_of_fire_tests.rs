use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use, uses};
use std::collections::HashMap;

const BREATH: &str = "Pyromancer_Breath_of_Fire_obj";
const FRAGMENT: &str = "Pyromancer_Breath_Heat_Combustion_obj";
const NOVA: &str = "Pyromancer_Firenova_obj";
const BLAST: &str = "Explosion_Item_obj";
const ORB: &str = "Pyromancer_Breath_Molten_Orb_obj";
const DUMMY: [f32; 2] = [160., 0.];

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("breath_of_fire:{node}"), *rank))
        .collect();
    Mods::of("breath_of_fire", &ranks)
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    let fx = crate::skill_sim::skill("breath_of_fire").expect("breath");
    crate::skill_mods::apply_cast("breath_of_fire", &fx.spawn, &mods(ranks), cast).into_owned()
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play(spawns: &[Spawn]) -> Timeline {
    let fx = crate::skill_sim::skill("breath_of_fire").expect("breath");
    let hitbox = crate::hitbox::skill("breath_of_fire").expect("breath hitboxes");
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

fn has(spawns: &[Spawn], object: &str) -> bool {
    spawns[0]
        .procs
        .iter()
        .any(|proc| proc.spawn.object == object)
}

#[test]
fn the_base_cast_keeps_one_breath_hitting_the_dummy_once() {
    let base = spawns(&[]);
    assert_eq!(base.len(), 1);
    assert_eq!((base[0].object.as_str(), base[0].extra), (BREATH, 0));
    assert!(base[0].procs.is_empty() && base[0].trail.is_none());
    let grow = base[0].grow.expect("a growth");
    assert!(near(grow.step, 0.1) && grow.max == Some(1.25));
    assert_eq!(shares(&play(&base)), [1.]);
    assert_eq!(uses("breath_of_fire", &mods(&[])).expect("uses").len(), 11);
}

#[test]
fn expanding_heat_and_scorching_exhale_speed_up_and_raise_the_growth_and_widen_it() {
    let both = spawns(&[("expanding_heat", 3), ("scorching_exhale", 5)]);
    let grow = both[0].grow.expect("a growth");
    assert!(near(grow.step, 0.305) && near(grow.max.expect("a cap"), 3.8125));
    assert_eq!(
        both[0].scale.map(|[x, y]| near(x, 0.25) && near(y, 2.5625)),
        Some(true)
    );
    let longer = spawns(&[("scorching_exhale", 5)]);
    assert_eq!(longer[0].scale.map(|[_, y]| near(y, 1.25)), Some(true));
    let widest = shapes(&play(&both), BREATH)
        .map(|shape| shape.scale[0])
        .fold(0., f32::max);
    assert!(near(widest, 3.8125), "{widest}");
}

#[test]
fn hydras_breeze_fans_a_breath_more_a_rank_15_degrees_apart() {
    let hits: Vec<usize> = (1..=3)
        .map(|rank| play(&spawns(&[("hydras_breeze", rank)])).hits.len())
        .collect();
    // Capped at x1.25, the ±15° breaths of rank 2 fall short of the dummy at 160 px.
    assert_eq!(hits, [2, 1, 2]);
    let fan = play(&spawns(&[("hydras_breeze", 2)]));
    let mut angles: Vec<f32> = births(&fan, BREATH).iter().map(|(_, s)| s.angle).collect();
    angles.sort_by(f32::total_cmp);
    assert_eq!(angles, [-15., 0., 15.]);
    let wide = play(&spawns(&[("hydras_breeze", 2), ("expanding_heat", 3)]));
    assert_eq!(wide.hits.len(), 3);
}

#[test]
fn heat_combustion_may_throw_five_fragments_out_of_the_dummy() {
    let ranks = [("heat_combustion", 5)];
    let rolled: Vec<bool> = (0..40)
        .map(|n| has(&spawns_of(&ranks, n), FRAGMENT))
        .collect();
    assert!(rolled.contains(&false) && rolled.contains(&true));
    let lucky = spawns_of(&ranks, cast_where(&ranks, 40, |s| has(s, FRAGMENT)));
    assert!(lucky[0].procs.iter().all(|proc| proc.once));
    let timeline = play(&lucky);
    let fragments = births(&timeline, FRAGMENT);
    assert_eq!(fragments.len(), 5);
    let mut turns: Vec<f32> = fragments.iter().map(|(_, s)| s.angle).collect();
    turns.sort_by(f32::total_cmp);
    assert_eq!(turns, [-20., -10., 0., 10., 20.]);
    assert!(fragments.iter().all(|(_, s)| near(from(s.pos, DUMMY), 8.)));
    assert!(shapes(&timeline, FRAGMENT).any(|s| s.height > 20.));
    assert_eq!(shares(&timeline), [1., 0.6, 0.6, 0.6, 0.6, 0.6]);
}

#[test]
fn double_breeze_may_send_a_plain_breath_at_the_aim_as_the_first_fades() {
    let ranks = [("double_breeze", 5), ("expanding_heat", 3)];
    let rolled = cast_where(&ranks, 40, |s| s[0].trail.is_some());
    let timeline = play(&spawns_of(&ranks, rolled));
    let breaths = births(&timeline, BREATH);
    assert_eq!(breaths.len(), 2);
    let (frame, second) = breaths[1];
    assert_eq!(frame, 21);
    assert!(near(second.pos[0], 16.) && near(second.pos[1], 0.));
    assert!(near(second.scale[1], 1.25) && near(breaths[0].1.scale[1], 2.5625));
    assert_eq!(shares(&timeline), [1., 1.]);
    let orbs = spawns_of(&[("double_breeze", 5), ("molten_orb", 1)], rolled);
    assert_eq!(
        orbs[0].trail.as_ref().map(|t| t.spawn.object.as_str()),
        Some(BREATH)
    );
    assert_eq!(orbs[0].object, ORB);
}

#[test]
fn burst_of_heat_may_set_off_a_small_nova_on_the_burning_dummy() {
    let ranks = [("burst_of_heat", 5)];
    let lucky = spawns_of(&ranks, cast_where(&ranks, 40, |s| has(s, NOVA)));
    let nova = lucky[0].procs.iter().find(|p| p.spawn.object == NOVA);
    assert!(!nova.expect("a nova").once);
    let timeline = play(&lucky);
    let novas = births(&timeline, NOVA);
    assert_eq!(novas.len(), 1);
    assert!(near(from(novas[0].1.pos, DUMMY), 0.));
    let widest = shapes(&timeline, NOVA)
        .map(|s| s.scale[0])
        .fold(0., f32::max);
    assert!(widest < 0.5 && widest > 0.4, "{widest}");
    assert_eq!(shares(&timeline), [1., 0.75]);
}

#[test]
fn chain_of_fire_chains_once_on_a_lone_dummy_whatever_its_rank() {
    for rank in [1, 3] {
        let timeline = play(&spawns(&[("chain_of_fire", rank)]));
        let breaths = births(&timeline, BREATH);
        assert_eq!(breaths.len(), 2, "rank {rank}");
        let chained = breaths[1].1;
        assert!(near(chained.pos[0], 170.) && near(chained.angle, 0.));
        assert_eq!(shares(&timeline), [1., 1.], "rank {rank}");
    }
    let fan = play(&spawns(&[("chain_of_fire", 1), ("hydras_breeze", 1)]));
    assert_eq!(fan.hits.len(), 4);
}

#[test]
fn scorching_hellflame_pulses_on_the_hit_finding_ten_stacks() {
    let ranks = [("scorching_hellflame", 1)];
    let pulses: Vec<Vec<f32>> = (0..11)
        .map(|n| shares(&play(&spawns_of(&ranks, n))))
        .collect();
    assert!(pulses[..10].iter().all(|cast| cast == &[1.]), "{pulses:?}");
    assert_eq!(pulses[10], [1., 16.]);
    let timeline = play(&spawns_of(&ranks, 10));
    let blasts = births(&timeline, BLAST);
    assert_eq!(blasts.len(), 1);
    assert!(near(blasts[0].1.scale[0], 2.75) && near(from(blasts[0].1.pos, DUMMY), 0.));
    // Two breaths a cast pile two stacks: the 6th cast's second breath finds ten.
    let fan = [("scorching_hellflame", 1), ("hydras_breeze", 1)];
    let first = (0..11).find(|&n| shares(&play(&spawns_of(&fan, n))).len() > 2);
    assert_eq!(first, Some(5));
}

#[test]
fn a_pulse_passes_on_only_the_novas_its_carrier_holds() {
    let ranks = [("burst_of_heat", 5), ("scorching_hellflame", 1)];
    let found = |from: u32| {
        let made = parity::rewritten(CLASS, "breath_of_fire", &ranks, 0, |spawns, tree| {
            super::cast(spawns, tree, &super::rolls(tree, |_, _| true), from)
        });
        let mut hit = shares(&play(&made));
        hit.sort_by(f32::total_cmp);
        hit
    };
    // The breath's pulse holds Burst of Heat; the nova's leaves it out (PC02 L0x9e, L0xf4).
    assert_eq!(found(10), [0.75, 0.75, 1., 16.]);
    assert_eq!(found(9), [0.75, 1., 12.]);
}

#[test]
fn molten_orb_flies_out_and_breathes_at_the_dummy_every_hit_frequency() {
    let orb = spawns(&[("molten_orb", 1)]);
    assert_eq!((orb[0].object.as_str(), orb[0].ability), (ORB, true));
    let timeline = play(&orb);
    assert_eq!(births(&timeline, ORB).len(), 1);
    let frames: Vec<usize> = births(&timeline, BREATH).iter().map(|(f, _)| *f).collect();
    assert_eq!(frames, [7, 33, 59, 85, 111]);
    for (_, breath) in births(&timeline, BREATH) {
        let toward = breath.pos[1].atan2(DUMMY[0] - breath.pos[0]).to_degrees();
        let off = (breath.angle - toward).rem_euclid(360.);
        assert!(off.min(360. - off) < 1., "{} {toward}", breath.angle);
    }
    assert_eq!(timeline.hits.len(), 4);
    let end = births(&timeline, BLAST);
    assert_eq!(end.len(), 1);
    assert!(end[0].1.pos[0] > 360., "{:?}", end[0].1.pos);
    let quick = play(&spawns(&[("molten_orb", 3)]));
    let frames: Vec<usize> = births(&quick, BREATH).iter().map(|(f, _)| *f).collect();
    assert_eq!(frames, [7, 24, 41, 58, 75, 92, 109]);
}

#[test]
fn burning_and_debuff_nodes_leave_the_cast_as_it_is() {
    let base = play(&spawns(&[]));
    for node in [
        "burning_haze",
        "burning_lungs",
        "collateral_ignition",
        "breaking_point",
        "reverse_hypothermia",
    ] {
        let cast = spawns(&[(node, 5)]);
        assert!(
            cast[0].procs.is_empty() && cast[0].trail.is_none(),
            "{node}"
        );
        let cast = play(&cast);
        assert_eq!((shares(&cast), &cast.hits), (shares(&base), &base.hits));
    }
}

#[test]
fn only_the_nodes_the_game_reads_its_own_way_keep_their_damage_off_the_hit() {
    let owned = crate::skill_mods::parity_tests::owned("pyromancer", "breath_of_fire");
    assert_eq!(
        owned,
        [
            "heat_combustion",
            "double_breeze",
            "burst_of_heat",
            "scorching_hellflame"
        ]
    );
}

const CLASS: &str = "pyromancer";

use super::parity::expected;

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_weighted(CLASS, "breath_of_fire", ranks)
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!(near(sim, 1.) && near(base, 1.), "sim {sim}, engine {base}");
    let wrong = parity::misses(CLASS, "breath_of_fire", expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_breaths_that_multiply_as_the_sim_plays_them() {
    let together: [&[(&str, u32)]; 14] = [
        &[("hydras_breeze", 1), ("chain_of_fire", 1)],
        &[("hydras_breeze", 2), ("chain_of_fire", 1)],
        &[("hydras_breeze", 3), ("chain_of_fire", 3)],
        &[("hydras_breeze", 2), ("molten_orb", 1)],
        &[("hydras_breeze", 3), ("molten_orb", 3)],
        &[("chain_of_fire", 2), ("molten_orb", 3)],
        &[("scorching_hellflame", 1), ("hydras_breeze", 1)],
        &[("scorching_hellflame", 2), ("molten_orb", 1)],
        &[("scorching_hellflame", 3), ("chain_of_fire", 1)],
        &[("heat_combustion", 5), ("hydras_breeze", 2)],
        &[("heat_combustion", 4), ("molten_orb", 1)],
        &[("burst_of_heat", 5), ("hydras_breeze", 1)],
        &[("burst_of_heat", 3), ("chain_of_fire", 1)],
        &[("double_breeze", 5), ("molten_orb", 2)],
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
fn the_engines_extra_breaths_follow_the_sims_fan() {
    let base = spawns(&[]);
    let hits: Vec<f32> = (1..=7)
        .map(|count| parity::weighted(&parity::play("breath_of_fire", &base, count)))
        .collect();
    assert_eq!(hits, [1., 2., 1., 2., 1., 2., 1.]);
    for (count, sim) in (1..=7).zip(hits) {
        let engine = parity::engine_volley_hits(CLASS, "breath_of_fire", &[], Some(count));
        assert!(
            near(sim, engine),
            "{count} breaths: sim {sim}, engine {engine}"
        );
    }
}
