//! Pyromancer's Hydra: TalentsPyromancer case 33 summons the hydras, Pyromancer_Hydra_obj grows
//! each one before it walks, shoots, bursts, clones and burns around itself, and
//! ProjectileCollision02Pyromancer turns the hits of what it makes into comets and novas.
use super::Tree;
use super::pyro::{self, lengthdir, sure, toward};
use crate::skill_effects::{Dice, Proc};
use crate::skill_motion::Trail;
use crate::skill_sim::{Grow, Placement, Spawn};

const SKILL: &str = "hydra";
const MORE_HEADS: &str = "more_heads_more_fire";
const SCORCHING_BLAST: &str = "scorching_blast";
const CLONING: &str = "cloning_combustion";
const BURSTFIRE: &str = "burstfire";
const ESSENCE: &str = "burning_essence";
const CALAMITY: &str = "chaotic_calamity";
const HERDER: &str = "hydra_herder";
const KRAKEN: &str = "scorching_kraken";
const SNAKES: &str = "searing_snakes";
/// Casts replayed, so the attacks' rolls show their odds.
pub(super) const CASTS: u64 = 8;
/// A trail that drops once: the preview's hydra lives for its first attack only.
const ONCE: u32 = u32::MAX;
/// GameMaker's epsilon when it compares reals.
const EPSILON: f32 = 1e-5;
/// Create_0 image_xscale 0.1; Step_0 L0xc grows it 0.05 × maxScale a frame, L4 shrinks it 0.05.
const START: f32 = 0.1;
const GROWTH: f32 = 0.05;
/// Case 33 L0x37d: the Kraken's maxScale.
const KRAKEN_SCALE: f32 = 2.25;
/// Case 33 L0x38d-L0x390: hydra i of n stands at 360 / n × i degrees, n × 8 px out once i > 1.
const RING: f32 = 8.;
/// Step_0 L0x5f, L0x62: Burstfire's alarm[3] = 0.15 s, Cloning Combustion's alarm[4] = 1.
const BURST_DELAY: u32 = 9;
const CLONE_DELAY: u32 = 1;
/// Alarm_4: the clone stands 40 px away in a random(360) direction and lasts 2 s.
const CLONE_OUT: f32 = 40.;
const CLONE_LIFE: u32 = 120;
/// Step_0 L0x27-L0x39: burningEssenceTimer (0.33 s) runs once grown, an aura every 20 frames.
const ESSENCE_EVERY: u32 = 20;
/// Create_0's shootFrequency (s): Step_0 L0x56 resets shootCooldown to it, and L0x3e checks it
/// before L0x58 counts it down, so a grown hydra attacks every ceil(60 × 1.5) + 1 frames.
#[cfg(test)]
const SHOOT_FREQUENCY: f32 = 1.5;
/// Step_0 L0x16-L0x20: 3 px a frame × (1 + Herder) toward 6 px under the host while over 128 px.
const HERD_SPEED: f32 = 3.;
const HERD_NEAR: f32 = 128.;
const HOST_BELOW: f32 = 6.;
const HERD_STEPS: usize = 256;
/// Box edges about the origin: Pyromancer_Hydra_Down_spr (L0 R30 T31 B46).
const HYDRA_BOX: [f32; 4] = [-16., -7., 14., 8.];
/// PC02 L0x184-L0x18f: Scorching Blast's Firenova on the enemy, maxScale 0.75 (Firenova Step_0).
const NOVA_MAX: f32 = 0.75;
/// PC02 L0x197-L0x1a6: comet i on the enemy, 0.25 big, yy 320 + 100 i, Create_0's gravityPower.
const COMET: &str = "Pyromancer_Comet_Hydra_obj";
const COMET_SCALE: f32 = 0.25;
const COMET_HEIGHT: f32 = 320.;
const COMET_STEP: f32 = 100.;
/// Comet_Hydra_obj Destroy_0: an explosion image_xscale × 2.2 big.
const COMET_BLAST: f32 = 2.2;
/// Breath_of_Fire_Hydra_obj Create_0: 0.25 × 1.5 big, 0.2 s, deadly 0.25 s; Step_0 L4: +0.33 x.
const BREATH: &str = "Pyromancer_Breath_of_Fire_Hydra_obj";
const BREATH_SCALE: [f32; 2] = [0.25, 1.5];
const BREATH_GROWTH: f32 = 0.33;
const BREATH_LIFE: u32 = 12;
const BREATH_DEADLY: u32 = 15;
/// Step_0 L0x1a fades alpha 0.05 a frame; L6-L7 breathes again under 0.75, 2 + 6 frames into it.
const BREATH_FADE: u32 = 20;
const SECOND_BREATH: u32 = BREATH_LIFE + 2 + 6;

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    summon_all(spawns, tree, |dice, _, chance| dice.roll(chance))
}

/// A cast whose rolls come up as `lucky` says for each node.
#[cfg(test)]
pub(super) fn apply_rolled(
    spawns: &[Spawn],
    tree: &Tree,
    lucky: &dyn Fn(&str) -> bool,
) -> Vec<Spawn> {
    summon_all(spawns, tree, |_, node, chance| chance > 0. && lucky(node))
}

/// The cast's hydras, each node's roll made by `roll` with the cast's dice and its chance.
fn summon_all(
    spawns: &[Spawn],
    tree: &Tree,
    mut roll: impl FnMut(&mut Dice, &str, f32) -> bool,
) -> Vec<Spawn> {
    let [hydra, ball] = spawns else {
        return spawns.to_vec();
    };
    let cast = Cast { tree, hydra, ball };
    let mut dice = Dice::of_cast(tree.cast);
    let places = cast.places();
    places
        .into_iter()
        .flat_map(|at| cast.summon(at, &mut dice, &mut roll))
        .collect()
}

/// One hydra: its spot from the cursor, creation frame, breath shot, burst roll and clone flag.
struct Head {
    at: [f32; 2],
    frame: u32,
    breathes: bool,
    burst: bool,
    clone: bool,
}

/// One cast: the build's nodes, the base hydra and its base fire ball.
struct Cast<'a> {
    tree: &'a Tree<'a>,
    hydra: &'a Spawn,
    ball: &'a Spawn,
}

impl Cast<'_> {
    fn kraken(&self) -> bool {
        self.tree.learned(KRAKEN)
    }

    /// The maxScale of the cast's hydras, which their clones copy.
    fn scale(&self) -> f32 {
        if self.kraken() { KRAKEN_SCALE } else { 1. }
    }

    /// Case 33 L0x337-L0x347: the sentry cap (1 for the Kraken, halved by the Herder) or one.
    fn count(&self) -> u32 {
        let more = self.tree.own(MORE_HEADS, "sentry_max_amount").round() as u32;
        let cap = if self.kraken() { 1 } else { 1 + more };
        let cap = if self.tree.learned(HERDER) {
            cap / 2
        } else {
            cap
        };
        if self.tree.learned(SNAKES) { cap } else { 1 }
    }

    /// L0x389-L0x398; the two hydras on the cursor block each other's steps (Step_0 L0x1c-L0x1e).
    fn places(&self) -> Vec<[f32; 2]> {
        let count = self.count();
        let ring = RING * count as f32;
        (0..count)
            .map(|i| {
                let out = if i > 1 { ring } else { 0. };
                let at = lengthdir(out, 360. / count as f32 * i as f32);
                if count > 1 && i < 2 {
                    at
                } else {
                    self.herded(at)
                }
            })
            .collect()
    }

    /// Where a herded hydra stops walking, a couple of steps after growing: shown there from birth.
    fn herded(&self, at: [f32; 2]) -> [f32; 2] {
        if !self.tree.learned(HERDER) {
            return at;
        }
        let speed = HERD_SPEED * (1. + self.tree.own(HERDER, "summon_movement_speed") / 100.);
        let cursor = pyro::cursor(SKILL);
        let start = [cursor[0] + at[0], cursor[1] + at[1]];
        let scale = self.scale();
        let walk = std::iter::successors(Some(start), |&pos| {
            let [dx, dy] = lengthdir(speed, toward(pos, [0., HOST_BELOW]));
            Some([pos[0] + dx, pos[1] + dy])
        });
        let stop = walk
            .take(HERD_STEPS)
            .find(|&pos| gap(pos, scale) <= HERD_NEAR)
            .unwrap_or(start);
        [stop[0] - cursor[0], stop[1] - cursor[1]]
    }

    /// One hydra of the cast and, when its first attack rolls one, its clone.
    fn summon(
        &self,
        at: [f32; 2],
        dice: &mut Dice,
        roll: &mut impl FnMut(&mut Dice, &str, f32) -> bool,
    ) -> Vec<Spawn> {
        let chance = |node: &str| self.tree.proc(node, "").map_or(0., |(chance, _)| chance);
        let (burst, cloning) = (chance(BURSTFIRE), chance(CLONING));
        let main = Head {
            at,
            frame: 0,
            breathes: self.kraken(),
            burst: roll(dice, BURSTFIRE, burst),
            clone: false,
        };
        let clone = roll(dice, CLONING, cloning).then(|| {
            let [dx, dy] = lengthdir(CLONE_OUT, dice.unit() * 360.);
            // Alarm_4 leaves scorchingKraken out: a Kraken's clone shoots fire balls.
            Head {
                at: self.herded([at[0] + dx, at[1] + dy]),
                frame: self.shot() + CLONE_DELAY,
                breathes: false,
                burst: roll(dice, BURSTFIRE, burst),
                clone: true,
            }
        });
        let heads = std::iter::once(main).chain(clone);
        heads.flat_map(|head| self.head(&head)).collect()
    }

    /// A hydra and the auras it burns around itself.
    fn head(&self, head: &Head) -> Vec<Spawn> {
        std::iter::once(self.hydra(head))
            .chain(self.auras(head))
            .collect()
    }

    /// Use frame of a fresh hydra's first shot: the base ball's, later by a bigger one's growth.
    fn shot(&self) -> u32 {
        self.ball.frame + grown(self.scale()) - grown(1.)
    }

    /// The preview's hydra lives until its first attack's sprite ends; a clone 2 s at most.
    fn life(&self, clone: bool) -> u32 {
        let life = self.hydra.life.unwrap_or(0) + grown(self.scale()) - grown(1.);
        if clone { life.min(CLONE_LIFE) } else { life }
    }

    fn hydra(&self, head: &Head) -> Spawn {
        let scale = self.scale();
        Spawn {
            layout: vec![Placement {
                frame: head.frame,
                offset: head.at,
            }],
            scale: Some([START; 2]),
            grow: Some(Grow {
                step: GROWTH * scale,
                max: Some(scale),
                ..Grow::default()
            }),
            life: Some(self.life(head.clone)),
            fade: (scale / GROWTH - EPSILON).ceil() as u32,
            trail: Some(Box::new(self.shots(head))),
            ..self.hydra.clone()
        }
    }

    /// Its first shot, then Burstfire's when rolled; Alarm_3 breathes for any hydra over scale 1.
    fn shots(&self, head: &Head) -> Trail {
        let first = self.shot() - 1;
        let burst = head.burst.then(|| Trail {
            every: ONCE,
            first: first + BURST_DELAY,
            gap: 0.,
            spawn: self.shot_of(self.scale() > 1.),
            next: None,
        });
        Trail {
            every: ONCE,
            first,
            gap: 0.,
            spawn: self.shot_of(head.breathes),
            next: burst.map(Box::new),
        }
    }

    fn shot_of(&self, breath: bool) -> Spawn {
        if breath {
            self.breath()
        } else {
            self.fire_ball()
        }
    }

    /// Step_0 L0x65-L0x77: the fire ball toward the dummy; Destroy_0 explodes on the enemy hit.
    fn fire_ball(&self) -> Spawn {
        let blast = self.ball.on_end.first().map(|blast| Spawn {
            procs: self.carried(true),
            ..blast.clone()
        });
        Spawn {
            frame: 0,
            on_end: blast.into_iter().collect(),
            ..self.ball.clone()
        }
    }

    /// Step_0 L0x79-L0x87: the Kraken's breath toward the dummy, breathing again as it fades.
    fn breath(&self) -> Spawn {
        let second = Spawn {
            offset: 0.,
            ..self.breath_with(self.carried(false))
        };
        Spawn {
            trail: Some(Box::new(Trail {
                every: ONCE,
                first: SECOND_BREATH,
                gap: 0.,
                spawn: second,
                next: None,
            })),
            ..self.breath_with(self.carried(true))
        }
    }

    fn breath_with(&self, procs: Vec<Proc>) -> Spawn {
        let bonus = self.tree.mods.size_bonus;
        Spawn {
            object: BREATH.into(),
            aim: true,
            offset: self.ball.offset,
            scale: Some(BREATH_SCALE.map(|s| s + bonus)),
            grow: Some(Grow {
                step: BREATH_GROWTH,
                x_only: true,
                ..Grow::default()
            }),
            life: Some(BREATH_LIFE),
            fade: BREATH_FADE,
            deadly: Some(BREATH_DEADLY),
            procs,
            ..Spawn::default()
        }
    }

    /// Step_0 L0x26-L0x39: Burning Essence's auras, floor(damage × 0.25r), 1 + 0.125r big.
    fn auras(&self, head: &Head) -> Option<Spawn> {
        let share = self.tree.own(ESSENCE, "of_total_damage") / 100.;
        if share <= 0. {
            return None;
        }
        let size = 1. + self.tree.own(ESSENCE, "area_of_effect") / 100.;
        let life = self.life(head.clone);
        let ages = (1..).map(|k| grown(self.scale()) + ESSENCE_EVERY * k);
        let layout = ages.take_while(|&age| age < life).map(|age| Placement {
            frame: head.frame + age,
            offset: head.at,
        });
        Some(Spawn {
            target: true,
            layout: layout.collect(),
            share: Some(share),
            procs: self.carried(true),
            ..pyro::aura(size)
        })
    }

    /// A carrier's effects in push order (340 zeroed by its hit, then 339); each skips its own.
    fn carried(&self, comets: bool) -> Vec<Proc> {
        let comets = if comets { self.comets() } else { Vec::new() };
        comets.into_iter().chain(self.nova()).collect()
    }

    fn comets(&self) -> Vec<Proc> {
        let count = self.tree.own(CALAMITY, "projectile_count").round() as u32;
        (0..count).map(|i| sure(self.comet(i))).collect()
    }

    fn comet(&self, i: u32) -> Spawn {
        let blast = Spawn {
            procs: self.nova().into_iter().collect(),
            ..pyro::blast(COMET_SCALE * COMET_BLAST)
        };
        Spawn {
            at_enemy: true,
            on_end: vec![blast],
            ..pyro::comet(COMET, COMET_SCALE, COMET_HEIGHT + COMET_STEP * i as f32)
        }
    }

    fn nova(&self) -> Option<Proc> {
        let (chance, _) = self.tree.proc(SCORCHING_BLAST, "")?;
        let spawn = Spawn {
            at_enemy: true,
            ..pyro::firenova(NOVA_MAX)
        };
        Some(Proc {
            chance,
            spawn,
            pull: None,
            once: false,
        })
    }
}

/// Frames a hydra takes to grow from 0.1 to `scale`.
fn grown(scale: f32) -> u32 {
    ((scale - START) / (GROWTH * scale) - EPSILON).ceil() as u32
}

/// distance_to_object from a hydra `scale` big at `pos` to the player at the origin.
fn gap(pos: [f32; 2], scale: f32) -> f32 {
    pyro::player_gap(HYDRA_BOX.map(|edge| edge * scale), pos)
}

#[cfg(test)]
#[path = "hydra_parity_tests.rs"]
mod parity;
#[cfg(test)]
#[path = "hydra_tests.rs"]
mod tests;
