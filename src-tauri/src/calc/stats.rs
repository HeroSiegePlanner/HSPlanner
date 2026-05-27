// Stat aggregation: source-tracking parity with TS computeBuildStats in
// src/utils/stats.ts.

use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;
use serde::Serialize;

use super::affix::{
    apply_stars_to_ranged_value, rolled_affix_range, rolled_affix_value, rolled_affix_value_with_stars,
};
use super::custom_stat::parse_custom_stat_value;
use super::data::{self, ForgeKind};
use super::rank::{aggregate_item_skill_bonuses, normalize_skill_name};
use super::skills::Ranged;
use super::tree::parse::{
    DisableTarget, ParsedConversion, ParsedMeta, parse_tree_node_meta, parse_tree_node_mod,
};
use super::types::{CustomStat, Inventory, SkillKind, SocketType, StatDef, TreeSocketContent};

pub const RAINBOW_MULTIPLIER: f64 = 1.5;

// ---------- top-level types ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Class,
    Allocated,
    Level,
    Attribute,
    Item,
    Socket,
    Skill,
    Subskill,
    Custom,
    Tree,
}

pub const CUSTOM_SOURCE_LABEL: &str = "Custom Config";

// Stats whose total fans out to per-element variants
// (e.g. `all_resistances` → each elemental resistance bucket).
pub const STAT_FAN_OUTS: &[(&str, &[&str])] = &[
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
        "max_all_resistances",
        &[
            "max_fire_resistance",
            "max_cold_resistance",
            "max_lightning_resistance",
            "max_poison_resistance",
            "max_arcane_resistance",
        ],
    ),
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forge {
    pub item_name: String,
    pub mod_name: String,
    pub kind: ForgeKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceContribution {
    pub label: String,
    pub source_type: SourceType,
    pub value: Ranged,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forge: Option<Forge>,
}

pub type SourceMap = HashMap<String, Vec<SourceContribution>>;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedStats {
    pub attributes: HashMap<String, Ranged>,
    pub stats: HashMap<String, Ranged>,
    pub attribute_sources: SourceMap,
    pub stat_sources: SourceMap,
}

// ---------- inline helpers ----------

#[inline]
pub fn is_zero(v: Ranged) -> bool {
    v.0 == 0.0 && v.1 == 0.0
}

// ---------- stat-def lookup with `_more` suffix fallback ----------

static STAT_DEFS_MAP: Lazy<HashMap<&'static str, &'static StatDef>> = Lazy::new(|| {
    let mut m: HashMap<&'static str, &'static StatDef> = HashMap::new();
    for stat in data::game_config().stats.iter() {
        m.insert(stat.key.as_str(), stat);
    }
    m
});

// `_more` keys fall back to base key (UI formatting overlay stays in TS).
pub fn stat_def(key: &str) -> Option<&'static StatDef> {
    if let Some(def) = STAT_DEFS_MAP.get(key) {
        return Some(*def);
    }
    if let Some(base) = key.strip_suffix("_more") {
        return STAT_DEFS_MAP.get(base).copied();
    }
    None
}

// ---------- contribution helpers ----------

pub fn push_source(map: &mut SourceMap, key: &str, source: SourceContribution) {
    if is_zero(source.value) {
        return;
    }
    map.entry(key.to_string()).or_default().push(source);
}

pub fn sum_contributions(sources: &[SourceContribution]) -> Ranged {
    let mut min = 0.0;
    let mut max = 0.0;
    for s in sources {
        min += s.value.0;
        max += s.value.1;
    }
    (min, max)
}

pub fn sum_ranged_from_map(map: &SourceMap, key: &str) -> Ranged {
    let Some(list) = map.get(key) else {
        return (0.0, 0.0);
    };
    if list.is_empty() {
        return (0.0, 0.0);
    }
    let v = sum_contributions(list);
    (v.0.floor(), v.1.floor())
}

#[allow(clippy::too_many_arguments)]
pub fn apply_contribution(
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
    stat_key: &str,
    value: Ranged,
    label: String,
    source_type: SourceType,
    forge: Option<Forge>,
) {
    if is_zero(value) {
        return;
    }
    let def_opt = stat_def(stat_key);
    if let Some(def) = def_opt {
        if def.item_only.unwrap_or(false) {
            return;
        }
    }
    let contribution = SourceContribution {
        label,
        source_type,
        value,
        forge,
    };
    if let Some(def) = def_opt {
        if let Some(target) = def.modifies_attribute.as_deref() {
            if target == "all" {
                for attr in data::game_config().attributes.iter() {
                    push_source(attr_sources, &attr.key, contribution.clone());
                }
            } else {
                push_source(attr_sources, target, contribution);
            }
            return;
        }
    }
    push_source(stat_sources, stat_key, contribution);
}

// ---------- compute_item_effective_defense ----------

// Returns the Ranged defense after applying enhanced_defense% (caller
// star-scales it), or None when the base has no defense range.
pub fn compute_item_effective_defense(
    defense_min: Option<f64>,
    defense_max: Option<f64>,
    enhanced_defense: Option<Ranged>,
) -> Option<Ranged> {
    let (Some(min_base), Some(max_base)) = (defense_min, defense_max) else {
        return None;
    };
    let (ed_min, ed_max) = enhanced_defense.unwrap_or((0.0, 0.0));
    let min = (min_base * (1.0 + ed_min / 100.0)).floor();
    let max = (max_base * (1.0 + ed_max / 100.0)).floor();
    Some((min, max))
}

// ---------- multiplier helpers ----------

// Collapses additive% and more% into a single equivalent additive%,
// rounded to 6 decimals via JS-style Math.round (`(x + 0.5).floor()`).
pub fn combine_additive_and_more(additive: Ranged, more: Ranged) -> Ranged {
    let round = |n: f64| ((n * 1e6) + 0.5).floor() / 1e6;
    let min = round(((1.0 + additive.0 / 100.0) * (1.0 + more.0 / 100.0) - 1.0) * 100.0);
    let max = round(((1.0 + additive.1 / 100.0) * (1.0 + more.1 / 100.0) - 1.0) * 100.0);
    (min, max)
}

// `floor=false` for replenish stats (preserves fractional regen).
pub fn apply_multiplier(
    stats: &mut HashMap<String, Ranged>,
    flat_key: &str,
    pct_key: Option<&str>,
    more_pct_key: Option<&str>,
    floor: bool,
) {
    let Some(flat) = stats.get(flat_key).copied() else {
        return;
    };
    let pct = pct_key
        .and_then(|k| stats.get(k).copied())
        .unwrap_or((0.0, 0.0));
    let more = more_pct_key
        .and_then(|k| stats.get(k).copied())
        .unwrap_or((0.0, 0.0));
    if is_zero(pct) && is_zero(more) {
        return;
    }
    let raw_min = flat.0 * (1.0 + pct.0 / 100.0) * (1.0 + more.0 / 100.0);
    let raw_max = flat.1 * (1.0 + pct.1 / 100.0) * (1.0 + more.1 / 100.0);
    let (min, max) = if floor {
        (raw_min.floor(), raw_max.floor())
    } else {
        (raw_min, raw_max)
    };
    stats.insert(flat_key.to_string(), (min, max));
}

// ---------- inventory loop ----------

// Pushes contributions from implicits, affixes, forge mods, runewords,
// sockets, and augments. Set bonuses are applied later in the pipeline.
// Returns `weapon_has_attack_speed` so the baseline step can skip the
// default APS when a weapon supplies one.
pub fn apply_inventory(
    inventory: &Inventory,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) -> bool {
    let mut weapon_has_attack_speed = false;

    for (slot_key, item) in inventory.iter() {
        let Some(base) = data::get_item(&item.base_id) else {
            continue;
        };
        let item_name = base.name.clone();

        // Runeword suppresses per-socket gem contributions and implicit scaling.
        let socketed_refs: Vec<Option<&str>> =
            item.socketed.iter().map(|s| s.as_deref()).collect();
        let runeword = data::detect_runeword(base, &socketed_refs);
        let scale_implicit = runeword.is_none();
        let is_gear = data::is_gear_slot(slot_key);
        let effective_stars: Option<u32> = if is_gear { item.stars } else { None };

        let aps_in_implicit = base
            .implicit
            .as_ref()
            .map(|m| m.contains_key("attacks_per_second"))
            .unwrap_or(false);
        if slot_key == "weapon" && (aps_in_implicit || base.attack_speed.is_some()) {
            weapon_has_attack_speed = true;
        }

        let ed_override = item.implicit_overrides.get("enhanced_defense").copied();
        let ed_raw = base
            .implicit
            .as_ref()
            .and_then(|m| m.get("enhanced_defense"))
            .copied();
        let ed_scaled: Option<Ranged> = match (ed_override, ed_raw) {
            (Some(ov), _) => Some((ov, ov)),
            (None, Some(raw)) if scale_implicit => Some(apply_stars_to_ranged_value(
                raw.as_ranged(),
                "enhanced_defense",
                effective_stars,
            )),
            (None, Some(raw)) => Some(raw.as_ranged()),
            (None, None) => None,
        };
        if let Some(eff) =
            compute_item_effective_defense(base.defense_min, base.defense_max, ed_scaled)
        {
            if !is_zero(eff) {
                push_source(
                    stat_sources,
                    "defense",
                    SourceContribution {
                        label: item_name.clone(),
                        source_type: SourceType::Item,
                        value: eff,
                        forge: None,
                    },
                );
            }
        }

        if let Some(implicit) = base.implicit.as_ref() {
            for (stat_key, value) in implicit.iter() {
                let override_val = item.implicit_overrides.get(stat_key).copied();
                let scaled: Ranged = match override_val {
                    Some(ov) => (ov, ov),
                    None if scale_implicit => apply_stars_to_ranged_value(
                        value.as_ranged(),
                        stat_key,
                        effective_stars,
                    ),
                    None => value.as_ranged(),
                };
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    stat_key,
                    scaled,
                    item_name.clone(),
                    SourceType::Item,
                    None,
                );
            }
        }

        // User-added overrides for keys not in base.implicit; ED handled above.
        for (stat_key, &value) in item.implicit_overrides.iter() {
            if let Some(implicit) = base.implicit.as_ref() {
                if implicit.contains_key(stat_key) {
                    continue;
                }
            }
            if stat_key == "enhanced_defense" {
                continue;
            }
            apply_contribution(
                attr_sources,
                stat_sources,
                stat_key,
                (value, value),
                item_name.clone(),
                SourceType::Item,
                None,
            );
        }

        if base.slot == "weapon" && !aps_in_implicit {
            if let Some(aps) = base.attack_speed {
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    "attacks_per_second",
                    aps.as_ranged(),
                    item_name.clone(),
                    SourceType::Item,
                    None,
                );
            }
        }

        for eq in item.affixes.iter() {
            let Some(affix) = data::get_affix(&eq.affix_id) else {
                continue;
            };
            let Some(stat_key) = affix.stat_key.as_deref() else {
                continue;
            };
            let signed: f64 = if let Some(cv) = eq.custom_value {
                cv
            } else {
                rolled_affix_value_with_stars(affix, eq.roll, effective_stars)
            };
            if signed == 0.0 {
                continue;
            }
            let label = format!("{} ({})", affix.name, item_name);
            apply_contribution(
                attr_sources,
                stat_sources,
                stat_key,
                (signed, signed),
                label,
                SourceType::Item,
                None,
            );
        }

        if is_gear {
            if let Some(forge_kind) = data::forge_kind_for(&base.rarity) {
                for eq in item.forged_mods.iter() {
                    let Some(mod_def) = data::get_crystal_mod(&eq.affix_id) else {
                        continue;
                    };
                    let Some(stat_key) = mod_def.stat_key.as_deref() else {
                        continue;
                    };
                    let ranged: Ranged = if let Some(cv) = eq.custom_value {
                        (cv, cv)
                    } else {
                        rolled_affix_range(mod_def)
                    };
                    if is_zero(ranged) {
                        continue;
                    }
                    apply_contribution(
                        attr_sources,
                        stat_sources,
                        stat_key,
                        ranged,
                        mod_def.name.clone(),
                        SourceType::Item,
                        Some(Forge {
                            item_name: item_name.clone(),
                            mod_name: mod_def.name.clone(),
                            kind: forge_kind,
                        }),
                    );
                }
            }
        }

        // Runeword XOR per-socket contributions.
        if let Some(rw) = runeword {
            let label = format!("{} ({})", rw.name, item_name);
            for (stat_key, &value) in rw.stats.iter() {
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    stat_key,
                    (value, value),
                    label.clone(),
                    SourceType::Item,
                    None,
                );
            }
        } else {
            for (i, slot_id_opt) in item.socketed.iter().enumerate() {
                let Some(id) = slot_id_opt.as_deref() else {
                    continue;
                };
                let socketable = data::get_socketable_by_id(id);
                let (source_name, source_stats): (String, &std::collections::HashMap<String, f64>) =
                    match socketable {
                        Some(data::Socketable::Gem(g)) => (g.name.clone(), &g.stats),
                        Some(data::Socketable::Rune(r)) => (r.name.clone(), &r.stats),
                        None => continue,
                    };
                let is_rainbow = item
                    .socket_types
                    .get(i)
                    .copied()
                    .unwrap_or(SocketType::Normal)
                    == SocketType::Rainbow;
                let mult = if is_rainbow { RAINBOW_MULTIPLIER } else { 1.0 };
                let transform = base.socket_transforms.as_ref().and_then(|m| m.get(id));
                let effective_stats: &std::collections::HashMap<String, f64> =
                    transform.unwrap_or(source_stats);
                let socket_label = {
                    let mut s = format!("{} in {} #{}", source_name, item_name, i + 1);
                    if is_rainbow {
                        s.push_str(" (Rainbow)");
                    }
                    if transform.is_some() {
                        s.push_str(" (Transform)");
                    }
                    s
                };
                for (stat_key, &raw_value) in effective_stats.iter() {
                    let v = raw_value * mult;
                    apply_contribution(
                        attr_sources,
                        stat_sources,
                        stat_key,
                        (v, v),
                        socket_label.clone(),
                        SourceType::Socket,
                        None,
                    );
                }
            }
        }

        if let Some(aug_ref) = item.augment.as_ref() {
            if let Some(aug) = data::get_augment(&aug_ref.id) {
                if !aug.levels.is_empty() {
                    let lvl = aug_ref
                        .level
                        .clamp(1, aug.levels.len() as u32);
                    if let Some(tier) = aug.levels.get((lvl - 1) as usize) {
                        let label =
                            format!("Augment: {} Lv {} ({})", aug.name, lvl, item_name);
                        for (stat_key, &value) in tier.stats.iter() {
                            apply_contribution(
                                attr_sources,
                                stat_sources,
                                stat_key,
                                (value, value),
                                label.clone(),
                                SourceType::Item,
                                None,
                            );
                        }
                    }
                }
            }
        }
    }

    weapon_has_attack_speed
}

// ---------- tree contributions ----------

// Conversions and disables collected during the tree pass; applied later
// in the pipeline after stats are summed.
#[derive(Debug, Default)]
pub struct TreeAggregation {
    pub conversions: Vec<(ParsedConversion, String)>,
    pub disables: HashSet<DisableTarget>,
}

// Walks allocated non-jewelry tree nodes; pushes mod contributions and
// collects conversions/disables for the caller to apply later.
pub fn apply_tree_contributions(
    allocated_tree_nodes: &HashSet<u32>,
    player_conditions: &HashMap<String, bool>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) -> TreeAggregation {
    let mut agg = TreeAggregation::default();
    if allocated_tree_nodes.is_empty() {
        return agg;
    }
    let jewelry_ids = data::tree_jewelry_ids();
    let tree = data::tree_nodes();

    for &node_id in allocated_tree_nodes.iter() {
        if jewelry_ids.contains(&node_id) {
            continue;
        }
        let key = node_id.to_string();
        let Some(info) = tree.get(&key) else {
            continue;
        };
        if info.l.is_empty() {
            continue;
        }
        for line in info.l.iter() {
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
                    format!("Tree: {} #{} (conditional)", info.t, node_id)
                } else {
                    format!("Tree: {} #{}", info.t, node_id)
                };
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    &parsed.key,
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
                            .push((c, format!("Tree: {} #{}", info.t, node_id)));
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

// ---------- base attributes + class baseline + per-level ----------

// Seeds attribute sources: default base + class base + allocated points.
// Runs FIRST so later stages can derive from totalled attributes.
pub fn apply_base_attributes(
    class_id: Option<&str>,
    allocated_attrs: &HashMap<String, u32>,
    attr_sources: &mut SourceMap,
) {
    let cfg = data::game_config();
    let cls = class_id.and_then(data::get_class);
    let class_name = cls
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Class".to_string());

    for attr in cfg.attributes.iter() {
        let default_base = cfg
            .default_base_attributes
            .as_ref()
            .and_then(|m| m.get(&attr.key))
            .copied()
            .unwrap_or(0.0);
        if default_base != 0.0 {
            push_source(
                attr_sources,
                &attr.key,
                SourceContribution {
                    label: "Base character".to_string(),
                    source_type: SourceType::Class,
                    value: (default_base, default_base),
                    forge: None,
                },
            );
        }
        let class_base = cls
            .and_then(|c| c.base_attributes.get(&attr.key))
            .copied()
            .unwrap_or(0.0);
        if class_base != 0.0 {
            push_source(
                attr_sources,
                &attr.key,
                SourceContribution {
                    label: format!("{class_name} base"),
                    source_type: SourceType::Class,
                    value: (class_base, class_base),
                    forge: None,
                },
            );
        }
        let added = allocated_attrs.get(&attr.key).copied().unwrap_or(0);
        if added > 0 {
            push_source(
                attr_sources,
                &attr.key,
                SourceContribution {
                    label: "Allocated points".to_string(),
                    source_type: SourceType::Allocated,
                    value: (added as f64, added as f64),
                    forge: None,
                },
            );
        }
    }
}

// ---------- set bonuses ----------

pub fn apply_set_bonuses(
    inventory: &Inventory,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for item in inventory.values() {
        if let Some(base) = data::get_item(&item.base_id) {
            if let Some(set_id) = base.set_id.as_deref() {
                *counts.entry(set_id.to_string()).or_insert(0) += 1;
            }
        }
    }
    for (set_id, count) in counts.iter() {
        let Some(set) = data::get_set(set_id) else {
            continue;
        };
        for bonus in set.bonuses.iter() {
            if *count < bonus.pieces {
                continue;
            }
            let label = format!("{} ({}-set)", set.name, bonus.pieces);
            for (stat_key, &value) in bonus.stats.iter() {
                if value == 0.0 {
                    continue;
                }
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    stat_key,
                    (value, value),
                    label.clone(),
                    SourceType::Item,
                    None,
                );
            }
        }
    }
}

// ---------- class baseline + per-level ----------

pub fn apply_class_baseline(
    class_id: Option<&str>,
    level: u32,
    weapon_has_attack_speed: bool,
    stat_sources: &mut SourceMap,
) {
    let cfg = data::game_config();
    let cls = class_id.and_then(data::get_class);
    let class_name = cls
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Class".to_string());

    if let Some(defaults) = cfg.default_base_stats.as_ref() {
        for (stat_key, &value) in defaults.iter() {
            if value == 0.0 {
                continue;
            }
            if stat_key == "attacks_per_second" && weapon_has_attack_speed {
                continue;
            }
            push_source(
                stat_sources,
                stat_key,
                SourceContribution {
                    label: "Base character".to_string(),
                    source_type: SourceType::Class,
                    value: (value, value),
                    forge: None,
                },
            );
        }
    }

    if let Some(cls) = cls {
        for (stat_key, &value) in cls.base_stats.iter() {
            if value == 0.0 {
                continue;
            }
            push_source(
                stat_sources,
                stat_key,
                SourceContribution {
                    label: format!("{class_name} base"),
                    source_type: SourceType::Class,
                    value: (value, value),
                    forge: None,
                },
            );
        }
        for (stat_key, &per_level) in cls.stats_per_level.iter() {
            let total = per_level * level as f64;
            if total == 0.0 {
                continue;
            }
            push_source(
                stat_sources,
                stat_key,
                SourceContribution {
                    label: format!("Per level × {level}"),
                    source_type: SourceType::Level,
                    value: (total, total),
                    forge: None,
                },
            );
        }
    }
}

// ---------- custom stats ----------

pub fn apply_custom_stats(
    custom_stats: &[CustomStat],
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) {
    for cs in custom_stats.iter() {
        if cs.stat_key.is_empty() {
            continue;
        }
        let Some(parsed) = parse_custom_stat_value(&cs.value) else {
            continue;
        };
        apply_contribution(
            attr_sources,
            stat_sources,
            &cs.stat_key,
            parsed,
            CUSTOM_SOURCE_LABEL.to_string(),
            SourceType::Custom,
            None,
        );
    }
}

// ---------- skill ranks + passive stats ----------

// Effective rank = base + all_skills + element bonus + item bonus.
// Applies `base + per_rank * (rank - 1)` to each passive-stat key.
// Aura skills need active_aura_id match; buff skills need active_buffs flag.
#[allow(clippy::too_many_arguments)]
pub fn apply_skill_ranks(
    class_id: Option<&str>,
    skill_ranks: &HashMap<String, u32>,
    active_aura_id: Option<&str>,
    active_buffs: &HashMap<String, bool>,
    inventory: &Inventory,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) {
    let Some(class_id) = class_id else {
        return;
    };
    let cfg = data::game_config();
    let attr_keys: HashSet<&str> = cfg.attributes.iter().map(|a| a.key.as_str()).collect();
    let class_skills = data::get_skills_by_class(class_id);
    let item_skill_bonuses = aggregate_item_skill_bonuses(inventory, &data::data().items);
    let all_skills_bonus = sum_ranged_from_map(stat_sources, "all_skills");

    for skill in class_skills.iter() {
        let base_rank = skill_ranks.get(&skill.id).copied().unwrap_or(0);
        if base_rank == 0 {
            continue;
        }
        let Some(passive) = skill.passive_stats.as_ref() else {
            continue;
        };

        if skill.kind == SkillKind::Aura && active_aura_id != Some(skill.id.as_str()) {
            continue;
        }
        let is_buff = skill.kind == SkillKind::Buff
            || skill
                .tags
                .as_ref()
                .is_some_and(|tags| tags.iter().any(|t| t == "Buff"));
        if is_buff && !active_buffs.get(&skill.id).copied().unwrap_or(false) {
            continue;
        }

        let elem_bonus = skill
            .damage_type
            .as_deref()
            .map(|dt| sum_ranged_from_map(stat_sources, &format!("{dt}_skills")))
            .unwrap_or((0.0, 0.0));
        let key_norm = normalize_skill_name(&skill.name);
        let item_bonus = item_skill_bonuses
            .get(&key_norm)
            .copied()
            .unwrap_or((0.0, 0.0));

        let eff_min =
            (base_rank as f64 + all_skills_bonus.0 + elem_bonus.0 + item_bonus.0).max(1.0);
        let eff_max =
            (base_rank as f64 + all_skills_bonus.1 + elem_bonus.1 + item_bonus.1).max(1.0);

        let mut combined: HashMap<String, Ranged> = HashMap::new();
        if let Some(base) = passive.base.as_ref() {
            for (k, &v) in base.iter() {
                combined.insert(k.clone(), (v, v));
            }
        }
        if let Some(per_rank) = passive.per_rank.as_ref() {
            for (k, &v) in per_rank.iter() {
                let existing = combined.get(k).copied().unwrap_or((0.0, 0.0));
                let min = existing.0 + v * (eff_min - 1.0);
                let max = existing.1 + v * (eff_max - 1.0);
                combined.insert(k.clone(), (min, max));
            }
        }

        let rank_label = if eff_min == eff_max {
            format!("{eff_min}")
        } else {
            format!("{eff_min}-{eff_max}")
        };
        for (key, value) in combined.iter() {
            if is_zero(*value) {
                continue;
            }
            // JS Math.round semantics (.5 ties round up).
            let rounded = (round3(value.0), round3(value.1));
            let label = format!("{} (rank {})", skill.name, rank_label);
            let contrib = SourceContribution {
                label,
                source_type: SourceType::Skill,
                value: rounded,
                forge: None,
            };
            if attr_keys.contains(key.as_str()) {
                push_source(attr_sources, key, contrib);
            } else {
                push_source(stat_sources, key, contrib);
            }
        }
    }
}

#[inline]
fn round3(x: f64) -> f64 {
    ((x * 1000.0) + 0.5).floor() / 1000.0
}

// ---------- attribute pipelines ----------

pub fn apply_increased_all_attributes(attr_sources: &mut SourceMap, stat_sources: &SourceMap) {
    let pct_sources = match stat_sources.get("increased_all_attributes") {
        Some(list) if !list.is_empty() => list.clone(),
        _ => return,
    };
    let cfg = data::game_config();
    for attr in cfg.attributes.iter() {
        let flat_sum = sum_contributions(attr_sources.get(&attr.key).map(|v| v.as_slice()).unwrap_or(&[]));
        for pct_src in pct_sources.iter() {
            let bonus_min = (flat_sum.0 * pct_src.value.0 / 100.0).floor();
            let bonus_max = (flat_sum.1 * pct_src.value.1 / 100.0).floor();
            if bonus_min == 0.0 && bonus_max == 0.0 {
                continue;
            }
            push_source(
                attr_sources,
                &attr.key,
                SourceContribution {
                    label: pct_src.label.clone(),
                    source_type: pct_src.source_type,
                    value: (bonus_min, bonus_max),
                    forge: None,
                },
            );
        }
    }
}

// Per-attribute `increased_X` + `increased_X_more` compounded as a delta.
// Hardcoded SourceType::Tree matches TS (these typically come from tree).
pub fn apply_increased_per_attribute(attr_sources: &mut SourceMap, stat_sources: &SourceMap) {
    let cfg = data::game_config();
    for attr in cfg.attributes.iter() {
        let add_key = format!("increased_{}", attr.key);
        let more_key = format!("increased_{}_more", attr.key);
        let empty: Vec<SourceContribution> = Vec::new();
        let add_list = stat_sources.get(&add_key).unwrap_or(&empty);
        let more_list = stat_sources.get(&more_key).unwrap_or(&empty);
        if add_list.is_empty() && more_list.is_empty() {
            continue;
        }
        let flat_sum = sum_contributions(attr_sources.get(&attr.key).map(|v| v.as_slice()).unwrap_or(&[]));
        if is_zero(flat_sum) {
            continue;
        }
        let add_sum = sum_contributions(add_list);
        let more_sum = sum_contributions(more_list);
        let final_min = (flat_sum.0 * (1.0 + add_sum.0 / 100.0) * (1.0 + more_sum.0 / 100.0)).floor();
        let final_max = (flat_sum.1 * (1.0 + add_sum.1 / 100.0) * (1.0 + more_sum.1 / 100.0)).floor();
        let bonus_min = final_min - flat_sum.0;
        let bonus_max = final_max - flat_sum.1;
        if bonus_min == 0.0 && bonus_max == 0.0 {
            continue;
        }
        let mut label_parts: Vec<String> = Vec::new();
        if add_sum.0 != 0.0 || add_sum.1 != 0.0 {
            if add_sum.0 == add_sum.1 {
                label_parts.push(format!("+{}%", add_sum.0));
            } else {
                label_parts.push(format!("+{}-{}%", add_sum.0, add_sum.1));
            }
        }
        if more_sum.0 != 0.0 || more_sum.1 != 0.0 {
            if more_sum.0 == more_sum.1 {
                label_parts.push(format!("Total +{}%", more_sum.0));
            } else {
                label_parts.push(format!("Total +{}-{}%", more_sum.0, more_sum.1));
            }
        }
        push_source(
            attr_sources,
            &attr.key,
            SourceContribution {
                label: format!("Increased {} ({})", attr.name, label_parts.join(", ")),
                source_type: SourceType::Tree,
                value: (bonus_min, bonus_max),
                forge: None,
            },
        );
    }
}

// (attribute → stat → per_point) scaling from default + class config.
pub fn apply_stats_per_attribute(
    class_id: Option<&str>,
    attr_sources: &SourceMap,
    stat_sources: &mut SourceMap,
) {
    let cfg = data::game_config();
    let cls = class_id.and_then(data::get_class);

    let mut maps: Vec<&HashMap<String, HashMap<String, f64>>> = Vec::new();
    if let Some(m) = cfg.default_stats_per_attribute.as_ref() {
        maps.push(m);
    }
    if let Some(cls) = cls {
        maps.push(&cls.stats_per_attribute);
    }
    if maps.is_empty() {
        return;
    }

    let mut totals: HashMap<String, Ranged> = HashMap::new();
    for attr in cfg.attributes.iter() {
        let sum = sum_contributions(attr_sources.get(&attr.key).map(|v| v.as_slice()).unwrap_or(&[]));
        totals.insert(attr.key.clone(), sum);
    }

    for map in maps {
        for (attr_key, stats_map) in map.iter() {
            let attr_val = totals.get(attr_key).copied().unwrap_or((0.0, 0.0));
            let attr_name = cfg
                .attributes
                .iter()
                .find(|a| &a.key == attr_key)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| attr_key.clone());
            for (stat_key, &per_point) in stats_map.iter() {
                let value = (attr_val.0 * per_point, attr_val.1 * per_point);
                if is_zero(value) {
                    continue;
                }
                push_source(
                    stat_sources,
                    stat_key,
                    SourceContribution {
                        label: format!("From {attr_name}"),
                        source_type: SourceType::Attribute,
                        value,
                        forge: None,
                    },
                );
            }
        }
    }
}

// SkillSpec → SubskillOwner adapter (keeps subskill module decoupled from data).
pub(crate) fn skill_spec_to_subskill_owner(
    skill: &super::types::SkillSpec,
) -> super::subskill::SubskillOwner {
    use super::subskill::{AmountSpec, AppliedStateSpec, SubskillEffect, SubskillNode, SubskillOwner, SubskillProc};
    use super::types::AppliedStateValue;

    let subskills = skill
        .subskills
        .as_ref()
        .map(|list| {
            list.iter()
                .map(|sub| SubskillNode {
                    id: sub.id.clone(),
                    effects: sub.effects.as_ref().map(|e| SubskillEffect {
                        base: e.base.clone().unwrap_or_default(),
                        per_rank: e.per_rank.clone().unwrap_or_default(),
                    }),
                    proc: sub.proc.as_ref().map(|p| SubskillProc {
                        trigger: p.trigger.clone(),
                        chance_base: p.chance.base.unwrap_or(0.0),
                        chance_per_rank: p.chance.per_rank.unwrap_or(0.0),
                        effects: p.effects.as_ref().map(|e| SubskillEffect {
                            base: e.base.clone().unwrap_or_default(),
                            per_rank: e.per_rank.clone().unwrap_or_default(),
                        }),
                        applies_states: p
                            .applies_states
                            .as_ref()
                            .map(|states| {
                                states
                                    .iter()
                                    .map(|s| match s {
                                        AppliedStateValue::Name(n) => {
                                            AppliedStateSpec::Name(n.clone())
                                        }
                                        AppliedStateValue::Full { state, amount } => {
                                            AppliedStateSpec::Full {
                                                state: state.clone(),
                                                amount: amount.as_ref().map(|a| AmountSpec {
                                                    base: a.base.unwrap_or(0.0),
                                                    per_rank: a.per_rank.unwrap_or(0.0),
                                                }),
                                            }
                                        }
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    }),
                })
                .collect()
        })
        .unwrap_or_default();
    SubskillOwner {
        id: skill.id.clone(),
        subskills,
    }
}

// Skill-scoped stats stay inside per-skill damage calc, not global stats.
pub fn apply_subskill_aggregation(
    class_id: Option<&str>,
    subskill_ranks: &HashMap<String, u32>,
    enemy_conditions: Option<&HashMap<String, bool>>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) {
    let Some(class_id) = class_id else {
        return;
    };
    let class_skills = data::get_skills_by_class(class_id);
    for skill in class_skills.iter() {
        if skill.subskills.as_ref().is_none_or(|s| s.is_empty()) {
            continue;
        }
        let owner = skill_spec_to_subskill_owner(skill);
        let agg = super::subskill::aggregate_subskill_stats(&owner, subskill_ranks, enemy_conditions);
        let label = format!("{} subtree", skill.name);
        for (key, &value) in agg.stats.iter() {
            if value == 0.0 {
                continue;
            }
            if let Some(def) = stat_def(key) {
                if def.skill_scoped.unwrap_or(false) {
                    continue;
                }
            }
            apply_contribution(
                attr_sources,
                stat_sources,
                key,
                (value, value),
                label.clone(),
                SourceType::Subskill,
                None,
            );
        }
    }
}

// e.g. vitality/8 → life_replenish. Must run AFTER attribute totals.
pub fn apply_attribute_divided_stats(
    attributes: &HashMap<String, Ranged>,
    stat_sources: &mut SourceMap,
) {
    let cfg = data::game_config();
    let Some(div_map) = cfg.attribute_divided_stats.as_ref() else {
        return;
    };
    for (attr_key, stats_map) in div_map.iter() {
        let attr_val = attributes.get(attr_key).copied().unwrap_or((0.0, 0.0));
        let attr_name = cfg
            .attributes
            .iter()
            .find(|a| &a.key == attr_key)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| attr_key.clone());
        for (stat_key, &divisor) in stats_map.iter() {
            if divisor <= 0.0 {
                continue;
            }
            let contrib_min = (attr_val.0 / divisor).floor();
            let contrib_max = (attr_val.1 / divisor).floor();
            if contrib_min == 0.0 && contrib_max == 0.0 {
                continue;
            }
            push_source(
                stat_sources,
                stat_key,
                SourceContribution {
                    label: format!("From {attr_name} (÷{divisor})"),
                    source_type: SourceType::Attribute,
                    value: (contrib_min, contrib_max),
                    forge: None,
                },
            );
        }
    }
}

// Side-effect: push passive stats. Returns ranks map for the conversion pass.
pub fn apply_item_granted_passive_stats(
    inventory: &Inventory,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) -> HashMap<String, Ranged> {
    let ranks = aggregate_item_skill_bonuses(inventory, &data::data().items);
    for granted in data::item_granted_skills().iter() {
        let key = normalize_skill_name(&granted.name);
        let (rank_min, rank_max) = ranks.get(&key).copied().unwrap_or((0.0, 0.0));
        if rank_max <= 0.0 {
            continue;
        }
        let Some(passive) = granted.passive_stats.as_ref() else {
            continue;
        };
        let mut out: HashMap<String, Ranged> = HashMap::new();
        if let Some(base) = passive.base.as_ref() {
            for (k, &v) in base.iter() {
                out.insert(k.clone(), (v, v));
            }
        }
        if let Some(per_rank) = passive.per_rank.as_ref() {
            for (k, &v) in per_rank.iter() {
                let existing = out.get(k).copied().unwrap_or((0.0, 0.0));
                let min = existing.0 + v * rank_min;
                let max = existing.1 + v * rank_max;
                out.insert(k.clone(), (min, max));
            }
        }
        let rank_label = if rank_min == rank_max {
            format!("{rank_min}")
        } else {
            format!("{rank_min}-{rank_max}")
        };
        let label = format!("{} (rank {rank_label})", granted.name);
        for (k, v) in out.iter() {
            if is_zero(*v) {
                continue;
            }
            apply_contribution(
                attr_sources,
                stat_sources,
                k,
                *v,
                label.clone(),
                SourceType::Item,
                None,
            );
        }
    }
    ranks
}

// Fans all_resistances / max_all_resistances (and `_more` variants) out
// to per-element buckets.
pub fn apply_stat_fan_outs(stat_sources: &mut SourceMap) {
    let variants: [&str; 2] = ["", "_more"];
    for (from, targets) in STAT_FAN_OUTS.iter() {
        for variant in variants.iter() {
            let from_key = format!("{from}{variant}");
            let sources_clone = match stat_sources.get(&from_key) {
                Some(list) if !list.is_empty() => list.clone(),
                _ => continue,
            };
            for target in targets.iter() {
                let target_key = format!("{target}{variant}");
                for src in sources_clone.iter() {
                    push_source(stat_sources, &target_key, src.clone());
                }
            }
        }
    }
}

// ---------- finalization helpers ----------

// Display name; `_more` variants are prefixed with "Total ".
fn stat_name(key: &str) -> String {
    if let Some(def) = stat_def(key) {
        if key.ends_with("_more") && def.key != key {
            return format!("Total {}", def.name);
        }
        return def.name.clone();
    }
    key.to_string()
}

fn compute_final_attributes(attr_sources: &SourceMap) -> HashMap<String, Ranged> {
    let mut attributes = HashMap::new();
    for attr in data::game_config().attributes.iter() {
        let sum =
            sum_contributions(attr_sources.get(&attr.key).map(|v| v.as_slice()).unwrap_or(&[]));
        attributes.insert(attr.key.clone(), sum);
    }
    attributes
}

fn compute_final_stats(stat_sources: &SourceMap) -> HashMap<String, Ranged> {
    let mut stats = HashMap::with_capacity(stat_sources.len());
    for (k, list) in stat_sources.iter() {
        stats.insert(k.clone(), sum_contributions(list));
    }
    stats
}

// life/mana × increased × more; replenishes opt out of floor.
pub fn apply_multipliers_pass(stats: &mut HashMap<String, Ranged>) {
    apply_multiplier(stats, "life", Some("increased_life"), Some("increased_life_more"), true);
    apply_multiplier(stats, "mana", Some("increased_mana"), Some("increased_mana_more"), true);
    apply_multiplier(
        stats,
        "mana_replenish",
        None,
        Some("mana_replenish_more"),
        false,
    );
    apply_multiplier(
        stats,
        "life_replenish",
        None,
        Some("life_replenish_more"),
        false,
    );
}

// Item-granted skill conversions; returns touched keys for re-sum.
pub fn apply_item_granted_conversions(
    item_granted_ranks: &HashMap<String, Ranged>,
    stats: &HashMap<String, Ranged>,
    stat_sources: &mut SourceMap,
) -> HashSet<String> {
    let mut touched: HashSet<String> = HashSet::new();
    for granted in data::item_granted_skills().iter() {
        let Some(converts) = granted.passive_converts.as_ref() else {
            continue;
        };
        let key = normalize_skill_name(&granted.name);
        let (rank_min, rank_max) = item_granted_ranks.get(&key).copied().unwrap_or((0.0, 0.0));
        if rank_max <= 0.0 {
            continue;
        }
        for conv in converts.per_rank.iter() {
            let from = stats.get(&conv.from).copied().unwrap_or((0.0, 0.0));
            let from_more = stats
                .get(&format!("{}_more", conv.from))
                .copied()
                .unwrap_or((0.0, 0.0));
            let effective = combine_additive_and_more(from, from_more);
            let add_min = (conv.pct * rank_min / 100.0) * effective.0;
            let add_max = (conv.pct * rank_max / 100.0) * effective.1;
            if add_min == 0.0 && add_max == 0.0 {
                continue;
            }
            let rank_label = if rank_min == rank_max {
                format!("{rank_min}")
            } else {
                format!("{rank_min}-{rank_max}")
            };
            let label = format!(
                "Converted from {} ({}, rank {rank_label})",
                stat_name(&conv.from),
                granted.name
            );
            push_source(
                stat_sources,
                &conv.to,
                SourceContribution {
                    label,
                    source_type: SourceType::Item,
                    value: (add_min, add_max),
                    forge: None,
                },
            );
            touched.insert(conv.to.clone());
        }
    }
    touched
}

// Tree conversions can target attributes (re-summed in place) or stats
// (returned in `touched` for the orchestrator to re-sum).
#[allow(clippy::too_many_arguments)]
pub fn apply_tree_conversions(
    tree_conversions: &[(ParsedConversion, String)],
    attributes: &mut HashMap<String, Ranged>,
    stats: &HashMap<String, Ranged>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) -> HashSet<String> {
    use super::tree::parse::ConvertKind;
    let mut touched: HashSet<String> = HashSet::new();
    for (conv, source_label) in tree_conversions.iter() {
        let source_value: Ranged = match conv.from_kind {
            ConvertKind::Attribute => attributes
                .get(&conv.from_key)
                .copied()
                .unwrap_or((0.0, 0.0)),
            ConvertKind::Stat => {
                let from = stats.get(&conv.from_key).copied().unwrap_or((0.0, 0.0));
                let from_more = stats
                    .get(&format!("{}_more", conv.from_key))
                    .copied()
                    .unwrap_or((0.0, 0.0));
                combine_additive_and_more(from, from_more)
            }
        };
        let add_min = (conv.pct / 100.0) * source_value.0;
        let add_max = (conv.pct / 100.0) * source_value.1;
        if add_min == 0.0 && add_max == 0.0 {
            continue;
        }
        let label = format!(
            "{source_label}: {}% of {}",
            conv.pct,
            stat_name(&conv.from_key)
        );
        let contrib = SourceContribution {
            label,
            source_type: SourceType::Tree,
            value: (add_min, add_max),
            forge: None,
        };
        match conv.to_kind {
            ConvertKind::Attribute => {
                push_source(attr_sources, &conv.to_key, contrib);
                if let Some(list) = attr_sources.get(&conv.to_key) {
                    attributes.insert(conv.to_key.clone(), sum_contributions(list));
                }
            }
            ConvertKind::Stat => {
                push_source(stat_sources, &conv.to_key, contrib);
                touched.insert(conv.to_key.clone());
            }
        }
    }
    touched
}

// Post-pipeline disable flags. Currently only zeros life_replenish/_pct.
pub fn apply_tree_disables(disables: &HashSet<DisableTarget>, stats: &mut HashMap<String, Ranged>) {
    if disables.contains(&DisableTarget::LifeReplenish) {
        stats.insert("life_replenish".to_string(), (0.0, 0.0));
        stats.insert("life_replenish_pct".to_string(), (0.0, 0.0));
    }
}

// ---------- orchestrator ----------

#[derive(Debug, Clone, Copy)]
pub struct BuildStatsInput<'a> {
    pub class_id: Option<&'a str>,
    pub level: u32,
    pub allocated_attrs: &'a HashMap<String, u32>,
    pub inventory: &'a Inventory,
    pub skill_ranks: &'a HashMap<String, u32>,
    pub active_aura_id: Option<&'a str>,
    pub active_buffs: &'a HashMap<String, bool>,
    pub custom_stats: &'a [CustomStat],
    pub allocated_tree_nodes: &'a HashSet<u32>,
    pub tree_socketed: &'a HashMap<u32, TreeSocketContent>,
    pub player_conditions: &'a HashMap<String, bool>,
    pub subskill_ranks: &'a HashMap<String, u32>,
    pub enemy_conditions: &'a HashMap<String, bool>,
}

// Single pass of the full stat-aggregation pipeline; compute_build_stats
// wraps it with an automatic crit-below-40 re-run when needed.
pub fn compute_build_stats_core(input: &BuildStatsInput) -> ComputedStats {
    let mut attr_sources: SourceMap = HashMap::new();
    let mut stat_sources: SourceMap = HashMap::new();

    // 1. Base attributes (defaults + class base + allocated)
    apply_base_attributes(input.class_id, input.allocated_attrs, &mut attr_sources);

    // 2. Inventory loop (implicits + affixes + sockets + runeword + augment)
    let weapon_has_aps =
        apply_inventory(input.inventory, &mut attr_sources, &mut stat_sources);

    // 3. Tree contributions (returns deferred conversions + disables)
    let tree_agg = apply_tree_contributions(
        input.allocated_tree_nodes,
        input.player_conditions,
        &mut attr_sources,
        &mut stat_sources,
    );

    // 4. Tree jewelry sockets
    apply_tree_jewelry_sockets(
        input.allocated_tree_nodes,
        input.tree_socketed,
        &mut attr_sources,
        &mut stat_sources,
    );

    // 5. Set bonuses
    apply_set_bonuses(input.inventory, &mut attr_sources, &mut stat_sources);

    // 6. Default/class base stats + per-level
    apply_class_baseline(input.class_id, input.level, weapon_has_aps, &mut stat_sources);

    // 7. Custom user-defined stats
    apply_custom_stats(input.custom_stats, &mut attr_sources, &mut stat_sources);

    // 8. Skill ranks → passive stats
    apply_skill_ranks(
        input.class_id,
        input.skill_ranks,
        input.active_aura_id,
        input.active_buffs,
        input.inventory,
        &mut attr_sources,
        &mut stat_sources,
    );

    // 9. Increased all attributes % (applies to each attribute's flat sum)
    apply_increased_all_attributes(&mut attr_sources, &stat_sources);

    // 10. Increased per-attribute additive + more (compound delta)
    apply_increased_per_attribute(&mut attr_sources, &stat_sources);

    // 11. Stats per attribute (e.g. strength → enhanced_damage)
    apply_stats_per_attribute(input.class_id, &attr_sources, &mut stat_sources);

    // 12. Subskill aggregation (gates skill-scoped stats out)
    apply_subskill_aggregation(
        input.class_id,
        input.subskill_ranks,
        Some(input.enemy_conditions),
        &mut attr_sources,
        &mut stat_sources,
    );

    // 13. Compute attribute totals
    let mut attributes = compute_final_attributes(&attr_sources);

    // 14. Attribute-divided stats (e.g. vitality/8 → life_replenish)
    apply_attribute_divided_stats(&attributes, &mut stat_sources);

    // 15. Item-granted skill bonuses → passive stats; ranks reused in step 19.
    let item_granted_ranks =
        apply_item_granted_passive_stats(input.inventory, &mut attr_sources, &mut stat_sources);

    // 16. Stat fan-outs (all_resistances → per-element variants)
    apply_stat_fan_outs(&mut stat_sources);

    // 17. Compute stat totals from sources
    let mut stats = compute_final_stats(&stat_sources);

    // 18. Multiplier pass (life/mana/replenishes)
    apply_multipliers_pass(&mut stats);

    // 19. Item-granted skill conversions
    let touched_item = apply_item_granted_conversions(&item_granted_ranks, &stats, &mut stat_sources);

    // 20. Tree conversions (can target attributes or stats)
    let touched_tree = apply_tree_conversions(
        &tree_agg.conversions,
        &mut attributes,
        &stats,
        &mut attr_sources,
        &mut stat_sources,
    );

    // 21. Re-sum touched stat keys after conversions injected new sources.
    for k in touched_item.iter().chain(touched_tree.iter()) {
        if let Some(list) = stat_sources.get(k) {
            stats.insert(k.clone(), sum_contributions(list));
        }
    }

    // 22. Tree disables (zero out life_replenish if flagged)
    apply_tree_disables(&tree_agg.disables, &mut stats);

    ComputedStats {
        attributes,
        stats,
        attribute_sources: attr_sources,
        stat_sources,
    }
}

// Re-runs the pipeline with `crit_chance_below_40` flipped on when the
// baseline crit < 40% and tree nodes are allocated.
pub fn compute_build_stats(input: &BuildStatsInput) -> ComputedStats {
    let baseline = compute_build_stats_core(input);
    let already_set = input
        .player_conditions
        .get("crit_chance_below_40")
        .copied()
        .unwrap_or(false);
    if !already_set && !input.allocated_tree_nodes.is_empty() {
        let crit = baseline
            .stats
            .get("crit_chance")
            .copied()
            .unwrap_or((0.0, 0.0));
        if crit.0 < 40.0 {
            let mut conds = input.player_conditions.clone();
            conds.insert("crit_chance_below_40".to_string(), true);
            let new_input = BuildStatsInput {
                player_conditions: &conds,
                ..*input
            };
            return compute_build_stats_core(&new_input);
        }
    }
    baseline
}

// ---------- per-stat breakdown ----------
//
// PoB-style explainability: surfaces additive/more lists per stat key and
// regroups them by source type for the UI.

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatTypeSubtotal {
    pub source_type: SourceType,
    pub sum: Ranged,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatBreakdown {
    pub stat_key: String,
    pub stat_name: String,
    pub is_percent: bool,
    pub has_more: bool,
    /// Only set for apply_multipliers_pass targets (life/mana/replenishes)
    /// when `increased_X` has contributions; UI uses it to split sections.
    pub has_increased: bool,

    /// Flat-units sources at `stat_key` for multiplied stats; percent
    /// sources for non-multiplied percent stats.
    pub additive_sum: Ranged,
    pub additive_sources: Vec<SourceContribution>,
    pub additive_by_type: Vec<StatTypeSubtotal>,

    pub increased_sum: Ranged,
    pub increased_sources: Vec<SourceContribution>,
    pub increased_by_type: Vec<StatTypeSubtotal>,

    pub more_sum: Ranged,
    pub more_sources: Vec<SourceContribution>,
    pub more_by_type: Vec<StatTypeSubtotal>,

    pub combined: Ranged,
}

// For multiplied-flat stats the percent multipliers live under different
// keys than the flat base; mirror apply_multipliers_pass calls.
fn multiplier_keys_for(stat_key: &str) -> (Option<&'static str>, Option<&'static str>) {
    match stat_key {
        "life" => (Some("increased_life"), Some("increased_life_more")),
        "mana" => (Some("increased_mana"), Some("increased_mana_more")),
        "mana_replenish" => (None, Some("mana_replenish_more")),
        "life_replenish" => (None, Some("life_replenish_more")),
        _ => (None, None),
    }
}

// `_more` → "Total X"; falls back to raw key when no def.
fn resolved_stat_name(stat_key: &str) -> String {
    if let Some(def) = STAT_DEFS_MAP.get(stat_key) {
        return def.name.clone();
    }
    if let Some(base) = stat_key.strip_suffix("_more") {
        if let Some(def) = STAT_DEFS_MAP.get(base) {
            return format!("Total {}", def.name);
        }
    }
    stat_key.to_string()
}

// Sorted by |magnitude| desc; NaN sinks; ties broken by SourceType for
// stable ordering across runs (HashMap iteration is non-deterministic).
fn group_by_source_type(sources: &[SourceContribution]) -> Vec<StatTypeSubtotal> {
    let mut map: HashMap<SourceType, (Ranged, u32)> = HashMap::new();
    for s in sources.iter() {
        let entry = map
            .entry(s.source_type)
            .or_insert(((0.0, 0.0), 0));
        entry.0 .0 += s.value.0;
        entry.0 .1 += s.value.1;
        entry.1 += 1;
    }
    let mut out: Vec<StatTypeSubtotal> = map
        .into_iter()
        .map(|(source_type, (sum, count))| StatTypeSubtotal {
            source_type,
            sum,
            count,
        })
        .collect();
    let safe_mag = |v: f64| if v.is_nan() { f64::NEG_INFINITY } else { v };
    out.sort_by(|a, b| {
        let a_mag = safe_mag(a.sum.0.abs().max(a.sum.1.abs()));
        let b_mag = safe_mag(b.sum.0.abs().max(b.sum.1.abs()));
        b_mag
            .partial_cmp(&a_mag)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.source_type.cmp(&b.source_type))
    });
    out
}

// Builds a StatBreakdown for `stat_key`. `final_value` is the engine's
// post-pipeline value; when provided it's used as `combined` so the modal
// can't disagree with the StatRow. Pass `None` for attributes.
//
// Two stat shapes:
//   1. Multiplied-flat (life/mana/replenishes): increased/more keys are
//      `increased_X` / `increased_X_more` (resolved via multiplier_keys_for).
//   2. Plain percent stats: more key is `${stat_key}_more`.
pub fn compute_stat_breakdown(
    stat_sources: &SourceMap,
    stat_key: &str,
    final_value: Option<Ranged>,
) -> StatBreakdown {
    let additive_sources: Vec<SourceContribution> = stat_sources
        .get(stat_key)
        .cloned()
        .unwrap_or_default();

    let (inc_key_opt, more_key_owned) = match multiplier_keys_for(stat_key) {
        (inc, more) => (inc, more.map(|s| s.to_string())),
    };
    let more_key = more_key_owned.unwrap_or_else(|| format!("{stat_key}_more"));

    let increased_sources: Vec<SourceContribution> = inc_key_opt
        .and_then(|k| stat_sources.get(k))
        .cloned()
        .unwrap_or_default();
    let more_sources: Vec<SourceContribution> = stat_sources
        .get(&more_key)
        .cloned()
        .unwrap_or_default();

    let additive_sum = sum_contributions(&additive_sources);
    let increased_sum = sum_contributions(&increased_sources);
    let more_sum = sum_contributions(&more_sources);
    let has_more = !more_sources.is_empty();
    let has_increased = !increased_sources.is_empty();

    // Prefer the engine's final value so the modal can't diverge from the
    // StatRow; reconstruct only when no final value is supplied.
    let combined_raw: Ranged = if let Some(fv) = final_value {
        fv
    } else if inc_key_opt.is_some() || matches!(stat_key, "mana_replenish" | "life_replenish") {
        let min = additive_sum.0
            * (1.0 + increased_sum.0 / 100.0)
            * (1.0 + more_sum.0 / 100.0);
        let max = additive_sum.1
            * (1.0 + increased_sum.1 / 100.0)
            * (1.0 + more_sum.1 / 100.0);
        (min, max)
    } else if has_more {
        combine_additive_and_more(additive_sum, more_sum)
    } else {
        additive_sum
    };
    // Normalise min/max — negative-spanning factor ranges can flip order
    // (e.g. (-100,-50) × (-50,50)).
    let combined: Ranged = if combined_raw.0 <= combined_raw.1 {
        combined_raw
    } else {
        (combined_raw.1, combined_raw.0)
    };

    let additive_by_type = group_by_source_type(&additive_sources);
    let increased_by_type = group_by_source_type(&increased_sources);
    let more_by_type = group_by_source_type(&more_sources);

    let is_percent = stat_def(stat_key)
        .and_then(|d| d.format.as_deref())
        .map(|f| f == "percent")
        .unwrap_or(false);

    StatBreakdown {
        stat_key: stat_key.to_string(),
        stat_name: resolved_stat_name(stat_key),
        is_percent,
        has_more,
        has_increased,
        additive_sum,
        additive_sources,
        additive_by_type,
        increased_sum,
        increased_sources,
        increased_by_type,
        more_sum,
        more_sources,
        more_by_type,
        combined,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrib(value: Ranged) -> SourceContribution {
        SourceContribution {
            label: "test".to_string(),
            source_type: SourceType::Item,
            value,
            forge: None,
        }
    }

    // ---- is_zero ----

    #[test]
    fn is_zero_recognises_zero_and_nonzero() {
        assert!(is_zero((0.0, 0.0)));
        assert!(!is_zero((1.0, 1.0)));
        assert!(!is_zero((0.0, 1.0)));
        assert!(!is_zero((-1.0, 0.0)));
    }

    // ---- push_source ----

    #[test]
    fn push_source_skips_zero_and_appends() {
        let mut map: SourceMap = HashMap::new();
        push_source(&mut map, "life", contrib((0.0, 0.0)));
        assert!(map.is_empty(), "zero value should be skipped");
        push_source(&mut map, "life", contrib((10.0, 20.0)));
        assert_eq!(map.get("life").map(|v| v.len()), Some(1));
        push_source(&mut map, "life", contrib((5.0, 5.0)));
        assert_eq!(map.get("life").map(|v| v.len()), Some(2));
    }

    // ---- sum_contributions ----

    #[test]
    fn sum_contributions_adds_endpoints() {
        let list = vec![
            contrib((10.0, 20.0)),
            contrib((5.0, 5.0)),
            contrib((-2.0, -1.0)),
        ];
        assert_eq!(sum_contributions(&list), (13.0, 24.0));
        assert_eq!(sum_contributions(&[]), (0.0, 0.0));
    }

    #[test]
    fn sum_ranged_from_map_floors_endpoints() {
        let mut map: SourceMap = HashMap::new();
        push_source(&mut map, "life", contrib((10.5, 20.7)));
        push_source(&mut map, "life", contrib((5.5, 5.5)));
        // sum = (16.0, 26.2); floored → (16, 26).
        assert_eq!(sum_ranged_from_map(&map, "life"), (16.0, 26.0));
        assert_eq!(sum_ranged_from_map(&map, "missing"), (0.0, 0.0));
    }

    // ---- compute_item_effective_defense ----

    #[test]
    fn effective_defense_none_when_base_missing() {
        assert_eq!(compute_item_effective_defense(None, None, None), None);
        assert_eq!(compute_item_effective_defense(Some(10.0), None, None), None);
        assert_eq!(compute_item_effective_defense(None, Some(10.0), None), None);
    }

    #[test]
    fn effective_defense_no_enhancement_floors_to_base() {
        assert_eq!(
            compute_item_effective_defense(Some(10.0), Some(20.0), None),
            Some((10.0, 20.0))
        );
    }

    #[test]
    fn effective_defense_applies_enhanced_percent() {
        // base 10..20, +50% min .. +100% max → 15..40
        assert_eq!(
            compute_item_effective_defense(Some(10.0), Some(20.0), Some((50.0, 100.0))),
            Some((15.0, 40.0))
        );
        // fractional product (10 * 1.33 = 13.3) gets floor'd
        assert_eq!(
            compute_item_effective_defense(Some(10.0), Some(10.0), Some((33.0, 33.0))),
            Some((13.0, 13.0))
        );
    }

    // ---- combine_additive_and_more ----

    #[test]
    fn combine_additive_only() {
        assert_eq!(
            combine_additive_and_more((50.0, 50.0), (0.0, 0.0)),
            (50.0, 50.0)
        );
    }

    #[test]
    fn combine_more_only() {
        assert_eq!(
            combine_additive_and_more((0.0, 0.0), (30.0, 30.0)),
            (30.0, 30.0)
        );
    }

    #[test]
    fn combine_additive_and_more_compounds() {
        // 50% additive × 30% more → (1.5)(1.3)-1 = 95%
        let v = combine_additive_and_more((50.0, 50.0), (30.0, 30.0));
        assert!((v.0 - 95.0).abs() < 1e-9, "got {v:?}");
        assert!((v.1 - 95.0).abs() < 1e-9, "got {v:?}");
    }

    #[test]
    fn combine_ranged_endpoints_independent() {
        // min: (1.5)(1.3)-1 = 95   max: (2.0)(1.5)-1 = 200
        let v = combine_additive_and_more((50.0, 100.0), (30.0, 50.0));
        assert!((v.0 - 95.0).abs() < 1e-9);
        assert!((v.1 - 200.0).abs() < 1e-9);
    }

    // ---- apply_multiplier ----

    #[test]
    fn apply_multiplier_no_flat_is_noop() {
        let mut stats: HashMap<String, Ranged> = HashMap::new();
        apply_multiplier(&mut stats, "life", Some("increased_life"), None, true);
        assert!(stats.is_empty());
    }

    #[test]
    fn apply_multiplier_zero_pct_and_more_skips() {
        let mut stats: HashMap<String, Ranged> = HashMap::new();
        stats.insert("life".to_string(), (100.0, 100.0));
        apply_multiplier(
            &mut stats,
            "life",
            Some("increased_life"),
            Some("increased_life_more"),
            true,
        );
        assert_eq!(stats.get("life"), Some(&(100.0, 100.0)));
    }

    #[test]
    fn apply_multiplier_additive_floors() {
        let mut stats: HashMap<String, Ranged> = HashMap::new();
        stats.insert("life".to_string(), (100.0, 100.0));
        stats.insert("increased_life".to_string(), (50.0, 50.0));
        apply_multiplier(&mut stats, "life", Some("increased_life"), None, true);
        assert_eq!(stats.get("life"), Some(&(150.0, 150.0)));
    }

    #[test]
    fn apply_multiplier_additive_plus_more_compounds() {
        let mut stats: HashMap<String, Ranged> = HashMap::new();
        stats.insert("life".to_string(), (100.0, 100.0));
        stats.insert("increased_life".to_string(), (50.0, 50.0));
        stats.insert("increased_life_more".to_string(), (30.0, 30.0));
        apply_multiplier(
            &mut stats,
            "life",
            Some("increased_life"),
            Some("increased_life_more"),
            true,
        );
        // 100 * 1.5 * 1.3 = 195
        assert_eq!(stats.get("life"), Some(&(195.0, 195.0)));
    }

    #[test]
    fn apply_multiplier_floor_false_preserves_fractional() {
        let mut stats: HashMap<String, Ranged> = HashMap::new();
        stats.insert("mana_replenish".to_string(), (3.0, 3.0));
        stats.insert("mana_replenish_more".to_string(), (15.0, 15.0));
        apply_multiplier(
            &mut stats,
            "mana_replenish",
            None,
            Some("mana_replenish_more"),
            false,
        );
        // 3 * 1.15 = 3.45 — no floor
        let v = stats.get("mana_replenish").copied().unwrap();
        assert!((v.0 - 3.45).abs() < 1e-9);
        assert!((v.1 - 3.45).abs() < 1e-9);
    }

    // ---- STAT_FAN_OUTS ----

    #[test]
    fn stat_fan_outs_present() {
        assert_eq!(STAT_FAN_OUTS.len(), 2);
        let (all_res_key, all_res_targets) = STAT_FAN_OUTS[0];
        assert_eq!(all_res_key, "all_resistances");
        assert_eq!(all_res_targets.len(), 5);
        let (max_key, max_targets) = STAT_FAN_OUTS[1];
        assert_eq!(max_key, "max_all_resistances");
        assert_eq!(max_targets.len(), 5);
    }

    // ---- stat_def ----

    #[test]
    fn stat_def_exact_lookup_resolves_known_key() {
        let def = stat_def("all_skills");
        assert!(def.is_some(), "all_skills should be defined");
        assert_eq!(def.unwrap().key, "all_skills");
    }

    #[test]
    fn stat_def_falls_back_to_base_for_more_suffix() {
        // Pick any key that exists in game-config and verify _more falls back
        // to the same base def.
        if let Some(base) = stat_def("all_skills") {
            let synthesised = stat_def("all_skills_more");
            assert!(synthesised.is_some());
            assert_eq!(synthesised.unwrap().key, base.key);
        }
    }

    #[test]
    fn stat_def_unknown_key_returns_none() {
        assert!(stat_def("definitely_not_a_stat").is_none());
        assert!(stat_def("definitely_not_a_stat_more").is_none());
    }

    // ---- apply_contribution ----

    #[test]
    fn apply_contribution_zero_value_is_noop() {
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_contribution(
            &mut attrs,
            &mut stats,
            "life",
            (0.0, 0.0),
            "src".to_string(),
            SourceType::Item,
            None,
        );
        assert!(attrs.is_empty());
        assert!(stats.is_empty());
    }

    #[test]
    fn apply_contribution_routes_modifies_attribute_to_attrs() {
        // `to_strength` has modifiesAttribute = 'strength' in game-config.
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_contribution(
            &mut attrs,
            &mut stats,
            "to_strength",
            (5.0, 5.0),
            "ring".to_string(),
            SourceType::Item,
            None,
        );
        assert!(
            attrs.contains_key("strength"),
            "should route to strength bucket"
        );
        assert!(stats.is_empty());
    }

    #[test]
    fn apply_contribution_normal_key_routes_to_stats() {
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_contribution(
            &mut attrs,
            &mut stats,
            "fire_resistance",
            (30.0, 30.0),
            "ring".to_string(),
            SourceType::Item,
            None,
        );
        assert!(attrs.is_empty());
        assert!(stats.contains_key("fire_resistance"));
        let entries = stats.get("fire_resistance").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source_type, SourceType::Item);
        assert_eq!(entries[0].value, (30.0, 30.0));
    }

    // ---- apply_inventory ----

    use super::super::types::EquippedItem;
    use crate::calc::data;

    #[test]
    fn apply_inventory_empty_is_noop() {
        let inv: Inventory = HashMap::new();
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        let weapon_aps = apply_inventory(&inv, &mut attrs, &mut stats);
        assert!(!weapon_aps);
        assert!(attrs.is_empty());
        assert!(stats.is_empty());
    }

    #[test]
    fn apply_inventory_unknown_base_is_skipped() {
        let mut inv: Inventory = HashMap::new();
        inv.insert(
            "weapon".to_string(),
            EquippedItem {
                base_id: "definitely_not_a_real_item".to_string(),
                ..Default::default()
            },
        );
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        let weapon_aps = apply_inventory(&inv, &mut attrs, &mut stats);
        assert!(!weapon_aps);
        assert!(attrs.is_empty());
        assert!(stats.is_empty());
    }

    #[test]
    fn apply_inventory_pushes_defense_source_for_armor_with_defense_range() {
        // Programmatically find any armor-slot item with non-zero defense_min/max
        // AND no enhanced_defense implicit (else the defense gets scaled and the
        // floor(base) assertion below diverges from the pushed value).
        let any_def_item = data::data()
            .items
            .values()
            .find(|i| {
                i.slot == "armor"
                    && i.defense_min.map(|v| v > 0.0).unwrap_or(false)
                    && i.defense_max.map(|v| v > 0.0).unwrap_or(false)
                    && !i
                        .implicit
                        .as_ref()
                        .map(|m| m.contains_key("enhanced_defense"))
                        .unwrap_or(false)
            })
            .cloned();
        let Some(item_base) = any_def_item else {
            eprintln!("no defense-armor item in data; skipping");
            return;
        };

        let mut inv: Inventory = HashMap::new();
        inv.insert(
            "armor".to_string(),
            EquippedItem {
                base_id: item_base.id.clone(),
                ..Default::default()
            },
        );
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_inventory(&inv, &mut attrs, &mut stats);

        let def_entries = stats.get("defense").expect("expected defense source");
        assert_eq!(def_entries.len(), 1);
        // The pushed range matches base.defense_min..defense_max (no enhanced_defense).
        let (lo, hi) = def_entries[0].value;
        assert_eq!(lo, item_base.defense_min.unwrap().floor());
        assert_eq!(hi, item_base.defense_max.unwrap().floor());
        assert_eq!(def_entries[0].source_type, SourceType::Item);
    }

    #[test]
    fn apply_inventory_weapon_slot_with_weapon_item_flags_attack_speed() {
        let any_weapon = data::data()
            .items
            .values()
            .find(|i| {
                i.slot == "weapon"
                    && (i.attack_speed.is_some()
                        || i.implicit
                            .as_ref()
                            .is_some_and(|m| m.contains_key("attacks_per_second")))
            })
            .cloned();
        let Some(weapon_base) = any_weapon else {
            eprintln!("no weapon with attack_speed in data; skipping");
            return;
        };

        let mut inv: Inventory = HashMap::new();
        inv.insert(
            "weapon".to_string(),
            EquippedItem {
                base_id: weapon_base.id.clone(),
                ..Default::default()
            },
        );
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        let weapon_aps = apply_inventory(&inv, &mut attrs, &mut stats);
        assert!(
            weapon_aps,
            "weapon item with APS should flip weapon_has_attack_speed"
        );
    }

    // ---- tree contributions ----

    #[test]
    fn apply_tree_contributions_empty_set_returns_empty() {
        let alloc: HashSet<u32> = HashSet::new();
        let conds: HashMap<String, bool> = HashMap::new();
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        let agg = apply_tree_contributions(&alloc, &conds, &mut attrs, &mut stats);
        assert!(agg.conversions.is_empty());
        assert!(agg.disables.is_empty());
        assert!(attrs.is_empty());
        assert!(stats.is_empty());
    }

    #[test]
    fn apply_tree_contributions_skips_jewelry_nodes() {
        // Pick a jewelry node id from data — its mod lines must NOT be parsed
        // here (jewelry contributions come from the socket function instead).
        let jewelry_id = data::tree_jewelry_ids().iter().next().copied();
        let Some(node_id) = jewelry_id else {
            eprintln!("no jewelry nodes in tree data; skipping");
            return;
        };
        let mut alloc = HashSet::new();
        alloc.insert(node_id);
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        let agg = apply_tree_contributions(&alloc, &HashMap::new(), &mut attrs, &mut stats);
        assert!(agg.conversions.is_empty());
        assert!(agg.disables.is_empty());
        assert!(attrs.is_empty());
        assert!(stats.is_empty());
    }

    #[test]
    fn apply_tree_contributions_pushes_parseable_mod_line() {
        // Find any non-jewelry node whose first line parses successfully.
        let jewelry = data::tree_jewelry_ids();
        let pick = data::tree_nodes().iter().find_map(|(id_str, info)| {
            if info.n == "jewelry" || info.l.is_empty() {
                return None;
            }
            let id: u32 = id_str.parse().ok()?;
            if jewelry.contains(&id) {
                return None;
            }
            for line in &info.l {
                if parse_tree_node_mod(line).is_some() {
                    return Some(id);
                }
            }
            None
        });
        let Some(node_id) = pick else {
            eprintln!("no parseable tree node found in data; skipping");
            return;
        };
        let mut alloc = HashSet::new();
        alloc.insert(node_id);
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_tree_contributions(&alloc, &HashMap::new(), &mut attrs, &mut stats);
        let total_sources = attrs.values().map(|v| v.len()).sum::<usize>()
            + stats.values().map(|v| v.len()).sum::<usize>();
        assert!(
            total_sources >= 1,
            "expected at least one contribution from a parseable tree node"
        );
    }

    #[test]
    fn apply_tree_jewelry_sockets_empty_alloc_is_noop() {
        let alloc: HashSet<u32> = HashSet::new();
        let socketed: HashMap<u32, TreeSocketContent> = HashMap::new();
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_tree_jewelry_sockets(&alloc, &socketed, &mut attrs, &mut stats);
        assert!(attrs.is_empty());
        assert!(stats.is_empty());
    }

    #[test]
    fn apply_tree_jewelry_sockets_skips_non_jewelry_allocations() {
        // Allocating a non-jewelry node — even with a TreeSocketContent
        // wrongly placed at that id — must not push anything.
        let any_non_jewelry = data::tree_nodes().iter().find_map(|(id_str, info)| {
            if info.n == "jewelry" {
                return None;
            }
            id_str.parse::<u32>().ok()
        });
        let Some(node_id) = any_non_jewelry else {
            eprintln!("no non-jewelry nodes; skipping");
            return;
        };
        let mut alloc = HashSet::new();
        alloc.insert(node_id);
        let mut socketed = HashMap::new();
        socketed.insert(
            node_id,
            TreeSocketContent::Item {
                id: "nonexistent_id".to_string(),
            },
        );
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_tree_jewelry_sockets(&alloc, &socketed, &mut attrs, &mut stats);
        assert!(attrs.is_empty());
        assert!(stats.is_empty());
    }

    // ---- base attributes ----

    #[test]
    fn apply_base_attributes_includes_default_base_when_present() {
        // game-config defines defaultBaseAttributes for every base attribute
        // (strength=2, vitality=10, etc.). Without a class, only the defaults
        // populate attr_sources.
        let mut attrs: SourceMap = HashMap::new();
        let allocated: HashMap<String, u32> = HashMap::new();
        apply_base_attributes(None, &allocated, &mut attrs);
        // At least one attribute should have a "Base character" source.
        let has_default = attrs.values().flatten().any(|c| c.label == "Base character");
        assert!(has_default, "expected 'Base character' source");
    }

    #[test]
    fn apply_base_attributes_appends_allocated() {
        let mut attrs: SourceMap = HashMap::new();
        let mut allocated = HashMap::new();
        allocated.insert("strength".to_string(), 12_u32);
        apply_base_attributes(None, &allocated, &mut attrs);
        let str_sources = attrs.get("strength").expect("strength bucket missing");
        let alloc_entry = str_sources
            .iter()
            .find(|c| c.label == "Allocated points")
            .expect("missing allocated source");
        assert_eq!(alloc_entry.value, (12.0, 12.0));
        assert_eq!(alloc_entry.source_type, SourceType::Allocated);
    }

    #[test]
    fn apply_base_attributes_with_real_class_id_succeeds() {
        // Pick any class — the function must not panic and must populate at
        // least one attribute bucket.
        let any_class = data::data().classes.keys().next().cloned();
        let Some(class_id) = any_class else {
            eprintln!("no classes in data; skipping");
            return;
        };
        let mut attrs: SourceMap = HashMap::new();
        apply_base_attributes(Some(&class_id), &HashMap::new(), &mut attrs);
        assert!(!attrs.is_empty());
    }

    // ---- set bonuses ----

    #[test]
    fn apply_set_bonuses_empty_inventory_is_noop() {
        let inv: Inventory = HashMap::new();
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_set_bonuses(&inv, &mut attrs, &mut stats);
        assert!(attrs.is_empty());
        assert!(stats.is_empty());
    }

    #[test]
    fn apply_set_bonuses_applies_when_threshold_met() {
        // The set bonus is triggered by the base item's `setId` field — NOT by
        // sets.json's `items[].itemId` (which is for UI display only).
        // So: group base items by setId, then pick a set whose actual item
        // count covers a 2-piece-or-less bonus with non-empty stats.
        let mut by_set: HashMap<String, Vec<String>> = HashMap::new();
        for item in data::data().items.values() {
            if let Some(set_id) = item.set_id.as_deref() {
                by_set
                    .entry(set_id.to_string())
                    .or_default()
                    .push(item.id.clone());
            }
        }
        let pick = by_set.iter().find_map(|(set_id, item_ids)| {
            if item_ids.len() < 2 {
                return None;
            }
            let set = data::get_set(set_id)?;
            let qualifying = set
                .bonuses
                .iter()
                .find(|b| b.pieces <= 2 && !b.stats.is_empty())?;
            Some((set_id.clone(), item_ids.clone(), qualifying.pieces))
        });
        let Some((set_id, item_ids, pieces)) = pick else {
            eprintln!("no setId with 2+ items + 2-piece bonus available; skipping");
            return;
        };

        let mut inv: Inventory = HashMap::new();
        for (i, item_id) in item_ids.iter().take(pieces as usize).enumerate() {
            inv.insert(
                format!("set_slot_{i}"),
                EquippedItem {
                    base_id: item_id.clone(),
                    ..Default::default()
                },
            );
        }
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_set_bonuses(&inv, &mut attrs, &mut stats);
        let total = attrs.values().map(|v| v.len()).sum::<usize>()
            + stats.values().map(|v| v.len()).sum::<usize>();
        assert!(
            total > 0,
            "expected at least one bonus source from set '{}' with {} items",
            set_id,
            pieces
        );
    }

    // ---- class baseline + per-level ----

    #[test]
    fn apply_class_baseline_seeds_defaults_only_without_class() {
        let mut stats: SourceMap = HashMap::new();
        apply_class_baseline(None, 1, false, &mut stats);
        // game-config has at least crit_chance / crit_damage / etc. defaults.
        let has_default = stats.values().flatten().any(|c| c.label == "Base character");
        assert!(has_default, "expected default base-stat sources");
    }

    #[test]
    fn apply_class_baseline_suppresses_aps_when_weapon_provides_it() {
        let mut stats: SourceMap = HashMap::new();
        apply_class_baseline(None, 1, true, &mut stats);
        // The 'Base character' source for attacks_per_second must be absent.
        let has_default_aps = stats
            .get("attacks_per_second")
            .map(|list| list.iter().any(|c| c.label == "Base character"))
            .unwrap_or(false);
        assert!(!has_default_aps, "default APS should be suppressed");
    }

    #[test]
    fn apply_class_baseline_per_level_multiplies() {
        // Find a class with at least one statsPerLevel entry.
        let class = data::data()
            .classes
            .values()
            .find(|c| c.stats_per_level.values().any(|&v| v != 0.0))
            .cloned();
        let Some(cls) = class else {
            eprintln!("no class with stats_per_level; skipping");
            return;
        };
        let (stat_key, &per_level) = cls
            .stats_per_level
            .iter()
            .find(|(_, &v)| v != 0.0)
            .unwrap();

        let mut stats: SourceMap = HashMap::new();
        apply_class_baseline(Some(&cls.id), 10, false, &mut stats);
        let expected = per_level * 10.0;
        let level_source = stats
            .get(stat_key)
            .and_then(|list| {
                list.iter().find(|c| {
                    c.source_type == SourceType::Level
                        && (c.value.0 - expected).abs() < 1e-9
                        && (c.value.1 - expected).abs() < 1e-9
                })
            });
        assert!(
            level_source.is_some(),
            "expected per-level source on '{stat_key}' for class '{}'",
            cls.id
        );
    }

    // ---- custom stats ----

    #[test]
    fn apply_custom_stats_routes_via_stat_def() {
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        let customs = vec![
            CustomStat {
                stat_key: "crit_chance".to_string(),
                value: "15".to_string(),
            },
            CustomStat {
                stat_key: "fire_skill_damage".to_string(),
                value: "50-80".to_string(),
            },
            // Empty key skipped.
            CustomStat {
                stat_key: "".to_string(),
                value: "10".to_string(),
            },
            // Garbage value skipped.
            CustomStat {
                stat_key: "crit_chance".to_string(),
                value: "abc".to_string(),
            },
        ];
        apply_custom_stats(&customs, &mut attrs, &mut stats);

        let crit = stats.get("crit_chance").expect("crit_chance missing");
        assert_eq!(crit.len(), 1);
        assert_eq!(crit[0].value, (15.0, 15.0));
        assert_eq!(crit[0].source_type, SourceType::Custom);
        assert_eq!(crit[0].label, CUSTOM_SOURCE_LABEL);

        let fire = stats.get("fire_skill_damage").expect("fire_skill missing");
        assert_eq!(fire[0].value, (50.0, 80.0));
    }

    // ---- skill ranks ----

    #[test]
    fn apply_skill_ranks_no_class_id_is_noop() {
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_skill_ranks(
            None,
            &HashMap::new(),
            None,
            &HashMap::new(),
            &HashMap::new(),
            &mut attrs,
            &mut stats,
        );
        assert!(attrs.is_empty());
        assert!(stats.is_empty());
    }

    #[test]
    fn apply_skill_ranks_with_unknown_class_is_noop() {
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_skill_ranks(
            Some("nonexistent_class"),
            &HashMap::new(),
            None,
            &HashMap::new(),
            &HashMap::new(),
            &mut attrs,
            &mut stats,
        );
        assert!(attrs.is_empty());
        assert!(stats.is_empty());
    }

    // ---- 3f finalization + orchestrator smoke tests ----

    fn empty_input<'a>(
        allocated: &'a HashMap<String, u32>,
        inventory: &'a Inventory,
        skill_ranks: &'a HashMap<String, u32>,
        active_buffs: &'a HashMap<String, bool>,
        custom_stats: &'a [CustomStat],
        allocated_tree_nodes: &'a HashSet<u32>,
        tree_socketed: &'a HashMap<u32, TreeSocketContent>,
        player_conditions: &'a HashMap<String, bool>,
        subskill_ranks: &'a HashMap<String, u32>,
        enemy_conditions: &'a HashMap<String, bool>,
    ) -> BuildStatsInput<'a> {
        BuildStatsInput {
            class_id: None,
            level: 1,
            allocated_attrs: allocated,
            inventory,
            skill_ranks,
            active_aura_id: None,
            active_buffs,
            custom_stats,
            allocated_tree_nodes,
            tree_socketed,
            player_conditions,
            subskill_ranks,
            enemy_conditions,
        }
    }

    #[test]
    fn compute_build_stats_with_empty_input_does_not_panic() {
        let allocated = HashMap::new();
        let inventory = HashMap::new();
        let skill_ranks = HashMap::new();
        let active_buffs = HashMap::new();
        let custom_stats: Vec<CustomStat> = Vec::new();
        let alloc_tree = HashSet::new();
        let tree_socketed = HashMap::new();
        let player_conditions = HashMap::new();
        let subskill_ranks = HashMap::new();
        let enemy_conditions = HashMap::new();
        let input = empty_input(
            &allocated,
            &inventory,
            &skill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &player_conditions,
            &subskill_ranks,
            &enemy_conditions,
        );
        let result = compute_build_stats(&input);
        // Even with no class, default base stats from game-config populate
        // some entries.
        assert!(!result.stats.is_empty(), "default base stats should be present");
        // Base attributes from game-config seed the attribute map.
        assert!(!result.attributes.is_empty(), "attributes should be seeded from defaults");
    }

    #[test]
    fn compute_build_stats_class_seeds_attributes() {
        let allocated = HashMap::new();
        let inventory = HashMap::new();
        let skill_ranks = HashMap::new();
        let active_buffs = HashMap::new();
        let custom_stats: Vec<CustomStat> = Vec::new();
        let alloc_tree = HashSet::new();
        let tree_socketed = HashMap::new();
        let player_conditions = HashMap::new();
        let subskill_ranks = HashMap::new();
        let enemy_conditions = HashMap::new();

        let any_class = data::data().classes.keys().next().cloned();
        let Some(class_id) = any_class else {
            eprintln!("no classes; skipping");
            return;
        };
        let mut input = empty_input(
            &allocated,
            &inventory,
            &skill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &player_conditions,
            &subskill_ranks,
            &enemy_conditions,
        );
        input.class_id = Some(&class_id);
        input.level = 10;

        let result = compute_build_stats(&input);
        assert!(!result.attributes.is_empty());
        // Every base attribute key from game-config should be present.
        for attr in data::game_config().attributes.iter() {
            assert!(
                result.attributes.contains_key(&attr.key),
                "missing attribute {}",
                attr.key
            );
        }
    }

    #[test]
    fn compute_build_stats_skips_crit_rerun_when_no_tree_nodes() {
        let allocated = HashMap::new();
        let inventory = HashMap::new();
        let skill_ranks = HashMap::new();
        let active_buffs = HashMap::new();
        let custom_stats: Vec<CustomStat> = Vec::new();
        let alloc_tree: HashSet<u32> = HashSet::new(); // empty → no re-run
        let tree_socketed = HashMap::new();
        let player_conditions = HashMap::new();
        let subskill_ranks = HashMap::new();
        let enemy_conditions = HashMap::new();
        let input = empty_input(
            &allocated,
            &inventory,
            &skill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &player_conditions,
            &subskill_ranks,
            &enemy_conditions,
        );
        let result = compute_build_stats(&input);
        // Without tree nodes, the re-run is skipped → result equals core's
        // first pass. Test just confirms no panic + valid output.
        assert!(!result.stats.is_empty());
    }

    #[test]
    fn compute_build_stats_custom_stat_appears_in_output() {
        let allocated = HashMap::new();
        let inventory = HashMap::new();
        let skill_ranks = HashMap::new();
        let active_buffs = HashMap::new();
        let custom_stats = vec![CustomStat {
            stat_key: "fire_skill_damage".to_string(),
            value: "60".to_string(),
        }];
        let alloc_tree = HashSet::new();
        let tree_socketed = HashMap::new();
        let player_conditions = HashMap::new();
        let subskill_ranks = HashMap::new();
        let enemy_conditions = HashMap::new();
        let input = empty_input(
            &allocated,
            &inventory,
            &skill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &player_conditions,
            &subskill_ranks,
            &enemy_conditions,
        );
        let result = compute_build_stats(&input);
        let fire_skill = result
            .stats
            .get("fire_skill_damage")
            .expect("custom fire_skill_damage source should land in stats");
        assert_eq!(*fire_skill, (60.0, 60.0));
    }

    #[test]
    fn apply_increased_all_attributes_applies_to_each() {
        let cfg = data::game_config();
        // Seed an attribute with a known flat value.
        let first_attr = cfg.attributes.first().expect("no attrs").key.clone();
        let mut attrs: SourceMap = HashMap::new();
        push_source(
            &mut attrs,
            &first_attr,
            SourceContribution {
                label: "seed".to_string(),
                source_type: SourceType::Item,
                value: (10.0, 10.0),
                forge: None,
            },
        );
        // Seed `increased_all_attributes` percent source.
        let mut stats: SourceMap = HashMap::new();
        push_source(
            &mut stats,
            "increased_all_attributes",
            SourceContribution {
                label: "tree".to_string(),
                source_type: SourceType::Tree,
                value: (20.0, 20.0),
                forge: None,
            },
        );

        apply_increased_all_attributes(&mut attrs, &stats);

        // 10 * 20% = 2 (floor)
        let bonus_present = attrs
            .get(&first_attr)
            .map(|list| list.iter().any(|c| c.value == (2.0, 2.0)))
            .unwrap_or(false);
        assert!(bonus_present, "expected +2 bonus from 20% of 10");
    }

    #[test]
    fn apply_increased_per_attribute_compounds_add_and_more() {
        let cfg = data::game_config();
        let first_attr = cfg.attributes.first().expect("no attrs").key.clone();

        let mut attrs: SourceMap = HashMap::new();
        push_source(
            &mut attrs,
            &first_attr,
            SourceContribution {
                label: "seed".to_string(),
                source_type: SourceType::Item,
                value: (100.0, 100.0),
                forge: None,
            },
        );
        let mut stats: SourceMap = HashMap::new();
        push_source(
            &mut stats,
            &format!("increased_{first_attr}"),
            SourceContribution {
                label: "ring".to_string(),
                source_type: SourceType::Item,
                value: (50.0, 50.0),
                forge: None,
            },
        );
        push_source(
            &mut stats,
            &format!("increased_{first_attr}_more"),
            SourceContribution {
                label: "tree total".to_string(),
                source_type: SourceType::Tree,
                value: (30.0, 30.0),
                forge: None,
            },
        );

        apply_increased_per_attribute(&mut attrs, &stats);

        // 100 * 1.5 * 1.3 = 195 → bonus = 195 - 100 = 95
        let bonus_present = attrs
            .get(&first_attr)
            .map(|list| list.iter().any(|c| c.value == (95.0, 95.0)))
            .unwrap_or(false);
        assert!(bonus_present, "expected +95 compounded bonus");
    }

    #[test]
    fn apply_attribute_divided_stats_floors_per_divisor() {
        let mut attributes: HashMap<String, Ranged> = HashMap::new();
        // game-config divides vitality by 8 → life_replenish.
        attributes.insert("vitality".to_string(), (50.0, 50.0));
        let mut stats: SourceMap = HashMap::new();
        apply_attribute_divided_stats(&attributes, &mut stats);
        // 50 / 8 = 6.25 → floor 6.
        let life_replenish = stats
            .get("life_replenish")
            .map(|list| list.iter().any(|c| c.value == (6.0, 6.0)))
            .unwrap_or(false);
        assert!(life_replenish, "expected life_replenish from vitality÷8");
    }

    #[test]
    fn apply_tree_disables_zeros_life_replenish() {
        let mut stats: HashMap<String, Ranged> = HashMap::new();
        stats.insert("life_replenish".to_string(), (50.0, 50.0));
        stats.insert("life_replenish_pct".to_string(), (10.0, 10.0));
        let mut disables = HashSet::new();
        disables.insert(DisableTarget::LifeReplenish);
        apply_tree_disables(&disables, &mut stats);
        assert_eq!(stats.get("life_replenish").copied(), Some((0.0, 0.0)));
        assert_eq!(stats.get("life_replenish_pct").copied(), Some((0.0, 0.0)));
    }

    #[test]
    fn apply_skill_ranks_passive_pushes_when_allocated() {
        // Find any class+skill combo with passive_stats.base set.
        let pick = data::data().skills_by_class.iter().find_map(|(class_id, skills)| {
            for s in skills.iter() {
                let has_passive = s
                    .passive_stats
                    .as_ref()
                    .is_some_and(|p| p.base.as_ref().is_some_and(|b| !b.is_empty()));
                if has_passive
                    && s.kind != SkillKind::Aura
                    && s.kind != SkillKind::Buff
                    && !s
                        .tags
                        .as_ref()
                        .is_some_and(|t| t.iter().any(|x| x == "Buff"))
                {
                    return Some((class_id.clone(), s.clone()));
                }
            }
            None
        });
        let Some((class_id, skill)) = pick else {
            eprintln!("no class/skill with passive_stats.base in data; skipping");
            return;
        };

        let mut ranks = HashMap::new();
        ranks.insert(skill.id.clone(), 3_u32);
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_skill_ranks(
            Some(&class_id),
            &ranks,
            None,
            &HashMap::new(),
            &HashMap::new(),
            &mut attrs,
            &mut stats,
        );
        let total = attrs.values().map(|v| v.len()).sum::<usize>()
            + stats.values().map(|v| v.len()).sum::<usize>();
        assert!(
            total > 0,
            "expected passive contributions for skill '{}'",
            skill.id
        );
    }

    #[test]
    fn apply_inventory_implicit_overrides_replace_base_implicits() {
        // Pick any item with non-empty implicit, override one of its keys
        // with a scalar value, and verify the override appears in the source.
        let item_with_implicit = data::data()
            .items
            .values()
            .find(|i| {
                i.implicit
                    .as_ref()
                    .is_some_and(|m| !m.is_empty() && !m.contains_key("enhanced_defense"))
            })
            .cloned();
        let Some(base) = item_with_implicit else {
            eprintln!("no item with non-ED implicit in data; skipping");
            return;
        };
        let (override_key, _) = base.implicit.as_ref().unwrap().iter().next().unwrap();
        let override_key = override_key.clone();
        let mut overrides = std::collections::HashMap::new();
        overrides.insert(override_key.clone(), 999.0_f64);

        let mut inv: Inventory = HashMap::new();
        inv.insert(
            base.slot.clone(),
            EquippedItem {
                base_id: base.id.clone(),
                implicit_overrides: overrides,
                ..Default::default()
            },
        );
        let mut attrs: SourceMap = HashMap::new();
        let mut stats: SourceMap = HashMap::new();
        apply_inventory(&inv, &mut attrs, &mut stats);

        // Override value should appear somewhere — either in attr_sources (if
        // the stat modifies an attribute) or in stat_sources.
        let in_stats = stats
            .get(&override_key)
            .map(|v| v.iter().any(|c| c.value == (999.0, 999.0)))
            .unwrap_or(false);
        let in_attrs = if let Some(def) = stat_def(&override_key) {
            if let Some(target) = def.modifies_attribute.as_deref() {
                let key = if target == "all" {
                    data::game_config().attributes.first().map(|a| a.key.clone())
                } else {
                    Some(target.to_string())
                };
                key.and_then(|k| attrs.get(&k).cloned())
                    .map(|v| v.iter().any(|c| c.value == (999.0, 999.0)))
                    .unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        };
        assert!(in_stats || in_attrs, "override value not found anywhere");
    }
}
