use std::collections::HashMap;

pub type ConditionMap = HashMap<String, bool>;

// Stat-key suffix -> enemy-condition key stored by the UI. Most specific first:
// `shadow_burning` must win over `burning`, `deep_frozen` over `frozen`.
const SUFFIX_TO_CONDITION: &[(&str, &str)] = &[
    ("shadow_burning", "shadow_burn"),
    ("serrated_chains", "serrated_chains"),
    ("lightning_break", "lightning_break"),
    ("arcane_break", "arcane_break"),
    ("poison_break", "poison_break"),
    ("fire_break", "fire_break"),
    ("cold_break", "cold_break"),
    ("deep_frozen", "deep_frozen"),
    ("frost_bitten", "frozenbite"),
    ("low_life", "low_life"),
    ("bleeding", "bleeding"),
    ("poisoned", "poisoned"),
    ("stunned", "stunned"),
    ("shocked", "shocked"),
    ("burning", "burning"),
    ("stasis", "shocked"),
    ("frozen", "frozen"),
    ("bosses", "is_boss"),
    ("slow", "slow"),
];

/// Matches TS regex `/_<suffix>$/`: leading `_` required, suffix must end string.
fn matches_suffix(key: &str, suffix: &str) -> bool {
    key.len() > suffix.len()
        && key.ends_with(suffix)
        && key.as_bytes()[key.len() - suffix.len() - 1] == b'_'
}

/// Stats that CREATE a state instead of requiring it; gating them on the state
/// already being present would make them unreachable.
const STATE_APPLIER_PREFIX: &str = "chance_inflict_";

/// Enemy-condition key gating `stat_key`, or `None` when the stat is unconditional.
pub fn condition_key_for(stat_key: &str) -> Option<&'static str> {
    if stat_key.starts_with(STATE_APPLIER_PREFIX) {
        return None;
    }
    SUFFIX_TO_CONDITION
        .iter()
        .find(|(suffix, _)| matches_suffix(stat_key, suffix))
        .map(|(_, cond)| *cond)
}

pub fn is_conditional_key(stat_key: &str) -> bool {
    condition_key_for(stat_key).is_some()
}

pub fn is_condition_active(stat_key: &str, conditions: Option<&ConditionMap>) -> bool {
    let (Some(cond), Some(map)) = (condition_key_for(stat_key), conditions) else {
        return false;
    };
    map.get(cond).copied().unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_suffix_to_ui_condition_key() {
        assert_eq!(condition_key_for("extra_damage_burning"), Some("burning"));
        assert_eq!(condition_key_for("extra_damage_stasis"), Some("shocked"));
        assert_eq!(condition_key_for("extra_damage_low_life"), Some("low_life"));
    }

    #[test]
    fn specific_suffixes_win_over_shorter_ones() {
        assert_eq!(
            condition_key_for("extra_damage_shadow_burning"),
            Some("shadow_burn"),
        );
        assert_eq!(
            condition_key_for("extra_dmg_to_deep_frozen"),
            Some("deep_frozen"),
        );
        assert_eq!(
            condition_key_for("extra_damage_frost_bitten"),
            Some("frozenbite"),
        );
        assert_eq!(condition_key_for("extra_damage_frozen"), Some("frozen"));
    }

    #[test]
    fn subtree_conditionals_reach_their_ui_toggles() {
        assert_eq!(condition_key_for("extra_damage_bosses"), Some("is_boss"));
        assert_eq!(
            condition_key_for("extra_damage_serrated_chains"),
            Some("serrated_chains"),
        );
    }

    #[test]
    fn bare_element_break_keys_stay_unconditional_at_aggregation() {
        // The stat key IS the suffix, so `/_<suffix>$/` cannot match: the
        // subtree value survives aggregation and damage.rs does the gating.
        for key in ["fire_break", "cold_break", "arcane_break", "poison_break"] {
            assert_eq!(condition_key_for(key), None);
        }
        assert_eq!(condition_key_for("extra_damage_fire_break"), Some("fire_break"));
    }

    #[test]
    fn recognises_conditional_keys() {
        assert!(is_conditional_key("extra_damage_burning"));
        assert!(is_conditional_key("damage_lightning_break"));
        assert!(is_conditional_key("foo_slow"));
    }

    #[test]
    fn unconditional_keys_have_no_condition() {
        assert_eq!(condition_key_for("life"), None);
        assert_eq!(condition_key_for("extra_damage_ailments"), None);
        assert_eq!(condition_key_for("burning"), None); // needs a leading "_"
        assert_eq!(condition_key_for("burning_other"), None);
        assert_eq!(condition_key_for("low_life_extra"), None); // suffix in middle
        assert_eq!(condition_key_for(""), None);
    }

    #[test]
    fn inactive_without_condition_map() {
        assert!(!is_condition_active("extra_damage_burning", None));
    }

    // A "chance to inflict X" stat must not be gated on the enemy already having X —
    // the stat is what creates the state, so gating it would make it unreachable.
    #[test]
    fn verify_chance_to_inflict_keys_are_not_enemy_gated() {
        for key in [
            "chance_inflict_burning",
            "chance_inflict_bleeding",
            "chance_inflict_poisoned",
            "chance_inflict_stasis",
        ] {
            assert_eq!(
                condition_key_for(key),
                None,
                "{key} must stay unconditional: it creates the state instead of requiring it"
            );
        }
    }
}
