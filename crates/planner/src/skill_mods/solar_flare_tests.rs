use crate::skill_mods::Mods;
use crate::skill_sim::{Shape, Spawn, Timeline, run, uses};
use std::collections::HashMap;
use std::sync::Arc;

const FLARE: &str = "Exo_Solar_Flare_obj";
const BLAST: &str = "Explosion_Item_obj";
const GRAND: &str = "Exo_Solar_Flare_Grand_Flare_obj";
const ORB: &str = "Exo_Solar_Flare_Solar_Orb_obj";
const DUMMY: [f32; 2] = [160., 0.];

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("solar_flare:{node}"), *rank))
        .collect();
    Mods::of("solar_flare", &ranks)
}

fn cast(ranks: &[(&str, u32)]) -> Timeline {
    run("solar_flare", &mods(ranks), 1).expect("solar flare")
}

fn casts(ranks: &[(&str, u32)]) -> Arc<[Timeline]> {
    uses("solar_flare", &mods(ranks)).expect("solar flare uses")
}

fn spawns_with(mods: &Mods, cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill("solar_flare")
        .expect("solar flare")
        .spawn;
    crate::skill_mods::apply_cast("solar_flare", base, mods, cast).into_owned()
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_with(&mods(ranks), 0)
}

fn shapes<'a>(timeline: &'a Timeline, object: &'a str) -> impl Iterator<Item = &'a Shape> {
    let all = timeline.frames.iter().flatten();
    all.filter(move |shape| shape.object.name == object && shape.scale[0] > 0.)
}

/// The first drawn shape of every seen instance of `object`, with the frame it is drawn in.
fn births<'a>(timeline: &'a Timeline, object: &'a str) -> Vec<(usize, &'a Shape)> {
    timeline
        .frames
        .iter()
        .enumerate()
        .flat_map(|(frame, shapes)| shapes.iter().map(move |shape| (frame, shape)))
        .filter(|(_, shape)| shape.object.name == object && shape.age == 1)
        .filter(|(_, shape)| shape.scale[0] > 0.)
        .collect()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn lengthdir(length: f32, degrees: f32) -> [f32; 2] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [cos * length, -sin * length]
}

fn from_dummy(pos: [f32; 2]) -> f32 {
    (pos[0] - DUMMY[0]).hypot(pos[1] - DUMMY[1])
}

fn shares(timeline: &Timeline) -> Vec<f32> {
    timeline.effects.damage.iter().map(|d| d.share).collect()
}

/// The flares the cast's unseen flare ends in.
fn cast_flares(spawns: &[Spawn]) -> &[Spawn] {
    &spawns[0].on_end
}

#[test]
fn the_cast_fans_three_flares_24_px_out_12_degrees_apart_each_a_bit_faster() {
    let base = cast(&[]);
    let flares = births(&base, FLARE);
    assert_eq!(flares.len(), 3);
    for ((frame, flare), (turn, speed)) in flares.iter().zip([(-12., 4.05), (0., 4.3), (12., 4.55)])
    {
        assert_eq!(*frame, 1);
        let at = lengthdir(24., turn);
        assert!(
            near(flare.pos[0], at[0]) && near(flare.pos[1], at[1]),
            "{:?}",
            flare.pos
        );
        assert!(near(flare.angle, turn), "{}", flare.angle);
        let moved = base.frames[frame + 1]
            .iter()
            .find(|s| s.object.name == FLARE && near(s.pos[0] - at[0], lengthdir(speed, turn)[0]))
            .expect("the flare's first move at its own speed");
        assert!(near(moved.pos[1] - at[1], lengthdir(speed, turn)[1]));
    }
}

#[test]
fn every_flare_homes_onto_the_dummy_and_explodes_where_its_centre_meets_it() {
    let base = cast(&[]);
    assert_eq!(base.hits.len(), 3, "{:?}", base.hits);
    let blasts = births(&base, BLAST);
    assert_eq!(blasts.len(), 3);
    assert!(blasts.iter().all(|(_, blast)| from_dummy(blast.pos) <= 12.));
    let on_the_dummy = shapes(&base, FLARE).filter(|s| from_dummy(s.pos) <= 12.);
    assert_eq!(on_the_dummy.count(), 3);
    assert_eq!(shares(&base), [1.; 3]);
}

#[test]
fn solar_fireworks_and_searing_flares_add_flares_to_the_fan() {
    let more = cast(&[("solar_fireworks", 4)]);
    assert_eq!(births(&more, FLARE).len(), 7);
    assert_eq!(more.hits.len(), 7);
    let turns: Vec<f32> = cast_flares(&spawns(&[("solar_fireworks", 1)]))
        .iter()
        .filter_map(|flare| flare.turn)
        .collect();
    assert_eq!(turns, [-18., -6., 6., 18.]);
    let searing = cast(&[("searing_flares", 3)]);
    assert_eq!(births(&searing, FLARE).len(), 12);
    assert_eq!(searing.hits.len(), 12);
}

#[test]
fn suns_orbit_spirals_its_flares_out_from_24_px_for_3_s() {
    let flares = spawns(&[("suns_orbit", 1)]);
    let flares = cast_flares(&flares);
    assert_eq!(flares.len(), 5);
    for (i, flare) in flares.iter().enumerate() {
        let orbit = flare.orbit.expect("an orbit");
        assert_eq!(flare.turn, Some(72. * i as f32));
        assert!(near(orbit.speed, 4.05 + 0.25 * i as f32), "{}", orbit.speed);
        assert_eq!(
            (orbit.radius, orbit.below, orbit.spread, orbit.face),
            (24., 6., 1.5, 90.)
        );
        assert_eq!((flare.life, flare.speed), (Some(180), 0.));
    }
    let timeline = cast(&[("suns_orbit", 1)]);
    let born = births(&timeline, FLARE);
    assert_eq!(born.len(), 5);
    let second = shapes(&timeline, FLARE)
        .find(|s| s.age == 2)
        .expect("a circling flare");
    let radius = second.pos[0].hypot(second.pos[1] - 6.);
    assert!(near(radius, 25.5), "{radius}");
    assert!(shapes(&timeline, FLARE).all(|s| s.age <= 180));
    assert!(
        (1..=5).contains(&timeline.hits.len()),
        "{:?}",
        timeline.hits
    );
}

#[test]
fn grand_flare_flies_a_flare_three_times_as_big_and_rings_out_the_flares_four_times() {
    let timeline = cast(&[("grand_flare", 1)]);
    let grand: Vec<&Shape> = shapes(&timeline, GRAND).collect();
    assert_eq!(grand[0].pos, [10., 0.]);
    assert_eq!(grand[0].scale, [3., 3.]);
    assert!(near(grand[10].pos[0], 50.), "{:?}", grand[10].pos);
    assert_eq!(grand.len(), 120);
    let rings: Vec<usize> = births(&timeline, FLARE).iter().map(|(f, _)| *f).collect();
    assert_eq!(rings, [15, 15, 15, 45, 45, 45, 75, 75, 75, 105, 105, 105]);
    let first: Vec<&Shape> = births(&timeline, FLARE)
        .into_iter()
        .take(3)
        .map(|(_, s)| s)
        .collect();
    for (i, flare) in first.iter().enumerate() {
        let out = lengthdir(10., 120. * i as f32);
        assert!(near(flare.pos[0], 70. + out[0]) && near(flare.pos[1], out[1]));
    }
    assert_eq!(timeline.hits.len(), 12, "{:?}", timeline.hits);
}

#[test]
fn solar_orbs_set_suns_that_ring_out_flares_every_1_25_s_for_their_seconds() {
    let one = cast(&[("solar_orbs", 1)]);
    let suns: Vec<&Shape> = shapes(&one, ORB).collect();
    assert_eq!((suns[0].pos, suns[0].scale), ([32., 0.], [0.75, 0.75]));
    assert_eq!(suns.len(), 180);
    let rings: Vec<usize> = births(&one, FLARE).iter().map(|(f, _)| *f).collect();
    assert_eq!(rings, [15, 15, 15, 90, 90, 90, 165, 165, 165]);
    assert_eq!(one.hits.len(), 9);
    let two = cast(&[("solar_orbs", 2)]);
    let places: Vec<[f32; 2]> = births(&two, ORB).iter().map(|(_, s)| s.pos).collect();
    assert!(
        near(places[1][0], -32.) && near(places[1][1], 0.),
        "{places:?}"
    );
    assert_eq!(two.hits.len(), 30);
}

#[test]
fn solar_orbs_win_over_grand_flare_which_wins_over_suns_orbit() {
    let objects = |ranks: &[(&str, u32)]| -> Vec<String> {
        spawns(ranks).iter().map(|s| s.object.clone()).collect()
    };
    let all = [("solar_orbs", 1), ("grand_flare", 1), ("suns_orbit", 1)];
    assert_eq!(objects(&all), [ORB, FLARE]);
    assert_eq!(objects(&all[1..]), [GRAND, FLARE]);
    let ring = &spawns(&all[1..])[1].on_end;
    assert_eq!(ring.len(), 5);
    assert!(ring.iter().all(|flare| flare.orbit.is_none()));
}

#[test]
fn burst_of_power_may_add_its_flares_and_fire_damage_to_a_cast() {
    let ranks = [("burst_of_power", 5), ("blue_flame_fire", 5)];
    let counts: Vec<usize> = (0..40)
        .map(|n| cast_flares(&spawns_with(&mods(&ranks), n)).len())
        .collect();
    assert!(counts.iter().all(|n| [3, 8].contains(n)), "{counts:?}");
    let lucky = (0..40).find(|&n| counts[n as usize] == 8).expect("a burst");
    let plain = (0..40)
        .find(|&n| counts[n as usize] == 3)
        .expect("a plain cast");
    let share = |n: u64| cast_flares(&spawns_with(&mods(&ranks), n))[0].on_end[0].share;
    assert_eq!(share(plain), Some(1.));
    let burst = share(lucky).expect("a share");
    assert!(near(burst, 2.25 / 1.5), "{burst}");
    assert_eq!(casts(&ranks).len(), 8);
}

#[test]
fn explosive_flares_only_go_off_on_a_cast_whose_everburning_roll_burns() {
    let alone = spawns(&[("explosive_flares", 5)]);
    assert!(cast_flares(&alone)[0].on_end[0].procs.is_empty());
    let ranks = [("explosive_flares", 5), ("everburning_fire", 5)];
    let chances: Vec<f32> = (0..40)
        .map(|n| cast_flares(&spawns_with(&mods(&ranks), n))[0].on_end[0].procs[0].chance)
        .collect();
    assert!(
        chances.iter().all(|c| near(*c, 0.) || near(*c, 0.15)),
        "{chances:?}"
    );
    assert!(chances.iter().any(|c| near(*c, 0.)) && chances.iter().any(|c| near(*c, 0.15)));
    let burning = spawns(&ranks);
    let more = &cast_flares(&burning)[0].on_end[0].procs[0].spawn;
    assert_eq!((more.object.as_str(), more.share), (BLAST, Some(2.25)));
    assert!(!more.at_enemy && more.procs.is_empty());
}

#[test]
fn nodes_without_an_effect_on_one_use_leave_the_cast_unchanged() {
    let base = cast(&[]);
    let quiet = [
        ("blue_flame_fire", 5),
        ("rapidfire_cannoneer", 5),
        ("god_slayer", 5),
        ("solars_edge", 4),
        ("flaming_hands", 5),
        ("celestial_step", 5),
    ];
    // Everburning Fire's burning ticks are not hits; Explosive Flares needs that burning.
    for alone in [("everburning_fire", 5), ("explosive_flares", 5)] {
        let mut ranks = quiet.to_vec();
        ranks.push(alone);
        for same in casts(&ranks).iter() {
            assert_eq!(same.hits, base.hits);
            assert_eq!(same.frames.len(), base.frames.len());
            assert_eq!(shares(same), shares(&base));
        }
    }
}

#[test]
fn a_burning_cast_may_set_off_explosive_flares_on_the_dummy() {
    let ranks = [("explosive_flares", 5), ("everburning_fire", 5)];
    let uses = casts(&ranks);
    let more = uses
        .iter()
        .find(|u| u.hits.len() > 3)
        .expect("an explosive flare");
    let shares = shares(more);
    assert!(shares.iter().any(|s| near(*s, 2.25)), "{shares:?}");
    let blasts = births(more, BLAST);
    assert_eq!(blasts.len(), more.hits.len());
}

#[test]
fn the_builds_size_bonus_widens_the_flares() {
    let mods = Mods {
        size_bonus: 0.25,
        ..mods(&[])
    };
    let wide = spawns_with(&mods, 0);
    assert!(
        cast_flares(&wide)
            .iter()
            .all(|f| f.scale == Some([1.25, 1.25]))
    );
}

#[test]
fn every_node_at_once_still_plays_bounded_uses() {
    let tree = [
        ("solar_fireworks", 4),
        ("blue_flame_fire", 5),
        ("rapidfire_cannoneer", 5),
        ("god_slayer", 5),
        ("explosive_flares", 5),
        ("solars_edge", 4),
        ("everburning_fire", 5),
        ("flaming_hands", 5),
        ("celestial_step", 5),
        ("burst_of_power", 5),
        ("grand_flare", 3),
        ("suns_orbit", 3),
        ("searing_flares", 3),
        ("solar_orbs", 3),
    ];
    for storm in casts(&tree).iter() {
        assert!(storm.hits.len() >= 3 * 8 * 22, "{}", storm.hits.len());
        assert!(storm.frames.len() < 1800, "{}", storm.frames.len());
    }
}
