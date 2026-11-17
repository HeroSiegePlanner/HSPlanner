use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use, uses};
use std::collections::HashMap;

const SKILL: &str = "phoenix_flight";
const MOVER: &str = "Draw_Player_Buff_obj";
const PHOENIX: &str = "Visual_Effect_obj";
const NOVA: &str = "Pyromancer_Firenova_obj";
const AURA: &str = "Pyromancer_Phoenix_Flight_obj";
const SEED: &str = "Pyromancer_Phoenix_Flight_Seed_obj";
const BLAST: &str = "Explosion_Item_obj";
const SMASH: &str = "Explosion_Item_obj:Viking_Odins_Fury_spr";
const CHAIN: &str = "Pyromancer_Searing_Chains_obj";
const BALL: &str = "Pyromancer_Fire_Ball_obj";
const SPLIT: &str = "Pyromancer_Fire_Ball_Split_obj";
const PATCH: &str = "Pyromancer_Blazing_Trail_obj";
const DUMMY: [f32; 2] = [160., 0.];
/// Where the base charge stops: 21 moves out, the 22nd's probe touching the dummy.
const LANDING: f32 = 129.733;
/// Where a charge that flies through ends: still speeding up, its loop round the cursor on the
/// dummy brings it back 23 px off it.
const LOOPED: [f32; 2] = [166.7, -22.];

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("{SKILL}:{node}"), *rank))
        .collect();
    Mods::of(SKILL, &ranks)
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill(SKILL)
        .expect("phoenix flight")
        .spawn;
    crate::skill_mods::apply_cast(SKILL, base, &mods(ranks), cast).into_owned()
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play(spawns: &[Spawn]) -> Timeline {
    let fx = crate::skill_sim::skill(SKILL).expect("phoenix flight");
    let hitbox = crate::hitbox::skill(SKILL).expect("phoenix flight hitboxes");
    replay_use(fx, hitbox, spawns, 1, 0)
}

fn flight(ranks: &[(&str, u32)]) -> Timeline {
    play(&spawns(ranks))
}

fn mover(ranks: &[(&str, u32)]) -> Spawn {
    spawns(ranks).swap_remove(0)
}

/// The first cast among the first 64 whose spawns pass `lucky`.
fn lucky(ranks: &[(&str, u32)], lucky: impl Fn(&[Spawn]) -> bool) -> Vec<Spawn> {
    (0..64)
        .map(|n| spawns_of(ranks, n))
        .find(|spawns| lucky(spawns))
        .expect("a cast that rolls it")
}

/// Novas the charge leaves on the ground where it ends.
fn landing_novas(spawn: &Spawn) -> usize {
    let novas = spawn.on_end.iter().filter(|s| s.object == NOVA);
    novas.filter(|s| s.fall.is_none()).count()
}

/// The drawn shapes of `object`; the unseen carriers have no size.
fn shapes<'a>(timeline: &'a Timeline, object: &'a str) -> impl Iterator<Item = &'a Shape> {
    let all = timeline.frames.iter().flatten();
    all.filter(move |shape| shape.object.name == object && shape.scale[0] > 0.)
}

/// The first shape of every instance of `object`, with the frame it is drawn in.
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

fn near(a: f32, b: f32, within: f32) -> bool {
    (a - b).abs() < within
}

/// Whether two headings in degrees point the same way.
fn heads(a: f32, b: f32) -> bool {
    near((a - b + 180.).rem_euclid(360.), 180., 0.01)
}

fn shares(timeline: &Timeline) -> Vec<f32> {
    timeline.effects.damage.iter().map(|d| d.share).collect()
}

/// The largest scale `object` reaches.
fn widest(timeline: &Timeline, object: &str) -> f32 {
    shapes(timeline, object)
        .map(|s| s.scale[0])
        .fold(0., f32::max)
}

/// The scale each landing nova grows toward.
fn nova_max(ranks: &[(&str, u32)]) -> f32 {
    let mover = mover(ranks);
    let nova = mover.on_end.iter().find(|s| s.object == NOVA);
    nova.and_then(|s| s.grow?.max).expect("a growing nova")
}

#[test]
fn the_charge_ramps_from_five_px_and_stops_short_of_the_dummy() {
    let base = flight(&[]);
    let moves: Vec<[f32; 2]> = shapes(&base, MOVER).map(|s| s.pos).collect();
    assert_eq!(moves.len(), 22);
    for (at, x) in moves.iter().zip([0., 5., 10.1, 15.3025]) {
        assert!(near(at[0], x, 1e-3) && at[1] == 0., "{moves:?}");
    }
    assert!(near(moves[21][0], LANDING, 1e-2), "{:?}", moves[21]);
    assert_eq!(mover(&[]).life, Some(22));
}

#[test]
fn the_landing_nova_grows_to_two_and_a_half_and_hits_once() {
    let base = flight(&[]);
    let novas = births(&base, NOVA);
    assert_eq!(novas.len(), 1);
    let (born, nova) = novas[0];
    assert_eq!(born, 22);
    assert!(near(nova.pos[0], LANDING, 1e-2) && nova.pos[1] == 0.);
    assert_eq!(nova_max(&[]), 2.5);
    // Closing 7.5 % of the gap a frame, it is 2.42 big when it goes.
    assert!(near(widest(&base, NOVA), 2.422, 0.01));
    assert_eq!(base.hits, [born as u32 + 2]);
    assert_eq!(shares(&base), [1.]);
}

#[test]
fn a_phoenix_is_left_every_48_px_of_the_flight() {
    let base = flight(&[]);
    let phoenixes: Vec<f32> = births(&base, PHOENIX)
        .iter()
        .map(|(_, s)| s.pos[0])
        .collect();
    assert_eq!(phoenixes.len(), 2, "{phoenixes:?}");
    for (x, want) in phoenixes.iter().zip([48.818, 100.459]) {
        assert!(near(*x, want, 0.01), "{phoenixes:?}");
    }
    assert!(shapes(&base, PHOENIX).all(|s| s.age <= 20));
}

#[test]
fn swift_flight_raises_the_start_and_the_cap_and_lands_sooner() {
    let swift = mover(&[("swift_flight", 5)]);
    assert!(near(swift.speed, 23.4, 1e-4));
    assert!(near(swift.accel.expect("a ramp").from, 7.4, 1e-4));
    assert!(swift.life < mover(&[]).life);
    assert_eq!(flight(&[("swift_flight", 5)]).hits.len(), 1);
}

#[test]
fn burning_screech_widens_the_nova_by_its_share_of_the_doubled_scale() {
    // skillsCharge L0x2c9: maxScale = (1 + [71]) × 1.25, [71] = 1 + 15 % × rank.
    assert!(near(nova_max(&[("burning_screech", 5)]), 1.25 * 2.75, 1e-5));
    assert_eq!(flight(&[("burning_screech", 5)]).hits.len(), 1);
}

#[test]
fn scorching_fury_rings_its_balls_out_of_the_landing() {
    let fury = flight(&[("scorching_fury", 2)]);
    let balls = births(&fury, SPLIT);
    assert_eq!(balls.len(), 6);
    for (j, (_, ball)) in balls.iter().enumerate() {
        let turn = 60. * j as f32;
        let (sin, cos) = turn.to_radians().sin_cos();
        // Made 12 px out from the player.
        let at = [LANDING + 12. * cos, -12. * sin];
        assert!(near(ball.pos[0], at[0], 0.05) && near(ball.pos[1], at[1], 0.05));
        assert!(heads(ball.angle, turn), "{}", ball.angle);
    }
    // 30 px from the dummy, the balls within 67° of it pass within 16 + 12 px: 0° and ±60°.
    assert_eq!(shares(&fury), [1., 0.5, 0.5, 0.5]);
}

#[test]
fn afterburn_may_add_a_second_nova_twelve_frames_later_with_its_own_ring() {
    let rolled = lucky(&[("afterburn", 5), ("scorching_fury", 1)], |s| {
        landing_novas(&s[0]) == 2
    });
    let use_ = play(&rolled);
    let novas: Vec<(usize, [f32; 2])> = births(&use_, NOVA)
        .iter()
        .map(|(f, s)| (*f, s.pos))
        .collect();
    assert_eq!(novas.len(), 2);
    assert_eq!(novas[1].0 - novas[0].0, 12);
    assert_eq!(novas[0].1, novas[1].1);
    // L0x2b9: both rings at once, 12 − 32 px out: behind their own heading.
    let balls = births(&use_, SPLIT);
    assert_eq!(balls.len(), 6);
    assert!(balls.iter().all(|(f, _)| *f == novas[0].0));
    let (_, first) = balls[0];
    assert!(near(first.pos[0], LANDING - 20., 0.05), "{:?}", first.pos);
    let plain = (0..64).map(|n| landing_novas(&spawns_of(&[("afterburn", 5)], n)[0]));
    assert!(plain.clone().any(|n| n == 1) && plain.max() == Some(2));
}

#[test]
fn infused_flames_buffs_the_player_for_3_s_from_the_novas_hit() {
    let infused = flight(&[("infused_flames", 3)]);
    let hit = infused.hits[0];
    assert_eq!(infused.effects.buffed, Some(([hit, hit + 180], 30.)));
    assert_eq!(infused.hits, flight(&[]).hits);
}

#[test]
fn molten_feathers_cuts_damage_taken_while_the_charge_lasts() {
    let feathers = flight(&[("molten_feathers", 3)]);
    assert_eq!(feathers.effects.buffed, Some(([0, 22], 45.)));
}

#[test]
fn a_node_that_flies_through_loops_once_round_the_cursor_and_lands_by_it() {
    for node in [
        "collateral_burning",
        "searing_touch",
        "mental_heat",
        "seed_of_fulmination",
        "wings_of_the_phoenix",
    ] {
        let through = mover(&[(node, 1)]);
        assert_eq!(through.life, Some(50), "{node}");
    }
    let collateral = flight(&[("collateral_burning", 1)]);
    let end = shapes(&collateral, MOVER).last().expect("a mover").pos;
    assert!(
        near(end[0], LOOPED[0], 0.1) && near(end[1], LOOPED[1], 0.1),
        "{end:?}"
    );
    // The aura hits the dummy on the way in, the landing nova once more.
    assert_eq!(shares(&collateral), [0.2, 1.]);
}

#[test]
fn the_aura_rides_six_px_under_the_flying_player() {
    let collateral = flight(&[("collateral_burning", 3)]);
    let movers: Vec<[f32; 2]> = shapes(&collateral, MOVER).map(|s| s.pos).collect();
    let auras: Vec<&Shape> = shapes(&collateral, AURA).collect();
    assert_eq!(auras.len(), movers.len());
    for (aura, at) in auras.iter().zip(&movers) {
        assert!(near(aura.pos[0], at[0], 1e-3) && near(aura.pos[1], at[1] + 6., 1e-3));
        assert_eq!(aura.scale, [1., 1.]);
    }
    assert!(movers.iter().any(|at| at[1] > 60.), "it loops");
    assert_eq!(shares(&collateral), [0.6, 1.]);
}

#[test]
fn burning_wings_ends_the_flight_after_180_px_when_it_flies_through() {
    assert_eq!(mover(&[("burning_wings", 1)]).life, Some(22));
    let wings = mover(&[("burning_wings", 1), ("collateral_burning", 1)]);
    assert_eq!(wings.life, Some(29));
    let patched = flight(&[("burning_wings", 1), ("collateral_burning", 1)]);
    let end = shapes(&patched, MOVER).last().expect("a mover").pos;
    assert!(near(end[0], 187.439, 0.01));
}

#[test]
fn burning_wings_lays_a_patch_on_the_enemy_each_carrier_hits() {
    let wings = flight(&[("burning_wings", 2)]);
    let patches = births(&wings, PATCH);
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].1.pos, DUMMY);
    // The nova, then six ticks of the patch, each the nova's whole hit.
    assert_eq!(shares(&wings), [1.; 7]);
}

#[test]
fn searing_touch_adds_its_share_and_may_chain_from_the_dummy() {
    let aura = spawns(&[("searing_touch", 2)])
        .into_iter()
        .find(|s| s.object == AURA)
        .expect("an aura");
    assert_eq!(aura.share, Some(0.5));
    let chain = &aura.procs[0];
    assert!(near(chain.chance, 0.5, 1e-4));
    assert_eq!(chain.spawn.object, CHAIN);
    let all = uses(SKILL, &mods(&[("searing_touch", 2)])).expect("uses");
    let chained = all.iter().filter(|u| !births(u, CHAIN).is_empty()).count();
    assert!(chained > 0 && chained < all.len(), "{chained}");
    for u in all.iter().filter(|u| !births(u, CHAIN).is_empty()) {
        let (born, chain) = births(u, CHAIN)[0];
        assert_eq!(chain.pos, DUMMY);
        assert!(u.hits.contains(&(born as u32 + 2)));
        assert_eq!(shares(u), [0.5, 0.5, 1.]);
    }
}

#[test]
fn seed_of_fulmination_grows_the_aura_and_plants_a_seed_that_explodes_later() {
    let seeded = flight(&[("seed_of_fulmination", 3)]);
    assert!(shapes(&seeded, AURA).all(|s| near(s.scale[0], 1.45, 1e-4)));
    let seeds = births(&seeded, SEED);
    assert_eq!(seeds.len(), 1);
    let (planted, seed) = seeds[0];
    assert_eq!(seed.pos, DUMMY);
    assert_eq!(planted as u32, seeded.hits[0] + 1);
    let (blown, blast) = births(&seeded, BLAST)[0];
    assert_eq!(blown - planted, 45);
    assert_eq!(blast.scale, [2., 2.]);
    // The aura deals nothing without Searing Touch or Collateral Burning, the landing nova its
    // whole hit, the seed 15 % × rank.
    let dealt = shares(&seeded);
    assert_eq!(dealt[0], 0.);
    let mut rest = dealt[1..].to_vec();
    rest.sort_by(f32::total_cmp);
    assert!(
        rest.len() == 2 && near(rest[0], 0.45, 1e-4) && rest[1] == 1.,
        "{dealt:?}"
    );
}

#[test]
fn mental_heat_flies_through_with_an_aura_that_deals_nothing() {
    let taunt = flight(&[("mental_heat", 5)]);
    assert_eq!(shares(&taunt), [0., 1.]);
}

#[test]
fn rise_from_the_ashes_leaps_with_novas_every_16_frames_then_smashes_the_ground() {
    let rise = flight(&[("rise_from_the_ashes", 3)]);
    let novas = births(&rise, NOVA);
    assert_eq!(novas.len(), 4);
    let landed = novas[0].0;
    let lifted: Vec<(usize, f32)> = novas[1..]
        .iter()
        .map(|(f, s)| (f - landed, s.height))
        .collect();
    for ((at, height), (want_at, want)) in
        lifted.iter().zip([(1, 64.), (17, 183.967), (33, 105.534)])
    {
        assert_eq!(*at, want_at);
        assert!(near(*height, want, 0.01), "{lifted:?}");
    }
    // skillsLeap L0x1f4: each grows to 0.75 + its height / 100.
    let maxes: Vec<f32> = mover(&[("rise_from_the_ashes", 3)])
        .on_end
        .iter()
        .filter(|s| s.object == NOVA && s.fall.is_some())
        .map(|s| s.grow.and_then(|g| g.max).unwrap_or(0.))
        .collect();
    for (max, height) in maxes.iter().zip([64., 183.967, 105.534]) {
        assert!(near(*max, 0.75 + height / 100., 1e-3), "{maxes:?}");
    }
    let smash = births(&rise, SMASH);
    assert_eq!(smash.len(), 1);
    assert_eq!(smash[0].0 - landed, 41);
    assert_eq!(smash[0].1.scale, [2., 2.]);
    assert_eq!(rise.hits.len(), 5);
}

#[test]
fn wings_of_the_phoenix_fans_balls_behind_the_flight_every_48_px() {
    let wings = flight(&[("wings_of_the_phoenix", 1)]);
    let phoenixes = births(&wings, PHOENIX);
    let balls = births(&wings, BALL);
    assert_eq!(balls.len(), 3 * phoenixes.len());
    let path = crate::skill_mods::charge::path(&mover(&[("wings_of_the_phoenix", 1)]));
    // L0x233: 3 × rank balls 15° apart, centred straight behind the player's heading, 8 px out;
    // the phoenix turned with it. Each drop's things appear two frames after its move.
    for (fan, (frame, phoenix)) in balls.chunks(3).zip(&phoenixes) {
        let heading = path[frame - 2].1;
        assert!(heads(phoenix.angle, heading), "{} {heading}", phoenix.angle);
        for ((_, ball), turn) in fan.iter().zip([165., 180., 195.]) {
            assert!(heads(ball.angle, heading + turn), "{}", ball.angle);
            let (sin, cos) = (heading + turn).to_radians().sin_cos();
            let at = [phoenix.pos[0] + 8. * cos, phoenix.pos[1] - 8. * sin];
            assert!(near(ball.pos[0], at[0], 0.05) && near(ball.pos[1], at[1], 0.05));
        }
    }
    // The fan left just short of the dummy reaches it with its balls' 60 px tails; round the
    // loop one ball of the fan after the turn and one of the last cross it; the nova lands too.
    let blasts = births(&wings, BLAST);
    let on_dummy = blasts.iter().filter(|(_, b)| b.pos == DUMMY).count();
    assert_eq!(on_dummy, 3 + 1 + 1);
    assert_eq!(wings.hits.len(), on_dummy + 1);
}

#[test]
fn damage_only_nodes_leave_the_use_unchanged() {
    let base = flight(&[]);
    let same = flight(&[("flaming_pecker", 5)]);
    assert_eq!(same.hits, base.hits);
    assert_eq!(same.frames.len(), base.frames.len());
    assert_eq!(shares(&same), shares(&base));
}

#[test]
fn every_node_at_once_still_plays_bounded_uses() {
    let tree = [
        ("flaming_pecker", 5),
        ("swift_flight", 5),
        ("scorching_fury", 5),
        ("infused_flames", 5),
        ("searing_touch", 2),
        ("burning_screech", 5),
        ("collateral_burning", 5),
        ("afterburn", 5),
        ("molten_feathers", 5),
        ("mental_heat", 5),
        ("rise_from_the_ashes", 3),
        ("seed_of_fulmination", 3),
        ("wings_of_the_phoenix", 3),
        ("burning_wings", 3),
    ];
    for storm in uses(SKILL, &mods(&tree)).expect("uses").iter() {
        assert!(!storm.hits.is_empty());
        assert!(storm.frames.len() < 1200, "{}", storm.frames.len());
    }
}

#[test]
fn only_the_nodes_the_game_reads_its_own_way_keep_their_damage_off_the_hit() {
    let owned = crate::skill_mods::parity_tests::owned("pyromancer", "phoenix_flight");
    assert_eq!(
        owned,
        [
            "scorching_fury",
            "infused_flames",
            "searing_touch",
            "collateral_burning",
            "afterburn"
        ]
    );
}

const CLASS: &str = "pyromancer";

/// Damage-weighted hits a use is expected to land: Afterburn's cast roll at its own odds and the
/// chains at theirs.
fn expected(ranks: &[(&str, u32)]) -> f32 {
    let afterburn = ranks.iter().find(|(node, _)| *node == "afterburn");
    let odds = afterburn.map_or(0., |&rank| parity::cast_odds(CLASS, SKILL, &[rank]));
    let at = |lucky: bool| {
        let made = parity::rewritten(CLASS, SKILL, ranks, 0, |spawns, tree| {
            super::apply_rolled(spawns, tree, lucky)
        });
        crate::skill_mods::proc_odds::expected(SKILL, &made)
    };
    let miss = at(false);
    match odds > 0. {
        true => miss + odds * (at(true) - miss),
        false => miss,
    }
}

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_weighted(CLASS, SKILL, ranks)
}

/// Each of `together` landed by the sim and counted by the engine within 0.01.
fn agree(together: &[Ranks]) {
    let wrong: Vec<String> = (together.iter())
        .filter_map(|ranks| {
            let (sim, engine) = (expected(ranks), engine(ranks));
            let off = (sim - engine).abs() >= 0.01;
            off.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
        })
        .collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// Every rank of `a` with every rank of `b`.
fn pairs(a: &'static str, b: &'static str) -> Vec<Ranks<'static>> {
    let max = |id: &str| {
        let node = parity::nodes(CLASS, SKILL).iter().find(|n| n.id == id);
        node.map_or(0, |n| n.max_rank)
    };
    let ranks = (1..=max(a)).flat_map(|ra| (1..=max(b)).map(move |rb| (ra, rb)));
    ranks.map(|(ra, rb)| vec![(a, ra), (b, rb)]).collect()
}

/// Searing Touch's chain copies the hit that rolled it: the aura's, with Collateral Burning's
/// share, or the seed's blast's, the seed keeping the aura's 319.
#[test]
fn the_engine_follows_the_procs_that_roll_each_other() {
    let mut together = pairs(super::SEARING, super::SEED);
    together.extend(pairs(super::SEARING, super::COLLATERAL));
    agree(&together);
}

#[test]
fn the_engine_chains_three_and_more_procs_as_the_sim_does() {
    let nodes = [
        super::SEARING,
        super::SEED,
        super::COLLATERAL,
        super::BURNING,
        super::AFTERBURN,
    ];
    let sets = parity::proc_sets(CLASS, SKILL, &nodes);
    let wrong = parity::set_misses(&sets, expected, engine);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!(
        near(sim, 1., 0.01) && near(base, 1., 0.01),
        "sim {sim}, engine {base}"
    );
    let wrong = parity::misses(CLASS, SKILL, expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// Pass-through nodes move the landing round the loop, Burning Wings straightens the flight and
/// lays a patch under every carrier's hit: the rings, the fans and Afterburn's nova.
#[test]
fn the_engine_follows_the_nodes_the_sim_plays_together() {
    let mut together: Vec<Vec<(&str, u32)>> = Vec::new();
    for rank in 1..=5 {
        let ring = ("scorching_fury", rank);
        together.push(vec![ring, ("burning_wings", 1)]);
        together.push(vec![ring, ("mental_heat", 1)]);
        together.push(vec![ring, ("collateral_burning", 2), ("burning_wings", 3)]);
        together.push(vec![("afterburn", rank), ("burning_wings", 2)]);
    }
    for rank in 1..=3 {
        together.push(vec![
            ("wings_of_the_phoenix", rank),
            ("burning_wings", rank),
        ]);
    }
    together.extend([
        vec![("wings_of_the_phoenix", 3), ("scorching_fury", 5)],
        vec![
            ("wings_of_the_phoenix", 2),
            ("scorching_fury", 4),
            ("burning_wings", 1),
        ],
        vec![
            ("rise_from_the_ashes", 3),
            ("seed_of_fulmination", 1),
            ("burning_wings", 1),
        ],
        vec![("searing_touch", 2), ("burning_wings", 1)],
    ]);
    agree(&together);
}

/// Learned nodes and their ranks.
type Ranks<'a> = Vec<(&'a str, u32)>;

/// The engine's frames from one use's trigger to the next use's cast, and whether the next
/// cast's trigger frames come after them.
fn engine_period(ranks: &[(&str, u32)]) -> (f64, Option<bool>) {
    let spec = hsplanner_engine::calc::data::get_skills_by_class(CLASS)
        .iter()
        .find(|skill| skill.id == SKILL)
        .expect("phoenix flight");
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("{SKILL}:{node}"), *rank))
        .collect();
    let learned = hsplanner_engine::calc::subskill::learned_use_period(spec, &ranks);
    let period = learned.use_period.as_ref().expect("a use period");
    (period.extra_frames, period.trigger)
}

/// Frames until the player may cast again: the charge, then Rise from the Ashes' leap down to
/// the landing smash.
fn sim_period(ranks: &[(&str, u32)]) -> f64 {
    let charge = mover(ranks).life.expect("a charge");
    let landing = births(&flight(ranks), SMASH)
        .first()
        .map(|(frame, _)| *frame);
    landing.map_or(f64::from(charge), |frame| frame as f64)
}

#[test]
fn the_engines_use_period_follows_the_sims_charge_and_leap() {
    // PlayerMovement L0x1ca reaches CheckTalentUse whenever playerEffect[142] <= 0, so the next
    // cast's trigger runs during a charge; only Rise from the Ashes' leap sets [142]
    // (skillsCharge L0x2cd) until skillsLeap L0x433 clears it on landing.
    let mut cases: Vec<(Ranks, f64, bool)> = vec![
        (vec![], 22., false),
        (vec![("burning_wings", 2)], 22., false),
        (vec![("rise_from_the_ashes", 1)], 63., true),
        (
            vec![("rise_from_the_ashes", 3), ("burning_wings", 1)],
            63.,
            true,
        ),
    ];
    for node in super::THROUGH {
        cases.push((vec![(node, 1)], 50., false));
        cases.push((vec![(node, 1), ("burning_wings", 1)], 29., false));
        cases.push((vec![(node, 1), ("rise_from_the_ashes", 2)], 91., true));
        let all = vec![(node, 1), ("rise_from_the_ashes", 1), ("burning_wings", 3)];
        cases.push((all, 70., true));
    }
    for (ranks, frames, trigger) in &cases {
        assert_eq!(sim_period(ranks), *frames, "{ranks:?}");
        assert_eq!(engine_period(ranks), (*frames, Some(*trigger)), "{ranks:?}");
    }
}

#[test]
fn the_engines_use_rate_waits_for_the_loop_and_the_leap() {
    let rate = |ranks: &[(&str, u32)]| {
        let perf = parity::performance(CLASS, SKILL, ranks, None);
        perf.skill_costs[SKILL].cast_rate_max.expect("a use rate")
    };
    // The contact stop ends within the cast animation's period: S casts a second.
    let speed = rate(&[]);
    assert!(60. / 22. > speed, "{speed}");
    let looped = rate(&[("collateral_burning", 1)]);
    assert!((looped - 60. / 50.).abs() < 1e-9, "{looped}");
    let trigger = hsplanner_engine::calc::skills::trigger_frames(speed);
    let leap = rate(&[("rise_from_the_ashes", 1)]);
    assert!((leap - 60. / (trigger + 63.)).abs() < 1e-9, "{leap}");
}

#[test]
fn the_heaviest_uses_stay_far_under_the_drawn_cap() {
    // The preview draws at most 256 objects at once; these peak far below it, so it cuts no hit.
    for ranks in [
        [
            ("wings_of_the_phoenix", 3),
            ("scorching_fury", 5),
            ("afterburn", 5),
            ("swift_flight", 5),
        ],
        [
            ("wings_of_the_phoenix", 3),
            ("scorching_fury", 5),
            ("afterburn", 5),
            ("burning_wings", 3),
        ],
    ] {
        let peak = (0..super::CASTS)
            .flat_map(|cast| play(&spawns_of(&ranks, cast)).frames)
            .map(|shapes| shapes.len())
            .max();
        assert!(peak.is_some_and(|peak| peak < 128), "{ranks:?}: {peak:?}");
    }
}
