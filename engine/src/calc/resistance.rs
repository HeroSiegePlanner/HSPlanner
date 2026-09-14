//! Canonical ignore-resistance names, including compatibility with older saves
//! and the game's "to Enemy … Resistance" tooltip wording.
use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

use regex::Regex;
use serde::{Deserialize, Deserializer};

const LEGACY_KEYS: &[(&str, &str)] = &[
    ("enemy_all_resist", "ignore_all_res"),
    ("enemy_fire_resist", "ignore_fire_res"),
    ("enemy_cold_resist", "ignore_cold_res"),
    ("enemy_lightning_resist", "ignore_lightning_res"),
    ("enemy_poison_resist", "ignore_poison_res"),
    ("enemy_arcane_resist", "ignore_arcane_res"),
];

pub fn canonical_key(key: &str) -> &str {
    LEGACY_KEYS
        .iter()
        .find(|(old, _)| *old == key)
        .map_or(key, |(_, new)| *new)
}

/// Import compatibility only; target resistance settings are not item bonuses.
pub fn canonical_text(text: &str) -> Cow<'_, str> {
    static LEGACY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r"(?i)\b(?:to\s+)?enemy\s+(all|fire|cold|lightning|poison|arcane)\s+resist(?:ance)?s?\b"
    ).unwrap()
    });
    let replaced = LEGACY.replace_all(text, "Ignore $1 Resistance");
    if matches!(replaced, Cow::Owned(_)) && replaced.trim_start().starts_with('-') {
        // Old minus-enemy wording denotes a positive amount of ignore.
        return Cow::Owned(replaced.replacen('-', "+", 1));
    }
    replaced
}

pub(super) fn deserialize_overrides<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut values = HashMap::<String, f64>::deserialize(deserializer)?;
    for &(old, new) in LEGACY_KEYS {
        if let Some(value) = values.remove(old) {
            // Explicit current spelling wins if both versions were saved.
            values.entry(new.into()).or_insert(value);
        }
    }
    Ok(values)
}

pub(super) fn deserialize_key<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let key = String::deserialize(deserializer)?;
    Ok(canonical_key(&key).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::types::{CustomStat, EquippedItem};

    #[test]
    fn saved_pins_and_custom_stats_keep_values_under_current_names() {
        let item: EquippedItem = serde_json::from_str(
            r#"{
            "baseId":"body_armor_angelic_st_jupe_s_plate_of_command",
            "implicitOverrides":{"enemy_all_resist":23,"enemy_cold_resist":17,
                "ignore_cold_res":19,"life":42}
        }"#,
        )
        .unwrap();
        assert_eq!(
            item.implicit_overrides,
            HashMap::from([
                ("ignore_all_res".into(), 23.),
                ("ignore_cold_res".into(), 19.),
                ("life".into(), 42.)
            ])
        );
        let saved = serde_json::to_string(&item).unwrap();
        let loaded: EquippedItem = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded.implicit_overrides, item.implicit_overrides);
        let custom: CustomStat =
            serde_json::from_str(r#"{"statKey":"enemy_all_resist","value":"12.5"}"#).unwrap();
        assert_eq!(custom.stat_key, "ignore_all_res");
        assert_eq!(custom.value, "12.5");
    }

    #[test]
    fn imports_old_phrasing_without_changing_current_negative_bonuses() {
        assert_eq!(
            canonical_text("-[15-25]% to Enemy Cold Resistance"),
            "+[15-25]% Ignore Cold Resistance"
        );
        assert_eq!(canonical_text("Enemy All Resist"), "Ignore All Resistance");
        assert_eq!(
            canonical_text("-10% Ignore Cold Resistance"),
            "-10% Ignore Cold Resistance"
        );
        assert_eq!(canonical_text("Enemy Resistances"), "Enemy Resistances");
    }
}
