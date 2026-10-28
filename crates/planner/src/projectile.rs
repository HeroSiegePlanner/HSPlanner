//! Skill projectile speeds (data/skill-projectile-speed.json) and how Projectile Speed changes them.
use crate::hitbox::SkillStats;
use hsplanner_build::BuildSnapshot;
use hsplanner_engine::calc::{data, stats::ComputedStats, subskill::effective_skill_tags};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Game frames per second; speeds are in game pixels per frame.
pub(crate) const FPS: f64 = 60.;
/// Damage objects vanish this far from where they spawned.
pub(crate) const LIMIT: f64 = 1500.;

#[derive(Deserialize)]
struct Speeds {
    skills: HashMap<String, SkillFlight>,
}

#[derive(Deserialize)]
pub(crate) struct SkillFlight {
    /// Moving objects; empty when nothing in the skill moves with its own speed.
    pub objects: Vec<Flyer>,
    /// Why nothing moves, for a skill without moving objects.
    pub note: Option<String>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Bonus {
    /// Takes the bonus for its whole flight.
    Yes,
    /// Takes it at launch, then its own Step sets a fixed speed.
    Partly,
    /// Never sees the cast's modifiers, or sets its speed after the bonus step.
    No,
}

/// What the skill's subtree projectile speed does to the object.
#[derive(Deserialize, Default, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Subtree {
    /// Adds to the cast's percent.
    #[default]
    Percent,
    /// Scales the object's own base speed.
    Base,
    None,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Flyer {
    pub name: String,
    pub result: Bonus,
    /// deltaSpeed in game pixels per frame.
    pub speed: f64,
    /// Destroy timer in frames.
    pub lifetime: Option<f64>,
    /// Vanishes 1500 px from its spawn.
    #[serde(default)]
    pub limited: bool,
    /// A weapon attack that flies until it drops.
    #[serde(default)]
    pub drops: bool,
    /// Chain lightning style: takes the percent only.
    #[serde(default)]
    pub percent_only: bool,
    #[serde(default)]
    pub subtree: Subtree,
    pub why: Option<String>,
    pub origin: [f32; 2],
    pub scale: f32,
    pub mask: Option<String>,
    #[serde(default)]
    pub at: [f32; 2],
    /// Inclusive sprite bounds of an object without a mask.
    #[serde(rename = "box")]
    pub bounds: Option<[f32; 4]>,
}

impl Flyer {
    /// Game pixels it covers at `speed`; None when only a hit or a drop ends it.
    pub(crate) fn range(&self, speed: f64) -> Option<f64> {
        let life = self.lifetime.map(|frames| speed * frames);
        match (life, self.limited) {
            (Some(life), true) => Some(life.min(LIMIT)),
            (Some(life), false) => Some(life),
            (None, true) => Some(LIMIT),
            (None, false) => None,
        }
    }
}

static SPEEDS: LazyLock<Speeds> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/skill-projectile-speed.json"))
        .expect("embedded projectile speeds must be valid")
});

pub(crate) fn skill(id: &str) -> Option<&'static SkillFlight> {
    SPEEDS.skills.get(id)
}

/// Item stats ignore tags; the incarnation's Melee and Ranged nodes need the tag and Projectile.
const SPEED_KEYS: [(&str, Option<&str>, bool); 6] = [
    ("projectile_speed_pct", None, false),
    ("projectile_speed", None, true),
    ("melee_projectile_speed_pct", Some("Melee"), false),
    ("melee_projectile_speed", Some("Melee"), true),
    ("ranged_projectile_speed_pct", Some("Ranged"), false),
    ("ranged_projectile_speed", Some("Ranged"), true),
];
/// Titans Power: a separate multiplier while the buff lasts, not part of the sum.
const TITAN: &str = "projectile_speed_more";

pub(crate) fn is_speed_stat(key: &str) -> bool {
    key == TITAN || SPEED_KEYS.iter().any(|(k, _, _)| *k == key)
}

/// A speed stat as the game adds it to the cast's percent or flat sum.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpeedTerm {
    pub key: &'static str,
    pub value: f64,
    pub flat: bool,
}

/// Speed terms that reach a skill with these tags, and the ones it ignores.
pub(crate) fn speed_terms(
    tags: &[String],
    value: impl Fn(&str) -> f64,
) -> (Vec<SpeedTerm>, Vec<SpeedTerm>) {
    let has = |tag: &str| tags.iter().any(|t| t == tag);
    let mut terms = Vec::new();
    let mut ignored = Vec::new();
    for (key, tag, flat) in SPEED_KEYS {
        let value = value(key);
        if value == 0. {
            continue;
        }
        let term = SpeedTerm { key, value, flat };
        if tag.is_none_or(|tag| has(tag) && has("Projectile")) {
            terms.push(term);
        } else {
            ignored.push(term);
        }
    }
    (terms, ignored)
}

/// The game's speed step, once right after the object appears: a total at or below zero is
/// skipped, the percent goes first and the flat amount is never multiplied.
pub(crate) fn fly(base: f64, percent: f64, flat: f64, percent_only: bool) -> f64 {
    let mut speed = base;
    if percent > 0. {
        speed *= 1. + percent / 100.;
    }
    if flat > 0. && !percent_only {
        speed += flat;
    }
    speed
}

/// Flight preview of one skill: which speed stats reach it and what they do.
pub(crate) struct FlightPreview {
    pub skill_name: String,
    pub tags: Vec<String>,
    pub flight: Option<&'static SkillFlight>,
    pub terms: Vec<SpeedTerm>,
    pub ignored: Vec<SpeedTerm>,
    /// The skill's own subtree projectile speed, in percent.
    pub own: f64,
    /// Titans Power's multiplier while its buff lasts, in percent.
    pub titan: f64,
}

impl FlightPreview {
    fn sum(&self, flat: bool) -> f64 {
        self.terms
            .iter()
            .filter(|t| t.flat == flat)
            .map(|t| t.value)
            .sum()
    }

    /// Percent sum the cast stores, without the subtree.
    pub(crate) fn percent(&self) -> f64 {
        self.sum(false)
    }

    pub(crate) fn flat(&self) -> f64 {
        self.sum(true)
    }

    pub(crate) fn base(&self, flyer: &Flyer) -> f64 {
        match flyer.subtree {
            Subtree::Base => flyer.speed * (1. + self.own.max(0.) / 100.),
            _ => flyer.speed,
        }
    }

    /// Speed right after the bonus step.
    pub(crate) fn speed(&self, flyer: &Flyer) -> f64 {
        let base = self.base(flyer);
        if flyer.result == Bonus::No {
            return base;
        }
        // ponytail: subtree speed is taken to join the cast's percent; the atlas only read the
        // sub-skills whose objects read it directly (Rapid Arrows, Thunder's Reach, …).
        let own = if flyer.subtree == Subtree::Percent {
            self.own
        } else {
            0.
        };
        fly(base, self.percent() + own, self.flat(), flyer.percent_only)
    }

    /// The object that stands for the skill: the first that takes the bonus.
    pub(crate) fn headline(&self) -> Option<&'static Flyer> {
        let objects = &self.flight?.objects;
        objects
            .iter()
            .find(|o| o.result != Bonus::No)
            .or(objects.first())
    }
}

pub(crate) fn preview(
    snapshot: &BuildSnapshot,
    computed: &ComputedStats,
    skill_id: &str,
) -> Option<FlightPreview> {
    let class = snapshot.class_id.as_deref().unwrap_or("");
    let skill = data::get_skills_by_class(class)
        .iter()
        .find(|s| s.id == skill_id)?;
    let tags = effective_skill_tags(
        &skill.id,
        skill.tags.as_deref().unwrap_or_default(),
        &snapshot.subskill_ranks,
    );
    let split = SkillStats::new(snapshot, computed, skill_id);
    let (terms, ignored) = speed_terms(&tags, |key| split.shared(key));
    Some(FlightPreview {
        skill_name: skill.name.clone(),
        tags,
        flight: self::skill(skill_id),
        terms,
        ignored,
        own: split.own("projectile_speed_pct"),
        titan: split.total(TITAN),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(list: &[&str]) -> Vec<String> {
        list.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn embedded_speeds_name_planner_skills_and_shapes() {
        let skills: Vec<&str> = data::data()
            .skills_by_class
            .values()
            .flatten()
            .map(|s| s.id.as_str())
            .collect();
        for (id, flight) in &SPEEDS.skills {
            assert!(skills.contains(&id.as_str()), "unknown skill {id}");
            assert!(flight.objects.is_empty() == flight.note.is_some(), "{id}");
            for o in &flight.objects {
                assert!(o.speed > 0., "{id} {}", o.name);
                let shaped = o
                    .mask
                    .as_deref()
                    .is_some_and(|m| crate::hitbox::mask(m).is_some())
                    || o.bounds.is_some();
                assert!(shaped, "{id} {} has no shape", o.name);
            }
        }
    }

    #[test]
    fn percent_goes_before_flat_and_only_a_positive_total_counts() {
        assert_eq!(fly(10., 15., 2., false), 13.5);
        assert_eq!(fly(10., -69., 2., false), 12.);
        assert_eq!(fly(10., 15., -3., false), 11.5);
        assert_eq!(fly(10., 20., 2., true), 12.);
    }

    #[test]
    fn item_speed_reaches_every_skill_and_tree_speed_needs_its_tags() {
        let value = |key: &str| match key {
            "projectile_speed" => 2.,
            "melee_projectile_speed_pct" => 8.,
            "ranged_projectile_speed" => 1.,
            _ => 0.,
        };
        let (terms, ignored) = speed_terms(&tags(&["Melee", "Projectile"]), value);
        assert_eq!(
            terms.iter().map(|t| t.key).collect::<Vec<_>>(),
            ["projectile_speed", "melee_projectile_speed_pct"]
        );
        assert_eq!(ignored[0].key, "ranged_projectile_speed");
        let (terms, ignored) = speed_terms(&tags(&["Melee"]), value);
        assert_eq!(terms.len(), 1);
        assert_eq!(ignored.len(), 2);
    }

    #[test]
    fn range_ends_at_the_lifetime_or_the_limit() {
        let ball = &skill("fireball").expect("fireball flight").objects[0];
        assert_eq!(ball.range(ball.speed), Some(1200.));
        assert_eq!(ball.range(15.), Some(1800.));
        let limited = Flyer {
            limited: true,
            ..serde_json::from_str(r#"{"name":"x","result":"yes","speed":10,"lifetime":200,"origin":[0,0],"scale":1,"box":[0,0,1,1]}"#).expect("flyer")
        };
        assert_eq!(limited.range(10.), Some(LIMIT));
        let weapon = skill("thunder_fury").expect("thunder fury flight");
        assert!(
            weapon
                .objects
                .iter()
                .any(|o| o.drops && o.range(12.).is_none())
        );
    }

    #[test]
    fn subtree_speed_follows_what_the_object_reads() {
        let snapshot = BuildSnapshot {
            class_id: Some("amazon".into()),
            active_skill_ids: vec!["thunder_fury".into()],
            ..Default::default()
        };
        let computed = ComputedStats::default();
        let mut preview = preview(&snapshot, &computed, "thunder_fury").expect("preview");
        preview.own = 50.;
        let objects = &preview.flight.expect("flight").objects;
        let weapon = objects.iter().find(|o| o.drops).expect("weapon");
        let bolt = objects.iter().find(|o| !o.drops).expect("bolt");
        assert_eq!(preview.speed(weapon), weapon.speed);
        assert_eq!(preview.speed(bolt), bolt.speed * 1.5);
    }
}
