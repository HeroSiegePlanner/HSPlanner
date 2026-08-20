use super::{AttrMap, StatMap, r_max, rg};

/// Where a `conversion_*` subtree note takes its value from.
pub enum Source {
    Attribute(&'static str),
    Stat(&'static str),
    /// `attacks_per_second` after increased-attack-speed, the same product the
    /// attack path uses.
    EffectiveAttackSpeed,
    /// "A portion of the skill's own damage" — no external source.
    OwnDamage,
    /// How many entities *this* skill fields: sentries for a Sentry skill,
    /// summons for a Summon one. Never the other kind's count.
    EntityCount,
    /// Keys the build model cannot read (block rating, summon life); kept in the
    /// table so they are accounted for rather than resolving to something else.
    Unresolved,
}

pub enum Mode {
    /// `pct%` of the source becomes flat skill damage.
    Flat,
    /// `pct%` per full 500 points of the source becomes additive skill damage.
    Per500,
}

/// Resolved once from `mapping.json` -> `conversionSources`. A key carries one
/// mode here; the two `per500` outliers are listed in the E3 report.
pub const CONVERSIONS: &[(&str, Source, Mode)] = &[
    ("conversion_max_life", Source::Stat("life"), Mode::Flat),
    // The grizzly node (`spirit_of_the_ent:12`) reads life in 500-point steps;
    // a key carries exactly one mode, so it cannot share `conversion_max_life`.
    (
        "conversion_max_life_per500",
        Source::Stat("life"),
        Mode::Per500,
    ),
    ("conversion_max_mana", Source::Stat("mana"), Mode::Flat),
    ("conversion_energy", Source::Attribute("energy"), Mode::Flat),
    (
        "conversion_strength",
        Source::Attribute("strength"),
        Mode::Flat,
    ),
    (
        "conversion_vitality",
        Source::Attribute("vitality"),
        Mode::Flat,
    ),
    (
        "conversion_dexterity",
        Source::Attribute("dexterity"),
        Mode::Flat,
    ),
    (
        "conversion_intelligence",
        Source::Attribute("intelligence"),
        Mode::Flat,
    ),
    ("conversion_armor", Source::Attribute("armor"), Mode::Flat),
    ("conversion_block_rating", Source::Unresolved, Mode::Flat),
    (
        "conversion_explosion_aoe",
        Source::Stat("explosion_aoe"),
        Mode::Flat,
    ),
    (
        "conversion_movement_speed",
        Source::Stat("movement_speed"),
        Mode::Flat,
    ),
    (
        "conversion_attack_speed",
        Source::EffectiveAttackSpeed,
        Mode::Flat,
    ),
    (
        "conversion_damage_return",
        Source::Stat("damage_return"),
        Mode::Flat,
    ),
    (
        "conversion_summon_count",
        Source::EntityCount,
        Mode::Flat,
    ),
    ("conversion_summon_life", Source::Unresolved, Mode::Flat),
    ("conversion_skill_damage", Source::OwnDamage, Mode::Flat),
];

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Conversions {
    /// Added to the same slot as `flat_skill_damage`.
    pub flat: f64,
    /// Added to the same slot as `<element>_skill_damage`.
    pub skill_damage_pct: f64,
}

fn source_value(
    source: &Source,
    attributes: &AttrMap,
    stats: &StatMap,
    tags: &[String],
) -> Option<f64> {
    match source {
        Source::Attribute(key) => Some(r_max(rg(attributes, key))),
        Source::Stat(key) => Some(r_max(rg(stats, key))),
        Source::EntityCount => Some(
            crate::calc::affix_tags::sum_for(
                crate::calc::types::AffixEffect::MaxAmount,
                tags,
                stats,
            )
            .1,
        ),
        Source::EffectiveAttackSpeed => Some(
            r_max(rg(stats, "attacks_per_second"))
                * (1.0 + r_max(rg(stats, "increased_attack_speed")) / 100.0)
                * (1.0 + r_max(rg(stats, "increased_attack_speed_more")) / 100.0),
        ),
        Source::OwnDamage | Source::Unresolved => None,
    }
}

/// Turns the owning skill's `conversion_*` subtree values into their two damage
/// slots. Every such key is skill-scoped, so `scoped` is the only source.
pub fn resolve(
    scoped: &StatMap,
    attributes: &AttrMap,
    stats: &StatMap,
    tags: &[String],
) -> Conversions {
    let mut out = Conversions::default();
    for (key, source, mode) in CONVERSIONS {
        let pct = r_max(rg(scoped, key));
        if pct == 0.0 {
            continue;
        }
        if matches!(source, Source::OwnDamage) {
            out.skill_damage_pct += pct;
            continue;
        }
        let Some(value) = source_value(source, attributes, stats, tags) else {
            continue;
        };
        match mode {
            Mode::Flat => out.flat += pct / 100.0 * value,
            Mode::Per500 => out.skill_damage_pct += pct * (value / 500.0).floor(),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every case below is a tagless skill; the entity-count case passes its own.
    fn resolve(scoped: &StatMap, attributes: &AttrMap, stats: &StatMap) -> Conversions {
        super::resolve(scoped, attributes, stats, &[])
    }

    fn map(pairs: &[(&str, f64)]) -> StatMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), (*v, *v)))
            .collect()
    }

    #[test]
    fn flat_conversion_from_max_life() {
        // 10% of 1000 life = +100 flat damage.
        let out = resolve(
            &map(&[("conversion_max_life", 10.0)]),
            &AttrMap::new(),
            &map(&[("life", 1000.0)]),
        );
        assert_eq!(out.flat, 100.0);
        assert_eq!(out.skill_damage_pct, 0.0);
    }

    #[test]
    fn flat_conversion_from_an_attribute() {
        let out = resolve(
            &map(&[("conversion_vitality", 50.0)]),
            &map(&[("vitality", 400.0)]),
            &StatMap::new(),
        );
        assert_eq!(out.flat, 200.0);
    }

    #[test]
    fn armor_reads_the_attribute_not_a_stat() {
        let out = resolve(
            &map(&[("conversion_armor", 10.0)]),
            &map(&[("armor", 2000.0)]),
            &map(&[("armor", 999_999.0)]),
        );
        assert_eq!(out.flat, 200.0);
    }

    #[test]
    fn per500_conversion_floors_the_source() {
        // 1400 life = 2 full 500-point steps, so 6% per step = 12% added.
        let out = resolve(
            &map(&[("conversion_max_life_per500", 6.0)]),
            &AttrMap::new(),
            &map(&[("life", 1400.0)]),
        );
        assert_eq!(out.skill_damage_pct, 12.0);
        assert_eq!(out.flat, 0.0);
    }

    #[test]
    fn per500_and_flat_life_conversions_do_not_share_a_key() {
        let out = resolve(
            &map(&[("conversion_max_life", 10.0), ("conversion_max_life_per500", 6.0)]),
            &AttrMap::new(),
            &map(&[("life", 1000.0)]),
        );
        assert_eq!(out.flat, 100.0);
        assert_eq!(out.skill_damage_pct, 12.0);
    }

    #[test]
    fn own_damage_conversion_needs_no_source() {
        let out = resolve(
            &map(&[("conversion_skill_damage", 30.0)]),
            &AttrMap::new(),
            &StatMap::new(),
        );
        assert_eq!(out.skill_damage_pct, 30.0);
    }

    #[test]
    fn unresolved_sources_contribute_nothing() {
        let out = resolve(
            &map(&[
                ("conversion_block_rating", 50.0),
                ("conversion_summon_life", 50.0),
            ]),
            &AttrMap::new(),
            &map(&[("block_chance", 75.0), ("summon_life", 900.0)]),
        );
        assert_eq!(out, Conversions::default());
    }

    #[test]
    fn conversions_accumulate() {
        let out = resolve(
            &map(&[("conversion_max_life", 10.0), ("conversion_max_mana", 20.0)]),
            &AttrMap::new(),
            &map(&[("life", 1000.0), ("mana", 500.0)]),
        );
        assert_eq!(out.flat, 200.0);
    }

    #[test]
    fn missing_source_value_contributes_nothing() {
        let out = resolve(
            &map(&[("conversion_explosion_aoe", 25.0)]),
            &AttrMap::new(),
            &StatMap::new(),
        );
        assert_eq!(out, Conversions::default());
    }

    #[test]
    fn effective_attack_speed_includes_increased_attack_speed() {
        let out = resolve(
            &map(&[("conversion_attack_speed", 100.0)]),
            &AttrMap::new(),
            &map(&[
                ("attacks_per_second", 2.0),
                ("increased_attack_speed", 50.0),
            ]),
        );
        assert_eq!(out.flat, 3.0);
    }

    #[test]
    fn table_covers_every_conversion_key_in_game_config() {
        let declared: Vec<&str> = crate::calc::data::game_config()
            .stats
            .iter()
            .map(|s| s.key.as_str())
            .filter(|k| k.starts_with("conversion_"))
            .collect();
        for key in declared {
            assert!(
                CONVERSIONS.iter().any(|(k, _, _)| *k == key),
                "conversion table is missing {key}"
            );
        }
    }

    #[test]
    fn summon_count_conversion_reads_the_skill_s_own_entity() {
        let scoped = map(&[("conversion_summon_count", 10.0)]);
        let stats = map(&[("sentry_max_amount", 16.0), ("summon_max_amount", 4.0)]);
        let sentry = super::resolve(&scoped, &AttrMap::new(), &stats, &["Sentry".into()]);
        assert_eq!(sentry.flat, 1.6, "a Sentry skill counts sentries, not summons");
        let summon = super::resolve(&scoped, &AttrMap::new(), &stats, &["Summon".into()]);
        assert_eq!(summon.flat, 0.4);
        let neither = super::resolve(&scoped, &AttrMap::new(), &stats, &[]);
        assert_eq!(neither.flat, 0.0);
    }
}
