//! The game's dummy hits for the engine's parity tests: every random draw of a use counted at its
//! odds and every object replayed apart, so the drawn preview's 256-object cap cuts nothing.
use super::{Cast, Mount};
use crate::skill_effects::Dice;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Placement, Shape, Spawn, Timeline};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

const CLASS: &str = "pyromancer";
const VOLCANO: &str = "Pyromancer_Volcano_obj";
const BLAST: &str = "Explosion_Item_obj";
const BURST: &str = "Pyromancer_Scorching_Searing_Burst_obj";

/// irandom(360)'s draws, each as likely.
const HEADINGS: u32 = super::FULL_TURN + 1;
const BOUNCE: &str = "Explosion_Item_obj:Viking_Odins_Fury_spr";
/// A lava controller's patches: reach 23 + 13 along its 600 px walk.
const PATCH_REACH: f32 = 40.;
const LAVA_WAY: f32 = 605.;
/// Walks of Trail of Volcanoes' chain averaged.
const WALKS: u64 = 1024;

/// `f` on the module's cast for `ranks`.
fn with_cast<T>(ranks: &[(&str, u32)], f: impl FnOnce(&Cast) -> T) -> T {
    let mods = parity::mods(super::SKILL, ranks);
    let tree = crate::skill_mods::Tree {
        nodes: parity::nodes(CLASS, super::SKILL),
        mods: &mods,
        cast: 0,
    };
    let base = &crate::skill_sim::skill(super::SKILL)
        .expect("volcano")
        .spawn[0];
    f(&Cast::new(base, &tree))
}

/// `items` made where an unseen carrier stands at `at` on the use's first frame.
fn made_at(at: [f32; 2], items: Vec<Spawn>) -> Spawn {
    Spawn {
        target: true,
        layout: vec![Placement {
            frame: 0,
            offset: [at[0] - cursor()[0], at[1] - cursor()[1]],
        }],
        ..super::carrier(items)
    }
}

fn weighed(spawn: Spawn) -> (f32, Timeline) {
    let timeline = play(&[spawn]);
    (parity::weighted(&timeline), timeline)
}

/// Whether the dummy stands within `reach` of the way `length` px long from `from` at `heading`.
fn near_way(from: [f32; 2], heading: f32, length: f32, reach: f32) -> bool {
    let (sin, cos) = heading.to_radians().sin_cos();
    let along = ((dummy()[0] - from[0]) * cos - (dummy()[1] - from[1]) * sin).clamp(0., length);
    self::from([from[0] + cos * along, from[1] - sin * along], dummy()) <= reach
}

fn reach_of(object: &str) -> f32 {
    let objects = &crate::hitbox::skill(super::SKILL)
        .expect("hitboxes")
        .objects;
    objects
        .iter()
        .find(|o| o.name == object)
        .expect(object)
        .reach
}

/// Expectations already worked out, by what they depend on.
fn cached(key: String, work: impl FnOnce() -> f32) -> f32 {
    static KNOWN: LazyLock<Mutex<HashMap<String, f32>>> = LazyLock::new(Default::default);
    if let Some(known) = KNOWN.lock().expect("cache").get(&key) {
        return *known;
    }
    let value = work();
    KNOWN.lock().expect("cache").insert(key, value);
    value
}

/// A volcano's own ticks where it stands, its Step's objects left out.
fn ticks(cast: &Cast, mount: &Mount) -> f32 {
    weighed(Spawn {
        target: true,
        layout: made_at(mount.at, Vec::new()).layout,
        ..cast.body(mount)
    })
    .0
}

/// Expected hits of one fragment `mount` throws, over its irandom(360) heading and its shot's
/// crushing roll, Yellowstone's small volcano and its own fragments counted.
fn fragment_hits(cast: &Cast, mount: &Mount) -> f32 {
    let key = format!(
        "fragment {:?} {} {} {} {} {} {}",
        mount.at,
        mount.harms,
        mount.yellowstone,
        cast.speed,
        cast.crushing,
        cast.life,
        cast.bonus()
    );
    cached(key, || {
        let crushing = cast.crushing;
        let lucky = if crushing > 0. {
            thrown(cast, mount, super::BOUNCES)
        } else {
            0.
        };
        (1. - crushing) * thrown(cast, mount, 0) + crushing * lucky
    })
}

fn thrown(cast: &Cast, mount: &Mount, bounces: u32) -> f32 {
    let stops = stops(cast, bounces);
    let mut dice = Dice::of_cast(0);
    let mut seen: HashMap<u32, f32> = HashMap::new();
    let mut total = 0.;
    for draw in 0..HEADINGS {
        let heading = cast.aimed(mount.at, draw as f32);
        let reaches = stops.iter().any(|&(out, reach)| {
            let [dx, dy] = super::pyro::lengthdir(out, heading);
            from([mount.at[0] + dx, mount.at[1] + dy], dummy()) <= reach + 1.
        });
        if !mount.yellowstone && !reaches {
            continue;
        }
        total += *seen.entry(heading.to_bits()).or_insert_with(|| {
            let fragment = cast.fragment(mount, heading, false, bounces, &mut dice);
            let (hits, timeline) = weighed(made_at(mount.at, vec![unraised(fragment)]));
            let raised = births(&timeline, BLAST).filter(|_| mount.yellowstone);
            hits + raised.map(|end| raised_hits(cast, end.pos)).sum::<f32>()
        });
    }
    total / HEADINGS as f32
}

/// Where a fragment's blasts go off, px along its heading, and how far each reaches the dummy's
/// centre: one fragment replayed far from the dummy.
fn stops(cast: &Cast, bounces: u32) -> Stops {
    static KNOWN: LazyLock<Mutex<HashMap<(u32, u32), Stops>>> = LazyLock::new(Default::default);
    let key = (cast.speed.to_bits(), bounces);
    if let Some(known) = KNOWN.lock().expect("stops").get(&key) {
        return known.clone();
    }
    let measured = measure_stops(cast, bounces);
    KNOWN.lock().expect("stops").insert(key, measured.clone());
    measured
}

type Stops = Vec<(f32, f32)>;

fn measure_stops(cast: &Cast, bounces: u32) -> Stops {
    let away = [cursor()[0], cursor()[1] - 2000.];
    let mount = cast.raised(away);
    let fragment = cast.fragment(&mount, 0., false, bounces, &mut Dice::of_cast(0));
    let timeline = play(&[made_at(away, vec![fragment])]);
    let mut stops = Stops::new();
    for (object, scale) in [(BLAST, 1.), (BOUNCE, 0.5)] {
        let reach = reach_of(object) * scale + 13.;
        stops.extend(births(&timeline, object).map(|blast| (from(blast.pos, away), reach)));
    }
    stops
}

/// The fragment without the volcano Destroy_0 raises, which `raised_hits` counts apart.
fn unraised(spawn: Spawn) -> Spawn {
    let ends = spawn.on_end.into_iter().filter(|s| s.object != VOLCANO);
    Spawn {
        on_end: ends.map(unraised).collect(),
        ..spawn
    }
}

/// A small volcano raised at `at`: its ticks where it can reach the dummy, and its fragments.
fn raised_hits(cast: &Cast, at: [f32; 2]) -> f32 {
    let mount = cast.raised(at);
    let reach = reach_of(VOLCANO) * mount.max + 13.;
    let ticks = if from(at, dummy()) <= reach {
        ticks(cast, &mount)
    } else {
        0.
    };
    let thrown = cast.shots(&mount).len() as f32 * mount.fragments as f32;
    ticks + thrown * fragment_hits(cast, &mount)
}

/// Expected hits of one burst m_cracklingEarth makes, over its heading and distance.
fn burst_hits(cast: &Cast, mount: &Mount) -> f32 {
    let key = format!("burst {:?} {} {}", mount.at, mount.share, cast.bonus());
    cached(key, || {
        let reach = reach_of(BURST) * 0.4 + 13.;
        let mut total = 0.;
        for draw in 0..HEADINGS {
            for spread in 0..=super::BURST_SPREAD {
                let out = super::BURST_NEAR + spread as f32;
                let [dx, dy] = crate::skill_mods::pyro::lengthdir(out, draw as f32);
                if from([mount.at[0] + dx, mount.at[1] + dy], dummy()) > reach {
                    continue;
                }
                let burst = cast.burst(mount, draw as f32, spread as f32);
                total += weighed(made_at(mount.at, vec![burst])).0;
            }
        }
        total / (HEADINGS * (super::BURST_SPREAD + 1)) as f32
    })
}

/// Expected hits of one lava controller, over its heading.
fn controller_hits(cast: &Cast, mount: &Mount) -> f32 {
    let key = format!("controller {:?} {} {}", mount.at, mount.share, cast.bonus());
    cached(key, || {
        let total: f32 = (0..HEADINGS)
            .filter(|&draw| near_way(mount.at, draw as f32, LAVA_WAY, PATCH_REACH))
            .map(|draw| {
                let controller = cast.controller(mount, draw as f32);
                weighed(made_at(mount.at, vec![controller])).0
            })
            .sum();
        total / HEADINGS as f32
    })
}

/// Expected hits of a volcano: its ticks and all its Step makes, counted apart.
fn volcano_hits(cast: &Cast, mount: &Mount, ticked: f32) -> f32 {
    let shots = cast.shots(mount).len() as f32 * mount.fragments as f32;
    let thrown = if cast.throws && mount.harms | mount.yellowstone {
        shots * fragment_hits(cast, mount)
    } else {
        0.
    };
    let bursts = (cast.crack.filter(|_| mount.cracks)).map_or(0., |every| {
        cast.every(every).len() as f32 * burst_hits(cast, mount)
    });
    let trails = cast.every(super::LAVA_EVERY).len() as f32 * mount.trails as f32;
    let lava = if trails > 0. {
        trails * controller_hits(cast, mount)
    } else {
        0.
    };
    ticked + thrown + bursts + lava
}

/// Trail of Volcanoes' chained volcanoes' ticks over many walks; what their Step makes lands
/// out of the engine's per-node model and is left out.
fn chained_hits(cast: &Cast, from: [f32; 2]) -> f32 {
    if cast.chain == 0 {
        return 0.;
    }
    let reach = reach_of(VOLCANO) * (1. + cast.bonus()) + 13.;
    let mut total = 0.;
    for walk in 0..WALKS {
        for at in cast.links(from, &mut Dice::of_cast(walk)) {
            if self::from(at, dummy()) <= reach {
                total += ticks(cast, &cast.chained(at));
            }
        }
    }
    total / WALKS as f32
}

/// Damage-weighted hits the game lands on the dummy in one use with `ranks`, every random draw at
/// its odds and every object counted apart, so no drawing cap cuts the count.
pub(super) fn uncapped(ranks: &[(&str, u32)]) -> f32 {
    with_cast(ranks, |cast| {
        let main = cast.main();
        volcano_hits(cast, &main, ticks(cast, &main)) + chained_hits(cast, main.at)
    })
}

fn cursor() -> [f32; 2] {
    super::pyro::cursor(super::SKILL)
}

fn dummy() -> [f32; 2] {
    [cursor()[0], 0.]
}

fn play(spawns: &[Spawn]) -> Timeline {
    parity::play(super::SKILL, spawns, 1)
}

fn from(pos: [f32; 2], to: [f32; 2]) -> f32 {
    (pos[0] - to[0]).hypot(pos[1] - to[1])
}

/// The first drawn shape of every instance of `object`.
fn births<'a>(timeline: &'a Timeline, object: &'a str) -> impl Iterator<Item = &'a Shape> {
    let shapes = timeline.frames.iter().flatten();
    shapes.filter(move |shape| shape.object.name == object && shape.age == 1)
}
