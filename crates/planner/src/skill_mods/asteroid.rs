//! Exo's Asteroid: TalentsExo case 356 sets the cast from the subskill ranks, Exo_Asteroid_obj
//! falls and explodes (Step_0 drops the wave, Destroy_0 the tsunamis) or the Galactic Cataclysm
//! controller rains asteroids instead, and ProjectileCollision20Exo rolls Perseids on each hit.
use super::Tree;
use crate::skill_effects::{Frostbite, Proc, Scatter};
use crate::skill_sim::{Fall, Placement, Spawn};

const AFTERSHOCK: &str = "asteroid_aftershock";
const SHARP: &str = "sharp_asteroids";
const COLDSHATTER: &str = "coldshatter_impact";
const PERSEIDS: &str = "perseids";
const WAVE: &str = "wave_of_asteroids";
const GEOMAGNETIC: &str = "geomagnetic_reversal";
const CATACLYSM: &str = "galactic_cataclysm";
const PLANETARY: &str = "planetary_collapse";
/// Case 356 L0x353: each further asteroid of a cast wakes 0.15 s after the one before.
const VOLLEY_DELAY: u32 = 9;
/// Case 356 L0x326: Planetary Collapse adds 2 to the scale before the AoE multiplies it.
const COLOSSAL: f32 = 2.;
/// Exo_Asteroid_obj Destroy_0: the explosion is 1.85 times the asteroid.
const BLAST: f32 = 1.85;
/// Exo_Asteroid_obj Step_0: under 32 px of height a wave asteroid drops the next one
/// floor(110 x scale) px further along the throw.
const WAVE_HEIGHT: f32 = 32.;
const WAVE_STEP: f32 = 110.;
/// Destroy_0: tsunamis start 16 px out, 20° apart; Exo_Tsunami_obj Create_0 caps their speed at
/// 4 px a frame and they last their 28-frame animation at 18 fps.
const TSUNAMI: &str = "Exo_Tsunami_obj";
const TSUNAMI_OFFSET: f32 = 16.;
const TSUNAMI_SPREAD: f32 = 20.;
const TSUNAMI_SPEED: f32 = 4.;
const TSUNAMI_LIFE: u32 = 93;
/// Case 356 L0x321-L0x322, Exo_Asteroid_Galactic_Cataclysm_obj Step_0: for 3 s an asteroid every
/// 0.35 s (at least 0.1 s), 32 + random(180) px from the player, from the frame after the cast.
const RAIN_TIME: f32 = 180.;
const RAIN_EVERY: f32 = 21.;
const RAIN_FLOOR: f32 = 6.;
const RAIN_RING: Scatter = Scatter {
    from: 32.,
    spread: 180.,
};
/// ProjectileCollision20Exo: Perseids drops a 0.2 asteroid on the enemy hit; it keeps Create_0's
/// 0.25 gravity and wakes on its 0.1 s alarm.
const PERSEID_SCALE: f32 = 0.2;
const PERSEID_GRAVITY: f32 = 0.25;
const PERSEID_DELAY: u32 = 5;
/// A frostbite lasts a second after the hit that leaves it, the engine's ailment uptime.
const FROSTBITE_FRAMES: u32 = 60;

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    spawns.iter().flat_map(|spawn| cast(spawn, tree)).collect()
}

/// One cast: the asteroid or the cataclysm's rain, and Aftershock's chance of a second one.
fn cast(base: &Spawn, tree: &Tree) -> Vec<Spawn> {
    let raining = tree.learned(CATACLYSM);
    let first = if raining {
        rain(base, tree)
    } else {
        asteroid(base, tree)
    };
    let delay = if raining { 0 } else { VOLLEY_DELAY };
    let second = tree
        .proc(AFTERSHOCK, "of_total_damage")
        .map(|(chance, _)| Spawn {
            chance: Some(chance),
            frame: first.frame + delay,
            ..first.clone()
        });
    std::iter::once(first).chain(second).collect()
}

/// The cast asteroid: the case's scale and fall, its wave and its tsunamis.
fn asteroid(base: &Spawn, tree: &Tree) -> Spawn {
    let scale = cast_scale(base, tree);
    let waves = tree.own(WAVE, "projectile_count").round() as u32;
    let asteroid = falling(base, scale, cast_gravity(base, tree), waves, tree);
    Spawn {
        on_end: [asteroid.on_end.as_slice(), &tsunamis(base, scale, tree)].concat(),
        frostbite: frostbite(tree),
        ..asteroid
    }
}

/// An asteroid and the wave it still drops: Alarm_0 doubles the gravity of one that drops another.
fn falling(base: &Spawn, scale: f32, gravity: f32, waves: u32, tree: &Tree) -> Spawn {
    let gravity = if waves > 0 { gravity * 2. } else { gravity };
    let next = (waves > 0).then(|| Spawn {
        turn: Some(0.),
        offset: (WAVE_STEP * scale).floor(),
        ..falling(base, scale, gravity, waves - 1, tree)
    });
    Spawn {
        fall: base.fall.map(|fall| Fall { gravity, ..fall }),
        scale: Some([scale; 2]),
        on_end: blasts(base, scale, tree),
        low: WAVE_HEIGHT,
        on_low: next.into_iter().collect(),
        ..base.clone()
    }
}

/// The explosion Destroy_0 leaves, 1.85 times the asteroid.
fn blasts(base: &Spawn, scale: f32, tree: &Tree) -> Vec<Spawn> {
    let blast = |end: &Spawn| Spawn {
        scale: Some([BLAST * scale; 2]),
        ..end.clone()
    };
    base.on_end
        .iter()
        .map(|end| hits(blast(end), base, tree))
        .collect()
}

/// Destroy_0's tsunamis from where the asteroid lands: Geomagnetic Reversal's fan along the throw
/// and Planetary Collapse's one of the asteroid's own size.
fn tsunamis(base: &Spawn, scale: f32, tree: &Tree) -> Vec<Spawn> {
    let count = tree.own(GEOMAGNETIC, "projectile_count").round() as u32;
    let middle = (count as f32 - 1.) / 2.;
    let fan = (0..count).map(|i| tsunami(TSUNAMI_SPREAD * (i as f32 - middle), 1.));
    let colossal = tree.learned(PLANETARY).then(|| tsunami(0., scale));
    fan.chain(colossal)
        .map(|wave| hits(wave, base, tree))
        .collect()
}

fn tsunami(turn: f32, scale: f32) -> Spawn {
    Spawn {
        object: TSUNAMI.into(),
        turn: Some(turn),
        offset: TSUNAMI_OFFSET,
        speed: TSUNAMI_SPEED,
        scale: Some([scale; 2]),
        life: Some(TSUNAMI_LIFE),
        ..Spawn::default()
    }
}

/// What the case's projEffect hands every damage object of the cast: Sharp Asteroids' frostbite,
/// Coldshatter Impact's extra damage to frostbitten enemies and Perseids.
fn hits(spawn: Spawn, base: &Spawn, tree: &Tree) -> Spawn {
    let perseids = tree
        .proc(PERSEIDS, "of_total_damage")
        .map(|(chance, share)| Proc {
            chance,
            spawn: perseid(base, share / 100., tree),
            pull: None,
            once: false,
        });
    Spawn {
        frostbite: frostbite(tree),
        vs_frostbitten: coldshatter(tree),
        procs: perseids.into_iter().collect(),
        ..spawn
    }
}

/// Perseids' asteroid on the enemy hit: its blast deals the node's share of the hit, and the
/// frostbite it inherits keeps the cast's damage.
fn perseid(base: &Spawn, share: f32, tree: &Tree) -> Spawn {
    let scale = PERSEID_SCALE + tree.mods.size_bonus;
    let blast = |end: &Spawn| Spawn {
        share: Some(share),
        scale: Some([BLAST * scale; 2]),
        frostbite: frostbite(tree).map(|frost| Frostbite {
            share: frost.share / share,
            ..frost
        }),
        vs_frostbitten: coldshatter(tree),
        ..end.clone()
    };
    Spawn {
        at_enemy: true,
        frame: PERSEID_DELAY,
        fall: base.fall.map(|fall| Fall {
            gravity: PERSEID_GRAVITY,
            ..fall
        }),
        scale: Some([scale; 2]),
        on_end: base.on_end.iter().map(blast).collect(),
        ..base.clone()
    }
}

/// Galactic Cataclysm: the cast leaves a controller that drops plain asteroids around the player.
fn rain(base: &Spawn, tree: &Tree) -> Spawn {
    let layout = rain_frames(tree)
        .into_iter()
        .map(|frame| Placement {
            frame,
            offset: [0., 0.],
        })
        .collect();
    let plain = falling(
        base,
        cast_scale(base, tree),
        cast_gravity(base, tree),
        0,
        tree,
    );
    Spawn {
        target: false,
        layout,
        scatter: Some(RAIN_RING),
        frostbite: frostbite(tree),
        ..plain
    }
}

/// The frames the controller drops on: after each drop skillProjTimer waits skillFrequency
/// frames, while skillTimer runs down.
fn rain_frames(tree: &Tree) -> Vec<u32> {
    let mut time = RAIN_TIME * (1. + tree.own(CATACLYSM, "skill_duration") / 100.);
    let every = RAIN_EVERY * (1. - tree.own(CATACLYSM, "faster_cast_rate") / 100.);
    let (mut wait, mut frame, mut frames) = (0., 1, Vec::new());
    while time > 0. {
        if wait <= 0. {
            frames.push(frame);
            wait = every.max(RAIN_FLOOR);
        } else {
            wait -= 1.;
        }
        time -= 1.;
        frame += 1;
    }
    frames
}

/// image_xscale: 0.66, plus Planetary Collapse's 2, times the subtree's AoE, then the size bonus.
fn cast_scale(base: &Spawn, tree: &Tree) -> f32 {
    let colossal = if tree.learned(PLANETARY) {
        COLOSSAL
    } else {
        0.
    };
    let scale = base.scale.map_or(1., |scale| scale[0]);
    (scale + colossal) * (1. + tree.stat("area_of_effect") / 100.) + tree.mods.size_bonus
}

/// gravityPower: 0.25, sped up by Impact Speed (the planner's projectile speed).
fn cast_gravity(base: &Spawn, tree: &Tree) -> f32 {
    let speed = 1. + tree.stat("projectile_speed_pct") / 100.;
    base.fall.map_or(0., |fall| fall.gravity) * speed
}

/// Coldshatter Impact's extra share of a hit on a frostbitten enemy.
fn coldshatter(tree: &Tree) -> f32 {
    tree.own(COLDSHATTER, "extra_damage_frost_bitten") / 100.
}

/// Sharp Asteroids' frostbite, rolled once a cast.
fn frostbite(tree: &Tree) -> Option<Frostbite> {
    let (chance, _) = tree.proc(SHARP, "increased_frost_bite_damage")?;
    Some(Frostbite {
        chance,
        share: tree.applies(SHARP, "frostbite") / 100.,
        lasts: FROSTBITE_FRAMES,
    })
}

#[cfg(test)]
#[path = "asteroid_parity_tests.rs"]
mod parity;

#[cfg(test)]
mod tests {
    use crate::skill_mods::Mods;
    use crate::skill_sim::{Shape, Spawn, Timeline, run};
    use std::collections::HashMap;

    const ASTEROID: &str = "Exo_Asteroid_obj";
    const BLAST: &str = "Explosion_Item_obj";
    const TSUNAMI: &str = "Exo_Tsunami_obj";
    /// Where the asteroid falls: the cursor point, 30 px above the dummy.
    const AT: [f32; 2] = [160., -30.];

    fn mods(ranks: &[(&str, u32)]) -> Mods {
        let ranks: HashMap<String, u32> = ranks
            .iter()
            .map(|(node, rank)| (format!("asteroid:{node}"), *rank))
            .collect();
        Mods::of("asteroid", &ranks)
    }

    fn cast_with(mods: &Mods) -> Timeline {
        run("asteroid", mods, 1).expect("asteroid")
    }

    fn cast(ranks: &[(&str, u32)]) -> Timeline {
        cast_with(&mods(ranks))
    }

    fn spawns_with(ranks: &[(&str, u32)]) -> Vec<Spawn> {
        let base = &crate::skill_sim::skill("asteroid").expect("asteroid").spawn;
        crate::skill_mods::apply("asteroid", base, &mods(ranks)).into_owned()
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

    fn scale_of(timeline: &Timeline, object: &str) -> f32 {
        births(timeline, object)[0].1.scale[0]
    }

    /// Degrees from the player through the cursor point: the throw.
    fn throw() -> f32 {
        (-AT[1]).atan2(AT[0]).to_degrees()
    }

    /// `distance` px from `from` along `degrees`, GameMaker style.
    fn along(from: [f32; 2], degrees: f32, distance: f32) -> [f32; 2] {
        let (sin, cos) = degrees.to_radians().sin_cos();
        [from[0] + cos * distance, from[1] - sin * distance]
    }

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-2
    }

    #[test]
    fn the_base_asteroid_lands_after_51_frames_and_its_explosion_hits_once() {
        let base = cast(&[]);
        assert_eq!(base.hits, [56]);
        let (frame, blast) = births(&base, BLAST)[0];
        assert_eq!((frame, blast.pos), (52, AT));
        assert!((blast.scale[0] - 1.221).abs() < 1e-4, "{}", blast.scale[0]);
    }

    #[test]
    fn damage_and_cost_nodes_leave_the_use_unchanged() {
        let base = cast(&[]);
        let same = cast(&[
            ("astral_crush", 5),
            ("a_gift_from_space", 5),
            ("clairvoyant_prophet", 5),
            ("frozen_core", 5),
        ]);
        assert_eq!(same.hits, base.hits);
        assert_eq!(same.frames.len(), base.frames.len());
        assert!(same.effects.damage.iter().all(|d| d.share == 1.));
    }

    #[test]
    fn aftershock_may_drop_a_second_asteroid_0_15_s_later() {
        let spawns = spawns_with(&[("asteroid_aftershock", 5)]);
        assert_eq!(spawns.len(), 2);
        assert_eq!((spawns[0].chance, spawns[0].frame), (None, 0));
        assert_eq!((spawns[1].chance, spawns[1].frame), (Some(0.25), 9));
        let uses = crate::skill_sim::uses("asteroid", &mods(&[("asteroid_aftershock", 5)]))
            .expect("asteroid uses");
        assert!(uses.iter().all(|u| u.hits == [56] || u.hits == [56, 65]));
        assert!(uses.iter().any(|u| u.hits == [56, 65]));
    }

    #[test]
    fn impact_speed_drops_the_asteroid_faster() {
        assert_eq!(cast(&[("impact_speed", 1)]).hits, [52]);
        assert_eq!(cast(&[("impact_speed", 5)]).hits, [42]);
    }

    #[test]
    fn world_ending_impact_widens_the_asteroid_and_its_explosion() {
        let wide = cast(&[("world_ending_impact", 5)]);
        assert!((scale_of(&wide, ASTEROID) - 1.485).abs() < 1e-4);
        assert!((scale_of(&wide, BLAST) - 1.85 * 1.485).abs() < 1e-4);
        assert_eq!(wide.hits, [56]);
    }

    #[test]
    fn size_stats_add_their_bonus_to_the_asteroid_before_the_explosion_grows() {
        let mods = Mods {
            size_bonus: 0.3,
            ..mods(&[])
        };
        let bigger = cast_with(&mods);
        assert!((scale_of(&bigger, ASTEROID) - 0.96).abs() < 1e-4);
        assert!((scale_of(&bigger, BLAST) - 1.776).abs() < 1e-4);
    }

    #[test]
    fn sharp_asteroids_rolls_frostbite_per_cast_and_coldshatter_raises_frostbitten_hits() {
        let spawns = spawns_with(&[
            ("sharp_asteroids", 5),
            ("coldshatter_impact", 5),
            ("geomagnetic_reversal", 1),
        ]);
        let asteroid = &spawns[0];
        let frost = asteroid.frostbite.expect("a frostbite roll");
        assert!((frost.chance - 0.15).abs() < 1e-5 && (frost.share - 1.75).abs() < 1e-5);
        assert_eq!(asteroid.on_end.len(), 4);
        for hit in &asteroid.on_end {
            assert!(hit.frostbite.is_some(), "{}", hit.object);
            assert!((hit.vs_frostbitten - 1.).abs() < 1e-5);
        }
    }

    #[test]
    fn perseids_drops_a_small_asteroid_on_the_enemy_hit() {
        let spawns = spawns_with(&[("perseids", 5)]);
        let blast = &spawns[0].on_end[0];
        let proc = &blast.procs[0];
        assert!((proc.chance - 0.15).abs() < 1e-5);
        let perseid = &proc.spawn;
        assert!(perseid.at_enemy && perseid.ability);
        assert_eq!((perseid.frame, perseid.scale), (5, Some([0.2, 0.2])));
        assert_eq!(perseid.fall.map(|fall| fall.gravity), Some(0.25));
        let small = &perseid.on_end[0];
        assert_eq!(small.share, Some(2.5));
        assert!((small.scale.expect("a scale")[0] - 0.37).abs() < 1e-5);
        let rain = cast(&[("perseids", 5), ("geomagnetic_reversal", 3)]);
        let (_, dropped) = births(&rain, ASTEROID)
            .into_iter()
            .find(|(_, shape)| (shape.scale[0] - 0.2).abs() < 1e-5)
            .expect("a perseid");
        assert_eq!(dropped.pos, rain.dummy);
        assert!(rain.effects.damage.iter().any(|d| d.share == 2.5));
    }

    #[test]
    fn wave_of_asteroids_marches_a_chain_out_along_the_throw() {
        // Four more asteroids, each dropped 72 px further once the one before is under 32 px,
        // every one that drops another falling twice as fast.
        let wave = cast(&[("wave_of_asteroids", 1)]);
        let drops = births(&wave, ASTEROID);
        let frames: Vec<usize> = drops.iter().map(|(frame, _)| *frame).collect();
        assert_eq!(frames, [0, 35, 60, 78, 91]);
        for (k, (_, drop)) in drops.iter().enumerate() {
            let expected = along(AT, throw(), 72. * k as f32);
            assert!(near(drop.pos, expected), "{k}: {:?} {expected:?}", drop.pos);
        }
        assert_eq!(wave.hits, [41]);
        assert_eq!(births(&wave, BLAST).len(), 5);
    }

    #[test]
    fn geomagnetic_reversal_sends_3_tsunamis_a_rank_20_degrees_apart() {
        let tides = cast(&[("geomagnetic_reversal", 1)]);
        let waves = births(&tides, TSUNAMI);
        let mut turns: Vec<f32> = waves.iter().map(|(_, s)| s.angle - throw()).collect();
        turns.sort_by(f32::total_cmp);
        for (turn, expected) in turns.iter().zip([-20., 0., 20.]) {
            assert!((turn - expected).abs() < 1e-3, "{turns:?}");
        }
        for (_, wave) in &waves {
            assert!(near(wave.pos, along(AT, wave.angle, 16.)), "{:?}", wave.pos);
            assert_eq!(wave.scale, [1., 1.]);
        }
        assert_eq!(tides.hits, [54, 54, 54, 56]);
        assert_eq!(cast(&[("geomagnetic_reversal", 3)]).hits.len(), 10);
    }

    #[test]
    fn galactic_cataclysm_rains_asteroids_around_the_player_instead() {
        let rain = cast(&[("galactic_cataclysm", 1)]);
        let drops = births(&rain, ASTEROID);
        let frames: Vec<usize> = drops.iter().map(|(frame, _)| *frame).collect();
        assert_eq!(frames, (0..15).map(|k| 1 + 15 * k).collect::<Vec<_>>());
        for (_, drop) in &drops {
            let distance = drop.pos[0].hypot(drop.pos[1]);
            assert!((32. ..=212.).contains(&distance), "{distance}");
        }
        assert_eq!(
            births(&cast(&[("galactic_cataclysm", 3)]), ASTEROID).len(),
            45
        );
        let plain = cast(&[
            ("galactic_cataclysm", 1),
            ("geomagnetic_reversal", 3),
            ("wave_of_asteroids", 3),
        ]);
        assert!(births(&plain, TSUNAMI).is_empty());
        assert_eq!(births(&plain, ASTEROID).len(), 15);
    }

    #[test]
    fn aftershock_with_the_cataclysm_may_add_a_second_rain() {
        let spawns = spawns_with(&[("galactic_cataclysm", 1), ("asteroid_aftershock", 2)]);
        assert_eq!(spawns.len(), 2);
        assert_eq!(spawns[1].chance, Some(0.1));
        assert_eq!(spawns[1].layout.len(), spawns[0].layout.len());
    }

    #[test]
    fn planetary_collapse_drops_a_colossal_asteroid_with_a_tsunami_its_size() {
        let colossal = cast(&[("planetary_collapse", 1)]);
        let scale = (0.66 + 2.) * 1.3;
        assert!((scale_of(&colossal, ASTEROID) - scale).abs() < 1e-4);
        assert!((scale_of(&colossal, BLAST) - 1.85 * scale).abs() < 1e-3);
        let waves = births(&colossal, TSUNAMI);
        assert_eq!(waves.len(), 1);
        assert!((waves[0].1.scale[0] - scale).abs() < 1e-4);
        assert!((waves[0].1.angle - throw()).abs() < 1e-3);
        assert_eq!(colossal.hits, [54, 56]);
    }

    #[test]
    fn every_node_at_once_still_plays_a_bounded_use() {
        let tree = [
            ("astral_crush", 5),
            ("asteroid_aftershock", 5),
            ("impact_speed", 5),
            ("world_ending_impact", 5),
            ("a_gift_from_space", 5),
            ("clairvoyant_prophet", 5),
            ("sharp_asteroids", 5),
            ("frozen_core", 5),
            ("coldshatter_impact", 5),
            ("perseids", 5),
            ("wave_of_asteroids", 3),
            ("geomagnetic_reversal", 3),
            ("planetary_collapse", 3),
        ];
        let storm = cast(&tree);
        assert!(storm.hits.len() >= 11, "{:?}", storm.hits);
        assert_eq!(births(&storm, TSUNAMI).len(), 10);
        let rain = cast(&[tree.as_slice(), &[("galactic_cataclysm", 3)]].concat());
        assert!(births(&rain, TSUNAMI).is_empty());
        assert!(rain.frames.len() < 600, "{}", rain.frames.len());
    }

    #[test]
    fn the_builds_own_size_stats_reach_the_asteroid() {
        use hsplanner_build::BuildSnapshot;
        use hsplanner_engine::calc::stats::ComputedStats;
        let snapshot = BuildSnapshot {
            class_id: Some("exo".into()),
            active_skill_ids: vec!["asteroid".into()],
            subskill_ranks: HashMap::from([("asteroid:world_ending_impact".into(), 2)]),
            ..Default::default()
        };
        // The asteroid has no Area of Effect tag: its Explosion tag takes Explosion AoE.
        let computed = ComputedStats {
            stats: HashMap::from([("explosion_aoe".into(), (30., 30.))]),
            ..Default::default()
        };
        let mods = Mods::for_build("asteroid", &snapshot, Some(&computed));
        assert_eq!(mods.rank("world_ending_impact"), 2);
        assert!((mods.size_bonus - 0.3).abs() < 1e-5, "{}", mods.size_bonus);
        let grown = cast_with(&mods);
        let expected = 0.66 * 1.5 + mods.size_bonus;
        assert!((scale_of(&grown, ASTEROID) - expected).abs() < 1e-4);
    }
}
