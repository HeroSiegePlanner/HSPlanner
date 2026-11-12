//! The game's dummy hits for the engine's parity tests: every patch's fork heading enumerated at
//! its odds, the patches' rolls counted at theirs and every object replayed apart, so the drawn
//! preview's 256-object cap cuts nothing.
use super::{BELOW, Cast, FORK_SPREAD, FORK_TURN, FULL_TURN, Leaves, SKILL, WALK, drops, pyro};
use crate::skill_mods::parity_tests as parity;
use crate::skill_sim::{Placement, Spawn, Timeline};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

const CLASS: &str = "pyromancer";
const PATCH: &str = "Pyromancer_Blazing_Trail_obj";
const NOVA: &str = "Pyromancer_Firenova_obj";
/// px inside a round mask's reach where it surely meets the dummy's 12 px disk.
const SURE_MARGIN: f32 = 3.;

/// Damage-weighted hits the game lands on the dummy in one use with `ranks`: every fork heading
/// and roll at its odds and every object counted apart. Forking Flames' 1 + 45 % × rank is left
/// out: the engine adds its fire damage to every hit instead.
pub(super) fn uncapped(ranks: &[(&str, u32)]) -> f32 {
    let mods = parity::mods(SKILL, ranks);
    let tree = crate::skill_mods::Tree {
        nodes: parity::nodes(CLASS, SKILL),
        mods: &mods,
        cast: 0,
    };
    let cast = Cast::new(&tree);
    if cast.rays == 0 {
        let made = parity::rewritten(CLASS, SKILL, ranks, 0, super::apply);
        return parity::weighted(&play(&made));
    }
    let mut game = Game::new(&cast);
    let headings = (0..cast.rays).map(|i| FULL_TURN / cast.rays as f32 * i as f32);
    headings.map(|heading| game.ray(heading)).sum()
}

/// One cast's objects counted apart, with what they land where.
struct Game<'a> {
    cast: &'a Cast<'a>,
    /// A patch's own ticks at a spot, by its spot and scale.
    ticks: HashMap<[u32; 3], f32>,
    /// Pulses a crackling patch makes, and the distances within which a pulse surely hits the
    /// dummy and beyond which it cannot.
    pulses: f32,
    nova: (f32, f32),
}

impl<'a> Game<'a> {
    fn new(cast: &'a Cast<'a>) -> Self {
        let far = [pyro::cursor(SKILL)[0], -4000.];
        let crackling = play(&[at(far, cast.sized(1., 1., true, None))]);
        Self {
            cast,
            ticks: HashMap::new(),
            pulses: births(&crackling, NOVA) as f32,
            nova: bounds(&cast.pulse(1.)),
        }
    }

    fn ray(&mut self, heading: f32) -> f32 {
        let cast = self.cast;
        let spots = drops(heading, cast.scale, cast.timer)
            .into_iter()
            .map(|moves| {
                let [x, y] = pyro::lengthdir(WALK * moves as f32, heading);
                [x, y + BELOW]
            });
        spots.map(|spot| self.dropped(spot)).sum()
    }

    /// What a controller's drop at `spot` lands (Step_0 L0xe-L0x13).
    fn dropped(&mut self, spot: [f32; 2]) -> f32 {
        let (cast, bonus) = (self.cast, self.cast.bonus());
        match cast.leaves {
            Leaves::Patch => self.patch(spot, cast.scale + bonus),
            Leaves::Comet => {
                let comet = cast.comet(spot, &mut crate::skill_effects::Dice::of_cast(0));
                let blast = comet.on_end[0].clone();
                lone(spot, blast, 140.) + self.patch(spot, cast.scale + 2. * bonus)
            }
            Leaves::Mine => lone(spot, cast.mine(spot), 150.),
        }
    }

    /// A patch's hits and, at Forking Flames' odds, its forks' over their 181 headings.
    fn patch(&mut self, spot: [f32; 2], max: f32) -> f32 {
        let plain = self.alone(spot, max);
        let Some((chance, _)) = self.cast.fork else {
            return plain;
        };
        let key = format!("{spot:?} {max} {}", self.key());
        let [ticks, pulses] = cached(key, || {
            let back = pyro::toward(spot, [0., 0.]) - FORK_TURN;
            let headings = (0..=FORK_SPREAD).map(|turn| back + turn as f32);
            let all = headings.fold([0.; 2], |[t, p], heading| {
                let [ticks, pulses] = self.forks(spot, max, heading);
                [t + ticks, p + pulses]
            });
            all.map(|sum| sum / (FORK_SPREAD + 1) as f32)
        });
        plain + chance * (ticks + self.cast.crackle * pulses)
    }

    /// What a patch's hits depend on besides its spot, its scale and its Crackling Heat roll.
    fn key(&self) -> String {
        let cast = self.cast;
        let life = cast.tree.mods.duration_pct;
        format!("{} {} {life}", cast.tick, cast.bonus())
    }

    /// The ten forks along `heading`, each counted alone: their ticks and, were they all
    /// crackling, their pulses.
    fn forks(&mut self, spot: [f32; 2], max: f32, heading: f32) -> [f32; 2] {
        let spots = self.cast.fork_spots(spot, max, heading);
        spots.into_iter().fold([0.; 2], |[t, p], (fork, max)| {
            let [ticks, pulses] = self.split(fork, max);
            [t + ticks, p + pulses]
        })
    }

    /// One patch's ticks and, at Crackling Heat's odds, its pulses.
    fn alone(&mut self, spot: [f32; 2], max: f32) -> f32 {
        let [ticks, pulses] = self.split(spot, max);
        ticks + self.cast.crackle * pulses
    }

    fn split(&mut self, spot: [f32; 2], max: f32) -> [f32; 2] {
        [self.ticks(spot, max), self.pulses * self.pulsed(spot)]
    }

    fn ticks(&mut self, spot: [f32; 2], max: f32) -> f32 {
        let reach = reach_of(PATCH) * max + 13.;
        if from(spot) > reach {
            return 0.;
        }
        let key = [spot[0].to_bits(), spot[1].to_bits(), max.to_bits()];
        if let Some(known) = self.ticks.get(&key) {
            return *known;
        }
        let patch = self.cast.sized(max, 1., false, None);
        let hits = parity::weighted(&play(&[at(spot, patch)]));
        self.ticks.insert(key, hits);
        hits
    }

    /// Whether a pulse from `spot` meets the dummy: sure or out of reach by distance, else
    /// replayed.
    fn pulsed(&self, spot: [f32; 2]) -> f32 {
        let (sure, reach) = self.nova;
        let off = from(spot);
        if off <= sure {
            1.
        } else if off > reach {
            0.
        } else {
            parity::weighted(&play(&[at(spot, self.cast.pulse(1.))]))
        }
    }
}

/// Expectations already worked out, by what they depend on.
fn cached(key: String, work: impl FnOnce() -> [f32; 2]) -> [f32; 2] {
    static KNOWN: LazyLock<Mutex<HashMap<String, [f32; 2]>>> = LazyLock::new(Default::default);
    if let Some(known) = KNOWN.lock().expect("cache").get(&key) {
        return *known;
    }
    let value = work();
    KNOWN.lock().expect("cache").insert(key, value);
    value
}

/// `spawn`'s hits at `spot` when the dummy is within `reach` of it.
fn lone(spot: [f32; 2], spawn: Spawn, reach: f32) -> f32 {
    if from(spot) > reach {
        return 0.;
    }
    parity::weighted(&play(&[at(spot, spawn)]))
}

/// `spawn` made at `spot` on the use's first frame.
fn at(spot: [f32; 2], spawn: Spawn) -> Spawn {
    Spawn {
        layout: vec![Placement {
            frame: 0,
            offset: spot,
        }],
        ..spawn
    }
}

/// Distances from the dummy within which `object` surely hits it and beyond which it cannot: its
/// solid round mask at the widest it gets while it can hit.
fn bounds(object: &Spawn) -> (f32, f32) {
    let reach = reach_of(&object.object);
    let far = play(&[at([pyro::cursor(SKILL)[0], -4000.], object.clone())]);
    let shapes = far.frames.iter().flatten().filter(|shape| shape.active);
    let widest = shapes.map(|shape| shape.scale[0]).fold(0., f32::max);
    let sure = (reach - SURE_MARGIN) * widest + 12. - SURE_MARGIN;
    (sure, reach * widest + 13.)
}

fn reach_of(object: &str) -> f32 {
    let objects = &crate::hitbox::skill(SKILL).expect("hitboxes").objects;
    objects
        .iter()
        .find(|o| o.name == object)
        .expect(object)
        .reach
}

fn births(timeline: &Timeline, object: &str) -> usize {
    let shapes = timeline.frames.iter().flatten();
    shapes
        .filter(|shape| shape.object.name == object && shape.age == 1)
        .count()
}

fn play(spawns: &[Spawn]) -> Timeline {
    parity::play(SKILL, spawns, 1)
}

/// px from the dummy.
fn from(spot: [f32; 2]) -> f32 {
    let dummy = pyro::cursor(SKILL)[0];
    (spot[0] - dummy).hypot(spot[1])
}
