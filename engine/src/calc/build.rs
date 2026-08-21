use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::data;
use super::rank::normalize_skill_name;
use super::skills::{
    AttackKind, AttackSkillDamageBreakdown, AttackSkillInput, AttackSkillScaling, BonusSource,
    DamageFormula, DamageRow, Ranged, Skill as CalcSkill, SkillDamageBreakdown, SkillInput, StatMap,
    Weapon, ailment, compute_attack_skill_damage, compute_skill_damage, conversion, r_max, r_min,
    rg,
};
use super::stats::{BuildStatsInput, ComputedStats, combine_additive_and_more, compute_build_stats};
use super::subskill::subskill_key;
use super::types::{CustomStat, Inventory, SkillKind, SkillSpec, TreeSocketContent};

/// A build can never execute the whole life bar; 90% keeps the multiplier finite.
const EXECUTE_MAX_PCT: f64 = 90.0;

/// Config default when a build predates the per-kind rate knobs.
pub const DEFAULT_ENTITY_RATE: f64 = 1.0;

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
    /// Attacks/casts per second of the entity itself, keyed by lowercase entity
    /// tag ("sentry", "summon", "guardian"). The game never exposes the base
    /// values, so they are Config knobs - one per kind, they are not the same
    /// thing.
    pub entity_rates: &'a HashMap<String, f64>,
    pub granted_skill_ranks: Option<&'a HashMap<String, Ranged>>,
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
    pub ailment_dps_min: Option<f64>,
    pub ailment_dps_max: Option<f64>,
    pub combined_dps_min: Option<f64>,
    pub combined_dps_max: Option<f64>,
    /// Factor already folded into `combined_dps_*`. Exported so a consumer that
    /// re-sums the DPS parts can reapply it instead of dropping execute.
    pub execute_mult: f64,
    pub active_skill_name: Option<String>,
    pub stats_combined: HashMap<String, Ranged>,
    pub diminished_raw: HashMap<String, Ranged>,
    pub item_skill_bonuses: HashMap<String, Ranged>,
    pub rank_bonuses: HashMap<String, Ranged>,
    /// How many entities the skill fields (1 + Maximum Sentry/Summon/Guardian
    /// Amount). Already folded into the DPS; exported so views can show it.
    pub entity_count: Option<Ranged>,
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
                            stat: b.stat.clone(),
                            value: b.value,
                        }),
                        "skill_level" => Some(BonusSource::SkillLevel {
                            source: normalize_skill_name(&b.source),
                            stat: b.stat.clone(),
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

/// Per-state chance (%) that the main skill's subtree inflicts a state; nodes that
/// apply a state without an amount count too. `ailment::AILMENTS` filters to damage.
fn subtree_apply_chances(
    spec: Option<&SkillSpec>,
    subskill_ranks: &HashMap<String, u32>,
    enemy_conditions: &HashMap<String, bool>,
) -> HashMap<String, f64> {
    let mut out: HashMap<String, f64> = HashMap::new();
    let Some(spec) = spec else {
        return out;
    };
    let owner = super::stats::skill_spec_to_subskill_owner(spec);
    let agg =
        super::subskill::aggregate_subskill_stats(&owner, subskill_ranks, Some(enemy_conditions));
    for state in agg.applied_states {
        *out.entry(state.state).or_insert(0.0) += state.chance;
    }
    out
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
    empty_scoped: &'a StatMap,
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
        // Proc targets are not the active skill, so no subtree of their own.
        of_total_damage: 0.0,
        scoped: ctx.empty_scoped,
        conversion_flat: 0.0,
        conversion_skill_damage_pct: 0.0,
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
        granted_skill_ranks: deps.granted_skill_ranks,
        main_skill_id: deps.main_skill_id,
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

    // Skill-scoped subtree values never enter the shared stat map; the stats
    // pass parks them here, already scoped to the main skill.
    let empty_scoped: StatMap = StatMap::new();
    let main_scoped: &StatMap = deps
        .main_skill_id
        .and_then(|id| computed.skill_scoped.get(id))
        .unwrap_or(&empty_scoped);
    let subtree_stat = |key: &str| -> f64 { r_max(rg(main_scoped, key)).max(0.0) };
    let active_projectile_boost: u32 = subtree_stat("projectile_count") as u32;
    let active_of_total_damage: f64 = subtree_stat("of_total_damage");
    let effective_projectiles: Option<u32> = active_skill.map(|s| {
        let base = deps.skill_projectiles.get(&s.id).copied().unwrap_or(1);
        base + active_projectile_boost
    });

    let active_calc_skill: Option<&CalcSkill> = active_skill
        .and_then(|s| skills_by_name.get(&normalize_skill_name(&s.name)));
    // Subskill transmutations rewrite the main skill's tags (e.g. Ancient
    // Device turns Death from Above into a Sentry).
    let effective_skill: Option<CalcSkill> = match (active_skill, active_calc_skill) {
        (Some(spec), Some(calc_skill)) => {
            let mut skill = calc_skill.clone();
            skill.tags = super::subskill::effective_skill_tags(
                &spec.id,
                &skill.tags,
                deps.subskill_ranks,
            );
            Some(skill)
        }
        _ => None,
    };
    let active_calc_skill: Option<&CalcSkill> = effective_skill.as_ref();
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

    let conversions = conversion::resolve(
        main_scoped,
        &computed.attributes,
        &computed.stats,
        active_calc_skill.map(|s| s.tags.as_slice()).unwrap_or(&[]),
    );

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
                of_total_damage: active_of_total_damage,
                scoped: main_scoped,
                conversion_flat: conversions.flat,
                conversion_skill_damage_pct: conversions.skill_damage_pct,
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
                    attributes: &computed.attributes,
                    stats: &computed.stats,
                    skill_ranks_by_name: &skill_ranks_by_name,
                    skills_by_name: &skills_by_name,
                    item_skill_bonuses,
                    enemy_conditions: deps.enemy_conditions,
                    weapon: weapon_for_attack.as_ref(),
                    poison_breakdown: damage.as_ref(),
                    scoped: main_scoped,
                    projectile_count: effective_projectiles.unwrap_or(1),
                    // The elemental breakdown already consumed the conversion;
                    // adding it to the physical swing too would count it twice.
                    conversion_flat: if damage.is_some() {
                        0.0
                    } else {
                        conversions.flat
                    },
                };
                compute_attack_skill_damage(&input)
            }
            _ => None,
        };

    let stat = |key: &str| computed.stats.get(key).copied().unwrap_or((0.0, 0.0));
    let entity_tags: &[String] = active_calc_skill.map(|s| s.tags.as_slice()).unwrap_or(&[]);
    let entity_kind = super::affix_tags::entity_tag_for(entity_tags);
    let is_entity = entity_kind.is_some();
    // Explosive Kunai is thrown at weapon attack speed; FCR never touches it.
    let (base_rate, rate_bonus) = if let Some(kind) = entity_kind {
        // The entity swings on its own cadence: config base rate scaled by its
        // own attack-speed stats. Player FCR / attack speed stay out of it.
        let kind_key = kind.to_lowercase();
        // A subskill can pin the entity to a literal rate (C.Y.C.L.O.P.S. lasers
        // tick 4/s); pinned means pinned, so knob and speed bonuses drop out.
        let fixed = r_max(stat(&format!("{kind_key}_attack_rate_fixed")));
        if fixed > 0.0 {
            (Some(fixed), (0.0, 0.0))
        } else {
            let bonus = super::affix_tags::sum_for(
                super::types::AffixEffect::AttackSpeed,
                entity_tags,
                &computed.stats,
            );
            let rate = deps
                .entity_rates
                .get(&kind_key)
                .copied()
                .unwrap_or(DEFAULT_ENTITY_RATE);
            (Some(rate), bonus)
        }
    } else if active_skill.is_some_and(|s| s.uses_attack_speed) {
        (
            Some(r_max(stat("attacks_per_second"))),
            combine_additive_and_more(
                stat("increased_attack_speed"),
                stat("increased_attack_speed_more"),
            ),
        )
    } else {
        (
            active_skill.and_then(|s| s.base_cast_rate),
            combine_additive_and_more(stat("faster_cast_rate"), stat("faster_cast_rate_more")),
        )
    };
    let eff_cast_min = base_rate.map(|r| r * (1.0 + rate_bonus.0 / 100.0));
    let eff_cast_max = base_rate.map(|r| r * (1.0 + rate_bonus.1 / 100.0));

    // Sentries, summons and guardians are counted apart: each entity the skill
    // fields is a full extra DPS source.
    let (count_min, count_max) = if is_entity {
        let extra = super::affix_tags::sum_for(
            super::types::AffixEffect::MaxAmount,
            entity_tags,
            &computed.stats,
        );
        (1.0 + extra.0.max(0.0), 1.0 + extra.1.max(0.0))
    } else {
        (1.0, 1.0)
    };

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
    let hit_dps_min = hit_dps_min.map(|v| v * count_min);
    let hit_dps_max = hit_dps_max.map(|v| v * count_max);
    let avg_hit_dps_min = avg_hit_dps_min.map(|v| v * count_min);
    let avg_hit_dps_max = avg_hit_dps_max.map(|v| v * count_max);

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
        empty_scoped: &empty_scoped,
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

    // Item procs fire on this ICD even when their listed cooldown is shorter.
    const ITEM_PROC_ICD_SECS: f64 = 1.5;
    {
        // ponytail: no ignore_res / crit parity with class skills — add when a proc build cares.
        let granted_ranks = &computed.item_granted_ranks;
        for granted in data::item_granted_skills().iter() {
            let Some(proc_damage) = granted.proc_damage.as_ref() else {
                continue;
            };
            let toggle_key = format!("granted:{}", granted.id);
            if !deps.proc_toggles.get(&toggle_key).copied().unwrap_or(false) {
                continue;
            }
            let key = normalize_skill_name(&granted.name);
            let Some(&(rank_min, rank_max)) = granted_ranks.get(&key) else {
                continue;
            };
            if rank_max <= 0.0 {
                continue;
            }
            let interval = granted
                .proc_cooldown
                .unwrap_or(ITEM_PROC_ICD_SECS)
                .max(ITEM_PROC_ICD_SECS);
            for p in proc_damage.iter() {
                let add = rg(&computed.stats, &format!("{}_skill_damage", p.damage_type));
                let more = rg(
                    &computed.stats,
                    &format!("{}_skill_damage_more", p.damage_type),
                );
                let res = deps
                    .enemy_resistances
                    .get(p.damage_type.as_str())
                    .copied()
                    .unwrap_or(0.0);
                let res_mult = 1.0 - res / 100.0;
                let dmg_min = (p.base + p.per_rank * rank_min)
                    * (1.0 + r_min(add) / 100.0)
                    * (1.0 + r_min(more) / 100.0)
                    * res_mult;
                let dmg_max = (p.base + p.per_rank * rank_max)
                    * (1.0 + r_max(add) / 100.0)
                    * (1.0 + r_max(more) / 100.0)
                    * res_mult;
                proc_dps_min += dmg_min / interval;
                proc_dps_max += dmg_max / interval;
            }
        }
    }

    let (hit_avg_min, hit_avg_max) = match (attack_damage.as_ref(), damage.as_ref()) {
        (Some(ad), _) => (ad.combined_avg_min as f64, ad.combined_avg_max as f64),
        (None, Some(d)) => (d.avg_min as f64, d.avg_max as f64),
        _ => (0.0, 0.0),
    };
    let apply_chances = subtree_apply_chances(active_skill, deps.subskill_ranks, deps.enemy_conditions);
    // A per-hit apply chance only becomes uptime once you know how often the
    // build lands a hit: every entity swinging on its own counts.
    let (rate_min, rate_max) = match attack_damage.as_ref() {
        Some(ad) => (ad.attacks_per_second_min, ad.attacks_per_second_max),
        None => (eff_cast_min.unwrap_or(0.0), eff_cast_max.unwrap_or(0.0)),
    };
    let ailment_min = ailment::ailment_dps(
        hit_avg_min,
        rate_min * count_min,
        &computed.stats,
        main_scoped,
        &apply_chances,
    );
    let ailment_max = ailment::ailment_dps(
        hit_avg_max,
        rate_max * count_max,
        &computed.stats,
        main_scoped,
        &apply_chances,
    );
    let ailment_dps_min = (ailment_min > 0.0).then_some(ailment_min);
    let ailment_dps_max = (ailment_max > 0.0).then_some(ailment_max);

    // Execution shortens the kill by the bottom `t%` of the life bar, so the
    // effective DPS rises by 1/(1 - t). Bosses cannot be executed.
    let execute_below = subtree_stat("execute_below").clamp(0.0, EXECUTE_MAX_PCT);
    let is_boss = deps.enemy_conditions.get("is_boss").copied().unwrap_or(false);
    let execute_mult = if is_boss || execute_below == 0.0 {
        1.0
    } else {
        1.0 / (1.0 - execute_below / 100.0)
    };

    // Proc-only builds (no active skill) should still report combined DPS.
    let combined_dps_min = if avg_hit_dps_min.is_some() || proc_dps_min > 0.0 || ailment_min > 0.0 {
        Some((avg_hit_dps_min.unwrap_or(0.0) + proc_dps_min + ailment_min) * execute_mult)
    } else {
        None
    };
    let combined_dps_max = if avg_hit_dps_max.is_some() || proc_dps_max > 0.0 || ailment_max > 0.0 {
        Some((avg_hit_dps_max.unwrap_or(0.0) + proc_dps_max + ailment_max) * execute_mult)
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
        ailment_dps_min,
        ailment_dps_max,
        combined_dps_min,
        combined_dps_max,
        execute_mult,
        active_skill_name: active_skill.map(|s| s.name.clone()),
        stats_combined: computed.stats_combined,
        diminished_raw: computed.diminished_raw,
        item_skill_bonuses: computed.item_skill_bonuses,
        rank_bonuses: computed.rank_bonuses,
        entity_count: is_entity.then_some((count_min, count_max)),
    }
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
