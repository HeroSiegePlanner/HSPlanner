use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StarScaleConfig {
    Percent { per_star: f64 },
    FlatSkillStaircase,
    ItemSpecificStaircase,
    None,
    Unknown,
    Glitch,
}

// Item-specific stairs: 2*=+1, 4*=+2, 5*=+3.
const ITEM_SPECIFIC_STAIRCASE: [f64; 6] = [0.0, 0.0, 1.0, 1.0, 2.0, 3.0];
// Flat-skill stairs: 3*=+1, 5*=+2.
const FLAT_SKILL_STAIRCASE: [f64; 6] = [0.0, 0.0, 0.0, 1.0, 1.0, 2.0];

#[inline]
fn percent(per_star: f64) -> StarScaleConfig {
    StarScaleConfig::Percent { per_star }
}

static STAR_SCALE_MAP: Lazy<HashMap<&'static str, StarScaleConfig>> = Lazy::new(|| {
    use StarScaleConfig::*;
    let entries: &[(&'static str, StarScaleConfig)] = &[
        ("to_strength", percent(5.0)),
        ("to_dexterity", percent(5.0)),
        ("to_intelligence", percent(5.0)),
        ("to_energy", percent(5.0)),
        ("to_vitality", percent(5.0)),
        ("to_armor", percent(5.0)),
        ("all_attributes", percent(4.0)),
        ("increased_all_attributes", percent(10.0)),
        ("fire_resistance", percent(5.0)),
        ("cold_resistance", percent(5.0)),
        ("lightning_resistance", percent(5.0)),
        ("poison_resistance", percent(5.0)),
        ("arcane_resistance", percent(5.0)),
        ("all_resistances", percent(3.0)),
        ("max_fire_resistance", Unknown),
        ("max_cold_resistance", Unknown),
        ("max_lightning_resistance", Unknown),
        ("max_poison_resistance", Unknown),
        ("max_arcane_resistance", Unknown),
        ("max_all_resistances", Unknown),
        ("fire_absorption", None),
        ("cold_absorption", None),
        ("lightning_absorption", None),
        ("poison_absorption", None),
        ("arcane_absorption", None),
        ("magic_absorption", None),
        ("all_skills", None),
        ("fire_skills", FlatSkillStaircase),
        ("cold_skills", FlatSkillStaircase),
        ("lightning_skills", FlatSkillStaircase),
        ("poison_skills", FlatSkillStaircase),
        ("arcane_skills", FlatSkillStaircase),
        ("physical_skills", percent(8.0)),
        ("summon_skills", Unknown),
        ("explosion_skills", None),
        ("magic_skill_damage", percent(3.0)),
        ("fire_skill_damage", percent(4.0)),
        ("cold_skill_damage", percent(4.0)),
        ("lightning_skill_damage", percent(4.0)),
        ("poison_skill_damage", percent(4.0)),
        ("arcane_skill_damage", percent(4.0)),
        ("additive_physical_damage", percent(5.0)),
        ("additive_fire_damage", percent(5.0)),
        ("additive_cold_damage", percent(5.0)),
        ("additive_lightning_damage", percent(5.0)),
        ("additive_poison_damage", percent(5.0)),
        ("additive_arcane_damage", percent(5.0)),
        ("flat_fire_skill_damage", percent(5.0)),
        ("flat_cold_skill_damage", percent(5.0)),
        ("flat_lightning_skill_damage", percent(5.0)),
        ("flat_poison_skill_damage", percent(5.0)),
        ("flat_arcane_skill_damage", percent(5.0)),
        ("flat_skill_damage", percent(5.0)),
        ("flat_elemental_skill_damage", percent(5.0)),
        ("attack_damage", percent(3.0)),
        ("enhanced_damage", percent(3.0)),
        ("enhanced_defense", None),
        ("damage_return", percent(5.0)),
        ("defense", None),
        ("defense_vs_missiles", percent(4.0)),
        ("damage_taken_reduced", percent(4.0)),
        ("all_damage_taken_reduced_pct", percent(4.0)),
        ("damage_recouped_as_life", None),
        ("damage_recouped_as_mana", None),
        ("magic_damage_reduction", Glitch),
        ("physical_damage_reduction", Glitch),
        ("ignore_fire_res", percent(3.0)),
        ("ignore_cold_res", percent(3.0)),
        ("ignore_lightning_res", percent(3.0)),
        ("ignore_poison_res", percent(3.0)),
        ("ignore_arcane_res", percent(3.0)),
        ("life", percent(4.0)),
        ("mana", percent(5.0)),
        ("increased_life", percent(4.0)),
        ("increased_mana", percent(4.0)),
        ("life_replenish", percent(4.0)),
        ("mana_replenish", percent(4.0)),
        ("life_replenish_pct", percent(4.0)),
        ("mana_replenish_pct", percent(4.0)),
        ("life_per_kill", percent(4.0)),
        ("mana_per_kill", percent(4.0)),
        ("life_steal", percent(2.0)),
        ("mana_steal", percent(2.0)),
        ("mana_cost_reduction", None),
        ("faster_cast_rate", percent(3.0)),
        ("faster_hit_recovery", percent(4.0)),
        ("movement_speed", percent(4.0)),
        ("jumping_power", percent(4.0)),
        ("light_radius", None),
        ("experience_gain", None),
        ("magic_find", percent(3.0)),
        ("gold_find", percent(3.0)),
        ("merchant_prices", None),
        ("increased_attack_speed", percent(3.0)),
        ("attacks_per_second", None),
        ("attack_rating", percent(5.0)),
        ("attack_rating_pct", percent(4.0)),
        ("attack_speed_below_40_life", percent(3.0)),
        ("deadly_blow", None),
        ("crit_chance", None),
        ("crit_damage", percent(4.0)),
        ("crushing_blow_chance", None),
        ("open_wounds", None),
        ("projectile_size", percent(2.0)),
        ("projectile_speed", None),
        ("skill_haste", percent(3.0)),
        ("area_of_effect", None),
        ("ranged_range", None),
        ("extra_damage_bleeding", percent(4.0)),
        ("extra_damage_burning", Unknown),
        ("extra_damage_stunned", percent(3.0)),
        ("extra_damage_poisoned", percent(3.0)),
        ("extra_damage_stasis", percent(3.0)),
        ("extra_damage_frozen", percent(3.0)),
        ("extra_damage_frost_bitten", Unknown),
        ("extra_damage_shadow_burning", percent(4.0)),
        ("extra_damage_ailments", percent(3.0)),
        ("explosion_damage", percent(3.0)),
        ("explosion_aoe", None),
        ("poison_length_reduced", None),
    ];
    entries.iter().copied().collect()
});

pub fn get_star_scale_config(stat_key: Option<&str>) -> StarScaleConfig {
    let Some(key) = stat_key else {
        return StarScaleConfig::None;
    };
    STAR_SCALE_MAP
        .get(key)
        .copied()
        .unwrap_or(StarScaleConfig::None)
}

pub fn is_stat_star_immune(stat_key: Option<&str>) -> bool {
    matches!(
        get_star_scale_config(stat_key),
        StarScaleConfig::None | StarScaleConfig::Unknown | StarScaleConfig::Glitch
    )
}

pub fn stat_star_percent_multiplier(stat_key: Option<&str>, stars: Option<u32>) -> f64 {
    let s = stars.unwrap_or(0);
    if s == 0 {
        return 1.0;
    }
    match get_star_scale_config(stat_key) {
        StarScaleConfig::Percent { per_star } => 1.0 + (s as f64 * per_star) / 100.0,
        _ => 1.0,
    }
}

pub fn stat_star_flat_bonus(stat_key: Option<&str>, stars: Option<u32>) -> f64 {
    let s = stars.unwrap_or(0);
    if s == 0 {
        return 0.0;
    }
    match get_star_scale_config(stat_key) {
        StarScaleConfig::FlatSkillStaircase => FLAT_SKILL_STAIRCASE
            .get(s as usize)
            .copied()
            .unwrap_or(0.0),
        StarScaleConfig::ItemSpecificStaircase => ITEM_SPECIFIC_STAIRCASE
            .get(s as usize)
            .copied()
            .unwrap_or(0.0),
        _ => 0.0,
    }
}

pub fn item_granted_skill_rank_flat_bonus(stars: Option<u32>) -> f64 {
    let s = stars.unwrap_or(0);
    if s == 0 {
        return 0.0;
    }
    ITEM_SPECIFIC_STAIRCASE
        .get(s as usize)
        .copied()
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_treated_as_none() {
        assert_eq!(get_star_scale_config(None), StarScaleConfig::None);
        assert_eq!(
            get_star_scale_config(Some("unknown_stat_key")),
            StarScaleConfig::None
        );
    }

    #[test]
    fn percent_kind_lookup() {
        assert_eq!(
            get_star_scale_config(Some("to_strength")),
            StarScaleConfig::Percent { per_star: 5.0 }
        );
        assert_eq!(
            get_star_scale_config(Some("increased_all_attributes")),
            StarScaleConfig::Percent { per_star: 10.0 }
        );
        assert_eq!(
            get_star_scale_config(Some("physical_skills")),
            StarScaleConfig::Percent { per_star: 8.0 }
        );
    }

    #[test]
    fn staircase_and_special_kinds() {
        assert_eq!(
            get_star_scale_config(Some("fire_skills")),
            StarScaleConfig::FlatSkillStaircase
        );
        assert_eq!(
            get_star_scale_config(Some("cold_skills")),
            StarScaleConfig::FlatSkillStaircase
        );
        assert_eq!(
            get_star_scale_config(Some("max_fire_resistance")),
            StarScaleConfig::Unknown
        );
        assert_eq!(
            get_star_scale_config(Some("magic_damage_reduction")),
            StarScaleConfig::Glitch
        );
        assert_eq!(
            get_star_scale_config(Some("attacks_per_second")),
            StarScaleConfig::None
        );
    }

    #[test]
    fn stars_zero_or_missing_returns_identity() {
        assert_eq!(stat_star_percent_multiplier(Some("to_strength"), None), 1.0);
        assert_eq!(stat_star_percent_multiplier(Some("to_strength"), Some(0)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(0)), 0.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(0)), 0.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(None), 0.0);
    }

    #[test]
    fn percent_multiplier_math() {
        // to_strength = percent 5, 3 stars => 1 + 3*5/100 = 1.15
        assert!((stat_star_percent_multiplier(Some("to_strength"), Some(3)) - 1.15).abs() < 1e-12);
        // to_strength, 5 stars => 1.25
        assert!((stat_star_percent_multiplier(Some("to_strength"), Some(5)) - 1.25).abs() < 1e-12);
        // increased_all_attributes = percent 10, 5 stars => 1.5
        assert!(
            (stat_star_percent_multiplier(Some("increased_all_attributes"), Some(5)) - 1.5).abs()
                < 1e-12
        );
        // physical_skills = percent 8, 5 stars => 1.4
        assert!(
            (stat_star_percent_multiplier(Some("physical_skills"), Some(5)) - 1.4).abs() < 1e-12
        );
    }

    #[test]
    fn flat_skill_staircase_values() {
        // 1*=0, 2*=0, 3*=+1, 4*=+1, 5*=+2
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(1)), 0.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(2)), 0.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(3)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(4)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(5)), 2.0);
    }

    #[test]
    fn item_specific_staircase_values() {
        // 1*=0, 2*=+1, 3*=+1, 4*=+2, 5*=+3
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(1)), 0.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(2)), 1.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(3)), 1.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(4)), 2.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(5)), 3.0);
    }

    #[test]
    fn out_of_range_stars_returns_zero() {
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(99)), 0.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(42)), 0.0);
    }

    #[test]
    fn percent_kind_yields_no_flat_bonus() {
        // percent stats use multiplier, not flat staircase
        assert_eq!(stat_star_flat_bonus(Some("to_strength"), Some(5)), 0.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skill_damage"), Some(5)), 0.0);
    }

    #[test]
    fn none_unknown_glitch_no_scaling() {
        // None
        assert_eq!(stat_star_percent_multiplier(Some("attacks_per_second"), Some(5)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("attacks_per_second"), Some(5)), 0.0);
        // Unknown
        assert_eq!(stat_star_percent_multiplier(Some("max_all_resistances"), Some(5)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("max_all_resistances"), Some(5)), 0.0);
        // Glitch
        assert_eq!(stat_star_percent_multiplier(Some("magic_damage_reduction"), Some(5)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("magic_damage_reduction"), Some(5)), 0.0);
    }

    #[test]
    fn star_immune_classification() {
        assert!(is_stat_star_immune(Some("attacks_per_second"))); // None
        assert!(is_stat_star_immune(Some("max_all_resistances"))); // Unknown
        assert!(is_stat_star_immune(Some("magic_damage_reduction"))); // Glitch
        assert!(is_stat_star_immune(None)); // missing
        assert!(!is_stat_star_immune(Some("to_strength"))); // Percent
        assert!(!is_stat_star_immune(Some("fire_skills"))); // FlatSkillStaircase
    }
}
