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
        let sum = sum_contributions(
            attr_sources
                .get(&attr.key)
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
        );
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
    // Item-local Enhanced Defense is already baked into each armor base.
    // Tree Defense bonuses scale the resulting character total once.
    apply_multiplier(
        stats,
        "defense",
        Some("defense_pct"),
        Some("defense_pct_more"),
        true,
    );
    apply_multiplier(
        stats,
        "life",
        Some("increased_life"),
        Some("increased_life_more"),
        true,
    );
    apply_multiplier(
        stats,
        "mana",
        Some("increased_mana"),
        Some("increased_mana_more"),
        true,
    );
    apply_multiplier(
        stats,
        "mana_replenish",
        Some("mana_replenish_increased"),
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
    apply_multiplier(
        stats,
        "mana_replenish_pct",
        Some("mana_replenish_increased"),
        Some("mana_replenish_more"),
        false,
    );
    apply_multiplier(
        stats,
        "life_replenish_pct",
        None,
        Some("life_replenish_more"),
        false,
    );
    apply_multiplier(stats, "life_steal", None, Some("life_steal_more"), false);
    // Light radius is counted in whole points by the "per point" nodes.
    apply_multiplier(stats, "light_radius", Some("light_radius_pct"), None, true);
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

// Late item/tree conversions can add raw life or mana, or change their
// additive percentages. Rebuild the affected final resource from sources;
// applying a multiplier directly to the already-final total would compound it.
pub fn refresh_converted_resources(
    stats: &mut HashMap<String, Ranged>,
    stat_sources: &SourceMap,
    touched: &HashSet<String>,
) {
    for (base, increased, more) in [
        ("life", "increased_life", "increased_life_more"),
        ("mana", "increased_mana", "increased_mana_more"),
    ] {
        if ![base, increased, more]
            .iter()
            .any(|key| touched.contains(*key))
        {
            continue;
        }
        if let Some(sources) = stat_sources.get(base) {
            stats.insert(base.to_string(), sum_contributions(sources));
            apply_multiplier(stats, base, Some(increased), Some(more), true);
        }
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

// "+X <stat> per N <source>" tree lines. The source total is only final after
// the multiplier pass, so these run late and return touched keys for re-sum.
const PER_MANA_STATS: &[(&str, &str, f64)] = &[
    (
        "magic_skill_damage_per_750_mana",
        "magic_skill_damage",
        750.0,
    ),
    (
        "ranged_physical_per_500_mana",
        "ranged_projectile_damage",
        500.0,
    ),
    ("additive_physical_per_500_mana", "enhanced_damage", 500.0),
];

const PER_LIGHT_RADIUS_STATS: &[(&str, &str, f64)] = &[
    (
        "magic_skill_damage_per_light_radius",
        "magic_skill_damage",
        1.0,
    ),
    (
        "flat_magic_skill_damage_per_light_radius",
        "flat_magic_skill_damage",
        1.0,
    ),
];

pub fn apply_per_mana_stats(
    stats: &HashMap<String, Ranged>,
    stat_sources: &mut SourceMap,
) -> HashSet<String> {
    apply_per_source_stats(stats, stat_sources, "mana", PER_MANA_STATS)
}

pub fn apply_per_light_radius_stats(
    stats: &HashMap<String, Ranged>,
    stat_sources: &mut SourceMap,
) -> HashSet<String> {
    apply_per_source_stats(stats, stat_sources, "light_radius", PER_LIGHT_RADIUS_STATS)
}

fn apply_per_source_stats(
    stats: &HashMap<String, Ranged>,
    stat_sources: &mut SourceMap,
    source_key: &str,
    table: &[(&str, &str, f64)],
) -> HashSet<String> {
    let mut touched: HashSet<String> = HashSet::new();
    let source = stats.get(source_key).copied().unwrap_or((0.0, 0.0));
    for (rate_key, target_key, per) in table.iter() {
        let rate = stats.get(*rate_key).copied().unwrap_or((0.0, 0.0));
        if rate == (0.0, 0.0) {
            continue;
        }
        // Only full steps pay out; a partial 749 mana is worth nothing.
        let value = (
            rate.0 * (source.0 / per).floor(),
            rate.1 * (source.1 / per).floor(),
        );
        let source_name = stat_name(source_key);
        let per_label = if *per == 1.0 {
            format!("per {source_name}")
        } else {
            format!("per {per} {source_name}")
        };
        push_source(
            stat_sources,
            target_key,
            SourceContribution {
                label: format!("{} ({per_label})", stat_name(target_key)),
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
            if !player_conditions
                .get(cond.as_str())
                .copied()
                .unwrap_or(false)
            {
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

// Powerfunnel reads the neutral physical attack after weapon Enhanced Damage.
fn attack_damage_after_weapon_enhanced(
    inventory: &Inventory,
    stats: &HashMap<String, Ranged>,
) -> Ranged {
    let (weapon_min, weapon_max) = inventory
        .get("weapon")
        .and_then(|item| data::get_item(&item.base_id))
        .and_then(|base| base.damage_min.zip(base.damage_max))
        .unwrap_or((2.0, 6.0));
    let stat = |key: &str| stats.get(key).copied().unwrap_or((0.0, 0.0));
    let ed = stat("enhanced_damage");
    let ed_more = stat("enhanced_damage_more");
    let flat = stat("additive_physical_damage");
    let attack = stat("attack_damage");
    let attack_more = stat("attack_damage_more");
    // Same neutral physical stages as the attack calculator, before skill,
    // crit, enemy and projectile modifiers.
    (
        (weapon_min * (1.0 + ed.0 / 100.0) * (1.0 + ed_more.0 / 100.0) + flat.0)
            * (1.0 + attack.0 / 100.0)
            * (1.0 + attack_more.0 / 100.0),
        (weapon_max * (1.0 + ed.1 / 100.0) * (1.0 + ed_more.1 / 100.0) + flat.1)
            * (1.0 + attack.1 / 100.0)
            * (1.0 + attack_more.1 / 100.0),
    )
}

// Tree conversions can target attributes (re-summed in place) or stats
// (returned in `touched` for the orchestrator to re-sum).
#[allow(clippy::too_many_arguments)]
pub fn apply_tree_conversions(
    tree_conversions: &[(ParsedConversion, String)],
    inventory: &Inventory,
    attributes: &mut HashMap<String, Ranged>,
    stats: &HashMap<String, Ranged>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) -> HashSet<String> {
    use crate::calc::tree::parse::ConvertKind;
    let mut touched: HashSet<String> = HashSet::new();
    for (conv, source_label) in tree_conversions.iter() {
        if conv.to_key == "str_to_unarmed_damage"
            && inventory
                .get("weapon")
                .and_then(|item| data::get_item(&item.base_id))
                .is_some()
        {
            continue;
        }
        let source_value: Ranged = if matches!(
            conv.from_key.as_str(),
            "sum_resistances" | "overcapped_resistances" | "negative_resistances"
        ) {
            // These nodes read all five final elemental resistances, including
            // individual-element gear and the difficulty penalty. They do not
            // read just the generic All Resistances affix bucket.
            let mut combined = stats.clone();
            combined.extend(stats_combined_map(stats));
            let mut sum = (0.0, 0.0);
            for element in crate::calc::skills::ELEMENTS {
                let key = format!("{element}_resistance");
                let value = combined.get(&key).copied().unwrap_or_default();
                let cap =
                    defense::effective_cap(&key, &combined).unwrap_or(defense::DEFAULT_RES_CAP);
                let contribution = match conv.from_key.as_str() {
                    "negative_resistances" => ((-value.1).max(0.0), (-value.0).max(0.0)),
                    "overcapped_resistances" => {
                        ((value.0 - cap).max(0.0), (value.1 - cap).max(0.0))
                    }
                    _ => value,
                };
                sum.0 += contribution.0;
                sum.1 += contribution.1;
            }
            sum
        } else if conv.from_key == "attack_damage"
            && conv.to_key == "increased_life"
            && conv.from_kind == ConvertKind::Stat
        {
            attack_damage_after_weapon_enhanced(inventory, stats)
        } else {
            match conv.from_kind {
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
    if disables.contains(&DisableTarget::Dodge) {
        for key in ["dodge_chance", "dodge_physical_damage_chance"] {
            stats.insert(key.to_string(), (0.0, 0.0));
            stats.insert(format!("{key}_more"), (0.0, 0.0));
        }
    }
    if disables.contains(&DisableTarget::ManaReplenish) {
        stats.insert("mana_replenish".to_string(), (0.0, 0.0));
        stats.insert("mana_replenish_pct".to_string(), (0.0, 0.0));
    }
}
