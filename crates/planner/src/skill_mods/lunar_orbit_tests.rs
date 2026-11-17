use crate::skill_mods::Mods;
use crate::skill_sim::{Shape, Spawn, Timeline, run, uses};
use std::collections::HashMap;
use std::sync::Arc;

const MOON: &str = "Exo_Lunar_Orbit_obj";
const SHRAPNEL: &str = "Exo_Lunar_Orbit_Shrapnel_obj";
const TSUNAMI: &str = "Exo_Tsunami_obj";
const NEBULA: &str = "Exo_Lunar_Orbit_Nebula_obj";
/// The orbit's centre, 6 px below the player, and the engine's dummy 72 px out.
const CENTRE: [f32; 2] = [0., 6.];
const DUMMY: [f32; 2] = [72., 0.];
/// The rings of the player's counter, from the 48 px one the preview starts on.
const RADII: [f32; 5] = [48., 128., 96., 64., 32.];

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("lunar_orbit:{node}"), *rank))
        .collect();
    Mods::of("lunar_orbit", &ranks)
}

fn cast(ranks: &[(&str, u32)]) -> Timeline {
    run("lunar_orbit", &mods(ranks), 1).expect("lunar orbit")
}

fn casts_with(mods: &Mods) -> Arc<[Timeline]> {
    uses("lunar_orbit", mods).expect("lunar orbit uses")
}

fn casts(ranks: &[(&str, u32)]) -> Arc<[Timeline]> {
    casts_with(&mods(ranks))
}

fn spawns_with(mods: &Mods, cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill("lunar_orbit")
        .expect("lunar orbit")
        .spawn;
    crate::skill_mods::apply_cast("lunar_orbit", base, mods, cast).into_owned()
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_with(&mods(ranks), 0)
}

fn shapes<'a>(timeline: &'a Timeline, object: &'a str) -> impl Iterator<Item = &'a Shape> {
    let all = timeline.frames.iter().flatten();
    all.filter(move |shape| shape.object.name == object)
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

fn from_centre(pos: [f32; 2]) -> f32 {
    (pos[0] - CENTRE[0]).hypot(pos[1] - CENTRE[1])
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn shares(timeline: &Timeline) -> Vec<f32> {
    let hits = timeline.effects.damage.iter().filter(|d| !d.frost);
    hits.map(|d| d.share).collect()
}

/// The first use whose timeline shows `object`.
fn showing<'a>(uses: &'a [Timeline], object: &str) -> &'a Timeline {
    let found = uses.iter().find(|u| shapes(u, object).next().is_some());
    found.unwrap_or_else(|| panic!("no {object} in {} uses", uses.len()))
}

fn proc_objects(spawn: &Spawn) -> Vec<&str> {
    spawn
        .procs
        .iter()
        .map(|p| p.spawn.object.as_str())
        .collect()
}

fn proc_of<'a>(spawn: &'a Spawn, object: &str) -> &'a Spawn {
    let found = spawn.procs.iter().find(|p| p.spawn.object == object);
    &found.expect("a proc creating the object").spawn
}

#[test]
fn each_cast_takes_the_next_ring_of_the_players_counter() {
    let uses = casts(&[]);
    assert_eq!(uses.len(), 10);
    for (k, timeline) in uses.iter().enumerate() {
        let moons = births(timeline, MOON);
        assert_eq!(moons.len(), 1);
        let radius = from_centre(moons[0].1.pos);
        assert!((radius - RADII[k % 5]).abs() < 1e-3, "use {k}: {radius}");
    }
    let hits: Vec<usize> = uses.iter().map(|u| u.hits.len()).collect();
    assert_eq!(hits, [3, 0, 6, 4, 0, 3, 0, 6, 4, 0]);
}

#[test]
fn five_casts_average_the_engines_dummy_hits() {
    let uses = casts(&[]);
    let mean = uses[..5].iter().map(|u| u.hits.len()).sum::<usize>() as f64 / 5.;
    let engine = hsplanner_engine::calc::data::get_skills_by_class("exo")
        .iter()
        .find(|skill| skill.id == "lunar_orbit")
        .and_then(|skill| skill.dummy_hits.as_ref())
        .map(|hits| hits.base);
    assert_eq!(Some(mean), engine);
}

#[test]
fn the_moon_circles_6_px_below_the_player_then_shrinks_away_after_5_s() {
    let base = cast(&[]);
    let first = births(&base, MOON)[0].1;
    let (sin, cos) = 3f32.to_radians().sin_cos();
    assert!(near(first.pos[0], 48. * cos) && near(first.pos[1], 6. - 48. * sin));
    assert_eq!((first.scale, first.angle), ([0.75, 0.75], 0.));
    let oldest = shapes(&base, MOON).map(|s| s.age).max();
    assert_eq!(oldest, Some(300 + 2 + 15));
    let last = shapes(&base, MOON).last().expect("a moon");
    assert!(last.scale[0] < 0.06, "{:?}", last.scale);
}

#[test]
fn full_moon_and_skill_duration_stretch_the_moons_life() {
    let oldest = |mods: &Mods| {
        let timeline = run("lunar_orbit", mods, 1).expect("lunar orbit");
        shapes(&timeline, MOON).map(|s| s.age).max()
    };
    assert_eq!(oldest(&mods(&[("full_moon", 5)])), Some(555 + 17));
    let stretched = Mods {
        duration_pct: 50.,
        ..mods(&[("full_moon", 5)])
    };
    assert_eq!(oldest(&stretched), Some(705 + 17));
}

#[test]
fn magnetic_pull_speeds_every_ring_and_so_its_turn_of_the_hit_map() {
    for cast in 0..5 {
        let moon = spawns_with(&mods(&[("magnetic_pull", 4)]), cast).swap_remove(0);
        let orbit = moon.orbit.expect("an orbit");
        let ring = [3., 5., 6., 4., 3.][cast as usize];
        assert!(near(orbit.speed, ring * 1.4), "{}", orbit.speed);
        assert!(orbit.clears && moon.tick == 0.);
    }
    let fast = cast(&[("magnetic_pull", 4)]);
    assert!(fast.hits.len() > cast(&[]).hits.len(), "{:?}", fast.hits);
}

#[test]
fn lunar_architect_casts_three_more_moons_a_rank_on_consecutive_rings() {
    let moons = spawns(&[("lunar_architect", 1)]);
    assert_eq!(moons.len(), 6);
    for (i, moon) in moons.iter().enumerate() {
        let orbit = moon.orbit.expect("an orbit");
        assert_eq!((orbit.radius, orbit.at), (RADII[i % 5], 60. * i as f32));
    }
    let next = spawns_with(&mods(&[("lunar_architect", 1)]), 1);
    assert_eq!(next[0].orbit.map(|orbit| orbit.radius), Some(RADII[1]));
    let six = cast(&[("lunar_architect", 1)]);
    assert_eq!(six.frames[0].len(), 6);
    assert!(six.hits.len() > 6, "{:?}", six.hits);
}

#[test]
fn extrasolar_moons_add_moons_only_with_lunar_architect() {
    assert_eq!(spawns(&[("extrasolar_moons", 4)]).len(), 1);
    let more = spawns(&[("extrasolar_moons", 4), ("lunar_architect", 1)]);
    assert_eq!(more.len(), 10);
}

#[test]
fn loss_of_orbit_may_lunge_a_moon_at_the_dummy_and_back() {
    let moon = spawns(&[("loss_of_orbit", 5)]).swap_remove(0);
    let lunge = moon.orbit.and_then(|orbit| orbit.lunge).expect("a lunge");
    assert!(near(lunge.chance, 0.25));
    assert_eq!((lunge.frames, lunge.speed, lunge.near), (40, 3., 8.));
    let uses = casts(&[("loss_of_orbit", 5), ("lunar_architect", 3)]);
    let strays = |timeline: &Timeline| {
        let radii: Vec<f32> = RADII.to_vec();
        shapes(timeline, MOON)
            .filter(|s| s.age > 2 && s.fade == 0.)
            .any(|s| radii.iter().all(|r| (from_centre(s.pos) - r).abs() > 10.))
    };
    assert!(uses.iter().any(strays), "no moon left its ring");
    assert!(!casts(&[("lunar_architect", 3)]).iter().any(strays));
}

#[test]
fn celestial_collapse_may_burst_all_moons_of_a_cast_into_five_shrapnel() {
    let ranks = [("celestial_collapse", 5), ("lunar_architect", 1)];
    let rolled: Vec<Vec<Spawn>> = (0..10).map(|c| spawns_with(&mods(&ranks), c)).collect();
    let bursting = |moons: &Vec<Spawn>| moons.iter().filter(|m| !m.on_end.is_empty()).count();
    assert!(rolled.iter().all(|moons| [0, 6].contains(&bursting(moons))));
    let cast = rolled.iter().find(|moons| bursting(moons) == 6);
    let shrapnel = &cast.expect("a collapsing cast")[0].on_end;
    let turns: Vec<Option<f32>> = shrapnel.iter().map(|s| s.turn).collect();
    assert_eq!(turns, [0., 72., 144., 216., 288.].map(Some));
    let piece = &shrapnel[0];
    assert_eq!((piece.object.as_str(), piece.share), (SHRAPNEL, Some(2.5)));
    assert_eq!((piece.offset, piece.speed), (8., 4.));
    let uses = casts(&ranks);
    let shown = showing(&uses, SHRAPNEL);
    assert_eq!(births(shown, SHRAPNEL).len() % 5, 0);
}

#[test]
fn tidalwaves_may_send_a_tsunami_from_the_player_at_the_dummy() {
    let moon = spawns(&[("tidalwaves", 8)]).swap_remove(0);
    assert_eq!(proc_objects(&moon), [TSUNAMI]);
    assert!(near(moon.procs[0].chance, 0.1));
    let wave = proc_of(&moon, TSUNAMI);
    assert!(wave.at_player && wave.aim);
    assert_eq!((wave.scale, wave.life), (Some([0.75; 2]), Some(93)));
    assert!(near(wave.share.unwrap_or(0.), 1.1));
    let uses = casts(&[("tidalwaves", 8), ("magnetic_pull", 4), ("full_moon", 5)]);
    let shown = showing(&uses, TSUNAMI);
    let (_, first) = births(shown, TSUNAMI)[0];
    assert!(
        first.pos[0] < 1. && first.pos[1].abs() < 1.,
        "{:?}",
        first.pos
    );
    assert!(near(first.angle, 0.));
}

#[test]
fn devastating_nebula_may_leave_a_growing_nebula_on_the_dummy() {
    let moon = spawns(&[("devastating_nebula", 3)]).swap_remove(0);
    assert_eq!(proc_objects(&moon), [NEBULA]);
    assert!(near(moon.procs[0].chance, 0.3));
    let cloud = proc_of(&moon, NEBULA);
    assert!(cloud.at_enemy && near(cloud.share.unwrap_or(0.), 2.95));
    assert_eq!((cloud.tick, cloud.life, cloud.fade), (30., Some(300), 14));
    let uses = casts(&[("devastating_nebula", 3)]);
    let shown = showing(&uses, NEBULA);
    let (_, first) = births(shown, NEBULA)[0];
    assert_eq!(first.pos, DUMMY);
    let widest = shapes(shown, NEBULA).map(|s| s.scale[0]).fold(0., f32::max);
    assert!(near(widest, 1.66), "{widest}");
    let ticks = shares(shown).iter().filter(|s| near(**s, 2.95)).count();
    assert!(ticks >= 10, "{:?}", shares(shown));
}

#[test]
fn what_a_proc_creates_rolls_the_other_proc_scaled_by_its_own_share() {
    let moon = spawns(&[("tidalwaves", 8), ("devastating_nebula", 3)]).swap_remove(0);
    assert_eq!(proc_objects(&moon), [TSUNAMI, NEBULA]);
    let wave = proc_of(&moon, TSUNAMI);
    assert_eq!(proc_objects(wave), [NEBULA]);
    assert!(near(proc_of(wave, NEBULA).share.unwrap_or(0.), 1.1 * 2.95));
    assert!(proc_of(wave, NEBULA).procs.is_empty());
    let cloud = proc_of(&moon, NEBULA);
    assert_eq!(proc_objects(cloud), [TSUNAMI]);
    assert!(proc_of(cloud, TSUNAMI).procs.is_empty());
}

#[test]
fn crescent_moon_summons_a_moon_every_skill_frequency_on_three_rings() {
    let summons = spawns(&[("crescent_moon", 3)]);
    let frames: Vec<u32> = summons.iter().map(|s| s.frame).collect();
    assert_eq!(frames, (6..300).step_by(15).collect::<Vec<u32>>());
    for (i, moon) in summons.iter().enumerate() {
        let orbit = moon.orbit.expect("an orbit");
        assert_eq!((orbit.radius, orbit.at), ([48., 128., 32.][i % 3], 0.));
        let drain = moon.drain.expect("a drain");
        assert_eq!((drain.first, drain.share), (1, 0.03));
    }
    let timeline = cast(&[("crescent_moon", 3)]);
    assert_eq!(timeline.effects.mana.len(), 20);
}

#[test]
fn crescent_moons_carry_only_what_the_controller_copies() {
    let all = [
        ("crescent_moon", 1),
        ("tidalwaves", 8),
        ("devastating_nebula", 3),
        ("orbit_breakout", 3),
        ("celestial_collapse", 5),
        ("loss_of_orbit", 5),
    ];
    for moon in spawns(&all) {
        assert_eq!(proc_objects(&moon), [TSUNAMI]);
        let orbit = moon.orbit.expect("an orbit");
        assert!(orbit.breakout.is_none() && orbit.lunge.is_some());
        assert!(moon.on_end.is_empty() && moon.ramp.is_none());
    }
}

#[test]
fn crescent_moons_reach_the_160_px_ring_with_seven_moons_a_turn() {
    let ranks = [("crescent_moon", 1), ("extrasolar_moons", 4)];
    let radii: Vec<f32> = spawns(&ranks)
        .iter()
        .filter_map(|moon| moon.orbit.map(|orbit| orbit.radius))
        .collect();
    assert_eq!(radii[..7], [48., 128., 96., 64., 160., 160., 32.]);
}

#[test]
fn orbit_breakout_flies_the_moon_out_and_raises_its_damage() {
    let moon = spawns(&[("orbit_breakout", 3)]).swap_remove(0);
    let breakout = moon
        .orbit
        .and_then(|orbit| orbit.breakout)
        .expect("a breakout");
    assert_eq!(breakout.at, 302);
    assert!(near(breakout.speed, 3. * 3.05), "{}", breakout.speed);
    assert_eq!(moon.life, Some(302 + 180));
    let ramp = moon.ramp.expect("a ramp");
    assert_eq!((ramp.every, ramp.from), (302, 0));
    assert!(near(ramp.rate, 2.55));
    let flown = cast(&[("orbit_breakout", 3)]);
    let out = shapes(&flown, MOON)
        .find(|s| s.age == 400)
        .expect("a moon at 400");
    assert!(from_centre(out.pos) > 800., "{:?}", out.pos);
    assert_eq!(shapes(&flown, MOON).map(|s| s.age).max(), Some(482 + 17));
}

#[test]
fn damage_and_condition_nodes_leave_the_cast_unchanged() {
    let base = cast(&[]);
    let same = cast(&[
        ("tidal_collision", 5),
        ("lunar_cycle", 5),
        ("mass_meets_mass", 5),
        ("heavy_core", 5),
    ]);
    assert_eq!(same.hits, base.hits);
    assert_eq!(same.frames.len(), base.frames.len());
    assert_eq!(shares(&same), shares(&base));
}

#[test]
fn every_node_at_once_still_plays_bounded_uses() {
    let tree = [
        ("tidal_collision", 5),
        ("full_moon", 5),
        ("extrasolar_moons", 4),
        ("magnetic_pull", 4),
        ("loss_of_orbit", 5),
        ("celestial_collapse", 5),
        ("lunar_cycle", 5),
        ("tidalwaves", 8),
        ("mass_meets_mass", 5),
        ("heavy_core", 5),
        ("lunar_architect", 3),
        ("orbit_breakout", 3),
        ("devastating_nebula", 3),
    ];
    for storm in casts(&tree).iter() {
        assert!(storm.hits.len() >= 16, "{:?}", storm.hits);
        assert!(storm.frames.len() < 1800, "{}", storm.frames.len());
    }
}

#[test]
fn the_builds_size_stats_widen_the_moon_and_lengthen_its_shrink() {
    let mods = Mods {
        size_bonus: 0.25,
        ..mods(&[])
    };
    let moon = spawns_with(&mods, 0).swap_remove(0);
    assert_eq!((moon.scale, moon.fade), (Some([1., 1.]), 20));
}
