use super::*;

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

// ---------- difficulty ----------

// Higher difficulties cut every resistance; the penalty rides `all_resistances`
// so it fans out per element and feeds the negative-resist conversions.
pub fn apply_difficulty_penalty(difficulty: Option<&str>, stat_sources: &mut SourceMap) {
    let Some(def) = data::get_difficulty(difficulty) else { return };
    if def.resist_penalty == 0.0 {
        return;
    }
    push_source(
        stat_sources,
        "all_resistances",
        SourceContribution {
            label: format!("{} difficulty", def.name),
            source_type: SourceType::Custom,
            value: (def.resist_penalty, def.resist_penalty),
            forge: None,
        },
    );
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
    skill: &crate::calc::types::SkillSpec,
) -> crate::calc::subskill::SubskillOwner {
    use crate::calc::subskill::{AmountSpec, AppliedStateSpec, SubskillEffect, SubskillNode, SubskillOwner, SubskillProc};
    use crate::calc::types::AppliedStateValue;

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

// A subskill only buffs its owning skill, so only the main skill's shared keys
// reach the stat map; the rest are returned for per-skill views.
pub fn apply_subskill_aggregation(
    class_id: Option<&str>,
    main_skill_id: Option<&str>,
    subskill_ranks: &HashMap<String, u32>,
    enemy_conditions: Option<&HashMap<String, bool>>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) -> HashMap<String, SubtreeAgg> {
    let mut out: HashMap<String, SubtreeAgg> = HashMap::new();
    let Some(class_id) = class_id else {
        return out;
    };
    for skill in data::get_skills_by_class(class_id).iter() {
        if skill.subskills.as_ref().is_none_or(|s| s.is_empty()) {
            continue;
        }
        let owner = skill_spec_to_subskill_owner(skill);
        let agg =
            crate::calc::subskill::aggregate_subskill_stats(&owner, subskill_ranks, enemy_conditions);
        let mut entry = SubtreeAgg::default();
        for (key, &value) in agg.stats.iter() {
            if value == 0.0 {
                continue;
            }
            let target = if stat_def(key).is_some_and(|d| d.skill_scoped.unwrap_or(false)) {
                &mut entry.scoped
            } else {
                &mut entry.shared
            };
            target.insert(key.clone(), (value, value));
        }
        if Some(skill.id.as_str()) == main_skill_id {
            let label = format!("{} subtree", skill.name);
            for (key, &value) in entry.shared.iter() {
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    key,
                    value,
                    label.clone(),
                    SourceType::Subskill,
                    None,
                );
            }
        }
        out.insert(skill.id.clone(), entry);
    }
    out
}

// For every non-main skill, hand the UI only the keys where its view differs:
// strip the main skill's contribution, add its own. Exact for additive keys only.
pub(crate) fn per_skill_stat_overrides(
    class_id: Option<&str>,
    main_skill_id: Option<&str>,
    subtree: &HashMap<String, SubtreeAgg>,
    stats: &HashMap<String, Ranged>,
) -> HashMap<String, HashMap<String, Ranged>> {
    let mut out: HashMap<String, HashMap<String, Ranged>> = HashMap::new();
    let Some(class_id) = class_id else {
        return out;
    };
    let keys: HashSet<&String> = subtree.values().flat_map(|a| a.shared.keys()).collect();
    if keys.is_empty() {
        return out;
    }
    let main_shared = main_skill_id
        .and_then(|id| subtree.get(id))
        .map(|a| &a.shared);
    for skill in data::get_skills_by_class(class_id).iter() {
        if Some(skill.id.as_str()) == main_skill_id {
            continue;
        }
        let own = subtree.get(&skill.id).map(|a| &a.shared);
        let pick = |map: Option<&HashMap<String, Ranged>>, key: &String| -> Ranged {
            map.and_then(|m| m.get(key)).copied().unwrap_or((0.0, 0.0))
        };
        let overrides: HashMap<String, Ranged> = keys
            .iter()
            .map(|key| {
                let base = stats.get(*key).copied().unwrap_or((0.0, 0.0));
                let main = pick(main_shared, key);
                let mine = pick(own, key);
                (
                    (*key).clone(),
                    (base.0 - main.0 + mine.0, base.1 - main.1 + mine.1),
                )
            })
            .collect();
        out.insert(skill.id.clone(), overrides);
    }
    out
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

// Pushes passive stats, returns the ranks map. `extra_ranks` carries externally
// granted skills; same-skill ranks do not stack — the higher level wins.
pub fn apply_item_granted_passive_stats(
    inventory: &Inventory,
    extra_ranks: Option<&HashMap<String, Ranged>>,
    player_conditions: &HashMap<String, bool>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) -> HashMap<String, Ranged> {
    let mut ranks = aggregate_item_skill_bonuses(inventory, &data::data().items);
    if let Some(extra) = extra_ranks {
        for (name, &(min, max)) in extra.iter() {
            let key = normalize_skill_name(name);
            let entry = ranks.entry(key).or_insert((0.0, 0.0));
            entry.0 = entry.0.max(min);
            entry.1 = entry.1.max(max);
        }
    }
    for granted in data::item_granted_skills().iter() {
        let key = normalize_skill_name(&granted.name);
        let (rank_min, rank_max) = ranks.get(&key).copied().unwrap_or((0.0, 0.0));
        if rank_max <= 0.0 {
            continue;
        }
        let Some(passive) = granted.passive_stats.as_ref() else {
            continue;
        };
        // Conditional blessings only apply when their config toggle is on.
        if let Some(cond) = granted.condition.as_ref() {
            if !player_conditions.get(cond.as_str()).copied().unwrap_or(false) {
                continue;
            }
        }
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

// "+X to All Skills (Class)" pays out only for that class; the stat key carries
// the class id, so item data needs no extra field and other classes see nothing.
pub fn apply_class_scoped_all_skills(class_id: Option<&str>, stat_sources: &mut SourceMap) {
    let Some(class_id) = class_id else {
        return;
    };
    let key = format!("all_skills_{class_id}");
    let Some(list) = stat_sources.remove(&key) else {
        return;
    };
    for src in list {
        push_source(stat_sources, "all_skills", src);
    }
}

// Every point of summed elemental resistance adds `damage_per_resist_point`% ED.
// Runs after fan-out so buckets already include the all_resistances spread.
pub fn apply_damage_per_resist(stat_sources: &mut SourceMap) {
    // Raw (unfloored) sums: the per-point rate is fractional (e.g. 0.4).
    let rate = match stat_sources.get("damage_per_resist_point") {
        Some(list) => sum_contributions(list),
        None => return,
    };
    if rate.0 == 0.0 && rate.1 == 0.0 {
        return;
    }
    const ELEMS: [&str; 5] = [
        "fire_resistance",
        "cold_resistance",
        "lightning_resistance",
        "poison_resistance",
        "arcane_resistance",
    ];
    let mut resist = (0.0, 0.0);
    for e in ELEMS.iter() {
        if let Some(list) = stat_sources.get(*e) {
            let r = sum_contributions(list);
            resist.0 += r.0;
            resist.1 += r.1;
        }
    }
    let bonus = (resist.0 * rate.0, resist.1 * rate.1);
    if bonus.0 == 0.0 && bonus.1 == 0.0 {
        return;
    }
    push_source(
        stat_sources,
        "enhanced_damage",
        SourceContribution {
            label: "Damage from Resistances".to_string(),
            source_type: SourceType::Item,
            value: bonus,
            forge: None,
        },
    );
}

// Item "X (Based on Level)": the stored value is the level-100 amount; the granted
// stat scales linearly with character level (e.g. 525 → 483 at level 92).
pub fn apply_stats_based_on_level(
    level: u32,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) {
    // (item display key, target key, label, target is an attribute → attr_sources)
    const MAP: [(&str, &str, &str, bool); 13] = [
        ("mana_based_on_level", "mana", "Mana (Based on Level)", false),
        ("life_based_on_level", "life", "Life (Based on Level)", false),
        ("damage_based_on_level", "additive_physical_damage", "Damage (Based on Level)", false),
        ("enhanced_damage_based_on_level", "enhanced_damage", "Enhanced Damage (Based on Level)", false),
        ("enhanced_defense_based_on_level", "enhanced_defense", "Enhanced Defense (Based on Level)", false),
        ("magic_find_based_on_level", "magic_find", "Magic Find (Based on Level)", false),
        ("attack_rating_based_on_level", "attack_rating", "Attack Rating (Based on Level)", false),
        ("strength_based_on_level", "strength", "Strength (Based on Level)", true),
        ("dexterity_based_on_level", "dexterity", "Dexterity (Based on Level)", true),
        ("vitality_based_on_level", "vitality", "Vitality (Based on Level)", true),
        ("energy_based_on_level", "energy", "Energy (Based on Level)", true),
        ("intelligence_based_on_level", "intelligence", "Intelligence (Based on Level)", true),
        ("armor_based_on_level", "armor", "Armor (Based on Level)", true),
    ];
    let f = level as f64 / 100.0;
    for (source, target, label, is_attr) in MAP.iter() {
        let per100 = match stat_sources.get(*source) {
            Some(list) => sum_contributions(list),
            None => continue,
        };
        if per100.0 == 0.0 && per100.1 == 0.0 {
            continue;
        }
        let bonus = (per100.0 * f, per100.1 * f);
        let dest = if *is_attr { &mut *attr_sources } else { &mut *stat_sources };
        push_source(
            dest,
            target,
            SourceContribution {
                label: label.to_string(),
                source_type: SourceType::Item,
                value: bonus,
                forge: None,
            },
        );
    }
}

