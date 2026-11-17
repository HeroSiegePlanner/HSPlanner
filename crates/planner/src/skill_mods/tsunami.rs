//! Exo's Tsunami: TalentsExo case 355 sets the waves from the subskill ranks, Exo_Tsunami_obj
//! speeds up, ramps its damage and turns back to the player at its animation's end (Other_7),
//! and ProjectileCollision20Exo raises the waterspouts on each hit.
use super::Tree;
use crate::skill_effects::{Buff, Frostbite, Proc, Ramp};
use crate::skill_sim::Spawn;

const ARCTIC: &str = "arctic_aqua";
const WALKER: &str = "water_walker";
const RAMPING: &str = "ramping_waves";
const WATERSPOUT: &str = "waterspout";
const SALT: &str = "salt_balance";
const GIANT: &str = "giant_waves";
const TREMBLING: &str = "trembling_tidalwave";
const TURBULENCE: &str = "watery_turbulence";
const PULLBACK: &str = "ocean_pullback";
/// Case 355 L0x2b0: Watery Turbulence halves the waves before their size multiplies them.
const TURBULENCE_SIZE: f32 = 0.5;
/// Case 355 L0x2ab, L0x2d3: Trembling Tidalwave adds 2 to image_yscale only.
const TREMBLING_WIDTH: f32 = 2.;
/// Case 355 L0x29d: Water Walker's buff 307 lasts 3 s.
const WALKER_FRAMES: u32 = 180;
/// Exo_Tsunami_obj Create_0, Step_0: rampingTimer runs out every 0.5 s.
const RAMP_EVERY: u32 = 30;
/// ProjectileCollision20Exo L0xc3-L0xc5: a 0.66 Exo_Tsunami_Waterspout_obj at the enemy hit plays
/// its 9 frames at 15 fps; Destroy_0 leaves an Explosion_Item_obj of its size showing the burst's
/// 12 frames at 24 fps at the explosion's 0.75 image speed.
const SPOUT: &str = "Exo_Tsunami_Waterspout_obj";
const BURST: &str = "Explosion_Item_obj";
const SPOUT_SCALE: f32 = 0.66;
const SPOUT_LIFE: u32 = 36;
const BURST_LIFE: u32 = 40;
const BURST_ACTIVE: u32 = 4;
/// A frostbite lasts a second after the hit that leaves it, the engine's ailment uptime.
const FROSTBITE_FRAMES: u32 = 60;

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    spawns.iter().map(|spawn| waves(spawn, tree)).collect()
}

/// One cast's waves: Watery Turbulence's fan, the case's size and top speed, the ramp, Water
/// Walker's buff, what the projEffect hands every wave and Ocean Pullback's way back.
fn waves(base: &Spawn, tree: &Tree) -> Spawn {
    let wave = Spawn {
        extra: tree.own(TURBULENCE, "projectile_count").round() as u32,
        speed: top_speed(base, tree),
        scale: Some(wave_scale(base, tree)),
        ramp: ramp(tree, 0),
        buff: tree.learned(WALKER).then(|| Buff {
            frames: WALKER_FRAMES,
            percent: tree.own(WALKER, "movement_speed"),
        }),
        ..base.clone()
    };
    let back = tree.learned(PULLBACK).then(|| pullback(&wave, tree));
    Spawn {
        on_end: back.into_iter().collect(),
        ..hits(wave, 1., tree)
    }
}

/// maxSpeed: 4.5 times one plus the speed Chaining Waves, Beyond the Horizon and The Last Water
/// Bender add up (L0x2bd); no duration reaches the wave, whose Begin Step resets image_speed.
fn top_speed(base: &Spawn, tree: &Tree) -> f32 {
    (base.speed * (1. + tree.stat("projectile_speed_pct") / 100.)).max(0.)
}

/// image_xscale: 0.75, halved by Watery Turbulence, times Giant Waves' size, then the size bonus;
/// Trembling Tidalwave widens image_yscale.
fn wave_scale(base: &Spawn, tree: &Tree) -> [f32; 2] {
    let half = if tree.learned(TURBULENCE) {
        TURBULENCE_SIZE
    } else {
        1.
    };
    let size = half * (1. + tree.own(GIANT, "projectile_size") / 100.);
    let [x, y] = base
        .scale
        .unwrap_or([1.; 2])
        .map(|s| s * size + tree.mods.size_bonus);
    let width = if tree.learned(TREMBLING) {
        TREMBLING_WIDTH
    } else {
        0.
    };
    [x, y + width]
}

/// Ramping Waves: rampingWaves more damage every 0.5 s, counted on from `from` frames of the wave.
fn ramp(tree: &Tree, from: u32) -> Option<Ramp> {
    tree.learned(RAMPING).then(|| Ramp {
        every: RAMP_EVERY,
        rate: tree.own(RAMPING, "cold_skill_damage") / 100.,
        from,
    })
}

/// Ocean Pullback (Other_7): at the animation's end the wave turns to the player at its top
/// speed, forgets whom it hit, deals oceanPullBack[0] more and grows by oceanPullBack[1] for one
/// more animation; it starts a step along, as the game keeps moving it.
fn pullback(wave: &Spawn, tree: &Tree) -> Spawn {
    let grow = tree.own(PULLBACK, "projectile_size") / 100.;
    let back = Spawn {
        object: wave.object.clone(),
        turn: Some(180.),
        offset: wave.speed,
        speed: wave.speed,
        scale: wave.scale.map(|scale| scale.map(|s| s + grow)),
        life: wave.life,
        active_from: Some(0),
        ramp: ramp(tree, wave.life.unwrap_or(0)),
        ..Spawn::default()
    };
    hits(
        back,
        1. + tree.own(PULLBACK, "cold_skill_damage") / 100.,
        tree,
    )
}

/// What the case's projEffect hands a wave dealing `share` of the hit: the waterspouts and what
/// every damage object of the cast gets.
fn hits(wave: Spawn, share: f32, tree: &Tree) -> Spawn {
    let spouts = tree
        .proc(WATERSPOUT, "of_total_damage")
        .map(|(chance, part)| Proc {
            chance,
            spawn: waterspout(share * part / 100., tree),
            pull: None,
            once: false,
        });
    Spawn {
        procs: spouts.into_iter().collect(),
        ..cold(wave, share, tree)
    }
}

/// A damage object dealing `share` of the hit with Arctic Aqua's frostbite, rolled once a cast at
/// the cast's damage, and Salt Balance's extra damage to frostbitten enemies.
fn cold(spawn: Spawn, share: f32, tree: &Tree) -> Spawn {
    let frostbite = tree
        .proc(ARCTIC, "increased_frost_bite_damage")
        .map(|(chance, _)| Frostbite {
            chance,
            share: tree.applies(ARCTIC, "frostbite") / 100. / share,
            lasts: FROSTBITE_FRAMES,
        });
    Spawn {
        share: Some(share),
        frostbite,
        vs_frostbitten: tree.own(SALT, "extra_damage_frost_bitten") / 100.,
        ..spawn
    }
}

/// The waterspout a hit may raise at the enemy and the burst it leaves for `share` of the hit.
fn waterspout(share: f32, tree: &Tree) -> Spawn {
    let burst = Spawn {
        object: BURST.into(),
        scale: Some([SPOUT_SCALE; 2]),
        active_from: Some(BURST_ACTIVE),
        life: Some(BURST_LIFE),
        ..Spawn::default()
    };
    Spawn {
        object: SPOUT.into(),
        at_enemy: true,
        ability: true,
        scale: Some([SPOUT_SCALE; 2]),
        life: Some(SPOUT_LIFE),
        on_end: vec![cold(burst, share, tree)],
        ..Spawn::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::skill_effects::Proc;
    use crate::skill_mods::Mods;
    use crate::skill_mods::parity_tests as parity;
    use crate::skill_sim::{Shape, Spawn, Timeline, run};
    use std::collections::HashMap;

    const WAVE: &str = "Exo_Tsunami_obj";
    const WATERSPOUT: &str = "Exo_Tsunami_Waterspout_obj";
    const BURST: &str = "Explosion_Item_obj";

    fn mods(ranks: &[(&str, u32)]) -> Mods {
        let ranks: HashMap<String, u32> = ranks
            .iter()
            .map(|(node, rank)| (format!("tsunami:{node}"), *rank))
            .collect();
        Mods::of("tsunami", &ranks)
    }

    fn cast_with(mods: &Mods) -> Timeline {
        run("tsunami", mods, 1).expect("tsunami")
    }

    fn cast(ranks: &[(&str, u32)]) -> Timeline {
        cast_with(&mods(ranks))
    }

    fn spawn_with(ranks: &[(&str, u32)]) -> Spawn {
        let base = &crate::skill_sim::skill("tsunami").expect("tsunami").spawn;
        crate::skill_mods::apply("tsunami", base, &mods(ranks))[0].clone()
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

    fn scale_of(timeline: &Timeline) -> [f32; 2] {
        births(timeline, WAVE)[0].1.scale
    }

    /// Positions of the first wave, frame by frame.
    fn track(timeline: &Timeline) -> Vec<[f32; 2]> {
        timeline
            .frames
            .iter()
            .filter_map(|shapes| shapes.iter().find(|s| s.object.name == WAVE))
            .map(|shape| shape.pos)
            .collect()
    }

    /// px the first wave moves between its two last frames.
    fn top_speed(timeline: &Timeline) -> f32 {
        let path = track(timeline);
        let [a, b] = [path[path.len() - 2], path[path.len() - 1]];
        (b[0] - a[0]).hypot(b[1] - a[1])
    }

    fn oldest(timeline: &Timeline) -> u32 {
        timeline
            .frames
            .iter()
            .flatten()
            .filter(|s| s.object.name == WAVE)
            .map(|s| s.age)
            .max()
            .unwrap_or(0)
    }

    fn shares(timeline: &Timeline) -> Vec<f32> {
        let hits = timeline.effects.damage.iter().filter(|d| !d.frost);
        hits.map(|d| d.share).collect()
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn the_base_wave_speeds_up_to_4_5_px_and_hits_the_dummy_once_after_21_moves() {
        // deltaSpeed 0.1, then x1.05 + 0.1 a frame: the front reaches the dummy after 21 moves.
        let base = cast(&[]);
        assert_eq!(base.hits, [21]);
        let path = track(&base);
        assert!(near(path[0][0], 16.) && near(path[1][0], 16.1), "{path:?}");
        assert!(near(path[2][0], 16.305), "{path:?}");
        assert!(near(top_speed(&base), 4.5));
        assert_eq!(oldest(&base), 93);
    }

    #[test]
    fn damage_and_crit_nodes_leave_the_use_unchanged() {
        let base = cast(&[]);
        let same = cast(&[
            ("heavy_water", 5),
            ("asphyxiating_depths", 5),
            ("salt_balance", 5),
            ("water_walker", 5),
        ]);
        assert_eq!(same.hits, base.hits);
        assert_eq!(same.frames.len(), base.frames.len());
        assert_eq!(shares(&same), [1.]);
    }

    #[test]
    fn speed_nodes_add_up_into_the_waves_top_speed() {
        assert!(near(
            top_speed(&cast(&[("chaining_waves", 5)])),
            4.5 * 1.625
        ));
        assert!(near(
            top_speed(&cast(&[("the_last_water_bender", 3)])),
            4.5 * 1.75
        ));
        let slow = cast(&[("beyond_the_horizon", 5)]);
        assert!(near(top_speed(&slow), 4.5 * 0.375), "{}", top_speed(&slow));
        let all = cast(&[
            ("chaining_waves", 5),
            ("beyond_the_horizon", 5),
            ("the_last_water_bender", 3),
        ]);
        assert!(near(top_speed(&all), 4.5 * 1.75), "{}", top_speed(&all));
    }

    #[test]
    fn no_duration_stretches_the_wave_whose_begin_step_resets_its_image_speed() {
        let mods = Mods {
            duration_pct: 50.,
            ..mods(&[("beyond_the_horizon", 5)])
        };
        let slow = cast_with(&mods);
        assert_eq!(oldest(&slow), 93);
        assert_eq!(slow.hits.len(), 1);
    }

    #[test]
    fn giant_waves_widen_the_wave_20_percent_a_rank() {
        let giant = spawn_with(&[("giant_waves", 5)]);
        assert_eq!(giant.scale, Some([1.5, 1.5]));
    }

    #[test]
    fn size_stats_add_their_bonus_to_the_wave() {
        let mods = Mods {
            size_bonus: 0.3,
            ..mods(&[])
        };
        let scale = scale_of(&cast_with(&mods));
        assert!(near(scale[0], 1.05) && near(scale[1], 1.05));
    }

    #[test]
    fn trembling_tidalwave_adds_2_to_the_waves_width_only() {
        assert_eq!(
            spawn_with(&[("trembling_tidalwave", 1)]).scale,
            Some([0.75, 2.75])
        );
        let both = spawn_with(&[("trembling_tidalwave", 3), ("giant_waves", 5)]);
        assert_eq!(both.scale, Some([1.5, 3.5]));
    }

    #[test]
    fn watery_turbulence_fans_3_more_half_size_waves_a_rank_18_degrees_apart() {
        let fan = cast(&[("watery_turbulence", 1)]);
        let waves = births(&fan, WAVE);
        let mut angles: Vec<f32> = waves.iter().map(|(_, s)| s.angle).collect();
        angles.sort_by(f32::total_cmp);
        assert_eq!(angles, [-27., -9., 9., 27.]);
        for (_, wave) in &waves {
            assert_eq!(wave.scale, [0.375, 0.375]);
            let (sin, cos) = wave.angle.to_radians().sin_cos();
            assert!(near(wave.pos[0], 16. * cos) && near(wave.pos[1], -16. * sin));
        }
        assert_eq!(births(&cast(&[("watery_turbulence", 3)]), WAVE).len(), 10);
    }

    #[test]
    fn arctic_aqua_rolls_frostbite_per_cast_and_salt_balance_raises_frostbitten_hits() {
        let cold = spawn_with(&[("arctic_aqua", 5), ("salt_balance", 5)]);
        let frost = cold.frostbite.expect("a frostbite roll");
        assert!(near(frost.chance, 0.25) && near(frost.share, 1.5));
        assert!(near(cold.vs_frostbitten, 0.75));
    }

    #[test]
    fn water_walker_speeds_the_player_for_3_s() {
        let fast = cast(&[("water_walker", 5)]);
        assert_eq!(fast.effects.buffed, Some(([0, 180], 100.)));
    }

    #[test]
    fn ocean_pullback_turns_the_wave_back_to_the_player_bigger_and_stronger() {
        let back = cast(&[("ocean_pullback", 3)]);
        let waves = births(&back, WAVE);
        assert_eq!(waves.len(), 2);
        let (frame, wave) = waves[1];
        assert_eq!(frame, 93);
        assert!(near(wave.angle, 180.), "{}", wave.angle);
        assert!(near(wave.scale[0], 1.2) && near(wave.scale[1], 1.2));
        let path = track(&back);
        let [end, start] = [path[92], path[93]];
        assert!(near(end[0] - start[0], 4.5), "{end:?} {start:?}");
        // From 363 px the 1.2 wave's front (132 px x 1.2) reaches the dummy 8 steps after it turns.
        assert_eq!(back.hits, [21, 101]);
        assert_eq!(shares(&back), [1., 2.5]);
        assert_eq!(cast(&[("ocean_pullback", 1)]).hits, [21, 110]);
    }

    #[test]
    fn ramping_waves_grow_the_damage_every_half_second() {
        // Ramps at frames 30, 60 and 90 before the return hit; the outgoing hit comes earlier.
        let ramped = cast(&[("ramping_waves", 5), ("ocean_pullback", 3)]);
        let got = shares(&ramped);
        assert!(near(got[0], 1.), "{got:?}");
        assert!(near(got[1], 2.5 * 1.325f32.powi(3)), "{got:?}");
        assert_eq!(shares(&cast(&[("ramping_waves", 5)])), [1.]);
    }

    #[test]
    fn the_frostbite_keeps_the_casts_damage_on_the_way_back() {
        let back = spawn_with(&[("arctic_aqua", 5), ("ocean_pullback", 3)]).on_end[0].clone();
        let frost = back.frostbite.expect("the cast's frostbite");
        assert!(near(back.share.unwrap_or(1.) * frost.share, 1.5));
    }

    #[test]
    fn waterspouts_rise_at_the_dummy_and_burst_for_their_share() {
        let wave = spawn_with(&[("waterspout", 5), ("ocean_pullback", 3)]);
        let proc = &wave.procs[0];
        assert!(near(proc.chance, 0.2));
        let spout = &proc.spawn;
        assert!(spout.at_enemy && spout.ability);
        assert_eq!((spout.scale, spout.life), (Some([0.66, 0.66]), Some(36)));
        let burst = &spout.on_end[0];
        assert_eq!(burst.object, BURST);
        assert_eq!(burst.share, Some(1.25));
        assert_eq!((burst.active_from, burst.life), (Some(4), Some(40)));
        let back = &wave.on_end[0].procs[0].spawn.on_end[0];
        assert_eq!(back.share, Some(1.25 * 2.5));
        let uses = crate::skill_sim::uses(
            "tsunami",
            &mods(&[
                ("waterspout", 5),
                ("ocean_pullback", 3),
                ("watery_turbulence", 2),
            ]),
        )
        .expect("tsunami uses");
        let rising: Vec<(usize, [f32; 2])> = uses
            .iter()
            .flat_map(|u| births(u, WATERSPOUT).into_iter().map(|(f, s)| (f, s.pos)))
            .collect();
        assert!(!rising.is_empty(), "no waterspout in {} uses", uses.len());
        assert!(
            rising.iter().all(|(_, pos)| *pos == [160., 0.]),
            "{rising:?}"
        );
        for timeline in uses.iter() {
            let frames = |object| births(timeline, object).into_iter().map(|(f, _)| f);
            let spouts: Vec<usize> = frames(WATERSPOUT).map(|f| f + 36).collect();
            assert_eq!(frames(BURST).collect::<Vec<_>>(), spouts);
        }
    }

    #[test]
    fn a_waterspouts_burst_carries_the_ramped_damage_of_the_hit() {
        let uses = crate::skill_sim::uses(
            "tsunami",
            &mods(&[
                ("waterspout", 5),
                ("ramping_waves", 5),
                ("ocean_pullback", 3),
                ("watery_turbulence", 2),
            ]),
        )
        .expect("tsunami uses");
        let ramped = 1.25 * 2.5 * 1.325f32.powi(3);
        let found = uses
            .iter()
            .any(|u| shares(u).iter().any(|s| near(*s, ramped)));
        assert!(found, "no burst from a ramped return hit");
    }

    #[test]
    fn every_node_at_once_still_plays_a_bounded_use() {
        let tree = [
            ("heavy_water", 5),
            ("arctic_aqua", 5),
            ("water_walker", 5),
            ("ramping_waves", 5),
            ("waterspout", 5),
            ("chaining_waves", 5),
            ("salt_balance", 5),
            ("giant_waves", 5),
            ("asphyxiating_depths", 5),
            ("beyond_the_horizon", 5),
            ("trembling_tidalwave", 3),
            ("watery_turbulence", 3),
            ("ocean_pullback", 3),
            ("the_last_water_bender", 3),
        ];
        let storm = cast(&tree);
        assert_eq!(births(&storm, WAVE).len(), 20);
        assert!(storm.hits.len() >= 2, "{:?}", storm.hits);
        assert!(storm.frames.len() < 400, "{}", storm.frames.len());
    }

    #[test]
    fn the_builds_own_size_stats_reach_the_wave() {
        use hsplanner_build::BuildSnapshot;
        use hsplanner_engine::calc::stats::ComputedStats;
        let snapshot = BuildSnapshot {
            class_id: Some("exo".into()),
            active_skill_ids: vec!["tsunami".into()],
            subskill_ranks: HashMap::from([("tsunami:giant_waves".into(), 2)]),
            ..Default::default()
        };
        let computed = ComputedStats {
            stats: HashMap::from([("area_of_effect".into(), (30., 30.))]),
            ..Default::default()
        };
        let mods = Mods::for_build("tsunami", &snapshot, Some(&computed));
        assert_eq!(mods.rank("giant_waves"), 2);
        assert!(mods.size_bonus > 0.1, "{}", mods.size_bonus);
        let scale = scale_of(&cast_with(&mods));
        assert!(near(scale[0], 0.75 * 1.4 + mods.size_bonus));
    }

    /// `spawn` and what it makes without frostbite: the engine deals that as its frostbite
    /// ailment, not as hits.
    fn unfrozen(spawn: &Spawn) -> Spawn {
        let procs = spawn.procs.iter().map(|proc| Proc {
            spawn: unfrozen(&proc.spawn),
            ..proc.clone()
        });
        Spawn {
            frostbite: None,
            on_end: spawn.on_end.iter().map(unfrozen).collect(),
            procs: procs.collect(),
            ..spawn.clone()
        }
    }

    /// Damage-weighted hits a cast is expected to land, Waterspout's roll at its odds.
    fn expected(ranks: &[(&str, u32)]) -> f32 {
        let made = parity::rewritten("exo", "tsunami", ranks, 0, super::apply);
        let made: Vec<Spawn> = made.iter().map(unfrozen).collect();
        parity::expected("tsunami", &made)
    }

    fn engine(ranks: &[(&str, u32)]) -> f32 {
        parity::engine_hits("exo", "tsunami", ranks)
    }

    /// Every rank of `a` with every rank of `b`, on top of `with`.
    fn pairs(
        with: &[(&'static str, u32)],
        a: (&'static str, u32),
        b: (&'static str, u32),
    ) -> Vec<Vec<(&'static str, u32)>> {
        let ranks = |(node, max): (&'static str, u32)| (1..=max).map(move |rank| (node, rank));
        let rows = ranks(a).flat_map(|x| ranks(b).map(move |y| [x, y]));
        rows.map(|pair| with.iter().copied().chain(pair).collect())
            .collect()
    }

    #[test]
    fn the_engines_subskill_hits_follow_the_sim() {
        let (sim, base) = (expected(&[]), engine(&[]));
        assert!(near(sim, 1.) && near(base, 1.), "sim {sim}, engine {base}");
        let wrong = parity::misses("exo", "tsunami", expected, engine, 0.01);
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }

    #[test]
    fn the_engine_follows_the_nodes_the_sim_plays_together() {
        let (spout, ramping) = (("waterspout", 5), ("ramping_waves", 5));
        let (fan, back) = (("watery_turbulence", 3), ("ocean_pullback", 3));
        let (giant, trembling) = (("giant_waves", 5), ("trembling_tidalwave", 3));
        let mut together = pairs(&[], spout, fan);
        together.extend(pairs(&[], spout, back));
        together.extend(pairs(&[], ramping, fan));
        together.extend(pairs(&[], ramping, back));
        together.extend(pairs(&[], fan, back));
        together.extend(pairs(&[], giant, fan));
        together.extend(pairs(&[], trembling, fan));
        for rank in 1..=3 {
            together.extend(pairs(&[("watery_turbulence", rank)], giant, trembling));
        }
        together.extend(pairs(&[], ("arctic_aqua", 5), ("salt_balance", 5)));
        let wrong: Vec<String> = (together.iter())
            .filter_map(|ranks| {
                let (sim, engine) = (expected(ranks), engine(ranks));
                let off = (sim - engine).abs() >= 0.01;
                off.then(|| format!("{ranks:?}: sim {sim:.4}, engine {engine:.4}"))
            })
            .collect();
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }
}
