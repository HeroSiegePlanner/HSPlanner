//! The game's dummy hits per pulse for the engine's parity tests: the pulse's rolls at their odds,
//! every object its hit sets off counted apart (played alone where a hit on the dummy makes it,
//! then what it makes on each of its hits, a full stack's procs at one hit in 101) and Searing
//! Burst's spots enumerated, so the preview's 256-object cap decides none of them.
use super::{
    AURA_SCALE, BLAZE, BURST_SCALE, BURST_SPREAD, BURSTS, CASTS, FISSURE, FULL_TURN, LANDS,
    PERCENT, SKILL, apply_rolled, every, pyro,
};
use crate::skill_effects::Proc;
use crate::skill_mods::{Tree, parity_tests as parity};
use crate::skill_sim::{Placement, Spawn, Timeline};
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

const CLASS: &str = "pyromancer";
const AURA: &str = "Pyromancer_Scorching_Aura_obj";
const BURST: &str = "Pyromancer_Scorching_Searing_Burst_obj";
/// px inside a round mask's reach where it surely meets the dummy's 12 px disk.
const SURE_MARGIN: f32 = 3.;
const DUMMY_RADIUS: f32 = 12.;

/// Damage-weighted hits the game lands on the in-zone dummy per pulse with `ranks`: the bursts at
/// the odds of their spots, the rest at the odds of the pulse's rolls.
pub(super) fn uncapped(ranks: &[(&str, u32)]) -> f32 {
    with_tree(ranks, |tree, rest| {
        let scale = AURA_SCALE * (1. + tree.own(LANDS, "area_of_effect") / PERCENT);
        let bursts = tree.own(BURSTS, "projectile_count") * burst_odds(scale);
        rolled(tree, |fissure, blaze| {
            let made = pulse(rest, 0, fissure, blaze);
            made.iter()
                .find(|spawn| spawn.object == AURA)
                .map_or(0., aura)
        }) + bursts
    })
}

/// `f` on the tree of `ranks` and the ranks but Searing Burst's, whose spots a forced pulse skips.
fn with_tree<T>(ranks: &[(&str, u32)], f: impl FnOnce(&Tree, &[(&str, u32)]) -> T) -> T {
    let mods = parity::mods(SKILL, ranks);
    let tree = Tree {
        nodes: parity::nodes(CLASS, SKILL),
        mods: &mods,
        cast: 0,
    };
    let rest: Vec<(&str, u32)> = (ranks.iter().copied())
        .filter(|(node, _)| *node != BURSTS)
        .collect();
    f(&tree, &rest)
}

/// `count` over Flaming Fissure's LST(32, 4, 9) % and Ground Blaze's chance, at their odds.
fn rolled(tree: &Tree, count: impl Fn(bool, bool) -> f32) -> f32 {
    let fissure =
        (tree.proc(FISSURE, "fire_skill_damage")).map_or(0., |(_, value)| value / PERCENT);
    let blaze = (tree.proc(BLAZE, "of_total_damage")).map_or(0., |(chance, _)| chance);
    let outcomes = [
        (false, false, (1. - fissure) * (1. - blaze)),
        (true, false, fissure * (1. - blaze)),
        (false, true, (1. - fissure) * blaze),
        (true, true, fissure * blaze),
    ];
    let rolls = outcomes.into_iter().filter(|&(.., odds)| odds > 0.);
    rolls.map(|(f, b, odds)| odds * count(f, b)).sum()
}

/// The `cast`-th pulse with `ranks`, every pulse rolling Flaming Fissure `fissure` and Ground
/// Blaze `blaze`.
fn pulse(ranks: &[(&str, u32)], cast: u64, fissure: bool, blaze: bool) -> Vec<Spawn> {
    parity::rewritten(CLASS, SKILL, ranks, cast, |spawns, tree| {
        apply_rolled(spawns, tree, fissure, blaze)
    })
}

/// The aura's hit and all its procs set off.
fn aura(aura: &Spawn) -> f32 {
    let alone = Spawn {
        procs: Vec::new(),
        ..aura.clone()
    };
    let hits = parity::weighted(&parity::play(SKILL, &[alone], 1));
    hits + set_off(&aura.procs, hits)
}

/// What `procs` make over `hits` hits of their carrier: each on every hit, a stack's at the odds
/// of the hit finding it full.
fn set_off(procs: &[Proc], hits: f32) -> f32 {
    let one = |proc: &Proc| {
        let hits = if proc.once { hits.min(1.) } else { hits };
        let full = (proc.spawn.stacks).map_or(1., |stacks| 1. / (stacks.max + 1) as f32);
        proc.chance * full * hits * apart(&proc.spawn)
    };
    procs.iter().map(one).sum()
}

/// Damage-weighted hits of `spawn` made by a hit on the dummy, with all it makes on its hits and
/// all its ends make on theirs.
fn apart(spawn: &Spawn) -> f32 {
    let (own, ends) = sighted(spawn);
    let made = spawn.on_end.iter().zip(ends);
    let ends = made.map(|(end, hits)| end.share.unwrap_or(1.) * hits + set_off(&end.procs, hits));
    spawn.share.unwrap_or(1.) * own + set_off(&spawn.procs, own) + ends.sum::<f32>()
}

/// Hits an object lands and hits each of its ends lands.
type Seen = (f32, Vec<f32>);

/// Hits `spawn` and each of its ends land when a hit on the dummy makes it, every object alone.
fn sighted(spawn: &Spawn) -> Seen {
    static KNOWN: LazyLock<Mutex<HashMap<String, Seen>>> = LazyLock::new(Default::default);
    let key = geometry(spawn);
    if let Some(known) = KNOWN.lock().expect("cache").get(&key) {
        return known.clone();
    }
    let bare = |share| Spawn {
        share: Some(share),
        procs: Vec::new(),
        on_end: Vec::new(),
        stacks: None,
        ..spawn.clone()
    };
    let own = hosted(bare(1.));
    let ends = spawn.on_end.iter().map(|end| {
        let end = Spawn {
            on_end: vec![Spawn {
                share: Some(1.),
                procs: Vec::new(),
                on_end: Vec::new(),
                ..end.clone()
            }],
            ..bare(0.)
        };
        hosted(end)
    });
    let seen = (own, ends.collect());
    KNOWN.lock().expect("cache").insert(key, seen.clone());
    seen
}

/// Damage-weighted hits of `child` made by the aura's hit on the dummy, the aura dealing none.
fn hosted(child: Spawn) -> f32 {
    let fx = crate::skill_sim::skill(SKILL).expect("scorching aura");
    let host = Spawn {
        share: Some(0.),
        scale: Some([AURA_SCALE; 2]),
        procs: vec![every(child)],
        ..fx.spawn[0].clone()
    };
    parity::weighted(&parity::play(SKILL, &[host], 1))
}

/// What decides where and how often an object hits: everything but its damage and its procs.
fn geometry(spawn: &Spawn) -> String {
    let orbit = spawn.orbit.map(|o| (o.radius, o.speed, o.at, o.spread));
    let grow = spawn.grow.map(|g| (g.step, g.max));
    let ends: Vec<_> = (spawn.on_end.iter())
        .map(|end| (end.object.as_str(), end.scale, end.life))
        .collect();
    format!(
        "{} {:?} {:?} {orbit:?} {grow:?} {} {} {} {} {} {ends:?}",
        spawn.object,
        spawn.scale,
        spawn.life,
        spawn.speed,
        spawn.tick,
        spawn.at_enemy,
        spawn.at_player,
        spawn.aim
    )
}

/// Odds that a burst irandom(360)° and irandom(floor(64 × scale)) px from the player meets the
/// dummy, every pair as likely; its round mask is replayed only near the edge of its reach.
pub(super) fn burst_odds(scale: f32) -> f32 {
    static KNOWN: LazyLock<Mutex<HashMap<u32, f32>>> = LazyLock::new(Default::default);
    let spread = (BURST_SPREAD * scale).floor() as u32;
    if let Some(known) = KNOWN.lock().expect("cache").get(&spread) {
        return *known;
    }
    let dummy = [crate::skill_sim::skill(SKILL).expect("motion").dummy, 0.];
    let reach = reach_of(BURST) * BURST_SCALE;
    let (sure, far) = (
        reach - SURE_MARGIN + DUMMY_RADIUS,
        reach + DUMMY_RADIUS + 1.,
    );
    let landed = |heading: u32, out: u32| {
        let spot = pyro::lengthdir(out as f32, heading as f32);
        let off = (spot[0] - dummy[0]).hypot(spot[1] - dummy[1]);
        if off > far {
            0.
        } else if off <= sure {
            1.
        } else {
            parity::weighted(&parity::play(SKILL, &[at(spot)], 1))
        }
    };
    let spots = (0..=FULL_TURN).flat_map(|heading| (0..=spread).map(move |out| (heading, out)));
    let hits: f32 = spots.map(|(heading, out)| landed(heading, out)).sum();
    let odds = hits / ((FULL_TURN + 1) * (spread + 1)) as f32;
    KNOWN.lock().expect("cache").insert(spread, odds);
    odds
}

/// A burst made at `spot` on the pulse's first frame.
fn at(spot: [f32; 2]) -> Spawn {
    Spawn {
        layout: vec![Placement {
            frame: 0,
            offset: spot,
        }],
        ..pyro::burst(BURST_SCALE, 1.)
    }
}

fn reach_of(object: &str) -> f32 {
    let objects = &crate::hitbox::skill(SKILL).expect("hitboxes").objects;
    (objects.iter())
        .find(|o| o.name == object)
        .expect(object)
        .reach
}

/// The live objects at the busiest frame of `timeline`.
fn busiest(timeline: &Timeline) -> usize {
    timeline.frames.iter().map(Vec::len).max().unwrap_or(0)
}

/// The same pulses played whole over an ignition cycle, each roll forced both ways.
fn whole(ranks: &[(&str, u32)]) -> (f32, usize) {
    with_tree(ranks, |tree, rest| {
        let most = Cell::new(0);
        let hits = rolled(tree, |fissure, blaze| {
            let cast = |n| {
                let timeline = parity::play(SKILL, &pulse(rest, n, fissure, blaze), 1);
                most.set(most.get().max(busiest(&timeline)));
                parity::weighted(&timeline)
            };
            (0..CASTS).map(cast).sum::<f32>() / CASTS as f32
        });
        (hits, most.get())
    })
}

#[test]
fn objects_counted_apart_add_up_to_whole_pulses_below_the_cap() {
    let ranks = [
        ("flaming_fissure", 5),
        ("ignite_the_flames", 5),
        ("ground_blaze", 5),
        ("scorching_harvester", 3),
        ("release_the_chaos", 3),
    ];
    let ((whole, most), apart) = (whole(&ranks), uncapped(&ranks));
    assert!(most < 256, "{most} objects at once");
    assert!((whole - apart).abs() < 0.01, "whole {whole}, apart {apart}");
}
