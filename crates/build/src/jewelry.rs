//! Incarnation jewelry edits. The build document owns socket contents, including dormant slots.
use crate::{BuildSnapshot, gear::jewel_affix_allowed};
use hsplanner_engine::calc::{
    data,
    types::{EquippedAffix, TreeSocketContent},
};
use std::collections::HashSet;

pub const MAX_AFFIXES: usize = 4;

pub fn can_edit(snapshot: &BuildSnapshot, node_id: u32) -> bool {
    data::tree_jewelry_ids().contains(&node_id) && snapshot.allocated_tree_nodes.contains(&node_id)
}

pub fn same_content(a: Option<&TreeSocketContent>, b: Option<&TreeSocketContent>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(TreeSocketContent::Item { id: a }), Some(TreeSocketContent::Item { id: b })) => {
            a == b
        }
        (
            Some(TreeSocketContent::Uncut { affixes: a }),
            Some(TreeSocketContent::Uncut { affixes: b }),
        ) => {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a.affix_id == b.affix_id
                        && a.tier == b.tier
                        && a.roll == b.roll
                        && a.custom_value == b.custom_value
                })
        }
        _ => false,
    }
}

pub fn add_affix(content: &mut Option<TreeSocketContent>, id: &str) -> Result<(), String> {
    let def = data::get_affix(id)
        .filter(|a| jewel_affix_allowed(a))
        .ok_or("Choose a jewel affix.")?;
    let mut affixes = match content.as_ref() {
        Some(TreeSocketContent::Uncut { affixes }) => affixes.clone(),
        _ => vec![],
    };
    if affixes.len() >= MAX_AFFIXES {
        return Err("An Uncut Jewel can have at most four affixes.".into());
    }
    if affixes
        .iter()
        .filter_map(|a| data::get_affix(&a.affix_id))
        .any(|a| a.group_id == def.group_id)
    {
        return Err("This jewel already has that affix family.".into());
    }
    affixes.push(EquippedAffix {
        affix_id: def.id.clone(),
        tier: def.tier,
        roll: 1.,
        custom_value: None,
    });
    *content = Some(TreeSocketContent::Uncut { affixes });
    Ok(())
}

pub fn commit(
    snapshot: &mut BuildSnapshot,
    node_id: u32,
    content: Option<TreeSocketContent>,
) -> Result<(), String> {
    if !can_edit(snapshot, node_id) {
        return Err("Allocate this Jewelry Socket before editing it.".into());
    }
    match &content {
        Some(TreeSocketContent::Item { id }) if data::get_socketable_by_id(id).is_none() => {
            return Err("Unknown gem, rune or jewel.".into());
        }
        Some(TreeSocketContent::Item { .. }) => {}
        Some(TreeSocketContent::Uncut { affixes }) => {
            if affixes.is_empty() || affixes.len() > MAX_AFFIXES {
                return Err("Choose one to four jewel affixes.".into());
            }
            let mut groups = HashSet::new();
            for eq in affixes {
                let def = data::get_affix(&eq.affix_id)
                    .filter(|a| jewel_affix_allowed(a))
                    .ok_or("Invalid jewel affix.")?;
                if !groups.insert(&def.group_id) {
                    return Err("Jewel affix families must be different.".into());
                }
                if eq.tier != def.tier
                    || !eq.roll.is_finite()
                    || !(0. ..=1.).contains(&eq.roll)
                    || eq.custom_value.is_some_and(|v| !v.is_finite())
                {
                    return Err("Invalid jewel tier or roll.".into());
                }
            }
        }
        None => {}
    }
    if let Some(content) = content {
        snapshot.tree_socketed.insert(node_id, content);
    } else {
        snapshot.tree_socketed.remove(&node_id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        codec,
        notes::Notes,
        session::{Session, WorkspaceState},
    };
    use hsplanner_engine::calc::planner::evaluate;

    fn socket() -> (BuildSnapshot, u32) {
        let id = *data::tree_jewelry_ids().iter().min().unwrap();
        (
            BuildSnapshot {
                allocated_tree_nodes: vec![id],
                ..Default::default()
            },
            id,
        )
    }
    fn gem() -> TreeSocketContent {
        let gem = data::data()
            .gems
            .values()
            .find(|g| g.stats.contains_key("additive_physical_damage"))
            .unwrap();
        TreeSocketContent::Item { id: gem.id.clone() }
    }
    fn jewel() -> TreeSocketContent {
        let def = data::data()
            .affixes
            .values()
            .find(|a| {
                jewel_affix_allowed(a) && a.stat_key.as_deref() == Some("additive_physical_damage")
            })
            .unwrap();
        let mut content = None;
        add_affix(&mut content, &def.id).unwrap();
        content.unwrap()
    }

    #[test]
    fn rejects_unallocated_non_jewelry_and_unknown_items_without_mutating() {
        let (mut snapshot, id) = socket();
        commit(&mut snapshot, id, Some(gem())).unwrap();
        let before = snapshot.tree_socketed.clone();
        assert!(commit(&mut snapshot, 0, Some(gem())).is_err());
        assert!(
            commit(
                &mut snapshot,
                id,
                Some(TreeSocketContent::Item {
                    id: "unknown".into()
                })
            )
            .is_err()
        );
        snapshot.allocated_tree_nodes.clear();
        assert!(commit(&mut snapshot, id, None).is_err());
        assert!(same_content(
            snapshot.tree_socketed.get(&id),
            before.get(&id)
        ));
    }

    #[test]
    fn uncut_allows_four_distinct_socketable_families_and_rejects_gear_affixes() {
        let mut content = None;
        let mut groups = HashSet::new();
        let pool: Vec<_> = data::data()
            .affixes
            .values()
            .filter(|a| jewel_affix_allowed(a) && groups.insert(a.group_id.clone()))
            .take(5)
            .collect();
        for def in &pool[..4] {
            add_affix(&mut content, &def.id).unwrap();
        }
        assert!(add_affix(&mut content, &pool[4].id).is_err());
        let (mut snapshot, id) = socket();
        commit(&mut snapshot, id, content.clone()).unwrap();
        let mut duplicate = None;
        add_affix(&mut duplicate, &pool[0].id).unwrap();
        assert!(add_affix(&mut duplicate, &pool[0].id).is_err());
        let other = data::data()
            .affixes
            .values()
            .find(|a| !jewel_affix_allowed(a))
            .unwrap();
        assert!(add_affix(&mut None, &other.id).is_err());
        if let Some(TreeSocketContent::Uncut { affixes }) = &mut content {
            affixes[1] = affixes[0].clone();
        }
        assert!(commit(&mut snapshot, id, content).is_err());
    }

    #[test]
    fn validates_tiers_and_finite_rolls_and_detects_custom_value_changes() {
        let (mut snapshot, id) = socket();
        let initial = jewel();
        for roll in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
            let mut content = initial.clone();
            if let TreeSocketContent::Uncut { affixes } = &mut content {
                affixes[0].roll = roll;
            }
            assert!(commit(&mut snapshot, id, Some(content)).is_err());
        }
        let mut content = initial.clone();
        if let TreeSocketContent::Uncut { affixes } = &mut content {
            affixes[0].tier += 1;
        }
        assert!(commit(&mut snapshot, id, Some(content)).is_err());
        let mut content = initial.clone();
        if let TreeSocketContent::Uncut { affixes } = &mut content {
            affixes[0].custom_value = Some(2.);
        }
        assert!(!same_content(Some(&content), Some(&initial)));
    }

    #[test]
    fn socket_stats_are_active_only_while_allocated_and_clear_removes_them() {
        for content in [gem(), jewel()] {
            let (mut snapshot, id) = socket();
            let base = evaluate(&snapshot.planner_input())
                .computed
                .stats
                .get("additive_physical_damage")
                .copied()
                .unwrap_or((0., 0.));
            commit(&mut snapshot, id, Some(content)).unwrap();
            let inserted = evaluate(&snapshot.planner_input()).computed;
            assert!(inserted.stats["additive_physical_damage"].0 > base.0);
            assert!(
                inserted.stat_sources["additive_physical_damage"]
                    .iter()
                    .any(|s| s.label.contains(&format!("Tree Socket #{id}")))
            );
            snapshot.allocated_tree_nodes.clear();
            assert_eq!(
                evaluate(&snapshot.planner_input())
                    .computed
                    .stats
                    .get("additive_physical_damage")
                    .copied()
                    .unwrap_or((0., 0.)),
                base
            );
            assert!(snapshot.tree_socketed.contains_key(&id));
            snapshot.allocated_tree_nodes.push(id);
            commit(&mut snapshot, id, None).unwrap();
            assert_eq!(
                evaluate(&snapshot.planner_input())
                    .computed
                    .stats
                    .get("additive_physical_damage")
                    .copied()
                    .unwrap_or((0., 0.)),
                base
            );
        }
    }

    #[test]
    fn sockets_survive_share_save_undo_and_redo() {
        let (snapshot, id) = socket();
        let mut state = WorkspaceState::default();
        state.draft.snapshot = snapshot;
        let mut session = Session::new(state);
        for content in [gem(), jewel()] {
            let before = session.snapshot().tree_socketed.get(&id).cloned();
            session.edit(|draft| commit(&mut draft.snapshot, id, Some(content.clone())).unwrap());
            let code = codec::encode(session.snapshot(), &Notes::default()).unwrap();
            let decoded = codec::decode(&code).unwrap().0;
            assert!(same_content(decoded.tree_socketed.get(&id), Some(&content)));
            let restored: WorkspaceState =
                serde_json::from_str(&serde_json::to_string(session.state()).unwrap()).unwrap();
            assert!(same_content(
                restored.draft.snapshot.tree_socketed.get(&id),
                Some(&content)
            ));
            session.undo();
            assert!(same_content(
                session.snapshot().tree_socketed.get(&id),
                before.as_ref()
            ));
            session.redo();
            assert!(same_content(
                session.snapshot().tree_socketed.get(&id),
                Some(&content)
            ));
        }
    }
}
