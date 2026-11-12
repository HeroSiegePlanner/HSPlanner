//! Objects several Pyromancer skills make, and the cursor geometry of the skills cast on it.
use super::Mods;
use crate::skill_effects::{Dice, Proc};
use crate::skill_motion::Trail;
use crate::skill_sim::{self, Fall, Grow, Spawn};

/// Universal_Double_Cast_obj: castDelay 0.25 s counted down from its first Step, then it casts.
pub(super) const DOUBLE_CAST_DELAY: u32 = 16;

/// Explosion_Item_obj: deadly at 4 frames, gone with Pyromancer_Explosion_spr after 18.
const BLAST: &str = "Explosion_Item_obj";
const BLAST_ACTIVE: u32 = 4;
const BLAST_LIFE: u32 = 18;
/// Firenova Create_0 and Step_0: 0.1 big, closing 0.075 of the gap to maxScale a frame.
const NOVA: &str = "Pyromancer_Firenova_obj";
const NOVA_START: f32 = 0.1;
const NOVA_LERP: f32 = 0.075;
const NOVA_LIFE: u32 = 45;
const NOVA_DEADLY: u32 = 15;
/// Comet_obj and Comet_Hydra_obj Create_0: gravityPower 1.25 unless their caster sets it.
const COMET_GRAVITY: f32 = 1.25;
/// Armageddon_obj Create_0: it falls under gravityPower 0.225.
const METEOR: &str = "Pyromancer_Armageddon_obj";
const METEOR_GRAVITY: f32 = 0.225;
/// Blazing_Trail_obj Create_0: it grows 0.075 a frame from 0.05, lasts 2.5 s and ticks every 0.5 s.
/// Alarm_0 resets maxScale to the unset image_xscale 1.
const PATCH: &str = "Pyromancer_Blazing_Trail_obj";
const PATCH_START: f32 = 0.05;
const PATCH_GROWTH: f32 = 0.075;
const PATCH_LIFE: f32 = 150.;
const PATCH_FADE: u32 = 20;
const PATCH_TICK: f32 = 30.;
/// Blazing_Trail_Controller_obj Step_0 and Step_1 with circleOfFire: it walks for skillTimer (2 s)
/// and drops a patch once over floor(42 × skillScale 1) px past the last, ticking every
/// skillTickRate (1 s).
const CONTROLLER: &str = "Pyromancer_Blazing_Trail_Controller_obj";
const CONTROLLER_LIFE: u32 = 121;
const CONTROLLER_GAP: f32 = 42.;
const CONTROLLER_TICK: f32 = 60.;
/// Steps a fall is followed for before it is taken to never land.
const MAX_STEPS: usize = 3600;
/// The 327 and 338 handlers' fireflies: 8 px round the player; Fly_obj: 6 px a frame, 10 s.
const FLY: &str = "Pyromancer_Fire_Ball_Fly_obj";
const FLY_OUT: f32 = 8.;
const FLY_SPEED: f32 = 6.;
const FLY_LIFE: u32 = 600;
/// Fire_Ball_obj and Split_obj Create_0 and Step_0: image_xscale grows 0.05 a frame from 0.25 to 1.
pub(super) const BALL_START: f32 = 0.25;
const BALL_GROWTH: f32 = 0.05;
/// Split_obj Create_0: 10 px a frame for 3 s; Destroy_0: an explosion 1 + explosionSize (1) big.
const SPLIT: &str = "Pyromancer_Fire_Ball_Split_obj";
const SPLIT_SPEED: f32 = 10.;
const SPLIT_LIFE: u32 = 180;
const SPLIT_BLAST: f32 = 2.;
/// Scorching_Aura_obj Create_0: alarm[10] = 0.1 s, singleHit.
const AURA: &str = "Pyromancer_Scorching_Aura_obj";
const AURA_LIFE: u32 = 6;
/// Scorching_Searing_Burst_obj: Other_7 ends it with its 10 frames at 25 fps.
const BURST: &str = "Pyromancer_Scorching_Searing_Burst_obj";
const BURST_LIFE: u32 = 24;
/// Mask_Player_spr's box about the player's origin: left, top, right, bottom.
const PLAYER_BOX: [f32; 4] = [-12., -6., 11., 17.];
/// Fire_Ball_Orbital_obj Step_0 turns it 40° past its place on the circle.
pub(super) const ORBITAL: &str = "Pyromancer_Fire_Ball_Orbital_obj";
pub(super) const ORBITAL_FACE: f32 = 40.;

/// A proc the collision always makes, once per carrier as the game zeroes it.
pub(super) fn sure(spawn: Spawn) -> Proc {
    Proc {
        chance: 1.,
        spawn,
        pull: None,
        once: true,
    }
}

/// A Fly_obj made round the player at `turn`, homing on the enemy and exploding where its centre
/// meets it (Step_0 L0xe-L0x1e), the blast dealing `share` of the hit.
pub(super) fn firefly(turn: f32, share: f32) -> Spawn {
    let blast = Spawn {
        share: Some(share),
        ..blast(1.)
    };
    Spawn {
        object: FLY.into(),
        at_player: true,
        turn: Some(turn),
        offset: FLY_OUT,
        speed: FLY_SPEED,
        ability: true,
        life: Some(FLY_LIFE),
        on_end: vec![blast],
        ..Spawn::default()
    }
}

/// An Explosion_Item_obj `scale` big.
pub(super) fn blast(scale: f32) -> Spawn {
    Spawn {
        object: BLAST.into(),
        scale: Some([scale; 2]),
        active_from: Some(BLAST_ACTIVE),
        life: Some(BLAST_LIFE),
        ..Spawn::default()
    }
}

/// The growth of a fire ball's image_xscale.
pub(super) fn ball_growth() -> Grow {
    Grow {
        step: BALL_GROWTH,
        max: Some(1.),
        x_only: true,
        ..Grow::default()
    }
}

/// A Split_obj as its Create_0 makes it, hitting each enemy once with `share` of the hit and
/// exploding where it ends.
pub(super) fn split(share: f32) -> Spawn {
    let blast = Spawn {
        share: Some(share),
        ..blast(SPLIT_BLAST)
    };
    Spawn {
        object: SPLIT.into(),
        speed: SPLIT_SPEED,
        scale: Some([BALL_START, 1.]),
        grow: Some(ball_growth()),
        life: Some(SPLIT_LIFE),
        share: Some(share),
        on_end: vec![blast],
        ..Spawn::default()
    }
}

/// A Scorching_Aura_obj `scale` big.
pub(super) fn aura(scale: f32) -> Spawn {
    Spawn {
        object: AURA.into(),
        scale: Some([scale; 2]),
        life: Some(AURA_LIFE),
        ..Spawn::default()
    }
}

/// A Searing_Burst_obj `scale` big dealing `share` of the hit.
pub(super) fn burst(scale: f32, share: f32) -> Spawn {
    Spawn {
        object: BURST.into(),
        scale: Some([scale; 2]),
        life: Some(BURST_LIFE),
        share: Some(share),
        ..Spawn::default()
    }
}

/// distance_to_object between a box, its edges about `pos`, and the player's at the origin.
pub(super) fn player_gap(edges: [f32; 4], pos: [f32; 2]) -> f32 {
    let [left, top, right, bottom] = edges;
    let [p_left, p_top, p_right, p_bottom] = PLAYER_BOX;
    let dx = (pos[0] + left - p_right)
        .max(p_left - pos[0] - right)
        .max(0.);
    let dy = (pos[1] + top - p_bottom)
        .max(p_top - pos[1] - bottom)
        .max(0.);
    dx.hypot(dy)
}

/// A Firenova growing to `max`.
pub(super) fn firenova(max: f32) -> Spawn {
    Spawn {
        object: NOVA.into(),
        scale: Some([NOVA_START; 2]),
        grow: Some(Grow {
            lerp: NOVA_LERP,
            max: Some(max),
            ..Grow::default()
        }),
        life: Some(NOVA_LIFE),
        deadly: Some(NOVA_DEADLY),
        ..Spawn::default()
    }
}

/// A comet `object`, `scale` big, falling from `height` under its own gravity; it never hits.
pub(super) fn comet(object: &str, scale: f32, height: f32) -> Spawn {
    Spawn {
        object: object.into(),
        ability: true,
        scale: Some([scale; 2]),
        fall: Some(Fall {
            height,
            gravity: COMET_GRAVITY,
            ..Fall::default()
        }),
        ..Spawn::default()
    }
}

/// An Armageddon_obj made at the enemy hit, falling from `height` and ending in `blast`.
pub(super) fn meteor(height: f32, blast: Spawn) -> Spawn {
    Spawn {
        at_enemy: true,
        ..falling(height, vec![blast])
    }
}

/// An Armageddon_obj falling from `height`, making `ends` where it lands.
pub(super) fn falling(height: f32, ends: Vec<Spawn>) -> Spawn {
    Spawn {
        object: METEOR.into(),
        ability: true,
        fall: Some(Fall {
            height,
            gravity: METEOR_GRAVITY,
            ..Fall::default()
        }),
        on_end: ends,
        ..Spawn::default()
    }
}

/// A Blazing_Trail_obj dealing `share` of the hit a tick, stretched and widened by the build.
pub(super) fn patch(mods: &Mods, share: f32) -> Spawn {
    sized_patch(mods, share, 1. + mods.size_bonus, PATCH_TICK)
}

/// A Blazing_Trail_obj growing to `max` and dealing `share` of the hit every `tick` frames, its
/// life stretched by the build.
pub(super) fn sized_patch(mods: &Mods, share: f32, max: f32, tick: f32) -> Spawn {
    let life = PATCH_LIFE * (1. + mods.duration_pct / 100.);
    Spawn {
        object: PATCH.into(),
        scale: Some([PATCH_START; 2]),
        grow: Some(Grow {
            step: PATCH_GROWTH,
            max: Some(max),
            ..Grow::default()
        }),
        life: Some(life.round() as u32),
        fade: PATCH_FADE,
        tick,
        share: Some(share),
        ..Spawn::default()
    }
}

/// A patch's image_xscale `steps` Steps after Alarm_0 shrinks it, Alarm_11's `bonus` added.
pub(super) fn patch_scale(steps: u32, max: f32, bonus: f32) -> f32 {
    (PATCH_START + PATCH_GROWTH * steps as f32 + bonus).min(max)
}

/// A trail dropping each spawn once, after its moves; the moves rise along `drops`.
pub(super) fn once_each(
    drops: impl DoubleEndedIterator<Item = (u32, Spawn)>,
) -> Option<Box<Trail>> {
    drops.rev().fold(None, |next, (moves, spawn)| {
        Some(Box::new(Trail {
            every: u32::MAX,
            first: moves,
            gap: 0.,
            spawn,
            next,
        }))
    })
}

/// An unseen Blazing_Trail_Controller_obj walking `speed` px a frame along its direction, its
/// patches dealing `share` of the hit a tick.
pub(super) fn controller(mods: &Mods, speed: f32, share: f32) -> Spawn {
    let patch = Spawn {
        tick: CONTROLLER_TICK,
        ..patch(mods, share)
    };
    Spawn {
        object: CONTROLLER.into(),
        speed,
        ability: true,
        life: Some(CONTROLLER_LIFE),
        trail: Some(Box::new(Trail {
            every: 0,
            first: 0,
            gap: CONTROLLER_GAP,
            spawn: patch,
            next: None,
        })),
        ..Spawn::default()
    }
}

/// Moves a falling object makes before it lands; the sim moves and drops it from its second Step.
pub(super) fn landing(fall: Fall) -> u32 {
    let path = std::iter::successors(Some((fall.height, fall.z_speed)), |&(height, z)| {
        let z = z - fall.gravity;
        Some((height + z, z))
    });
    let landed = path
        .take(MAX_STEPS)
        .skip(1)
        .position(|(height, _)| height <= 0.);
    landed.map_or(0, |at| at as u32 + 1)
}

/// Where `skill`'s cursor objects appear in the sim.
pub(super) fn cursor(skill: &str) -> [f32; 2] {
    let dummy = skill_sim::skill(skill).map_or(0., |fx| fx.dummy);
    [dummy, -skill_sim::TARGET_ABOVE]
}

/// GameMaker's lengthdir_x and lengthdir_y.
pub(super) fn lengthdir(length: f32, degrees: f32) -> [f32; 2] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [cos * length, -sin * length]
}

/// GameMaker's point_direction.
pub(super) fn toward(from: [f32; 2], to: [f32; 2]) -> f32 {
    (from[1] - to[1]).atan2(to[0] - from[0]).to_degrees()
}

/// GameMaker's irandom(n): a whole number from 0 to n.
pub(super) fn irandom(dice: &mut Dice, n: u32) -> f32 {
    (dice.unit() * (n + 1) as f32).floor().min(n as f32)
}
