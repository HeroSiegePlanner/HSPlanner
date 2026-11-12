use super::drops;
use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_motion::Trail;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use};
use std::collections::HashMap;

const SKILL: &str = "blazing_trail";
const CONTROLLER: &str = "Pyromancer_Blazing_Trail_Controller_obj";
const CAST: &str = "TalentsPyromancer_20";
const PATCH: &str = "Pyromancer_Blazing_Trail_obj";
const FORK_STEP: &str = "Pyromancer_Blazing_Trail_obj_Step_0";
const NOVA: &str = "Pyromancer_Firenova_obj";
const COMET: &str = "Pyromancer_Trail_of_Comets_obj";
const COMET_BLAST: &str = "Explosion_Item_obj:Pyromancer_Explosion_spr";
const MINE: &str = "Pyromancer_Blazing_Detonation_Field_obj";
const MINE_BLAST: &str = "Explosion_Item_obj";

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("{SKILL}:{node}"), *rank))
        .collect();
    Mods::of(SKILL, &ranks)
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill(SKILL).expect("blazing trail").spawn;
    crate::skill_mods::apply_cast(SKILL, base, &mods(ranks), cast).into_owned()
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play(spawns: &[Spawn]) -> Timeline {
    let fx = crate::skill_sim::skill(SKILL).expect("blazing trail");
    let hitbox = crate::hitbox::skill(SKILL).expect("blazing trail hitboxes");
    replay_use(fx, hitbox, spawns, 1, 0)
}

fn trail(ranks: &[(&str, u32)]) -> Timeline {
    play(&spawns(ranks))
}

/// The controllers Circle of Fire's cast makes, in the order of their rays.
fn walkers(spawns: &[Spawn]) -> Vec<Spawn> {
    let [cast] = spawns else {
        panic!("one cast carrier");
    };
    assert_eq!(cast.object, CAST);
    cast.on_end.clone()
}

/// Each drop of a controller's trail chain with the moves it is dropped after.
fn dropped(controller: &Spawn) -> Vec<(u32, Spawn)> {
    let chain = std::iter::successors(controller.trail.as_deref(), |t| t.next.as_deref());
    chain.map(|t| (t.first, t.spawn.clone())).collect()
}

/// The first cast among the first 64 whose spawns pass `lucky`.
fn lucky(ranks: &[(&str, u32)], lucky: impl Fn(&[Spawn]) -> bool) -> Vec<Spawn> {
    (0..64)
        .map(|n| spawns_of(ranks, n))
        .find(|spawns| lucky(spawns))
        .expect("a cast that rolls it")
}

/// The patches every controller drops directly.
fn patches(spawns: &[Spawn]) -> Vec<Spawn> {
    let walkers = walkers(spawns);
    let drops = walkers.iter().flat_map(dropped).map(|(_, spawn)| spawn);
    drops.filter(|spawn| spawn.object == PATCH).collect()
}

fn pulses(patch: &Spawn) -> Option<&Trail> {
    let chain = std::iter::successors(patch.trail.as_deref(), |t| t.next.as_deref());
    chain.into_iter().find(|t| t.spawn.object == NOVA)
}

fn fork(patch: &Spawn) -> Option<&Trail> {
    let chain = std::iter::successors(patch.trail.as_deref(), |t| t.next.as_deref());
    chain.into_iter().find(|t| t.spawn.object == FORK_STEP)
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

/// GameMaker's point_direction.
fn toward(from: [f32; 2], to: [f32; 2]) -> f32 {
    (from[1] - to[1]).atan2(to[0] - from[0]).to_degrees()
}

#[test]
fn a_standing_player_leaves_no_patch_for_the_controllers_five_seconds() {
    let base = spawns(&[]);
    let [controller] = base.as_slice() else {
        panic!("one controller");
    };
    assert_eq!(controller.object, CONTROLLER);
    assert_eq!((controller.speed, controller.life), (0., Some(301)));
    let timeline = play(&base);
    assert!(timeline.hits.is_empty(), "{:?}", timeline.hits);
    assert_eq!(timeline.frames.len(), 301);
    assert!(
        timeline
            .frames
            .iter()
            .flatten()
            .all(|s| s.object.name == CONTROLLER)
    );
}

#[test]
fn a_controller_drops_once_over_floor_42_times_its_scale_from_its_last_drop() {
    // Step_0 measures from (x, y + 6) to the last drop's (x, y): 45 px apart along the aim, 50 up,
    // 40 down; Expanded Flames' 1.15 raises the gap to 48.
    assert_eq!(&drops(0., 1., 120)[..3], &[9, 18, 27]);
    assert_eq!(drops(0., 1., 120).len(), 13);
    assert_eq!(&drops(90., 1., 120)[..3], &[10, 20, 30]);
    assert_eq!(&drops(270., 1., 120)[..3], &[8, 16, 24]);
    assert_eq!(&drops(0., 1.15, 120)[..2], &[10, 20]);
    // A Step checks before its move, so the 120th Step's 119 moves are the last that can drop.
    assert_eq!(drops(0., 1., 120).last(), Some(&117));
    assert!(drops(0., 1., 20).iter().all(|&m| m < 20));
}

#[test]
fn timed_fire_stretches_the_controllers_timer() {
    assert_eq!(spawns(&[("timed_fire", 1)])[0].life, Some(331));
    let walkers = walkers(&spawns(&[("circle_of_fire", 1), ("timed_fire", 5)]));
    assert!(walkers.iter().all(|w| w.life == Some(181)));
    let last = dropped(&walkers[0]).last().map(|(moves, _)| *moves);
    assert_eq!(last, Some(171));
}

#[test]
fn circle_of_fire_sends_five_controllers_a_rank_from_zero_degrees() {
    let cast = spawns(&[("circle_of_fire", 1)]);
    assert_eq!(cast[0].layout[0].offset, [0., 6.]);
    let rays = walkers(&cast);
    let headings: Vec<f32> = rays.iter().map(|w| w.turn.unwrap() - 90.).collect();
    assert_eq!(headings, [0., 72., 144., 216., 288.]);
    assert!(rays.iter().all(|w| w.speed == 5. && w.life == Some(121)));
    assert_eq!(walkers(&spawns(&[("circle_of_fire", 3)])).len(), 15);
    let timeline = trail(&[("circle_of_fire", 1)]);
    let along: Vec<[f32; 2]> = births(&timeline, PATCH)
        .into_iter()
        .map(|(_, shape)| shape.pos)
        .filter(|pos| near(pos[1], 6., 0.01))
        .collect();
    for (pos, x) in along.iter().zip([45., 90., 135., 180.]) {
        assert!(near(pos[0], x, 0.01), "{along:?}");
    }
    // Only the 0° ray's patches at 135 and 180 px cover the dummy, six ticks each.
    assert_eq!(timeline.hits.len(), 12, "{:?}", timeline.hits);
}

#[test]
fn expanded_flames_grows_the_patches_and_spreads_them() {
    let walkers = walkers(&spawns(&[("circle_of_fire", 1), ("expanded_flames", 1)]));
    let ray = dropped(&walkers[0]);
    assert_eq!(ray[0].0, 10);
    let max = ray[0].1.grow.and_then(|grow| grow.max);
    assert!(near(max.unwrap(), 1.15, 1e-5));
}

#[test]
fn fueled_flames_shortens_the_patches_tick() {
    let ticks = |rank| patches(&spawns(&[("circle_of_fire", 1), ("fueled_flames", rank)]))[0].tick;
    assert!(near(ticks(1), 27., 1e-4));
    assert!(near(ticks(5), 15., 1e-4));
}

#[test]
fn flaming_steps_buffs_the_player_for_five_seconds() {
    let alone = trail(&[("flaming_steps", 3)]);
    assert_eq!(alone.effects.buffed, Some(([0, 300], 36.)));
    let circle = trail(&[("circle_of_fire", 1), ("flaming_steps", 1)]);
    assert_eq!(circle.effects.buffed, Some(([0, 300], 12.)));
}

#[test]
fn nodes_that_only_burn_break_slow_or_add_damage_change_no_object() {
    let base = trail(&[]);
    for node in [
        "blazing_trail_core",
        "scorch",
        "agonizing_heat",
        "combusting_flames",
        "burning_veins",
        "paralyzing_fire",
    ] {
        let with = trail(&[(node, 1)]);
        assert_eq!(with.frames.len(), base.frames.len(), "{node}");
        assert!(
            with.hits.is_empty() && with.effects.buffed.is_none(),
            "{node}"
        );
    }
    let circle = trail(&[("circle_of_fire", 1)]);
    let burning = trail(&[("circle_of_fire", 1), ("combusting_flames", 5)]);
    assert_eq!(burning.hits, circle.hits);
}

#[test]
fn crackling_heat_pulses_a_nova_on_its_patch_each_second() {
    let ranks = [("circle_of_fire", 1), ("crackling_heat", 5)];
    let spawns = lucky(&ranks, |s| patches(s).iter().any(|p| pulses(p).is_some()));
    let crackling = patches(&spawns).into_iter().find(|p| pulses(p).is_some());
    let pulse = pulses(crackling.as_ref().unwrap()).unwrap();
    assert_eq!((pulse.first, pulse.every), (59, 60));
    assert_eq!(pulse.spawn.grow.and_then(|grow| grow.max), Some(1.));
    let timeline = play(&spawns);
    let patches = births(&timeline, PATCH);
    let novas = births(&timeline, NOVA);
    assert!(!novas.is_empty());
    for (frame, nova) in novas {
        let mother = patches.iter().find(|(_, p)| p.pos == nova.pos);
        let (born, _) = mother.expect("a nova sits on its patch");
        assert!([60, 120].contains(&(frame - born)), "{frame} {born}");
    }
}

/// The first forking patch: its walker's heading, the moves it is dropped after and the patch.
fn forking(spawns: &[Spawn]) -> Option<(f32, u32, Spawn)> {
    walkers(spawns).into_iter().find_map(|walker| {
        let heading = walker.turn? - 90.;
        let found = dropped(&walker)
            .into_iter()
            .find(|(_, s)| fork(s).is_some());
        found.map(|(moves, patch)| (heading, moves, patch))
    })
}

#[test]
fn forking_flames_forks_ten_times_back_toward_the_player() {
    let ranks = [("circle_of_fire", 1), ("forking_flames", 1)];
    let spawns = lucky(&ranks, |s| forking(s).is_some());
    let (heading, moves, patch) = forking(&spawns).unwrap();
    assert!(near(patch.share.unwrap(), 1.45, 1e-5));
    let (sin, cos) = heading.to_radians().sin_cos();
    let at = [5. * moves as f32 * cos, 6. - 5. * moves as f32 * sin];
    // Alarm_0: irandom(180) + point_direction(patch -> player) - 90, so it turns 90 to 270 degrees
    // from the way out.
    let first = fork(&patch).unwrap();
    assert!(
        (90. ..=270.).contains(&first.spawn.turn.unwrap()),
        "{:?}",
        first.spawn.turn
    );
    let along = toward([0., 0.], at) + first.spawn.turn.unwrap();
    let mut children = Vec::new();
    let mut next = Some(first.clone());
    while let Some(trail) = next {
        assert_eq!(trail.first, 13);
        assert!(
            near(trail.spawn.offset, 48., 1e-3),
            "{}",
            trail.spawn.offset
        );
        let child = trail.spawn.on_end[0].clone();
        assert!(near(child.share.unwrap(), 1.45, 1e-5));
        next = fork(&child).cloned();
        children.push(child);
    }
    assert_eq!(children.len(), 10);
    let timeline = play(&spawns);
    let born = births(&timeline, PATCH);
    let (sin, cos) = along.to_radians().sin_cos();
    for k in 1..=10u32 {
        let spot = [at[0] + 48. * k as f32 * cos, at[1] - 48. * k as f32 * sin];
        let frame = moves as usize + 2 + 15 * k as usize;
        let found = born
            .iter()
            .any(|(f, s)| *f == frame && near(s.pos[0], spot[0], 0.05));
        assert!(found, "fork {k} at {spot:?} in frame {frame}");
    }
}

#[test]
fn trail_of_comets_drops_comets_landing_in_an_explosion_and_a_patch() {
    let walkers = walkers(&spawns(&[
        ("circle_of_fire", 1),
        ("trail_of_comets", 1),
        ("detonation_field", 1),
    ]));
    let (_, comet) = dropped(&walkers[0]).swap_remove(0);
    assert_eq!(comet.object, COMET);
    let fall = comet.fall.expect("a falling comet");
    assert_eq!((fall.height, fall.gravity), (320., 0.25));
    let ends: Vec<&str> = comet.on_end.iter().map(|s| s.object.as_str()).collect();
    assert_eq!(ends, [COMET_BLAST, PATCH]);
    assert_eq!(comet.on_end[0].scale, Some([3.5, 3.5]));
    let timeline = trail(&[("circle_of_fire", 1), ("trail_of_comets", 1)]);
    let blasts = births(&timeline, COMET_BLAST);
    let first = blasts
        .iter()
        .find(|(_, s)| near(s.pos[0], 45., 0.01) && near(s.pos[1], 6., 0.01));
    // The 0° ray's first comet is made 10 frames in and lands on its 52nd Step.
    assert_eq!(first.map(|(frame, _)| *frame), Some(63));
    assert!(!timeline.hits.is_empty());
}

#[test]
fn detonation_field_mines_go_off_two_seconds_after_finding_the_dummy_else_at_ten() {
    let walkers = walkers(&spawns(&[("circle_of_fire", 1), ("detonation_field", 1)]));
    let ray = dropped(&walkers[0]);
    let lives: Vec<(u32, Option<u32>)> = ray.iter().map(|(moves, s)| (*moves, s.life)).collect();
    assert!(ray.iter().all(|(_, s)| s.object == MINE));
    // On the 0° ray the mines at 90, 135, 180 and 225 px are within 48 px of the dummy's box.
    let armed: Vec<u32> = lives
        .iter()
        .filter(|(_, l)| *l == Some(122))
        .map(|(m, _)| *m)
        .collect();
    assert_eq!(armed, [18, 27, 36, 45]);
    assert!(
        lives
            .iter()
            .all(|(_, l)| [Some(122), Some(601)].contains(l))
    );
    let blast = &ray[0].1.on_end[0];
    assert_eq!(
        (blast.object.as_str(), blast.scale),
        (MINE_BLAST, Some([4., 4.]))
    );
    let timeline = trail(&[("circle_of_fire", 1), ("detonation_field", 1)]);
    assert!(
        births(&timeline, MINE_BLAST)
            .iter()
            .any(|(frame, _)| *frame == 19 + 122 + 1)
    );
}

#[test]
fn only_the_nodes_the_game_reads_its_own_way_keep_their_damage_off_the_hit() {
    let owned = crate::skill_mods::parity_tests::owned("pyromancer", "blazing_trail");
    assert_eq!(owned, ["forking_flames"]);
}

const CLASS: &str = "pyromancer";

/// Damage-weighted hits the game lands on the dummy in one use, uncapped, the rolls at their odds.
fn expected(ranks: &[(&str, u32)]) -> f32 {
    super::parity::uncapped(ranks)
}

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_weighted(CLASS, SKILL, ranks)
}

fn disagree(together: &[Vec<(&str, u32)>]) -> Vec<String> {
    let off = together.iter().filter_map(|ranks| {
        let (sim, engine) = (expected(ranks), engine(ranks));
        ((sim - engine).abs() >= 0.01)
            .then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
    });
    off.collect()
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!(sim == 0. && base == 0., "sim {sim}, engine {base}");
    let wrong = parity::misses(CLASS, SKILL, expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_every_node_with_circle_of_fires_controllers() {
    // A standing player drops nothing: every node needs Circle of Fire's walking controllers.
    let mut together = Vec::new();
    for circle in 1..=3 {
        let walking = ("circle_of_fire", circle);
        for node in parity::nodes(CLASS, SKILL) {
            let others = (1..=node.max_rank).filter(|_| node.id != walking.0);
            together.extend(others.map(|rank| vec![walking, (node.id.as_str(), rank)]));
        }
        // Controller Step_0 L0x10-L0x13: comets win over mines.
        together.push(vec![
            walking,
            ("trail_of_comets", 1),
            ("detonation_field", 3),
        ]);
    }
    let wrong = disagree(&together);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_nodes_the_game_plays_together() {
    let mut together = Vec::new();
    for circle in 1..=3 {
        let walking = ("circle_of_fire", circle);
        // Blazing_Trail_obj Alarm_0 L0x1b: every fork rolls Crackling Heat as its patch did.
        for fork in 1..=3 {
            let crackling = (1..=5).map(|rank| ("crackling_heat", rank));
            together.extend(crackling.map(|k| vec![walking, ("forking_flames", fork), k]));
        }
        together.push(vec![walking, ("fueled_flames", 5), ("forking_flames", 3)]);
        together.push(vec![walking, ("expanded_flames", 3), ("fueled_flames", 4)]);
        together.push(vec![walking, ("crackling_heat", 5), ("trail_of_comets", 2)]);
    }
    let wrong = disagree(&together);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
