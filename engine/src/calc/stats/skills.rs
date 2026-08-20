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
        // Tag-scoped skill bonus ("+X to Projectile/Sentry/... Skills"), paired
        // with its tag in data/affix-tags.json.
        let tag_bonus = crate::calc::affix_tags::keys_for(
            crate::calc::types::AffixEffect::Rank,
            skill.tags.as_deref().unwrap_or(&[]),
        )
        .iter()
        .fold((0.0, 0.0), |acc, key| {
            let v = sum_ranged_from_map(stat_sources, key);
            (acc.0 + v.0, acc.1 + v.1)
        });
        let key_norm = normalize_skill_name(&skill.name);
        let item_bonus = item_skill_bonuses
            .get(&key_norm)
            .copied()
            .unwrap_or((0.0, 0.0));

        let eff_min = (base_rank as f64
            + all_skills_bonus.0
            + elem_bonus.0
            + tag_bonus.0
            + item_bonus.0)
            .max(1.0);
        let eff_max = (base_rank as f64
            + all_skills_bonus.1
            + elem_bonus.1
            + tag_bonus.1
            + item_bonus.1)
            .max(1.0);

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

        // Buffing Aura Effectiveness scales the active aura's buff output.
        if skill.kind == SkillKind::Aura {
            let buff_eff = sum_ranged_from_map(stat_sources, "buffing_aura_effectiveness");
            if buff_eff.0 != 0.0 || buff_eff.1 != 0.0 {
                let mult_min = 1.0 + buff_eff.0 / 100.0;
                let mult_max = 1.0 + buff_eff.1 / 100.0;
                for value in combined.values_mut() {
                    *value = (value.0 * mult_min, value.1 * mult_max);
                }
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
pub(crate) fn round3(x: f64) -> f64 {
    ((x * 1000.0) + 0.5).floor() / 1000.0
}

