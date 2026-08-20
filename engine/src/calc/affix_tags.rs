//! One table (`data/affix-tags.json`) says which skill tags each tag-scoped
//! affix works with rank, damage and entity code all read it instead of
//! keeping their own tag lists.

use super::data;
use super::skills::{Ranged, StatMap, r_max, r_min, rg};
use super::types::AffixEffect;

/// Stat keys of `effect` whose required tags the skill all carries.
pub fn keys_for(effect: AffixEffect, tags: &[String]) -> Vec<&'static str> {
    data::affix_tags()
        .iter()
        .filter(|(_, def)| {
            def.effect == effect && def.tags.iter().all(|need| tags.iter().any(|t| t == need))
        })
        .map(|(key, _)| key.as_str())
        .collect()
}

pub fn sum_for(effect: AffixEffect, tags: &[String], stats: &StatMap) -> Ranged {
    keys_for(effect, tags).iter().fold((0.0, 0.0), |acc, key| {
        let v = rg(stats, key);
        (acc.0 + r_min(v), acc.1 + r_max(v))
    })
}

/// `_more` stats stack multiplicatively, so they need their own combiner.
pub fn more_for(effect: AffixEffect, tags: &[String], stats: &StatMap) -> Ranged {
    keys_for(effect, tags).iter().fold((1.0, 1.0), |acc, key| {
        let v = rg(stats, key);
        (
            acc.0 * (1.0 + r_min(v) / 100.0),
            acc.1 * (1.0 + r_max(v) / 100.0),
        )
    })
}

/// The entity kind this skill fields: "Sentry", "Summon" or "Guardian" - three
/// separate things, never interchangeable.
pub fn entity_tag_for(tags: &[String]) -> Option<&'static str> {
    data::affix_tags()
        .values()
        .find(|def| {
            def.effect == AffixEffect::AttackSpeed
                && def.tags.iter().all(|need| tags.iter().any(|t| t == need))
        })
        .and_then(|def| def.tags.first())
        .map(|t| t.as_str())
}

pub fn is_entity(tags: &[String]) -> bool {
    entity_tag_for(tags).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn tags(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn stats(pairs: &[(&str, f64)]) -> StatMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), (*v, *v)))
            .collect::<HashMap<_, _>>()
    }

    #[test]
    fn rank_keys_need_the_tag() {
        assert_eq!(
            keys_for(AffixEffect::Rank, &tags(&["Spell", "Sentry"])),
            vec!["sentry_skills"]
        );
        assert!(keys_for(AffixEffect::Rank, &tags(&["Spell"])).is_empty());
    }

    #[test]
    fn compound_keys_need_every_tag() {
        let both = tags(&["Spell", "Area of Effect"]);
        assert!(keys_for(AffixEffect::Damage, &both).contains(&"spell_aoe_damage"));
        assert!(
            !keys_for(AffixEffect::Damage, &tags(&["Spell"])).contains(&"spell_aoe_damage"),
            "Spell alone must not pick up the Spell+AoE affix"
        );
    }

    #[test]
    fn sum_adds_every_matching_key() {
        let s = stats(&[("sentry_damage", 30.0), ("explosion_damage", 20.0)]);
        assert_eq!(
            sum_for(AffixEffect::Damage, &tags(&["Sentry", "Explosion"]), &s),
            (50.0, 50.0)
        );
        assert_eq!(
            sum_for(AffixEffect::Damage, &tags(&["Sentry"]), &s),
            (30.0, 30.0)
        );
    }

    #[test]
    fn more_multiplies_every_matching_key() {
        let s = stats(&[
            ("sentry_damage_more", 50.0),
            ("orbital_skill_damage_more", 100.0),
        ]);
        let m = more_for(AffixEffect::DamageMore, &tags(&["Sentry", "Orbital"]), &s);
        assert!((m.0 - 3.0).abs() < 1e-9, "1.5 * 2.0, got {}", m.0);
        assert_eq!(more_for(AffixEffect::DamageMore, &tags(&[]), &s), (1.0, 1.0));
    }

    #[test]
    fn entity_tags_are_the_ones_with_an_attack_speed_key() {
        assert_eq!(entity_tag_for(&tags(&["Spell", "Sentry"])), Some("Sentry"));
        assert_eq!(entity_tag_for(&tags(&["Summon"])), Some("Summon"));
        assert_eq!(entity_tag_for(&tags(&["Guardian"])), Some("Guardian"));
        assert_eq!(entity_tag_for(&tags(&["Spell", "Projectile"])), None);
        assert!(is_entity(&tags(&["Sentry"])));
    }
}
