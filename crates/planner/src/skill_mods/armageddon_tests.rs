use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use, uses};
use std::collections::HashMap;

const METEOR: &str = "Pyromancer_Armageddon_obj";
const BLAST: &str = "Explosion_Item_obj";
const NOVA: &str = "Pyromancer_Firenova_obj";
const CONTROLLER: &str = "Pyromancer_Armageddon_Controller_obj";
const WARPED: &str = "Pyromancer_Armageddon_Warped_Controller_obj";
const HORIZONTAL: &str = "Pyromancer_Armageddon_Horizontal_obj";
const HORIZONTAL_BLAST: &str = "Explosion_Item_obj:Pyromancer_Explosion_spr";
const CURSOR: [f32; 2] = [160., -30.];
const DUMMY: [f32; 2] = [160., 0.];
/// A volley's meteors are drawn two frames after the base use's, every 6 frames from frame 1.
const FIRST_METEOR: usize = 3;

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("armageddon:{node}"), *rank))
        .collect();
    Mods::of("armageddon", &ranks)
}

fn base() -> &'static [Spawn] {
    &crate::skill_sim::skill("armageddon")
        .expect("armageddon")
        .spawn
}

fn spawns_with(mods: &Mods, cast: u64) -> Vec<Spawn> {
    crate::skill_mods::apply_cast("armageddon", base(), mods, cast).into_owned()
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    spawns_with(&mods(ranks), cast)
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play(spawns: &[Spawn]) -> Timeline {
    play_use(spawns, 0)
}

/// The `n`-th use of `spawns`, rolling its own dice.
fn play_use(spawns: &[Spawn], n: u64) -> Timeline {
    let fx = crate::skill_sim::skill("armageddon").expect("armageddon");
    let hitbox = crate::hitbox::skill("armageddon").expect("armageddon hitboxes");
    replay_use(fx, hitbox, spawns, 1, n)
}

/// The spawns of the first cast among the first `tries` whose spawns pass `lucky`.
fn lucky(ranks: &[(&str, u32)], tries: u64, lucky: impl Fn(&[Spawn]) -> bool) -> Vec<Spawn> {
    (0..tries)
        .map(|n| spawns_of(ranks, n))
        .find(|spawns| lucky(spawns))
        .expect("a cast that rolls it")
}

/// Every spawn of the cast, with all those its spawns make on ending or along their trails.
fn all(spawns: &[Spawn]) -> Vec<&Spawn> {
    let mut found = Vec::new();
    for spawn in spawns {
        found.push(spawn);
        found.extend(all(&spawn.on_end));
        let trails = std::iter::successors(spawn.trail.as_deref(), |t| t.next.as_deref());
        for trail in trails {
            found.extend(all(std::slice::from_ref(&trail.spawn)));
        }
    }
    found
}

fn made<'a>(spawns: &'a [Spawn], object: &str) -> Vec<&'a Spawn> {
    let all = all(spawns).into_iter();
    all.filter(|spawn| spawn.object == object).collect()
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

fn spots(timeline: &Timeline, object: &str) -> Vec<[f32; 2]> {
    births(timeline, object)
        .iter()
        .map(|(_, s)| s.pos)
        .collect()
}

fn shares(timeline: &Timeline) -> Vec<f32> {
    timeline.effects.damage.iter().map(|d| d.share).collect()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn close(a: [f32; 2], b: [f32; 2]) -> bool {
    from(a, b) < 1e-2
}

fn from(pos: [f32; 2], to: [f32; 2]) -> f32 {
    (pos[0] - to[0]).hypot(pos[1] - to[1])
}

/// The base use's meteors, by volley: where each lands, as an offset from the cursor point.
fn base_offsets() -> Vec<[f32; 2]> {
    base()[0].layout.iter().map(|p| p.offset).collect()
}

/// The aim from the player to the cursor point, and its unit vectors along and across.
fn aim() -> (f32, [f32; 2], [f32; 2]) {
    let aim = (-CURSOR[1]).atan2(CURSOR[0]).to_degrees();
    let unit = |deg: f32| {
        let (sin, cos) = deg.to_radians().sin_cos();
        [cos, -sin]
    };
    (aim, unit(aim), unit(aim + 90.))
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

#[test]
fn the_base_use_is_the_seeded_rain_of_forty_meteors() {
    let cast = spawns(&[]);
    assert_eq!(cast.len(), 1);
    assert_eq!(cast[0].layout.len(), 40);
    let timeline = play(&cast);
    let meteors = births(&timeline, METEOR);
    assert_eq!(meteors.len(), 40);
    let offsets = base_offsets();
    for ((_, meteor), offset) in meteors.iter().zip(&offsets) {
        assert!(close(
            meteor.pos,
            [CURSOR[0] + offset[0], CURSOR[1] + offset[1]]
        ));
    }
    assert_eq!(shares(&timeline), [1.; 4]);
    assert_eq!(uses("armageddon", &mods(&[])).expect("uses").len(), 8);
}

#[test]
fn damage_burning_and_cooldown_nodes_leave_the_rain_as_it_is() {
    let base = play(&spawns(&[]));
    for (node, rank) in [
        ("armageddon_core", 1),
        ("searing_meteor", 5),
        ("doomsday_clock", 5),
        ("blisters", 5),
        ("doom_fire", 5),
    ] {
        let cast = spawns(&[(node, rank)]);
        assert!(cast.len() == 1 && cast[0].layout.len() == 40, "{node}");
        let cast = play(&cast);
        assert_eq!((shares(&cast), &cast.hits), (shares(&base), &base.hits));
    }
}

#[test]
fn falling_wrath_drops_rank_more_meteors_with_every_one_of_the_rain() {
    let timeline = play(&spawns(&[("falling_wrath", 2)]));
    let meteors = births(&timeline, METEOR);
    assert_eq!(meteors.len(), 120);
    let volleys: Vec<usize> = (0..40).flat_map(|i| [FIRST_METEOR + 6 * i; 3]).collect();
    assert_eq!(frames_of(&timeline, METEOR), volleys);
    // The first of each volley lands where the base use's does, the others irandom(340) px out.
    for (volley, offset) in meteors.chunks(3).zip(base_offsets()) {
        let spot = [CURSOR[0] + offset[0], CURSOR[1] + offset[1]];
        assert!(
            close(volley[0].1.pos, spot),
            "{:?} {spot:?}",
            volley[0].1.pos
        );
        assert!(volley.iter().all(|(_, m)| from(m.pos, CURSOR) <= 340.01));
    }
    // Every meteor explodes where it lands.
    assert_eq!(births(&timeline, BLAST).len(), 120);
}

#[test]
fn falling_wrath_keeps_every_explosion_of_its_biggest_rain() {
    let timeline = play(&spawns(&[("falling_wrath", 8)]));
    assert_eq!(births(&timeline, METEOR).len(), 360);
    assert_eq!(births(&timeline, BLAST).len(), 360);
}

#[test]
fn storm_of_fire_and_searing_concentration_stretch_the_rain_about_the_cursor() {
    let stretched = |ranks: &[(&str, u32)], by: f32| {
        let timeline = play(&spawns(ranks));
        for (spot, offset) in spots(&timeline, METEOR).iter().zip(base_offsets()) {
            let want = [CURSOR[0] + offset[0] * by, CURSOR[1] + offset[1] * by];
            assert!(close(*spot, want), "{ranks:?}: {spot:?} {want:?}");
        }
        timeline
    };
    // L0x59a: 340 × (1 + 0.15 × rank); L0x5a8: then × (1 − 0.25 × rank).
    let wide = stretched(&[("storm_of_fire", 2)], 1.3);
    let tight = stretched(&[("searing_concentration", 3)], 0.25);
    stretched(&[("storm_of_fire", 4), ("searing_concentration", 2)], 0.8);
    assert!(shares(&wide).len() < 5 && shares(&tight).len() > 10);
}

#[test]
fn colossal_chunks_and_the_size_bonus_grow_the_meteors_and_their_explosions() {
    let scales = |mods: &Mods| {
        let timeline = play(&spawns_with(mods, 0));
        let of = |object| {
            births(&timeline, object)
                .iter()
                .map(|(_, s)| s.scale[0])
                .collect()
        };
        (of(METEOR), of(BLAST))
    };
    let (meteors, blasts): (Vec<f32>, Vec<f32>) = scales(&mods(&[("colossal_chunks", 3)]));
    assert!(meteors.iter().chain(&blasts).all(|&s| near(s, 1.6)));
    assert_eq!((meteors.len(), blasts.len()), (40, 40));
    let mut sized = mods(&[("colossal_chunks", 3)]);
    sized.size_bonus = 0.2;
    let (meteors, blasts) = scales(&sized);
    assert!(meteors.iter().chain(&blasts).all(|&s| near(s, 1.8)));
    let mut bonus = mods(&[]);
    bonus.size_bonus = 0.5;
    let (meteors, _) = scales(&bonus);
    assert!(meteors.iter().all(|&s| near(s, 1.5)));
}

#[test]
fn pyrokinetic_madness_recasts_a_fresh_rain_sixteen_frames_later() {
    let ranks = [("pyrokinetic_madness", 2)];
    let casts: Vec<usize> = (0..40).map(|n| spawns_of(&ranks, n).len()).collect();
    assert!(casts.contains(&1) && casts.contains(&2));
    let twice = lucky(&ranks, 40, |spawns| spawns.len() == 2);
    assert_eq!(twice[0].layout.len(), 40);
    assert_eq!(twice[1].object, CONTROLLER);
    assert_eq!(twice[1].layout[0].frame, 16);
    let timeline = play(&twice);
    let meteors = births(&timeline, METEOR);
    assert_eq!(meteors.len(), 80);
    let copied: Vec<usize> = (0..40).map(|i| 16 + FIRST_METEOR + 6 * i).collect();
    let frames = frames_of(&timeline, METEOR);
    assert!(copied.iter().all(|frame| frames.contains(frame)));
    // The copy rains its own irandom spots, not the first cast's.
    let first: Vec<[f32; 2]> = meteors.iter().take(40).map(|(_, s)| s.pos).collect();
    let copy = made(&twice[1..], METEOR);
    assert_eq!(copy.len(), 40);
    let spots = spots(&timeline, METEOR);
    let fresh = spots
        .iter()
        .filter(|s| !first.iter().any(|f| close(*f, **s)));
    assert!(fresh.count() >= 39);
    assert!(spots.iter().all(|s| from(*s, CURSOR) <= 340.01));
}

#[test]
fn ball_of_fire_swells_a_lucky_meteor_and_its_explosion() {
    let ranks = [("ball_of_fire", 5)];
    let mut sizes = Vec::new();
    for cast in 0..8 {
        let spawns = spawns_of(&ranks, cast);
        for meteor in made(&spawns, METEOR) {
            let scale = meteor.scale.expect("a scale")[0];
            let blast = &meteor.on_end[0];
            assert_eq!(blast.object, BLAST);
            assert_eq!(blast.scale.map(|s| s[0]), Some(scale));
            // Alarm_0 L0xa-L0xe: 4 % a rank, 0.15 bigger a rank, floor(damage × (1 + 0.15 × rank)).
            if near(scale, 1.75) {
                assert_eq!(blast.share, Some(1.75));
            } else {
                assert!(near(scale, 1.) && blast.share.is_none(), "{scale}");
            }
            sizes.push(scale);
        }
    }
    assert_eq!(sizes.len(), 320);
    let swollen = sizes.iter().filter(|&&s| near(s, 1.75)).count() as f32 / 320.;
    assert!((0.12..0.28).contains(&swollen), "{swollen}");
}

#[test]
fn blazing_pulse_may_set_off_a_nova_where_a_meteor_lands() {
    let mut sized = mods(&[("blazing_pulse", 5)]);
    sized.size_bonus = 0.2;
    let spawns = spawns_with(&sized, 0);
    let meteors = made(&spawns, METEOR);
    assert_eq!(meteors.len(), 40);
    for meteor in meteors {
        // Step_0 L0x1d-L0x24: irandom(99) < 3 × rank on landing, maxScale 0.75, damage × 1.8.
        let nova = &meteor.on_end[1];
        assert_eq!(nova.object, NOVA);
        assert_eq!((nova.chance, nova.share), (Some(0.15), Some(1.8)));
        assert!(near(nova.grow.and_then(|g| g.max).expect("a max"), 0.95));
    }
    let novas: usize = (0..8)
        .map(|n| play_use(&spawns_of(&[("blazing_pulse", 5)], n), n))
        .map(|timeline| {
            let novas = births(&timeline, NOVA);
            let blasts = spots(&timeline, BLAST);
            assert!(
                novas
                    .iter()
                    .all(|(_, n)| blasts.iter().any(|b| close(*b, n.pos)))
            );
            novas.len()
        })
        .sum();
    // 15 % of 320 landings.
    assert!((25..75).contains(&novas), "{novas}");
}

#[test]
fn warm_shower_rains_the_meteors_around_the_player() {
    let timeline = play(&spawns(&[("warm_shower", 1)]));
    // Controller Step_0 L4-L6: skillFollow keeps it at the player's x and 6 px under its y.
    for (spot, offset) in spots(&timeline, METEOR).iter().zip(base_offsets()) {
        assert!(close(*spot, [offset[0], 6. + offset[1]]), "{spot:?}");
    }
    assert_eq!(births(&timeline, METEOR).len(), 40);
}

#[test]
fn blistering_bolt_drops_one_meteor_on_the_dummy_instead_of_the_rain() {
    let bolt = spawns(&[("blistering_bolt", 1)]);
    assert_eq!(bolt.len(), 1);
    assert_eq!(bolt[0].object, METEOR);
    let timeline = play(&bolt);
    let meteors = births(&timeline, METEOR);
    assert_eq!(meteors.len(), 1);
    assert_eq!((meteors[0].0, meteors[0].1.pos), (0, DUMMY));
    assert_eq!(shares(&timeline), [1.]);
    assert!(made(&bolt, CONTROLLER).is_empty());
    // floor(340 × 1.15 × 0.5) shrinks to 97 px with Searing Concentration 2, short of the dummy's
    // box 100.8 px from the circle's centre 48 px along the aim; Blistering Bolt 2 makes it 110.
    assert!(spawns(&[("blistering_bolt", 1), ("searing_concentration", 2)]).is_empty());
    assert_eq!(
        spawns(&[("blistering_bolt", 2), ("searing_concentration", 2)]).len(),
        1
    );
}

#[test]
fn warped_doom_sends_the_meteors_along_the_aim_from_behind_the_cursor() {
    let spawns = spawns(&[("warped_doom", 1)]);
    assert_eq!(spawns[0].object, WARPED);
    let timeline = play(&spawns);
    let meteors = births(&timeline, HORIZONTAL);
    assert_eq!(meteors.len(), 40);
    let volleys: Vec<usize> = (0..40).map(|i| 4 + 4 * i).collect();
    assert_eq!(frames_of(&timeline, HORIZONTAL), volleys);
    // Warped Step_0 L0x9-L0xd: 280 px back along the aim, skillRadius / 2 − irandom(skillRadius)
    // px across it, flying along it.
    let (aim, along, across) = aim();
    for (_, meteor) in &meteors {
        let rel = [meteor.pos[0] - CURSOR[0], meteor.pos[1] - CURSOR[1]];
        assert!(near(dot(rel, along), -280.), "{:?}", meteor.pos);
        assert!(dot(rel, across).abs() <= 170.01, "{:?}", meteor.pos);
        assert!(near(meteor.angle, aim));
    }
}

#[test]
fn a_warped_meteor_whose_centre_meets_the_dummy_stops_and_explodes_on_it() {
    let (mut stops, mut touches) = (0, 0);
    for cast in 0..8 {
        let spawns = spawns_of(&[("warped_doom", 1)], cast);
        let stopping = made(&spawns, HORIZONTAL)
            .into_iter()
            .filter(|m| !m.on_end.is_empty())
            .count();
        let timeline = play(&spawns);
        let blasts = births(&timeline, HORIZONTAL_BLAST);
        assert_eq!(blasts.len(), stopping);
        assert!(blasts.iter().all(|(_, b)| from(b.pos, DUMMY) <= 12.));
        // Each meteor touching the dummy hits it once; one stopping on it adds its explosion.
        let hits = shares(&timeline).len();
        assert!(hits >= 2 * stopping, "{hits} {stopping}");
        stops += stopping;
        touches += hits - stopping;
    }
    // 24 of irandom(340)'s 341 spots pass within 12 px, about 56 within reach of the box.
    let (stops, touches) = (stops as f32 / 8., touches as f32 / 8.);
    assert!((1.5..5.5).contains(&stops) && (4.5..9.5).contains(&touches));
}

#[test]
fn warped_doom_ignores_warm_shower_and_blistering_bolt_overrides_both() {
    let warped = spawns(&[("warped_doom", 1), ("warm_shower", 1)]);
    assert_eq!(warped[0].object, WARPED);
    assert!(close(warped[0].layout[0].offset, CURSOR));
    let bolt = spawns(&[("warped_doom", 1), ("blistering_bolt", 1)]);
    assert_eq!(bolt.len(), 1);
    assert_eq!(bolt[0].object, METEOR);
}

#[test]
fn warped_doom_keeps_every_meteor_of_its_biggest_volleys() {
    let timeline = play(&spawns(&[("warped_doom", 1), ("falling_wrath", 8)]));
    assert_eq!(births(&timeline, HORIZONTAL).len(), 360);
}

#[test]
fn only_the_nodes_the_game_reads_its_own_way_keep_their_damage_off_the_hit() {
    let owned = crate::skill_mods::parity_tests::owned("pyromancer", "armageddon");
    assert_eq!(
        owned,
        [
            "pyrokinetic_madness",
            "ball_of_fire",
            "blazing_pulse",
            "warm_shower",
            "blistering_bolt",
            "warped_doom"
        ]
    );
}

#[test]
fn every_node_at_its_top_rank_still_plays_through_the_preview() {
    let nodes = [
        ("falling_wrath", 8),
        ("storm_of_fire", 5),
        ("colossal_chunks", 5),
        ("searing_meteor", 5),
        ("pyrokinetic_madness", 2),
        ("doomsday_clock", 5),
        ("blisters", 5),
        ("ball_of_fire", 5),
        ("doom_fire", 5),
        ("blazing_pulse", 5),
        ("searing_concentration", 3),
        ("warm_shower", 3),
    ];
    let built = |ranks: &[(&str, u32)]| Mods {
        size_bonus: 0.3,
        duration_pct: 40.,
        ..mods(ranks)
    };
    let rain = built(&nodes);
    let warped = built(&[nodes.as_slice(), &[("warped_doom", 3)]].concat());
    for (name, mods) in [("rain", rain), ("warped", warped)] {
        let uses = uses("armageddon", &mods).expect(name);
        assert_eq!(uses.len(), 8, "{name}");
        assert!(
            uses.iter().all(|timeline| !timeline.hits.is_empty()),
            "{name}"
        );
    }
}

const CLASS: &str = "pyromancer";

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_weighted(CLASS, "armageddon", ranks)
}

/// Each of `together` counted by the game, every spot and roll at its odds, and by the engine.
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
fn the_engines_subskill_hits_follow_the_sim() {
    let uncapped = super::parity::uncapped;
    let (sim, base) = (uncapped(&[]), engine(&[]));
    assert!((sim - base).abs() < 0.01, "sim {sim}, engine {base}");
    let wrong = parity::misses(CLASS, "armageddon", uncapped, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_every_node_under_warm_shower_warped_doom_and_blistering_bolt() {
    let mut together = Vec::new();
    for mode in [
        ("warm_shower", 1),
        ("warped_doom", 1),
        ("blistering_bolt", 2),
    ] {
        for node in parity::nodes(CLASS, "armageddon") {
            let others = (1..=node.max_rank).filter(|_| node.id != mode.0);
            together.extend(others.map(|rank| vec![mode, (node.id.as_str(), rank)]));
        }
    }
    agree(&together);
}

/// The cast's rolls: a nova deals the damage of the meteor that made it, Ball of Fire's included,
/// and Pyrokinetic Madness recasts both.
const ROLLS: [&str; 3] = ["ball_of_fire", "blazing_pulse", "pyrokinetic_madness"];
const MODES: [&[(&str, u32)]; 4] = [
    &[],
    &[("warm_shower", 1)],
    &[("warped_doom", 1)],
    &[("blistering_bolt", 2)],
];

#[test]
fn the_engine_follows_the_procs_that_roll_each_other() {
    let max = |id: &str| {
        let node = parity::nodes(CLASS, "armageddon")
            .iter()
            .find(|n| n.id == id);
        node.map_or(0, |n| n.max_rank)
    };
    let mut together = Vec::new();
    for mode in MODES {
        for (i, a) in ROLLS.iter().enumerate() {
            for b in &ROLLS[i + 1..] {
                let ranks = (1..=max(a)).flat_map(|ra| (1..=max(b)).map(move |rb| (ra, rb)));
                together.extend(ranks.map(|(ra, rb)| [mode, &[(*a, ra), (*b, rb)]].concat()));
            }
        }
        let sets = parity::proc_sets(CLASS, "armageddon", &ROLLS);
        together.extend(sets.iter().map(|set| [mode, set.as_slice()].concat()));
    }
    agree(&together);
}

#[test]
fn the_engine_follows_the_nodes_the_game_plays_together() {
    agree(&[
        // The drawn preview's 256-object cap trims this use; every meteor is counted apart here.
        vec![
            ("warped_doom", 1),
            ("falling_wrath", 8),
            ("pyrokinetic_madness", 2),
        ],
        vec![
            ("falling_wrath", 3),
            ("colossal_chunks", 5),
            ("blazing_pulse", 5),
        ],
        vec![("storm_of_fire", 5), ("colossal_chunks", 3)],
        vec![("storm_of_fire", 2), ("blazing_pulse", 4)],
        vec![("searing_concentration", 2), ("blazing_pulse", 5)],
        vec![("searing_concentration", 1), ("colossal_chunks", 5)],
        vec![("pyrokinetic_madness", 2), ("ball_of_fire", 5)],
        vec![
            ("pyrokinetic_madness", 2),
            ("blistering_bolt", 3),
            ("blazing_pulse", 5),
        ],
        vec![
            ("warm_shower", 3),
            ("storm_of_fire", 3),
            ("falling_wrath", 4),
        ],
        vec![
            ("warm_shower", 2),
            ("colossal_chunks", 4),
            ("blazing_pulse", 3),
        ],
        vec![
            ("warped_doom", 2),
            ("colossal_chunks", 5),
            ("blazing_pulse", 5),
        ],
        vec![
            ("warped_doom", 3),
            ("searing_concentration", 2),
            ("falling_wrath", 5),
        ],
    ]);
}
