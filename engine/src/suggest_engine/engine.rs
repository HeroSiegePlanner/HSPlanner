use std::collections::{HashMap, HashSet};

use super::aggregate::{
    aggregate_tree_mods, RESIST_KEYS, apply_attribute_divided_stats, apply_attribute_increased, apply_disables,
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
    pub difficulty: Option<&'a str>,
    pub stack_counts: &'a HashMap<String, u32>,
}

// Mirrors calc::stats::apply_stack_effects: a stack type pays out per stack, and
// an unconfigured build reads as full uptime so the suggester sees the same DPS.
fn apply_stack_effects(stats: &mut StatMap, stack_counts: &HashMap<String, u32>) {
    for def in crate::calc::data::game_config().stack_types.iter() {
        let max = stats.get(&def.max_stat).copied().unwrap_or((0.0, 0.0)).1;
        if max <= 0.0 {
            continue;
        }
        let count = stack_counts
            .get(&def.key)
            .map(|c| f64::from(*c))
            .unwrap_or(max)
            .clamp(0.0, max);
        if count <= 0.0 {
            continue;
        }
        for (target, per_stack) in def.per_stack.iter() {
            let value = per_stack * count;
            add_into(stats, target, (value, value));
        }
        for (rate_key, target) in def.per_stack_stats.iter() {
            let rate = stats.get(rate_key).copied().unwrap_or((0.0, 0.0));
            add_into(stats, target, (rate.0 * count, rate.1 * count));
        }
    }
}

// Mirrors calc::stats::apply_damage_per_resist. Runs after fan-out so the
// elemental buckets already carry the all_resistances spread.
fn apply_damage_per_resist(stats: &mut StatMap) {
    let rate = stats.get("damage_per_resist_point").copied().unwrap_or((0.0, 0.0));
    if rate.0 == 0.0 && rate.1 == 0.0 {
        return;
    }
    let mut resist = (0.0, 0.0);
    for elem in RESIST_KEYS {
        let r = stats.get(*elem).copied().unwrap_or((0.0, 0.0));
        resist.0 += r.0;
        resist.1 += r.1;
    }
    let bonus = (resist.0 * rate.0, resist.1 * rate.1);
    if bonus.0 == 0.0 && bonus.1 == 0.0 {
        return;
    }
    add_into(stats, "enhanced_damage", bonus);
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

    apply_stack_effects(&mut stats, inputs.stack_counts);

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

    let resist_penalty = crate::calc::data::difficulty_resist_penalty(inputs.difficulty);
    if resist_penalty != 0.0 {
        add_into(&mut stats, "all_resistances", (resist_penalty, resist_penalty));
    }

    apply_fan_outs(&mut stats);

    apply_damage_per_resist(&mut stats);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn state(difficulty: Option<&str>) -> FinalState {
        state_with(difficulty, &HashMap::new(), &HashMap::new())
    }

    fn state_with(
        difficulty: Option<&str>,
        stats: &HashMap<String, Vec<Ranged>>,
        stack_counts: &HashMap<String, u32>,
    ) -> FinalState {
        let attrs = HashMap::new();
        let nodes = HashMap::new();
        let conds = HashMap::new();
        let jewelry = HashSet::new();
        let cfg = GameConfig::default();
        compute_final_state(&EngineInputs {
            attr_contributions: &attrs,
            stat_contributions: stats,
            allocated_tree_nodes: &[],
            tree_node_info: &nodes,
            player_conditions: &conds,
            jewelry_ids: &jewelry,
            game_config: &cfg,
            difficulty,
            stack_counts,
        })
    }

    #[test]
    fn rage_stacks_pay_out_in_the_suggester() {
        let seeded: HashMap<String, Vec<Ranged>> = HashMap::from([
            ("max_rage_stacks".to_string(), vec![(4.0, 4.0)]),
            ("damage_per_rage_stack".to_string(), vec![(1.0, 1.0)]),
        ]);
        let full = state_with(None, &seeded, &HashMap::new());
        assert_eq!(
            full.stats.get("enhanced_damage").copied(),
            Some((4.0, 4.0)),
            "4 stacks x 1% must reach enhanced_damage"
        );
        assert_eq!(
            full.stats.get("increased_attack_speed").copied(),
            Some((20.0, 20.0))
        );

        let capped = state_with(None, &seeded, &HashMap::from([("rage".to_string(), 1u32)]));
        assert_eq!(capped.stats.get("enhanced_damage").copied(), Some((1.0, 1.0)));
    }

    #[test]
    fn difficulty_penalty_reaches_the_suggester() {
        let base = state(None);
        let hell = state(Some("hell"));
        let before = base.stats.get("fire_resistance").copied().unwrap_or((0.0, 0.0)).1;
        let after = hell.stats.get("fire_resistance").copied().unwrap_or((0.0, 0.0)).1;
        assert!((before - after - 45.0).abs() < 1e-6, "before {before}, after {after}");
    }

    // 75 res per element x 5 x 0.4 = 150% ED; Hell drops each to 30 -> 60%.
    #[test]
    fn difficulty_penalty_cuts_damage_from_resistances() {
        let seeded: HashMap<String, Vec<Ranged>> = HashMap::from([
            ("all_resistances".to_string(), vec![(75.0, 75.0)]),
            ("damage_per_resist_point".to_string(), vec![(0.4, 0.4)]),
        ]);
        let ed = |d| {
            state_with(d, &seeded, &HashMap::new())
                .stats
                .get("enhanced_damage")
                .copied()
                .unwrap_or((0.0, 0.0))
                .1
        };
        assert!((ed(None) - 150.0).abs() < 1e-6, "got {}", ed(None));
        assert!((ed(Some("hell")) - 60.0).abs() < 1e-6, "got {}", ed(Some("hell")));
    }
}
