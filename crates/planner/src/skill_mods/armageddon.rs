//! Pyromancer's Armageddon: TalentsPyromancer case 37 sets the cast up and rolls its recast; with
//! Blistering Bolt it drops one meteor per enemy near instead of any rain, else its controller
//! rains Armageddon_obj or, with Warped Doom, sends Armageddon_Horizontal_obj along the aim. A
//! meteor's Alarm_0 rolls Ball of Fire, its landing or Destroy_0 Blazing Pulse (Sep 16 exe).
use super::{Tree, pyro};
use crate::skill_effects::Dice;
use crate::skill_sim::{Placement, Spawn};

const SKILL: &str = "armageddon";
const FALLING: &str = "falling_wrath";
const STORM: &str = "storm_of_fire";
const COLOSSAL: &str = "colossal_chunks";
const MADNESS: &str = "pyrokinetic_madness";
const BALL: &str = "ball_of_fire";
const PULSE: &str = "blazing_pulse";
const CONCENTRATION: &str = "searing_concentration";
const SHOWER: &str = "warm_shower";
const BOLT: &str = "blistering_bolt";
const WARPED: &str = "warped_doom";
/// Nodes that change the rain; without them a cast rains the base use's meteors.
const RAIN_NODES: [&str; 7] = [FALLING, STORM, COLOSSAL, BALL, PULSE, CONCENTRATION, SHOWER];
/// Casts replayed, so the cast's rolls show their odds.
pub(super) const CASTS: u64 = 8;
/// L0x56b-L0x56c, L0x5ab: skillRadius 340 px and skillAmount 40 volleys.
const RADIUS: f32 = 340.;
const VOLLEYS: u32 = 40;
const FULL_TURN: u32 = 360;
/// Controller Step_0 L0x9-L0x24: volleys from its first Step every 0.075 s; Warped's every 0.05 s.
const FIRST_VOLLEY: u32 = 1;
const RAIN_EVERY: u32 = 6;
const WARPED_EVERY: u32 = 4;
/// Controller Step_0 L4-L6: skillFollow keeps it at the player's x and 6 px under its y.
const FOLLOW_BELOW: f32 = 6.;
/// The unseen controllers and what one of their Steps drops.
const CONTROLLER: &str = "Pyromancer_Armageddon_Controller_obj";
const WARPED_CONTROLLER: &str = "Pyromancer_Armageddon_Warped_Controller_obj";
const DROP: &str = "Pyromancer_Armageddon_Controller_obj_Step_0";
/// L0x5b7-L0x5b9: collision_circle_list, not precise, so it meets the dummy's 24 px box.
const BOLT_OUT: f32 = 48.;
const DUMMY_BOX: f32 = 12.;
/// Armageddon_obj Step_0 L0x23 and Horizontal Destroy_0 L0x1f: the nova grows to maxScale 0.75.
const NOVA_MAX: f32 = 0.75;
/// Horizontal Create_0 L0x22-L0x23: 10 px a frame for 3 s; Warped Step_0 L0xd: made 280 px back.
const HORIZONTAL: &str = "Pyromancer_Armageddon_Horizontal_obj";
const HORIZONTAL_SPEED: f32 = 10.;
const HORIZONTAL_LIFE: f32 = 180.;
const BEHIND: f32 = 280.;
const HORIZONTAL_BLAST: &str = "Explosion_Item_obj:Pyromancer_Explosion_spr";
/// StepDamageParentCheckCollisions L0x62-L0x70: projType 2 stops on the enemy; alarm[10] = 3.
const STOP_REACH: f32 = 12.;
const DESTROY_DELAY: u32 = 2;
/// A miss is followed 900 px, out of the 1280 px view; the game explodes it unseen 1800 px out.
const SEEN: u32 = 90;

pub(super) fn apply(spawns: &[Spawn], tree: &Tree) -> Vec<Spawn> {
    let [meteor] = spawns else {
        return spawns.to_vec();
    };
    let cast = Cast::new(meteor, tree);
    let mut dice = Dice::of_cast(tree.cast);
    // L0x59d: irandom(99) < 15 × rank, and the recast itself never rolls it.
    let recast = tree
        .proc(MADNESS, "")
        .is_some_and(|(chance, _)| dice.roll(chance));
    let first = cast.at(0, Some(&meteor.layout), &mut dice);
    let copy = recast
        .then(|| cast.at(pyro::DOUBLE_CAST_DELAY, None, &mut dice))
        .flatten();
    first.into_iter().chain(copy).collect()
}

/// One cast: what case 37 reads from the subtree (L0x57f-L0x5a8) for every meteor it makes.
struct Cast<'a> {
    tree: &'a Tree<'a>,
    base: &'a Spawn,
    /// L0x580-L0x5a8: L0x596 adds Blistering Bolt's area again for Warped Doom, not its own.
    radius: f32,
    /// skillProjectiles: meteors a volley (L0x57f).
    volley: u32,
    /// skillSize (L0x581) plus the size bonus parent Alarm_11 adds to image_xscale.
    scale: f32,
    /// Ball of Fire's chance, damage and size (L0x585-L0x587).
    ball: Option<(f32, f32, f32)>,
    /// Blazing Pulse's chance and damage (L0x588-L0x589).
    pulse: Option<(f32, f32)>,
}

impl<'a> Cast<'a> {
    fn new(base: &'a Spawn, tree: &'a Tree<'a>) -> Self {
        let part = |node: &str, key: &str| tree.own(node, key) / 100.;
        let bolt = part(BOLT, "area_of_effect");
        let slip = if tree.learned(WARPED) { bolt } else { 0. };
        let area = part(STORM, "area_of_effect") + bolt + slip;
        let ball = tree.proc(BALL, "fire_skill_damage");
        let size = tree.proc(BALL, "projectile_size");
        Self {
            tree,
            base,
            radius: RADIUS * (1. + area) * (1. + part(CONCENTRATION, "area_of_effect")),
            volley: 1 + tree.own(FALLING, "projectile_count").round() as u32,
            scale: 1. + part(COLOSSAL, "projectile_size") + tree.mods.size_bonus,
            ball: ball
                .zip(size)
                .map(|((chance, damage), (_, size))| (chance, damage, size)),
            pulse: tree.proc(PULSE, "fire_skill_damage"),
        }
    }

    /// L0x5b6 and L0x5d2: Blistering Bolt's meteor breaks out of the case before any controller.
    fn at(&self, frame: u32, layout: Option<&[Placement]>, dice: &mut Dice) -> Option<Spawn> {
        if self.tree.learned(BOLT) {
            return self.bolt(frame, dice);
        }
        if self.tree.learned(WARPED) {
            return Some(self.warped(frame, dice));
        }
        match layout {
            Some(_) if self.plain() => Some(self.base.clone()),
            _ => Some(self.rain(frame, layout, dice)),
        }
    }

    fn plain(&self) -> bool {
        let learned = RAIN_NODES.iter().any(|node| self.tree.learned(node));
        !learned && self.tree.mods.size_bonus == 0.
    }

    /// Where the rain falls around: the cursor point, or under the player with Warm Shower.
    fn centre(&self) -> [f32; 2] {
        if self.tree.learned(SHOWER) {
            [0., FOLLOW_BELOW]
        } else {
            pyro::cursor(SKILL)
        }
    }

    /// Controller Step_0 L0xa-L0x22; a volley's first meteor lands where the base use's does.
    fn rain(&self, frame: u32, layout: Option<&[Placement]>, dice: &mut Dice) -> Spawn {
        let centre = self.centre();
        let stretch = self.radius / RADIUS;
        let volleys = (0..VOLLEYS)
            .map(|i| {
                let first = layout
                    .and_then(|layout| layout.get(i as usize))
                    .map(|placed| placed.offset.map(|v| v * stretch));
                let random = std::iter::repeat_with(|| self.scatter(dice));
                let spots: Vec<[f32; 2]> = first
                    .into_iter()
                    .chain(random)
                    .take(self.volley as usize)
                    .collect();
                let meteors = spots
                    .into_iter()
                    .map(|spot| self.falling(centre, spot, self.lucky(dice)))
                    .collect();
                (FIRST_VOLLEY + RAIN_EVERY * i, meteors)
            })
            .collect();
        controller(CONTROLLER, centre, frame, volleys)
    }

    fn scatter(&self, dice: &mut Dice) -> [f32; 2] {
        let heading = pyro::irandom(dice, FULL_TURN);
        pyro::lengthdir(pyro::irandom(dice, self.radius as u32), heading)
    }

    /// A meteor made `spot` px from a drop at `centre`.
    fn falling(&self, centre: [f32; 2], spot: [f32; 2], lucky: bool) -> Spawn {
        Spawn {
            turn: Some(pyro::toward([0., 0.], spot) - pyro::toward([0., 0.], centre)),
            offset: spot[0].hypot(spot[1]),
            ..self.meteor(lucky)
        }
    }

    /// Step_0 L0x4-L0x15: it lands in an explosion as big as it, dealing what it deals.
    fn meteor(&self, lucky: bool) -> Spawn {
        let (scale, share) = self.ball(lucky);
        let blast = self.base.on_end.first().cloned();
        let blast = blast.unwrap_or_else(|| pyro::blast(1.));
        let blast = Spawn {
            scale: Some([scale; 2]),
            share: (share > 1.).then_some(share),
            ..blast
        };
        let height = self.base.fall.map_or(0., |fall| fall.height);
        Spawn {
            scale: Some([scale; 2]),
            ..pyro::falling(height, self.ends(share, blast, 0))
        }
    }

    /// Alarm_0 L0xa-L0xb: irandom(99) < Ball of Fire's chance.
    fn lucky(&self, dice: &mut Dice) -> bool {
        self.ball.is_some_and(|(chance, ..)| dice.roll(chance))
    }

    /// Alarm_0 L0xc-L0xe: a ball of fire is bigger and deals more; its scale and share.
    fn ball(&self, lucky: bool) -> (f32, f32) {
        match self.ball {
            Some((_, damage, size)) if lucky => (self.scale + size / 100., 1. + damage / 100.),
            _ => (self.scale, 1.),
        }
    }

    /// Its explosion and Blazing Pulse's nova, which Alarm_11 also grows by the size bonus.
    fn ends(&self, share: f32, blast: Spawn, delay: u32) -> Vec<Spawn> {
        let nova = self.pulse.map(|(chance, damage)| Spawn {
            frame: delay,
            chance: Some(chance),
            share: Some(share * (1. + damage / 100.)),
            ..pyro::firenova(NOVA_MAX + self.tree.mods.size_bonus)
        });
        let blast = Spawn {
            frame: delay,
            ..blast
        };
        std::iter::once(blast).chain(nova).collect()
    }

    /// L0x5be: a meteor on the dummy if the circle finds it.
    fn bolt(&self, frame: u32, dice: &mut Dice) -> Option<Spawn> {
        self.found().then(|| Spawn {
            layout: vec![Placement {
                frame,
                offset: dummy(),
            }],
            ..self.meteor(self.lucky(dice))
        })
    }

    /// L0x5b7-L0x5b9: whether the dummy's box meets floor(skillRadius / 2) px 48 px out.
    fn found(&self) -> bool {
        let [x, y] = pyro::lengthdir(BOLT_OUT, pyro::toward([0., 0.], pyro::cursor(SKILL)));
        let dummy = dummy();
        let box_x = x.clamp(dummy[0] - DUMMY_BOX, dummy[0] + DUMMY_BOX);
        let box_y = y.clamp(dummy[1] - DUMMY_BOX, dummy[1] + DUMMY_BOX);
        (box_x - x).hypot(box_y - y) <= (self.radius * 0.5).floor()
    }

    /// Warped_Controller_obj Step_0 never reads skillFollow, so it stays on the cursor point.
    fn warped(&self, frame: u32, dice: &mut Dice) -> Spawn {
        let cursor = pyro::cursor(SKILL);
        let volleys = (0..VOLLEYS)
            .map(|i| {
                let meteors = (0..self.volley)
                    .map(|_| self.across(cursor, dice))
                    .collect();
                (FIRST_VOLLEY + WARPED_EVERY * i, meteors)
            })
            .collect();
        controller(WARPED_CONTROLLER, cursor, frame, volleys)
    }

    /// Step_0 L0x9-L0xb: skillRadius / 2 − irandom(skillRadius) px across the aim.
    fn across(&self, cursor: [f32; 2], dice: &mut Dice) -> Spawn {
        let off = self.radius * 0.5 - pyro::irandom(dice, self.radius as u32);
        let lucky = self.lucky(dice);
        Spawn {
            turn: Some(90.),
            offset: off,
            on_end: vec![self.horizontal(side(cursor, off), lucky)],
            ..carrier(Vec::new())
        }
    }

    /// A damage object hitting what it touches once (singleHit); Alarm_0 may make it a ball of fire.
    fn horizontal(&self, side: [f32; 2], lucky: bool) -> Spawn {
        let (scale, share) = self.ball(lucky);
        let aim = pyro::toward([0., 0.], pyro::cursor(SKILL));
        let [x, y] = pyro::lengthdir(-BEHIND, aim);
        let stops = stop([side[0] + x, side[1] + y], aim);
        let blast = Spawn {
            object: HORIZONTAL_BLAST.into(),
            share: (share > 1.).then_some(share),
            ..pyro::blast(scale)
        };
        let stretched = HORIZONTAL_LIFE * (1. + self.tree.mods.duration_pct / 100.);
        let missed = (stretched.ceil() as u32).min(SEEN);
        Spawn {
            object: HORIZONTAL.into(),
            turn: Some(aim - pyro::toward([0., 0.], side)),
            offset: -BEHIND,
            speed: HORIZONTAL_SPEED,
            scale: Some([scale; 2]),
            share: (share > 1.).then_some(share),
            life: Some(stops.map_or(missed, |moves| moves + 1)),
            on_end: stops.map_or_else(Vec::new, |_| self.ends(share, blast, DESTROY_DELAY)),
            ..Spawn::default()
        }
    }
}

/// Moves after which a meteor flying the aim from `start` has its centre on the dummy, if ever.
fn stop(start: [f32; 2], aim: f32) -> Option<u32> {
    let dummy = dummy();
    (1..=SEEN).find(|&moves| {
        let [x, y] = pyro::lengthdir(HORIZONTAL_SPEED * moves as f32, aim);
        (start[0] + x - dummy[0]).hypot(start[1] + y - dummy[1]) <= STOP_REACH
    })
}

/// An unseen controller at `at` from `frame`, dropping each volley on its move.
fn controller(object: &str, at: [f32; 2], frame: u32, volleys: Vec<(u32, Vec<Spawn>)>) -> Spawn {
    let last = volleys.last().map_or(0, |(moves, _)| *moves);
    let drops = volleys
        .into_iter()
        .map(|(moves, items)| (moves, carrier(items)));
    let trail = pyro::once_each(drops);
    Spawn {
        object: object.into(),
        ability: true,
        layout: vec![Placement { frame, offset: at }],
        life: Some(last + 1),
        trail,
        ..Spawn::default()
    }
}

fn dummy() -> [f32; 2] {
    [pyro::cursor(SKILL)[0], 0.]
}

/// The point `off` px across the aim from the cursor point.
fn side(cursor: [f32; 2], off: f32) -> [f32; 2] {
    let [x, y] = pyro::lengthdir(off, pyro::toward([0., 0.], cursor) + 90.);
    [cursor[0] + x, cursor[1] + y]
}

/// The unseen object making `items` where it is.
fn carrier(items: Vec<Spawn>) -> Spawn {
    Spawn {
        object: DROP.into(),
        ability: true,
        life: Some(0),
        on_end: items,
        ..Spawn::default()
    }
}

#[cfg(test)]
#[path = "armageddon_parity_tests.rs"]
mod parity;
#[cfg(test)]
#[path = "armageddon_tests.rs"]
mod tests;
