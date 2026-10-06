use super::*;

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

// Effective rank = base + all_skills + element + item; applies base + per_rank *
// (rank - 1) per key. Auras need active_aura_id, buffs need the active_buffs flag.
#[allow(clippy::too_many_arguments)]
pub fn apply_skill_ranks(
    class_id: Option<&str>,
    skill_ranks: &HashMap<String, u32>,
    active_aura_id: Option<&str>,
    active_buffs: &HashMap<String, bool>,
    stack_counts: &HashMap<String, u32>,
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
    if class_id == "butcher" {
        apply_butcher_forms(
            class_skills,
            skill_ranks,
            active_buffs,
            all_skills_bonus,
            &item_skill_bonuses,
            stat_sources,
        );
    }

    for skill in class_skills.iter() {
        if class_id == "butcher" && is_butcher_form_skill(&skill.id) {
            continue;
        }
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
        if skill.has_buff_toggle() && !active_buffs.get(&skill.id).copied().unwrap_or(false) {
            continue;
        }
        let effective_rank = passive_rank_range(
            skill,
            base_rank,
            all_skills_bonus,
            &item_skill_bonuses,
            stat_sources,
        );
        let audited_butcher_value = class_id == "butcher"
            && matches!(
                skill.id.as_str(),
                "ending_fate"
                    | "awakening_fury"
                    | "hunger_for_blood"
                    | "fuel_to_fire"
                    | "enraged_mania"
            );
        let minimum_rank = if audited_butcher_value { 0.0 } else { 1.0 };
        let eff_min = effective_rank.0.max(minimum_rank);
        let eff_max = effective_rank.1.max(minimum_rank);

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
        if audited_butcher_value {
            for (key, value) in combined.iter_mut() {
                let low = passive_coefficient_at_rank(passive, key, effective_rank.0);
                let high = passive_coefficient_at_rank(passive, key, effective_rank.1);
                *value = (low.min(high), low.max(high));
            }
        }
        if skill.id == "fuel_to_fire" {
            let consumed = stack_counts
                .get("fuel_to_fire_consumed_objects")
                .copied()
                .unwrap_or(0);
            let factor = 1.0 + f64::from(consumed) * 0.01;
            if let Some(damage) = combined.get_mut("damage") {
                damage.0 *= factor;
                damage.1 *= factor;
            }
        }

        if skill.kind == SkillKind::Aura {
            apply_aura_effectiveness(&mut combined, stat_sources);
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

pub(super) fn apply_triggered_subskill_buffs(
    class_id: Option<&str>,
    skill_ranks: &HashMap<String, u32>,
    subskill_ranks: &HashMap<String, u32>,
    active_buffs: &HashMap<String, bool>,
    stat_sources: &mut SourceMap,
) {
    let Some(class_id) = class_id else {
        return;
    };
    for skill in data::get_skills_by_class(class_id) {
        if skill_ranks.get(&skill.id).copied().unwrap_or(0) == 0 {
            continue;
        }
        for node in skill.subskills.iter().flatten() {
            if node.triggered_buff.is_none() {
                continue;
            }
            let key = crate::calc::subskill::subskill_key(&skill.id, &node.id);
            let rank = subskill_ranks.get(&key).copied().unwrap_or(0);
            if rank == 0 || !active_buffs.get(&key).copied().unwrap_or(false) {
                continue;
            }
            let Some(effects) = node.effects.as_ref() else {
                continue;
            };
            let mut combined = effects.base.clone().unwrap_or_default();
            for (key, per_rank) in effects.per_rank.iter().flatten() {
                *combined.entry(key.clone()).or_default() += per_rank * f64::from(rank);
            }
            for (key, value) in combined {
                if value == 0.0 {
                    continue;
                }
                push_source(
                    stat_sources,
                    &key,
                    SourceContribution {
                        label: format!("{} / {} (rank {rank})", skill.name, node.name),
                        source_type: SourceType::Skill,
                        value: (value, value),
                        forge: None,
                    },
                );
            }
        }
    }
}

fn is_butcher_form_skill(id: &str) -> bool {
    matches!(
        id,
        "holy_form"
            | "unholy_form"
            | "insatiable_hunger"
            | "sacrilegious_scorn"
            | "spiritual_duality"
    )
}

fn apply_butcher_forms(
    skills: &[crate::calc::types::SkillSpec],
    ranks: &HashMap<String, u32>,
    buffs: &HashMap<String, bool>,
    all_skills: Ranged,
    item_bonuses: &HashMap<String, Ranged>,
    sources: &mut SourceMap,
) {
    let forms: HashMap<_, _> = skills
        .iter()
        .filter(|skill| is_butcher_form_skill(&skill.id))
        .map(|skill| {
            (
                skill.id.as_str(),
                (
                    skill,
                    passive_rank_range(
                        skill,
                        ranks.get(&skill.id).copied().unwrap_or(0),
                        all_skills,
                        item_bonuses,
                        sources,
                    ),
                ),
            )
        })
        .collect();
    let mut samples = vec![0.0_f64, 1.0];
    for (_, rank) in forms.values() {
        if rank.1 > rank.0 {
            for threshold in [0.0, 1.0] {
                let position = (threshold - rank.0) / (rank.1 - rank.0);
                if position > 0.0 && position < 1.0 {
                    samples.push(position);
                }
            }
        }
    }
    samples.sort_by(f64::total_cmp);
    samples.dedup();
    let coefficient = |id: &str, key: &str, position: f64| {
        let Some((skill, rank)) = forms.get(id) else {
            return 0.0;
        };
        let Some(passive) = skill.passive_stats.as_ref() else {
            return 0.0;
        };
        let mut effective_rank = rank.0 + (rank.1 - rank.0) * position;
        if rank.1 > rank.0 {
            for threshold in [0.0, 1.0] {
                if position == (threshold - rank.0) / (rank.1 - rank.0) {
                    effective_rank = threshold;
                }
            }
        }
        passive_coefficient_at_rank(passive, key, effective_rank)
    };
    let active = |id: &str| {
        ranks.get(id).copied().unwrap_or(0) > 0 && buffs.get(id).copied().unwrap_or(false)
    };
    let mut states = Vec::with_capacity(samples.len());
    let mut contributions: HashMap<(&str, &str), Vec<f64>> = HashMap::new();
    for (sample_index, &position) in samples.iter().enumerate() {
        let factor = 1.0 + coefficient("insatiable_hunger", "effectiveness", position) / 100.0;
        let hunger_damage = coefficient("unholy_form", "damage", position) * factor;
        let holy =
            active("holy_form") && coefficient("holy_form", "life_replenish_more", position) > 0.0;
        let unholy = active("unholy_form") && coefficient("unholy_form", "damage", position) > 0.0;
        let hunger = active("insatiable_hunger") && hunger_damage > 0.0;
        states.push((holy, unholy, hunger));
        let mut add = |id, key, value| {
            let entry = contributions
                .entry((id, key))
                .or_insert_with(|| vec![0.0; samples.len()]);
            entry[sample_index] += value;
        };
        if hunger {
            add("insatiable_hunger", "damage", hunger_damage);
            add(
                "insatiable_hunger",
                "life_replenish_more",
                coefficient("holy_form", "life_steal_more", position) * factor,
            );
            add(
                "insatiable_hunger",
                "life_steal_more",
                coefficient("holy_form", "damage", position) * factor,
            );
            add("insatiable_hunger", "life_replenish", 2.0);
        } else {
            for (id, keys) in [
                (
                    "holy_form",
                    ["damage", "life_replenish_more", "life_steal_more"],
                ),
                (
                    "unholy_form",
                    ["damage", "life_replenish_more", "life_steal_more"],
                ),
            ] {
                if active(id) {
                    for key in keys {
                        add(id, key, coefficient(id, key, position));
                    }
                }
            }
            if holy {
                add("holy_form", "life_replenish", 2.0);
            }
        }
        for (id, key, enabled) in [
            (
                "sacrilegious_scorn",
                "increased_attack_speed",
                holy || hunger,
            ),
            ("sacrilegious_scorn", "crit_damage", unholy || hunger),
            ("spiritual_duality", "life_steal", holy || hunger),
            ("spiritual_duality", "attack_damage", unholy || hunger),
        ] {
            if enabled {
                add(id, key, coefficient(id, key, position));
            }
        }
    }
    let push = |sources: &mut SourceMap, key: &str, label, value: Ranged| {
        push_source(
            sources,
            key,
            SourceContribution {
                label,
                source_type: SourceType::Skill,
                forge: None,
                value: (round3(value.0.min(value.1)), round3(value.0.max(value.1))),
            },
        );
    };
    let bounds = |values: &[f64]| {
        (
            values.iter().copied().fold(f64::INFINITY, f64::min),
            values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    };
    if states.iter().any(|state| *state != states[0]) {
        let mut totals: HashMap<&str, Vec<f64>> = HashMap::new();
        for ((_, key), values) in contributions {
            let total = totals
                .entry(key)
                .or_insert_with(|| vec![0.0; samples.len()]);
            for (sum, value) in total.iter_mut().zip(values) {
                *sum += value;
            }
        }
        for (key, value) in totals {
            push(
                sources,
                key,
                "Butcher forms and passives (rank range)".into(),
                bounds(&value),
            );
        }
    } else {
        for ((id, key), value) in contributions {
            if let Some((skill, rank)) = forms.get(id) {
                let rank_label = if rank.0 == rank.1 {
                    format!("{}", rank.0)
                } else {
                    format!("{}-{}", rank.0, rank.1)
                };
                push(
                    sources,
                    key,
                    format!("{} (rank {})", skill.name, rank_label),
                    bounds(&value),
                );
            }
        }
    }
}

fn passive_rank_range(
    skill: &crate::calc::types::SkillSpec,
    base_rank: u32,
    all_skills_bonus: Ranged,
    item_skill_bonuses: &HashMap<String, Ranged>,
    sources: &SourceMap,
) -> Ranged {
    if base_rank == 0 {
        return (0.0, 0.0);
    }
    let elem_bonus = skill
        .damage_type
        .as_deref()
        .map(|dt| sum_ranged_from_map(sources, &format!("{dt}_skills")))
        .unwrap_or((0.0, 0.0));
    let tag_bonus = crate::calc::affix_tags::keys_for(
        crate::calc::types::AffixEffect::Rank,
        skill.tags.as_deref().unwrap_or(&[]),
    )
    .iter()
    .fold((0.0, 0.0), |acc, key| {
        let value = sum_ranged_from_map(sources, key);
        (acc.0 + value.0, acc.1 + value.1)
    });
    let item_bonus = item_skill_bonuses
        .get(&normalize_skill_name(&skill.name))
        .copied()
        .unwrap_or((0.0, 0.0));
    (
        base_rank as f64 + all_skills_bonus.0 + elem_bonus.0 + tag_bonus.0 + item_bonus.0,
        base_rank as f64 + all_skills_bonus.1 + elem_bonus.1 + tag_bonus.1 + item_bonus.1,
    )
}

fn passive_coefficient_at_rank(
    passive: &crate::calc::types::PassiveStatsSpec,
    key: &str,
    rank: f64,
) -> f64 {
    if rank <= 0.0 {
        return 0.0;
    }
    let base = passive
        .base
        .as_ref()
        .and_then(|base| base.get(key))
        .copied()
        .unwrap_or(0.0);
    let slope = passive
        .per_rank
        .as_ref()
        .and_then(|values| values.get(key))
        .copied()
        .unwrap_or(0.0);
    base + slope * (rank - 1.0)
}

// Effectiveness scales the aura's contribution, not the character's complete
// damage. Mixed auras use the matching effectiveness for each output stat.
pub(super) fn apply_aura_effectiveness(values: &mut HashMap<String, Ranged>, sources: &SourceMap) {
    let damaging = sum_ranged_from_map(sources, "damaging_aura_effectiveness");
    let buffing = sum_ranged_from_map(sources, "buffing_aura_effectiveness");
    for (key, value) in values.iter_mut() {
        let damage_output = matches!(key.as_str(), "damage" | "attack_damage" | "enhanced_damage")
            || key.ends_with("_skill_damage")
            || (key.starts_with("additive_") && key.ends_with("_damage"));
        let effect = if damage_output { damaging } else { buffing };
        *value = (
            value.0 * (1.0 + effect.0 / 100.0).max(0.0),
            value.1 * (1.0 + effect.1 / 100.0).max(0.0),
        );
    }
}

#[inline]
pub(crate) fn round3(x: f64) -> f64 {
    ((x * 1000.0) + 0.5).floor() / 1000.0
}
