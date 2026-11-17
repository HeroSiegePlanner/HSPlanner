//! Shared helpers of the parity tests that hold the engine's training-dummy hits to the preview.
use super::{Mods, Tree};
use crate::skill_sim::{Spawn, Timeline, replay_use};
use hsplanner_engine::calc::build::{
    BuildPerformance, BuildPerformanceDeps, compute_build_performance,
};
use hsplanner_engine::calc::data;
use hsplanner_engine::calc::subskill::subskill_key;
use hsplanner_engine::calc::types::SubskillNodeSpec;
use std::collections::{HashMap, HashSet};

/// The build the engine's numbers are taken on, as build_dps_snapshot_tests takes them.
const LEVEL: u32 = 100;
const SKILL_RANK: u32 = 20;

/// The preview's modifiers of `skill` with only `ranks` learned.
pub(crate) fn mods(skill: &str, ranks: &[(&str, u32)]) -> Mods {
    let ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (subskill_key(skill, node), *rank))
        .collect();
    Mods::of(skill, &ranks)
}

/// What `rewrite`, a module's hook that decides the cast's own rolls, makes of the `cast`-th use
/// of `class`'s `skill` with `ranks`.
pub(super) fn rewritten(
    class: &str,
    skill: &str,
    ranks: &[(&str, u32)],
    cast: u64,
    rewrite: impl Fn(&[Spawn], &Tree) -> Vec<Spawn>,
) -> Vec<Spawn> {
    let mods = mods(skill, ranks);
    let nodes = nodes(class, skill);
    let base = &crate::skill_sim::skill(skill).expect("skill motion").spawn;
    rewrite(
        base,
        &Tree {
            nodes,
            mods: &mods,
            cast,
        },
    )
}

/// The odds, 0 to 1, of the cast roll of the first node in `ranks` that rolls one: its proc's
/// chance, else its Multicast chance.
pub(crate) fn cast_odds(class: &str, skill: &str, ranks: &[(&str, u32)]) -> f32 {
    let odds = |&(id, rank): &(&str, u32)| {
        let node = nodes(class, skill).iter().find(|node| node.id == id)?;
        let proc = node.proc.as_ref().map(|proc| {
            proc.chance.base.unwrap_or(0.) + proc.chance.per_rank.unwrap_or(0.) * f64::from(rank)
        });
        let chance = proc.unwrap_or_else(|| node.planner_stat("multicast_chance", rank)) / 100.;
        (chance > 0.).then_some(chance as f32)
    };
    ranks.iter().find_map(odds).unwrap_or(0.)
}

/// Damage-weighted hits a use is expected to land when its module rolls the cast itself: `hook`
/// makes the cast with every roll coming up `lucky`, counted at the odds of the one node in
/// `ranks` that rolls, averaged over a turn of the module's casts.
pub(super) fn expected_over_rolls(
    class: &str,
    skill: &str,
    ranks: &[(&str, u32)],
    hook: impl Fn(&[Spawn], &Tree, bool) -> Vec<Spawn>,
) -> f32 {
    let chance = cast_odds(class, skill, ranks);
    let turn = crate::skill_mods::casts(skill, 1);
    let one = |cast: u64| {
        let at = |lucky| {
            let made = rewritten(class, skill, ranks, cast, |spawns, tree| {
                hook(spawns, tree, lucky)
            });
            expected(skill, &made)
        };
        let miss = at(false);
        miss + chance * (at(true) - miss)
    };
    (0..turn).map(one).sum::<f32>() / turn as f32
}

/// One use of `spawns` with `projectiles` in the volley, rolled with the first use's dice.
pub(crate) fn play(skill: &str, spawns: &[Spawn], projectiles: u32) -> Timeline {
    let fx = crate::skill_sim::skill(skill).expect("skill motion");
    let hitbox = crate::hitbox::skill(skill).expect("skill hitboxes");
    replay_use(fx, hitbox, spawns, projectiles, 0)
}

/// Damage-weighted hits of one use: the shares of its hits.
pub(crate) fn weighted(timeline: &Timeline) -> f32 {
    timeline.effects.damage.iter().map(|d| d.share).sum()
}

/// `spawn` with every creation roll and every chancy proc in it set to `chance`.
pub(crate) fn rolled(spawn: &Spawn, chance: f32) -> Spawn {
    let all = |spawns: &[Spawn]| spawns.iter().map(|s| rolled(s, chance)).collect();
    Spawn {
        chance: spawn.chance.map(|_| chance),
        on_end: all(&spawn.on_end),
        on_low: all(&spawn.on_low),
        trail: (spawn.trail.as_deref()).map(|trail| {
            Box::new(crate::skill_motion::Trail {
                spawn: rolled(&trail.spawn, chance),
                ..trail.clone()
            })
        }),
        procs: (spawn.procs.iter())
            .map(|proc| crate::skill_effects::Proc {
                chance: if proc.chance < 1. { chance } else { 1. },
                spawn: rolled(&proc.spawn, chance),
                ..proc.clone()
            })
            .collect(),
        ..spawn.clone()
    }
}

/// The first creation roll or chancy proc among `spawns` and what they make.
pub(crate) fn odds(spawns: &[Spawn]) -> Option<f32> {
    spawns.iter().find_map(|spawn| {
        let trail = spawn.trail.as_deref().map(|trail| &trail.spawn);
        let made = spawn.procs.iter().map(|proc| &proc.spawn);
        let chancy = spawn.procs.iter().map(|proc| proc.chance).find(|c| *c < 1.);
        spawn
            .chance
            .or(chancy)
            .or_else(|| odds(&spawn.on_end))
            .or_else(|| odds(&spawn.on_low))
            .or_else(|| {
                made.chain(trail)
                    .find_map(|s| odds(std::slice::from_ref(s)))
            })
    })
}

/// Damage-weighted hits a use of `spawns` is expected to land, its rolls counted at their odds:
/// E = play(0) + p·(play(1) − play(0)), while one node's rolls add up independently.
pub(crate) fn expected(skill: &str, spawns: &[Spawn]) -> f32 {
    let at = |chance: f32| {
        let spawns: Vec<Spawn> = spawns.iter().map(|s| rolled(s, chance)).collect();
        weighted(&play(skill, &spawns, 1))
    };
    match odds(spawns) {
        Some(chance) => at(0.) + chance * (at(1.) - at(0.)),
        None => weighted(&play(skill, spawns, 1)),
    }
}

/// Every set of three or more of `procs`, all at rank 1 and all at their maximum rank.
pub(crate) fn proc_sets(
    class: &str,
    skill: &str,
    procs: &[&'static str],
) -> Vec<Vec<(&'static str, u32)>> {
    let max = |id: &str| {
        let node = nodes(class, skill).iter().find(|node| node.id == id);
        node.map_or(0, |node| node.max_rank)
    };
    let sets = (0u32..1 << procs.len()).filter(|set| set.count_ones() >= 3);
    sets.flat_map(|set| {
        let chosen = (procs.iter().enumerate()).filter(move |(i, _)| set & (1 << i) != 0);
        let chosen: Vec<&'static str> = chosen.map(|(_, proc)| *proc).collect();
        [1, u32::MAX].map(|rank| chosen.iter().map(|&p| (p, rank.min(max(p)))).collect())
    })
    .collect()
}

/// Each of `sets` whose hits differ between `sim` and `engine` by more than 0.01 plus a
/// thousandth of the sim's, the rounding of f32 sums over hundreds of objects.
pub(crate) fn set_misses(
    sets: &[Vec<(&str, u32)>],
    sim: impl Fn(&[(&str, u32)]) -> f32,
    engine: impl Fn(&[(&str, u32)]) -> f32,
) -> Vec<String> {
    let off = |ranks: &[(&str, u32)]| {
        let (sim, engine) = (sim(ranks), engine(ranks));
        let wrong = (sim - engine).abs() > 0.01 + sim.abs() / 1000.;
        wrong.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
    };
    sets.iter().filter_map(|ranks| off(ranks)).collect()
}

/// The nodes of `class`'s `skill` whose damage the game keeps to their own objects or never reads.
pub(crate) fn owned(class: &str, skill: &str) -> Vec<&'static str> {
    let nodes = nodes(class, skill).iter();
    nodes
        .filter(|node| node.owns_damage())
        .map(|node| node.id.as_str())
        .collect()
}

/// The subskill nodes of `class`'s `skill`.
pub(crate) fn nodes(class: &str, skill: &str) -> &'static [SubskillNodeSpec] {
    data::get_skills_by_class(class)
        .iter()
        .find(|s| s.id == skill)
        .and_then(|s| s.subskills.as_deref())
        .expect("a skill with subskills")
}

/// The engine's dummy hits per use of `skill`, at level 100 and rank 20 with only `ranks`
/// learned (`BuildPerformance::hits_per_cast`, one when it has none).
pub(crate) fn engine_hits(class: &str, skill: &str, ranks: &[(&str, u32)]) -> f32 {
    engine_volley_hits(class, skill, ranks, None)
}

/// The same with `projectiles` in the volley in place of the skill's own count.
pub(crate) fn engine_volley_hits(
    class: &str,
    skill: &str,
    ranks: &[(&str, u32)],
    projectiles: Option<u32>,
) -> f32 {
    let perf = performance(class, skill, ranks, projectiles);
    perf.hits_per_cast.map_or(1., |hits| hits.1) as f32
}

/// The engine's performance of `skill` at level 100 and rank 20 with only `ranks` learned, and
/// `projectiles` in the volley when given.
pub(crate) fn performance(
    class: &str,
    skill: &str,
    ranks: &[(&str, u32)],
    projectiles: Option<u32>,
) -> BuildPerformance {
    let skill_ranks = HashMap::from([(skill.to_owned(), SKILL_RANK)]);
    let subskill_ranks: HashMap<String, u32> = ranks
        .iter()
        .map(|(node, rank)| (subskill_key(skill, node), *rank))
        .collect();
    let volley: HashMap<String, u32> = projectiles
        .map(|n| (skill.to_owned(), n))
        .into_iter()
        .collect();
    compute_build_performance(&BuildPerformanceDeps {
        class_id: Some(class),
        level: LEVEL,
        allocated_attrs: &HashMap::new(),
        inventory: &HashMap::new(),
        skill_ranks: &skill_ranks,
        subskill_ranks: &subskill_ranks,
        active_aura_id: None,
        active_buffs: &HashMap::new(),
        custom_stats: &[],
        allocated_tree_nodes: &HashSet::new(),
        tree_socketed: &HashMap::new(),
        main_skill_id: Some(skill),
        enemy_conditions: &HashMap::new(),
        player_conditions: &HashMap::new(),
        skill_projectiles: &volley,
        enemy_resistances: &HashMap::new(),
        proc_toggles: &HashMap::new(),
        kills_per_sec: 0.,
        entity_rates: &HashMap::new(),
        stack_counts: &HashMap::new(),
        granted_skill_ranks: None,
        difficulty: None,
    })
}

/// The factor the engine folds into its expected hit, not its hits, for the cast rolls a module
/// draws as shares: Multicast, double damage and subtree damage on proc (skills/damage.rs).
pub(crate) fn engine_rolls(class: &str, skill: &str, ranks: &[(&str, u32)]) -> f32 {
    let rank = |id: &str| {
        ranks
            .iter()
            .find(|(node, _)| *node == id)
            .map_or(0, |r| r.1)
    };
    let stat = |key: &str| -> f64 {
        let all = nodes(class, skill).iter();
        all.map(|node| node.planner_stat(key, rank(&node.id))).sum()
    };
    let subtree = 100. + stat("subtree_damage");
    let multicast = 1. + stat("multicast_chance").max(0.) / 100.;
    let double = 1. + stat("double_damage_chance").clamp(0., 100.) / 100.;
    let on_proc = (subtree + stat("subtree_damage_on_proc")) / subtree;
    (multicast * double * on_proc) as f32
}

/// The engine's damage-weighted hits per use: its hits times the rolls it folds into its hit.
pub(crate) fn engine_weighted(class: &str, skill: &str, ranks: &[(&str, u32)]) -> f32 {
    engine_hits(class, skill, ranks) * engine_rolls(class, skill, ranks)
}

/// The factor the engine folds into its hit for `node`'s cast roll when the roll's bonus is
/// planner `<element>_skill_damage` that the game gives the whole cast: its chance-weighted
/// share of the engine's skill-damage pool.
pub(crate) fn engine_skill_roll(
    class: &str,
    skill: &str,
    ranks: &[(&str, u32)],
    node: &str,
) -> f32 {
    let Some(&(_, rank)) = ranks.iter().find(|(id, _)| *id == node) else {
        return 1.;
    };
    let spec = data::get_skills_by_class(class)
        .iter()
        .find(|s| s.id == skill);
    let element = spec
        .and_then(|s| s.damage_type.as_deref())
        .unwrap_or_default();
    let found = nodes(class, skill).iter().find(|n| n.id == node);
    let added = found.map_or(0., |n| {
        n.planner_stat(&format!("{element}_skill_damage"), rank)
    });
    let perf = performance(class, skill, ranks, None);
    let pool = perf.damage.map_or(0., |d| d.skill_damage_max_pct);
    ((100. + pool) / (100. + pool - added)) as f32
}

/// Each node of `class`'s `skill` at each rank, learned alone, whose hits over the base use
/// differ by `within` or more between `sim` and `engine`.
pub(crate) fn misses(
    class: &str,
    skill: &str,
    sim: impl Fn(&[(&str, u32)]) -> f32,
    engine: impl Fn(&[(&str, u32)]) -> f32,
    within: f32,
) -> Vec<String> {
    let (sim_base, engine_base) = (sim(&[]), engine(&[]));
    let mut wrong = Vec::new();
    for node in nodes(class, skill) {
        for rank in 1..=node.max_rank {
            let ranks = [(node.id.as_str(), rank)];
            let (sim, engine) = (sim(&ranks) - sim_base, engine(&ranks) - engine_base);
            if (sim - engine).abs() >= within {
                wrong.push(format!(
                    "{} r{rank}: sim {sim:.4}, engine {engine:.4}",
                    node.id
                ));
            }
        }
    }
    wrong
}
