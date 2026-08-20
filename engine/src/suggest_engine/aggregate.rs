use std::collections::{HashMap, HashSet};

use super::types::{
    ranged_add, AttrMap, ConvertKind, ParsedConversion, ParsedMeta, Ranged, StatMap,
    TreeNodeInfo, ranged_is_zero, r_min, r_max,
};
use crate::calc::tree::parse::{classify_tree_node_line, ParsedMod, TreeLineClass};

const STAT_FAN_OUTS: &[(&str, &[&str])] = &[
    (
        "all_resistances",
        &[
            "fire_resistance",
            "cold_resistance",
            "lightning_resistance",
            "poison_resistance",
            "arcane_resistance",
        ],
    ),
    (
        "all_attributes",
        &[
            "to_strength",
            "to_dexterity",
            "to_intelligence",
            "to_energy",
            "to_vitality",
            "to_armor",
        ],
    ),
];

pub fn combine_additive_and_more(add: Ranged, more: Ranged) -> Ranged {
    let min = ((1.0 + r_min(add) / 100.0) * (1.0 + r_min(more) / 100.0) - 1.0) * 100.0;
    let max = ((1.0 + r_max(add) / 100.0) * (1.0 + r_max(more) / 100.0) - 1.0) * 100.0;
    
    (
        (min * 1e6).round() / 1e6,
        (max * 1e6).round() / 1e6,
    )
}

pub fn apply_fan_outs(stats: &mut StatMap) {
    for (src, targets) in STAT_FAN_OUTS {
        for variant in &["", "_more"] {
            let src_key = format!("{}{}", src, variant);
            if let Some(&value) = stats.get(&src_key) {
                if ranged_is_zero(value) {
                    continue;
                }
                for tgt in *targets {
                    let tgt_key = format!("{}{}", tgt, variant);
                    let cur = *stats.get(&tgt_key).unwrap_or(&(0.0, 0.0));
                    stats.insert(tgt_key, ranged_add(cur, value));
                }
            }
        }
    }
}

pub fn apply_multiplier(
    stats: &mut StatMap,
    flat_key: &str,
    pct_key: Option<&str>,
    more_pct_key: Option<&str>,
    floor: bool,
) {
    let flat = *stats.get(flat_key).unwrap_or(&(0.0, 0.0));
    if ranged_is_zero(flat) {
        return;
    }
    let pct = pct_key
        .and_then(|k| stats.get(k))
        .copied()
        .unwrap_or((0.0, 0.0));
    let more = more_pct_key
        .and_then(|k| stats.get(k))
        .copied()
        .unwrap_or((0.0, 0.0));
    let mut min_v = flat.0 * (1.0 + pct.0 / 100.0) * (1.0 + more.0 / 100.0);
    let mut max_v = flat.1 * (1.0 + pct.1 / 100.0) * (1.0 + more.1 / 100.0);
    if floor {
        min_v = min_v.floor();
        max_v = max_v.floor();
    }
    stats.insert(flat_key.to_string(), (min_v, max_v));
}

pub fn apply_attribute_increased(attrs: &mut AttrMap, stats: &StatMap, attribute_keys: &[String]) {
    let all_pct = stats
        .get("increased_all_attributes")
        .copied()
        .unwrap_or((0.0, 0.0));

    for key in attribute_keys {
        let flat = *attrs.get(key).unwrap_or(&(0.0, 0.0));
        if ranged_is_zero(flat) && ranged_is_zero(all_pct) {
            continue;
        }
        let key_pct = format!("increased_{}", key);
        let key_more = format!("increased_{}_more", key);
        let pct = *stats.get(&key_pct).unwrap_or(&(0.0, 0.0));
        let more = *stats.get(&key_more).unwrap_or(&(0.0, 0.0));
        let after_all_min = flat.0 + (flat.0 * all_pct.0 / 100.0).floor();
        let after_all_max = flat.1 + (flat.1 * all_pct.1 / 100.0).floor();
        let final_min =
            (after_all_min * (1.0 + pct.0 / 100.0) * (1.0 + more.0 / 100.0)).floor();
        let final_max =
            (after_all_max * (1.0 + pct.1 / 100.0) * (1.0 + more.1 / 100.0)).floor();
        attrs.insert(key.clone(), (final_min, final_max));
    }
}

pub fn apply_per_attribute_stats(
    stats: &mut StatMap,
    attrs: &AttrMap,
    per_attribute: &HashMap<String, HashMap<String, f64>>,
) {
    for (attr_key, contributions) in per_attribute {
        let attr_val = attrs.get(attr_key).copied().unwrap_or((0.0, 0.0));
        if ranged_is_zero(attr_val) {
            continue;
        }
        for (stat_key, per_point) in contributions {
            if per_point.abs() < 1e-9 {
                continue;
            }
            let contribution = (attr_val.0 * per_point, attr_val.1 * per_point);
            let cur = *stats.get(stat_key).unwrap_or(&(0.0, 0.0));
            stats.insert(stat_key.clone(), ranged_add(cur, contribution));
        }
    }
}

/// floor(attr / divisor) added to stat; floor is intentional to match TS.
pub fn apply_attribute_divided_stats(
    stats: &mut StatMap,
    attrs: &AttrMap,
    divided: &HashMap<String, HashMap<String, f64>>,
) {
    for (attr_key, contributions) in divided {
        let attr_val = attrs.get(attr_key).copied().unwrap_or((0.0, 0.0));
        if ranged_is_zero(attr_val) {
            continue;
        }
        for (stat_key, divisor) in contributions {
            if divisor.abs() < 1e-9 {
                continue;
            }
            let contribution = ((attr_val.0 / divisor).floor(), (attr_val.1 / divisor).floor());
            let cur = *stats.get(stat_key).unwrap_or(&(0.0, 0.0));
            stats.insert(stat_key.clone(), ranged_add(cur, contribution));
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TreeAggregateResult {
    pub stat_contributions: StatMap,
    pub attr_contributions: AttrMap,
    pub conversions: Vec<ParsedConversion>,
    pub disables: HashSet<String>,
    pub unsupported_lines: Vec<String>,
}

const ATTRIBUTE_KEYS_BASE: &[&str] = &[
    "strength",
    "dexterity",
    "intelligence",
    "energy",
    "vitality",
    "armor",
];

fn looks_like_attribute(key: &str) -> bool {
    if let Some(stripped) = key.strip_prefix("to_") {
        ATTRIBUTE_KEYS_BASE.contains(&stripped)
    } else { key == "all_attributes" }
}

pub fn aggregate_tree_mods(
    allocated_tree_nodes: &[u32],
    tree_node_info: &HashMap<u32, TreeNodeInfo>,
    player_conditions: &HashMap<String, bool>,
    jewelry_ids: &HashSet<u32>,
) -> TreeAggregateResult {
    let mut out = TreeAggregateResult::default();
    for &node_id in allocated_tree_nodes {
        let info = match tree_node_info.get(&node_id) {
            Some(i) => i,
            None => continue,
        };
        if jewelry_ids.contains(&node_id) {
            continue;
        }
        for line in &info.lines {
            match classify_tree_node_line(line) {
                TreeLineClass::Stat(parsed) => {
                    if condition_active(&parsed, player_conditions) {
                        push_mod(&mut out, parsed);
                    }
                }
                TreeLineClass::Meta(ParsedMeta::Convert(c)) => out.conversions.push(c),
                TreeLineClass::Meta(ParsedMeta::Disable(d)) => match d.target {
                    super::types::DisableTarget::LifeReplenish => {
                        out.disables.insert("life_replenish".to_string());
                    }
                },
                TreeLineClass::RecognizedNoStat => {}
                TreeLineClass::Unknown => {
                    out.unsupported_lines.push(format!("#{}: {}", node_id, line))
                }
            }
        }
    }
    out
}

fn condition_active(parsed: &ParsedMod, player_conditions: &HashMap<String, bool>) -> bool {
    match parsed.self_condition {
        Some(cond) => player_conditions
            .get(cond.as_str())
            .copied()
            .unwrap_or(false),
        None => true,
    }
}

fn push_mod(out: &mut TreeAggregateResult, parsed: ParsedMod) {
    let value = (parsed.value, parsed.value);
    if !looks_like_attribute(&parsed.key) {
        let cur = *out.stat_contributions.get(&parsed.key).unwrap_or(&(0.0, 0.0));
        out.stat_contributions
            .insert(parsed.key, ranged_add(cur, value));
        return;
    }
    let target_key = if parsed.key == "all_attributes" {
        parsed.key
    } else {
        parsed.key.trim_start_matches("to_").to_string()
    };
    let cur = *out.attr_contributions.get(&target_key).unwrap_or(&(0.0, 0.0));
    out.attr_contributions
        .insert(target_key, ranged_add(cur, value));
}

pub fn apply_tree_conversions(
    attrs: &mut AttrMap,
    stats: &mut StatMap,
    conversions: &[ParsedConversion],
) {
    for conv in conversions {
        let source_val = match conv.from_kind {
            ConvertKind::Attribute => attrs.get(&conv.from_key).copied().unwrap_or((0.0, 0.0)),
            ConvertKind::Stat => {
                let base = stats.get(&conv.from_key).copied().unwrap_or((0.0, 0.0));
                let more_key = format!("{}_more", conv.from_key);
                let more = stats.get(&more_key).copied().unwrap_or((0.0, 0.0));
                combine_additive_and_more(base, more)
            }
        };
        let contribution = (
            source_val.0 * conv.pct / 100.0,
            source_val.1 * conv.pct / 100.0,
        );
        match conv.to_kind {
            ConvertKind::Attribute => {
                let cur = attrs.get(&conv.to_key).copied().unwrap_or((0.0, 0.0));
                attrs.insert(conv.to_key.clone(), ranged_add(cur, contribution));
            }
            ConvertKind::Stat => {
                let cur = stats.get(&conv.to_key).copied().unwrap_or((0.0, 0.0));
                stats.insert(conv.to_key.clone(), ranged_add(cur, contribution));
            }
        }
    }
}

pub fn apply_disables(stats: &mut StatMap, disables: &HashSet<String>) {
    if disables.contains("life_replenish") {
        stats.insert("life_replenish".to_string(), (0.0, 0.0));
        stats.insert("life_replenish_pct".to_string(), (0.0, 0.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aggregate(lines: &[&str], conditions: &[(&str, bool)]) -> TreeAggregateResult {
        let nodes: HashMap<u32, TreeNodeInfo> = HashMap::from([(
            1u32,
            TreeNodeInfo {
                title: "Test Node".to_string(),
                kind: "normal".to_string(),
                lines: lines.iter().map(|s| s.to_string()).collect(),
            },
        )]);
        let conds: HashMap<String, bool> = conditions
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect();
        aggregate_tree_mods(&[1], &nodes, &conds, &HashSet::new())
    }

    // Weapon-conditional lines must keep their own stat key. The damage engine
    // ignores those keys on purpose, so folding them into the generic key would
    // credit the bonus no matter which weapon is equipped.
    #[test]
    fn weapon_conditional_line_keeps_its_own_key() {
        let out = aggregate(
            &["+50% Increased Spell Projectile Damage when wielding a Staff or a Cane"],
            &[],
        );
        assert_eq!(
            out.stat_contributions
                .get("two_handed_spell_projectile_damage"),
            Some(&(50.0, 50.0))
        );
        assert!(!out.stat_contributions.contains_key("spell_projectile_damage"));
    }

    #[test]
    fn routes_stats_attributes_conversions_and_disables() {
        let out = aggregate(
            &[
                "+15 to Maximum Life",
                "+5 to Strength",
                "+15% of Dexterity Added as Ranged Projectile Damage",
                "You cannot regenerate life from life replenish anymore",
                "+1 Socketable Slot",
                "Completely Made Up Line",
            ],
            &[],
        );
        assert_eq!(out.stat_contributions.get("life"), Some(&(15.0, 15.0)));
        assert_eq!(out.attr_contributions.get("strength"), Some(&(5.0, 5.0)));
        assert_eq!(out.conversions.len(), 1);
        assert_eq!(out.conversions[0].to_key, "ranged_projectile_damage");
        assert!(out.disables.contains("life_replenish"));
        // "+1 Socketable Slot" is recognized but carries no stat, so only the
        // bogus line may be reported back to the UI.
        assert_eq!(
            out.unsupported_lines,
            vec!["#1: Completely Made Up Line".to_string()]
        );
    }

    #[test]
    fn self_conditional_line_counts_only_while_its_condition_holds() {
        const LINE: &str =
            "+15% Increased Total Attack Speed when Critical Strike Chance is below 40%";
        assert!(aggregate(&[LINE], &[]).stat_contributions.is_empty());
        let on = aggregate(&[LINE], &[("crit_chance_below_40", true)]);
        assert_eq!(
            on.stat_contributions.get("increased_attack_speed_more"),
            Some(&(15.0, 15.0))
        );
    }

    // Anti-drift guard: the optimizer and the damage engine read the same tree
    // lines, so anything the engine parses must reach the optimizer's stat map
    // instead of its unsupported list.
    #[test]
    fn optimizer_never_drops_a_line_the_engine_parses() {
        let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
        let conditions = [("crit_chance_below_40", true), ("life_below_40", true)];
        let mut dropped: Vec<String> = Vec::new();
        for info in crate::calc::data::tree_nodes().values() {
            for line in &info.lines {
                let parsed_by_engine = matches!(
                    classify_tree_node_line(line),
                    TreeLineClass::Stat(_) | TreeLineClass::Meta(_)
                );
                if !parsed_by_engine {
                    continue;
                }
                if !aggregate(&[line.as_str()], &conditions)
                    .unsupported_lines
                    .is_empty()
                {
                    dropped.push(line.clone());
                }
            }
        }
        assert!(dropped.is_empty(), "optimizer dropped: {dropped:?}");
    }
}
