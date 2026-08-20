use std::collections::{HashMap, HashSet};

use super::aggregate::{
    aggregate_tree_mods, apply_attribute_divided_stats, apply_attribute_increased, apply_disables,
    apply_fan_outs, apply_multiplier, apply_per_attribute_stats, apply_tree_conversions,
    TreeAggregateResult,
};
use super::types::{AttrMap, GameConfig, Ranged, StatMap, TreeNodeInfo};

pub struct FinalState {
    pub attrs: AttrMap,
    pub stats: StatMap,
    pub unsupported_lines: Vec<String>,
}

pub struct EngineInputs<'a> {
    pub attr_contributions: &'a HashMap<String, Vec<Ranged>>,
    pub stat_contributions: &'a HashMap<String, Vec<Ranged>>,
    pub allocated_tree_nodes: &'a [u32],
    pub tree_node_info: &'a HashMap<u32, TreeNodeInfo>,
    pub player_conditions: &'a HashMap<String, bool>,
    pub jewelry_ids: &'a HashSet<u32>,
    pub game_config: &'a GameConfig,
}

fn sum_contributions(contributions: &HashMap<String, Vec<Ranged>>) -> HashMap<String, Ranged> {
    let mut out: HashMap<String, Ranged> = HashMap::with_capacity(contributions.len());
    for (k, vs) in contributions {
        let mut sum: Ranged = (0.0, 0.0);
        for r in vs {
            sum.0 += r.0;
            sum.1 += r.1;
        }
        out.insert(k.clone(), sum);
    }
    out
}

fn add_into(map: &mut HashMap<String, Ranged>, k: &str, v: Ranged) {
    let cur = map.get(k).copied().unwrap_or((0.0, 0.0));
    map.insert(k.to_string(), (cur.0 + v.0, cur.1 + v.1));
}

pub fn compute_final_state(inputs: &EngineInputs) -> FinalState {
    let tree: TreeAggregateResult = aggregate_tree_mods(
        inputs.allocated_tree_nodes,
        inputs.tree_node_info,
        inputs.player_conditions,
        inputs.jewelry_ids,
    );

    let attribute_keys: Vec<String> = if inputs.game_config.attribute_keys.is_empty() {
        vec![
            "strength".to_string(),
            "dexterity".to_string(),
            "intelligence".to_string(),
            "energy".to_string(),
            "vitality".to_string(),
            "armor".to_string(),
        ]
    } else {
        inputs.game_config.attribute_keys.clone()
    };

    let mut attrs: AttrMap = sum_contributions(inputs.attr_contributions);
    let mut stats: StatMap = sum_contributions(inputs.stat_contributions);

    for ak in &attribute_keys {
        attrs.entry(ak.clone()).or_insert((0.0, 0.0));
    }

    for (k, v) in &tree.attr_contributions {
        if k == "all_attributes" {
            for ak in &attribute_keys {
                add_into(&mut attrs, ak, *v);
            }
        } else {
            add_into(&mut attrs, k, *v);
        }
    }
    for (k, v) in &tree.stat_contributions {
        add_into(&mut stats, k, *v);
    }

    apply_attribute_increased(&mut attrs, &stats, &attribute_keys);

    apply_per_attribute_stats(
        &mut stats,
        &attrs,
        &inputs.game_config.default_stats_per_attribute,
    );
    apply_attribute_divided_stats(
        &mut stats,
        &attrs,
        &inputs.game_config.attribute_divided_stats,
    );

    apply_fan_outs(&mut stats);

    apply_multiplier(
        &mut stats,
        "life",
        Some("increased_life"),
        Some("increased_life_more"),
        true,
    );
    apply_multiplier(
        &mut stats,
        "mana",
        Some("increased_mana"),
        Some("increased_mana_more"),
        true,
    );
    apply_multiplier(&mut stats, "mana_replenish", None, Some("mana_replenish_more"), false);
    apply_multiplier(&mut stats, "life_replenish", None, Some("life_replenish_more"), false);

    apply_tree_conversions(&mut attrs, &mut stats, &tree.conversions);

    apply_disables(&mut stats, &tree.disables);

    for v in stats.values_mut() {
        if v.0.abs() < 1e-9 && v.1.abs() < 1e-9 {
            *v = (0.0, 0.0);
        }
    }

    FinalState {
        attrs,
        stats,
        unsupported_lines: tree.unsupported_lines,
    }
}
