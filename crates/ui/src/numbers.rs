//! Compact number labels (k/M suffixes) matching the legacy planner output.
//!
//! Formatting follows the system's preferred language. English and Polish rules
//! are verified against Intl.NumberFormat; other locales currently use English.
//! Scale thresholds are chosen before rounding, and are independent of locale.

use std::sync::OnceLock;

#[derive(Clone, Copy)]
enum NumberLocale {
    English,
    Polish,
}

impl NumberLocale {
    fn from_tag(tag: &str) -> Self {
        match tag
            .split(['-', '_'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "pl" => Self::Polish,
            _ => Self::English,
        }
    }

    fn system() -> Self {
        static LOCALE: OnceLock<NumberLocale> = OnceLock::new();
        *LOCALE.get_or_init(|| Self::from_tag(&sys_locale::get_locale().unwrap_or_default()))
    }

    fn separators(self) -> (char, char, usize) {
        match self {
            Self::English => (',', '.', 4),
            Self::Polish => ('\u{a0}', ',', 5),
        }
    }
}

/// Format a number using the saved `none`, `thousands`, `millions` or `billions` cap.
/// A thousands suffix starts at 10,000, exactly as in the Tauri reference.
pub fn compact(value: f64, scale: &str) -> String {
    compact_in(value, scale, NumberLocale::system())
}

/// Collapse near-equal bounds using the first bound; otherwise join with an en dash.
pub fn compact_range(value: (f64, f64), scale: &str) -> String {
    compact_range_in(value, scale, NumberLocale::system())
}

fn compact_range_in((low, high): (f64, f64), scale: &str, locale: NumberLocale) -> String {
    let low_text = compact_in(low, scale, locale);
    if (low - high).abs() < 0.5 {
        low_text
    } else {
        format!("{low_text}–{}", compact_in(high, scale, locale))
    }
}

fn compact_in(value: f64, scale: &str, locale: NumberLocale) -> String {
    let cap = match scale {
        "billions" => 3,
        "millions" => 2,
        "thousands" => 1,
        _ => 0,
    };
    let magnitude = value.abs();
    let (unit, suffix) = if magnitude >= 1e9 && cap >= 3 {
        (1e9, "B")
    } else if magnitude >= 1e6 && cap >= 2 {
        (1e6, "M")
    } else if magnitude >= 1e4 && cap >= 1 {
        (1e3, "k")
    } else {
        (1., "")
    };
    let text = number_in(value / unit, usize::from(!suffix.is_empty()), locale);
    format!("{text}{suffix}")
}

fn number_in(value: f64, fraction_digits: usize, locale: NumberLocale) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-∞"
        } else {
            "∞"
        }
        .into();
    }

    let negative = value.is_sign_negative();
    let (integer, fraction) = decimal_parts(value.abs());
    let mut digits = format!("{integer}{fraction}").into_bytes();
    let keep = integer.len() + fraction_digits;
    let discarded = digits.get(keep..).unwrap_or_default();
    let first = discarded.first().copied().unwrap_or(b'0');
    // Plain values use Math.round (ties towards +infinity), whereas scaled
    // values use Intl's halfExpand rounding. Work on the shortest decimal
    // representation to avoid f64 multiplication and ties-to-even formatting.
    let increment = first > b'5'
        || (first == b'5'
            && (fraction_digits > 0 || !negative || discarded[1..].iter().any(|d| *d != b'0')));
    digits.truncate(keep);
    digits.resize(keep, b'0');
    if increment {
        let mut carry = true;
        for digit in digits.iter_mut().rev() {
            if *digit == b'9' {
                *digit = b'0';
            } else {
                *digit += 1;
                carry = false;
                break;
            }
        }
        if carry {
            digits.insert(0, b'1');
        }
    }
    let decimal_position = digits.len() - fraction_digits;
    let (integer, fraction) = digits.split_at(decimal_position);
    let (group_separator, decimal_separator, minimum_grouped_digits) = locale.separators();
    let mut output = String::new();
    if negative {
        output.push('-');
    }
    for (index, digit) in integer.iter().enumerate() {
        if index > 0
            && integer.len() >= minimum_grouped_digits
            && (integer.len() - index).is_multiple_of(3)
        {
            output.push(group_separator);
        }
        output.push(*digit as char);
    }
    let fraction_end = fraction
        .iter()
        .rposition(|digit| *digit != b'0')
        .map_or(0, |index| index + 1);
    if fraction_end > 0 {
        output.push(decimal_separator);
        output.extend(fraction[..fraction_end].iter().map(|digit| *digit as char));
    }
    output
}

/// Expand a shortest decimal representation without converting through an integer.
/// This also preserves values beyond u64 and very small finite values.
fn decimal_parts(value: f64) -> (String, String) {
    let text = value.to_string();
    let (mantissa, exponent) = text
        .split_once(['e', 'E'])
        .map_or((text.as_str(), 0), |(mantissa, exponent)| {
            (mantissa, exponent.parse::<i32>().unwrap_or(0))
        });
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut digits = format!("{whole}{fraction}");
    let position = whole.len() as i32 + exponent;
    if position <= 0 {
        return (
            "0".into(),
            format!("{}{digits}", "0".repeat((-position) as usize)),
        );
    }
    let position = position as usize;
    if position >= digits.len() {
        digits.push_str(&"0".repeat(position - digits.len()));
        (digits, String::new())
    } else {
        let fraction = digits.split_off(position);
        (digits, fraction)
    }
}

#[cfg(test)]
mod tests {
    use super::{NumberLocale, compact_in, compact_range_in};

    // Frozen outputs from the reference algorithm with explicit Intl locales.
    // U+00A0 is the Polish grouping separator, rather than an ordinary space.
    const FIXTURES: &[(f64, &str, &str, &str)] = &[
        (0., "billions", "0", "0"),
        (999.5, "none", "1,000", "1000"),
        (9999., "billions", "9,999", "9999"),
        (9999.9, "billions", "10,000", "10\u{a0}000"),
        (10000., "billions", "10k", "10k"),
        (12345., "billions", "12.3k", "12,3k"),
        (-12345., "billions", "-12.3k", "-12,3k"),
        (12350., "billions", "12.4k", "12,4k"),
        (-12350., "billions", "-12.4k", "-12,4k"),
        (999999.9, "billions", "1,000k", "1000k"),
        (1e6, "billions", "1M", "1M"),
        (2.5e9, "billions", "2.5B", "2,5B"),
        (2.5e9, "millions", "2,500M", "2500M"),
        (2.5e9, "thousands", "2,500,000k", "2\u{a0}500\u{a0}000k"),
        (12345678., "none", "12,345,678", "12\u{a0}345\u{a0}678"),
        (-1.5, "none", "-1", "-1"),
        (-0.5, "none", "-0", "-0"),
        (-0.5000000000000001, "none", "-1", "-1"),
        (1.25e9, "billions", "1.3B", "1,3B"),
        (-1.25e9, "billions", "-1.3B", "-1,3B"),
        (
            1e25,
            "none",
            "10,000,000,000,000,000,000,000,000",
            "10\u{a0}000\u{a0}000\u{a0}000\u{a0}000\u{a0}000\u{a0}000\u{a0}000\u{a0}000",
        ),
    ];

    #[test]
    fn matches_english_and_polish_intl_fixtures() {
        for &(value, scale, english, polish) in FIXTURES {
            assert_eq!(
                compact_in(value, scale, NumberLocale::from_tag("en-US")),
                english,
                "en-US {value} {scale}"
            );
            assert_eq!(
                compact_in(value, scale, NumberLocale::from_tag("pl-PL")),
                polish,
                "pl-PL {value} {scale}"
            );
        }
    }

    #[test]
    fn ranges_keep_reference_threshold_and_first_bound() {
        for (locale, expected) in [
            (NumberLocale::English, "12.3k–67.9k"),
            (NumberLocale::Polish, "12,3k–67,9k"),
        ] {
            assert_eq!(
                compact_range_in((12345., 67890.), "billions", locale),
                expected
            );
            assert_eq!(compact_range_in((10.49, 10.51), "none", locale), "10");
            assert_eq!(compact_range_in((10., 10.5), "none", locale), "10–11");
        }
    }

    #[test]
    fn retains_nonfinite_and_negative_zero_behavior() {
        let locale = NumberLocale::English;
        assert_eq!(compact_in(-0., "none", locale), "-0");
        assert_eq!(compact_in(f64::NAN, "billions", locale), "NaN");
        assert_eq!(compact_in(f64::INFINITY, "billions", locale), "∞B");
        assert_eq!(compact_in(f64::NEG_INFINITY, "none", locale), "-∞");
        assert_eq!(compact_in(f64::MIN_POSITIVE, "none", locale), "0");
    }
}
