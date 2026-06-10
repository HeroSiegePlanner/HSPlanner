use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::data;
use super::rank::normalize_skill_name;
use super::skills::{
    AttackKind, AttackSkillDamageBreakdown, AttackSkillInput, AttackSkillScaling, BonusSource,
    DamageFormula, DamageRow, Ranged, Skill as CalcSkill, SkillDamageBreakdown, SkillInput, Weapon,
    compute_attack_skill_damage, compute_skill_damage,
};
use super::stats::{
    BuildStatsInput, ComputedStats, combine_additive_and_more, compute_build_stats,
    skill_spec_to_subskill_owner,
};
use super::subskill::{aggregate_subskill_stats, subskill_key};
use super::types::{CustomStat, Inventory, SkillKind, SkillSpec, TreeSocketContent};

#[derive(Debug, Clone, Copy)]
pub struct BuildPerformanceDeps<'a> {
    pub class_id: Option<&'a str>,
    pub level: u32,
    pub allocated_attrs: &'a HashMap<String, u32>,
    pub inventory: &'a Inventory,
    pub skill_ranks: &'a HashMap<String, u32>,
    pub subskill_ranks: &'a HashMap<String, u32>,
    pub active_aura_id: Option<&'a str>,
    pub active_buffs: &'a HashMap<String, bool>,
    pub custom_stats: &'a [CustomStat],
    pub allocated_tree_nodes: &'a HashSet<u32>,
    pub tree_socketed: &'a HashMap<u32, TreeSocketContent>,
    pub main_skill_id: Option<&'a str>,
    pub enemy_conditions: &'a HashMap<String, bool>,
    pub player_conditions: &'a HashMap<String, bool>,
    pub skill_projectiles: &'a HashMap<String, u32>,
    pub enemy_resistances: &'a HashMap<String, f64>,
    pub proc_toggles: &'a HashMap<String, bool>,
    pub kills_per_sec: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPerformance {
    pub attributes: HashMap<String, Ranged>,
    pub stats: HashMap<String, Ranged>,
    pub damage: Option<SkillDamageBreakdown>,
    pub attack_damage: Option<AttackSkillDamageBreakdown>,
    pub hit_dps_min: Option<f64>,
    pub hit_dps_max: Option<f64>,
    pub avg_hit_dps_min: Option<f64>,
    pub avg_hit_dps_max: Option<f64>,
    pub proc_dps_min: f64,
    pub proc_dps_max: f64,
    pub combined_dps_min: Option<f64>,
    pub combined_dps_max: Option<f64>,
    pub active_skill_name: Option<String>,
    pub stats_combined: HashMap<String, Ranged>,
    pub item_skill_bonuses: HashMap<String, Ranged>,
    pub rank_bonuses: HashMap<String, Ranged>,
}

fn skill_spec_to_calc_skill(spec: &SkillSpec) -> CalcSkill {
    let to_formula = |f: super::types::DamageFormulaSpec| DamageFormula {
        base: f.base,
        per_level: f.per_level,
    };
    CalcSkill {
        name: normalize_skill_name(&spec.name),
        tags: spec.tags.clone().unwrap_or_default(),
        damage_type: spec.damage_type.clone(),
        damage_formula: spec.damage_formula.map(to_formula),
        damage_per_rank: spec.damage_per_rank.as_ref().map(|rows| {
            rows.iter()
                .map(|r| DamageRow {
                    min: r.min,
                    max: r.max,
                })
                .collect()
        }),
        bonus_sources: spec
            .bonus_sources
            .as_ref()
            .map(|sources| {
                sources
                    .iter()
                    .filter_map(|b| match b.per.as_str() {
                        "attribute_point" => Some(BonusSource::AttributePoint {
                            source: normalize_skill_name(&b.source),
                            value: b.value,
                        }),
                        "skill_level" => Some(BonusSource::SkillLevel {
                            source: normalize_skill_name(&b.source),
                            value: b.value,
                        }),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        attack_kind: spec.attack_kind.map(|k| match k {
            super::types::AttackKindSpec::Attack => AttackKind::Attack,
            super::types::AttackKindSpec::Spell => AttackKind::Spell,
        }),
        attack_scaling: spec.attack_scaling.map(|s| AttackSkillScaling {
            weapon_damage_pct: s.weapon_damage_pct.map(to_formula),
            flat_physical_min: s.flat_physical_min.map(to_formula),
            flat_physical_max: s.flat_physical_max.map(to_formula),
            attack_rating_pct: s.attack_rating_pct.map(to_formula),
        }),
    }
}

struct ProcContext<'a> {
    computed: &'a ComputedStats,
    skill_ranks_by_name: &'a HashMap<String, f64>,
    item_skill_bonuses: &'a HashMap<String, Ranged>,
    enemy_conditions: &'a HashMap<String, bool>,
    enemy_resistances: &'a HashMap<String, f64>,
    skills_by_name: &'a HashMap<String, CalcSkill>,
    skill_projectiles: &'a HashMap<String, u32>,
    skill_ranks: &'a HashMap<String, u32>,
    all_class_skills: &'a [SkillSpec],
}

fn proc_target_damage(
    ctx: &ProcContext<'_>,
    target_name_norm: &str,
) -> Option<SkillDamageBreakdown> {
    let target_calc = ctx.skills_by_name.get(target_name_norm)?;
    let target_spec = ctx
        .all_class_skills
        .iter()
        .find(|s| normalize_skill_name(&s.name) == target_name_norm)?;
    let target_rank = ctx.skill_ranks.get(&target_spec.id).copied().unwrap_or(0);
    if target_rank == 0 {
        return None;
    }
    let input = SkillInput {
        skill: target_calc,
        allocated_rank: target_rank as f64,
        attributes: &ctx.computed.attributes,
        stats: &ctx.computed.stats,
        skill_ranks_by_name: ctx.skill_ranks_by_name,
        item_skill_bonuses: ctx.item_skill_bonuses,
        enemy_conditions: ctx.enemy_conditions,
        enemy_resistances: ctx.enemy_resistances,
        skills_by_name: ctx.skills_by_name,
        projectile_count: ctx
            .skill_projectiles
            .get(&target_spec.id)
            .copied()
            .unwrap_or(1),
    };
    compute_skill_damage(&input)
}

pub fn compute_build_performance(deps: &BuildPerformanceDeps<'_>) -> BuildPerformance {
    let stats_input = BuildStatsInput {
        class_id: deps.class_id,
        level: deps.level,
        allocated_attrs: deps.allocated_attrs,
        inventory: deps.inventory,
        skill_ranks: deps.skill_ranks,
        active_aura_id: deps.active_aura_id,
        active_buffs: deps.active_buffs,
        custom_stats: deps.custom_stats,
        allocated_tree_nodes: deps.allocated_tree_nodes,
        tree_socketed: deps.tree_socketed,
        player_conditions: deps.player_conditions,
        subskill_ranks: deps.subskill_ranks,
        enemy_conditions: deps.enemy_conditions,
    };
    let computed = compute_build_stats(&stats_input);

    let all_class_skills: &[SkillSpec] = match deps.class_id {
        Some(cid) => data::get_skills_by_class(cid),
        None => &[],
    };
    let active_skill: Option<&SkillSpec> = deps.main_skill_id.and_then(|mid| {
        all_class_skills
            .iter()
            .filter(|s| s.kind == SkillKind::Active)
            .find(|s| s.id == mid)
    });
    let active_rank = active_skill
        .and_then(|s| deps.skill_ranks.get(&s.id).copied())
        .unwrap_or(0);

    let item_skill_bonuses = &computed.item_skill_bonuses;
    let skill_ranks_by_name: HashMap<String, f64> = all_class_skills
        .iter()
        .map(|s| {
            (
                normalize_skill_name(&s.name),
                deps.skill_ranks.get(&s.id).copied().unwrap_or(0) as f64,
            )
        })
        .collect();
    let skills_by_name: HashMap<String, CalcSkill> = all_class_skills
        .iter()
        .map(|s| (normalize_skill_name(&s.name), skill_spec_to_calc_skill(s)))
        .collect();

    // Re-aggregate the active skill alone to pull projectile_count out — the
    // main stats path filters skill-scoped values.
    let active_projectile_boost: u32 = active_skill
        .map(|s| {
            let owner = skill_spec_to_subskill_owner(s);
            let agg =
                aggregate_subskill_stats(&owner, deps.subskill_ranks, Some(deps.enemy_conditions));
            agg.stats
                .get("projectile_count")
                .copied()
                .unwrap_or(0.0)
                .max(0.0) as u32
        })
        .unwrap_or(0);
    let effective_projectiles: Option<u32> = active_skill.map(|s| {
        let base = deps.skill_projectiles.get(&s.id).copied().unwrap_or(1);
        base + active_projectile_boost
    });

    let active_calc_skill: Option<&CalcSkill> = active_skill
        .and_then(|s| skills_by_name.get(&normalize_skill_name(&s.name)));
    let is_attack_skill = active_calc_skill
        .and_then(|s| s.attack_kind)
        == Some(AttackKind::Attack);

    let weapon_for_attack: Option<Weapon> = is_attack_skill
        .then(|| {
            deps.inventory
                .get("weapon")
                .and_then(|eq| data::get_item(&eq.base_id))
                .and_then(|base| match (base.damage_min, base.damage_max) {
                    (Some(min), Some(max)) => Some(Weapon {
                        name: base.name.clone(),
                        damage_min: min,
                        damage_max: max,
                    }),
                    _ => None,
                })
        })
        .flatten();

    let damage: Option<SkillDamageBreakdown> = match (active_calc_skill, active_rank > 0) {
        (Some(calc_skill), true) => {
            let input = SkillInput {
                skill: calc_skill,
                allocated_rank: active_rank as f64,
                attributes: &computed.attributes,
                stats: &computed.stats,
                skill_ranks_by_name: &skill_ranks_by_name,
                item_skill_bonuses,
                enemy_conditions: deps.enemy_conditions,
                enemy_resistances: deps.enemy_resistances,
                skills_by_name: &skills_by_name,
                projectile_count: effective_projectiles.unwrap_or(1),
            };
            compute_skill_damage(&input)
        }
        _ => None,
    };

    // Attack-kind skills reuse the elemental `damage` breakdown above and layer
    // weapon physical + attacks-per-second on top.
    let attack_damage: Option<AttackSkillDamageBreakdown> =
        match (active_calc_skill, active_rank > 0, is_attack_skill) {
            (Some(calc_skill), true, true) => {
                let input = AttackSkillInput {
                    skill: calc_skill,
                    allocated_rank: active_rank as f64,
                    stats: &computed.stats,
                    item_skill_bonuses,
                    enemy_conditions: deps.enemy_conditions,
                    weapon: weapon_for_attack.as_ref(),
                    poison_breakdown: damage.as_ref(),
                };
                compute_attack_skill_damage(&input)
            }
            _ => None,
        };

    let fcr = computed
        .stats
        .get("faster_cast_rate")
        .copied()
        .unwrap_or((0.0, 0.0));
    let fcr_more = computed
        .stats
        .get("faster_cast_rate_more")
        .copied()
        .unwrap_or((0.0, 0.0));
    let fcr_combined = combine_additive_and_more(fcr, fcr_more);
    let base_cast = active_skill.and_then(|s| s.base_cast_rate);
    let eff_cast_min = base_cast.map(|r| r * (1.0 + fcr_combined.0 / 100.0));
    let eff_cast_max = base_cast.map(|r| r * (1.0 + fcr_combined.1 / 100.0));

    let (hit_dps_min, hit_dps_max, avg_hit_dps_min, avg_hit_dps_max) = if let Some(ad) =
        attack_damage.as_ref()
    {
        (
            Some(ad.combined_hit_min as f64 * ad.attacks_per_second_min),
            Some(ad.combined_hit_max as f64 * ad.attacks_per_second_max),
            Some(ad.combined_avg_min as f64 * ad.attacks_per_second_min),
            Some(ad.combined_avg_max as f64 * ad.attacks_per_second_max),
        )
    } else {
        (
            damage
                .as_ref()
                .and_then(|d| eff_cast_min.map(|c| d.final_min as f64 * c)),
            damage
                .as_ref()
                .and_then(|d| eff_cast_max.map(|c| d.final_max as f64 * c)),
            damage
                .as_ref()
                .and_then(|d| eff_cast_min.map(|c| d.avg_min as f64 * c)),
            damage
                .as_ref()
                .and_then(|d| eff_cast_max.map(|c| d.avg_max as f64 * c)),
        )
    };

    let ctx = ProcContext {
        computed: &computed,
        skill_ranks_by_name: &skill_ranks_by_name,
        item_skill_bonuses,
        enemy_conditions: deps.enemy_conditions,
        enemy_resistances: deps.enemy_resistances,
        skills_by_name: &skills_by_name,
        skill_projectiles: deps.skill_projectiles,
        skill_ranks: deps.skill_ranks,
        all_class_skills,
    };
    let mut proc_dps_min: f64 = 0.0;
    let mut proc_dps_max: f64 = 0.0;
    for proc_skill in all_class_skills.iter() {
        let Some(proc) = proc_skill.proc.as_ref() else {
            continue;
        };
        if !deps
            .proc_toggles
            .get(&proc_skill.id)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }
        let proc_rank = deps.skill_ranks.get(&proc_skill.id).copied().unwrap_or(0);
        if proc_rank == 0 {
            continue;
        }
        let target_name = normalize_skill_name(&proc.target);
        let Some(target_dmg) = proc_target_damage(&ctx, &target_name) else {
            continue;
        };
        let rate = if proc.trigger == "on_kill" {
            deps.kills_per_sec
        } else {
            1.0
        };
        let factor = rate * (proc.chance / 100.0);
        proc_dps_min += factor * target_dmg.avg_min as f64;
        proc_dps_max += factor * target_dmg.avg_max as f64;
    }

    for owner_skill in all_class_skills.iter() {
        let Some(subskills) = owner_skill.subskills.as_ref() else {
            continue;
        };
        for sub in subskills.iter() {
            let Some(sub_proc) = sub.proc.as_ref() else {
                continue;
            };
            let Some(sub_target) = sub_proc.target.as_ref() else {
                continue;
            };
            let toggle_key = subskill_key(&owner_skill.id, &sub.id);
            if !deps
                .proc_toggles
                .get(&toggle_key)
                .copied()
                .unwrap_or(false)
            {
                continue;
            }
            let sub_rank = deps.subskill_ranks.get(&toggle_key).copied().unwrap_or(0);
            if sub_rank == 0 {
                continue;
            }
            let target_name = normalize_skill_name(sub_target);
            let Some(target_dmg) = proc_target_damage(&ctx, &target_name) else {
                continue;
            };
            let chance = sub_proc.chance.base.unwrap_or(0.0)
                + sub_proc.chance.per_rank.unwrap_or(0.0) * sub_rank as f64;
            let rate = if sub_proc.trigger == "on_kill" {
                deps.kills_per_sec
            } else {
                1.0
            };
            let factor = rate * (chance / 100.0);
            proc_dps_min += factor * target_dmg.avg_min as f64;
            proc_dps_max += factor * target_dmg.avg_max as f64;
        }
    }

    // Proc-only builds (no active skill) should still report combined DPS.
    let combined_dps_min = if avg_hit_dps_min.is_some() || proc_dps_min > 0.0 {
        Some(avg_hit_dps_min.unwrap_or(0.0) + proc_dps_min)
    } else {
        None
    };
    let combined_dps_max = if avg_hit_dps_max.is_some() || proc_dps_max > 0.0 {
        Some(avg_hit_dps_max.unwrap_or(0.0) + proc_dps_max)
    } else {
        None
    };

    BuildPerformance {
        attributes: computed.attributes,
        stats: computed.stats,
        damage,
        attack_damage,
        hit_dps_min,
        hit_dps_max,
        avg_hit_dps_min,
        avg_hit_dps_max,
        proc_dps_min,
        proc_dps_max,
        combined_dps_min,
        combined_dps_max,
        active_skill_name: active_skill.map(|s| s.name.clone()),
        stats_combined: computed.stats_combined,
        item_skill_bonuses: computed.item_skill_bonuses,
        rank_bonuses: computed.rank_bonuses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    fn empty_deps<'a>(
        allocated: &'a HashMap<String, u32>,
        inventory: &'a Inventory,
        skill_ranks: &'a HashMap<String, u32>,
        subskill_ranks: &'a HashMap<String, u32>,
        active_buffs: &'a HashMap<String, bool>,
        custom_stats: &'a [CustomStat],
        alloc_tree: &'a HashSet<u32>,
        tree_socketed: &'a HashMap<u32, TreeSocketContent>,
        enemy_conditions: &'a HashMap<String, bool>,
        player_conditions: &'a HashMap<String, bool>,
        skill_projectiles: &'a HashMap<String, u32>,
        enemy_resistances: &'a HashMap<String, f64>,
        proc_toggles: &'a HashMap<String, bool>,
    ) -> BuildPerformanceDeps<'a> {
        BuildPerformanceDeps {
            class_id: None,
            level: 1,
            allocated_attrs: allocated,
            inventory,
            skill_ranks,
            subskill_ranks,
            active_aura_id: None,
            active_buffs,
            custom_stats,
            allocated_tree_nodes: alloc_tree,
            tree_socketed,
            main_skill_id: None,
            enemy_conditions,
            player_conditions,
            skill_projectiles,
            enemy_resistances,
            proc_toggles,
            kills_per_sec: 0.0,
        }
    }

    #[test]
    fn empty_build_produces_no_damage_no_proc() {
        let allocated = HashMap::new();
        let inventory = HashMap::new();
        let skill_ranks = HashMap::new();
        let subskill_ranks = HashMap::new();
        let active_buffs = HashMap::new();
        let custom_stats: Vec<CustomStat> = Vec::new();
        let alloc_tree = HashSet::new();
        let tree_socketed = HashMap::new();
        let enemy_conditions = HashMap::new();
        let player_conditions = HashMap::new();
        let skill_projectiles = HashMap::new();
        let enemy_resistances = HashMap::new();
        let proc_toggles = HashMap::new();
        let deps = empty_deps(
            &allocated,
            &inventory,
            &skill_ranks,
            &subskill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &enemy_conditions,
            &player_conditions,
            &skill_projectiles,
            &enemy_resistances,
            &proc_toggles,
        );
        let perf = compute_build_performance(&deps);
        assert!(perf.damage.is_none());
        assert_eq!(perf.proc_dps_min, 0.0);
        assert_eq!(perf.proc_dps_max, 0.0);
        assert_eq!(perf.hit_dps_min, None);
        assert_eq!(perf.combined_dps_min, None);
        assert_eq!(perf.active_skill_name, None);
        assert!(!perf.stats.is_empty(), "default base stats should populate");
    }

    #[test]
    fn class_with_active_skill_produces_damage() {
        let pick = data::data().skills_by_class.iter().find_map(|(cid, skills)| {
            skills.iter().find_map(|s| {
                if s.kind != SkillKind::Active {
                    return None;
                }
                if s.damage_formula.is_none() && s.damage_per_rank.is_none() {
                    return None;
                }
                Some((cid.clone(), s.id.clone()))
            })
        });
        let Some((class_id, skill_id)) = pick else {
            eprintln!("no active skill with damage formula/table; skipping");
            return;
        };

        let allocated = HashMap::new();
        let inventory = HashMap::new();
        let mut skill_ranks = HashMap::new();
        skill_ranks.insert(skill_id.clone(), 10_u32);
        let subskill_ranks = HashMap::new();
        let active_buffs = HashMap::new();
        let custom_stats: Vec<CustomStat> = Vec::new();
        let alloc_tree = HashSet::new();
        let tree_socketed = HashMap::new();
        let enemy_conditions = HashMap::new();
        let player_conditions = HashMap::new();
        let skill_projectiles = HashMap::new();
        let enemy_resistances = HashMap::new();
        let proc_toggles = HashMap::new();

        let mut deps = empty_deps(
            &allocated,
            &inventory,
            &skill_ranks,
            &subskill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &enemy_conditions,
            &player_conditions,
            &skill_projectiles,
            &enemy_resistances,
            &proc_toggles,
        );
        deps.class_id = Some(&class_id);
        deps.level = 50;
        deps.main_skill_id = Some(&skill_id);

        let perf = compute_build_performance(&deps);
        assert!(
            perf.damage.is_some(),
            "expected damage breakdown for active skill '{skill_id}'"
        );
        assert!(perf.active_skill_name.is_some());
    }

    #[test]
    fn active_skill_without_rank_yields_no_damage() {
        let pick = data::data().skills_by_class.iter().find_map(|(cid, skills)| {
            skills.iter().find_map(|s| {
                if s.kind != SkillKind::Active {
                    return None;
                }
                if s.damage_formula.is_none() && s.damage_per_rank.is_none() {
                    return None;
                }
                Some((cid.clone(), s.id.clone()))
            })
        });
        let Some((class_id, skill_id)) = pick else {
            eprintln!("no active skill; skipping");
            return;
        };

        let allocated = HashMap::new();
        let inventory = HashMap::new();
        let skill_ranks: HashMap<String, u32> = HashMap::new();
        let subskill_ranks = HashMap::new();
        let active_buffs = HashMap::new();
        let custom_stats: Vec<CustomStat> = Vec::new();
        let alloc_tree = HashSet::new();
        let tree_socketed = HashMap::new();
        let enemy_conditions = HashMap::new();
        let player_conditions = HashMap::new();
        let skill_projectiles = HashMap::new();
        let enemy_resistances = HashMap::new();
        let proc_toggles = HashMap::new();

        let mut deps = empty_deps(
            &allocated,
            &inventory,
            &skill_ranks,
            &subskill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &enemy_conditions,
            &player_conditions,
            &skill_projectiles,
            &enemy_resistances,
            &proc_toggles,
        );
        deps.class_id = Some(&class_id);
        deps.main_skill_id = Some(&skill_id);

        let perf = compute_build_performance(&deps);
        assert!(perf.damage.is_none());
        assert!(perf.hit_dps_min.is_none());
    }
}
