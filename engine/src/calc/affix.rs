use super::skills::Ranged;
use super::star_scaling::{
    is_stat_star_immune, stat_star_flat_bonus, stat_star_percent_bonus,
    stat_star_percent_multiplier,
};
pub use super::types::{Affix, AffixFormat, AffixSign};

pub fn rolled_affix_value(affix: &Affix, roll: f64) -> f64 {
    let (Some(min), Some(max)) = (affix.value_min, affix.value_max) else {
        return 0.0;
    };
    let raw = if min == max {
        max
    } else {
        min + (max - min) * roll
    };
    let rounded = match affix.format {
        AffixFormat::Flat => raw.round(),
        AffixFormat::Percent => raw,
    };
    match affix.sign {
        AffixSign::Minus => -rounded,
        AffixSign::Plus => rounded,
    }
}

pub fn rolled_affix_range(affix: &Affix) -> Ranged {
    let (Some(min), Some(max)) = (affix.value_min, affix.value_max) else {
        return (0.0, 0.0);
    };
    let round = |n: f64| -> f64 {
        match affix.format {
            AffixFormat::Flat => n.round(),
            AffixFormat::Percent => n,
        }
    };
    let lo = round(min);
    let hi = round(max);
    match affix.sign {
        AffixSign::Minus => (-hi, -lo),
        AffixSign::Plus => (lo, hi),
    }
}

pub fn is_affix_star_immune(stat_key: Option<&str>) -> bool {
    is_stat_star_immune(stat_key)
}

pub fn affix_star_multiplier(stat_key: Option<&str>, stars: Option<u32>) -> f64 {
    stat_star_percent_multiplier(stat_key, stars)
}

// CreateItemCheckGenerationCases (0x14070d1a6): floor only whole starting values.
// Its percent helpers (0x14070d5a0 / 0x14070d640) add base * bonus * 0.01
// to the base; preserving that order avoids losing whole points to float error.
fn star_scaled(base: f64, percent: f64, flat: f64) -> f64 {
    let scaled = base + (base * percent) * 0.01 + flat;
    if base.fract() == 0.0 {
        scaled.floor()
    } else {
        scaled
    }
}

pub fn rolled_affix_value_with_stars(affix: &Affix, roll: f64, stars: Option<u32>) -> f64 {
    let base = rolled_affix_value(affix, roll);
    let stat_key = affix.stat_key.as_deref();
    let percent = stat_star_percent_bonus(stat_key, stars);
    let flat = stat_star_flat_bonus(stat_key, stars);
    if base == 0.0 && flat == 0.0 {
        return 0.0;
    }
    let direction = match affix.sign {
        AffixSign::Minus => -1.0,
        AffixSign::Plus => 1.0,
    };
    let stars_active = percent != 0.0 || flat != 0.0;
    if stars_active {
        return star_scaled(base, percent, flat * direction);
    }
    match affix.format {
        AffixFormat::Flat => base.round(),
        AffixFormat::Percent => base,
    }
}

pub fn apply_stars_to_ranged_value(value: Ranged, stat_key: &str, stars: Option<u32>) -> Ranged {
    let s = stars.unwrap_or(0);
    if s == 0 {
        return value;
    }
    let flat = stat_star_flat_bonus(Some(stat_key), Some(s));
    let percent = stat_star_percent_bonus(Some(stat_key), Some(s));
    if percent == 0.0 && flat == 0.0 {
        return value;
    }
    let (min, max) = value;
    (
        star_scaled(min, percent, flat),
        star_scaled(max, percent, flat),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aff(sign: AffixSign, format: AffixFormat, vmin: f64, vmax: f64, key: &str) -> Affix {
        Affix {
            sign,
            format,
            value_min: Some(vmin),
            value_max: Some(vmax),
            stat_key: Some(key.to_string()),
            ..Default::default()
        }
    }

    fn aff_no_value(sign: AffixSign, format: AffixFormat, key: &str) -> Affix {
        Affix {
            sign,
            format,
            value_min: None,
            value_max: None,
            stat_key: Some(key.to_string()),
            ..Default::default()
        }
    }

    // ---- rolled_affix_value ----

    #[test]
    fn rolled_value_null_endpoints_returns_zero() {
        let a = aff_no_value(AffixSign::Plus, AffixFormat::Flat, "to_strength");
        assert_eq!(rolled_affix_value(&a, 0.5), 0.0);
    }

    #[test]
    fn rolled_value_flat_rounds() {
        let a = aff(
            AffixSign::Plus,
            AffixFormat::Flat,
            10.0,
            20.0,
            "to_strength",
        );
        assert_eq!(rolled_affix_value(&a, 0.5), 15.0);
        assert_eq!(rolled_affix_value(&a, 0.33), 13.0);
        assert_eq!(rolled_affix_value(&a, 1.0), 20.0);
        assert_eq!(rolled_affix_value(&a, 0.0), 10.0);
    }

    #[test]
    fn rolled_value_percent_no_rounding() {
        let a = aff(
            AffixSign::Plus,
            AffixFormat::Percent,
            10.0,
            20.0,
            "increased_life",
        );
        assert!((rolled_affix_value(&a, 0.33) - 13.3).abs() < 1e-12);
    }

    #[test]
    fn rolled_value_negative_sign_flips() {
        let a = aff(
            AffixSign::Minus,
            AffixFormat::Flat,
            10.0,
            20.0,
            "to_strength",
        );
        assert_eq!(rolled_affix_value(&a, 0.5), -15.0);
        let p = aff(
            AffixSign::Minus,
            AffixFormat::Percent,
            10.0,
            20.0,
            "cold_resistance",
        );
        assert!((rolled_affix_value(&p, 0.5) - (-15.0)).abs() < 1e-12);
    }

    #[test]
    fn rolled_value_collapsed_when_min_equals_max() {
        let a = aff(
            AffixSign::Plus,
            AffixFormat::Flat,
            12.0,
            12.0,
            "to_strength",
        );
        assert_eq!(rolled_affix_value(&a, 0.0), 12.0);
        assert_eq!(rolled_affix_value(&a, 1.0), 12.0);
        assert_eq!(rolled_affix_value(&a, 0.5), 12.0);
    }

    // ---- rolled_affix_range ----

    #[test]
    fn rolled_range_null_endpoints_returns_zero_zero() {
        let a = aff_no_value(AffixSign::Plus, AffixFormat::Flat, "to_strength");
        assert_eq!(rolled_affix_range(&a), (0.0, 0.0));
    }

    #[test]
    fn rolled_range_positive_signed() {
        let a = aff(
            AffixSign::Plus,
            AffixFormat::Flat,
            12.0,
            18.0,
            "to_strength",
        );
        assert_eq!(rolled_affix_range(&a), (12.0, 18.0));
        let p = aff(
            AffixSign::Plus,
            AffixFormat::Percent,
            12.5,
            18.7,
            "increased_life",
        );
        assert_eq!(rolled_affix_range(&p), (12.5, 18.7));
    }

    #[test]
    fn rolled_range_negative_flips_endpoints() {
        let a = aff(
            AffixSign::Minus,
            AffixFormat::Flat,
            10.0,
            20.0,
            "to_strength",
        );
        assert_eq!(rolled_affix_range(&a), (-20.0, -10.0));
        let eq = aff(
            AffixSign::Minus,
            AffixFormat::Flat,
            12.0,
            12.0,
            "to_strength",
        );
        assert_eq!(rolled_affix_range(&eq), (-12.0, -12.0));
    }

    // ---- rolled_affix_value_with_stars ----

    #[test]
    fn with_stars_no_stars_matches_unscaled() {
        let a = aff(
            AffixSign::Plus,
            AffixFormat::Flat,
            10.0,
            20.0,
            "to_strength",
        );
        // stars=None and stars=Some(0) both fall through to format-aware rounding
        // (Flat → round, Percent → as-is) so the result equals rolled_affix_value.
        assert_eq!(rolled_affix_value_with_stars(&a, 0.5, None), 15.0);
        assert_eq!(rolled_affix_value_with_stars(&a, 0.5, Some(0)), 15.0);
    }

    #[test]
    fn with_stars_percent_multiplier_floors() {
        // to_strength = percent per_star=5. stars=5 → mult=1.25.
        // base=15, scaled=18.75, floor → 18.
        let a = aff(
            AffixSign::Plus,
            AffixFormat::Flat,
            10.0,
            20.0,
            "to_strength",
        );
        assert_eq!(rolled_affix_value_with_stars(&a, 0.5, Some(5)), 18.0);
    }

    #[test]
    fn with_stars_flat_skill_staircase() {
        // fire_skills = flat-skill-staircase. stars=5 → flat=+2, mult=1.
        // base=1, scaled = 1*1 + 2*1 = 3.
        let a = aff(AffixSign::Plus, AffixFormat::Flat, 1.0, 1.0, "fire_skills");
        assert_eq!(rolled_affix_value_with_stars(&a, 0.5, Some(5)), 3.0);
    }

    #[test]
    fn with_stars_immune_stat_falls_back_to_format_rounding() {
        // attacks_per_second = kind None. mult=1, flat=0 → stars_active=false.
        // Percent format keeps fractional; scaled = base = 1.2.
        let a = aff(
            AffixSign::Plus,
            AffixFormat::Percent,
            1.0,
            1.4,
            "attacks_per_second",
        );
        assert!((rolled_affix_value_with_stars(&a, 0.5, Some(5)) - 1.2).abs() < 1e-12);
    }

    #[test]
    fn with_stars_negative_sign_applies_direction_to_flat() {
        // sign=-, fire_skills. base = -1, flat=2, direction=-1.
        // scaled = -1*1 + 2*(-1) = -3.
        let a = aff(AffixSign::Minus, AffixFormat::Flat, 1.0, 1.0, "fire_skills");
        assert_eq!(rolled_affix_value_with_stars(&a, 0.5, Some(5)), -3.0);
    }

    // ---- apply_stars_to_ranged_value ----

    #[test]
    fn summon_and_sentry_star_bonuses_add_ranks_instead_of_percentages() {
        // GetStatUpgrades IDs 331/335/337 set the flat flag and a 0.2 step
        // at 0x14397261a / 0x14397271d / 0x1439727a0.
        for key in ["summon_skills", "sentry_skills", "sentry_max_amount"] {
            assert_eq!(
                apply_stars_to_ranged_value((1., 3.), key, Some(4)),
                (1., 3.),
                "{key}"
            );
            assert_eq!(
                apply_stars_to_ranged_value((1., 3.), key, Some(5)),
                (2., 4.),
                "{key}"
            );
        }
    }

    #[test]
    fn percent_stars_preserve_whole_results_at_float_boundaries() {
        // CreateItemCheckGenerationCases uses base + (base * bonus) * 0.01
        // before flooring. Folding that into a multiplier can lose a point.
        for (key, base, stars, expected) in [
            ("magic_find", 100., 5, 115.),
            ("life", 25., 4, 29.),
            ("increased_strength", 45., 4, 63.),
        ] {
            assert_eq!(
                apply_stars_to_ranged_value((base, base), key, Some(stars)),
                (expected, expected)
            );
            let a = aff(AffixSign::Plus, AffixFormat::Flat, base, base, key);
            assert_eq!(rolled_affix_value_with_stars(&a, 1., Some(stars)), expected);
        }
        assert_eq!(
            apply_stars_to_ranged_value((12.5, 12.5), "magic_find", Some(5)),
            (14.375, 14.375)
        );
    }

    #[test]
    fn apply_stars_zero_returns_input() {
        let v = (10.0, 20.0);
        assert_eq!(apply_stars_to_ranged_value(v, "to_strength", None), v);
        assert_eq!(apply_stars_to_ranged_value(v, "to_strength", Some(0)), v);
    }

    #[test]
    fn apply_stars_percent_scales_both_endpoints_then_floors() {
        // to_strength percent per_star=5, 5 stars → mult=1.25
        // (10, 20) → (12.5, 25) → floor (12, 25)
        let v = (10.0, 20.0);
        assert_eq!(
            apply_stars_to_ranged_value(v, "to_strength", Some(5)),
            (12.0, 25.0)
        );
    }

    #[test]
    fn apply_stars_flat_skill_staircase() {
        // fire_skills, 3 stars → flat=+1, mult=1.
        // (2, 4) → (3, 5)
        let v = (2.0, 4.0);
        assert_eq!(
            apply_stars_to_ranged_value(v, "fire_skills", Some(3)),
            (3.0, 5.0)
        );
    }

    #[test]
    fn apply_stars_item_granted_skill_rank_uses_item_specific_table() {
        // Synthetic key bypasses STAR_SCALE_MAP and reads ITEM_SPECIFIC_STAIRCASE.
        // 4 stars → +2 flat.
        let v = (1.0, 2.0);
        assert_eq!(
            apply_stars_to_ranged_value(v, "item_granted_skill_rank", Some(4)),
            (3.0, 4.0)
        );
    }

    #[test]
    fn apply_stars_immune_stat_returns_input() {
        // attacks_per_second is kind None → mult=1, flat=0 → bail without floor
        let v = (1.0, 1.4);
        assert_eq!(
            apply_stars_to_ranged_value(v, "attacks_per_second", Some(5)),
            v
        );
    }

    // ---- thin wrappers ----

    #[test]
    fn wrappers_delegate_to_star_scaling() {
        assert!(is_affix_star_immune(Some("attacks_per_second")));
        assert!(!is_affix_star_immune(Some("to_strength")));
        assert!((affix_star_multiplier(Some("to_strength"), Some(5)) - 1.25).abs() < 1e-12);
        assert_eq!(affix_star_multiplier(Some("to_strength"), None), 1.0);
    }
}
