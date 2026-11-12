//! The game's dummy hits for the engine's parity tests: the cast's Celestial Collapse roll, Loss
//! of Orbit's lunges and the procs of Tidalwaves and Devastating Nebula counted at their odds.
use crate::skill_effects::{Lunge, Orbit, Proc};
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::Spawn;

const CLASS: &str = "exo";
const SKILL: &str = "lunar_orbit";
const TSUNAMI: &str = "Exo_Tsunami_obj";
/// The nodes whose rolls a use's hits hang on; the first rolls once a cast.
const ROLLED: [&str; 4] = [
    "celestial_collapse",
    "loss_of_orbit",
    "tidalwaves",
    "devastating_nebula",
];

type Ranks = Vec<(&'static str, u32)>;

/// `spawn` with its lunges and its tsunami and nebula procs at the chances `at`.
fn forced(spawn: &Spawn, at: [f32; 3]) -> Spawn {
    let [lunge, tidal, nebula] = at;
    let proc = |proc: &Proc| Proc {
        chance: if proc.spawn.object == TSUNAMI {
            tidal
        } else {
            nebula
        },
        spawn: forced(&proc.spawn, at),
        ..proc.clone()
    };
    Spawn {
        orbit: spawn.orbit.map(|orbit| Orbit {
            lunge: orbit.lunge.map(|old| Lunge {
                chance: lunge,
                ..old
            }),
            ..orbit
        }),
        on_end: spawn.on_end.iter().map(|end| forced(end, at)).collect(),
        procs: spawn.procs.iter().map(proc).collect(),
        ..spawn.clone()
    }
}

/// The odds, 0 to 1, of `node`'s roll in `ranks`.
fn odds(ranks: &[(&str, u32)], node: &str) -> f32 {
    let learned = ranks.iter().find(|(id, _)| *id == node);
    learned.map_or(0., |&rank| parity::cast_odds(CLASS, SKILL, &[rank]))
}

/// Damage-weighted hits a use is expected to land: each of the `ROLLED` rolls comes up with its
/// odds, over a turn of casts. What the procs make adds up multilinearly in their chances; a
/// lunge, which only moves its moon, is taken at its two extremes.
pub(super) fn expected(ranks: &[(&str, u32)]) -> f32 {
    let odds = ROLLED.map(|node| odds(ranks, node));
    let turn = crate::skill_mods::casts(SKILL, 1);
    let one = |cast: u64| {
        let corner = |up: u32| {
            let comes = |i: usize| up & (1 << i) != 0;
            let weight: f32 = (0..ROLLED.len())
                .map(|i| if comes(i) { odds[i] } else { 1. - odds[i] })
                .product();
            if weight <= 0. {
                return 0.;
            }
            let made = parity::rewritten(CLASS, SKILL, ranks, cast, |spawns, tree| {
                super::apply_rolled(spawns, tree, comes(0))
            });
            let at = [1, 2, 3].map(|i| if comes(i) { 1. } else { 0. });
            let spawns: Vec<Spawn> = made.iter().map(|spawn| forced(spawn, at)).collect();
            weight * parity::weighted(&parity::play(SKILL, &spawns, 1))
        };
        (0..1 << ROLLED.len()).map(corner).sum::<f32>()
    };
    (0..turn).map(one).sum::<f32>() / turn as f32
}

fn engine(ranks: &[(&str, u32)]) -> f32 {
    parity::engine_hits(CLASS, SKILL, ranks)
}

/// Each of `together` landed by the sim and counted by the engine within 0.01.
fn agree(together: &[Ranks]) {
    let miss = |ranks: &Ranks| {
        let (sim, engine) = (expected(ranks), engine(ranks));
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

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let (sim, base) = (expected(&[]), engine(&[]));
    let near = |hits: f32| (hits - 2.6).abs() < 1e-4;
    assert!(near(sim) && near(base), "sim {sim}, engine {base}");
    assert_eq!(
        parity::owned(CLASS, SKILL),
        [
            "celestial_collapse",
            "tidalwaves",
            "lunar_architect",
            "orbit_breakout",
            "devastating_nebula"
        ]
    );
    let wrong = parity::misses(CLASS, SKILL, expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_engine_follows_lunar_architects_moons_and_crescent_moons_rings() {
    let mut together = pairs(("extrasolar_moons", 4), ("lunar_architect", 3));
    for crescent in 1..=3 {
        for extrasolar in 0..=4 {
            for architect in 0..=3 {
                let ranks = [
                    ("crescent_moon", crescent),
                    ("extrasolar_moons", extrasolar),
                    ("lunar_architect", architect),
                ];
                together.push(ranks.into_iter().filter(|(_, rank)| *rank > 0).collect());
            }
        }
    }
    // Crescent moons carry only Tidalwaves; Orbit Breakout's moons end too far out to collapse.
    for node in [
        ("devastating_nebula", 3),
        ("celestial_collapse", 5),
        ("orbit_breakout", 3),
        ("tidalwaves", 8),
    ] {
        together.extend(pairs(("crescent_moon", 3), node));
    }
    together.extend(pairs(("celestial_collapse", 5), ("orbit_breakout", 3)));
    agree(&together);
}

#[test]
fn the_engine_follows_full_moon_and_magnetic_pull_on_every_kind_of_cast() {
    let mut together = pairs(("full_moon", 5), ("magnetic_pull", 4));
    for cast in ["lunar_architect", "crescent_moon"] {
        together.extend(pairs((cast, 3), ("full_moon", 5)));
        together.extend(pairs((cast, 3), ("magnetic_pull", 4)));
    }
    for crescent in 1..=3 {
        for both in pairs(("full_moon", 5), ("magnetic_pull", 4)) {
            together.push([vec![("crescent_moon", crescent)], both].concat());
        }
    }
    agree(&together);
}

#[test]
fn the_engine_follows_the_procs_the_shrapnel_and_the_breakouts_together() {
    let mut together = pairs(("tidalwaves", 8), ("devastating_nebula", 3));
    for node in [
        ("full_moon", 5),
        ("magnetic_pull", 4),
        ("lunar_architect", 3),
    ] {
        together.extend(pairs(("celestial_collapse", 5), node));
        together.extend(pairs(("orbit_breakout", 3), node));
    }
    for architect in 1..=3 {
        for node in [("full_moon", 5), ("magnetic_pull", 4)] {
            for ranks in pairs(("celestial_collapse", 5), node) {
                together.push([vec![("lunar_architect", architect)], ranks].concat());
            }
        }
    }
    together.extend([
        vec![
            ("celestial_collapse", 5),
            ("tidalwaves", 8),
            ("devastating_nebula", 3),
        ],
        vec![("celestial_collapse", 2), ("devastating_nebula", 1)],
        vec![
            ("lunar_architect", 1),
            ("tidalwaves", 4),
            ("devastating_nebula", 2),
        ],
        vec![
            ("full_moon", 3),
            ("magnetic_pull", 2),
            ("tidalwaves", 8),
            ("devastating_nebula", 3),
        ],
        vec![
            ("orbit_breakout", 2),
            ("lunar_architect", 2),
            ("devastating_nebula", 3),
        ],
    ]);
    agree(&together);
}
