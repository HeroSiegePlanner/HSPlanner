//! How game numbers and data keys are written on a page.
use hsplanner_engine::calc::skills::Ranged;

/// `fire_resistance` → `Fire resistance`.
pub fn readable(key: &str) -> String {
    let text = key.replace('_', " ");
    let mut characters = text.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

pub fn ranged((low, high): Ranged, percent: bool) -> String {
    let unit = if percent { "%" } else { "" };
    if compact(low) == compact(high) {
        return format!("{}{unit}", compact(low));
    }
    format!("{}–{}{unit}", compact(low), compact(high))
}

/// Short form of a game number: `950`, `12.3K`, `1.50B`.
pub fn compact(value: f64) -> String {
    let size = value.abs();
    if !value.is_finite() || size >= 1e15 {
        return format!("{value:.2e}");
    }
    for (limit, unit, suffix) in [
        (1e12, 1e12, "T"),
        (1e9, 1e9, "B"),
        (1e6, 1e6, "M"),
        (1e4, 1e3, "K"),
    ] {
        if size >= limit {
            let scaled = value / unit;
            let digits = match scaled.abs() {
                scaled if scaled >= 100. => 0,
                scaled if scaled >= 10. => 1,
                _ => 2,
            };
            return format!("{scaled:.digits$}{suffix}");
        }
    }
    if size >= 100. || value.fract() == 0. {
        return format!("{value:.0}");
    }
    let short = format!("{value:.2}");
    short.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_shorten_by_magnitude_and_ranges_collapse_when_equal() {
        for (value, expected) in [
            (0., "0"),
            (5., "5"),
            (24.95, "24.95"),
            (22.3255, "22.33"),
            (950.4, "950"),
            (9_999., "9999"),
            (12_345., "12.3K"),
            (1_500_000., "1.50M"),
            (2.5e9, "2.50B"),
            (123e12, "123T"),
        ] {
            assert_eq!(compact(value), expected);
        }
        assert_eq!(ranged((90., 90.), false), "90");
        assert_eq!(ranged((10., 25.5), true), "10–25.5%");
        assert_eq!(readable("fire_resistance"), "Fire resistance");
    }
}
