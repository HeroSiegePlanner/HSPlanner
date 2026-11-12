use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use, uses};
use std::collections::HashMap;

const COMET: &str = "Pyromancer_Comet_obj";
const BLAST: &str = "Explosion_Item_obj";
const BOUNCE_BLAST: &str = "Explosion_Item_obj:Hit_Impact_Fire_spr";
const NOVA: &str = "Pyromancer_Firenova_obj";
const PATCH: &str = "Pyromancer_Blazing_Trail_obj";
const SHRAPNEL: &str = "Marksman_Frag_Grenade_Shrapnel_obj";
const CURSOR: [f32; 2] = [160., -30.];

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("comet:{node}"), *rank))
        .collect();
    Mods::of("comet", &ranks)
}

fn spawns_with(mods: &Mods, cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill("comet").expect("comet").spawn;
    crate::skill_mods::apply_cast("comet", base, mods, cast).into_owned()
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    spawns_with(&mods(ranks), cast)
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play(spawns: &[Spawn]) -> Timeline {
    let fx = crate::skill_sim::skill("comet").expect("comet");
    let hitbox = crate::hitbox::skill("comet").expect("comet hitboxes");
    replay_use(fx, hitbox, spawns, 1, 0)
}

/// The spawns of the first cast among the first `tries` whose spawns pass `lucky`.
fn lucky(ranks: &[(&str, u32)], tries: u64, lucky: impl Fn(&[Spawn]) -> bool) -> Vec<Spawn> {
    (0..tries)
        .map(|n| spawns_of(ranks, n))
        .find(|spawns| lucky(spawns))
        .expect("a cast that rolls it")
}

/// Whether some comet of the cast leaves `object` where it lands.
fn lands(spawns: &[Spawn], object: &str) -> bool {
    spawns
        .iter()
        .any(|comet| comet.on_end.iter().any(|end| end.object == object))
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

fn shapes<'a>(timeline: &'a Timeline, object: &'a str) -> impl Iterator<Item = &'a Shape> {
    let all = timeline.frames.iter().flatten();
    all.filter(move |shape| shape.object.name == object)
}

fn widest(timeline: &Timeline, object: &str) -> f32 {
    shapes(timeline, object)
        .map(|s| s.scale[0])
        .fold(0., f32::max)
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

/// GameMaker's lengthdir from `at`.
fn out(at: [f32; 2], length: f32, degrees: f32) -> [f32; 2] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [at[0] + cos * length, at[1] - sin * length]
}

/// point_direction from the player to the cursor point, along which bouncing comets hop.
fn aim() -> f32 {
    30f32.atan2(160.).to_degrees()
}

#[test]
fn the_base_use_is_one_comet_exploding_once_on_the_cursor() {
    let base = spawns(&[]);
    assert_eq!(base.len(), 1);
    let timeline = play(&base);
    let comets = births(&timeline, COMET);
    assert_eq!(comets.len(), 1);
    assert_eq!((comets[0].1.pos, comets[0].1.height), (CURSOR, 320.));
    // sqrt(2 × 320 / 0.25): it lands on frame 51 and explodes from the next.
    assert_eq!(frames_of(&timeline, BLAST), [52]);
    assert!(near(widest(&timeline, BLAST), 2.2));
    assert_eq!(shares(&timeline), [1.]);
    assert_eq!(uses("comet", &mods(&[])).expect("uses").len(), 8);
}

#[test]
fn fiery_shower_drops_two_more_comets_a_rank_on_the_same_spot_each_from_higher_up() {
    let timeline = play(&spawns(&[("fiery_shower", 2)]));
    let comets = births(&timeline, COMET);
    let heights: Vec<f32> = comets.iter().map(|(_, s)| s.height).collect();
    assert_eq!(heights, [320., 400., 480., 560., 640.]);
    assert!(comets.iter().all(|(_, s)| s.pos == CURSOR));
    assert_eq!(births(&timeline, BLAST).len(), 5);
    assert_eq!(shares(&timeline), [1.; 5]);
}

#[test]
fn blast_radius_and_crushing_comet_widen_the_comet_and_its_explosion() {
    let wide = play(&spawns(&[("blast_radius", 5)]));
    assert!(near(widest(&wide, COMET), 1.75));
    assert!(near(widest(&wide, BLAST), 1.75 * 2.2));
    let crushing = play(&spawns(&[("blast_radius", 5), ("crushing_comet", 3)]));
    assert!(near(widest(&crushing, BLAST), 2.65 * 2.2));
    let mut sized = mods(&[]);
    sized.size_bonus = 0.2;
    let bonus = play(&spawns_with(&sized, 0));
    assert!(near(widest(&bonus, COMET), 1.2) && near(widest(&bonus, BLAST), 2.64));
}

#[test]
fn crushing_comet_drops_the_comet_with_twice_the_gravity() {
    // L0x3fb: gravityPower 0.5, so it lands on frame 36 instead of 51.
    let timeline = play(&spawns(&[("crushing_comet", 1)]));
    assert_eq!(frames_of(&timeline, BLAST), [37]);
    assert_eq!(shares(&timeline), [1.]);
    let bouncing = play(&spawns(&[("crushing_comet", 1), ("bouncing_comets", 1)]));
    assert_eq!(frames_of(&bouncing, BOUNCE_BLAST)[..2], [37, 37]);
}

#[test]
fn extra_projectiles_fall_on_the_cursor_point_from_higher_up() {
    let fx = crate::skill_sim::skill("comet").expect("comet");
    let hitbox = crate::hitbox::skill("comet").expect("comet hitboxes");
    let timeline = replay_use(fx, hitbox, &spawns(&[("fiery_shower", 1)]), 2, 0);
    let heights: Vec<f32> = births(&timeline, COMET)
        .iter()
        .map(|(_, s)| s.height)
        .collect();
    assert_eq!(heights, [320., 400., 480., 560.]);
    assert_eq!(timeline.hits.len(), 4);
}

#[test]
fn molten_starfall_rings_small_comets_round_one_on_the_cursor() {
    let timeline = play(&spawns(&[("molten_starfall", 1), ("fiery_shower", 3)]));
    let comets = births(&timeline, COMET);
    assert_eq!(comets.len(), 4);
    assert!(
        comets
            .iter()
            .all(|(_, s)| s.height == 320. && near(s.scale[0], 0.33))
    );
    assert_eq!(comets[0].1.pos, CURSOR);
    for (i, (_, comet)) in comets.iter().enumerate().skip(1) {
        let ring = out(CURSOR, 96., 120. * i as f32);
        assert!(from(comet.pos, ring) < 1e-3, "{:?}", comet.pos);
    }
    // Only the comet on the cursor reaches the dummy.
    assert_eq!(shares(&timeline), [1.]);
}

#[test]
fn bouncing_comets_fan_out_and_hop_three_times_along_the_aim() {
    let timeline = play(&spawns(&[("bouncing_comets", 1)]));
    let comets = births(&timeline, COMET);
    assert!(comets.iter().all(|(_, s)| near(s.scale[0], 0.5)));
    let first = out(CURSOR, 54., aim() - 27.);
    assert!(from(comets[0].1.pos, first) < 1e-3, "{:?}", comets[0].1.pos);
    assert!(from(comets[1].1.pos, out(CURSOR, 54., aim() + 27.)) < 1e-3);
    let bounces = births(&timeline, BOUNCE_BLAST);
    assert_eq!(bounces.len(), 6);
    assert!(bounces.iter().all(|(_, s)| near(s.scale[0], 1.75)));
    let ends = births(&timeline, BLAST);
    assert_eq!(ends.len(), 2);
    assert!(ends.iter().all(|(_, s)| near(s.scale[0], 1.1)));
    // Comet 0 hops 64, 49 and 33 frames at 4, 3 and 2.25 px a frame along the aim.
    let second = out(first, 256., aim());
    let third = out(second, 147., aim());
    let last = out(third, 74.25, aim());
    let at = |spot: [f32; 2], born: &[(usize, &Shape)]| {
        born.iter().any(|(_, s)| from(s.pos, spot) < 0.01)
    };
    assert!(at(first, &bounces) && at(second, &bounces) && at(third, &bounces));
    assert!(at(last, &ends));
    // 54 px out on the fan, no landing reaches the dummy 30 px under the cursor point; with six
    // comets the two fanned down hit it on their first landing.
    assert!(timeline.hits.is_empty());
    let six = play(&spawns(&[("bouncing_comets", 3)]));
    assert_eq!(shares(&six), [1., 1.]);
}

#[test]
fn comet_fragments_may_rain_three_small_comets_on_the_landing_spot() {
    let ranks = [("comet_fragments", 5)];
    let rolled: Vec<bool> = (0..40)
        .map(|n| spawns_of(&ranks, n)[0].on_end.len() > 1)
        .collect();
    assert!(rolled.contains(&true) && rolled.contains(&false));
    let timeline = play(&lucky(&ranks, 60, |s| lands(s, COMET)));
    let comets = births(&timeline, COMET);
    assert_eq!(comets.len(), 4);
    let fragments = &comets[1..];
    assert!(fragments.iter().all(|(f, s)| *f == 52 && s.pos == CURSOR));
    let heights: Vec<f32> = fragments.iter().map(|(_, s)| s.height).collect();
    assert_eq!(heights, [260., 320., 380.]);
    assert!(fragments.iter().all(|(_, s)| near(s.scale[0], 0.25)));
    let blasts = births(&timeline, BLAST);
    assert_eq!(blasts.len(), 4);
    assert!(blasts[1..].iter().all(|(_, s)| near(s.scale[0], 0.55)));
    // Their 17.6 px explosions fall short of the dummy 30 px under the cursor point.
    assert_eq!(shares(&timeline), [1.]);
}

#[test]
fn a_fragment_deals_a_third_of_the_comets_damage() {
    let spawns = lucky(&[("comet_fragments", 5)], 60, |s| lands(s, COMET));
    let fragment = spawns[0].on_end.iter().find(|s| s.object == COMET);
    let blast = &fragment.expect("a fragment").on_end[0];
    assert_eq!(blast.object, BLAST);
    assert!(blast.share.is_some_and(|share| near(share, 1. / 3.)));
}

#[test]
fn molten_aftershock_may_pulse_a_firenova_and_raise_that_casts_damage() {
    let ranks = [("molten_aftershock", 5), ("molten_rock", 5)];
    let timeline = play(&lucky(&ranks, 80, |s| lands(s, NOVA)));
    let novas = births(&timeline, NOVA);
    assert_eq!(novas.len(), 1);
    assert_eq!((novas[0].0, novas[0].1.pos), (52, CURSOR));
    let max = widest(&timeline, NOVA);
    assert!(max > 0.85 && max < 0.9, "{max}");
    // L0x3d8: LSSD + 35 % a rank on top of Molten Rock's 40 %.
    let share = 315. / 140.;
    assert!(shares(&timeline).iter().all(|s| near(*s, share)));
    assert_eq!(shares(&timeline).len(), 2);
    let plain = play(&lucky(&ranks, 80, |s| !lands(s, NOVA)));
    assert_eq!(shares(&plain), [1.]);
}

#[test]
fn flaming_terrain_may_leave_a_blazing_patch_on_the_landing_spot() {
    let ranks = [("flaming_terrain", 4)];
    let spawns = lucky(&ranks, 80, |s| lands(s, PATCH));
    let patch = spawns[0].on_end.iter().find(|s| s.object == PATCH);
    assert!(patch.is_some_and(|p| p.share.is_some_and(|share| near(share, 1.4))));
    let timeline = play(&spawns);
    let patches = births(&timeline, PATCH);
    assert_eq!((patches.len(), patches[0].0), (1, 52));
    assert_eq!(patches[0].1.pos, CURSOR);
    // Its mask ends 18 px under the cursor point, short of the dummy.
    assert_eq!(shares(&timeline), [1.]);
}

#[test]
fn searing_shrapnel_may_throw_eight_pieces_out_of_the_explosion() {
    let ranks = [("searing_shrapnel", 5)];
    let timeline = play(&lucky(&ranks, 40, |s| lands(s, SHRAPNEL)));
    let pieces = births(&timeline, SHRAPNEL);
    assert_eq!(pieces.len(), 8);
    for (i, (frame, piece)) in pieces.iter().enumerate() {
        let direction = 45. * i as f32;
        let off = (piece.angle - direction + 180.).rem_euclid(360.) - 180.;
        assert_eq!(*frame, 52);
        assert!(off.abs() < 1e-3, "{i}: {}", piece.angle);
        assert!(from(piece.pos, out(CURSOR, 8., direction)) < 1e-3, "{i}");
    }
    // The piece thrown down at the dummy hits it with 0.3 × rank of the damage.
    let mut hit = shares(&timeline);
    hit.sort_by(f32::total_cmp);
    assert_eq!(hit, [1., 1.5]);
}

#[test]
fn fragments_carry_the_casts_other_landing_effects_but_not_their_own() {
    let ranks = [
        ("comet_fragments", 5),
        ("molten_aftershock", 5),
        ("flaming_terrain", 5),
    ];
    let all = |s: &[Spawn]| lands(s, COMET) && lands(s, NOVA) && lands(s, PATCH);
    let timeline = play(&lucky(&ranks, 4000, all));
    assert_eq!(births(&timeline, COMET).len(), 4);
    assert_eq!(births(&timeline, NOVA).len(), 4);
    assert_eq!(births(&timeline, PATCH).len(), 4);
}

#[test]
fn every_bounce_lands_the_casts_effects_again() {
    let ranks = [("bouncing_comets", 1), ("molten_aftershock", 5)];
    let timeline = play(&lucky(&ranks, 80, |s| lands(s, NOVA)));
    assert_eq!(births(&timeline, NOVA).len(), 8);
    assert!(births(&timeline, SHRAPNEL).is_empty());
}

#[test]
fn damage_and_cast_rate_nodes_leave_the_use_as_it_is() {
    let base = play(&spawns(&[]));
    for node in [
        "comet_core",
        "molten_rock",
        "skycaller",
        "shattering_impact",
        "stunning_hit",
        "cracked_terrain",
    ] {
        let cast = play(&spawns(&[(node, 5)]));
        assert_eq!(
            (shares(&cast), &cast.hits),
            (shares(&base), &base.hits),
            "{node}"
        );
    }
    let every = mods(&[
        ("comet_fragments", 5),
        ("molten_aftershock", 5),
        ("searing_shrapnel", 5),
        ("flaming_terrain", 5),
        ("bouncing_comets", 3),
        ("molten_starfall", 3),
    ]);
    assert!(uses("comet", &every).is_some());
}

#[test]
fn only_the_nodes_the_game_reads_its_own_way_keep_their_damage_off_the_hit() {
    let owned = crate::skill_mods::parity_tests::owned("pyromancer", "comet");
    // Molten Aftershock's 35r% joins the whole cast's skill damage, which the engine averages.
    assert_eq!(
        owned,
        ["comet_fragments", "searing_shrapnel", "flaming_terrain"]
    );
}

const CLASS: &str = "pyromancer";
const SKILL: &str = "comet";
/// The cast's rolls whose objects deal damage, each its own draw (L0x3ce-L0x3e4).
const ROLLED: [&str; 4] = [
    "comet_fragments",
    "molten_aftershock",
    "flaming_terrain",
    "searing_shrapnel",
];

/// Damage-weighted hits a cast is expected to land, each node's roll at its own odds.
fn expected(ranks: &[(&str, u32)]) -> f32 {
    let rolled: Vec<(&str, f32)> = (ranks.iter())
        .filter(|(node, _)| ROLLED.contains(node))
        .map(|&(node, rank)| (node, parity::cast_odds(CLASS, SKILL, &[(node, rank)])))
        .collect();
    let corner = |mask: usize| {
        let on = |i: usize| mask >> i & 1 == 1;
        let odds = rolled.iter().enumerate();
        let weight: f32 = odds
            .map(|(i, (_, p))| if on(i) { *p } else { 1. - p })
            .product();
        let lucky = |node: &str| rolled.iter().position(|(n, _)| *n == node).is_some_and(on);
        let made = parity::rewritten(CLASS, SKILL, ranks, 0, |spawns, tree| {
            super::apply_rolled(spawns, tree, &lucky)
        });
        weight * parity::expected(SKILL, &made)
    };
    (0..1 << rolled.len()).map(corner).sum()
}

/// The engine averages Molten Aftershock's roll, which the sim draws as a share of the whole
/// cast, into its hit as fire skill damage.
fn engine(ranks: &[(&str, u32)]) -> f32 {
    let aftershock = parity::engine_skill_roll(CLASS, "comet", ranks, "molten_aftershock");
    parity::engine_weighted(CLASS, "comet", ranks) * aftershock
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!(near(sim, 1.) && near(base, 1.), "sim {sim}, engine {base}");
    let wrong = parity::misses(CLASS, "comet", expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_procs_and_transformations_the_sim_plays_together() {
    let mut together: Vec<[(&str, u32); 2]> = Vec::new();
    for bouncing in [1, 2, 3] {
        for rank in [1, 3, 5] {
            for node in ["comet_fragments", "flaming_terrain", "searing_shrapnel"] {
                together.push([(node, rank), ("bouncing_comets", bouncing)]);
            }
        }
    }
    together.extend([
        [("searing_shrapnel", 5), ("fiery_shower", 3)],
        [("searing_shrapnel", 2), ("fiery_shower", 1)],
        [("molten_starfall", 2), ("fiery_shower", 3)],
        [("bouncing_comets", 2), ("fiery_shower", 3)],
        [("molten_starfall", 3), ("blast_radius", 5)],
    ]);
    for ranks in &together {
        let (sim, engine) = (expected(ranks), engine(ranks));
        assert!(
            (sim - engine).abs() < 0.01,
            "{ranks:?}: sim {sim}, engine {engine}"
        );
    }
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

/// Every pair of `procs` at every rank of each.
fn all_pairs(procs: &[&'static str]) -> Vec<Vec<(&'static str, u32)>> {
    let mut together = Vec::new();
    for (i, a) in procs.iter().enumerate() {
        for b in &procs[i + 1..] {
            together.extend(pairs(a, b));
        }
    }
    together
}

/// The rolls the engine follows past Bouncing Comets' landings; their novas are not modelled.
const BOUNCED: [&str; 3] = ["comet_fragments", "flaming_terrain", "searing_shrapnel"];

/// `sets`, each with Bouncing Comets at every rank.
fn bouncing(sets: &[Vec<(&'static str, u32)>]) -> Vec<Vec<(&'static str, u32)>> {
    let with = |set: &[(&'static str, u32)], rank| [set, &[("bouncing_comets", rank)]].concat();
    (sets.iter())
        .flat_map(|set| (1..=3).map(move |rank| with(set, rank)))
        .collect()
}

#[test]
fn the_engine_follows_the_procs_that_roll_each_other() {
    let together = [all_pairs(&ROLLED), bouncing(&all_pairs(&BOUNCED))].concat();
    let wrong: Vec<String> = (together.iter())
        .filter_map(|ranks| {
            let (sim, engine) = (expected(ranks), engine(ranks));
            let off = (sim - engine).abs() >= 0.01;
            off.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
        })
        .collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_chains_three_and_more_procs_as_the_sim_does() {
    let bounced = bouncing(&parity::proc_sets(CLASS, SKILL, &BOUNCED));
    let sets = [parity::proc_sets(CLASS, SKILL, &ROLLED), bounced].concat();
    let wrong = parity::set_misses(&sets, expected, engine);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
