use once_cell::sync::Lazy;
use regex::Regex;

use super::skills::Ranged;

static RANGE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([+-]?\d+(?:\.\d+)?)\s*-\s*([+-]?\d+(?:\.\d+)?)$").unwrap());

pub fn parse_custom_stat_value(raw: &str) -> Option<Ranged> {
    let stripped: String = raw
        .trim()
        .chars()
        .filter(|c| *c != '[' && *c != ']' && *c != '%')
        .collect();
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(caps) = RANGE_RE.captures(trimmed) {
        let a: f64 = caps[1].parse().ok()?;
        let b: f64 = caps[2].parse().ok()?;
        if !a.is_finite() || !b.is_finite() {
            return None;
        }
        // Normalise so the Ranged invariant (.0 <= .1) holds for "80-50".
        return Some((a.min(b), a.max(b)));
    }

    let num: f64 = trimmed.parse().ok()?;
    if !num.is_finite() {
        return None;
    }
    Some((num, num))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_none() {
        assert_eq!(parse_custom_stat_value(""), None);
        assert_eq!(parse_custom_stat_value("   "), None);
        assert_eq!(parse_custom_stat_value("[]"), None);
        assert_eq!(parse_custom_stat_value("%"), None);
        assert_eq!(parse_custom_stat_value("[%]"), None);
    }

    #[test]
    fn single_integer() {
        assert_eq!(parse_custom_stat_value("100"), Some((100.0, 100.0)));
        assert_eq!(parse_custom_stat_value("+12"), Some((12.0, 12.0)));
        assert_eq!(parse_custom_stat_value("-5"), Some((-5.0, -5.0)));
        assert_eq!(parse_custom_stat_value("0"), Some((0.0, 0.0)));
    }

    #[test]
    fn percent_suffix_stripped() {
        assert_eq!(parse_custom_stat_value("100%"), Some((100.0, 100.0)));
        assert_eq!(parse_custom_stat_value("50.5%"), Some((50.5, 50.5)));
        assert_eq!(parse_custom_stat_value("-25%"), Some((-25.0, -25.0)));
    }

    #[test]
    fn brackets_stripped() {
        assert_eq!(parse_custom_stat_value("[42]"), Some((42.0, 42.0)));
        assert_eq!(parse_custom_stat_value("[12-18]"), Some((12.0, 18.0)));
        assert_eq!(parse_custom_stat_value("[12-18]%"), Some((12.0, 18.0)));
        assert_eq!(parse_custom_stat_value("  [+3]  "), Some((3.0, 3.0)));
    }

    #[test]
    fn range_pattern() {
        assert_eq!(parse_custom_stat_value("12-18"), Some((12.0, 18.0)));
        assert_eq!(parse_custom_stat_value("12 - 18"), Some((12.0, 18.0)));
        assert_eq!(parse_custom_stat_value("50-80"), Some((50.0, 80.0)));
        assert_eq!(parse_custom_stat_value("-3 - +7"), Some((-3.0, 7.0)));
        assert_eq!(parse_custom_stat_value("-5--3"), Some((-5.0, -3.0)));
    }

    #[test]
    fn decimal_values() {
        assert_eq!(parse_custom_stat_value("0.5"), Some((0.5, 0.5)));
        assert_eq!(parse_custom_stat_value("3.14159"), Some((3.14159, 3.14159)));
        assert_eq!(parse_custom_stat_value("1.5-2.5"), Some((1.5, 2.5)));
    }

    #[test]
    fn garbage_returns_none() {
        assert_eq!(parse_custom_stat_value("abc"), None);
        assert_eq!(parse_custom_stat_value("12abc"), None);
        assert_eq!(parse_custom_stat_value("12.3.4"), None);
        assert_eq!(parse_custom_stat_value("--"), None);
        assert_eq!(parse_custom_stat_value("5-"), None);
        assert_eq!(parse_custom_stat_value("-"), None);
    }

    #[test]
    fn equal_endpoints_kept_as_pair() {
        // TS collapses min === max to a scalar (number) but the Rust calc
        // layer normalises everything to Ranged = (f64, f64). Both encodings
        // round-trip to the same downstream stat value, so parity holds.
        assert_eq!(parse_custom_stat_value("12-12"), Some((12.0, 12.0)));
    }

    #[test]
    fn matches_fixture_inputs() {
        // The class_with_custom_stat parity fixture passes these three strings
        // through computeBuildStatsCore -> parseCustomStatValue. Asserting them
        // explicitly here pins the Rust output against the same baseline.
        assert_eq!(parse_custom_stat_value("+2"), Some((2.0, 2.0)));
        assert_eq!(parse_custom_stat_value("15"), Some((15.0, 15.0)));
        assert_eq!(parse_custom_stat_value("50-80"), Some((50.0, 80.0)));
    }
}
