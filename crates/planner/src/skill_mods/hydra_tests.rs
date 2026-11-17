use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, replay_use, uses};
use std::collections::HashMap;

const HYDRA: &str = "Pyromancer_Hydra_obj";
const BALL: &str = "Pyromancer_Hydra_Fire_Ball_obj";
const BLAST: &str = "Explosion_Item_obj";
const NOVA: &str = "Pyromancer_Firenova_obj";
const COMET: &str = "Pyromancer_Comet_Hydra_obj";
const AURA: &str = "Pyromancer_Scorching_Aura_obj";
const BREATH: &str = "Pyromancer_Breath_of_Fire_Hydra_obj";
const DUMMY: [f32; 2] = [160., 0.];
const CURSOR: [f32; 2] = [160., -30.];

fn mods(ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (format!("hydra:{node}"), *rank))
        .collect();
    Mods::of("hydra", &ranks)
}

fn spawns_of(ranks: &[(&str, u32)], cast: u64) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill("hydra").expect("hydra").spawn;
    crate::skill_mods::apply_cast("hydra", base, &mods(ranks), cast).into_owned()
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    spawns_of(ranks, 0)
}

fn play_use(spawns: &[Spawn], n: u64) -> Timeline {
    let fx = crate::skill_sim::skill("hydra").expect("hydra");
    let hitbox = crate::hitbox::skill("hydra").expect("hydra hitboxes");
    replay_use(fx, hitbox, spawns, 1, n)
}

fn play(spawns: &[Spawn]) -> Timeline {
    play_use(spawns, 0)
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

fn frames_of(timeline: &Timeline, object: &str) -> Vec<usize> {
    births(timeline, object).iter().map(|(f, _)| *f).collect()
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

/// Where each hydra of a cast stands, from the cursor.
fn heads(spawns: &[Spawn]) -> Vec<[f32; 2]> {
    let hydras = spawns.iter().filter(|s| s.object == HYDRA);
    hydras
        .flat_map(|s| s.layout.iter().map(|p| p.offset))
        .collect()
}

#[test]
fn the_base_use_is_the_hydras_first_attack_exploding_once_on_the_dummy() {
    let base = spawns(&[]);
    assert_eq!(base.len(), 1);
    assert_eq!(heads(&base), [[0., 0.]]);
    let trail = base[0].trail.as_deref().expect("its shots");
    assert_eq!(trail.spawn.object, BALL);
    assert!(trail.next.is_none() && trail.spawn.on_end[0].procs.is_empty());
    let timeline = play(&base);
    assert_eq!(
        (frames_of(&timeline, BALL), shares(&timeline)),
        (vec![54], vec![1.])
    );
    assert_eq!(births(&timeline, HYDRA)[0].1.pos, CURSOR);
    assert_eq!(uses("hydra", &mods(&[])).expect("uses").len(), 8);
}

#[test]
fn a_hydra_counts_its_first_shot_down_only_once_fully_grown() {
    // Sep 16 Step_0: shootCooldown 0.2 s runs only while image_xscale >= maxScale, then the
    // attack sprite reaches image_index 3 after 23 frames.
    let timeline = play(&spawns(&[]));
    let grown = shapes(&timeline, HYDRA).find(|s| near(s.scale[0], 1.));
    assert_eq!(grown.map(|s| s.age), Some(19));
    assert_eq!(frames_of(&timeline, BALL), [18 + 12 + 1 + 23]);
    let kraken = play(&spawns(&[("scorching_kraken", 1)]));
    let grown = shapes(&kraken, HYDRA).find(|s| near(s.scale[0], 2.25));
    assert_eq!(grown.map(|s| s.age), Some(21));
    assert_eq!(frames_of(&kraken, BREATH)[0], 20 + 12 + 1 + 23);
}

#[test]
fn scorching_blast_may_set_off_a_firenova_on_the_dummy_its_explosion_hits() {
    let ranks = [("scorching_blast", 5)];
    let base = spawns(&ranks);
    let blast = &base[0].trail.as_deref().expect("shots").spawn.on_end[0];
    let nova = blast.procs.iter().find(|p| p.spawn.object == NOVA);
    assert!(nova.is_some_and(|p| near(p.chance, 0.2) && !p.once));
    let novas: Vec<usize> = (0..60)
        .map(|n| births(&play_use(&base, n), NOVA).len())
        .collect();
    assert!(novas.contains(&0) && novas.contains(&1), "{novas:?}");
    let lucky = play_use(
        &base,
        novas.iter().position(|&n| n == 1).expect("a nova") as u64,
    );
    let nova = births(&lucky, NOVA)[0].1;
    assert!(from(nova.pos, DUMMY) < 1., "{:?}", nova.pos);
    let widest = shapes(&lucky, NOVA).map(|s| s.scale[0]).fold(0., f32::max);
    assert!(widest > 0.7 && widest < 0.75, "{widest}");
    assert_eq!(shares(&lucky), [1., 1.]);
}

#[test]
fn chaotic_calamity_drops_a_comet_a_rank_on_the_dummy_each_from_higher_up() {
    let timeline = play(&spawns(&[("chaotic_calamity", 3)]));
    let comets = births(&timeline, COMET);
    assert_eq!(comets.len(), 3);
    let heights: Vec<f32> = comets.iter().map(|(_, s)| s.height).collect();
    assert_eq!(heights, [320., 420., 520.]);
    assert!(comets.iter().all(|(_, s)| from(s.pos, DUMMY) < 1.));
    let blasts = births(&timeline, BLAST);
    assert_eq!(blasts.len(), 4);
    assert!(blasts.windows(2).all(|w| w[0].0 < w[1].0));
    assert!(blasts[1..].iter().all(|(_, s)| near(s.scale[0], 0.55)));
    assert_eq!(shares(&timeline), [1.; 4]);
}

#[test]
fn burstfire_may_fire_a_second_ball_nine_frames_after_the_first() {
    let ranks = [("burstfire", 5)];
    let burst = |s: &[Spawn]| s[0].trail.as_deref().is_some_and(|t| t.next.is_some());
    let rolled: Vec<bool> = (0..40).map(|n| burst(&spawns_of(&ranks, n))).collect();
    assert!(rolled.contains(&false) && rolled.contains(&true));
    let lucky = spawns_of(&ranks, cast_where(&ranks, 40, burst));
    let timeline = play(&lucky);
    assert_eq!(frames_of(&timeline, BALL), [54, 63]);
    assert_eq!(shares(&timeline), [1., 1.]);
}

#[test]
fn cloning_combustion_may_raise_a_clone_40_px_away_that_fires_its_own_ball() {
    let ranks = [("cloning_combustion", 5)];
    let cloned = |s: &[Spawn]| heads(s).len() == 2;
    let lucky = spawns_of(&ranks, cast_where(&ranks, 60, cloned));
    let clone = &lucky[1];
    assert_eq!(clone.layout[0].frame, 55);
    assert!(near(from(clone.layout[0].offset, [0., 0.]), 40.));
    assert_eq!(clone.life, Some(70));
    let timeline = play(&lucky);
    assert_eq!(frames_of(&timeline, HYDRA), [0, 55]);
    assert_eq!(frames_of(&timeline, BALL), [54, 55 + 54]);
    assert_eq!(timeline.hits.len(), 2);
    let tried: Vec<usize> = (0..60)
        .map(|n| heads(&spawns_of(&ranks, n)).len())
        .collect();
    assert!(tried.iter().all(|&n| n <= 2), "{tried:?}");
}

#[test]
fn burning_essence_hits_the_dummy_with_an_aura_every_20_frames_once_grown() {
    let essence = spawns(&[("burning_essence", 2)]);
    let aura = essence.iter().find(|s| s.object == AURA).expect("auras");
    assert_eq!((aura.scale, aura.share), (Some([1.25, 1.25]), Some(0.5)));
    let timeline = play(&essence);
    assert_eq!(frames_of(&timeline, AURA), [38, 58]);
    assert!(births(&timeline, AURA).iter().all(|(_, s)| s.pos == CURSOR));
    let mut hit = shares(&timeline);
    hit.sort_by(f32::total_cmp);
    assert_eq!(hit, [0.5, 0.5, 1.]);
    // Each aura inherits the hydra's comets (InheritModifiers in Step_0 L0x37).
    let comets = play(&spawns(&[("burning_essence", 2), ("chaotic_calamity", 1)]));
    assert_eq!(births(&comets, COMET).len(), 3);
}

#[test]
fn searing_snakes_summons_every_head_at_once_round_the_cursor() {
    let three = spawns(&[("searing_snakes", 1), ("more_heads_more_fire", 1)]);
    let at = heads(&three);
    assert_eq!(at.len(), 3);
    assert_eq!(at[..2], [[0., 0.], [0., 0.]]);
    assert!(near(at[2][0], -12.) && near(at[2][1], 20.7846), "{at:?}");
    let timeline = play(&three);
    assert_eq!(frames_of(&timeline, BALL), [54; 3]);
    assert_eq!(timeline.hits.len(), 3);
    let eleven = heads(&spawns(&[
        ("searing_snakes", 1),
        ("more_heads_more_fire", 5),
    ]));
    assert_eq!(eleven.len(), 11);
    assert!(eleven[2..].iter().all(|at| near(from(*at, [0., 0.]), 88.)));
    let herded = [
        ("searing_snakes", 1),
        ("more_heads_more_fire", 5),
        ("hydra_herder", 1),
    ];
    assert_eq!(heads(&spawns(&herded)).len(), 5);
    // floor(1 × 0.5) heads: the Herder leaves Searing Snakes nothing to summon.
    let none = spawns(&[("searing_snakes", 1), ("hydra_herder", 1)]);
    assert!(none.is_empty() && play(&none).hits.is_empty());
}

#[test]
fn hydra_herder_walks_the_hydra_until_its_box_is_128_px_from_the_players() {
    let walked = |rank| from(heads(&spawns(&[("hydra_herder", rank)]))[0], [0., 0.]);
    assert!(near(walked(1), 7.5) && near(walked(3), 10.5));
    let at = heads(&spawns(&[("hydra_herder", 1)]))[0];
    assert!(at[0] < 0. && at[1] > 0., "{at:?}");
    assert_eq!(play(&spawns(&[("hydra_herder", 3)])).hits.len(), 1);
    // Two hydras on the cursor block each other's steps (Step_0 L0x1c-L0x1e).
    let pair = heads(&spawns(&[
        ("hydra_herder", 1),
        ("searing_snakes", 1),
        ("more_heads_more_fire", 2),
    ]));
    assert_eq!(pair[..2], [[0., 0.], [0., 0.]]);
}

#[test]
fn scorching_kraken_raises_one_massive_hydra_that_breathes_twice_a_shot() {
    let kraken = spawns(&[
        ("scorching_kraken", 1),
        ("more_heads_more_fire", 5),
        ("searing_snakes", 1),
    ]);
    assert_eq!(heads(&kraken).len(), 1);
    assert_eq!(kraken[0].grow.and_then(|g| g.max), Some(2.25));
    assert_eq!(kraken[0].fade, 45);
    let timeline = play(&kraken);
    assert!(births(&timeline, BALL).is_empty());
    let breaths = births(&timeline, BREATH);
    assert_eq!(breaths.len(), 2);
    // As Breath of Fire's second breath: 21 frames after the first shows.
    assert_eq!(breaths[1].0 - breaths[0].0, 21);
    assert!(from(breaths[1].1.pos, breaths[0].1.pos) < 1e-3);
    assert_eq!(shares(&timeline), [1., 1.]);
    // The first breath's hit zeroes its comets, so the second carries none.
    let comets = play(&spawns(&[("scorching_kraken", 1), ("chaotic_calamity", 2)]));
    assert_eq!(births(&comets, COMET).len(), 2);
    // Like Breath of Fire's breath, both of its scales take the build's size bonus.
    let mut sized = mods(&[("scorching_kraken", 1)]);
    sized.size_bonus = 0.2;
    let base = &crate::skill_sim::skill("hydra").expect("hydra").spawn;
    let wide = crate::skill_mods::apply_cast("hydra", base, &sized, 0);
    let breath = &wide[0].trail.as_deref().expect("shots").spawn;
    assert_eq!(
        breath.scale.map(|[x, y]| near(x, 0.45) && near(y, 1.7)),
        Some(true)
    );
}

#[test]
fn lifetime_range_speed_and_damage_nodes_leave_the_first_attack_as_it_is() {
    let base = play(&spawns(&[]));
    for node in [
        "lasting_fire",
        "fiery_snakebite",
        "flaming_reach",
        "flaming_reflexes",
        "slitherin_army",
        "more_heads_more_fire",
    ] {
        let cast = spawns(&[(node, 5)]);
        assert_eq!(heads(&cast).len(), 1, "{node}");
        let cast = play(&cast);
        assert_eq!(
            (shares(&cast), &cast.hits),
            (shares(&base), &base.hits),
            "{node}"
        );
    }
}

#[test]
fn only_the_nodes_the_game_reads_its_own_way_keep_their_damage_off_the_hit() {
    let owned = crate::skill_mods::parity_tests::owned("pyromancer", "hydra");
    assert_eq!(owned, ["burstfire", "burning_essence"]);
}

const CLASS: &str = "pyromancer";

use super::parity::{auras_per_attack, expected};

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_weighted(CLASS, "hydra", ranks)
}

fn agree(together: &[&[(&str, u32)]], engine: impl Fn(&[(&str, u32)]) -> f32) {
    for ranks in together {
        let (sim, engine) = (expected(ranks), engine(ranks));
        assert!(
            (sim - engine).abs() < 0.01,
            "{ranks:?}: sim {sim}, engine {engine}"
        );
    }
}

#[test]
fn burning_essence_lands_the_auras_a_grown_hydra_pulses_in_one_attack_period() {
    assert!(near(auras_per_attack(), 4.55), "{}", auras_per_attack());
    let essence = parity::nodes(CLASS, "hydra")
        .iter()
        .find(|node| node.id == "burning_essence")
        .expect("burning essence");
    for rank in 1..=essence.max_rank {
        let share = essence.planner_stat("of_total_damage", rank) as f32 / 100.;
        let auras = engine(&[("burning_essence", rank)]) - engine(&[]);
        let want = auras_per_attack() * share;
        assert!((auras - want).abs() < 0.01, "r{rank}: {auras}, want {want}");
    }
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!(near(sim, 1.) && near(base, 1.), "sim {sim}, engine {base}");
    let wrong = parity::misses(CLASS, "hydra", expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_krakens_breaths_and_the_auras_carriers_as_the_sim_plays_them() {
    agree(
        &[
            &[("scorching_kraken", 1), ("chaotic_calamity", 3)],
            &[("scorching_blast", 5), ("scorching_kraken", 1)],
            &[("burstfire", 5), ("scorching_kraken", 2)],
            &[("cloning_combustion", 5), ("scorching_kraken", 1)],
            &[("scorching_kraken", 3), ("burning_essence", 5)],
            &[("scorching_kraken", 1), ("hydra_herder", 3)],
            &[
                ("scorching_kraken", 1),
                ("hydra_herder", 1),
                ("more_heads_more_fire", 5),
            ],
            &[("burning_essence", 1), ("chaotic_calamity", 1)],
            &[("burning_essence", 5), ("chaotic_calamity", 3)],
            &[
                ("scorching_kraken", 1),
                ("burning_essence", 3),
                ("chaotic_calamity", 2),
            ],
            &[("scorching_blast", 5), ("burning_essence", 1)],
            &[("scorching_blast", 3), ("burning_essence", 5)],
            &[("burstfire", 5), ("burning_essence", 5)],
            &[("hydra_herder", 3), ("burning_essence", 5)],
        ],
        engine,
    );
}

#[test]
fn with_searing_snakes_one_cast_fields_every_hydra_the_engine_counts() {
    // Searing Snakes summons the whole cap at once: the engine's hits per hydra x its count.
    let fielded = |ranks: &[(&str, u32)]| {
        let count = parity::performance(CLASS, "hydra", ranks, None).entity_count;
        engine(ranks) * count.map_or(1., |count| count.1) as f32
    };
    let snakes = ("searing_snakes", 1);
    let mut together: Vec<Vec<(&str, u32)>> = (1..=5)
        .flat_map(|heads| {
            let more = ("more_heads_more_fire", heads);
            [vec![snakes, more], vec![snakes, more, ("hydra_herder", 3)]]
        })
        .collect();
    together.push(vec![
        ("chaotic_calamity", 3),
        snakes,
        ("more_heads_more_fire", 5),
    ]);
    together.push(vec![
        ("scorching_kraken", 1),
        snakes,
        ("more_heads_more_fire", 5),
    ]);
    let together: Vec<&[(&str, u32)]> = together.iter().map(Vec::as_slice).collect();
    agree(&together, fielded);
}
