//! Exo's Scorching Whip: TalentsExo case 344 sets the whips and Fire Starter's fireballs from the
//! subskill ranks, Exo_Scorching_Whip_obj swings (again on its echo, around the player when
//! orbital), and ProjectileCollision20Exo rolls the whip's procs on each hit; what a proc spawns
//! inherits the other procs (InheritModifiers leaves out only its own).
use super::Tree;
use crate::skill_effects::{Orbit, Proc};
use crate::skill_sim::{Grow, Spawn};

const DOUBLE: &str = "double_whip";
const PUNISHING: &str = "punishing_flames";
const ARCH: &str = "solar_arch";
const FIRE_STARTER: &str = "fire_starter";
const AID: &str = "celestial_aid";
const HARVESTER: &str = "solar_harvester";
const BLASTING: &str = "blasting_whiplash";
const FLAMING: &str = "flaming_whip";
const WHIRL: &str = "whirlwhipper";
/// Exo_Scorching_Whip_obj Step_0 L0x2f, L0x46-L0x4d: an echo restarts the swing, deadly again
/// once imageIndex passes 5, 15 frames after the restart.
const REDEADLY: u32 = 15;
/// Case 344 L0x7c and Step_0 L8-L0x28: orbital whips sit 8 px from (x, y + 6) and turn 12° a
/// frame from their activation, pointing outward.
const ORBIT: Orbit = Orbit {
    radius: 8.,
    speed: 12.,
    count: 1,
    below: 6.,
    from: 4,
    turns: true,
    ..Orbit::CIRCLE
};
/// Case 344 L0x62-L0x72 and the fireball's events: 10 px out, fanned 15° like the whips, at 0.85;
/// 8 px a frame, growing 0.05 a frame to 1, 1.5 s then shrinking away.
const FIREBALL: &str = "Exo_Solar_Dash_Fireball_obj";
const FIREBALL_OFFSET: f32 = 10.;
const FIREBALL_SCALE: f32 = 0.85;
const FIREBALL_SPEED: f32 = 8.;
const FIREBALL_GROWTH: f32 = 0.05;
const FIREBALL_LIFE: u32 = 90;
const FIREBALL_FADE: u32 = 20;
/// ProjectileCollision20Exo 848 and Exo_Solar_Burst_obj: a 0.75 burst on the enemy hit; Other_7
/// clears its hit list at the end of each 24-frame loop (a 23-frame tick) for 11 loops.
const BURST: &str = "Exo_Solar_Burst_obj";
const BURST_SCALE: f32 = 0.75;
const BURST_TICK: f32 = 23.;
const BURST_LIFE: u32 = 264;
/// 849: a whip 10 px from the enemy hit toward its nearest enemy (a lone dummy is its own, at 0°).
const AID_OFFSET: f32 = 10.;
/// 850: an Explosion_Item_obj of 0.8 x (1 + area) on the enemy hit, without a roll; deadly from
/// its 4-frame alarm for its 9-frame explosion at 30 fps and 0.75 image speed.
const BLAST: &str = "Explosion_Item_obj";
const BLAST_SCALE: f32 = 0.8;
const BLAST_ACTIVE: u32 = 4;
const BLAST_LIFE: u32 = 24;
/// 851: Exo_Solar_Dash_Flame_obj on the enemy hit, 3 s ticking every 0.5 s, then shrinking 0.075
/// a frame; its maxScale 0.75 only stops growth, so it keeps its scale of 1.
const FLAME: &str = "Exo_Solar_Dash_Flame_obj";
const FLAME_TICK: f32 = 30.;
const FLAME_LIFE: u32 = 180;
const FLAME_FADE: u32 = 14;
const FLAME_SHRINK: f32 = 0.075;

/// The whip's on-hit procs, in the order case 344 fills projEffect 824-827.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Burst,
    Aid,
    Blast,
    Flame,
}

const KINDS: [Kind; 4] = [Kind::Burst, Kind::Aid, Kind::Blast, Kind::Flame];

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    spawns.iter().flat_map(|spawn| cast(spawn, tree)).collect()
}

/// One cast: the whips, Double Whip's echo of them and Fire Starter's fireballs.
fn cast(base: &Spawn, tree: &Tree) -> Vec<Spawn> {
    let template = whips(base, tree);
    let hands = Cast {
        tree,
        whip: &template,
    };
    let share = template.share.unwrap_or(1.);
    let whips = Spawn {
        procs: hands.procs(&KINDS, share, true),
        ..template.clone()
    };
    let echo = echo(&whips, tree);
    let fireballs = hands.fireballs(share);
    [Some(whips), echo, fireballs]
        .into_iter()
        .flatten()
        .collect()
}

/// Case 344 L0x46-L0x56: 1 + Solar Harvester's + Whirlwhipper's whips, fanned 15° or circling the
/// player with Whirlwhipper, each dealing both nodes' share of the hit.
fn whips(base: &Spawn, tree: &Tree) -> Spawn {
    let count = tree.own(HARVESTER, "projectile_count") + tree.own(WHIRL, "projectile_count");
    let more = count.round() as u32;
    let orbit = tree.learned(WHIRL).then_some(Orbit {
        count: 1 + more,
        ..ORBIT
    });
    Spawn {
        extra: if orbit.is_some() { 0 } else { more },
        scale: Some(whip_scale(base, tree)),
        orbit,
        share: Some(part(tree, HARVESTER) * part(tree, WHIRL)),
        ..base.clone()
    }
}

/// A transformation's share of the hit, or the whole hit when it is not learned.
fn part(tree: &Tree, node: &str) -> f32 {
    if tree.learned(node) {
        tree.own(node, "of_total_damage") / 100.
    } else {
        1.
    }
}

/// L0x57, L0x83-L0x84: image_xscale = scale + 1 and image_yscale = scale, scale = 1 + Solar
/// Arch's area (Blasting Whiplash's area grows only its explosions), then the build's size bonus.
fn whip_scale(base: &Spawn, tree: &Tree) -> [f32; 2] {
    let [x, y] = base.scale.unwrap_or([2., 1.]);
    let scale = y * (1. + tree.own(ARCH, "area_of_effect") / 100.);
    [scale + x - y, scale].map(|s| s + tree.mods.size_bonus)
}

/// Double Whip, rolled once a cast (L0x2f-L0x30): every whip swings again with a cleared hit
/// list where it is, made as early as the whip wakes so its swing starts when the first ends.
fn echo(whips: &Spawn, tree: &Tree) -> Option<Spawn> {
    let (chance, _) = tree.proc(DOUBLE, "of_total_damage")?;
    let wake = whips.active_from.unwrap_or(0);
    let life = whips.life.unwrap_or(0);
    Some(Spawn {
        chance: Some(chance),
        frame: whips.frame + life.saturating_sub(wake),
        active_from: Some(wake + REDEADLY),
        orbit: whips.orbit.map(|orbit| Orbit { from: 0, ..orbit }),
        ..whips.clone()
    })
}

/// What the cast hands its damage objects: the tree and the whip Celestial Aid copies.
struct Cast<'a> {
    tree: &'a Tree<'a>,
    whip: &'a Spawn,
}

impl Cast<'_> {
    /// Fire Starter, rolled once a cast (L0x3b-L0x3c): its count of fireballs dealing the whips'
    /// damage; they carry only projEffect 824 (L0x70), Punishing Flames.
    fn fireballs(&self, share: f32) -> Option<Spawn> {
        let (chance, count) = self.tree.proc(FIRE_STARTER, "projectile_count")?;
        Some(Spawn {
            object: FIREBALL.into(),
            chance: Some(chance),
            extra: (count.round() as u32).saturating_sub(1),
            offset: FIREBALL_OFFSET,
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
            share: Some(share),
            procs: self.procs(&[Kind::Burst], share, false),
            ..Spawn::default()
        })
    }

    /// The procs an object dealing `share` of the hit rolls on each hit; only a whip rolls
    /// Celestial Aid (L0x171 checks object_index).
    fn procs(&self, kinds: &[Kind], share: f32, whip: bool) -> Vec<Proc> {
        let rolled = kinds.iter().filter(|&&kind| whip || kind != Kind::Aid);
        rolled
            .filter_map(|&kind| {
                let rest: Vec<Kind> = kinds.iter().copied().filter(|k| *k != kind).collect();
                self.proc(kind, &rest, share)
            })
            .collect()
    }

    /// One learned proc and what it creates, carrying the `rest` along.
    fn proc(&self, kind: Kind, rest: &[Kind], share: f32) -> Option<Proc> {
        let tree = self.tree;
        let (chance, spawn) = match kind {
            Kind::Burst => {
                let (chance, part) = tree.proc(PUNISHING, "of_total_damage")?;
                (chance, self.burst(share * part / 100., rest))
            }
            Kind::Aid => {
                let (chance, more) = tree.proc(AID, "fire_skill_damage")?;
                (chance, self.aid(share * (1. + more / 100.), rest))
            }
            Kind::Blast => {
                let more = tree
                    .learned(BLASTING)
                    .then(|| tree.own(BLASTING, "fire_skill_damage"))?;
                (1., self.blast(share * (1. + more / 100.), rest))
            }
            Kind::Flame => {
                let (chance, part) = tree.proc(FLAMING, "of_total_damage")?;
                (chance, self.flame(share * part / 100., rest))
            }
        };
        Some(Proc {
            chance,
            spawn,
            pull: None,
            once: false,
        })
    }

    /// Punishing Flames: floor(damage x punishingFlames) a loop (L0x19).
    fn burst(&self, share: f32, rest: &[Kind]) -> Spawn {
        Spawn {
            object: BURST.into(),
            at_enemy: true,
            scale: Some([BURST_SCALE; 2]),
            tick: BURST_TICK,
            life: Some(BURST_LIFE),
            share: Some(share),
            procs: self.procs(rest, share, false),
            ..Spawn::default()
        }
    }

    /// Celestial Aid: a whip of the hitting whip's size dealing floor(damage x (1 + aid)) that
    /// ignores the player (L0x17b-L0x184); the echo it copies is left out.
    fn aid(&self, share: f32, rest: &[Kind]) -> Spawn {
        let whip = self.whip;
        Spawn {
            object: whip.object.clone(),
            at_enemy: true,
            offset: AID_OFFSET,
            scale: whip.scale,
            life: whip.life,
            active_from: whip.active_from,
            share: Some(share),
            procs: self.procs(rest, share, true),
            ..Spawn::default()
        }
    }

    /// Blasting Whiplash: floor(damage x (1 + blasting)) on every hit (L0x194-L0x1a1).
    fn blast(&self, share: f32, rest: &[Kind]) -> Spawn {
        let area = 1. + self.tree.own(BLASTING, "area_of_effect") / 100.;
        Spawn {
            object: BLAST.into(),
            at_enemy: true,
            scale: Some([BLAST_SCALE * area; 2]),
            active_from: Some(BLAST_ACTIVE),
            life: Some(BLAST_LIFE),
            share: Some(share),
            procs: self.procs(rest, share, false),
            ..Spawn::default()
        }
    }

    /// Flaming Whip: floor(damage x flamingWhip) a tick (L0x15d-L0x163); its slow shows nothing.
    fn flame(&self, share: f32, rest: &[Kind]) -> Spawn {
        Spawn {
            object: FLAME.into(),
            at_enemy: true,
            tick: FLAME_TICK,
            life: Some(FLAME_LIFE),
            fade: FLAME_FADE,
            shrink: FLAME_SHRINK,
            share: Some(share),
            procs: self.procs(rest, share, false),
            ..Spawn::default()
        }
    }
}

#[cfg(test)]
#[path = "scorching_whip_parity_tests.rs"]
mod parity;

#[cfg(test)]
mod tests {
    use crate::skill_mods::Mods;
    use crate::skill_sim::{Shape, Spawn, Timeline, run, uses};
    use std::collections::HashMap;
    use std::sync::Arc;

    const WHIP: &str = "Exo_Scorching_Whip_obj";
    const FIREBALL: &str = "Exo_Solar_Dash_Fireball_obj";
    const BURST: &str = "Exo_Solar_Burst_obj";
    const BLAST: &str = "Explosion_Item_obj";
    const FLAME: &str = "Exo_Solar_Dash_Flame_obj";
    /// The whip's dummy, 160 px in front of the player.
    const DUMMY: [f32; 2] = [160., 0.];

    fn mods(ranks: &[(&str, u32)]) -> Mods {
        let ranks: HashMap<String, u32> = ranks
            .iter()
            .map(|(node, rank)| (format!("scorching_whip:{node}"), *rank))
            .collect();
        Mods::of("scorching_whip", &ranks)
    }

    fn cast_with(mods: &Mods) -> Timeline {
        run("scorching_whip", mods, 1).expect("scorching whip")
    }

    fn cast(ranks: &[(&str, u32)]) -> Timeline {
        cast_with(&mods(ranks))
    }

    fn casts(ranks: &[(&str, u32)]) -> Arc<[Timeline]> {
        uses("scorching_whip", &mods(ranks)).expect("scorching whip uses")
    }

    fn spawns_with(mods: &Mods) -> Vec<Spawn> {
        let base = &crate::skill_sim::skill("scorching_whip")
            .expect("scorching whip")
            .spawn;
        crate::skill_mods::apply("scorching_whip", base, mods).into_owned()
    }

    fn whip_with(ranks: &[(&str, u32)]) -> Spawn {
        spawns_with(&mods(ranks)).swap_remove(0)
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

    fn shapes<'a>(timeline: &'a Timeline, object: &'a str) -> impl Iterator<Item = &'a Shape> {
        let all = timeline.frames.iter().flatten();
        all.filter(move |shape| shape.object.name == object)
    }

    fn shares(timeline: &Timeline) -> Vec<f32> {
        let hits = timeline.effects.damage.iter().filter(|d| !d.frost);
        hits.map(|d| d.share).collect()
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn proc_objects(spawn: &Spawn) -> Vec<&str> {
        let procs = spawn.procs.iter();
        procs.map(|p| p.spawn.object.as_str()).collect()
    }

    fn proc_of<'a>(spawn: &'a Spawn, object: &str) -> &'a Spawn {
        let found = spawn.procs.iter().find(|p| p.spawn.object == object);
        &found.expect("a proc creating the object").spawn
    }

    /// The first use whose timeline shows `object`.
    fn showing<'a>(uses: &'a [Timeline], object: &str) -> &'a Timeline {
        let found = uses.iter().find(|u| !births(u, object).is_empty());
        found.unwrap_or_else(|| panic!("no {object} in {} uses", uses.len()))
    }

    #[test]
    fn the_base_whip_stands_10_px_out_and_hits_the_dummy_once() {
        let base = cast(&[]);
        assert_eq!(base.hits, [3]);
        assert_eq!(shares(&base), [1.]);
        for shape in shapes(&base, WHIP) {
            assert_eq!(
                (shape.pos, shape.scale, shape.angle),
                ([10., 0.], [2., 1.], 0.)
            );
        }
    }

    #[test]
    fn double_whip_may_swing_every_whip_again_with_a_cleared_hit_list() {
        let spawns = spawns_with(&mods(&[("double_whip", 5)]));
        assert_eq!(spawns.len(), 2);
        let echo = &spawns[1];
        assert!(near(echo.chance.unwrap_or(0.), 0.2));
        assert_eq!(
            (echo.frame, echo.active_from, echo.life),
            (34, Some(18), Some(37))
        );
        let uses = casts(&[("double_whip", 5)]);
        assert!(uses.iter().all(|u| u.hits == [3] || u.hits == [3, 52]));
        assert!(uses.iter().any(|u| u.hits == [3, 52]));
    }

    #[test]
    fn solar_arch_lengthens_and_widens_the_whip_10_percent_a_rank() {
        assert_eq!(whip_with(&[("solar_arch", 5)]).scale, Some([2.5, 1.5]));
    }

    #[test]
    fn size_stats_add_their_bonus_to_both_whip_axes() {
        let mods = Mods {
            size_bonus: 0.3,
            ..mods(&[("solar_arch", 2)])
        };
        let scale = spawns_with(&mods)[0].scale.expect("a scale");
        assert!(near(scale[0], 2.5) && near(scale[1], 1.5), "{scale:?}");
    }

    #[test]
    fn solar_harvester_fans_more_whips_each_dealing_its_share() {
        let two = cast(&[("solar_harvester", 1)]);
        let angles: Vec<f32> = births(&two, WHIP).iter().map(|(_, s)| s.angle).collect();
        assert_eq!(angles, [-7.5, 7.5]);
        assert_eq!(two.hits, [3, 3]);
        assert!(shares(&two).iter().all(|s| near(*s, 0.32)));
        // Of three whips 15° apart only the middle one reaches the dummy at 160 px.
        assert_eq!(cast(&[("solar_harvester", 2)]).hits.len(), 1);
        let four = cast(&[("solar_harvester", 3)]);
        assert_eq!((births(&four, WHIP).len(), four.hits.len()), (4, 2));
        assert!(shares(&four).iter().all(|s| near(*s, 0.96)));
    }

    #[test]
    fn whirlwhipper_circles_whips_around_the_player_from_their_activation() {
        let whip = whip_with(&[("whirlwhipper", 1)]);
        let orbit = whip.orbit.expect("an orbit");
        assert_eq!((orbit.count, orbit.from, orbit.turns), (3, 4, true));
        assert_eq!((orbit.radius, orbit.speed, orbit.below), (8., 12., 6.));
        let spin = cast(&[("whirlwhipper", 1)]);
        let first: Vec<&Shape> = spin.frames[3].iter().collect();
        for (shape, angle) in first.iter().zip([0f32, 120., 240.]) {
            let (sin, cos) = angle.to_radians().sin_cos();
            assert!(near(shape.pos[0], 10. * cos) && near(shape.pos[1], -10. * sin));
            assert_eq!(shape.angle, angle);
        }
        let turned = spin.frames[4][0];
        let (sin, cos) = 12f32.to_radians().sin_cos();
        assert!(near(turned.pos[0], 8. * cos) && near(turned.pos[1], 6. - 8. * sin));
        assert_eq!(turned.angle, 12.);
        assert_eq!(spin.hits.len(), 3, "{:?}", spin.hits);
        assert!(shares(&spin).iter().all(|s| near(*s, 0.28)));
        assert_eq!(cast(&[("whirlwhipper", 3)]).hits.len(), 5);
    }

    #[test]
    fn harvester_and_whirlwhipper_share_the_count_and_multiply_their_shares() {
        let whip = whip_with(&[("solar_harvester", 3), ("whirlwhipper", 3)]);
        assert_eq!(whip.orbit.map(|orbit| orbit.count), Some(8));
        assert!(near(whip.share.unwrap_or(0.), 0.96 * 0.84));
    }

    #[test]
    fn double_whips_echo_keeps_circling_where_the_whips_are() {
        let spawns = spawns_with(&mods(&[("whirlwhipper", 1), ("double_whip", 1)]));
        let echo = spawns[1].orbit.expect("a circling echo");
        assert_eq!((echo.count, echo.from), (3, 0));
        let uses = casts(&[("whirlwhipper", 3), ("double_whip", 5)]);
        let twice = uses.iter().find(|u| u.frames.len() > 40);
        let twice = twice.expect("an echo in some use");
        let (first, echoes): (Vec<&Shape>, Vec<&Shape>) =
            twice.frames[36].iter().partition(|s| s.age == 37);
        assert_eq!((first.len(), echoes.len()), (5, 5));
        for (whip, echo) in first.iter().zip(&echoes) {
            assert!(near(whip.pos[0], echo.pos[0]) && near(whip.pos[1], echo.pos[1]));
            assert!(near(
                whip.angle.rem_euclid(360.),
                echo.angle.rem_euclid(360.)
            ));
        }
        // Three of the five whips sweep past the dummy while the echo can hit.
        assert_eq!(twice.hits.len(), 5 + 3, "{:?}", twice.hits);
    }

    #[test]
    fn fire_starter_may_throw_its_rank_of_fireballs_fanned_like_the_whips() {
        let spawns = spawns_with(&mods(&[("fire_starter", 5)]));
        let fireball = spawns.iter().find(|s| s.object == FIREBALL);
        let fireball = fireball.expect("a fireball spawn");
        assert!(near(fireball.chance.unwrap_or(0.), 0.25));
        assert_eq!(
            (fireball.extra, fireball.offset, fireball.speed),
            (4, 10., 8.)
        );
        assert_eq!((fireball.scale, fireball.life), (Some([0.85; 2]), Some(90)));
        let uses = casts(&[("fire_starter", 5)]);
        let thrown = showing(&uses, FIREBALL);
        let angles: Vec<f32> = births(thrown, FIREBALL)
            .iter()
            .map(|(_, s)| s.angle)
            .collect();
        assert_eq!(angles, [-30., -15., 0., 15., 30.]);
        // The ±15° fireballs pass 41.4 px from the dummy, 0.4 px beyond their reach.
        assert_eq!(thrown.hits, [3, 13]);
    }

    #[test]
    fn fireballs_deal_the_whips_damage_and_roll_only_punishing_flames() {
        let all = [
            ("fire_starter", 1),
            ("solar_harvester", 1),
            ("punishing_flames", 1),
            ("celestial_aid", 1),
            ("blasting_whiplash", 1),
            ("flaming_whip", 1),
        ];
        let spawns = spawns_with(&mods(&all));
        let fireball = spawns.iter().find(|s| s.object == FIREBALL);
        let fireball = fireball.expect("a fireball spawn");
        assert!(near(fireball.share.unwrap_or(0.), 0.32));
        assert_eq!(proc_objects(fireball), [BURST]);
        assert!(proc_of(fireball, BURST).procs.is_empty());
    }

    #[test]
    fn punishing_flames_may_burst_on_the_dummy_eleven_times() {
        let whip = whip_with(&[("punishing_flames", 5)]);
        assert_eq!(proc_objects(&whip), [BURST]);
        assert!(near(whip.procs[0].chance, 0.15));
        let burst = &whip.procs[0].spawn;
        assert!(burst.at_enemy);
        assert_eq!((burst.scale, burst.share), (Some([0.75; 2]), Some(1.25)));
        assert_eq!((burst.life, burst.tick), (Some(264), 23.));
        let uses = casts(&[("punishing_flames", 5), ("whirlwhipper", 3)]);
        let burnt = showing(&uses, BURST);
        assert_eq!(births(burnt, BURST)[0].1.pos, DUMMY);
        let pulses = shares(burnt)
            .iter()
            .filter(|s| near(**s, 0.84 * 1.25))
            .count();
        assert!(pulses > 0 && pulses % 11 == 0, "{:?}", shares(burnt));
    }

    #[test]
    fn celestial_aid_may_swing_a_whip_from_the_dummy_that_hits_it() {
        let whip = whip_with(&[("celestial_aid", 5), ("solar_arch", 5)]);
        let aid = proc_of(&whip, WHIP);
        assert!(near(whip.procs[0].chance, 0.25));
        assert!(aid.at_enemy && aid.orbit.is_none());
        assert_eq!((aid.offset, aid.share), (10., Some(2.)));
        assert_eq!(
            (aid.scale, aid.life, aid.active_from),
            (whip.scale, Some(37), Some(3))
        );
        assert!(aid.procs.is_empty());
        let uses = casts(&[("celestial_aid", 5), ("whirlwhipper", 3)]);
        let aided = uses.iter().find(|u| births(u, WHIP).len() > 5);
        let aided = aided.expect("a whip from the dummy in some use");
        let (_, from) = births(aided, WHIP)[5];
        assert_eq!((from.pos, from.angle), ([170., 0.], 0.));
        assert!(shares(aided).iter().any(|s| near(*s, 0.84 * 2.)));
    }

    #[test]
    fn blasting_whiplash_explodes_on_every_whip_hit() {
        let whip = whip_with(&[("blasting_whiplash", 3)]);
        assert_eq!(whip.scale, Some([2., 1.]));
        assert_eq!(proc_objects(&whip), [BLAST]);
        assert_eq!(whip.procs[0].chance, 1.);
        let blast = &whip.procs[0].spawn;
        assert!(blast.at_enemy && near(blast.share.unwrap_or(0.), 1.75));
        assert!(near(blast.scale.expect("a scale")[0], 0.8 * 2.05));
        assert_eq!((blast.active_from, blast.life), (Some(4), Some(24)));
        let blasted = cast(&[("blasting_whiplash", 3)]);
        assert_eq!(births(&blasted, BLAST)[0].1.pos, DUMMY);
        assert_eq!(shares(&blasted), [1., 1.75]);
    }

    #[test]
    fn flaming_whip_may_leave_a_flame_on_the_dummy() {
        let whip = whip_with(&[("flaming_whip", 3)]);
        assert_eq!(proc_objects(&whip), [FLAME]);
        assert!(near(whip.procs[0].chance, 0.54));
        let flame = &whip.procs[0].spawn;
        assert!(flame.at_enemy && near(flame.share.unwrap_or(0.), 1.8));
        assert_eq!((flame.tick, flame.life, flame.fade), (30., Some(180), 14));
        let uses = casts(&[("flaming_whip", 3)]);
        let lit = showing(&uses, FLAME);
        assert_eq!(births(lit, FLAME)[0].1.pos, DUMMY);
        assert_eq!(shares(lit).iter().filter(|s| near(**s, 1.8)).count(), 7);
    }

    #[test]
    fn what_a_proc_spawns_rolls_the_other_procs_and_only_a_whip_rolls_celestial_aid() {
        let all = [
            ("punishing_flames", 1),
            ("celestial_aid", 1),
            ("blasting_whiplash", 1),
            ("flaming_whip", 1),
        ];
        let whip = whip_with(&all);
        assert_eq!(proc_objects(&whip), [BURST, WHIP, BLAST, FLAME]);
        assert_eq!(proc_objects(proc_of(&whip, BURST)), [BLAST, FLAME]);
        assert_eq!(proc_objects(proc_of(&whip, WHIP)), [BURST, BLAST, FLAME]);
        assert_eq!(proc_objects(proc_of(&whip, BLAST)), [BURST, FLAME]);
        assert_eq!(proc_objects(proc_of(&whip, FLAME)), [BURST, BLAST]);
        let deep = proc_of(proc_of(&whip, BLAST), BURST);
        assert_eq!(proc_objects(deep), [FLAME]);
        assert!(proc_of(deep, FLAME).procs.is_empty());
        let aid = proc_of(&whip, WHIP);
        assert!(near(proc_of(aid, BLAST).share.unwrap_or(0.), 1.2 * 1.25));
    }

    #[test]
    fn damage_crit_speed_and_cost_nodes_leave_the_use_unchanged() {
        let base = cast(&[]);
        let same = cast(&[
            ("celestial_burn", 5),
            ("overheating", 5),
            ("sun_gods_chosen", 5),
            ("critical_temperature", 5),
            ("scorching_snap", 5),
        ]);
        assert_eq!(same.hits, base.hits);
        assert_eq!(same.frames.len(), base.frames.len());
        assert_eq!(shares(&same), [1.]);
    }

    #[test]
    fn every_node_at_once_still_plays_bounded_uses() {
        let tree = [
            ("double_whip", 5),
            ("celestial_burn", 5),
            ("overheating", 5),
            ("punishing_flames", 5),
            ("sun_gods_chosen", 5),
            ("solar_arch", 5),
            ("critical_temperature", 5),
            ("fire_starter", 5),
            ("scorching_snap", 5),
            ("celestial_aid", 5),
            ("solar_harvester", 3),
            ("blasting_whiplash", 3),
            ("flaming_whip", 3),
            ("whirlwhipper", 3),
        ];
        for storm in casts(&tree).iter() {
            assert!(storm.hits.len() >= 8, "{:?}", storm.hits);
            assert!(storm.frames.len() < 900, "{}", storm.frames.len());
        }
    }

    #[test]
    fn the_builds_own_size_stats_reach_the_whip() {
        use hsplanner_build::BuildSnapshot;
        use hsplanner_engine::calc::stats::ComputedStats;
        let snapshot = BuildSnapshot {
            class_id: Some("exo".into()),
            active_skill_ids: vec!["scorching_whip".into()],
            subskill_ranks: HashMap::from([("scorching_whip:solar_arch".into(), 2)]),
            ..Default::default()
        };
        // A melee spell without the Area of Effect tag takes half the melee range, not the AoE.
        let computed = ComputedStats {
            stats: HashMap::from([
                ("melee_range".into(), (40., 40.)),
                ("area_of_effect".into(), (30., 30.)),
            ]),
            ..Default::default()
        };
        let mods = Mods::for_build("scorching_whip", &snapshot, Some(&computed));
        assert_eq!(mods.rank("solar_arch"), 2);
        assert!(near(mods.size_bonus, 0.2), "{}", mods.size_bonus);
        let scale = spawns_with(&mods)[0].scale.expect("a scale");
        let bonus = mods.size_bonus;
        assert!(
            near(scale[0], 2.2 + bonus) && near(scale[1], 1.2 + bonus),
            "{scale:?}"
        );
    }
}
