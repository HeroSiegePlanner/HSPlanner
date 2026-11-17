//! Pyromancer's Comet: TalentsPyromancer case 34 rolls the cast and drops its comets, each
//! Pyromancer_Comet_obj's Step_0 leaves its landing effects and bounces, and its Destroy_0 leaves
//! the explosion and Searing Shrapnel's pieces (Sep 16 exe).
use super::{Tree, pyro};
use crate::skill_effects::Dice;
use crate::skill_sim::{Fall, Placement, Spawn};

const SKILL: &str = "comet";
const BLAST_RADIUS: &str = "blast_radius";
const FRAGMENTS: &str = "comet_fragments";
const AFTERSHOCK: &str = "molten_aftershock";
const SEARING: &str = "searing_shrapnel";
const TERRAIN: &str = "flaming_terrain";
const FIERY: &str = "fiery_shower";
const STARFALL: &str = "molten_starfall";
const BOUNCING: &str = "bouncing_comets";
const CRUSHING: &str = "crushing_comet";
/// Casts replayed, so the cast's rolls show their odds.
pub(super) const CASTS: u64 = 8;
const COMET: &str = "Pyromancer_Comet_obj";
/// L0x3f0, L0x415-L0x41d: Molten Starfall's comets are 0.33 big, all but the first on a 96 px ring.
const STARFALL_SCALE: f32 = 0.33;
const RING: f32 = 96.;
/// L0x402, L0x420-L0x428: bouncing comets are half size, 54 px out on a 54° fan, with 3 bounces.
const BOUNCING_SCALE: f32 = 0.5;
const FAN: f32 = 54.;
const BOUNCES: u32 = 3;
/// L0x3fb: Crushing Comet's gravityPower.
const CRUSHING_GRAVITY: f32 = 0.5;
/// Step_0 L0x2f-L0x37: a bounce sets yy 8 and zSpeed 2 × (1 + bounces left), hops 4 px a frame.
const HOP_HEIGHT: f32 = 8.;
const HOP_LIFT: f32 = 2.;
const HOP_SPEED: f32 = 4.;
/// Step_0 L0x32: each later bounce hops 0.75 times as fast.
const HOP_SLOWDOWN: f32 = 0.75;
/// Destroy_0 L0xe: the explosion is image_xscale × 2.2 big.
const BLAST_SCALE: f32 = 2.2;
/// Step_0 L0x39-L0x61: a bounce's is × 3.5 as Hit_Impact_Fire_spr: its look, mask and 8 frames.
const BOUNCE_BLAST: &str = "Explosion_Item_obj:Hit_Impact_Fire_spr";
const BOUNCE_BLAST_SCALE: f32 = 3.5;
const BOUNCE_BLAST_LIFE: u32 = 21;
/// Step_0 L0x1b-L0x23: 3 fragments 0.25 big from yy 260 + 60 × i, each with a third of the damage.
const FRAGMENT_COUNT: u32 = 3;
const FRAGMENT_SCALE: f32 = 0.25;
const FRAGMENT_HEIGHT: f32 = 260.;
const FRAGMENT_STEP: f32 = 60.;
/// Step_0 L0xf: Molten Aftershock's Firenova grows to 0.9.
const AFTERSHOCK_MAX: f32 = 0.9;
/// Destroy_0 L0x1f-L0x39: a piece per fragment frame, 8 px out 45° apart, with x = 0, 0.5, 1, 0…:
/// speed 2 + x, zSpeed 1 + x, yy 12 + 4x, all × (1 + range); Shrapnel Create_0: gravity 0.15.
const SHRAPNEL: &str = "Marksman_Frag_Grenade_Shrapnel_obj";
const SHRAPNEL_COUNT: u32 = 8;
const SHRAPNEL_OUT: f32 = 8.;
const SHRAPNEL_SPEED: f32 = 2.;
const SHRAPNEL_LIFT: f32 = 1.;
const SHRAPNEL_HEIGHT: f32 = 12.;
const SHRAPNEL_RISE: f32 = 4.;
const SHRAPNEL_STEP: f32 = 0.5;
const SHRAPNEL_STEPS: u32 = 3;
const SHRAPNEL_GRAVITY: f32 = 0.15;

/// What a cast rolls before its comets exist (L0x3ce-L0x3e4), each roll its own draw; Stunning
/// Hit's roll (L0x3e0) only stuns.
struct Rolls {
    shrapnel: bool,
    fragments: bool,
    aftershock: bool,
    terrain: bool,
}

fn rolls(tree: &Tree, mut roll: impl FnMut(&str, f32) -> bool) -> Rolls {
    let mut lucky = |node: &str| {
        let chance = tree.proc(node, "").map(|(chance, _)| chance);
        chance.is_some_and(|chance| roll(node, chance))
    };
    Rolls {
        shrapnel: lucky(SEARING),
        fragments: lucky(FRAGMENTS),
        aftershock: lucky(AFTERSHOCK),
        terrain: lucky(TERRAIN),
    }
}

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    let mut dice = Dice::of_cast(tree.cast);
    cast(spawns, tree, rolls(tree, |_, chance| dice.roll(chance)))
}

/// A cast whose roll for each node comes up as `lucky` says.
#[cfg(test)]
pub(super) fn apply_rolled(
    spawns: &[Spawn],
    tree: &Tree,
    lucky: &dyn Fn(&str) -> bool,
) -> Vec<Spawn> {
    cast(spawns, tree, rolls(tree, |node, _| lucky(node)))
}

fn cast(spawns: &[Spawn], tree: &Tree, rolls: Rolls) -> Vec<Spawn> {
    let [comet] = spawns else {
        return spawns.to_vec();
    };
    let cast = Cast::new(comet, tree, rolls);
    if !cast.transformed() {
        return vec![cast.shower()];
    }
    let count = cast.count();
    (0..count).map(|i| cast.comet(i, count)).collect()
}

/// One cast: its rolls, the base comet, its damage, size, gravity and direction.
struct Cast<'a> {
    tree: &'a Tree<'a>,
    rolls: Rolls,
    base: &'a Spawn,
    /// floor(LTD × (1 + LSSD)) over the engine's hit; a lucky Molten Aftershock adds to LSSD.
    share: f32,
    /// image_xscale: 1 × (1 + area) × Starfall's and Bouncing's (L0x3fe), with the size bonus.
    scale: f32,
    gravity: f32,
    cursor: [f32; 2],
    /// point_direction from the player to the cursor point, every comet's direction (L0x433).
    aim: f32,
}

impl<'a> Cast<'a> {
    fn new(base: &'a Spawn, tree: &'a Tree<'a>, rolls: Rolls) -> Self {
        let subtree = 100. + tree.stat("subtree_damage");
        let aftershock = tree
            .proc(AFTERSHOCK, "fire_skill_damage")
            .filter(|_| rolls.aftershock)
            .map_or(0., |(_, damage)| damage);
        let area = tree.own(BLAST_RADIUS, "area_of_effect") + tree.own(CRUSHING, "projectile_size");
        let shrink = |node: &str, scale: f32| if tree.learned(node) { scale } else { 1. };
        let scale = base.scale.map_or(1., |[x, _]| x)
            * (1. + area / 100.)
            * shrink(STARFALL, STARFALL_SCALE)
            * shrink(BOUNCING, BOUNCING_SCALE);
        let gravity = if tree.learned(CRUSHING) {
            CRUSHING_GRAVITY
        } else {
            base.fall.map_or(0., |fall| fall.gravity)
        };
        let cursor = pyro::cursor(SKILL);
        Self {
            tree,
            rolls,
            base,
            share: (subtree + aftershock) / subtree,
            scale: scale + tree.mods.size_bonus,
            gravity,
            cursor,
            aim: pyro::toward([0., 0.], cursor),
        }
    }

    fn transformed(&self) -> bool {
        self.tree.learned(STARFALL) || self.tree.learned(BOUNCING)
    }

    /// L0x3ea, L0x40f-L0x412: the comets and Fiery Shower's 2 × rank more, all on the cursor point
    /// from yy 320 + 80 × i.
    fn shower(&self) -> Spawn {
        Spawn {
            extra: self.tree.own(FIERY, "projectile_count").round() as u32,
            scale: Some([self.scale; 2]),
            fall: Some(self.fall()),
            on_end: self.landed(self.cursor, 0),
            ..self.base.clone()
        }
    }

    /// The base comet's fall under the cast's gravity.
    fn fall(&self) -> Fall {
        Fall {
            gravity: self.gravity,
            ..self.base.fall.unwrap_or_default()
        }
    }

    /// L0x3ee-L0x3f5: Molten Starfall's 4 × rank comets, or Bouncing Comets' 2 × rank instead.
    fn count(&self) -> u32 {
        let node = if self.tree.learned(BOUNCING) {
            BOUNCING
        } else {
            STARFALL
        };
        self.tree.own(node, "projectile_count").round() as u32
    }

    /// L0x415-L0x425: comet i of `count` from the cursor point, ringed, then fanned.
    fn offset(&self, i: u32, count: u32) -> [f32; 2] {
        let gaps = count.saturating_sub(1);
        let ring = if self.tree.learned(STARFALL) && i > 0 {
            pyro::lengthdir(RING, 360. / gaps.max(1) as f32 * i as f32)
        } else {
            [0., 0.]
        };
        let turn = FAN * (i as f32 - gaps as f32 / 2.);
        let fan = if self.tree.learned(BOUNCING) {
            pyro::lengthdir(FAN, self.aim + turn)
        } else {
            [0., 0.]
        };
        [ring[0] + fan[0], ring[1] + fan[1]]
    }

    /// Transformed comet i of `count`, from yy 320 wherever it lands.
    fn comet(&self, i: u32, count: u32) -> Spawn {
        let offset = self.offset(i, count);
        let at = [self.cursor[0] + offset[0], self.cursor[1] + offset[1]];
        let bounces = if self.tree.learned(BOUNCING) {
            BOUNCES
        } else {
            0
        };
        Spawn {
            layout: vec![Placement { frame: 0, offset }],
            scale: Some([self.scale; 2]),
            fall: Some(Fall {
                height_step: 0.,
                ..self.fall()
            }),
            on_end: self.landed(at, bounces),
            ..self.base.clone()
        }
    }

    /// Step_0 where a cast comet lands at `at`: its effects, then a bounce hopping on, or once out
    /// of bounces Destroy_0's explosion and shrapnel.
    fn landed(&self, at: [f32; 2], bounces: u32) -> Vec<Spawn> {
        let rest = if bounces == 0 {
            let blast = self.blast(BLAST_SCALE * self.scale, self.share);
            std::iter::once(blast).chain(self.shrapnel(at)).collect()
        } else {
            let blast = Spawn {
                object: BOUNCE_BLAST.into(),
                life: Some(BOUNCE_BLAST_LIFE),
                ..self.blast(BOUNCE_BLAST_SCALE * self.scale, self.share)
            };
            vec![blast, self.hop(at, bounces)]
        };
        let effects = self.effects(self.share, true);
        effects.into_iter().chain(rest).collect()
    }

    /// Step_0 L0x2d-L0x37: off along the comets' direction from `from`, `bounces` left.
    fn hop(&self, from: [f32; 2], bounces: u32) -> Spawn {
        let speed = HOP_SPEED * HOP_SLOWDOWN.powi((BOUNCES - bounces) as i32);
        let fall = Fall {
            height: HOP_HEIGHT,
            z_speed: HOP_LIFT * (1 + bounces) as f32,
            gravity: self.gravity,
            height_step: 0.,
        };
        let [dx, dy] = pyro::lengthdir(speed * pyro::landing(fall) as f32, self.aim);
        Spawn {
            turn: Some(self.aim - pyro::toward([0., 0.], from)),
            speed,
            fall: Some(fall),
            on_end: self.landed([from[0] + dx, from[1] + dy], bounces - 1),
            ..pyro::comet(COMET, self.scale, HOP_HEIGHT)
        }
    }

    /// Step_0 L0x4-L0x28 where a comet worth `share` lands: Flaming Terrain's patch, Molten
    /// Aftershock's Firenova, then the fragments, which inherit all but their own effect (351).
    fn effects(&self, share: f32, fragments: bool) -> Vec<Spawn> {
        let terrain = self.tree.proc(TERRAIN, "fire_skill_damage");
        let patch = terrain
            .filter(|_| self.rolls.terrain)
            .map(|(_, damage)| pyro::patch(self.tree.mods, share * damage / 100.));
        let nova = self.rolls.aftershock.then(|| Spawn {
            share: Some(share),
            ..pyro::firenova(AFTERSHOCK_MAX)
        });
        let fragments = (fragments && self.rolls.fragments).then(|| self.fragments(share));
        let fragments = fragments.into_iter().flatten();
        patch.into_iter().chain(nova).chain(fragments).collect()
    }

    fn fragments(&self, share: f32) -> Vec<Spawn> {
        let share = share / FRAGMENT_COUNT as f32;
        let blast = self.blast(BLAST_SCALE * FRAGMENT_SCALE, share);
        let landed: Vec<Spawn> = self
            .effects(share, false)
            .into_iter()
            .chain([blast])
            .collect();
        let fragment = |i: u32| Spawn {
            on_end: landed.clone(),
            ..pyro::comet(
                COMET,
                FRAGMENT_SCALE,
                FRAGMENT_HEIGHT + FRAGMENT_STEP * i as f32,
            )
        };
        (0..FRAGMENT_COUNT).map(fragment).collect()
    }

    fn blast(&self, scale: f32, share: f32) -> Spawn {
        Spawn {
            share: Some(share),
            ..pyro::blast(scale)
        }
    }

    /// Searing Shrapnel's pieces from `at`; they stop and shrink where they land (Step_0 L5-L10),
    /// which the preview leaves out.
    fn shrapnel(&self, at: [f32; 2]) -> Vec<Spawn> {
        let searing = self.tree.proc(SEARING, "fire_skill_damage");
        let Some((_, damage)) = searing.filter(|_| self.rolls.shrapnel) else {
            return Vec::new();
        };
        let range = self
            .tree
            .proc(SEARING, "skill_range")
            .map_or(0., |(_, r)| r);
        let range = 1. + range / 100.;
        let from = pyro::toward([0., 0.], at);
        let piece = |i: u32| {
            let x = (i % SHRAPNEL_STEPS) as f32 * SHRAPNEL_STEP;
            Spawn {
                object: SHRAPNEL.into(),
                turn: Some(360. / SHRAPNEL_COUNT as f32 * i as f32 - from),
                offset: SHRAPNEL_OUT,
                speed: (SHRAPNEL_SPEED + x) * range,
                fall: Some(Fall {
                    height: (SHRAPNEL_HEIGHT + SHRAPNEL_RISE * x) * range,
                    height_step: 0.,
                    z_speed: (SHRAPNEL_LIFT + x) * range,
                    gravity: SHRAPNEL_GRAVITY,
                }),
                share: Some(self.share * damage / 100.),
                ..Spawn::default()
            }
        };
        (0..SHRAPNEL_COUNT).map(piece).collect()
    }
}

#[cfg(test)]
#[path = "comet_tests.rs"]
mod tests;
