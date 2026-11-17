//! Pyromancer's Breath of Fire: TalentsPyromancer case 30 rolls the cast and makes its breaths or
//! Molten Orbs, their Step_0 add Double Breeze's breath and the orb's, and
//! ProjectileCollision02Pyromancer turns each carrier's hit into fragments, novas, chains, stacks.
use super::Tree;
use super::pyro::{self, sure};
use crate::hitbox;
use crate::skill_effects::{Dice, Proc, Stacks};
use crate::skill_motion::Trail;
use crate::skill_sim::{self, Fall, Grow, Spawn};

const SKILL: &str = "breath_of_fire";
const EXPANDING: &str = "expanding_heat";
const COMBUSTION: &str = "heat_combustion";
const DOUBLE: &str = "double_breeze";
const EXHALE: &str = "scorching_exhale";
const BURST: &str = "burst_of_heat";
const HYDRA: &str = "hydras_breeze";
const CHAIN: &str = "chain_of_fire";
const HELLFLAME: &str = "scorching_hellflame";
const ORB: &str = "molten_orb";
/// Casts replayed: a Hellflame cycle of ten stacks and the pulse.
pub(super) const CASTS: u64 = 11;
/// Breath Create_0: growthSizeMax 1.25 and yscale 1.25, growing 0.1 a frame from 0.25.
const BREATH_CAP: f32 = 1.25;
const BREATH_Y: f32 = 1.25;
const BREATH_GROWTH: f32 = 0.1;
/// Step_0 makes the second breath once the fade (Alarm_10, 2 frames late) has alpha under 0.75.
const FADE_DELAY: u32 = 2;
const SECOND_AFTER_FADE: u32 = 6;
/// PC02 L0x79-L0x7f: 5 Heat_Combustion_obj 10° apart about the breath, 8 px out, 6 px a frame.
const FRAGMENT: &str = "Pyromancer_Breath_Heat_Combustion_obj";
const FRAGMENTS: u32 = 5;
const FRAGMENT_APART: f32 = 10.;
const FRAGMENT_OUT: f32 = 8.;
const FRAGMENT_SPEED: f32 = 6.;
/// Heat_Combustion_obj Create_0: 20 px up, 6 px a frame upward, gravity 0.5.
const FRAGMENT_FALL: Fall = Fall {
    height: 20.,
    height_step: 0.,
    z_speed: 6.,
    gravity: 0.5,
};
/// PC02 L0x93-L0x94: a Firenova on the burning enemy, maxScale 0.5.
const NOVA_MAX: f32 = 0.5;
/// PC02 L0xc5-L199: the chained breath starts 10 px from the enemy toward the nearest enemy.
const CHAIN_OUT: f32 = 10.;
/// PC02 L0xe3-L0xef: at 10 stacks, damage × (1 + 0.01 × value × stacks), 1 + 0.01 × size big.
const HELLFLAME_STACKS: u32 = 10;
/// Molten_Orb_obj Create_0: 3 px a frame for 2 s, scale 0.25 + 0.05 a frame, first breath at 0.1 s.
const ORB_OBJ: &str = "Pyromancer_Breath_Molten_Orb_obj";
const ORB_OUT: f32 = 10.;
const ORB_SPEED: f32 = 3.;
const ORB_LIFE: f32 = 120.;
const ORB_START: f32 = 0.25;
const ORB_GROWTH: f32 = 0.05;
const ORB_FIRST: u32 = 6;
/// Case 30 L0x2df-L0x2e0: hitFrequency = 0.5 s × (1 − Molten Orb's rate).
const ORB_PERIOD: f32 = 30.;
/// Molten_Orb_obj Destroy_0: an explosion 1 + explosionSize big where the orb ends.
const ORB_BLAST: f32 = 2.;
/// GameMaker's epsilon when it compares reals.
const EPSILON: f32 = 1e-5;

/// What a cast rolls before its breaths exist (L0x2bb-L0x2c8), each roll its own draw.
struct Rolls {
    /// Heat Combustion's fragment share of the breath's hit.
    combustion: Option<f32>,
    double: bool,
    /// Burst of Heat's nova share of the breath's hit.
    burst: Option<f32>,
}

/// The cast's rolls, `roll` deciding each node's from its chance.
fn rolls(tree: &Tree, mut roll: impl FnMut(&str, f32) -> bool) -> Rolls {
    let mut lucky = |node: &str, effect: &str| {
        tree.proc(node, effect)
            .filter(|(chance, _)| roll(node, *chance))
            .map(|(_, value)| value / 100.)
    };
    let combustion = lucky(COMBUSTION, "fire_skill_damage");
    let double = lucky(DOUBLE, "of_total_damage").is_some();
    let burst = lucky(BURST, "of_total_damage");
    Rolls {
        combustion,
        double,
        burst,
    }
}

/// The rolls the cast's own dice make.
fn dealt(tree: &Tree) -> Rolls {
    let mut dice = Dice::of_cast(tree.cast);
    rolls(tree, |_, chance| dice.roll(chance))
}

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    cast(spawns, tree, &dealt(tree), stacks_before(spawns, tree))
}

/// The cast's spawns, `from` Hellflame stacks left by the casts before it.
fn cast(spawns: &[Spawn], tree: &Tree, rolls: &Rolls, from: u32) -> Vec<Spawn> {
    let cast = |base| Cast {
        tree,
        rolls,
        base,
        from,
    };
    spawns.iter().map(|base| cast(base).volley()).collect()
}

/// Hellflame stacks the turn's earlier casts left, in cycles of ten stacks and the pulse's hit.
fn stacks_before(spawns: &[Spawn], tree: &Tree) -> u32 {
    if !tree.learned(HELLFLAME) {
        return 0;
    }
    let (Some(fx), Some(hitbox)) = (skill_sim::skill(SKILL), hitbox::skill(SKILL)) else {
        return 0;
    };
    let piled = (0..tree.cast).map(|n| {
        let earlier = Tree {
            nodes: tree.nodes,
            mods: tree.mods,
            cast: n,
        };
        let spawns = cast(spawns, &earlier, &dealt(&earlier), 0);
        skill_sim::replay_use(fx, hitbox, &spawns, 1, n)
            .effects
            .stacks
    });
    piled.sum::<u32>() % (HELLFLAME_STACKS + 1)
}

/// One cast: its rolls, the base breath and the Hellflame stacks it starts with.
struct Cast<'a> {
    tree: &'a Tree<'a>,
    rolls: &'a Rolls,
    base: &'a Spawn,
    from: u32,
}

impl Cast<'_> {
    /// L0x2e8-L0x2ed, L0x2fb: 1 + Hydra's Breeze breaths fanned 15° apart, or as many orbs.
    fn volley(&self) -> Spawn {
        let extra = self.tree.own(HYDRA, "projectile_count").round() as u32;
        let one = if self.tree.learned(ORB) {
            self.orb()
        } else {
            self.breath()
        };
        Spawn { extra, ..one }
    }

    /// Expanding Heat's share (L0x2b6).
    fn wider(&self) -> f32 {
        self.tree.own(EXPANDING, "area_of_effect") / 100.
    }

    /// Scorching Exhale's share (L0x2b7).
    fn longer(&self) -> f32 {
        self.tree.own(EXHALE, "skill_range") / 100.
    }

    /// destroyTimer stretched by the build's Skill Duration.
    fn stretched(&self, life: f32) -> u32 {
        (life * (1. + self.tree.mods.duration_pct / 100.)).ceil() as u32
    }

    /// A breath with `growth` a frame up to `cap` and `y` high, the size bonus on both scales.
    fn sized(&self, growth: f32, cap: f32, y: f32, procs: Vec<Proc>) -> Spawn {
        let base = self.base;
        let bonus = self.tree.mods.size_bonus;
        let start = base.scale.map_or(0., |[x, _]| x);
        Spawn {
            life: Some(self.stretched(base.life.unwrap_or(0) as f32)),
            scale: Some([start + bonus, y + bonus]),
            grow: Some(Grow {
                step: growth,
                max: Some(cap),
                x_only: true,
                ..Grow::default()
            }),
            procs,
            ..base.clone()
        }
    }

    /// L0x2e3-L0x2e5: growthSize and growthSizeMax take both nodes, the yscale Expanding Heat's.
    fn grown(&self, procs: Vec<Proc>) -> Spawn {
        let (wider, longer) = (self.wider(), self.longer());
        let both = 1. + wider + longer;
        let y = BREATH_Y * (1. + wider);
        self.sized(BREATH_GROWTH * both, BREATH_CAP * both, y, procs)
    }

    /// The cast's breath, with Double Breeze's second one when the cast rolled it.
    fn breath(&self) -> Spawn {
        let breath = self.grown(self.carried(1., true));
        let second = self.rolls.double.then(|| Box::new(self.second(&breath)));
        Spawn {
            trail: second,
            ..breath
        }
    }

    /// Step_0: Create's sizes and dirGap, so a plain breath at the aim, keeping Burst of Heat.
    fn second(&self, breath: &Spawn) -> Trail {
        let after = breath.life.unwrap_or(0) + FADE_DELAY + SECOND_AFTER_FADE;
        let plain = self.sized(BREATH_GROWTH, BREATH_CAP, BREATH_Y, self.nova(false));
        let spawn = Spawn {
            at_player: true,
            aim: true,
            ..plain
        };
        Trail {
            every: after,
            first: after,
            gap: 0.,
            spawn,
            next: None,
        }
    }

    /// A carrier's effects in push order (329, 331, 332, 334); 329 and 332 act from breaths only.
    fn carried(&self, share: f32, breath: bool) -> Vec<Proc> {
        let fragments = (self.rolls.combustion)
            .filter(|_| breath)
            .map(|part| self.fragments(part))
            .unwrap_or_default();
        let chain = (breath && self.tree.learned(CHAIN)).then(|| sure(self.chained()));
        let nova = self.nova(true);
        let stack = self.hellflame(share, true);
        fragments
            .into_iter()
            .chain(nova)
            .chain(chain)
            .chain(stack)
            .collect()
    }

    /// PC02 L0x7c-L0x88: 5 fragments flying out of the enemy about the breath's angle.
    fn fragments(&self, share: f32) -> Vec<Proc> {
        let middle = (FRAGMENTS - 1) as f32 / 2.;
        let fragment = |i: u32| Spawn {
            object: FRAGMENT.into(),
            at_enemy: true,
            turn: Some(FRAGMENT_APART * (i as f32 - middle)),
            offset: FRAGMENT_OUT,
            speed: FRAGMENT_SPEED,
            fall: Some(FRAGMENT_FALL),
            share: Some(share),
            procs: self.carried(share, false),
            ..Spawn::default()
        };
        (0..FRAGMENTS).map(|i| sure(fragment(i))).collect()
    }

    /// PC02 L0x91-L0x9e: never zeroed, so every carrier's hit on the burning dummy makes a nova.
    fn nova(&self, stacking: bool) -> Vec<Proc> {
        let Some(share) = self.rolls.burst else {
            return Vec::new();
        };
        let procs = if stacking {
            self.hellflame(share, false).into_iter().collect()
        } else {
            Vec::new()
        };
        let nova = Spawn {
            at_enemy: true,
            share: Some(share),
            procs,
            ..pyro::firenova(NOVA_MAX)
        };
        vec![Proc {
            once: false,
            ..sure(nova)
        }]
    }

    /// PC02 L0xbe-L0xd8: toward the player's nearest enemy, here the dummy (0°); Create's cap.
    fn chained(&self) -> Spawn {
        let grown = self.grown(self.carried(1., false));
        let cap = grown.grow.map(|grow| Grow {
            max: Some(BREATH_CAP),
            ..grow
        });
        Spawn {
            at_enemy: true,
            aim: true,
            offset: CHAIN_OUT,
            grow: cap,
            ..grown
        }
    }

    /// PC02 L0xe2-L0xfa: each carrier's first hit stacks the buff, or pulses when it finds ten;
    /// the pulse inherits its carrier's nova, which a nova carrier has left out.
    fn hellflame(&self, share: f32, nova: bool) -> Option<Proc> {
        if !self.tree.learned(HELLFLAME) {
            return None;
        }
        let value = self.tree.own(HELLFLAME, "fire_skill_damage") / 100.;
        let size = 1. + self.tree.own(HELLFLAME, "area_of_effect") / 100.;
        let pulse = Spawn {
            at_enemy: true,
            share: Some(share * (1. + value * HELLFLAME_STACKS as f32)),
            procs: if nova { self.nova(false) } else { Vec::new() },
            stacks: Some(Stacks {
                max: HELLFLAME_STACKS,
                from: self.from,
            }),
            ..pyro::blast(size)
        };
        Some(sure(pulse))
    }

    /// The orb flying out along the volley's angle, breathing at the dummy from where it is.
    fn orb(&self) -> Spawn {
        let rate = self.tree.own(ORB, "faster_cast_rate") / 100.;
        let period = (ORB_PERIOD * (1. - rate) - EPSILON).ceil().max(1.) as u32;
        let breath = Spawn {
            aim: true,
            ..self.grown(self.carried(1., true))
        };
        let blast = pyro::blast(ORB_BLAST);
        Spawn {
            object: ORB_OBJ.into(),
            offset: ORB_OUT,
            speed: ORB_SPEED,
            life: Some(self.stretched(ORB_LIFE)),
            scale: Some([ORB_START; 2]),
            grow: Some(Grow {
                step: ORB_GROWTH,
                max: Some(1.),
                ..Grow::default()
            }),
            ability: true,
            trail: Some(Box::new(Trail {
                every: period,
                first: ORB_FIRST,
                gap: 0.,
                spawn: breath,
                next: None,
            })),
            on_end: vec![blast],
            ..Spawn::default()
        }
    }
}

#[cfg(test)]
#[path = "breath_of_fire_parity_tests.rs"]
mod parity;
#[cfg(test)]
#[path = "breath_of_fire_tests.rs"]
mod tests;
