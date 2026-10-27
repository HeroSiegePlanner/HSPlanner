//! Skill hitboxes (data/skill-hitboxes.json) and how the game's size stats grow them.
use hsplanner_build::BuildSnapshot;
use hsplanner_engine::calc::{
    data,
    stats::{ComputedStats, SourceType},
    subskill::effective_skill_tags,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Deserialize)]
struct Hitboxes {
    player: Player,
    skills: HashMap<String, SkillHitbox>,
    masks: HashMap<String, Mask>,
}

#[derive(Deserialize)]
pub(crate) struct Player {
    pub mask: String,
    pub origin: [f32; 2],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillHitbox {
    /// abilityAOEScale: the share of the size stats this skill takes.
    #[serde(default = "one")]
    pub aoe_scale: f64,
    /// Created by another skill without the cast's modifiers.
    #[serde(default)]
    pub triggered: bool,
    #[serde(default)]
    pub objects: Vec<HitObject>,
    /// collision_circle_list radii in game pixels.
    #[serde(default)]
    pub circles: Vec<f32>,
    /// The game's target rings (AoE indicator, landing flash); fixed at cast, size stats never grow them.
    #[serde(default)]
    pub rings: Vec<Ring>,
}

/// A band `band` px wide centred `radius` px around the target.
#[derive(Deserialize)]
pub(crate) struct Ring {
    pub radius: f32,
    pub band: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HitObject {
    pub name: String,
    pub origin: [f32; 2],
    pub scale: [f64; 2],
    pub mask: Option<String>,
    /// Sprite position of the mask's first row and column.
    #[serde(default)]
    pub at: [f32; 2],
    /// Inclusive sprite bounds of an object without a mask.
    #[serde(rename = "box")]
    pub bounds: Option<[f32; 4]>,
    /// Share of the size bonus the object receives.
    #[serde(default)]
    pub bonus: f64,
    #[serde(default)]
    pub growth: Growth,
    pub start: Option<f64>,
    pub cap: Option<f64>,
    /// Farthest hit pixel from the origin, in sprite pixels.
    #[serde(default)]
    pub reach: f32,
    /// Turns while alive, so it sweeps its reach in every direction.
    #[serde(default)]
    pub spins: bool,
    /// An explosion: its parent adds Explosion AoE × 0.01 whatever the skill's tags.
    #[serde(default)]
    pub explosion: bool,
    /// A passive's object: the skills whose cast modifiers it carries.
    #[serde(default)]
    pub from: Vec<String>,
}

/// What the object's own Step does with the size bonus.
#[derive(Deserialize, Default, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Growth {
    #[default]
    Add,
    Max,
    Cap,
    Lost,
}

/// Collision mask as pixel runs per row: [start, end, start, end, …].
#[derive(Deserialize)]
pub(crate) struct Mask {
    pub rows: Vec<Vec<u16>>,
}

fn one() -> f64 {
    1.
}

static HITBOXES: LazyLock<Hitboxes> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/skill-hitboxes.json"))
        .expect("embedded skill hitboxes must be valid")
});

pub(crate) fn skill(id: &str) -> Option<&'static SkillHitbox> {
    HITBOXES.skills.get(id)
}

pub(crate) fn mask(id: &str) -> Option<&'static Mask> {
    HITBOXES.masks.get(id)
}

pub(crate) fn player() -> &'static Player {
    &HITBOXES.player
}

/// A size stat as the game's LoadAllModifiers adds it to the skill's scale.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SizeTerm {
    pub key: &'static str,
    pub percent: f64,
    pub scale: f64,
}

/// Stats that can change a skill's size, in display order.
const SIZE_KEYS: [&str; 12] = [
    "area_of_effect",
    "spell_aoe_radius",
    "attack_radius",
    "explosion_aoe",
    "orbital_skill_size",
    "projectile_size_increase",
    "spell_projectile_size",
    "melee_projectile_size",
    "shield_skill_radius",
    "melee_range",
    "orbiting_relic_size",
    "skill_relic_projectile_size",
];
/// Total Chaos raises both the AoE and the explosion radius.
const TOTAL_AREA: &str = "area_of_effect_more";
/// Skill-scoped subtree size, which only ever belongs to its own skill.
const SUBTREE_SIZE: &str = "projectile_size";

pub(crate) fn is_size_stat(key: &str) -> bool {
    SIZE_KEYS.contains(&key) || key == TOTAL_AREA || key == SUBTREE_SIZE
}

/// (key, weight) the tags take; melee range counts half and ignores abilityAOEScale.
fn counted_keys(tags: &[String]) -> Vec<(&'static str, f64)> {
    let has = |tag: &str| tags.iter().any(|t| t == tag);
    let (melee, ranged, spell, projectile) =
        (has("Melee"), has("Ranged"), has("Spell"), has("Projectile"));
    let mut keys = Vec::new();
    if has("Shield") {
        keys.push(("shield_skill_radius", 1.));
    }
    if melee {
        if projectile {
            keys.push(("melee_projectile_size", 1.));
        }
        keys.push(("melee_range", 0.5));
    }
    if has("Relic") {
        if has("Orbital") {
            keys.push(("orbiting_relic_size", 1.));
        } else if has("Active") {
            keys.push(("skill_relic_projectile_size", 1.));
        }
    } else {
        if has("Orbital") {
            keys.push(("orbital_skill_size", 1.));
        }
        if has("Area of Effect") {
            keys.push(("area_of_effect", 1.));
            if spell {
                keys.push(("spell_aoe_radius", 1.));
            }
            if melee || ranged {
                keys.push(("attack_radius", 1.));
            }
        }
    }
    if has("Explosion") {
        keys.push(("explosion_aoe", 1.));
    }
    if projectile {
        keys.push(("projectile_size_increase", 1.));
        if spell {
            keys.push(("spell_projectile_size", 1.));
        }
    }
    keys
}

/// A size stat's total after Spacial Conversion / Concentrated Damage take their share (0–100)
/// of the AoE and explosion radius.
fn kept(value: &impl Fn(&str) -> f64, key: &str, converted: [f64; 2]) -> f64 {
    let (total, share) = match key {
        "area_of_effect" => (value(key) + value(TOTAL_AREA), converted[0]),
        "explosion_aoe" => (value(key) + value(TOTAL_AREA), converted[1]),
        _ => (value(key), 0.),
    };
    total * (1. - share.clamp(0., 100.) / 100.)
}

/// Size terms that reach a skill with these tags, and the size stats it ignores.
pub(crate) fn size_terms(
    tags: &[String],
    aoe_scale: f64,
    value: impl Fn(&str) -> f64,
    converted: [f64; 2],
) -> (Vec<SizeTerm>, Vec<SizeTerm>) {
    let total = |key: &str| match key {
        "area_of_effect" | "explosion_aoe" => value(key) + value(TOTAL_AREA),
        _ => value(key),
    };
    let kept = |key: &str| kept(&value, key, converted);
    let counted = counted_keys(tags);
    let mut terms = Vec::new();
    let mut ignored = Vec::new();
    for key in SIZE_KEYS {
        match counted.iter().find(|(k, _)| *k == key) {
            Some(&(_, weight)) if kept(key) != 0. => {
                let share = if key == "melee_range" { 1. } else { aoe_scale };
                terms.push(SizeTerm {
                    key,
                    percent: kept(key),
                    scale: kept(key) / 100. * weight * share,
                });
            }
            // Total Chaos is listed once, under Area of Effect.
            None if key != "explosion_aoe" && total(key) != 0. || value(key) != 0. => {
                ignored.push(SizeTerm {
                    key,
                    percent: if key == "explosion_aoe" {
                        value(key)
                    } else {
                        total(key)
                    },
                    scale: 0.,
                })
            }
            _ => {}
        }
    }
    (terms, ignored)
}

/// Scale of an object with the skill's own subtree size, the cast's size bonus and, for an
/// explosion, the Explosion AoE its parent adds.
pub(crate) fn object_scale(object: &HitObject, own: f64, bonus: f64, explosion: f64) -> [f64; 2] {
    let takes = object.bonus > 0.;
    // ponytail: subtree size is taken to multiply the base scale; the atlas reads
    // objects without sub-talents, so the exact sub-talent formula is unknown.
    let own = if takes { (1. + own / 100.).max(0.) } else { 1. };
    let [bx, by] = [object.scale[0] * own, object.scale[1] * own];
    let direct = if object.explosion {
        explosion.max(0.)
    } else {
        0.
    };
    let add = object.bonus * bonus + direct;
    if add <= 0. || object.growth == Growth::Lost {
        return [bx, by];
    }
    if let (Growth::Cap, Some(start)) = (object.growth, object.start) {
        let value = (start + add).max(object.cap.unwrap_or(bx));
        return [bx.max(value), by.max(value)];
    }
    [bx + add, by + add]
}

/// Area preview of one skill: which size stats reach it and how much they add.
pub(crate) struct AreaPreview {
    pub skill_name: String,
    pub tags: Vec<String>,
    pub hitbox: Option<&'static SkillHitbox>,
    /// The skill's objects, then those of allocated passives that carry its modifiers.
    pub objects: Vec<&'static HitObject>,
    /// Names of those passives.
    pub companions: Vec<String>,
    pub terms: Vec<SizeTerm>,
    pub ignored: Vec<SizeTerm>,
    /// The skill's own subtree size, in percent.
    pub own: f64,
    /// Added to the scale of objects that take the size bonus (never negative).
    pub bonus: f64,
    /// Explosion AoE as the explosions' parent adds it, whatever the skill's tags.
    pub explosion: Option<SizeTerm>,
}

impl AreaPreview {
    pub(crate) fn scale(&self, object: &HitObject) -> [f64; 2] {
        let explosion = self.explosion.as_ref().map_or(0., |term| term.scale);
        object_scale(object, self.own, self.bonus, explosion)
    }

    /// The object that best stands for the skill's size: the first that can grow.
    pub(crate) fn headline(&self) -> Option<&'static HitObject> {
        let grows = |o: &&&'static HitObject| o.bonus > 0. || o.explosion;
        self.objects
            .iter()
            .find(grows)
            .or(self.objects.first())
            .copied()
    }
}

/// Share (0–100) of a radius the allocated tree converts to damage, e.g. Spacial Conversion.
fn converted_share(snapshot: &BuildSnapshot, phrase: &str) -> f64 {
    let nodes = data::tree_nodes();
    snapshot
        .allocated_tree_nodes
        .iter()
        .filter_map(|id| nodes.get(&id.to_string()))
        .flat_map(|node| &node.lines)
        .filter(|line| line.to_lowercase().contains(phrase))
        .filter_map(|line| {
            line.trim_start_matches('+')
                .split('%')
                .next()?
                .parse::<f64>()
                .ok()
        })
        .sum::<f64>()
        .clamp(0., 100.)
}

/// A skill's stats split into its own subtree and the rest. The engine adds the main
/// skill's subtree to the shared stats; other skills carry it in their overrides.
pub(crate) struct SkillStats<'a> {
    computed: &'a ComputedStats,
    skill_id: &'a str,
    is_main: bool,
}

impl<'a> SkillStats<'a> {
    pub(crate) fn new(
        snapshot: &BuildSnapshot,
        computed: &'a ComputedStats,
        skill_id: &'a str,
    ) -> Self {
        let is_main = snapshot.active_skill_ids.first().map(String::as_str) == Some(skill_id);
        Self {
            computed,
            skill_id,
            is_main,
        }
    }

    pub(crate) fn total(&self, key: &str) -> f64 {
        self.computed.stats.get(key).map_or(0., |v| v.1)
    }

    pub(crate) fn sources(&self, key: &str, kind: SourceType) -> f64 {
        self.computed.stat_sources.get(key).map_or(0., |sources| {
            sources
                .iter()
                .filter(|s| s.source_type == kind)
                .map(|s| s.value.1)
                .sum()
        })
    }

    pub(crate) fn own(&self, key: &str) -> f64 {
        let scoped = self
            .computed
            .skill_scoped
            .get(self.skill_id)
            .and_then(|m| m.get(key))
            .map_or(0., |v| v.1);
        let shared = if self.is_main {
            self.sources(key, SourceType::Subskill)
        } else {
            self.computed
                .skill_stat_overrides
                .get(self.skill_id)
                .and_then(|m| m.get(key))
                .map_or(0., |v| v.1 - self.total(key))
        };
        scoped + shared
    }

    pub(crate) fn shared(&self, key: &str) -> f64 {
        if self.is_main {
            self.total(key) - self.own(key)
        } else {
            self.total(key)
        }
    }
}

/// Preview for `skill_id` from the stats it was calculated with.
pub(crate) fn preview(
    snapshot: &BuildSnapshot,
    computed: &ComputedStats,
    skill_id: &str,
) -> Option<AreaPreview> {
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
    let own_of = |key: &str| split.own(key);
    // The tree's "Attack Radius" (Far Reaching Carnage, Swinging Axes) is the game's melee
    // attack range; only items carry the Attack Area of Effect radius.
    let tree_attack_radius = split.sources("attack_radius", SourceType::Tree);
    let shared_of = |key: &str| {
        let shared = split.shared(key);
        match key {
            "melee_range" => shared + tree_attack_radius,
            "attack_radius" => shared - tree_attack_radius,
            _ => shared,
        }
    };
    let hitbox = self::skill(skill_id);
    let converted = [
        converted_share(snapshot, "of area of effect radius converted"),
        converted_share(snapshot, "of explosion area of effect radius converted"),
    ];
    let (terms, mut ignored) = size_terms(
        &tags,
        hitbox.map_or(1., |h| h.aoe_scale),
        &shared_of,
        converted,
    );
    let own = SIZE_KEYS
        .iter()
        .chain([&SUBTREE_SIZE])
        .map(|&key| own_of(key))
        .sum();
    let bonus = if hitbox.is_some_and(|h| h.triggered) {
        0.
    } else {
        terms.iter().map(|t| t.scale).sum::<f64>().max(0.)
    };
    // Allocated passives whose objects this skill creates carry its modifiers (InheritModifiers).
    let mut objects: Vec<&'static HitObject> = hitbox
        .map(|h| h.objects.iter().collect())
        .unwrap_or_default();
    let mut companions = Vec::new();
    for passive in data::get_skills_by_class(class) {
        let allocated = snapshot
            .skill_ranks
            .get(&passive.id)
            .is_some_and(|r| *r > 0);
        if passive.id == skill_id || !allocated {
            continue;
        }
        let carried: Vec<&'static HitObject> = self::skill(&passive.id)
            .into_iter()
            .flat_map(|h| &h.objects)
            .filter(|o| o.from.iter().any(|id| id == skill_id))
            .filter(|o| objects.iter().all(|known| known.name != o.name))
            .collect();
        if !carried.is_empty() {
            companions.push(passive.name.clone());
            objects.extend(carried);
        }
    }
    let explosion = objects
        .iter()
        .any(|o| o.explosion)
        .then(|| kept(&shared_of, "explosion_aoe", converted))
        .filter(|percent| *percent != 0.)
        .map(|percent| SizeTerm {
            key: "explosion_aoe",
            percent,
            scale: percent.max(0.) / 100.,
        });
    if explosion.is_some() {
        ignored.retain(|term| term.key != "explosion_aoe");
    }
    Some(AreaPreview {
        skill_name: skill.name.clone(),
        tags,
        hitbox,
        objects,
        companions,
        terms,
        ignored,
        own,
        bonus,
        explosion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(list: &[&str]) -> Vec<String> {
        list.iter().map(|t| t.to_string()).collect()
    }

    fn stats(pairs: &[(&str, f64)]) -> impl Fn(&str) -> f64 {
        let map: HashMap<String, f64> = pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        move |key| map.get(key).copied().unwrap_or(0.)
    }

    #[test]
    fn embedded_hitboxes_resolve_their_masks() {
        assert!(mask(&player().mask).is_some());
        for (id, skill) in &HITBOXES.skills {
            for object in &skill.objects {
                if let Some(m) = &object.mask {
                    assert!(mask(m).is_some(), "{id} uses missing mask {m}");
                } else {
                    assert!(
                        object.bounds.is_some(),
                        "{id} has an object without a shape"
                    );
                }
            }
        }
        assert_eq!(skill("fire_nova").map(|s| s.aoe_scale), Some(0.4));
        let skills: Vec<&str> = data::data()
            .skills_by_class
            .values()
            .flatten()
            .map(|s| s.id.as_str())
            .collect();
        for (id, skill) in &HITBOXES.skills {
            for source in skill.objects.iter().flat_map(|o| &o.from) {
                assert!(skills.contains(&source.as_str()), "{id} names {source}");
            }
            assert!(skill.rings.iter().all(|r| r.radius > 0. && r.band > 0.));
        }
    }

    #[test]
    fn area_of_effect_only_reaches_skills_with_the_tag() {
        let value = stats(&[
            ("area_of_effect", 30.),
            ("orbital_skill_size", 15.),
            ("melee_range", 20.),
        ]);
        let (terms, ignored) = size_terms(&tags(&["Melee", "Orbital"]), 1., &value, [0., 0.]);
        let keys: Vec<_> = terms.iter().map(|t| (t.key, t.scale)).collect();
        assert_eq!(keys, [("orbital_skill_size", 0.15), ("melee_range", 0.1)]);
        assert_eq!(
            ignored.iter().map(|t| t.key).collect::<Vec<_>>(),
            ["area_of_effect"]
        );
    }

    #[test]
    fn spell_aoe_and_total_chaos_scale_with_ability_aoe_scale() {
        let value = stats(&[
            ("area_of_effect", 40.),
            ("spell_aoe_radius", 10.),
            (TOTAL_AREA, 8.),
        ]);
        let spell_aoe = tags(&["Spell", "Area of Effect"]);
        let (terms, _) = size_terms(&spell_aoe, 0.4, &value, [0., 0.]);
        let total: f64 = terms.iter().map(|t| t.scale).sum();
        assert!((total - 0.58 * 0.4).abs() < 1e-9);
        let (converted, ignored) = size_terms(&spell_aoe, 1., &value, [100., 0.]);
        assert_eq!(
            converted.iter().map(|t| t.key).collect::<Vec<_>>(),
            ["spell_aoe_radius"]
        );
        assert!(ignored.is_empty());
    }

    #[test]
    fn blender_takes_its_subtree_and_melee_range_but_not_area_of_effect() {
        use hsplanner_engine::calc::stats::SourceContribution;
        let source = |source_type, value| SourceContribution {
            label: "test".into(),
            source_type,
            value: (value, value),
            forge: None,
        };
        let snapshot = BuildSnapshot {
            class_id: Some("butcher".into()),
            active_skill_ids: vec!["blender".into()],
            ..Default::default()
        };
        let computed = ComputedStats {
            stats: HashMap::from([
                ("area_of_effect".into(), (135., 135.)),
                ("attack_radius".into(), (15., 15.)),
            ]),
            stat_sources: HashMap::from([
                (
                    "area_of_effect".into(),
                    vec![
                        source(SourceType::Subskill, 105.),
                        source(SourceType::Tree, 30.),
                    ],
                ),
                ("attack_radius".into(), vec![source(SourceType::Tree, 15.)]),
            ]),
            ..Default::default()
        };
        let preview = preview(&snapshot, &computed, "blender").expect("blender preview");
        assert_eq!(preview.own, 105.);
        assert_eq!(
            preview.terms,
            [SizeTerm {
                key: "melee_range",
                percent: 15.,
                scale: 0.075
            }]
        );
        assert_eq!(preview.ignored[0].key, "area_of_effect");
        assert_eq!(preview.ignored[0].percent, 30.);
        let blade = preview.headline().expect("blender hitbox");
        let [scale, _] = preview.scale(blade);
        assert!((scale - (0.66 * 2.05 + 0.075)).abs() < 1e-9);
    }

    // Explosions take Explosion AoE from their parent even when the skill lacks the tag.
    #[test]
    fn explosions_grow_with_explosion_aoe_whatever_the_tags() {
        let snapshot = BuildSnapshot {
            class_id: Some("pirate".into()),
            active_skill_ids: vec!["buckshot".into()],
            ..Default::default()
        };
        let computed = ComputedStats {
            stats: HashMap::from([("explosion_aoe".into(), (40., 40.))]),
            ..Default::default()
        };
        let preview = preview(&snapshot, &computed, "buckshot").expect("buckshot preview");
        assert!(!preview.tags.iter().any(|t| t == "Explosion"));
        assert_eq!(preview.explosion.as_ref().map(|t| t.scale), Some(0.4));
        assert!(preview.ignored.iter().all(|t| t.key != "explosion_aoe"));
        let blast = preview
            .objects
            .iter()
            .find(|o| o.explosion)
            .expect("an explosion");
        assert_eq!(preview.scale(blast)[0], blast.scale[0] + 0.4);
    }

    #[test]
    fn allocated_passives_add_the_objects_that_carry_the_skills_modifiers() {
        let mut snapshot = BuildSnapshot {
            class_id: Some("marauder".into()),
            active_skill_ids: vec!["crazy_grapple".into()],
            ..Default::default()
        };
        let computed = ComputedStats::default();
        let names = |p: &AreaPreview| p.objects.iter().map(|o| o.name.clone()).collect::<Vec<_>>();
        let alone = preview(&snapshot, &computed, "crazy_grapple").expect("grapple preview");
        assert!(alone.companions.is_empty());
        assert!(!names(&alone).contains(&"Marauder_Unstable_Bomb_obj".to_owned()));
        snapshot.skill_ranks.insert("unstable_bomb".into(), 1);
        let with = preview(&snapshot, &computed, "crazy_grapple").expect("grapple preview");
        assert_eq!(with.companions, ["Unstable Bomb"]);
        assert!(names(&with).contains(&"Marauder_Unstable_Bomb_obj".to_owned()));
    }

    #[test]
    fn object_growth_follows_the_objects_own_step() {
        let object = |growth, bonus, start, cap| HitObject {
            name: String::new(),
            origin: [0., 0.],
            scale: [1., 1.],
            mask: None,
            at: [0., 0.],
            bounds: Some([0., 0., 1., 1.]),
            bonus,
            growth,
            start,
            cap,
            reach: 1.,
            spins: false,
            explosion: false,
            from: Vec::new(),
        };
        let add = object(Growth::Add, 1., None, None);
        assert_eq!(object_scale(&add, 0., 0.5, 0.), [1.5, 1.5]);
        assert_eq!(object_scale(&add, 100., 0.5, 0.), [2.5, 2.5]);
        let lost = object(Growth::Lost, 1., None, None);
        assert_eq!(object_scale(&lost, 0., 0.5, 0.), [1., 1.]);
        let fixed = object(Growth::Add, 0., None, None);
        assert_eq!(object_scale(&fixed, 100., 0.5, 0.4), [1., 1.]);
        let capped = object(Growth::Cap, 1., Some(0.2), Some(1.2));
        assert_eq!(object_scale(&capped, 0., 0.5, 0.), [1.2, 1.2]);
        let blast = HitObject {
            explosion: true,
            ..object(Growth::Add, 0., None, None)
        };
        assert_eq!(object_scale(&blast, 0., 0.5, 0.25), [1.25, 1.25]);
        assert_eq!(object_scale(&blast, 0., 0.5, -0.25), [1., 1.]);
    }
}
