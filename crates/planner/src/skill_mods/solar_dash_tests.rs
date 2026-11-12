use crate::skill_mods::Mods;
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Shape, Spawn, Timeline, run, uses};
use std::sync::Arc;

const MOVER: &str = "Draw_Player_Buff_obj";
const FLAME: &str = "Exo_Solar_Dash_Flame_obj";
const AURA: &str = "Pyromancer_Scorching_Aura_obj";
const BLAST: &str = "Explosion_Item_obj";
const BALL: &str = "Exo_Solar_Dash_Fireball_obj";
const ORB: &str = "Exo_Solar_Dash_Solar_Orb_obj";
const DUMMY: [f32; 2] = [160., 0.];
/// The moves after which the base dash drops its flames: 48 px of its path past the last, on the
/// way in, then every 4 moves round the loop.
const DROPS: [usize; 8] = [5, 10, 14, 18, 22, 26, 30, 34];
/// Where the base dash passes the cursor on the dummy, and where it is back over it a loop later.
const PASSED: f32 = 162.16;

fn mods(ranks: &[(&str, u32)]) -> Mods {
    parity::mods("solar_dash", ranks)
}

fn dash(ranks: &[(&str, u32)]) -> Timeline {
    run("solar_dash", &mods(ranks), 1).expect("solar dash")
}

fn dashes(ranks: &[(&str, u32)]) -> Arc<[Timeline]> {
    uses("solar_dash", &mods(ranks)).expect("solar dash uses")
}

fn spawns(ranks: &[(&str, u32)]) -> Vec<Spawn> {
    let base = &crate::skill_sim::skill("solar_dash")
        .expect("solar dash")
        .spawn;
    crate::skill_mods::apply("solar_dash", base, &mods(ranks)).into_owned()
}

fn mover(ranks: &[(&str, u32)]) -> Spawn {
    spawns(ranks).swap_remove(0)
}

fn shapes<'a>(timeline: &'a Timeline, object: &'a str) -> impl Iterator<Item = &'a Shape> {
    let all = timeline.frames.iter().flatten();
    all.filter(move |shape| shape.object.name == object && shape.scale[0] > 0.)
}

/// The first drawn shape of every seen instance of `object`, with the frame it is drawn in.
fn births<'a>(timeline: &'a Timeline, object: &'a str) -> Vec<(usize, &'a Shape)> {
    timeline
        .frames
        .iter()
        .enumerate()
        .flat_map(|(frame, shapes)| shapes.iter().map(move |shape| (frame, shape)))
        .filter(|(_, shape)| shape.object.name == object && shape.age == 1)
        .filter(|(_, shape)| shape.scale[0] > 0.)
        .collect()
}

fn near(a: f32, b: f32, within: f32) -> bool {
    (a - b).abs() < within
}

/// Whether two headings in degrees point the same way.
fn heads(a: f32, b: f32) -> bool {
    near((a - b + 180.).rem_euclid(360.), 180., 0.01)
}

fn shares(timeline: &Timeline) -> Vec<f32> {
    timeline.effects.damage.iter().map(|d| d.share).collect()
}

fn last(timeline: &Timeline, object: &str) -> [f32; 2] {
    shapes(timeline, object).last().expect("an object").pos
}

/// The mover's path from where it starts and its heading: before its first move, then after each.
fn path(ranks: &[(&str, u32)]) -> Vec<([f32; 2], f32)> {
    crate::skill_mods::charge::path(&mover(ranks))
}

/// The mover's heading when it stops, as the end block reads playerEffect[154].
fn final_heading(ranks: &[(&str, u32)]) -> f32 {
    let life = mover(ranks).life.expect("a stop") as usize;
    path(ranks)[life - 1].1
}

#[test]
fn the_dash_ramps_through_the_dummy_and_loops_once_round_the_cursor() {
    let base = dash(&[]);
    let moves: Vec<[f32; 2]> = shapes(&base, MOVER).map(|s| s.pos).collect();
    assert_eq!(moves.len(), 39);
    for (at, x) in moves.iter().zip([0., 10., 20.225]) {
        assert!(near(at[0], x, 1e-4) && at[1] == 6., "{moves:?}");
    }
    // Past the cursor it turns 15° a frame: a 99 px circle under it, back over it 24 moves on.
    for at in [moves[14], moves[38]] {
        assert!(near(at[0], PASSED, 0.05) && near(at[1], 6., 0.05), "{at:?}");
    }
    let lowest = moves.iter().map(|at| at[1]).fold(0., f32::max);
    assert!(near(lowest, 6. + 98.745, 0.05), "{lowest}");
    assert_eq!(mover(&[]).life, Some(39));
}

#[test]
fn the_dash_drops_a_flame_every_48_px_of_its_loop() {
    let base = dash(&[]);
    let moves: Vec<[f32; 2]> = shapes(&base, MOVER).map(|s| s.pos).collect();
    let flames = births(&base, FLAME);
    assert_eq!(flames.len(), DROPS.len());
    for ((frame, flame), m) in flames.iter().zip(DROPS) {
        assert_eq!(*frame, m + 1);
        assert!(near(flame.pos[0], moves[m][0], 1e-3) && near(flame.pos[1], moves[m][1], 1e-3));
    }
    assert!(shares(&base).iter().all(|share| *share == 1.));
}

#[test]
fn the_dash_ends_in_a_scorching_aura_over_the_dummy() {
    let base = dash(&[]);
    let auras = births(&base, AURA);
    assert_eq!(auras.len(), 1);
    let (born, aura) = auras[0];
    assert!(
        near(aura.pos[0], PASSED, 0.05) && near(aura.pos[1], 0., 0.05),
        "{:?}",
        aura.pos
    );
    assert!(near(aura.scale[0], 1.15, 1e-4));
    assert!(base.hits.contains(&(born as u32 + 2)), "{:?}", base.hits);
}

#[test]
fn speed_nodes_change_the_dash_and_its_loop_round_the_cursor() {
    let fast = mover(&[("interstellar_traveller", 5)]);
    assert_eq!(fast.speed, 22.);
    let ramp = fast.accel.expect("a ramp");
    assert!(near(ramp.from, 16.75, 1e-4));
    let slow = mover(&[("cooldown_lap", 5)]);
    assert_eq!(slow.speed, 4.);
    assert!(fast.life < mover(&[]).life && slow.life > mover(&[]).life);
    let star = mover(&[("shooting_star", 2)]);
    assert!(near(star.speed, 12. * 1.3 + 1., 1e-4));
    // Whatever the speed, the loop brings the player back by the cursor.
    for ranks in [[("interstellar_traveller", 5)], [("cooldown_lap", 5)]] {
        let end = last(&dash(&ranks), MOVER);
        assert!((end[0] - DUMMY[0]).hypot(end[1] - 6.) < 40., "{end:?}");
    }
}

#[test]
fn intense_flames_shortens_the_flames_tick_down_to_0_1_s() {
    let tick = |rank: u32| spawns(&[("intense_flames", rank)])[1].tick;
    assert_eq!([tick(0), tick(1), tick(5)], [30., 27., 15.]);
    assert!(dash(&[("intense_flames", 5)]).hits.len() > dash(&[]).hits.len());
}

#[test]
fn solar_burst_explodes_two_and_a_half_times_as_big_where_the_dash_stops() {
    let burst = dash(&[("solar_burst", 4)]);
    let (_, blast) = births(&burst, BLAST)[0];
    assert!(near(blast.pos[0], PASSED, 0.1) && near(blast.pos[1], 0., 0.1));
    assert_eq!(blast.scale, [2.5, 2.5]);
    let spawn = mover(&[("solar_burst", 4)]);
    let blast = spawn
        .on_end
        .iter()
        .find(|s| s.object == BLAST)
        .expect("a blast");
    assert_eq!((blast.share, blast.active_from), (Some(2.), Some(4)));
}

#[test]
fn shooting_star_rings_two_fireballs_a_rank_out_of_the_dash_end() {
    let star = dash(&[("shooting_star", 3)]);
    let balls = births(&star, BALL);
    assert_eq!(balls.len(), 6);
    let end = last(&star, MOVER);
    // End case L0x4e3: 360 / n × i past the heading the charge stops with.
    let heading = final_heading(&[("shooting_star", 3)]);
    for (i, (_, ball)) in balls.iter().enumerate() {
        let turn = heading + 60. * i as f32;
        let (sin, cos) = turn.to_radians().sin_cos();
        let out = [end[0] + 8. * cos, end[1] - 6. - 8. * sin];
        assert!(near(ball.pos[0], out[0], 0.01) && near(ball.pos[1], out[1], 0.01));
        assert!(heads(ball.angle, turn), "{}", ball.angle);
        assert!(near(ball.scale[0], 0.1, 1e-4));
    }
    // Back over the dummy, the ring's balls within 8 + 16 px of it cross it: three of the six.
    let crossing = shares(&star)
        .iter()
        .filter(|s| near(**s, 0.75, 1e-4))
        .count();
    assert_eq!(crossing, 3, "{:?}", shares(&star));
}

#[test]
fn bursting_flames_may_burst_each_flame_into_a_ring_of_fireballs() {
    let flame = &spawns(&[("bursting_flames", 2)])[1];
    assert_eq!(flame.object, FLAME);
    let burst = &flame.on_end[0];
    assert!(near(burst.chance.unwrap_or(0.), 0.3, 1e-4));
    assert_eq!(burst.on_end.len(), 10);
    let all = dashes(&[("bursting_flames", 3)]);
    let rings: Vec<usize> = all.iter().map(|u| births(u, BALL).len()).collect();
    assert!(rings.iter().all(|n| n % 15 == 0), "{rings:?}");
    assert!(rings.iter().any(|n| *n > 0) && rings.iter().any(|n| *n < 8 * 15));
}

#[test]
fn solar_carnage_throws_two_fireballs_back_at_each_flame_ever_wider() {
    let carnage = dash(&[("solar_carnage", 1)]);
    let balls = births(&carnage, BALL);
    let flames = births(&carnage, FLAME);
    assert_eq!(balls.len(), 2 * flames.len());
    let path = path(&[("solar_carnage", 1)]);
    for (k, pair) in balls.chunks(2).enumerate().take(5) {
        let spread = 16. * k as f32;
        let heading = path[DROPS[k]].1;
        for ((frame, ball), turn) in pair.iter().zip([180. + spread, 180. - spread]) {
            assert!(heads(ball.angle, heading + turn), "{}", ball.angle);
            assert_eq!(*frame, flames[k].0);
            // From the player above the flame, 8 px out along its throw.
            let (sin, cos) = (heading + turn).to_radians().sin_cos();
            let from = [
                flames[k].1.pos[0] + 8. * cos,
                flames[k].1.pos[1] - 6. - 8. * sin,
            ];
            assert!(near(ball.pos[0], from[0], 0.01) && near(ball.pos[1], from[1], 0.01));
        }
    }
    assert!(
        carnage.hits.len() > dash(&[]).hits.len(),
        "{:?}",
        carnage.hits
    );
}

#[test]
fn the_sun_goddess_halves_the_dash_and_circles_its_orbs_round_the_dashing_player() {
    let goddess = dash(&[("the_sun_goddess", 2)]);
    let orbs = births(&goddess, ORB);
    assert_eq!(orbs.len(), 6);
    let spawn = mover(&[("the_sun_goddess", 2)]);
    assert_eq!(spawn.speed, 7.);
    let all = spawns(&[("the_sun_goddess", 2)]);
    let orb = all.iter().find(|s| s.object == ORB).expect("the orbs");
    assert_eq!(orb.share, Some(1.4));
    let mover_at: Vec<[f32; 2]> = shapes(&goddess, MOVER).map(|s| s.pos).collect();
    for (frame, shapes) in goddess.frames.iter().enumerate().take(mover_at.len()) {
        for orb in shapes.iter().filter(|s| s.object.name == ORB) {
            let centre = mover_at[frame];
            let r = (orb.pos[0] - centre[0]).hypot(orb.pos[1] - centre[1]);
            assert!(near(r, 120., 0.5), "{frame}: {r}");
        }
    }
    assert!(shapes(&goddess, ORB).all(|s| s.age <= spawn.life.unwrap_or(0)));
    assert!(shares(&goddess).iter().any(|s| near(*s, 1.4, 1e-4)));
}

#[test]
fn photon_dash_triples_the_speed_and_stops_short_of_the_dummy_burning_it() {
    let photon = dash(&[("photon_dash", 1)]);
    let end = last(&photon, MOVER);
    assert!(near(end[0], 116.12, 0.05), "{end:?}");
    let aura = births(&photon, AURA)[0].1;
    assert!(
        near(aura.pos[0], end[0], 1e-3) && near(aura.pos[1], 0., 1e-3),
        "{:?}",
        aura.pos
    );
    let flames = births(&photon, FLAME);
    let under = flames
        .iter()
        .find(|(_, f)| f.pos == DUMMY)
        .expect("a flame under the dummy");
    assert!(near(under.1.scale[0], 0.6, 1e-4));
    assert!(
        shares(&photon).iter().any(|s| near(*s, 1.5, 1e-4)),
        "{:?}",
        shares(&photon)
    );
}

#[test]
fn what_carries_the_dash_values_leaves_photon_flames_but_they_leave_none() {
    let spawn = mover(&[("photon_dash", 3), ("solar_burst", 5)]);
    let blast = spawn
        .on_end
        .iter()
        .find(|s| s.object == BLAST)
        .expect("a blast");
    let flame = &blast.procs[0].spawn;
    assert_eq!(flame.object, FLAME);
    assert!(near(flame.share.unwrap_or(0.), 2.25 * 2.5, 1e-4));
    assert!(flame.procs.is_empty());
    let flame = &spawns(&[("photon_dash", 3), ("solar_burst", 5)])[1];
    assert_eq!(flame.object, FLAME);
    assert_eq!(flame.procs.len(), 1);
}

#[test]
fn restoring_flames_or_else_protective_cover_buffs_the_player_for_3_s() {
    assert_eq!(
        dash(&[("restoring_flames", 2)]).effects.buffed,
        Some(([0, 180], 50.))
    );
    assert_eq!(
        dash(&[("protective_cover", 5)]).effects.buffed,
        Some(([0, 180], 35.))
    );
    let both = dash(&[("restoring_flames", 1), ("protective_cover", 5)]);
    assert_eq!(both.effects.buffed, Some(([0, 180], 25.)));
}

#[test]
fn damage_burn_and_stun_nodes_leave_the_dash_unchanged() {
    let base = dash(&[]);
    let same = dash(&[("burn_burn_burn", 5), ("plasma_dash", 5), ("to_ashes", 5)]);
    assert_eq!(same.hits, base.hits);
    assert_eq!(same.frames.len(), base.frames.len());
    assert_eq!(shares(&same), shares(&base));
}

#[test]
fn every_node_at_once_still_plays_bounded_uses() {
    let tree = [
        ("interstellar_traveller", 5),
        ("burn_burn_burn", 5),
        ("solar_burst", 5),
        ("restoring_flames", 5),
        ("cooldown_lap", 5),
        ("plasma_dash", 5),
        ("to_ashes", 5),
        ("protective_cover", 5),
        ("intense_flames", 5),
        ("shooting_star", 5),
        ("bursting_flames", 3),
        ("solar_carnage", 3),
        ("the_sun_goddess", 3),
        ("photon_dash", 3),
    ];
    for storm in dashes(&tree).iter() {
        assert!(!storm.hits.is_empty());
        assert!(storm.frames.len() < 1200, "{}", storm.frames.len());
    }
}

/// Damage-weighted hits a use is expected to land, each creation roll counted at its odds.
fn expected(ranks: &[(&str, u32)]) -> f32 {
    parity::expected("solar_dash", &spawns(ranks))
}

#[test]
fn the_engines_subskill_hits_follow_the_sim() {
    let base = expected(&[]);
    assert!(near(base, 26., 1e-4), "{base}");
    let engine = |ranks: &[(&str, u32)]| parity::engine_hits("exo", "solar_dash", ranks);
    let wrong = parity::misses("exo", "solar_dash", expected, engine, 0.01);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
