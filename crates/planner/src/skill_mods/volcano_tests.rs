use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use, uses};
use std::collections::HashMap;

const VOLCANO: &str = "Pyromancer_Volcano_obj";
const YELLOWSTONE: &str = "Pyromancer_Volcano_obj:Pyromancer_Volcano_Yellowstone_spr";
const FRAGMENT: &str = "Pyromancer_Volcano_Fragment_obj";
const BLAST: &str = "Explosion_Item_obj";
const BOUNCE: &str = "Explosion_Item_obj:Viking_Odins_Fury_spr";
const BURST: &str = "Pyromancer_Scorching_Searing_Burst_obj";
const CONTROLLER: &str = "Pyromancer_Blazing_Trail_Controller_obj";
const PATCH: &str = "Pyromancer_Blazing_Trail_obj";
const CURSOR: [f32; 2] = [160., -30.];
const DUMMY: [f32; 2] = [160., 0.];
/// The base volcano's shots: grown on age 18, shootCooldown 15 counted down, then every 16 frames
/// until the fade on age 242.
const SHOTS: [usize; 13] = [34, 50, 66, 82, 98, 114, 130, 146, 162, 178, 194, 210, 226];
/// A fragment is drawn the frame after the shot and explodes 61 frames later, on its 60th move.
const DRAWN: usize = 1;
const EXPLODES: usize = 62;

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("volcano:{node}"), *rank))
        .collect();
    Mods::of("volcano", &ranks)
}

fn spawns_with(mods: &Mods, cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill("volcano").expect("volcano").spawn;
    crate::skill_mods::apply_cast("volcano", base, mods, cast).into_owned()
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    spawns_with(&mods(ranks), cast)
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play(spawns: &[Spawn]) -> Timeline {
    let fx = crate::skill_sim::skill("volcano").expect("volcano");
    let hitbox = crate::hitbox::skill("volcano").expect("volcano hitboxes");
    replay_use(fx, hitbox, spawns, 1, 0)
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

fn widest(timeline: &Timeline, object: &str) -> f32 {
    let shapes = timeline.frames.iter().flatten();
    let of = shapes.filter(|shape| shape.object.name == object);
    of.map(|s| s.scale[0]).fold(0., f32::max)
}

fn shares(timeline: &Timeline) -> Vec<f32> {
    timeline.effects.damage.iter().map(|d| d.share).collect()
}

fn max_of(spawn: &Spawn) -> f32 {
    spawn.grow.and_then(|grow| grow.max).expect("a max")
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn from(pos: [f32; 2], to: [f32; 2]) -> f32 {
    (pos[0] - to[0]).hypot(pos[1] - to[1])
}

/// GameMaker's point_direction.
fn direction(from: [f32; 2], to: [f32; 2]) -> f32 {
    (from[1] - to[1]).atan2(to[0] - from[0]).to_degrees()
}

/// GameMaker's angle_difference, in [-180, 180).
fn turned(to: f32, from: f32) -> f32 {
    (to - from + 180.).rem_euclid(360.) - 180.
}

#[test]
fn the_base_use_is_one_volcano_on_the_cursor_hitting_eight_times() {
    let base = spawns(&[]);
    assert_eq!(base.len(), 1);
    assert!(base[0].trail.is_none());
    let timeline = play(&base);
    let volcanoes = births(&timeline, VOLCANO);
    assert_eq!(volcanoes.len(), 1);
    assert_eq!((volcanoes[0].0, volcanoes[0].1.pos), (0, CURSOR));
    assert_eq!(shares(&timeline), [1.; 8]);
    assert!(births(&timeline, FRAGMENT).is_empty());
    assert_eq!(uses("volcano", &mods(&[])).expect("uses").len(), 8);
}

#[test]
fn skill_duration_and_extended_lava_stretch_the_volcano() {
    let mut longer = mods(&[]);
    longer.duration_pct = 25.;
    let stretched = spawns_with(&longer, 0);
    assert_eq!(stretched[0].life, Some(300));
    assert_eq!(play(&stretched).hits.len(), 10);
    let lava = spawns(&[("extended_lava", 5)]);
    assert_eq!(lava[0].life, Some(360));
    assert_eq!(play(&lava).hits.len(), 12);
    // Case 36 L0x54f stretches destroyTimer by 10 % a rank, Alarm_11 by the build's duration.
    let mut both = mods(&[("extended_lava", 1)]);
    both.duration_pct = 25.;
    assert_eq!(spawns_with(&both, 0)[0].life, Some(330));
}

#[test]
fn searing_mountain_grows_the_volcano_and_the_size_bonus_adds_to_it() {
    let big = spawns(&[("searing_mountain", 5)]);
    assert!(near(max_of(&big[0]), 2.));
    // m_fadeDestroy takes 0.05 × maxScale a frame, so the fade still lasts 20 frames.
    assert!(near(big[0].shrink, 0.1));
    assert!(near(widest(&play(&big), VOLCANO), 2.));
    let mut sized = mods(&[("searing_mountain", 5)]);
    sized.size_bonus = 0.2;
    assert!(near(max_of(&spawns_with(&sized, 0)[0]), 2.2));
}

#[test]
fn eruption_throws_two_fragments_a_shot_once_the_volcano_is_grown() {
    let timeline = play(&spawns(&[("eruption", 1)]));
    let thrown = births(&timeline, FRAGMENT);
    let pairs = |delay: usize| SHOTS.iter().flat_map(move |&age| [age + delay; 2]);
    let drawn: Vec<usize> = pairs(DRAWN).collect();
    assert_eq!(thrown.iter().map(|(f, _)| *f).collect::<Vec<_>>(), drawn);
    assert!(thrown.iter().all(|(_, s)| s.pos == CURSOR));
    // Each lands 60 moves of 2 px later, 120 px out, and explodes there.
    let blasts = births(&timeline, BLAST);
    assert_eq!(
        frames_of(&timeline, BLAST),
        pairs(EXPLODES).collect::<Vec<_>>()
    );
    assert!(
        blasts
            .iter()
            .all(|(_, s)| (from(s.pos, CURSOR) - 120.).abs() < 0.01)
    );
    // None lands near the dummy 30 px under the volcano.
    assert_eq!(shares(&timeline), [1.; 8]);
}

#[test]
fn danger_zone_throws_the_fragments_faster_so_they_land_farther() {
    let timeline = play(&spawns(&[("danger_zone", 5)]));
    let blasts = births(&timeline, BLAST);
    assert_eq!(blasts.len(), 13);
    // Fragment Alarm_0: 2 px a frame × 1.75, for the same 60 moves.
    assert!(
        blasts
            .iter()
            .all(|(_, s)| (from(s.pos, CURSOR) - 210.).abs() < 0.01)
    );
}

#[test]
fn a_fragment_flies_at_the_dummy_when_it_stands_within_128_px_of_its_random_spot() {
    let down = direction(CURSOR, DUMMY);
    let (mut aimed, mut thrown) = (0, 0);
    for cast in 0..8 {
        let timeline = play(&spawns_of(&[("eruption", 5)], cast));
        for (_, fragment) in births(&timeline, FRAGMENT) {
            thrown += 1;
            if turned(fragment.angle, down).abs() < 1e-3 {
                aimed += 1;
                continue;
            }
            // Otherwise it keeps irandom(360)'s whole degrees.
            let whole = turned(fragment.angle, fragment.angle.round());
            assert!(whole.abs() < 1e-2, "{}", fragment.angle);
        }
    }
    // 167 of irandom(360)'s 361 spots lie within 128 px of the dummy: 46 %.
    let share = aimed as f32 / thrown as f32;
    assert!((0.36..0.56).contains(&share), "{share}");
}

#[test]
fn colossal_fragment_makes_some_fragments_bigger() {
    let mut sizes = Vec::new();
    for cast in 0..8 {
        let spawns = spawns_of(&[("colossal_fragment", 5)], cast);
        sizes.extend(made(&spawns, FRAGMENT).iter().map(|f| max_of(f)));
    }
    // m_fireProjectile L0x54 and L0x7f: irandom(99) < 35 makes it 0.75 bigger than 0.5.
    assert!(sizes.iter().all(|&s| near(s, 0.5) || near(s, 1.25)));
    let colossal = sizes.iter().filter(|&&s| near(s, 1.25)).count() as f32;
    let share = colossal / sizes.len() as f32;
    assert!((0.25..0.45).contains(&share), "{share}");
    let spawns = spawns(&[("colossal_fragment", 5)]);
    assert!(made(&spawns, BLAST).iter().all(|b| b.share == Some(1.5)));
}

#[test]
fn crushing_fragments_bounce_three_times_before_their_last_explosion() {
    let ranks = [("crushing_fragment", 5)];
    let spawns = (0..40)
        .map(|n| spawns_of(&ranks, n))
        .find(|spawns| !made(spawns, BOUNCE).is_empty())
        .expect("a crushing shot");
    // Step_0 L0x19-L0x1c: Viking_Odins_Fury_spr 0.5 big with half the fragment's damage.
    let bounces = made(&spawns, BOUNCE);
    assert!(
        bounces
            .iter()
            .all(|b| b.share == Some(0.5) && b.scale == Some([0.5; 2]))
    );
    let timeline = play(&spawns);
    let crushing = births(&timeline, BOUNCE).len() / 3;
    assert_eq!(births(&timeline, BOUNCE).len(), 3 * crushing);
    // 60 moves at 2 px, then hops of 68, 55 and 42 moves at 1.5, 1.125 and 0.84375 px.
    let hops = [120., 222., 283.875];
    for (_, bounce) in births(&timeline, BOUNCE) {
        let out = from(bounce.pos, CURSOR);
        assert!(hops.iter().any(|h| (out - h).abs() < 0.05), "{out}");
    }
    let last = births(&timeline, BLAST);
    let far = last
        .iter()
        .filter(|(_, s)| (from(s.pos, CURSOR) - 319.3125).abs() < 0.05);
    assert_eq!(far.count(), crushing);
    assert!(made(&spawns, BLAST).iter().all(|b| b.share == Some(1.5)));
}

#[test]
fn crackling_earth_bursts_80_to_208_px_around_the_volcano() {
    let timeline = play(&spawns(&[("crackling_earth", 1)]));
    // cracklingEarthTimer: 0.25 s, then ceil(0.75 s × 0.88) = 40 frames while the volcano stands.
    assert_eq!(
        frames_of(&timeline, BURST),
        [16, 56, 96, 136, 176, 216, 256]
    );
    for (_, burst) in births(&timeline, BURST) {
        assert!((80. ..=208.01).contains(&from(burst.pos, CURSOR)));
        assert!(near(burst.scale[0], 0.4));
    }
    // 50 px from the dummy at least, out of a burst's reach.
    assert_eq!(shares(&timeline), [1.; 8]);
    assert_eq!(
        frames_of(&play(&spawns(&[("crackling_earth", 5)])), BURST).len(),
        14
    );
}

#[test]
fn too_hot_to_touch_multiplies_only_the_volcanos_own_damage() {
    let timeline = play(&spawns(&[("too_hot_to_touch", 5)]));
    assert_eq!(shares(&timeline), [3.; 8]);
    // Case 36 L0x55a-L0x55b: fragmentDamage leaves the multiplier out, damage keeps it.
    let ranks = [
        ("too_hot_to_touch", 5),
        ("eruption", 1),
        ("crackling_earth", 1),
    ];
    let spawns = spawns(&ranks);
    assert!(made(&spawns, BLAST).iter().all(|b| b.share == Some(1.5)));
    assert!(made(&spawns, BURST).iter().all(|b| b.share == Some(3.)));
}

#[test]
fn yellowstone_raises_a_massive_volcano_whose_mask_misses_the_dummy() {
    let spawns = spawns(&[("yellowstone", 1)]);
    let main = &spawns[0];
    assert_eq!(main.object, YELLOWSTONE);
    assert!(near(max_of(main), 2.25));
    // Its mask keeps Pyromancer_Volcano_spr's box under the larger sprite's origin, up and left.
    let alone = Spawn {
        trail: None,
        ..main.clone()
    };
    assert!(play(&[alone]).hits.is_empty());
    assert!(near(max_of(&spawns_of(&[("yellowstone", 2)], 0)[0]), 2.25));
    // At rank 3 yellowStone (0.75) reads as true and Alarm_0 keeps a quarter of the growth.
    assert!(near(
        max_of(&spawns_of(&[("yellowstone", 3)], 0)[0]),
        1.3125
    ));
}

#[test]
fn yellowstone_fragments_raise_plain_volcanoes_where_they_land() {
    let timeline = play(&spawns(&[("yellowstone", 1)]));
    let main = |timeline: &Timeline| -> Vec<usize> {
        let thrown = births(timeline, FRAGMENT).into_iter();
        thrown
            .filter(|(_, s)| s.pos == CURSOR)
            .map(|(f, _)| f)
            .collect()
    };
    // Grown on age 43, it shoots on 59, then every max(1.5 s × 0.75, 0.25 s) + 1 = 69 frames.
    assert_eq!(main(&timeline), [60, 129, 198]);
    let small = births(&timeline, VOLCANO);
    assert_eq!(
        small.iter().map(|(f, _)| *f).collect::<Vec<_>>(),
        [59 + EXPLODES, 128 + EXPLODES, 197 + EXPLODES]
    );
    assert!(
        small
            .iter()
            .all(|(_, s)| (from(s.pos, CURSOR) - 120.).abs() < 0.01)
    );
    let spawns = spawns(&[("yellowstone", 1), ("eruption", 5)]);
    let raised: Vec<&Spawn> = made(&spawns, VOLCANO);
    assert_eq!(raised.len(), 18);
    assert!(raised.iter().all(|v| near(max_of(v), 1.)));
    // The small ones keep Create_0's one fragment a shot: 13 each.
    let theirs = made(std::slice::from_ref(raised[0]), FRAGMENT).len();
    assert_eq!(theirs, 13);
    // Rank 3 grows on age 25 and shoots every 24 frames.
    assert_eq!(
        main(&play(&spawns_of(&[("yellowstone", 3)], 0)))[..2],
        [42, 66]
    );
}

#[test]
fn lava_trail_sends_three_controllers_a_rank_every_second() {
    let timeline = play(&spawns(&[("lava_trail", 1)]));
    // lavaTrailTimer: 0.25 s, then 1 s, while the volcano stands until age 262.
    let series = [16, 76, 136, 196, 256];
    let expected: Vec<usize> = series.iter().flat_map(|&f| [f; 3]).collect();
    assert_eq!(frames_of(&timeline, CONTROLLER), expected);
    // Each walks 5 px a frame: its first patch drops 9 moves out, 45 px from the volcano.
    let patches = births(&timeline, PATCH);
    let first: Vec<&&Shape> = patches
        .iter()
        .filter(|(f, _)| *f == 26)
        .map(|(_, s)| s)
        .collect();
    assert_eq!(first.len(), 3);
    assert!(
        first
            .iter()
            .all(|s| (from(s.pos, CURSOR) - 45.).abs() < 0.01)
    );
    let ranks = [("lava_trail", 1), ("too_hot_to_touch", 2)];
    let spawns = spawns(&ranks);
    assert!(
        made(&spawns, PATCH)
            .iter()
            .all(|p| p.share == Some(1.8) && p.tick == 60.)
    );
}

#[test]
fn i_am_the_volcano_now_stands_it_under_the_player() {
    let ranks = [("i_am_the_volcano_now", 1)];
    let spawns = spawns(&ranks);
    let timeline = play(&spawns);
    assert_eq!(births(&timeline, VOLCANO)[0].1.pos, [0., 6.]);
    let alone = Spawn {
        trail: None,
        ..spawns[0].clone()
    };
    assert!(play(&[alone]).hits.is_empty());
    assert!(
        births(&timeline, FRAGMENT)
            .iter()
            .all(|(_, s)| s.pos == [0., 6.])
    );
    // A fragment thrown at the dummy lands 40 px short of it, inside its explosion's reach.
    let reached = (0..8).any(|n| shares(&play(&spawns_of(&ranks, n))).contains(&1.5));
    assert!(reached);
}

#[test]
fn trail_of_volcanoes_chains_four_volcanoes_a_rank_96_px_apart() {
    let ranks = [("trail_of_volcanoes", 1), ("searing_mountain", 5)];
    let spawns = spawns(&ranks);
    let timeline = play(&spawns);
    let volcanoes = births(&timeline, VOLCANO);
    let frames: Vec<usize> = volcanoes.iter().map(|(f, _)| *f).collect();
    assert_eq!(frames, [0, 30, 60, 90, 120]);
    let mut heading = direction([0., 0.], CURSOR);
    for pair in volcanoes.windows(2) {
        let (a, b) = (pair[0].1.pos, pair[1].1.pos);
        assert!((from(a, b) - 96.).abs() < 0.01);
        assert!(turned(direction(a, b), heading).abs() <= 40.01);
        heading = direction(a, b);
    }
    // Alarm_1 copies neither maxScale nor fragmentDamage.
    assert!(near(max_of(&spawns[0]), 2.));
    assert!(spawns[1..].iter().all(|v| near(max_of(v), 1.)));
    let thrown = self::spawns(&[("trail_of_volcanoes", 1), ("eruption", 1)]);
    let harmless = |v: &Spawn| {
        made(std::slice::from_ref(v), BLAST)
            .iter()
            .all(|b| b.ability)
    };
    assert!(thrown[1..].iter().all(harmless));
    let first = made(&thrown[..1], BLAST);
    assert!(first.iter().all(|b| !b.ability && b.share == Some(1.5)));
    // Nor yellowStone: they keep the plain look and shoot every 16 frames.
    let yellow = self::spawns(&[("trail_of_volcanoes", 1), ("yellowstone", 1)]);
    assert!(yellow[1..].iter().all(|v| v.object == VOLCANO));
    assert_eq!(made(&yellow[1..2], FRAGMENT).len(), 13);
}

#[test]
fn damage_only_nodes_leave_the_use_as_it_is() {
    let base = play(&spawns(&[]));
    for node in ["volcano_core", "scorching_lava", "lava_dipping"] {
        let cast = play(&spawns(&[(node, 3)]));
        assert_eq!(
            (shares(&cast), &cast.hits),
            (shares(&base), &base.hits),
            "{node}"
        );
    }
}

#[test]
fn every_node_together_still_plays() {
    let every = mods(&[
        ("eruption", 5),
        ("extended_lava", 5),
        ("danger_zone", 5),
        ("searing_mountain", 5),
        ("colossal_fragment", 5),
        ("crackling_earth", 5),
        ("crushing_fragment", 5),
        ("too_hot_to_touch", 5),
        ("yellowstone", 3),
        ("lava_trail", 3),
        ("i_am_the_volcano_now", 3),
        ("trail_of_volcanoes", 3),
    ]);
    let uses = uses("volcano", &every).expect("uses");
    assert_eq!(uses.len(), 8);
    assert!(uses.iter().all(|u| u.frames.iter().all(|f| f.len() <= 256)));
}

#[test]
fn only_the_nodes_the_game_reads_its_own_way_keep_their_damage_off_the_hit() {
    let owned = crate::skill_mods::parity_tests::owned("pyromancer", "volcano");
    assert_eq!(owned, ["too_hot_to_touch"]);
}

const CLASS: &str = "pyromancer";

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_hits(CLASS, super::SKILL, ranks)
}

/// Each of `together` played by the sim and counted by the engine, within `within`.
fn agree(together: &[&[(&str, u32)]], within: f32) {
    for ranks in together {
        let (sim, engine) = (super::parity::uncapped(ranks), engine(ranks));
        assert!(
            (sim - engine).abs() < within,
            "{ranks:?}: sim {sim}, engine {engine}"
        );
    }
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let uncapped = super::parity::uncapped;
    let (sim, base) = (uncapped(&[]), engine(&[]));
    assert!(near(sim, 8.) && near(base, 8.), "sim {sim}, engine {base}");
    let wrong = parity::misses(CLASS, super::SKILL, uncapped, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_fragments_yellowstone_and_the_player_volcano_land() {
    let i_am = ("i_am_the_volcano_now", 2);
    let mut together: Vec<Vec<(&str, u32)>> = Vec::new();
    for rank in 1..=5 {
        for node in ["eruption", "danger_zone", "crushing_fragment"] {
            together.push(vec![i_am, (node, rank)]);
        }
        for node in ["eruption", "danger_zone", "extended_lava"] {
            together.push(vec![("yellowstone", 3), (node, rank)]);
        }
        together.push(vec![i_am, ("extended_lava", rank)]);
        together.push(vec![i_am, ("crackling_earth", rank)]);
        together.push(vec![("yellowstone", 1), ("danger_zone", rank)]);
    }
    // The sim takes about a second for each of these.
    for rank in [1, 5] {
        together.push(vec![("yellowstone", 3), ("crushing_fragment", rank)]);
        together.push(vec![("yellowstone", 2), ("crushing_fragment", rank)]);
    }
    for rank in 1..=3 {
        together.push(vec![i_am, ("lava_trail", rank)]);
        together.push(vec![i_am, ("yellowstone", rank)]);
        together.push(vec![("yellowstone", rank), ("lava_trail", 2)]);
        together.push(vec![("yellowstone", 1), ("eruption", rank)]);
        together.push(vec![i_am, ("danger_zone", 5), ("eruption", rank)]);
    }
    let all: Vec<&[(&str, u32)]> = together.iter().map(Vec::as_slice).collect();
    agree(&all, 0.01);
}

#[test]
fn the_engine_follows_the_chain_the_player_volcano_starts() {
    // A random walk: 1024 seeded walks against the engine's 8192-walk mean of 1.766.
    agree(
        &[
            &[("i_am_the_volcano_now", 1), ("trail_of_volcanoes", 1)],
            &[("i_am_the_volcano_now", 3), ("trail_of_volcanoes", 3)],
        ],
        0.2,
    );
}
