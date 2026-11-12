//! Exo's Black Hole: TalentsExo case 361 sets the hole from the subskill ranks, Exo_Black_Hole_obj
//! moves and swells it, ProjectileCollision20Exo rolls its procs on each hit.
use super::Tree;
use crate::skill_effects::{Buff, Drain, Frostbite, Orbit, Proc, Pull, Swell};
use crate::skill_sim::{Grow, Spawn};

const HIDDEN: &str = "hidden_in_plain_sight";
const SUPERMASSIVE: &str = "supermassive_black_hole";
const GALAXY: &str = "galaxy_devouring_growth";
const EMPOWERED: &str = "empowered_singularity";
const RADIATION: &str = "interstellar_radiation";
const SINGULARITY: &str = "singularity";
const WATERMELTING: &str = "watermelting_cold";
const SUBZERO: &str = "subzero_frequency";
const TRAVELER: &str = "time_traveler";
/// TalentsExo case 361 constants: the shortest tick (0.1 s) and the transformations' factors.
const TICK_FLOOR: f32 = 6.;
const HIDDEN_SIZE: f32 = 0.5;
const SUPERMASSIVE_SIZE: f32 = 2.25;
const SUPERMASSIVE_TICK: f32 = 0.75;
const GALAXY_TICK: f32 = 1.5;
const GALAXY_LIFE: f32 = 0.75;
/// Empowered Singularity's hole lasts 0.5 s whatever stretches it.
const EMPOWERED_LIFE: u32 = 30;
/// Exo_Black_Hole_obj: orbiting holes circle 6 px below the player, 128 px out, 4° a frame.
const ORBIT: Orbit = Orbit {
    radius: 128.,
    speed: 4.,
    count: 1,
    below: 6.,
    ..Orbit::CIRCLE
};
/// Exo_Black_Hole_obj Step_0 rolls Galaxy Devouring Growth every 0.25 s.
const GALAXY_EVERY: u32 = 15;
/// Exo_Black_Hole_obj Step_0: Supermassive's first drain after 0.5 s, then one a second.
const DRAIN_FIRST: u32 = 30;
const DRAIN_EVERY: u32 = 60;
/// A frostbite lasts a second after the hit that leaves it, the engine's ailment uptime.
const FROSTBITE_FRAMES: u32 = 60;
/// TalentPullMonster pulls for a second; Empowered Singularity pulls only enemies beyond 64 px.
const PULL_FRAMES: u32 = 60;
const EMPOWERED_REACH: f32 = 64.;

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    spawns
        .iter()
        .map(|spawn| hole(spawn.clone(), tree))
        .collect()
}

/// A use whose every swell roll comes up `lucky`.
#[cfg(test)]
pub(super) fn apply_rolled(spawns: &[Spawn], tree: &Tree, lucky: bool) -> Vec<Spawn> {
    let chance = if lucky { 1. } else { 0. };
    let rolled = |hole: Spawn| Spawn {
        swell: hole.swell.map(|swell| Swell { chance, ..swell }),
        ..hole
    };
    apply(spawns, tree).into_iter().map(rolled).collect()
}

fn hole(mut hole: Spawn, tree: &Tree) -> Spawn {
    let duration = hole.life;
    hole.grow = Some(Grow {
        max: Some(hole_size(&hole, tree)),
        ..hole.grow.unwrap_or_default()
    });
    hole.tick = hole_tick(hole.tick, tree);
    hole.life = hole_life(hole.life, tree);
    if tree.learned(HIDDEN) {
        hole.target = false;
        hole.orbit = Some(Orbit {
            count: 1 + tree.stat("projectile_count").round() as u32,
            ..ORBIT
        });
    }
    hole.share = Some(hole_share(tree));
    add_ailments(&mut hole, tree);
    add_player_effects(&mut hole, tree, duration);
    add_rolls(&mut hole, tree);
    if tree.learned(EMPOWERED) {
        hole.pull = Some(Pull {
            frames: PULL_FRAMES,
            beyond: EMPOWERED_REACH,
            at: None,
        });
    }
    hole
}

/// Hidden's and Supermassive's holes deal their of_total_damage share of the hit.
fn hole_share(tree: &Tree) -> f32 {
    [HIDDEN, SUPERMASSIVE]
        .into_iter()
        .filter(|node| tree.learned(node))
        .map(|node| tree.own(node, "of_total_damage") / 100.)
        .product()
}

/// Watermelting Cold's frostbite, rolled per cast as the cast rolls it, and Subzero Frequency's
/// extra damage on frostbitten targets.
fn add_ailments(hole: &mut Spawn, tree: &Tree) {
    if let Some((chance, _)) = tree.proc(WATERMELTING, "increased_frost_bite_damage") {
        hole.frostbite = Some(Frostbite {
            chance,
            share: tree.applies(WATERMELTING, "frostbite") / 100.,
            lasts: FROSTBITE_FRAMES,
        });
    }
    hole.vs_frostbitten = tree.own(SUBZERO, "extra_damage_frost_bitten") / 100.;
}

/// Time Traveler's movement speed for the skill's duration, given at the cast, and Supermassive's
/// drain on the player's mana.
fn add_player_effects(hole: &mut Spawn, tree: &Tree, duration: Option<u32>) {
    if tree.learned(TRAVELER) {
        hole.buff = Some(Buff {
            frames: duration.unwrap_or(0),
            percent: tree.own(TRAVELER, "movement_speed"),
        });
    }
    if tree.learned(SUPERMASSIVE) {
        hole.drain = Some(Drain {
            first: DRAIN_FIRST,
            every: DRAIN_EVERY,
            share: tree.own(SUPERMASSIVE, "mana_drain_pct") / 100.,
        });
    }
}

/// maxScale: the subtree's size, Hidden's half and Supermassive's 2.25, then the build's size bonus.
fn hole_size(hole: &Spawn, tree: &Tree) -> f32 {
    let base = hole.grow.and_then(|grow| grow.max).unwrap_or(1.);
    let mut max = base * (1. + tree.stat("area_of_effect") / 100.);
    if tree.learned(HIDDEN) {
        max *= HIDDEN_SIZE;
    }
    if tree.learned(SUPERMASSIVE) {
        max *= SUPERMASSIVE_SIZE;
    }
    max + tree.mods.size_bonus
}

/// tick_frequency: Time != Space shortens it (the planner files it under faster_cast_rate),
/// Galaxy slows it and Supermassive speeds it, then the cast floors it at 0.1 s.
fn hole_tick(tick: f32, tree: &Tree) -> f32 {
    let mut tick = tick * (1. - tree.stat("faster_cast_rate") / 100.);
    if tree.learned(GALAXY) {
        tick *= GALAXY_TICK;
    }
    if tree.learned(SUPERMASSIVE) {
        tick *= SUPERMASSIVE_TICK;
    }
    tick.max(TICK_FLOOR)
}

/// destroyTimer: Empowered's fixed 0.5 s, else stretched by Skill Duration and cut by Galaxy.
fn hole_life(life: Option<u32>, tree: &Tree) -> Option<u32> {
    if tree.learned(EMPOWERED) {
        return Some(EMPOWERED_LIFE);
    }
    let stretch = 1. + (tree.stat("skill_duration") + tree.mods.duration_pct) / 100.;
    let galaxy = if tree.learned(GALAXY) {
        GALAXY_LIFE
    } else {
        1.
    };
    life.map(|life| (life as f32 * stretch * galaxy).round() as u32)
}

/// Galaxy's swell rolls and the procs each hit rolls.
fn add_rolls(hole: &mut Spawn, tree: &Tree) {
    if let Some((chance, size)) = tree.proc(GALAXY, "projectile_size") {
        let damage = tree
            .proc(GALAXY, "cold_skill_damage")
            .map_or(0., |(_, damage)| damage);
        hole.swell = Some(Swell {
            every: GALAXY_EVERY,
            chance,
            step: size / 100.,
            damage: damage / 100.,
        });
    }
    let share = hole.share.unwrap_or(1.);
    let kinds = [Roll::Radiation, Roll::Singularity];
    hole.procs.extend(rolls(&kinds, share, tree));
}

#[derive(Clone, Copy, PartialEq)]
enum Roll {
    Radiation,
    Singularity,
}

/// The rolls among `kinds` that a damage object dealing `share` of the hit makes on each hit.
/// What one makes rolls the others: InheritModifiers leaves out only the roll that made it.
fn rolls(kinds: &[Roll], share: f32, tree: &Tree) -> Vec<Proc> {
    kinds
        .iter()
        .filter_map(|&kind| {
            let node = match kind {
                Roll::Radiation => RADIATION,
                Roll::Singularity => SINGULARITY,
            };
            let (chance, part) = tree.proc(node, "of_total_damage")?;
            let share = share * part / 100.;
            let rest: Vec<Roll> = kinds
                .iter()
                .copied()
                .filter(|other| *other != kind)
                .collect();
            let made = match kind {
                Roll::Radiation => flare(share),
                Roll::Singularity => burst(share),
            };
            let pull = (kind == Roll::Singularity).then_some(Pull {
                frames: PULL_FRAMES,
                beyond: 0.,
                at: None,
            });
            let spawn = Spawn {
                procs: rolls(&rest, share, tree),
                ..made
            };
            Some(Proc {
                chance,
                spawn,
                pull,
                once: false,
            })
        })
        .collect()
}

/// Interstellar Radiation's cosmic flare: from the hole's centre at the enemy it hit, 8 px a frame,
/// hitting each enemy once (Exo_Black_Hole_Cosmic_Flare_obj, ProjectileCollision20Exo).
fn flare(share: f32) -> Spawn {
    Spawn {
        object: "Exo_Black_Hole_Cosmic_Flare_obj".into(),
        share: Some(share),
        aim: true,
        speed: 8.,
        scale: Some([0.75, 0.75]),
        grow: Some(Grow {
            step: 0.05,
            max: Some(1.),
            ..Grow::default()
        }),
        life: Some(90),
        fade: 20,
        shrink: 0.05,
        ..Spawn::default()
    }
}

/// Singularity's burst: an Explosion_Item_obj with Hit_Impact_Arcane_spr on the enemy hit.
fn burst(share: f32) -> Spawn {
    Spawn {
        object: "Explosion_Item_obj".into(),
        share: Some(share),
        at_enemy: true,
        scale: Some([0.4, 0.4]),
        active_from: Some(4),
        life: Some(21),
        ..Spawn::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::skill_mods::Mods;
    use crate::skill_sim::{Shape, Timeline, run};
    use std::collections::HashMap;

    const HOLE: &str = "Exo_Black_Hole_obj";
    const FLARE: &str = "Exo_Black_Hole_Cosmic_Flare_obj";
    const BURST: &str = "Explosion_Item_obj";

    fn mods(ranks: &[(&str, u32)]) -> Mods {
        let ranks: HashMap<String, u32> = ranks
            .iter()
            .map(|(node, rank)| (format!("black_hole:{node}"), *rank))
            .collect();
        Mods::of("black_hole", &ranks)
    }

    fn hole_with(mods: &Mods) -> Timeline {
        run("black_hole", mods, 1).expect("black hole")
    }

    fn hole(ranks: &[(&str, u32)]) -> Timeline {
        hole_with(&mods(ranks))
    }

    fn shapes<'a>(timeline: &'a Timeline, object: &'a str) -> impl Iterator<Item = &'a Shape> {
        timeline
            .frames
            .iter()
            .flatten()
            .filter(move |shape| shape.object.name == object)
    }

    fn widest(timeline: &Timeline) -> f32 {
        shapes(timeline, HOLE)
            .map(|s| s.scale[0])
            .fold(0., f32::max)
    }

    fn oldest(timeline: &Timeline) -> u32 {
        shapes(timeline, HOLE).map(|s| s.age).max().unwrap_or(0)
    }

    #[test]
    fn the_base_hole_grows_to_0_8_lives_4_5_s_and_ticks_every_0_4_s() {
        let base = hole(&[]);
        assert!((widest(&base) - 0.8).abs() < 1e-4, "{}", widest(&base));
        assert_eq!(oldest(&base), 270 + 2 + 20);
        assert_eq!(base.hits.len(), 12);
    }

    #[test]
    fn super_massive_widens_the_hole_by_12_percent_a_rank() {
        let wide = hole(&[("super_massive", 5)]);
        assert!(
            (widest(&wide) - 0.8 * 1.6).abs() < 1e-4,
            "{}",
            widest(&wide)
        );
    }

    #[test]
    fn size_stats_add_their_bonus_to_the_holes_scale() {
        let mods = Mods {
            size_bonus: 0.3,
            ..mods(&[])
        };
        assert!((widest(&hole_with(&mods)) - 1.1).abs() < 1e-4);
    }

    #[test]
    fn mass_density_and_skill_duration_stretch_the_hole_together() {
        let dense = hole(&[("mass_density", 5)]);
        assert_eq!((oldest(&dense), dense.hits.len()), (513 + 22, 22));
        let mods = Mods {
            duration_pct: 50.,
            ..mods(&[("mass_density", 5)])
        };
        let longer = hole_with(&mods);
        assert_eq!((oldest(&longer), longer.hits.len()), (648 + 22, 27));
    }

    #[test]
    fn time_space_shortens_the_tick_down_to_0_1_s() {
        // Rule 20 hits of a 290-frame hole: a 6-frame tick at rank 5.
        assert_eq!(hole(&[("time_space", 5)]).hits.len(), 42);
        assert_eq!(hole(&[("time_space", 1)]).hits.len(), 14);
    }

    #[test]
    fn galaxy_devouring_growth_swells_a_shorter_slower_hole() {
        let galaxy = hole(&[("galaxy_devouring_growth", 3)]);
        assert_eq!(oldest(&galaxy), 203 + 22);
        assert_eq!(galaxy.hits.len(), 7);
        assert!(widest(&galaxy) > 1.0, "{}", widest(&galaxy));
    }

    #[test]
    fn hidden_in_plain_sight_orbits_three_half_size_holes_around_the_player() {
        let orbit = hole(&[("hidden_in_plain_sight", 1)]);
        let first: Vec<&Shape> = orbit.frames[0]
            .iter()
            .filter(|s| s.object.name == HOLE)
            .collect();
        assert_eq!(first.len(), 3);
        let mut angles: Vec<f32> = first
            .iter()
            .map(|s| {
                let [x, y] = s.pos;
                assert!((x.hypot(y - 6.) - 128.).abs() < 0.5, "{:?}", s.pos);
                (-(y - 6.)).atan2(x).to_degrees().rem_euclid(360.)
            })
            .collect();
        angles.sort_by(f32::total_cmp);
        for (angle, expected) in angles.iter().zip([4., 124., 244.]) {
            assert!((angle - expected).abs() < 0.5, "{angles:?}");
        }
        assert!((widest(&orbit) - 0.4).abs() < 1e-4, "{}", widest(&orbit));
    }

    #[test]
    fn supermassive_black_hole_is_2_25_times_bigger_and_ticks_every_0_3_s() {
        let huge = hole(&[("supermassive_black_hole", 1)]);
        assert!((widest(&huge) - 1.8).abs() < 1e-4, "{}", widest(&huge));
        assert_eq!(huge.hits.len(), 16);
    }

    #[test]
    fn supermassive_with_time_space_keeps_the_0_1_s_floor() {
        let both = hole(&[("supermassive_black_hole", 1), ("time_space", 5)]);
        assert_eq!(both.hits.len(), 42);
    }

    #[test]
    fn hidden_and_supermassive_multiply_before_the_size_bonus() {
        let mods = Mods {
            size_bonus: 0.3,
            ..mods(&[("hidden_in_plain_sight", 1), ("supermassive_black_hole", 1)])
        };
        let both = hole_with(&mods);
        assert!(
            (widest(&both) - (0.8 * 0.5 * 2.25 + 0.3)).abs() < 1e-4,
            "{}",
            widest(&both)
        );
    }

    #[test]
    fn empowered_singularity_lives_half_a_second_and_grows_20_percent_a_rank() {
        let short = hole(&[("empowered_singularity", 1), ("mass_density", 5)]);
        assert_eq!((oldest(&short), short.hits.len()), (30 + 22, 2));
        assert!((widest(&short) - 0.96).abs() < 1e-4, "{}", widest(&short));
    }

    #[test]
    fn interstellar_radiation_shoots_flares_from_the_hole_at_the_dummy() {
        let flares = hole(&[("interstellar_radiation", 5), ("time_space", 5)]);
        let shots: Vec<&Shape> = shapes(&flares, FLARE).collect();
        assert!(!shots.is_empty(), "no flare in {} hits", flares.hits.len());
        let start = shots.iter().find(|s| s.age == 1).expect("a new flare");
        assert!((start.pos[0] - 160.).abs() < 1e-3 && (start.pos[1] + 30.).abs() < 1e-3);
        let next = shots.iter().find(|s| s.age == 2).expect("a moving flare");
        assert!(
            (next.pos[1] - start.pos[1] - 8.).abs() < 1e-3,
            "{:?}",
            next.pos
        );
    }

    #[test]
    fn singularity_bursts_on_the_dummy() {
        let bursts = hole(&[("singularity", 5), ("time_space", 5)]);
        let burst = shapes(&bursts, BURST).next().expect("a singularity burst");
        assert_eq!(burst.pos, bursts.dummy);
        assert!((burst.scale[0] - 0.4).abs() < 1e-4);
    }

    #[test]
    fn the_builds_own_skill_duration_and_size_stats_reach_the_hole() {
        use hsplanner_build::BuildSnapshot;
        use hsplanner_engine::calc::stats::{ComputedStats, SourceContribution, SourceType};
        let source = |source_type, value| SourceContribution {
            label: "test".into(),
            source_type,
            value: (value, value),
            forge: None,
        };
        let snapshot = BuildSnapshot {
            class_id: Some("exo".into()),
            active_skill_ids: vec!["black_hole".into()],
            subskill_ranks: HashMap::from([("black_hole:mass_density".into(), 5)]),
            ..Default::default()
        };
        // Mass Density's 90% is the subtree's own; the gear adds 50% duration and 30% area.
        let computed = ComputedStats {
            stats: HashMap::from([
                ("skill_duration".into(), (140., 140.)),
                ("area_of_effect".into(), (30., 30.)),
            ]),
            stat_sources: HashMap::from([(
                "skill_duration".into(),
                vec![
                    source(SourceType::Subskill, 90.),
                    source(SourceType::Item, 50.),
                ],
            )]),
            ..Default::default()
        };
        let mods = Mods::for_build("black_hole", &snapshot, Some(&computed));
        assert_eq!(mods.rank("mass_density"), 5);
        assert_eq!(mods.duration_pct, 50.);
        assert!(mods.size_bonus > 0.1, "{}", mods.size_bonus);
        assert_eq!(oldest(&hole_with(&mods)), 648 + 22);
    }

    fn spawn_with(ranks: &[(&str, u32)]) -> crate::skill_sim::Spawn {
        let base = &crate::skill_sim::skill("black_hole")
            .expect("black hole")
            .spawn;
        crate::skill_mods::apply("black_hole", base, &mods(ranks))[0].clone()
    }

    #[test]
    fn transformed_holes_deal_their_share_of_the_hit() {
        assert_eq!(spawn_with(&[]).share, Some(1.));
        assert_eq!(spawn_with(&[("hidden_in_plain_sight", 2)]).share, Some(1.1));
        assert_eq!(
            spawn_with(&[("supermassive_black_hole", 3)]).share,
            Some(1.5)
        );
        let orbit = hole(&[("hidden_in_plain_sight", 1)]);
        assert!(
            orbit
                .effects
                .damage
                .iter()
                .all(|d| (d.share - 0.55).abs() < 1e-5)
        );
    }

    #[test]
    fn watermelting_cold_rolls_frostbite_and_subzero_frequency_raises_its_hits() {
        let cold = spawn_with(&[("watermelting_cold", 5), ("subzero_frequency", 5)]);
        let frostbite = cold.frostbite.expect("a frostbite roll");
        assert!((frostbite.chance - 0.15).abs() < 1e-5 && (frostbite.share - 0.875).abs() < 1e-5);
        assert!((cold.vs_frostbitten - 0.775).abs() < 1e-5);
    }

    #[test]
    fn time_traveler_speeds_the_player_for_the_holes_duration() {
        let fast = hole(&[("time_traveler", 5), ("mass_density", 5)]);
        assert_eq!(fast.effects.buffed, Some(([0, 270], 75.)));
    }

    #[test]
    fn supermassive_black_hole_drains_mana_every_second() {
        let huge = hole(&[("supermassive_black_hole", 2)]);
        let frames: Vec<u32> = huge.effects.mana.iter().map(|(frame, _)| *frame).collect();
        assert_eq!(frames.len(), 5, "{frames:?}");
        assert!(frames.windows(2).all(|w| w[1] - w[0] == 60), "{frames:?}");
        assert!(
            huge.effects
                .mana
                .iter()
                .all(|(_, share)| (share - 0.06).abs() < 1e-5)
        );
    }

    #[test]
    fn singularity_pulls_and_empowered_singularity_only_pulls_from_afar() {
        let bursts = hole(&[("singularity", 5), ("time_space", 5)]);
        assert!(!bursts.effects.pulls.is_empty());
        assert!(
            hole(&[("empowered_singularity", 3)])
                .effects
                .pulls
                .is_empty()
        );
        let pull = spawn_with(&[("empowered_singularity", 1)])
            .pull
            .expect("a pull");
        assert_eq!((pull.frames, pull.beyond), (60, 64.));
    }

    #[test]
    fn procs_and_swells_carry_their_damage() {
        let rolls = spawn_with(&[
            ("interstellar_radiation", 5),
            ("singularity", 5),
            ("galaxy_devouring_growth", 3),
        ]);
        let shares: Vec<f32> = rolls
            .procs
            .iter()
            .map(|p| p.spawn.share.unwrap_or(1.))
            .collect();
        assert_eq!(shares, [2.5, 1.25]);
        assert!((rolls.swell.expect("a swell").damage - 0.96).abs() < 1e-5);
    }

    #[test]
    fn flares_and_bursts_roll_each_other_on_the_holes_damage() {
        let rolls = spawn_with(&[
            ("interstellar_radiation", 5),
            ("singularity", 5),
            ("hidden_in_plain_sight", 1),
        ]);
        let near = |share: Option<f32>, want: f32| (share.unwrap_or(1.) - want).abs() < 1e-5;
        let [flare, burst] = [&rolls.procs[0].spawn, &rolls.procs[1].spawn];
        // Hidden's hole deals 55 % of the hit, and what it makes deals its share of that.
        assert!(near(flare.share, 0.55 * 2.5) && near(burst.share, 0.55 * 1.25));
        assert_eq!((flare.procs.len(), burst.procs.len()), (1, 1));
        let (from_flare, from_burst) = (&flare.procs[0], &burst.procs[0]);
        assert!(from_flare.spawn.object == BURST && from_flare.pull.is_some());
        assert!(from_burst.spawn.object == FLARE && from_burst.pull.is_none());
        assert!(near(from_flare.spawn.share, 0.55 * 2.5 * 1.25));
        assert!(near(from_burst.spawn.share, 0.55 * 1.25 * 2.5));
        // InheritModifiers leaves out the roll that made each, so the chain stops there.
        assert!(from_flare.spawn.procs.is_empty() && from_burst.spawn.procs.is_empty());
    }

    #[test]
    fn damage_and_buff_nodes_leave_the_use_unchanged() {
        let base = hole(&[]);
        let same = hole(&[
            ("event_horizon", 5),
            ("watermelting_cold", 5),
            ("time_bender", 5),
            ("time_traveler", 5),
            ("subzero_frequency", 5),
        ]);
        assert_eq!(same.hits, base.hits);
        assert_eq!(same.frames.len(), base.frames.len());
    }
}

#[cfg(test)]
#[path = "black_hole_parity_tests.rs"]
mod parity;
