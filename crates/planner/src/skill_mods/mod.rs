//! What the build changes about one use of a skill in the preview: its subskills and the stats
//! that stretch or widen it. Each skill whose modifiers are modelled has its own module.
use crate::hitbox::{self, SkillStats};
use crate::skill_sim::Spawn;
use hsplanner_build::BuildSnapshot;
use hsplanner_engine::calc::data;
use hsplanner_engine::calc::skills::{AttackSkillDamageBreakdown, SkillDamageBreakdown};
use hsplanner_engine::calc::stats::ComputedStats;
use hsplanner_engine::calc::types::{AppliedStateValue, SubskillEffectSpec, SubskillNodeSpec};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

mod armageddon;
mod asteroid;
mod black_hole;
mod blazing_trail;
mod breath_of_fire;
mod charge;
mod comet;
mod fire_nova;
mod fireball;
mod hydra;
mod inferno_slash;
mod lunar_orbit;
#[cfg(test)]
mod parity_tests;
mod phoenix_flight;
#[cfg(test)]
mod proc_odds;
mod pyro;
mod scorching_aura;
mod scorching_whip;
mod solar_dash;
mod solar_flare;
mod supernova;
mod tsunami;
mod volcano;

/// A modelled skill: its id, its class, how its modifiers rewrite one use's spawns, and the nodes
/// whose whole-cast bonus it rolls per cast while the engine averages it into the hit. Nodes
/// whose damage the game keeps to their own objects carry `ownDamage` in the data instead.
type Modelled = (
    &'static str,
    &'static str,
    fn(&[Spawn], &Tree) -> Vec<Spawn>,
    &'static [&'static str],
);

const MODELLED: &[Modelled] = &[
    ("black_hole", "exo", black_hole::apply, &[]),
    ("asteroid", "exo", asteroid::apply, &[]),
    ("tsunami", "exo", tsunami::apply, &[]),
    ("supernova", "exo", supernova::apply, &[]),
    ("scorching_whip", "exo", scorching_whip::apply, &[]),
    ("lunar_orbit", "exo", lunar_orbit::apply, &[]),
    (
        "solar_flare",
        "exo",
        solar_flare::apply,
        &["burst_of_power"],
    ),
    ("solar_dash", "exo", solar_dash::apply, &[]),
    ("fireball", "pyromancer", fireball::apply, &[]),
    ("breath_of_fire", "pyromancer", breath_of_fire::apply, &[]),
    ("hydra", "pyromancer", hydra::apply, &[]),
    ("comet", "pyromancer", comet::apply, &["molten_aftershock"]),
    ("fire_nova", "pyromancer", fire_nova::apply, &[]),
    ("volcano", "pyromancer", volcano::apply, &[]),
    ("armageddon", "pyromancer", armageddon::apply, &[]),
    ("inferno_slash", "pyromancer", inferno_slash::apply, &[]),
    ("phoenix_flight", "pyromancer", phoenix_flight::apply, &[]),
    ("blazing_trail", "pyromancer", blazing_trail::apply, &[]),
    ("scorching_aura", "pyromancer", scorching_aura::apply, &[]),
];

/// Learned subskill ranks of the previewed skill, by node id, and the build's stats that stretch it.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Mods {
    ranks: BTreeMap<String, u32>,
    /// Skill Duration % from outside the skill's subtree.
    pub duration_pct: f32,
    /// Scale the build's size stats add to an object that takes the size bonus.
    pub size_bonus: f32,
    /// Share of the hit in the skill's elemental part: all of a spell's, part of an attack's.
    pub elemental: f32,
    /// Share of an attack's other part that its weapon bonus adds.
    pub weapon_bonus: f32,
}

impl Default for Mods {
    fn default() -> Self {
        Self {
            ranks: BTreeMap::new(),
            duration_pct: 0.,
            size_bonus: 0.,
            elemental: 1.,
            weapon_bonus: 0.,
        }
    }
}

impl Mods {
    /// The learned nodes of `skill_id` among the build's `skill:node` ranks.
    pub(crate) fn of(skill_id: &str, subskill_ranks: &HashMap<String, u32>) -> Self {
        let prefix = format!("{skill_id}:");
        let ranks = subskill_ranks
            .iter()
            .filter(|(_, rank)| **rank > 0)
            .filter_map(|(key, rank)| Some((key.strip_prefix(&prefix)?.to_owned(), *rank)))
            .collect();
        Self {
            ranks,
            ..Self::default()
        }
    }

    /// What the build changes about `skill_id`: nothing unless its modifiers are modelled, else
    /// its nodes, the Skill Duration and the size bonus the calculation gave it.
    pub(crate) fn for_build(
        skill_id: &str,
        snapshot: &BuildSnapshot,
        computed: Option<&ComputedStats>,
    ) -> Self {
        if !models(skill_id) {
            return Self::default();
        }
        let mut mods = Self::of(skill_id, &snapshot.subskill_ranks);
        if let Some(computed) = computed {
            // A NaN would miss the timeline cache on every frame and restart the loop.
            let finite = |value: f64| if value.is_finite() { value as f32 } else { 0. };
            let stats = SkillStats::new(snapshot, computed, skill_id);
            mods.duration_pct = finite(stats.shared("skill_duration"));
            mods.size_bonus =
                finite(hitbox::preview(snapshot, computed, skill_id).map_or(0., |p| p.bonus));
        }
        mods
    }

    pub(crate) fn rank(&self, node: &str) -> u32 {
        self.ranks.get(node).copied().unwrap_or(0)
    }
}

/// Whether the preview follows this skill's modifiers.
pub(crate) fn models(skill_id: &str) -> bool {
    modelled(skill_id).is_some()
}

fn modelled(skill_id: &str) -> Option<&'static Modelled> {
    MODELLED.iter().find(|(id, ..)| *id == skill_id)
}

/// Frames from one use to the next of a skill its own timer repeats: an aura's pulse.
pub(crate) fn pulse_frames(skill_id: &str, mods: &Mods) -> Option<usize> {
    let pulse = match skill_id {
        "scorching_aura" => scorching_aura::pulse_frames,
        _ => return None,
    };
    Some(pulse(&tree(skill_id, mods, 0)) as usize)
}

/// Uses to replay, raised to whole turns of a skill whose casts carry state from one to the next
/// or roll on their own before their objects exist.
pub(crate) fn casts(skill_id: &str, uses: u64) -> u64 {
    let turn = match skill_id {
        "lunar_orbit" => lunar_orbit::CASTS,
        "solar_flare" => solar_flare::CASTS,
        "solar_dash" => solar_dash::CASTS,
        "fireball" => fireball::CASTS,
        "breath_of_fire" => breath_of_fire::CASTS,
        "hydra" => hydra::CASTS,
        "comet" => comet::CASTS,
        "fire_nova" => fire_nova::CASTS,
        "volcano" => volcano::CASTS,
        "armageddon" => armageddon::CASTS,
        "inferno_slash" => inferno_slash::CASTS,
        "phoenix_flight" => phoenix_flight::CASTS,
        "blazing_trail" => blazing_trail::CASTS,
        "scorching_aura" => scorching_aura::CASTS,
        _ => 1,
    };
    uses.next_multiple_of(turn)
}

/// One use's spawns with the build's modifiers; a skill without modelled modifiers keeps its data.
pub(crate) fn apply<'a>(skill_id: &str, spawns: &'a [Spawn], mods: &Mods) -> Cow<'a, [Spawn]> {
    apply_cast(skill_id, spawns, mods, 0)
}

/// The `cast`-th use's spawns with the build's modifiers.
pub(crate) fn apply_cast<'a>(
    skill_id: &str,
    spawns: &'a [Spawn],
    mods: &Mods,
    cast: u64,
) -> Cow<'a, [Spawn]> {
    let Some((_, _, rewrite, _)) = modelled(skill_id) else {
        return Cow::Borrowed(spawns);
    };
    Cow::Owned(rewrite(spawns, &tree(skill_id, mods, cast)))
}

/// A modelled skill's subskills under `mods`, for its `cast`-th use.
fn tree<'a>(skill_id: &str, mods: &'a Mods, cast: u64) -> Tree<'a> {
    let class = modelled(skill_id).map_or("", |(_, class, ..)| class);
    let nodes = data::get_skills_by_class(class)
        .iter()
        .find(|skill| skill.id == skill_id)
        .and_then(|skill| skill.subskills.as_deref())
        .unwrap_or_default();
    Tree { nodes, mods, cast }
}

/// The learned nodes of one skill, the planner values they carry and which use is cast.
struct Tree<'a> {
    nodes: &'a [SubskillNodeSpec],
    mods: &'a Mods,
    /// Uses cast before this one, for state a cast leaves to the next.
    cast: u64,
}

/// A planner value at `rank`: its base plus its per-rank step.
fn at(base: Option<f64>, per_rank: Option<f64>, rank: u32) -> f32 {
    (base.unwrap_or(0.) + per_rank.unwrap_or(0.) * f64::from(rank)) as f32
}

/// One stat of a node's effects or proc effects at `rank`, in percent.
fn of(effects: Option<&SubskillEffectSpec>, key: &str, rank: u32) -> f32 {
    effects.map_or(0., |effects| {
        let get = |map: &Option<HashMap<String, f64>>| map.as_ref()?.get(key).copied();
        at(get(&effects.base), get(&effects.per_rank), rank)
    })
}

impl Tree<'_> {
    fn learned(&self, node: &str) -> bool {
        self.mods.rank(node) > 0
    }

    fn node(&self, id: &str) -> Option<(&SubskillNodeSpec, u32)> {
        let rank = self.mods.rank(id);
        let node = self.nodes.iter().find(|node| node.id == id)?;
        (rank > 0).then_some((node, rank))
    }

    /// A planner stat summed over the learned nodes, in percent.
    fn stat(&self, key: &str) -> f32 {
        self.nodes
            .iter()
            .map(|node| node.planner_stat(key, self.mods.rank(&node.id)))
            .sum::<f64>() as f32
    }

    /// One stat of a learned node's own effects, its proc left out, in percent.
    fn own(&self, node: &str, key: &str) -> f32 {
        self.node(node)
            .map_or(0., |(node, rank)| of(node.effects.as_ref(), key, rank))
    }

    /// A learned node's proc: its chance per roll (0 to 1) and one of its effects, in percent.
    fn proc(&self, node: &str, effect: &str) -> Option<(f32, f32)> {
        let (node, rank) = self.node(node)?;
        let proc = node.proc.as_ref()?;
        let chance = at(proc.chance.base, proc.chance.per_rank, rank) / 100.;
        Some((chance, of(proc.effects.as_ref(), effect, rank)))
    }

    /// The amount of `state` a learned node's proc applies, in percent.
    fn applies(&self, node: &str, state: &str) -> f32 {
        let amount = self.node(node).and_then(|(node, rank)| {
            let states = node.proc.as_ref()?.applies_states.as_ref()?;
            states.iter().find_map(|applied| match applied {
                AppliedStateValue::Full {
                    state: name,
                    amount: Some(amount),
                } if name == state => Some(at(amount.base, amount.per_rank, rank)),
                _ => None,
            })
        });
        amount.unwrap_or(0.)
    }
}

/// Percent in the two additive pools that planner damage joins in a hit: skill damage and
/// of_total_damage.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(crate) struct Pools {
    pub skill: f64,
    pub of_total: f64,
}

impl Pools {
    /// What is left of the skill-damage and of_total_damage parts of a hit with these pools once
    /// `drawn` leaves them.
    fn left(self, drawn: Pools) -> [f64; 2] {
        let left = |pool: f64, part: f64| {
            if part == 0. || pool <= -100. {
                1.
            } else {
                ((100. + pool - part) / (100. + pool)).max(0.)
            }
        };
        [
            left(self.skill, drawn.skill),
            left(self.of_total, drawn.of_total),
        ]
    }
}

/// The hit the preview's numbers start from, without the planner damage it draws itself.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Hit {
    /// An attack's full hit, weapon and elemental parts, or a spell's.
    pub damage: f64,
    /// Share of it in the skill's elemental part.
    pub elemental: f32,
    /// Share of the rest that the attack's weapon bonus adds.
    pub weapon_bonus: f32,
}

impl Hit {
    /// From the engine's elemental and attack breakdowns, `drawn` taken out.
    pub(crate) fn of(
        elemental: Option<&SkillDamageBreakdown>,
        attack: Option<&AttackSkillDamageBreakdown>,
        drawn: Pools,
    ) -> Option<Self> {
        let mean = |min: i64, max: i64| (min + max) as f64 / 2.;
        let sources = elemental.map(|d| &d.extra_damage_sources);
        let sources = sources.or(attack.map(|a| &a.extra_damage_sources));
        let pools = Pools {
            skill: elemental.map_or(0., |d| {
                (d.skill_damage_min_pct + d.skill_damage_max_pct) / 2.
            }),
            of_total: (sources.into_iter().flatten())
                .filter(|source| source.label == "Subtree")
                .map(|source| source.pct)
                .sum(),
        };
        let part = elemental.map(|d| mean(d.hit_min, d.hit_max));
        let full = attack.map(|a| mean(a.combined_hit_min, a.combined_hit_max));
        let multiplier = attack.map_or(1., |a| {
            (a.weapon_multiplier_min + a.weapon_multiplier_max) / 2.
        });
        (part.is_some() || full.is_some())
            .then(|| Self::split(full, part, multiplier, pools, drawn))
    }

    /// An attack's `full` hit, or a spell's elemental `part`: skill damage reaches only the
    /// elemental part, of_total_damage every part.
    fn split(
        full: Option<f64>,
        part: Option<f64>,
        multiplier: f64,
        pools: Pools,
        drawn: Pools,
    ) -> Self {
        let [skill, of_total] = pools.left(drawn);
        let kept = part.unwrap_or(0.) * skill;
        let whole = full.map_or(kept, |full| (full - part.unwrap_or(0.)).max(0.) + kept);
        let elemental = if whole > 0. { kept / whole } else { 1. };
        let weapon_bonus = if multiplier > 0. {
            1. - 1. / multiplier
        } else {
            0.
        };
        Self {
            damage: whole * of_total,
            elemental: elemental as f32,
            weapon_bonus: weapon_bonus.max(0.) as f32,
        }
    }
}

/// Planner damage of `skill_id`'s learned nodes whose whole-cast roll the preview draws itself
/// although the engine averages it into the hit.
pub(crate) fn drawn_damage(skill_id: &str, mods: &Mods) -> Pools {
    let Some((_, class, _, drawn)) = MODELLED.iter().find(|(id, ..)| *id == skill_id) else {
        return Pools::default();
    };
    let skills = data::get_skills_by_class(class);
    let Some(skill) = skills.iter().find(|skill| skill.id == skill_id) else {
        return Pools::default();
    };
    let key = format!(
        "{}_skill_damage",
        skill.damage_type.as_deref().unwrap_or_default()
    );
    let nodes = skill.subskills.iter().flatten();
    let drawn_nodes = nodes.filter(|node| drawn.contains(&node.id.as_str()));
    drawn_nodes.fold(Pools::default(), |pools, node| {
        let rank = mods.rank(&node.id);
        let of_total = if node.counts_dummy_hits() {
            0.
        } else {
            node.planner_stat("of_total_damage", rank)
        };
        Pools {
            skill: pools.skill + node.planner_stat(&key, rank),
            of_total: pools.of_total + of_total,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{Hit, Mods, Pools, drawn_damage};
    use hsplanner_engine::calc::skills::{AttackSkillDamageBreakdown, SkillDamageBreakdown};
    use std::collections::HashMap;

    fn drawn(skill: &str, node: &str, rank: u32) -> Pools {
        let ranks = HashMap::from([(format!("{skill}:{node}"), rank)]);
        drawn_damage(skill, &Mods::of(skill, &ranks))
    }

    #[test]
    fn only_whole_cast_rolls_leave_the_hit_the_engine_gives_the_preview() {
        // Molten Aftershock: 3r% of casts gain 35r% skill damage, averaged by the engine.
        let aftershock = Pools {
            skill: 26.25,
            of_total: 0.,
        };
        assert_eq!(drawn("comet", "molten_aftershock", 5), aftershock);
        // The engine already keeps Too Hot to Touch's damage off the hit.
        assert_eq!(drawn("volcano", "too_hot_to_touch", 5), Pools::default());
    }

    #[test]
    fn a_hit_loses_only_the_drawn_part_of_each_pool() {
        let pools = Pools {
            skill: 100.,
            of_total: 50.,
        };
        let drawn = Pools {
            skill: 50.,
            of_total: 25.,
        };
        let [skill, of_total] = pools.left(drawn);
        assert!((skill * of_total - 0.625).abs() < 1e-9);
        assert_eq!(pools.left(Pools::default()), [1., 1.]);
    }

    fn elemental(hit: i64, skill_pct: f64) -> SkillDamageBreakdown {
        let mut breakdown = SkillDamageBreakdown::default();
        (breakdown.hit_min, breakdown.hit_max) = (hit, hit);
        breakdown.skill_damage_min_pct = skill_pct;
        breakdown.skill_damage_max_pct = skill_pct;
        breakdown
    }

    fn attack(full: i64, multiplier: f64) -> AttackSkillDamageBreakdown {
        let mut breakdown = AttackSkillDamageBreakdown::default();
        (breakdown.combined_hit_min, breakdown.combined_hit_max) = (full, full);
        breakdown.weapon_multiplier_min = multiplier;
        breakdown.weapon_multiplier_max = multiplier;
        breakdown
    }

    #[test]
    fn a_spell_starts_from_its_hit_without_the_drawn_pools() {
        let drawn = Pools {
            skill: 50.,
            of_total: 0.,
        };
        let hit = Hit::of(Some(&elemental(100, 100.)), None, drawn);
        assert_eq!(
            hit,
            Some(Hit {
                damage: 75.,
                elemental: 1.,
                weapon_bonus: 0.,
            })
        );
        assert_eq!(Hit::of(None, None, drawn), None);
    }

    #[test]
    fn an_attack_starts_from_its_full_hit_and_skill_damage_leaves_only_its_elemental_part() {
        let drawn = Pools {
            skill: 50.,
            of_total: 0.,
        };
        // 300 weapon and 100 fire with a 100 % fire pool: half the pool leaves the fire alone.
        let hit = Hit::of(Some(&elemental(100, 100.)), Some(&attack(400, 2.5)), drawn);
        let hit = hit.expect("an attack's hit");
        assert!((hit.damage - 375.).abs() < 1e-9);
        assert!((hit.elemental - 0.2).abs() < 1e-6);
        assert!((hit.weapon_bonus - 0.6).abs() < 1e-6);
        let weapon_only = Hit::of(None, Some(&attack(300, 1.)), Pools::default());
        assert_eq!(
            weapon_only,
            Some(Hit {
                damage: 300.,
                elemental: 0.,
                weapon_bonus: 0.,
            })
        );
    }

    #[test]
    fn of_total_damage_leaves_every_part_of_an_attack() {
        let pools = Pools {
            skill: 100.,
            of_total: 50.,
        };
        let drawn = Pools {
            skill: 0.,
            of_total: 25.,
        };
        let hit = Hit::split(Some(400.), Some(100.), 1., pools, drawn);
        assert!((hit.damage - 400. * 125. / 150.).abs() < 1e-9);
        assert!((hit.elemental - 0.25).abs() < 1e-6);
    }
}
