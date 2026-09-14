//! Stash rows grouped by slot; pure so the panel layout can be tested without a window.
use hsplanner_build::{
    gear::{is_relic, slot_group},
    library::StashEntry,
};
use hsplanner_engine::calc::{data, types::ItemBase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StashRow {
    Header { group: String, count: usize },
    Entry { index: usize, id: String },
}

fn group_rank(group: &str) -> usize {
    let mut order: Vec<&str> = vec![];
    for slot in data::game_config().slots.iter().flatten() {
        let candidate = slot_group(&slot.key);
        if !order.contains(&candidate) {
            order.push(candidate);
        }
    }
    order
        .iter()
        .position(|known| *known == group)
        .unwrap_or(usize::MAX)
}

fn entry_group(entry: &StashEntry) -> Option<(&'static ItemBase, String)> {
    let base = data::get_item(&entry.item.base_id).filter(|base| !is_relic(base))?;
    Some((base, slot_group(&base.slot).to_owned()))
}

/// Slot groups present in `entries`, in game-config slot order.
pub(crate) fn stash_groups(entries: &[StashEntry]) -> Vec<String> {
    let mut groups: Vec<String> = entries
        .iter()
        .filter_map(entry_group)
        .map(|(_, group)| group)
        .collect();
    groups.sort_by_key(|group| (group_rank(group), group.clone()));
    groups.dedup();
    groups
}

/// Display name for a slot group: the first matching slot's name without its number.
pub(crate) fn group_label(group: &str) -> String {
    data::game_config()
        .slots
        .iter()
        .flatten()
        .find(|slot| slot_group(&slot.key) == group)
        .map(|slot| {
            slot.name
                .trim_end_matches(|c: char| c.is_ascii_digit() || c == ' ')
                .to_owned()
        })
        .unwrap_or_else(|| group.to_owned())
}

/// Rows for the stash list: entries matching `query` (name or base type), grouped by slot
/// in config order with a header per group. `group` narrows to one group and drops headers.
pub(crate) fn group_stash_rows(
    entries: &[StashEntry],
    query: &str,
    group: Option<&str>,
) -> Vec<StashRow> {
    let query = query.trim().to_lowercase();
    let mut matched: Vec<(usize, &StashEntry, String)> = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let (base, entry_group) = entry_group(entry)?;
            let wanted = group.is_none_or(|group| group == entry_group)
                && format!("{} {}", base.name, base.base_type)
                    .to_lowercase()
                    .contains(&query);
            wanted.then_some((index, entry, entry_group))
        })
        .collect();
    matched.sort_by_key(|(_, _, group)| (group_rank(group), group.clone()));
    let mut rows = Vec::with_capacity(matched.len());
    let mut current: Option<&str> = None;
    for (index, entry, entry_group) in &matched {
        if group.is_none() && current != Some(entry_group.as_str()) {
            let count = matched.iter().filter(|(_, _, g)| g == entry_group).count();
            rows.push(StashRow::Header {
                group: entry_group.clone(),
                count,
            });
            current = Some(entry_group);
        }
        rows.push(StashRow::Entry {
            index: *index,
            id: entry.id.clone(),
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsplanner_engine::calc::types::EquippedItem;

    fn entry(id: &str, base_id: &str) -> StashEntry {
        StashEntry {
            id: id.into(),
            saved_at: 0,
            item: EquippedItem {
                base_id: base_id.into(),
                ..Default::default()
            },
        }
    }

    fn sample() -> Vec<StashEntry> {
        vec![
            entry("r1", "ring_angelic_fury_of_tarethiel"),
            entry("w1", "sword_angelic_st_mika_s_zweih_nder"),
            entry("r2", "ring_angelic_fury_of_tarethiel"),
            entry("h1", "helmet_angelic_lucifers_crown"),
        ]
    }

    #[::core::prelude::v1::test]
    fn rows_group_entries_by_slot_in_config_order_with_counts() {
        let rows = group_stash_rows(&sample(), "", None);
        assert_eq!(
            rows,
            vec![
                StashRow::Header {
                    group: "helmet".into(),
                    count: 1
                },
                StashRow::Entry {
                    index: 3,
                    id: "h1".into()
                },
                StashRow::Header {
                    group: "weapon".into(),
                    count: 1
                },
                StashRow::Entry {
                    index: 1,
                    id: "w1".into()
                },
                StashRow::Header {
                    group: "ring".into(),
                    count: 2
                },
                StashRow::Entry {
                    index: 0,
                    id: "r1".into()
                },
                StashRow::Entry {
                    index: 2,
                    id: "r2".into()
                },
            ]
        );
    }

    #[::core::prelude::v1::test]
    fn query_filters_entries_and_drops_empty_groups() {
        let rows = group_stash_rows(&sample(), "lucifer", None);
        assert_eq!(
            rows,
            vec![
                StashRow::Header {
                    group: "helmet".into(),
                    count: 1
                },
                StashRow::Entry {
                    index: 3,
                    id: "h1".into()
                },
            ]
        );
        assert!(group_stash_rows(&sample(), "no such item", None).is_empty());
    }

    #[::core::prelude::v1::test]
    fn group_filter_keeps_one_group_without_header() {
        let rows = group_stash_rows(&sample(), "", Some("ring"));
        assert_eq!(
            rows,
            vec![
                StashRow::Entry {
                    index: 0,
                    id: "r1".into()
                },
                StashRow::Entry {
                    index: 2,
                    id: "r2".into()
                },
            ]
        );
    }

    #[::core::prelude::v1::test]
    fn groups_follow_config_order_and_labels_drop_slot_numbers() {
        assert_eq!(stash_groups(&sample()), vec!["helmet", "weapon", "ring"]);
        assert_eq!(group_label("ring"), "Ring");
        assert_eq!(group_label("charm"), "Charm");
        assert_eq!(group_label("weapon"), "Weapon");
    }

    #[::core::prelude::v1::test]
    fn unknown_bases_are_skipped() {
        let rows = group_stash_rows(&[entry("x", "missing_base")], "", None);
        assert!(rows.is_empty());
        assert!(stash_groups(&[entry("x", "missing_base")]).is_empty());
    }

    #[::core::prelude::v1::test]
    fn relic_entries_are_hidden() {
        let entries = [
            entry("x", "relic_relic_1000_kg"),
            entry("h1", "helmet_angelic_lucifers_crown"),
        ];
        let rows = group_stash_rows(&entries, "", None);
        assert!(
            !rows
                .iter()
                .any(|row| matches!(row, StashRow::Entry { id, .. } if id == "x"))
        );
        assert_eq!(stash_groups(&entries), vec!["helmet"]);
        assert!(group_stash_rows(&entries, "", Some("relic")).is_empty());
    }
}
