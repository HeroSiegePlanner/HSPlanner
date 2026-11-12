//! The engine's dummy hits held to the game's as the preview plays Asteroid: Aftershock's second
//! cast and Perseids at their odds, the cataclysm's random drops integrated over their ring.
use crate::skill_mods::parity_tests as parity;
use crate::skill_mods::proc_odds;
use crate::skill_sim::{Placement, Spawn};

const CLASS: &str = "exo";
const SKILL: &str = "asteroid";
const CATACLYSM: &str = "galactic_cataclysm";
/// Steps across and around the cataclysm's ring.
const RING_STEPS: (u32, u32) = (45, 180);
/// Drops replayed in one use, with their blasts and Perseids under the sim's live-object cap.
const BATCH: usize = 50;
/// The dummy's 12 px disk and a pixel to spare.
const DUMMY_REACH: f32 = 13.;

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    let mods = parity::mods(SKILL, ranks);
    let base = &crate::skill_sim::skill(SKILL).expect("asteroid").spawn;
    crate::skill_mods::apply(SKILL, base, &mods).into_owned()
}

/// Damage-weighted hits a cast is expected to land. Frostbite ticks and the frostbitten bonus stay
/// out: the engine counts them in its ailment DPS and under its Frost Bitten enemy condition.
fn expected(ranks: &[(&str, u32)]) -> f32 {
    let one = |spawn: &Spawn| {
        let spawn = Spawn {
            frostbite: None,
            ..spawn.clone()
        };
        match spawn.scatter {
            Some(_) => rain(&spawn),
            None => proc_odds::expected(SKILL, &[spawn]),
        }
    };
    spawns(ranks).iter().map(one).sum()
}

/// The cataclysm's drops times one drop's mean hits over the ring it falls on, distance and angle
/// uniform as the sim's scatter draws them; only the spots its explosion can reach are replayed.
fn rain(rain: &Spawn) -> f32 {
    let ring = rain.scatter.expect("a ring");
    let (across, around) = RING_STEPS;
    let spot = |(i, j): (u32, u32)| {
        let distance = ring.from + ring.spread * (i as f32 + 0.5) / across as f32;
        let degrees = 360. * (j as f32 + 0.5) / around as f32;
        let (sin, cos) = degrees.to_radians().sin_cos();
        [cos * distance, -sin * distance]
    };
    let dummy = crate::skill_sim::skill(SKILL).expect("asteroid").dummy;
    let reach = reach(rain);
    let near = |at: &[f32; 2]| (at[0] - dummy).hypot(at[1]) <= reach;
    let grid = (0..across).flat_map(|i| (0..around).map(move |j| (i, j)));
    let spots: Vec<Placement> = (grid.map(spot).filter(near))
        .map(|offset| Placement { frame: 1, offset })
        .collect();
    let batch = |drops: &[Placement]| {
        let spawn = Spawn {
            layout: drops.to_vec(),
            scatter: None,
            ..rain.clone()
        };
        proc_odds::expected(SKILL, &[spawn])
    };
    let hits: f32 = spots.chunks(BATCH).map(batch).sum();
    hits / (across * around) as f32 * rain.layout.len() as f32
}

/// px from the dummy beyond which a drop's explosion cannot hit it.
fn reach(rain: &Spawn) -> f32 {
    let blast = rain.on_end.first().expect("an explosion");
    let objects = &crate::hitbox::skill(SKILL).expect("hitboxes").objects;
    let object = objects.iter().find(|o| o.name == blast.object);
    let object = object.expect("the explosion's hitbox");
    let scale = blast.scale.map_or(object.scale[0] as f32, |scale| scale[0]);
    object.reach * scale + DUMMY_REACH
}

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_hits(CLASS, SKILL, ranks)
}

/// Each of `together` counted by the game and by the engine, `within` apart.
fn agree(together: &[Vec<(&str, u32)>], within: f32) {
    let wrong: Vec<String> = (together.iter())
        .filter_map(|ranks| {
            let (sim, engine) = (expected(ranks), engine(ranks));
            let off = (sim - engine).abs() >= within;
            off.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
        })
        .collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// Every other node at its first and its last rank, learned alongside `with`.
fn every_node_with(with: &[(&'static str, u32)]) -> Vec<Vec<(&'static str, u32)>> {
    let nodes = parity::nodes(CLASS, SKILL).iter();
    let others = nodes.filter(|node| with.iter().all(|(id, _)| *id != node.id));
    let ranks = others.flat_map(|node| [(node.id.as_str(), 1), (node.id.as_str(), node.max_rank)]);
    ranks.map(|rank| [with, &[rank]].concat()).collect()
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    assert!((sim - base).abs() < 0.01, "sim {sim}, engine {base}");
    let wrong = parity::misses(CLASS, SKILL, expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_the_nodes_the_game_plays_together() {
    let mut together = Vec::new();
    for node in ["asteroid_aftershock", "perseids"] {
        for rank in [1, 3, 5] {
            together.extend(every_node_with(&[(node, rank)]));
        }
    }
    for rank in 1..=3 {
        together.extend(every_node_with(&[("geomagnetic_reversal", rank)]));
        together.extend(every_node_with(&[("planetary_collapse", rank)]));
    }
    together.retain(|ranks| ranks.iter().all(|(id, _)| *id != CATACLYSM));
    agree(&together, 0.01);
}

#[test]
fn the_engine_follows_the_cataclysms_rain_under_every_node_that_reaches_it() {
    let mut together = every_node_with(&[(CATACLYSM, 1)]);
    together.extend(every_node_with(&[(CATACLYSM, 3)]));
    for planetary in 0..=3 {
        for world_ending in 0..=5 {
            let size = [
                ("planetary_collapse", planetary),
                ("world_ending_impact", world_ending),
            ];
            let learned = size.into_iter().filter(|(_, rank)| *rank > 0);
            together.push([(CATACLYSM, 2)].into_iter().chain(learned).collect());
        }
    }
    // The ring's quadrature moves a 45-drop rain of grown asteroids by up to 0.03.
    agree(&together, 0.05);
}
