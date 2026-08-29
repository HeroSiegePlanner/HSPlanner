use super::*;

// ---------- tree contributions ----------

// Conversions and disables collected during the tree pass; applied later
// in the pipeline after stats are summed.
#[derive(Debug, Default)]
pub struct TreeAggregation {
    pub conversions: Vec<(ParsedConversion, String)>,
    pub disables: HashSet<DisableTarget>,
}

// A branch's `g` tags gate only the lines whose own text carries no tag; every
// other key stays a global stat.
const GROUP_GATED_KEYS: &[(&str, &str, &str)] = &[
    ("Spell", "magic_skill_damage", "spell_damage"),
    ("Spell", "magic_skill_damage_more", "spell_damage_more"),
];

pub(crate) fn group_gated_key<'a>(key: &'a str, groups: Option<&Vec<String>>) -> &'a str {
    let Some(groups) = groups else { return key };
    GROUP_GATED_KEYS
        .iter()
        .find(|(tag, from, _)| *from == key && groups.iter().any(|g| g == tag))
        .map_or(key, |(_, _, to)| *to)
}

// Walks allocated non-jewelry tree nodes; pushes mod contributions and
// collects conversions/disables for the caller to apply later.
pub fn apply_tree_contributions(
    allocated_tree_nodes: &HashSet<u32>,
    player_conditions: &HashMap<String, bool>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) -> TreeAggregation {
    apply_node_line_contributions(
        data::tree_nodes(),
        allocated_tree_nodes,
        Some(data::tree_jewelry_ids()),
        "Tree",
        player_conditions,
        attr_sources,
        stat_sources,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_node_line_contributions(
    nodes: &HashMap<String, crate::calc::types::TreeNodeInfo>,
    allocated: &HashSet<u32>,
    skip_ids: Option<&HashSet<u32>>,
    label_prefix: &str,
    player_conditions: &HashMap<String, bool>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) -> TreeAggregation {
    let mut agg = TreeAggregation::default();
    if allocated.is_empty() {
        return agg;
    }

    for &node_id in allocated.iter() {
        if skip_ids.is_some_and(|s| s.contains(&node_id)) {
            continue;
        }
        let key = node_id.to_string();
        let Some(info) = nodes.get(&key) else {
            continue;
        };
        if info.lines.is_empty() {
            continue;
        }
        for line in info.lines.iter() {
            if let Some(parsed) = parse_tree_node_mod(line) {
                if let Some(cond) = parsed.self_condition {
                    let active = player_conditions
                        .get(cond.as_str())
                        .copied()
                        .unwrap_or(false);
                    if !active {
                        continue;
                    }
                }
                // node_id embedded so TS resolves the exact allocated node
                // (multiple nodes share the same display title).
                let label = if parsed.self_condition.is_some() {
                    format!("{}: {} #{} (conditional)", label_prefix, info.title, node_id)
                } else {
                    format!("{}: {} #{}", label_prefix, info.title, node_id)
                };
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    group_gated_key(&parsed.key, info.groups.as_ref()),
                    (parsed.value, parsed.value),
                    label,
                    SourceType::Tree,
                    None,
                );
                continue;
            }
            if let Some(meta) = parse_tree_node_meta(line) {
                match meta {
                    ParsedMeta::Convert(c) => {
                        agg.conversions
                            .push((c, format!("{}: {} #{}", label_prefix, info.title, node_id)));
                    }
                    ParsedMeta::Disable(d) => {
                        agg.disables.insert(d.target);
                    }
                }
            }
        }
    }
    agg
}

// Tree-socket gem/rune/uncut affix contributions. Tagged Tree because the
// bonus is anchored on the tree, not on a worn item.
pub fn apply_tree_jewelry_sockets(
    allocated_tree_nodes: &HashSet<u32>,
    tree_socketed: &HashMap<u32, TreeSocketContent>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) {
    if allocated_tree_nodes.is_empty() {
        return;
    }
    let jewelry_ids = data::tree_jewelry_ids();
    for &node_id in allocated_tree_nodes.iter() {
        if !jewelry_ids.contains(&node_id) {
            continue;
        }
        let Some(content) = tree_socketed.get(&node_id) else {
            continue;
        };
        let socket_label = format!("Tree Socket #{node_id}");
        match content {
            TreeSocketContent::Item { id } => {
                let socketable = data::get_socketable_by_id(id);
                let (name, stats): (String, &HashMap<String, f64>) = match socketable {
                    Some(data::Socketable::Gem(g)) => (g.name.clone(), &g.stats),
                    Some(data::Socketable::Rune(r)) => (r.name.clone(), &r.stats),
                    None => continue,
                };
                for (stat_key, &value) in stats.iter() {
                    if value == 0.0 {
                        continue;
                    }
                    apply_contribution(
                        attr_sources,
                        stat_sources,
                        stat_key,
                        (value, value),
                        format!("{name} ({socket_label})"),
                        SourceType::Tree,
                        None,
                    );
                }
            }
            TreeSocketContent::Uncut { affixes } => {
                for eq in affixes.iter() {
                    let Some(affix) = data::get_affix(&eq.affix_id) else {
                        continue;
                    };
                    let Some(stat_key) = affix.stat_key.as_deref() else {
                        continue;
                    };
                    let signed: f64 = if let Some(cv) = eq.custom_value {
                        cv
                    } else {
                        rolled_affix_value(affix, eq.roll)
                    };
                    if signed == 0.0 {
                        continue;
                    }
                    apply_contribution(
                        attr_sources,
                        stat_sources,
                        stat_key,
                        (signed, signed),
                        format!("{} ({socket_label})", affix.name),
                        SourceType::Tree,
                        None,
                    );
                }
            }
        }
    }
}

