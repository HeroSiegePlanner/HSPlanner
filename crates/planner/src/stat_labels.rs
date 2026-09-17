//! Presentation labels for canonical game-stat and attribute identifiers.
//! Parsers, persistence and calculations continue to use the original game data.
use hsplanner_engine::calc::performance_diff::PerformanceDiff;
use hsplanner_ui::i18n::{tr, tr_or};

pub(crate) fn stat_name(key: &str, fallback: &str) -> String {
    let attribute = format!("planner.attribute.{key}");
    let stat = format!("planner.stat.{key}");
    tr_or(&stat, tr_or(&attribute, fallback)).to_owned()
}

pub(crate) fn attribute_name(key: &str, fallback: &str) -> String {
    tr_or(&format!("planner.attribute.{key}"), fallback).to_owned()
}

pub(crate) fn performance_name(change: &PerformanceDiff) -> String {
    match change.key() {
        "hit_dps" => tr("tree.changes.hit_dps").to_owned(),
        "combined_dps" => tr("tree.changes.combined_dps").to_owned(),
        "avg_hit" => tr("tree.changes.average_hit").to_owned(),
        key => {
            if let Some(attribute) = key.strip_prefix("attribute:") {
                attribute_name(attribute, change.label())
            } else if let Some(stat) = key.strip_prefix("stat:") {
                stat_name(stat, change.label())
            } else {
                change.label().to_owned()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsplanner_engine::calc::data;
    use hsplanner_ui::i18n::{Language, text_for};

    #[test]
    fn known_definitions_have_translations_without_changing_english_names() {
        let config = data::game_config();
        for (kind, key, name) in config
            .stats
            .iter()
            .map(|stat| ("stat", &stat.key, &stat.name))
            .chain(
                config
                    .attributes
                    .iter()
                    .map(|attribute| ("attribute", &attribute.key, &attribute.name)),
            )
        {
            let catalog_key = format!("planner.{kind}.{key}");
            assert_eq!(text_for(Language::English, &catalog_key), name);
            for language in [Language::Korean, Language::Russian, Language::Chinese] {
                let label = text_for(language, &catalog_key);
                assert!(!label.is_empty(), "{language:?}: {catalog_key}");
                assert_ne!(
                    label, "⟦missing translation⟧",
                    "{language:?}: {catalog_key}"
                );
                assert_ne!(label, name, "{language:?}: {catalog_key}");
            }
        }
    }

    #[test]
    fn unknown_future_definition_keeps_its_source_name() {
        assert_eq!(stat_name("future_stat", "Future stat"), "Future stat");
        assert_eq!(
            attribute_name("future_attribute", "Future attribute"),
            "Future attribute"
        );
    }
}
