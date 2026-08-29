use super::*;

// ---------- finalization helpers ----------

// Display name; `_more` variants are prefixed with "Total ".
pub(crate) fn stat_name(key: &str) -> String {
    if let Some(def) = stat_def(key) {
        if key.ends_with("_more") && def.key != key {
            return format!("Total {}", def.name);
        }
        return def.name.clone();
    }
    key.to_string()
}

pub(crate) fn compute_final_attributes(attr_sources: &SourceMap) -> HashMap<String, Ranged> {
    let mut attributes = HashMap::new();
    for attr in data::game_config().attributes.iter() {
        let sum =
            sum_contributions(attr_sources.get(&attr.key).map(|v| v.as_slice()).unwrap_or(&[]));
        attributes.insert(attr.key.clone(), sum);
    }
    attributes
}

pub(crate) fn compute_final_stats(stat_sources: &SourceMap) -> HashMap<String, Ranged> {
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
    // Ailment durations: (base + flat seconds) × increased%. No floor — the
    // fraction is visible progress toward the next once-per-second tick.
    for a in AILMENT_DURATION_PREFIXES {
        apply_multiplier(
            stats,
            &format!("{a}_duration"),
            Some(&format!("{a}_duration_pct")),
            None,
            false,
        );
    }
}

// Ailments with a modeled base duration in game-config defaultBaseStats.
pub(crate) const AILMENT_DURATION_PREFIXES: &[&str] = &[
    "bleed",
    "burning",
    "frostbite",
    "permafrost",
    "poisoned",
    "rabies",
    "shadowburn",
    "stasis",
];

// "+X% <stat> per N Mana" tree lines. Mana is only final after the multiplier
// pass, so this runs late and returns touched keys for re-sum.
const PER_MANA_STATS: &[(&str, &str, f64)] = &[
    ("magic_skill_damage_per_750_mana", "magic_skill_damage", 750.0),
    ("ranged_physical_per_500_mana", "ranged_projectile_damage", 500.0),
    ("additive_physical_per_500_mana", "enhanced_damage", 500.0),
];

pub fn apply_per_mana_stats(
    stats: &HashMap<String, Ranged>,
    stat_sources: &mut SourceMap,
) -> HashSet<String> {
    let mut touched: HashSet<String> = HashSet::new();
    let mana = stats.get("mana").copied().unwrap_or((0.0, 0.0));
    for (source_key, target_key, per) in PER_MANA_STATS.iter() {
        let rate = stats.get(*source_key).copied().unwrap_or((0.0, 0.0));
        if rate == (0.0, 0.0) {
            continue;
        }
        // Only full steps pay out; a partial 749 mana is worth nothing.
        let value = (
            rate.0 * (mana.0 / per).floor(),
            rate.1 * (mana.1 / per).floor(),
        );
        push_source(
            stat_sources,
            target_key,
            SourceContribution {
                label: format!("{} (per {per} Mana)", stat_name(target_key)),
                source_type: SourceType::Tree,
                value,
                forge: None,
            },
        );
        touched.insert((*target_key).to_string());
    }
    touched
}

// Item-granted skill conversions; returns touched keys for re-sum.
pub fn apply_item_granted_conversions(
    item_granted_ranks: &HashMap<String, Ranged>,
    stats: &HashMap<String, Ranged>,
    player_conditions: &HashMap<String, bool>,
    stat_sources: &mut SourceMap,
) -> HashSet<String> {
    let mut touched: HashSet<String> = HashSet::new();
    for granted in data::item_granted_skills().iter() {
        let Some(converts) = granted.passive_converts.as_ref() else {
            continue;
        };
        // Conditional blessings only convert while their config toggle is on.
        if let Some(cond) = granted.condition.as_ref() {
            if !player_conditions.get(cond.as_str()).copied().unwrap_or(false) {
                continue;
            }
        }
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
            let share_min = (conv.base_pct + conv.pct * rank_min) / 100.0;
            let share_max = (conv.base_pct + conv.pct * rank_max) / 100.0;
            let add_min = share_min * effective.0;
            let add_max = share_max * effective.1;
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
                    label: label.clone(),
                    source_type: SourceType::Item,
                    value: (add_min, add_max),
                    forge: None,
                },
            );
            touched.insert(conv.to.clone());
            if conv.replaces {
                // Take the share out of each half separately. The combined
                // figure includes the `_more` multiplier, so subtracting it
                // from the additive key alone would drive that key negative.
                for (key, val) in [
                    (conv.from.clone(), from),
                    (format!("{}_more", conv.from), from_more),
                ] {
                    push_source(
                        stat_sources,
                        &key,
                        SourceContribution {
                            label: format!("{label} (removed)"),
                            source_type: SourceType::Item,
                            value: (-share_min * val.0, -share_max * val.1),
                            forge: None,
                        },
                    );
                    touched.insert(key);
                }
            }
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
    use crate::calc::tree::parse::ConvertKind;
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

