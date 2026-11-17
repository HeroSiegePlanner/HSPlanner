//! The game's dummy hits for the engine's parity tests: the cast's Everburning Fire and Burst of
//! Power rolls and Explosive Flares' proc counted at their odds.
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::Spawn;

const CLASS: &str = "exo";
const SKILL: &str = "solar_flare";
const BURNING: &str = "everburning_fire";
const BURST: &str = "burst_of_power";

type Ranks = Vec<(&'static str, u32)>;

/// The odds, 0 to 1, of `node`'s cast roll in `ranks`.
fn odds(ranks: &[(&str, u32)], node: &str) -> f32 {
    let learned = ranks.iter().find(|(id, _)| *id == node);
    learned.map_or(0., |&rank| parity::cast_odds(CLASS, SKILL, &[rank]))
}

/// The `cast`-th use with its Everburning Fire roll `burning` and Burst of Power roll `burst`.
fn made(ranks: &[(&str, u32)], cast: u64, burning: bool, burst: bool) -> Vec<Spawn> {
    parity::rewritten(CLASS, SKILL, ranks, cast, |spawns, tree| {
        super::apply_rolls(spawns, tree, burning, burst)
    })
}

/// `at` over both cast rolls, each coming up with its odds, averaged over a turn of casts.
fn over_rolls(ranks: &[(&str, u32)], at: impl Fn(u64, bool, bool) -> f32) -> f32 {
    let (burning, burst) = (odds(ranks, BURNING), odds(ranks, BURST));
    let turn = crate::skill_mods::casts(SKILL, 1);
    let chance = |odds: f32, up: bool| if up { odds } else { 1. - odds };
    let one = |cast: u64| {
        let corners = [(false, false), (true, false), (false, true), (true, true)];
        let corner = |(lit, lucky): (bool, bool)| {
            let weight = chance(burning, lit) * chance(burst, lucky);
            if weight <= 0. {
                0.
            } else {
                weight * at(cast, lit, lucky)
            }
        };
        corners.into_iter().map(corner).sum::<f32>()
    };
    (0..turn).map(one).sum::<f32>() / turn as f32
}

/// Damage-weighted hits a use is expected to land, the cast's rolls and Explosive Flares' proc at
/// their odds.
fn expected(ranks: &[(&str, u32)]) -> f32 {
    over_rolls(ranks, |cast, burning, burst| {
        parity::expected(SKILL, &made(ranks, cast, burning, burst))
    })
}

/// Hits a use lands with Burst of Power's bonus whole on its lucky casts, as the engine's pools
/// take it: the sim dilutes it by the subtree's other damage (L0xe1).
fn undiluted(ranks: &[(&str, u32)]) -> f32 {
    let bonus = burst_bonus(ranks);
    over_rolls(ranks, |cast, burning, burst| {
        let timeline = parity::play(SKILL, &made(ranks, cast, burning, burst), 1);
        let hits = timeline.hits.len() as f32;
        if burst { hits * (1. + bonus) } else { hits }
    })
}

/// Burst of Power's fire damage on a lucky cast at its rank in `ranks`, 0 to 1.
fn burst_bonus(ranks: &[(&str, u32)]) -> f32 {
    let (chance, rank) = (
        odds(ranks, BURST),
        ranks.iter().find(|(id, _)| *id == BURST),
    );
    let node = parity::nodes(CLASS, SKILL)
        .iter()
        .find(|node| node.id == BURST);
    match (node, rank) {
        (Some(node), Some(&(_, rank))) if chance > 0. => {
            node.planner_stat("fire_skill_damage", rank) as f32 / 100. / chance
        }
        _ => 0.,
    }
}

/// The engine averages Burst of Power's roll, which the sim draws as a share of the whole cast,
/// into its hit as fire skill damage.
fn engine(ranks: &[(&str, u32)]) -> f32 {
    let burst = parity::engine_skill_roll(CLASS, SKILL, ranks, BURST);
    parity::engine_weighted(CLASS, SKILL, ranks) * burst
}

/// Each of `together` landed by `sim` and counted by the engine within 0.01.
fn agree(sim: impl Fn(&[(&str, u32)]) -> f32, together: &[Ranks]) {
    let miss = |ranks: &Ranks| {
        let (sim, engine) = (sim(ranks), engine(ranks));
        let off = (sim - engine).abs() >= 0.01;
        off.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
    };
    let wrong: Vec<String> = together.iter().filter_map(miss).collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// Every rank of `a` with every rank of `b`.
fn pairs(a: (&'static str, u32), b: (&'static str, u32)) -> Vec<Ranks> {
    let with = move |at| (1..=b.1).map(move |bt| vec![(a.0, at), (b.0, bt)]);
    (1..=a.1).flat_map(with).collect()
}

/// `ranks` without the nodes at rank 0.
fn learned(ranks: &[(&'static str, u32)]) -> Ranks {
    ranks
        .iter()
        .copied()
        .filter(|(_, rank)| *rank > 0)
        .collect()
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    let near = |hits: f32| (hits - 3.).abs() < 1e-4;
    assert!(near(sim) && near(base), "sim {sim}, engine {base}");
    assert_eq!(
        parity::owned(CLASS, SKILL),
        ["explosive_flares", "solars_edge"]
    );
    let wrong = parity::misses(CLASS, SKILL, expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    agree(expected, &pairs((BURST, 5), ("solar_fireworks", 4)));
    agree(expected, &pairs((BURST, 5), ("searing_flares", 3)));
}

#[test]
fn the_engine_follows_the_transformations_and_the_flares_they_ring_out() {
    let mut together = Vec::new();
    // Solar Orbs win over Grand Flare, which wins over Sun's Orbit (L0xeb-L0x139).
    for orbs in 0..=3 {
        for grand in 0..=3 {
            for suns in 0..=3 {
                together.push(learned(&[
                    ("solar_orbs", orbs),
                    ("grand_flare", grand),
                    ("suns_orbit", suns),
                ]));
            }
        }
    }
    for ring in [("grand_flare", 3), ("solar_orbs", 3)] {
        together.extend(pairs(ring, ("solar_fireworks", 4)));
        together.extend(pairs(ring, ("searing_flares", 3)));
    }
    together.extend(pairs(("solar_orbs", 3), (BURST, 5)));
    for suns in 1..=3 {
        for fireworks in 0..=4 {
            for searing in 0..=3 {
                together.push(learned(&[
                    ("suns_orbit", suns),
                    ("solar_fireworks", fireworks),
                    ("searing_flares", searing),
                ]));
            }
        }
    }
    together.retain(|ranks| !ranks.is_empty());
    agree(expected, &together);
}

#[test]
fn explosive_flares_go_off_only_on_a_cast_that_burns() {
    let mut together = pairs(("explosive_flares", 5), (BURNING, 5));
    for node in [
        ("grand_flare", 2),
        ("solar_orbs", 3),
        ("suns_orbit", 1),
        ("solar_fireworks", 4),
        ("searing_flares", 2),
    ] {
        together.push(vec![("explosive_flares", 5), (BURNING, 3), node]);
    }
    agree(expected, &together);
}

#[test]
fn burst_of_powers_flares_follow_the_sim_under_grand_flare_and_suns_orbit() {
    let mut together = pairs(("grand_flare", 3), (BURST, 5));
    together.extend(pairs(("suns_orbit", 3), (BURST, 5)));
    together.push(vec![("grand_flare", 1), (BURST, 3), ("solar_fireworks", 2)]);
    agree(undiluted, &together);
}
