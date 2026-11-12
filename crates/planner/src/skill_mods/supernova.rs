//! Exo's Supernova: TalentsExo case 352 sets the novas from the subskill ranks, Exo_Supernova_obj
//! grows around the player, and ProjectileCollision20Exo spawns the procs on each hit; what a proc
//! spawns inherits the other procs, as InheritModifiers leaves out only its own (ARG4). Knockback
//! shows nothing: Attack_Dummy_obj Step_2 puts the dummy back on its spot every frame.
use super::Tree;
use crate::skill_effects::{Buff, Proc};
use crate::skill_motion::{Rebound, Trail};
use crate::skill_sim::{Fall, Grow, Spawn};

const CHAINED: &str = "chained_explosions";
const TARGETED: &str = "targeted_cosmic_event";
const FIERY: &str = "fiery_frenzy";
const CHAIN: &str = "chain_reactions";
const OUTFLUX: &str = "particle_outflux";
const CONNECTED: &str = "connected_by_sun";
const PLANETARY: &str = "planetary_collision";
/// Case 352 L0x23c: the second nova wakes 0.2 s after the first.
const NOVA_DELAY: u32 = 12;
/// Case 352 L0x211: Fiery Frenzy's buff 162 lasts 5 s.
const FRENZY_FRAMES: u32 = 300;
/// A 0.85 Explosion_Item_obj on the enemy hit (L0x82), deadly from its 4-frame alarm for its
/// 9-frame explosion at 30 fps and 0.75 image speed.
const BURST: &str = "Explosion_Item_obj";
const BURST_SCALE: f32 = 0.85;
const BURST_ACTIVE: u32 = 4;
const BURST_LIFE: u32 = 24;
/// L0x66-L0x6c and the fireball's events: 0.75 from the player at the enemy, 8 px a frame, growing
/// 0.05 a frame to 1, 1.5 s then shrinking away; it looks for an enemy within 320 px every 0.25 s.
const FIREBALL: &str = "Exo_Solar_Dash_Fireball_obj";
const FIREBALL_SPEED: f32 = 8.;
const FIREBALL_SCALE: f32 = 0.75;
const FIREBALL_GROWTH: f32 = 0.05;
const FIREBALL_LIFE: u32 = 90;
const FIREBALL_FADE: u32 = 20;
const REBOUND_EVERY: u32 = 16;
const REBOUND_REACH: f32 = 320.;
/// L0x115-L0x11a and Exo_Supernova_Connected_obj Step_0: from 12 px in front of the player, 5 px a
/// frame for 3 s, a flame whenever 40 px past the last (the check adds 6 px of height).
const TRAIL: &str = "Exo_Supernova_Connected_obj";
const TRAIL_OFFSET: f32 = 12.;
const TRAIL_SPEED: f32 = 5.;
const TRAIL_LIFE: u32 = 180;
const TRAIL_EVERY: u32 = 8;
/// Exo_Solar_Dash_Flame_obj: 3 s ticking every 0.5 s, then shrinking 0.075 a frame; the trail's
/// maxScale 0.5 only stops growth, so the flame keeps its scale of 1.
const FLAME: &str = "Exo_Solar_Dash_Flame_obj";
const FLAME_TICK: f32 = 30.;
const FLAME_LIFE: u32 = 180;
const FLAME_FADE: u32 = 14;
const FLAME_SHRINK: f32 = 0.075;
/// L0xae-L0xb3: a 0.35 Exo_Asteroid_obj on the enemy hit; Create_0 drops it from 320 px at 0.25
/// gravity once its 0.1 s alarm wakes it, and Destroy_0 leaves an explosion 1.85 times its size.
const ASTEROID: &str = "Exo_Asteroid_obj";
const ASTEROID_SCALE: f32 = 0.35;
const ASTEROID_DELAY: u32 = 5;
const ASTEROID_HEIGHT: f32 = 320.;
const ASTEROID_GRAVITY: f32 = 0.25;
const ASTEROID_BLAST: f32 = 1.85;

/// The on-hit procs case 352 pushes to the nova's collision list, in its order.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Chain,
    Outflux,
    Connected,
    Planetary,
}

const KINDS: [Kind; 4] = [Kind::Chain, Kind::Outflux, Kind::Connected, Kind::Planetary];

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    spawns.iter().flat_map(|spawn| cast(spawn, tree)).collect()
}

/// One cast: the nova and, on Chained Explosions' roll, a second one (L0x202-L0x203).
fn cast(base: &Spawn, tree: &Tree) -> Vec<Spawn> {
    let nova = nova(base, tree);
    let second = tree
        .proc(CHAINED, "of_total_damage")
        .map(|(chance, _)| Spawn {
            chance: Some(chance),
            frame: nova.frame + NOVA_DELAY,
            buff: None,
            ..nova.clone()
        });
    std::iter::once(nova).chain(second).collect()
}

/// maxScale 1.5 widened by Fight the Gravity then the size bonus; Targeted Cosmic Event drops it
/// at the cursor and lets go of the player (L0x215-L0x216); Fiery Frenzy buffs the cast.
fn nova(base: &Spawn, tree: &Tree) -> Spawn {
    let max = base.grow.and_then(|grow| grow.max).unwrap_or(1.);
    let max = max * (1. + tree.stat("area_of_effect") / 100.) + tree.mods.size_bonus;
    Spawn {
        target: base.target || tree.learned(TARGETED),
        grow: Some(Grow {
            max: Some(max),
            ..base.grow.unwrap_or_default()
        }),
        buff: tree.learned(FIERY).then(|| Buff {
            frames: FRENZY_FRAMES,
            percent: tree.own(FIERY, "fire_skill_damage"),
        }),
        procs: procs(&KINDS, 1., tree),
        ..base.clone()
    }
}

/// The procs a damage object dealing `share` of the hit rolls on each hit, each roll its own.
fn procs(kinds: &[Kind], share: f32, tree: &Tree) -> Vec<Proc> {
    kinds
        .iter()
        .filter_map(|&kind| {
            let rest: Vec<Kind> = kinds.iter().copied().filter(|k| *k != kind).collect();
            proc(kind, &rest, share, tree)
        })
        .collect()
}

/// One learned proc and what it creates, carrying the `rest` along.
fn proc(kind: Kind, rest: &[Kind], share: f32, tree: &Tree) -> Option<Proc> {
    let (chance, spawn) = match kind {
        Kind::Chain => {
            let (chance, part) = tree.proc(CHAIN, "of_total_damage")?;
            (chance, burst(share * part / 100., rest, tree))
        }
        Kind::Outflux => {
            let (chance, _) = tree.proc(OUTFLUX, "of_total_damage")?;
            (chance, fireball(share, rest, tree))
        }
        Kind::Connected => {
            let (chance, part) = tree.proc(CONNECTED, "of_total_damage")?;
            (chance, trail(share * (1. + part / 100.), rest, tree))
        }
        Kind::Planetary => {
            let (chance, part) = tree.proc(PLANETARY, "fire_skill_damage")?;
            (chance, asteroid(share * (1. + part / 100.), rest, tree))
        }
    };
    Some(Proc {
        chance,
        spawn,
        pull: None,
        once: false,
    })
}

/// Chain Reactions: ceil(damage x chainReactions) on the enemy hit.
fn burst(share: f32, rest: &[Kind], tree: &Tree) -> Spawn {
    Spawn {
        object: BURST.into(),
        at_enemy: true,
        scale: Some([BURST_SCALE; 2]),
        ..explosion(share, rest, tree)
    }
}

/// An Explosion_Item_obj dealing `share` of the hit once.
fn explosion(share: f32, rest: &[Kind], tree: &Tree) -> Spawn {
    Spawn {
        object: BURST.into(),
        active_from: Some(BURST_ACTIVE),
        life: Some(BURST_LIFE),
        share: Some(share),
        procs: procs(rest, share, tree),
        ..Spawn::default()
    }
}

/// Particle Outflux: the creator's damage, bouncing 3 times a rank (L0x71).
fn fireball(share: f32, rest: &[Kind], tree: &Tree) -> Spawn {
    let bounces = tree.own(OUTFLUX, "projectile_bounces").round() as u32;
    Spawn {
        object: FIREBALL.into(),
        at_player: true,
        aim: true,
        speed: FIREBALL_SPEED,
        scale: Some([FIREBALL_SCALE; 2]),
        grow: Some(Grow {
            step: FIREBALL_GROWTH,
            max: Some(1.),
            ..Grow::default()
        }),
        life: Some(FIREBALL_LIFE),
        fade: FIREBALL_FADE,
        shrink: FIREBALL_GROWTH,
        rebound: Some(Rebound {
            count: bounces,
            every: REBOUND_EVERY,
            reach: REBOUND_REACH,
        }),
        share: Some(share),
        procs: procs(rest, share, tree),
        ..Spawn::default()
    }
}

/// Connected by Sun: an unseen trail whose flames deal floor(damage x (1 + connectedBySun)).
fn trail(share: f32, rest: &[Kind], tree: &Tree) -> Spawn {
    let flame = Spawn {
        object: FLAME.into(),
        tick: FLAME_TICK,
        life: Some(FLAME_LIFE),
        fade: FLAME_FADE,
        shrink: FLAME_SHRINK,
        share: Some(share),
        procs: procs(rest, share, tree),
        ..Spawn::default()
    };
    Spawn {
        object: TRAIL.into(),
        at_player: true,
        aim: true,
        offset: TRAIL_OFFSET,
        speed: TRAIL_SPEED,
        ability: true,
        life: Some(TRAIL_LIFE),
        trail: Some(Box::new(Trail {
            every: TRAIL_EVERY,
            first: 0,
            gap: 0.,
            spawn: flame,
            next: None,
        })),
        ..Spawn::default()
    }
}

/// Planetary Collision: a small asteroid whose explosion deals floor(damage x (1 + planetary)).
fn asteroid(share: f32, rest: &[Kind], tree: &Tree) -> Spawn {
    let blast = Spawn {
        scale: Some([ASTEROID_BLAST * ASTEROID_SCALE; 2]),
        ..explosion(share, rest, tree)
    };
    Spawn {
        object: ASTEROID.into(),
        at_enemy: true,
        ability: true,
        frame: ASTEROID_DELAY,
        fall: Some(Fall {
            height: ASTEROID_HEIGHT,
            gravity: ASTEROID_GRAVITY,
            ..Fall::default()
        }),
        scale: Some([ASTEROID_SCALE; 2]),
        on_end: vec![blast],
        ..Spawn::default()
    }
}

#[cfg(test)]
#[path = "supernova_parity_tests.rs"]
mod parity;

#[cfg(test)]
mod tests {
    use crate::skill_mods::Mods;
    use crate::skill_sim::{Shape, Spawn, Timeline, run, uses};
    use std::collections::HashMap;
    use std::sync::Arc;

    const NOVA: &str = "Exo_Supernova_obj";
    const BURST: &str = "Explosion_Item_obj";
    const FIREBALL: &str = "Exo_Solar_Dash_Fireball_obj";
    const TRAIL: &str = "Exo_Supernova_Connected_obj";
    const FLAME: &str = "Exo_Solar_Dash_Flame_obj";
    const ASTEROID: &str = "Exo_Asteroid_obj";
    /// The supernova's dummy, 116 px in front of the player.
    const DUMMY: [f32; 2] = [116., 0.];

    fn mods(ranks: &[(&str, u32)]) -> Mods {
        let ranks: HashMap<String, u32> = ranks
            .iter()
            .map(|(node, rank)| (format!("supernova:{node}"), *rank))
            .collect();
        Mods::of("supernova", &ranks)
    }

    fn cast_with(mods: &Mods) -> Timeline {
        run("supernova", mods, 1).expect("supernova")
    }

    fn cast(ranks: &[(&str, u32)]) -> Timeline {
        cast_with(&mods(ranks))
    }

    fn casts(ranks: &[(&str, u32)]) -> Arc<[Timeline]> {
        uses("supernova", &mods(ranks)).expect("supernova uses")
    }

    fn spawns_with(ranks: &[(&str, u32)]) -> Vec<Spawn> {
        let base = &crate::skill_sim::skill("supernova")
            .expect("supernova")
            .spawn;
        crate::skill_mods::apply("supernova", base, &mods(ranks)).into_owned()
    }

    fn nova_with(ranks: &[(&str, u32)]) -> Spawn {
        spawns_with(ranks).swap_remove(0)
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

    fn widest(timeline: &Timeline) -> f32 {
        let novas = timeline.frames.iter().flatten();
        let novas = novas.filter(|shape| shape.object.name == NOVA);
        novas.map(|shape| shape.scale[0]).fold(0., f32::max)
    }

    fn shares(timeline: &Timeline) -> Vec<f32> {
        let hits = timeline.effects.damage.iter().filter(|d| !d.frost);
        hits.map(|d| d.share).collect()
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    /// The objects the procs of `spawn` create.
    fn proc_objects(spawn: &Spawn) -> Vec<&str> {
        spawn
            .procs
            .iter()
            .map(|p| p.spawn.object.as_str())
            .collect()
    }

    fn proc_of<'a>(spawn: &'a Spawn, object: &str) -> &'a Spawn {
        let found = spawn.procs.iter().find(|p| p.spawn.object == object);
        &found.expect("a proc creating the object").spawn
    }

    #[test]
    fn the_base_nova_grows_toward_1_5_around_the_player_and_hits_the_dummy_once() {
        let base = cast(&[]);
        assert_eq!(base.hits, [4]);
        assert!((widest(&base) - 1.5).abs() < 0.01, "{}", widest(&base));
        let novas = base.frames.iter().flatten();
        assert!(
            novas
                .filter(|s| s.object.name == NOVA)
                .all(|s| s.pos == [0., 0.])
        );
    }

    #[test]
    fn fight_the_gravity_widens_the_nova_15_percent_a_rank() {
        let wide = nova_with(&[("fight_the_gravity", 5)]);
        assert_eq!(wide.grow.and_then(|grow| grow.max), Some(1.5 * 1.75));
        // Lerping toward 2.625 it is 0.80 wide by its activation, 127 px: it hits at once.
        assert_eq!(cast(&[("fight_the_gravity", 5)]).hits, [2]);
    }

    #[test]
    fn size_stats_add_their_bonus_to_the_novas_scale() {
        let mods = Mods {
            size_bonus: 0.3,
            ..mods(&[])
        };
        let nova = &crate::skill_mods::apply(
            "supernova",
            &crate::skill_sim::skill("supernova")
                .expect("supernova")
                .spawn,
            &mods,
        )[0];
        assert!(near(nova.grow.and_then(|g| g.max).unwrap_or(0.), 1.8));
    }

    #[test]
    fn chained_explosions_may_cast_a_second_nova_0_2_s_later() {
        let novas = spawns_with(&[("chained_explosions", 5)]);
        assert_eq!(novas.len(), 2);
        assert_eq!((novas[0].chance, novas[0].frame), (None, 0));
        assert_eq!(novas[1].frame, 12);
        assert!(near(novas[1].chance.unwrap_or(0.), 0.2));
        let uses = casts(&[("chained_explosions", 5)]);
        assert!(uses.iter().all(|u| u.hits == [4] || u.hits == [4, 16]));
        assert!(uses.iter().any(|u| u.hits == [4, 16]));
    }

    #[test]
    fn targeted_cosmic_event_leaves_the_nova_at_the_cursor() {
        let aimed = cast(&[("targeted_cosmic_event", 1)]);
        let novas = aimed.frames.iter().flatten();
        assert!(
            novas
                .filter(|s| s.object.name == NOVA)
                .all(|s| s.pos == [116., -30.])
        );
        assert_eq!(aimed.hits.len(), 1);
    }

    #[test]
    fn fiery_frenzy_buffs_the_players_fire_damage_for_5_s_once_a_cast() {
        let frenzy = cast(&[("fiery_frenzy", 5)]);
        assert_eq!(frenzy.effects.buffed, Some(([0, 300], 50.)));
        let novas = spawns_with(&[("fiery_frenzy", 5), ("chained_explosions", 5)]);
        assert!(novas[0].buff.is_some() && novas[1].buff.is_none());
    }

    #[test]
    fn chain_reactions_may_burst_on_the_dummy_for_15_percent_a_rank() {
        let nova = nova_with(&[("chain_reactions", 5)]);
        assert_eq!(proc_objects(&nova), [BURST]);
        assert!(near(nova.procs[0].chance, 0.15));
        let burst = &nova.procs[0].spawn;
        assert!(burst.at_enemy);
        assert_eq!((burst.scale, burst.share), (Some([0.85, 0.85]), Some(0.75)));
        assert_eq!((burst.active_from, burst.life), (Some(4), Some(24)));
        let uses = casts(&[("chain_reactions", 5)]);
        let bursts: Vec<[f32; 2]> = uses
            .iter()
            .flat_map(|u| births(u, BURST).into_iter().map(|(_, s)| s.pos))
            .collect();
        assert!(!bursts.is_empty(), "no burst in {} uses", uses.len());
        assert!(bursts.iter().all(|pos| *pos == DUMMY), "{bursts:?}");
        assert!(uses.iter().any(|u| shares(u).contains(&0.75)));
    }

    #[test]
    fn particle_outflux_sends_a_fireball_from_the_player_back_and_forth_through_the_dummy() {
        let one = cast(&[("particle_outflux", 1)]);
        let (frame, fireball) = births(&one, FIREBALL)[0];
        assert_eq!((frame, fireball.pos, fireball.angle), (5, [0., 0.], 0.));
        assert!(near(fireball.scale[0], 0.75));
        // Three, six and nine bounces; the fireball lives through only seven looks.
        assert_eq!(one.hits.len(), 1 + 3, "{:?}", one.hits);
        assert_eq!(cast(&[("particle_outflux", 2)]).hits.len(), 1 + 6);
        assert_eq!(cast(&[("particle_outflux", 3)]).hits.len(), 1 + 7);
        assert!(shares(&one).iter().all(|share| *share == 1.));
    }

    #[test]
    fn connected_by_sun_runs_a_trail_from_the_player_dropping_flames_every_40_px() {
        let nova = nova_with(&[("connected_by_sun", 3)]);
        assert_eq!(proc_objects(&nova), [TRAIL]);
        assert!(near(nova.procs[0].chance, 0.51));
        let trail = &nova.procs[0].spawn;
        assert!(trail.at_player && trail.aim && trail.ability);
        assert_eq!(
            (trail.offset, trail.speed, trail.life),
            (12., 5., Some(180))
        );
        let flame = &trail.trail.as_ref().expect("a trail").spawn;
        assert_eq!((flame.object.as_str(), flame.share), (FLAME, Some(2.5)));
        assert_eq!((flame.tick, flame.life), (30., Some(180)));
        let uses = casts(&[("connected_by_sun", 3)]);
        let lit = uses.iter().find(|u| !births(u, FLAME).is_empty());
        let lit = lit.expect("a trail in some use");
        let spots: Vec<[f32; 2]> = births(lit, FLAME).iter().map(|(_, s)| s.pos).collect();
        assert_eq!(spots.len(), 22);
        for (k, spot) in spots.iter().enumerate() {
            assert!(near(spot[0], 12. + 40. * (k + 1) as f32) && near(spot[1], 0.));
        }
        let flame_hits = shares(lit).iter().filter(|s| near(**s, 2.5)).count();
        assert!(flame_hits >= 12, "{:?}", shares(lit));
    }

    #[test]
    fn planetary_collision_drops_a_small_asteroid_on_the_dummy() {
        let nova = nova_with(&[("planetary_collision", 3)]);
        assert_eq!(proc_objects(&nova), [ASTEROID]);
        assert!(near(nova.procs[0].chance, 0.45));
        let asteroid = &nova.procs[0].spawn;
        assert!(asteroid.at_enemy && asteroid.ability);
        assert_eq!((asteroid.frame, asteroid.scale), (5, Some([0.35, 0.35])));
        let burst = &asteroid.on_end[0];
        assert_eq!(burst.object, BURST);
        assert!(near(burst.scale.expect("a scale")[0], 1.85 * 0.35));
        assert!(near(burst.share.expect("a share"), 1.6));
        let uses = casts(&[("planetary_collision", 3)]);
        let fell = uses.iter().find(|u| !births(u, ASTEROID).is_empty());
        let fell = fell.expect("an asteroid in some use");
        assert_eq!(births(fell, ASTEROID)[0].1.pos, DUMMY);
        assert_eq!(births(fell, BURST)[0].1.pos, DUMMY);
        assert!(shares(fell).iter().any(|s| near(*s, 1.6)));
    }

    #[test]
    fn what_a_proc_spawns_rolls_the_other_procs_but_never_its_own() {
        let all = [
            ("chain_reactions", 5),
            ("particle_outflux", 3),
            ("connected_by_sun", 3),
            ("planetary_collision", 3),
        ];
        let nova = nova_with(&all);
        assert_eq!(proc_objects(&nova), [BURST, FIREBALL, TRAIL, ASTEROID]);
        let burst = proc_of(&nova, BURST);
        assert_eq!(proc_objects(burst), [FIREBALL, TRAIL, ASTEROID]);
        let fireball = proc_of(&nova, FIREBALL);
        assert_eq!(proc_objects(fireball), [BURST, TRAIL, ASTEROID]);
        let flame = &proc_of(&nova, TRAIL).trail.as_ref().expect("a trail").spawn;
        assert_eq!(proc_objects(flame), [BURST, FIREBALL, ASTEROID]);
        let blast = &proc_of(&nova, ASTEROID).on_end[0];
        assert_eq!(proc_objects(blast), [BURST, FIREBALL, TRAIL]);
        assert_eq!(proc_objects(proc_of(fireball, BURST)), [TRAIL, ASTEROID]);
        let deep = proc_of(proc_of(burst, FIREBALL), TRAIL);
        let deep_flame = &deep.trail.as_ref().expect("a trail").spawn;
        assert_eq!(proc_objects(deep_flame), [ASTEROID]);
        assert!(proc_of(deep_flame, ASTEROID).on_end[0].procs.is_empty());
    }

    #[test]
    fn a_proc_childs_damage_builds_on_its_creators() {
        let nova = nova_with(&[
            ("chain_reactions", 5),
            ("particle_outflux", 1),
            ("planetary_collision", 3),
        ]);
        let burst_fireball = proc_of(proc_of(&nova, BURST), FIREBALL);
        assert_eq!(burst_fireball.share, Some(0.75));
        let fireball_asteroid = proc_of(proc_of(&nova, FIREBALL), ASTEROID);
        assert!(near(fireball_asteroid.on_end[0].share.unwrap_or(0.), 1.6));
    }

    #[test]
    fn damage_crit_and_debuff_nodes_leave_the_use_unchanged() {
        let base = cast(&[]);
        let same = cast(&[
            ("white_dwarf_star", 5),
            ("renewing_flare", 5),
            ("penetrating_blast", 5),
            ("weakening_nova", 5),
            ("blast_right_trough", 5),
            ("solar_shockwave", 3),
        ]);
        assert_eq!(same.hits, base.hits);
        assert_eq!(same.frames.len(), base.frames.len());
        assert_eq!(shares(&same), [1.]);
    }

    #[test]
    fn every_node_at_once_still_plays_bounded_uses() {
        let tree = [
            ("white_dwarf_star", 5),
            ("renewing_flare", 5),
            ("penetrating_blast", 5),
            ("chained_explosions", 5),
            ("fight_the_gravity", 5),
            ("weakening_nova", 5),
            ("chain_reactions", 5),
            ("fiery_frenzy", 5),
            ("blast_right_trough", 5),
            ("targeted_cosmic_event", 5),
            ("particle_outflux", 3),
            ("solar_shockwave", 3),
            ("connected_by_sun", 3),
            ("planetary_collision", 3),
        ];
        let uses = casts(&tree);
        for storm in uses.iter() {
            assert!(storm.hits.len() >= 8, "{:?}", storm.hits);
            assert!(storm.frames.len() < 600, "{}", storm.frames.len());
        }
    }

    #[test]
    fn the_builds_own_size_stats_reach_the_nova() {
        use hsplanner_build::BuildSnapshot;
        use hsplanner_engine::calc::stats::ComputedStats;
        let snapshot = BuildSnapshot {
            class_id: Some("exo".into()),
            active_skill_ids: vec!["supernova".into()],
            subskill_ranks: HashMap::from([("supernova:fight_the_gravity".into(), 2)]),
            ..Default::default()
        };
        let computed = ComputedStats {
            stats: HashMap::from([("area_of_effect".into(), (30., 30.))]),
            ..Default::default()
        };
        let mods = Mods::for_build("supernova", &snapshot, Some(&computed));
        assert_eq!(mods.rank("fight_the_gravity"), 2);
        assert!(mods.size_bonus > 0.1, "{}", mods.size_bonus);
        let nova = &crate::skill_mods::apply(
            "supernova",
            &crate::skill_sim::skill("supernova")
                .expect("supernova")
                .spawn,
            &mods,
        )[0];
        let max = nova.grow.and_then(|grow| grow.max).unwrap_or(0.);
        assert!(near(max, 1.5 * 1.3 + mods.size_bonus), "{max}");
    }
}
