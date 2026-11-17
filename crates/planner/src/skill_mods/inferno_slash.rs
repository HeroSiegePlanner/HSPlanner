//! Pyromancer's Inferno Slash: TalentsPyromancer case 23 rolls the attack's chances, draws the
//! swing and fills the waves' projEffect slots; attackClasses fires 1 + [117] salvos of 1 + [76]
//! Projectile_Player_obj that LoadProjectileInfernoSlash turns into flying crescents. Step_0
//! forks Searing Fork's waves on contact (LoadForkingProjectiles) and
//! ProjectileCollision02Pyromancer makes Molten Ground's patch and Flies of Agony's flies on each
//! carrier's first hit.
use super::Tree;
use super::pyro::{self, sure};
use crate::hitbox;
use crate::skill_effects::{Buff, Dice, Proc};
use crate::skill_sim::{self, Spawn};

const SKILL: &str = "inferno_slash";
const REACH: &str = "infernal_reach";
const MOLTEN: &str = "molten_ground";
const STACKED: &str = "stacked_burn";
const FUMES: &str = "accelerated_fumes";
const IGNITING: &str = "igniting_wind";
const CLEAVE: &str = "infernal_cleave";
const FORK: &str = "searing_fork";
const FLIES: &str = "flies_of_agony";
/// Casts replayed, so the attack's chance rolls show their odds.
pub(super) const CASTS: u64 = 8;
/// What draws nothing: the salvo fanning its waves out, and the player's buffs.
const UNSEEN: &str = "Draw_Player_Buff_obj";
/// attackClasses L0xc, L0x1a5: Igniting Wind's salvo wakes [120] = 0.15 s after the first.
const SALVO_DELAY: u32 = 9;
/// L0x164-L0x165, attackClasses L0xf2-L0xf3: Infernal Cleave adds 8 waves 360 / 8° apart.
const CLEAVE_WAVES: u32 = 8;
const FULL_TURN: f32 = 360.;
/// L0x16a: Searing Fork's waves are 12° apart.
const FORK_APART: f32 = 12.;
/// LoadForkingProjectiles: two forks 8 px out, 15° either side of the wave.
const FORK_TURN: f32 = 15.;
const FORK_OUT: f32 = 8.;
/// Step_0 L0xa1: a wave that forked is gone 3 frames later (alarm[0] = 3).
const AFTER_FORK: u32 = 3;
/// L0x159: Accelerated Fumes lasts 2 s; the 326 handler's Stacked Burn 5 s.
const FUMES_FRAMES: u32 = 120;
const STACKED_FRAMES: u32 = 300;

/// The attack's own chance rolls (L0x14a-L0x15d), each its own draw.
struct Rolls {
    molten: bool,
    stacked: bool,
    fumes: bool,
    igniting: bool,
}

/// The attack's rolls, `roll` deciding each learned node's from the node and its chance.
fn rolls(tree: &Tree, mut roll: impl FnMut(&str, f32) -> bool) -> Rolls {
    let mut comes_up = |node: &str| {
        tree.proc(node, "")
            .is_some_and(|(chance, _)| roll(node, chance))
    };
    Rolls {
        molten: comes_up(MOLTEN),
        stacked: comes_up(STACKED),
        fumes: comes_up(FUMES),
        igniting: comes_up(IGNITING),
    }
}

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    let mut dice = Dice::of_cast(tree.cast);
    attack(spawns, tree, rolls(tree, |_, chance| dice.roll(chance)))
}

/// An attack whose roll of each node comes up as `lucky` says for it.
#[cfg(test)]
pub(super) fn apply_rolled(
    spawns: &[Spawn],
    tree: &Tree,
    lucky: &dyn Fn(&str) -> bool,
) -> Vec<Spawn> {
    attack(spawns, tree, rolls(tree, |node, _| lucky(node)))
}

/// The nodes whose chance the attack rolls itself (L0x14a-L0x15d).
#[cfg(test)]
pub(super) const ROLLED: [&str; 4] = [MOLTEN, STACKED, FUMES, IGNITING];

fn attack(spawns: &[Spawn], tree: &Tree, rolls: Rolls) -> Vec<Spawn> {
    let [swing, wave] = spawns else {
        return spawns.to_vec();
    };
    let attack = Attack::new(wave, tree, rolls);
    let salvos = 1 + u32::from(attack.rolls.igniting);
    std::iter::once(swing.clone())
        .chain(attack.fumes())
        .chain((0..salvos).flat_map(|k| attack.salvo(k * SALVO_DELAY)))
        .collect()
}

/// One attack: its rolls, the base wave and what the nodes make of every wave.
struct Attack<'a> {
    tree: &'a Tree<'a>,
    rolls: Rolls,
    base: &'a Spawn,
    /// deltaSpeed 5 × [197], [197] = 1 + Infernal Reach.
    speed: f32,
    /// Waves of a salvo and the degrees between them.
    count: u32,
    apart: f32,
    /// Share of the full hit a wave deals, and of its fire part (damage[3]), the only part the
    /// 325 and 327 handlers copy.
    share: f32,
    fire: f32,
}

impl<'a> Attack<'a> {
    fn new(base: &'a Spawn, tree: &'a Tree<'a>, rolls: Rolls) -> Self {
        let fork = tree.own(FORK, "projectile_count").round() as u32;
        let (extra, apart) = if fork > 0 {
            (fork, FORK_APART)
        } else if tree.learned(CLEAVE) {
            (CLEAVE_WAVES, FULL_TURN / CLEAVE_WAVES as f32)
        } else {
            (0, 0.)
        };
        // L0x17f-L0x181: [1020] (the weapon bonus) and [993] (the fire) × (1 + its fire damage)
        // / [76]; the weapon's own 100 % stays.
        let cleave = if tree.learned(CLEAVE) {
            (1. + tree.own(CLEAVE, "fire_skill_damage") / 100.) / extra as f32
        } else {
            1.
        };
        let (fire, bonus) = (tree.mods.elemental, tree.mods.weapon_bonus);
        Self {
            tree,
            rolls,
            base,
            speed: base.speed * (1. + tree.own(REACH, "skill_range") / 100.),
            count: 1 + extra,
            apart,
            share: fire * cleave + (1. - fire) * (1. - bonus * (1. - cleave)),
            fire: fire * cleave,
        }
    }

    /// A wave as attackClasses makes it, the build's size added to its scale ([1086]).
    fn wave(&self) -> Spawn {
        Spawn {
            speed: self.speed,
            scale: Some([1. + self.tree.mods.size_bonus; 2]),
            share: Some(self.share),
            ..self.base.clone()
        }
    }

    /// The salvo woken `delay` frames late: a lone wave, or waves fanned about the aim that share
    /// one hit (attackClasses L0xf5-L0xf6: one singleHitPackId per salvo).
    fn salvo(&self, delay: u32) -> Vec<Spawn> {
        if self.count == 1 {
            let wave = self.wave();
            let lone = Spawn {
                frame: wave.frame + delay,
                procs: self.carried(self.fire),
                ..wave
            };
            return vec![lone];
        }
        let middle = (self.count - 1) as f32 / 2.;
        let turns: Vec<f32> = (0..self.count)
            .map(|i| self.apart * (i as f32 - middle))
            .collect();
        let contacts: Vec<Option<u32>> = turns.iter().map(|&turn| self.contact(turn)).collect();
        let first = contacts.iter().flatten().min().copied();
        let hitter = contacts.iter().position(|&at| at.is_some() && at == first);
        let waves = turns.iter().zip(&contacts).enumerate();
        let waves = waves.map(|(i, (&turn, &at))| self.fanned(turn, at, Some(i) == hitter));
        vec![unseen(delay, waves.collect())]
    }

    /// A fanned wave: the one that lands the salvo's hit forks on it; the others pass through,
    /// forking where they touch.
    fn fanned(&self, turn: f32, contact: Option<u32>, hits: bool) -> Spawn {
        let wave = Spawn {
            frame: 0,
            turn: Some(turn),
            ..self.wave()
        };
        let forks = self.tree.learned(FORK);
        match contact {
            Some(at) if hits => Spawn {
                life: forks
                    .then_some(at + 1 + self.remnant_steps(at))
                    .or(wave.life),
                procs: (self.forks(at).into_iter().map(sure))
                    .chain(self.carried(self.fire))
                    .collect(),
                ..wave
            },
            Some(at) if forks => Spawn {
                ability: true,
                ends_on_contact: true,
                on_end: self.remnant(at).into_iter().chain(self.forks(at)).collect(),
                ..wave
            },
            _ => Spawn {
                ability: true,
                ..wave
            },
        }
    }

    /// The age at which a lone wave turned `turn` from the aim first touches the dummy.
    fn contact(&self, turn: f32) -> Option<u32> {
        let fx = skill_sim::skill(SKILL)?;
        let hitbox = hitbox::skill(SKILL)?;
        let lone = Spawn {
            frame: 0,
            turn: Some(turn),
            ..self.wave()
        };
        let hits = skill_sim::replay_use(fx, hitbox, &[unseen(0, vec![lone])], 1, 0).hits;
        // Fanned out on frame 0, it steps from frame 1 at age 0.
        hits.first().map(|frame| frame - 1)
    }

    /// Searing Fork's two forks of a wave touching at `at`: Projectile_Player_obj that copy its
    /// damage, xscale (not yscale), speed, modifiers and animation, wake a frame later and hit
    /// on their own; the sim shows them from that frame, one move out.
    fn forks(&self, at: u32) -> Vec<Spawn> {
        let life = self.left(at);
        if !self.tree.learned(FORK) || life == 0 {
            return Vec::new();
        }
        let fork = |turn: f32| Spawn {
            object: self.base.object.clone(),
            turn: Some(turn),
            offset: FORK_OUT + self.speed,
            speed: self.speed,
            active_from: Some(0),
            scale: Some([1. + self.tree.mods.size_bonus, 1.]),
            life: Some(life),
            share: Some(self.share),
            procs: self.carried(self.fire),
            ..Spawn::default()
        };
        vec![fork(FORK_TURN), fork(-FORK_TURN)]
    }

    /// Steps a wave has after touching at `at`, until Other_7 ends it with its animation.
    fn left(&self, at: u32) -> u32 {
        let last = self.base.life.unwrap_or(0).saturating_sub(1);
        last.saturating_sub(at)
    }

    /// Steps a wave that forked at `at` takes before its alarm, or its animation, ends it.
    fn remnant_steps(&self, at: u32) -> u32 {
        self.left(at).min(AFTER_FORK - 1)
    }

    /// A wave that forked without the salvo's hit, flying on until it is gone; the sim shows it
    /// from the next frame, one move on.
    fn remnant(&self, at: u32) -> Option<Spawn> {
        let steps = self.remnant_steps(at);
        (steps > 0).then(|| Spawn {
            turn: Some(0.),
            offset: self.speed,
            ability: true,
            life: Some(steps),
            ..Spawn {
                frame: 0,
                ..self.wave()
            }
        })
    }

    /// What a carrier with `fire` share of the hit in damage[3] makes on its first hit, in the
    /// handlers' push order: Molten Ground's patch (325), Stacked Burn's buff (326), then Flies
    /// of Agony's flies (327), which keep the 325 that no handler zeroes.
    fn carried(&self, fire: f32) -> Vec<Proc> {
        let patch = self.rolls.molten.then(|| sure(self.patch(fire)));
        patch
            .into_iter()
            .chain(self.stacked().map(sure))
            .chain(self.flies(fire, self.rolls.molten))
            .collect()
    }

    /// The 325 handler's Blazing_Trail_obj at the enemy, which carries the carrier's other
    /// modifiers: its first tick buffs and sends flies.
    fn patch(&self, carrier: f32) -> Spawn {
        let buff = self.stacked().map(sure);
        let flies = self.flies(self.molten(carrier), false);
        Spawn {
            procs: buff.into_iter().chain(flies).collect(),
            ..self.bare_patch(carrier)
        }
    }

    /// A 325 patch at the enemy with nothing left to make, as a fly's blast lays it.
    fn bare_patch(&self, carrier: f32) -> Spawn {
        Spawn {
            at_enemy: true,
            ..pyro::patch(self.tree.mods, self.molten(carrier))
        }
    }

    /// floor(d3 × [325]) a tick under a carrier dealing `carrier`, [325] = 20 % × rank.
    fn molten(&self, carrier: f32) -> f32 {
        let part = (self.tree.proc(MOLTEN, "fire_skill_damage")).map_or(0., |(_, v)| v);
        carrier * part / 100.
    }

    /// The 326 handler's buff: [326] = 3 × rank for 5 s.
    fn stacked(&self) -> Option<Spawn> {
        let percent = self.tree.proc(STACKED, "fire_skill_damage")?.1;
        self.rolls.stacked.then(|| buffing(STACKED_FRAMES, percent))
    }

    /// L0x159: BuffAdd 308 as the attack goes off, its speed for 2 s.
    fn fumes(&self) -> Option<Spawn> {
        let percent = self.tree.proc(FUMES, "increased_attack_speed")?.1;
        self.rolls.fumes.then(|| buffing(FUMES_FRAMES, percent))
    }

    /// The 327 handler: [328] = 3 × rank flies, 360 / [328]° apart round the player, each blast
    /// dealing floor(d3 × [327]), [327] = 35 % × rank. Flies keeping the carrier's `molten` 325
    /// hand it to their blasts (Fly_obj Step_0), each laying a patch.
    fn flies(&self, carrier: f32, molten: bool) -> Vec<Proc> {
        let count = self.tree.own(FLIES, "projectile_count").round() as u32;
        let share = carrier * self.tree.own(FLIES, "fire_skill_damage") / 100.;
        let round = FULL_TURN / count.max(1) as f32;
        let patch = molten.then(|| sure(self.bare_patch(share)));
        let fly = |i: u32| laying(pyro::firefly(round * i as f32, share), patch.as_ref());
        (0..count).map(|i| sure(fly(i))).collect()
    }
}

/// `fly` whose blast makes `patch`, if any, on its hit.
fn laying(fly: Spawn, patch: Option<&Proc>) -> Spawn {
    let Some(patch) = patch else {
        return fly;
    };
    let on_end = (fly.on_end.iter())
        .map(|blast| Spawn {
            procs: vec![patch.clone()],
            ..blast.clone()
        })
        .collect();
    Spawn { on_end, ..fly }
}

/// An unseen object made `frame` frames into the use that ends at once, making `then`.
fn unseen(frame: u32, then: Vec<Spawn>) -> Spawn {
    Spawn {
        object: UNSEEN.into(),
        frame,
        ability: true,
        scale: Some([0., 0.]),
        life: Some(0),
        on_end: then,
        ..Spawn::default()
    }
}

/// An unseen object giving the player `percent` for `frames` as it is made.
fn buffing(frames: u32, percent: f32) -> Spawn {
    Spawn {
        buff: Some(Buff { frames, percent }),
        ..unseen(0, Vec::new())
    }
}

#[cfg(test)]
#[path = "inferno_slash_tests.rs"]
mod tests;
