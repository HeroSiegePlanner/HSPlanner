use super::skills::calculation::{
    number, range, scalar, scoped_inputs, stat_inputs, CalculationStep,
};
use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::data;
use super::defense::{DefenseInsight, EhpResult};
use super::rank::normalize_skill_name;
use super::skill_cost::SkillCost;
use super::skills::{
    ailment, compute_attack_skill_damage, compute_skill_damage, conversion, r_max, r_min, rg,
    AttackKind, AttackSkillDamageBreakdown, AttackSkillInput, AttackSkillScaling, BonusSource,
    DamageFormula, DamageRow, Ranged, Skill as CalcSkill, SkillDamageBreakdown, SkillInput,
    StatMap, Weapon,
};
use super::stats::{
    combine_additive_and_more, compute_build_stats, BuildStatsInput, ComputedStats,
};
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
    pub stack_counts: &'a HashMap<String, u32>,
    pub granted_skill_ranks: Option<&'a HashMap<String, Ranged>>,
    pub difficulty: Option<&'a str>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPerformance {
    #[serde(skip)]
    pub(crate) calculation: Vec<CalculationStep>,
    #[serde(skip)]
    pub(crate) calculation_sources: super::stats::SourceMap,
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
    /// How many times one cast hits the same target. Already folded into the DPS;
    /// exported so views can show it. `None` when the skill hits once.
    pub hits_per_cast: Option<Ranged>,
    pub ehp: EhpResult,
    pub defense_insights: Vec<DefenseInsight>,
    pub skill_costs: HashMap<String, SkillCost>,
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
        damage_scaling: spec.damage_scaling,
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
            weapon_bonus_scaling: s.weapon_bonus_scaling,
            weapon_damage_pct: s.weapon_damage_pct.map(to_formula),
            flat_physical_min: s.flat_physical_min.map(to_formula),
            flat_physical_max: s.flat_physical_max.map(to_formula),
            attack_rating_pct: s.attack_rating_pct.map(to_formula),
        }),
    }
}

/// Hits one target takes from a volley. A fan or arc lands only a couple of
/// its projectiles on a single enemy (`single_target_hit_cap`), and a wave
/// proc repeats the whole volley (`extra_volleys_pct`, already chance-weighted).
fn effective_projectile_count(base: u32, subtree_stat: &dyn Fn(&str) -> f64) -> u32 {
    let transformed_count = subtree_stat("primary_projectile_count") as u32;
    let base = if transformed_count > 0 { transformed_count } else { base };
    let boosted = base + subtree_stat("projectile_count") as u32;
    let cap = subtree_stat("single_target_hit_cap") as u32;
    let capped = if cap > 0 { boosted.min(cap) } else { boosted };
    // ponytail: whole hits; go f64 if a 1-projectile skill ever takes a wave node
    (capped as f64 * (1.0 + subtree_stat("extra_volleys_pct") / 100.0)).round() as u32
}

/// Hits one target takes per cast. The damage object clears its hit list every
/// `tickFrequency`, so it lands `floor(lifetime / tick) + 1` hits on a target that
/// stays in range. Objects the game kills by animation or alarm have no extracted
/// lifetime and count as a single hit.
fn hits_per_cast(spec: Option<&SkillSpec>, duration_bonus: Ranged) -> Ranged {
    let Some(model) = spec.and_then(|s| s.hit_model.as_ref()) else {
        return (1.0, 1.0);
    };
    let (Some(tick), Some(lifetime)) = (model.tick_frequency, model.lifetime) else {
        return (1.0, 1.0);
    };
    if tick <= 0.0 || lifetime <= 0.0 {
        return (1.0, 1.0);
    }
    // An object always lands at least one hit, even below -100% duration.
    let hits = |bonus: f64| ((lifetime * (1.0 + bonus / 100.0) / tick).floor() + 1.0).max(1.0);
    (hits(duration_bonus.0), hits(duration_bonus.1))
}

/// Attack entities use their own action rate. A declared cooldown gates player
/// attacks just as it gates spells; weapon attack speed does not shorten it.
fn attack_action_rate_override(
    spec: &SkillSpec,
    tags: &[String],
    stats: &StatMap,
    scoped: &StatMap,
    entity_rates: &HashMap<String, f64>,
) -> Option<Ranged> {
    if let Some(kind) = super::affix_tags::entity_tag_for(tags) {
        let rate = super::skill_cost::entity_rate(kind, tags, stats, entity_rates);
        return Some((rate.min.max(0.0), rate.max.max(0.0)));
    }
    let cooldown = super::skill_cost::effective_cooldown(spec, scoped)?;
    let haste = rg(stats, "skill_haste");
    let gate = super::skill_cost::cooldown_rate(spec, cooldown, haste);
    if spec.id == "blender" {
        return Some(gate);
    }
    let base = rg(stats, "attacks_per_second");
    let speed = combine_additive_and_more(
        rg(stats, "increased_attack_speed"),
        rg(stats, "increased_attack_speed_more"),
    );
    Some((
        gate.0
            .min(base.0.max(0.0) * (1.0 + speed.0 / 100.0).max(0.0)),
        gate.1
            .min(base.1.max(0.0) * (1.0 + speed.1 / 100.0).max(0.0)),
    ))
}

/// Trigger cadence is independent of the damage dealt by the triggered skill.
/// Incoming-hit/block events have no supplied rate yet and retain their legacy
/// one-event/s estimate; outgoing events use the modeled build's own cadence.
fn proc_trigger_rate(trigger: &str, actions: Ranged, contacts: Ranged, kills: f64) -> Ranged {
    match trigger {
        "on_hit" => contacts,
        "on_cast" | "on_attack" => actions,
        "on_kill" => (kills.max(0.0), kills.max(0.0)),
        _ => (1.0, 1.0),
    }
}

fn proc_trigger_assumption(trigger: &str) -> &'static str {
    match trigger {
        "on_hit" | "on_cast" | "on_attack" | "on_kill" => "",
        _ => " (estimate: incoming event rate is not configured)",
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
    deps: BuildPerformanceDeps<'a>,
    skill_ranks_by_name: &'a HashMap<String, f64>,
    skills_by_name: &'a HashMap<String, CalcSkill>,
    all_class_skills: &'a [SkillSpec],
    empty_scoped: &'a StatMap,
}

/// Item proc rows shown in the Config view.
pub fn item_cast_toggle_key(base_id: &str, target_name_norm: &str) -> String {
    format!("cast:{base_id}:{target_name_norm}")
}

fn proc_target_damage(
    ctx: &ProcContext<'_>,
    target_name_norm: &str,
    rank_override: Option<f64>,
) -> Option<SkillDamageBreakdown> {
    let target_spec = ctx
        .all_class_skills
        .iter()
        .find(|s| normalize_skill_name(&s.name) == target_name_norm)
        .or_else(|| {
            // An explicit item cast can invoke a skill from another class.
            // Class and subtree procs still require a skill in their own class.
            rank_override?;
            data::data()
                .skills_by_class
                .values()
                .flatten()
                .find(|s| normalize_skill_name(&s.name) == target_name_norm)
        })?;
    let foreign = ctx.deps.class_id != Some(target_spec.class_id.as_str());
    let rank = match rank_override {
        Some(r) => r,
        None => match ctx
            .deps
            .skill_ranks
            .get(&target_spec.id)
            .copied()
            .unwrap_or(0)
        {
            0 => return None,
            r => r as f64,
        },
    };
    // Rebuild the target's context from the original inputs. Subtracting a
    // finalized main-skill bonus cannot reverse attribute scaling, diminishing
    // returns or other derived stats, and would leak them into the target.
    let target_computed = (foreign || ctx.deps.main_skill_id != Some(target_spec.id.as_str()))
        .then(|| compute_stats_for_skill(&ctx.deps, (!foreign).then_some(target_spec.id.as_str())));
    let computed = target_computed.as_ref().unwrap_or(ctx.computed);
    let scoped = if foreign {
        ctx.empty_scoped
    } else {
        computed
            .skill_scoped
            .get(&target_spec.id)
            .unwrap_or(ctx.empty_scoped)
    };
    let subtree_stat = |key: &str| -> f64 { r_max(rg(scoped, key)).max(0.0) };
    let mut target_calc = skill_spec_to_calc_skill(target_spec);
    if !foreign {
        target_calc.tags = super::subskill::effective_skill_tags(
            &target_spec.id,
            &target_calc.tags,
            ctx.deps.subskill_ranks,
        );
    }
    let (conversions, conversion_steps) = conversion::resolve_with_calculation(
        scoped,
        &computed.attributes,
        &computed.stats,
        &target_calc.tags,
    );
    let input = SkillInput {
        skill: &target_calc,
        allocated_rank: rank,
        attributes: &computed.attributes,
        stats: &computed.stats,
        skill_ranks_by_name: ctx.skill_ranks_by_name,
        item_skill_bonuses: &computed.item_skill_bonuses,
        enemy_conditions: ctx.deps.enemy_conditions,
        enemy_resistances: ctx.deps.enemy_resistances,
        skills_by_name: ctx.skills_by_name,
        projectile_count: effective_projectile_count(
            ctx.deps
                .skill_projectiles
                .get(&target_spec.id)
                .copied()
                .unwrap_or_else(|| target_spec.base_projectiles.unwrap_or(1)),
            &subtree_stat,
        ),
        of_total_damage: r_max(rg(scoped, "of_total_damage")),
        scoped,
        conversion_flat: conversions.flat,
        conversion_skill_damage_pct: conversions.skill_damage_pct,
    };
    let mut damage = match rank_override {
        Some(rank) => super::skills::damage::compute_skill_damage_at_rank(&input, rank),
        None => compute_skill_damage(&input),
    }?;
    let mut context_steps = Vec::new();
    scoped_inputs(&mut context_steps, scoped);
    context_steps.extend(conversion_steps);
    damage.calculation.splice(0..0, context_steps);
    Some(damage)
}

fn compute_stats_for_skill(
    deps: &BuildPerformanceDeps<'_>,
    main_skill_id: Option<&str>,
) -> ComputedStats {
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
        stack_counts: deps.stack_counts,
        granted_skill_ranks: deps.granted_skill_ranks,
        main_skill_id,
        difficulty: deps.difficulty,
        entity_rates: deps.entity_rates,
    };
    compute_build_stats(&stats_input)
}

pub fn compute_build_performance(deps: &BuildPerformanceDeps<'_>) -> BuildPerformance {
    performance_from_stats(deps, compute_stats_for_skill(deps, deps.main_skill_id))
}

/// Native callers retain the primary stat result for its source breakdown.
/// The legacy entry point follows this same calculation path.
pub(crate) fn performance_from_stats(
    deps: &BuildPerformanceDeps<'_>,
    computed: ComputedStats,
) -> BuildPerformance {
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
    let active_of_total_damage = r_max(rg(main_scoped, "of_total_damage"));
    let mut calculation = Vec::new();
    scoped_inputs(&mut calculation, main_scoped);
    // A verified secondary coefficient is not an extra primary hit. Until the
    // collision/lifetime audit supplies its contact model, expose its exclusion
    // instead of silently treating its percentage as primary increased damage.
    if let Some(skill) = active_skill.filter(|_| active_rank > 0) {
        let pending: &[&str] = match skill.id.as_str() {
            "fireball" => &["mark_of_doom", "splitfire", "path_of_destruction", "fireflies"],
            "storm_bolt" => &["unleash_discharge", "storm_breeds_storm", "bouncing_charge", "storm_claw", "magnetize"],
            _ => &[],
        };
        for node in skill.subskills.as_deref().unwrap_or(&[]) {
            if pending.contains(&node.id.as_str())
                && deps.subskill_ranks.get(&subskill_key(&skill.id, &node.id)).copied().unwrap_or(0) > 0
            {
                calculation.push(CalculationStep::new(
                    format!("Secondary DPS not yet modeled · {}", node.name),
                    "Secondary/rebound contacts and modifier inheritance still require verification; excluded from displayed DPS. Confirmed primary-hit bonuses are calculated separately.",
                    (0.0, 0.0),
                ));
            }
        }
    }
    // Static YYC analysis gives a 1.5s outer orbit. Contact coverage is an
    // explicit estimate: average one/two contacting blades => 75% per blade.
    // See engine/src/calc/blender-model.md for assumptions and binary evidence.
    let nano_rank = deps
        .subskill_ranks
        .get(&subskill_key("blender", "a_i_empowered_nanoblenders"))
        .copied()
        .unwrap_or(0)
        .min(3);
    let is_blender = active_skill.is_some_and(|s| s.id == "blender");
    let circle_of_slugs = active_skill.is_some_and(|s| s.id == "buckshot")
        && subtree_stat("wave_count") > 0.0;
    let nanoblenders = is_blender && nano_rank > 0;
    let blender_cast_rate = is_blender.then(|| {
        let haste = rg(&computed.stats, "skill_haste");
        let scoped = rg(main_scoped, "skill_haste");
        let cooldown = active_skill
            .and_then(|s| s.base_cooldown)
            .unwrap_or(8.0)
            .max(0.001);
        super::skill_cost::cooldown_rate(
            active_skill.unwrap(),
            cooldown,
            (haste.0 + scoped.0, haste.1 + scoped.1),
        )
    });
    let effective_projectiles: Option<u32> = active_skill.map(|s| {
        let base = deps
            .skill_projectiles
            .get(&s.id)
            .copied()
            .unwrap_or_else(|| s.base_projectiles.unwrap_or(if is_blender { 2 } else { 1 }));
        if nanoblenders {
            let groups = 2 + nano_rank;
            // The subtree currently stores group count alongside extra blades.
            // Remove that contribution before multiplying these separate axes.
            let blades = base + (subtree_stat("projectile_count") - groups as f64).max(0.0) as u32;
            let count = groups * blades.max(1);
            calculation.push(CalculationStep::new("Nanoblender contact estimate", format!("Estimated: {} small blenders × {} blades; 75% contact per 1.5s orbit (position and target size vary); overlapping casts counted independently", groups, blades.max(1)), scalar(count as f64 * 0.75 / 1.5)));
            return count;
        }
        if circle_of_slugs {
            // The projectiles spread through 360 degrees. Each has its own
            // hit list, but that does not mean every projectile hits one target.
            // Separate the verified wave count from an explicit spatial model.
            calculation.push(CalculationStep::new(
                "Circle of Slugs contact estimate",
                format!(
                    "Estimated: one projectile hits the target per wave; 1 + {} additional waves. Angular projectiles are not guaranteed single-target contacts; target size and position change overlap",
                    number(subtree_stat("wave_count"))
                ),
                scalar(1.0 + subtree_stat("wave_count")),
            ));
            return 1;
        }
        let count = effective_projectile_count(base, &subtree_stat);
        calculation.push(CalculationStep::new("Effective projectiles", format!("round(({} base + {} subtree, capped at {} when nonzero) × (1 + {}% extra volleys / 100)); damage uses at least 1", base, number(subtree_stat("projectile_count")), number(subtree_stat("single_target_hit_cap")), number(subtree_stat("extra_volleys_pct"))), scalar(count.max(1) as f64)));
        count
    });

    let active_calc_skill: Option<&CalcSkill> =
        active_skill.and_then(|s| skills_by_name.get(&normalize_skill_name(&s.name)));
    // Subskill transmutations rewrite the main skill's tags (e.g. Ancient
    // Device turns Death from Above into a Sentry).
    let effective_skill: Option<CalcSkill> = match (active_skill, active_calc_skill) {
        (Some(spec), Some(calc_skill)) => {
            let mut skill = calc_skill.clone();
            skill.tags =
                super::subskill::effective_skill_tags(&spec.id, &skill.tags, deps.subskill_ranks);
            Some(skill)
        }
        _ => None,
    };
    let active_calc_skill: Option<&CalcSkill> = effective_skill.as_ref();
    let is_attack_skill = active_calc_skill.and_then(|s| s.attack_kind) == Some(AttackKind::Attack);

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

    let (conversions, conversion_steps) = conversion::resolve_with_calculation(
        main_scoped,
        &computed.attributes,
        &computed.stats,
        active_calc_skill.map(|s| s.tags.as_slice()).unwrap_or(&[]),
    );

    calculation.extend(conversion_steps);

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
                    of_total_damage: active_of_total_damage,
                    action_rate_override: active_skill.and_then(|spec| {
                        attack_action_rate_override(
                            spec,
                            &calc_skill.tags,
                            &computed.stats,
                            main_scoped,
                            deps.entity_rates,
                        )
                    }),
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
    let (eff_cast_min, eff_cast_max) = if let Some(kind) = entity_kind {
        // The entity swings on its own cadence; player FCR / attack speed stay out of it.
        let swing =
            super::skill_cost::entity_rate(kind, entity_tags, &computed.stats, deps.entity_rates);
        stat_inputs(
            &mut calculation,
            &computed.stats,
            super::affix_tags::keys_for(super::types::AffixEffect::AttackSpeed, entity_tags),
        );
        stat_inputs(
            &mut calculation,
            &computed.stats,
            [format!("{}_attack_rate_fixed", kind.to_lowercase())],
        );
        calculation.push(CalculationStep::new("Entity actions per second", format!("{} configured/default base × (1 + {}% entity attack speed / 100); a fixed-rate subtree overrides this", number(swing.base), range(super::affix_tags::sum_for(super::types::AffixEffect::AttackSpeed, entity_tags, &computed.stats))), (swing.min, swing.max)));
        (Some(swing.min), Some(swing.max))
    } else {
        let (base_rate, rate_bonus) = if active_skill.is_some_and(|s| s.uses_attack_speed) {
            (
                Some(r_max(stat("attacks_per_second"))),
                combine_additive_and_more(
                    stat("increased_attack_speed"),
                    stat("increased_attack_speed_more"),
                ),
            )
        } else if active_skill.is_some_and(|s| s.uses_skill_haste) {
            // Cooldown-gated cast: one cast per cooldown, and skill haste is what shortens it.
            (
                active_skill.and_then(|s| {
                    super::skill_cost::effective_cooldown(s, main_scoped)
                        .map(|cd| 1.0 / cd)
                        .or(s.base_cast_rate)
                }),
                (stat("skill_haste").0 * 0.5, stat("skill_haste").1 * 0.5),
            )
        } else {
            (
                active_skill.and_then(|s| s.base_cast_rate),
                combine_additive_and_more(stat("faster_cast_rate"), stat("faster_cast_rate_more")),
            )
        };
        let base_rate_min = if active_skill.is_some_and(|s| s.uses_attack_speed) {
            Some(stat("attacks_per_second").0)
        } else {
            base_rate
        };
        let animation_rate = (
            base_rate_min.map(|r| (r * (1.0 + rate_bonus.0 / 100.0)).max(0.0)),
            base_rate.map(|r| (r * (1.0 + rate_bonus.1 / 100.0)).max(0.0)),
        );
        let cooldown = active_skill
            .and_then(|s| super::skill_cost::effective_cooldown(s, main_scoped));
        let effective_rate = if let Some(cooldown) = cooldown {
            let haste = stat("skill_haste");
            let gate = super::skill_cost::cooldown_rate(active_skill.unwrap(), cooldown, haste);
            // usesSkillHaste already selected the cooldown as its base. Other
            // declared base rates remain an animation/action-speed ceiling.
            if active_skill.is_some_and(|s| s.uses_skill_haste) {
                (Some(gate.0), Some(gate.1))
            } else {
                (
                    Some(animation_rate.0.map_or(gate.0, |r| r.min(gate.0))),
                    Some(animation_rate.1.map_or(gate.1, |r| r.min(gate.1))),
                )
            }
        } else {
            animation_rate
        };
        if attack_damage.is_none() {
            let rate_keys = if active_skill.is_some_and(|s| s.uses_attack_speed) {
                vec![
                    "attacks_per_second",
                    "increased_attack_speed",
                    "increased_attack_speed_more",
                ]
            } else if active_skill.is_some_and(|s| s.uses_skill_haste) {
                vec!["skill_haste"]
            } else {
                vec!["faster_cast_rate", "faster_cast_rate_more"]
            };
            stat_inputs(&mut calculation, &computed.stats, rate_keys);
            if cooldown.is_some() {
                stat_inputs(&mut calculation, &computed.stats, ["skill_haste"]);
            }
            if let Some(rate) = effective_rate.0.zip(effective_rate.1) {
                let expression = if let Some(cd) = cooldown {
                    format!("Cooldown ceiling (1 + {} skill haste × 0.005) / {}s; limited by the configured action rate when present", range(stat("skill_haste")), number(cd))
                } else {
                    format!(
                        "{} base rate × (1 + {}% effective speed / 100)",
                        range((base_rate_min.unwrap_or(0.0), base_rate.unwrap_or(0.0))),
                        range(rate_bonus)
                    )
                };
                calculation.push(CalculationStep::new("Actions per second", expression, rate));
            }
        }
        effective_rate
    };

    // Sentries, summons and guardians are counted apart: each entity the skill
    // fields is a full extra DPS source.
    // A "one massive X scaling with the maximum amount" note fields a single
    // entity; the count feeds its damage through conversion_summon_count instead.
    let is_single_entity = subtree_stat("conversion_summon_count") > 0.0;
    let (count_min, count_max) = if is_entity && !is_single_entity {
        let extra = super::affix_tags::sum_for(
            super::types::AffixEffect::MaxAmount,
            entity_tags,
            &computed.stats,
        );
        (1.0 + extra.0.max(0.0), 1.0 + extra.1.max(0.0))
    } else {
        (1.0, 1.0)
    };

    // Skill Duration stretches the damage object's life, which buys extra ticks.
    let duration_scoped = rg(main_scoped, "skill_duration");
    let duration_global = stat("skill_duration");
    let (mut hits_min, mut hits_max) = hits_per_cast(
        active_skill,
        (
            duration_scoped.0 + duration_global.0,
            duration_scoped.1 + duration_global.1,
        ),
    );

    if let Some(spec) = active_skill.filter(|_| is_blender) {
        (hits_min, hits_max) = super::blender::contacts(
            spec,
            deps.subskill_ranks,
            &computed.stats,
            main_scoped,
            &mut calculation,
        );
    }
    if circle_of_slugs {
        let waves = 1.0 + subtree_stat("wave_count");
        (hits_min, hits_max) = (waves, waves);
    }

    let multicast = damage
        .as_ref()
        .map(|d| d.multicast_multiplier)
        .unwrap_or(1.0);
    let projectiles = effective_projectiles.unwrap_or(1).max(1) as f64;
    let (hit_dps_min, hit_dps_max, avg_hit_dps_min, avg_hit_dps_max) =
        if let Some(ad) = attack_damage.as_ref() {
            (
                Some(
                    (ad.physical_hit_min as f64 + ad.poison_hit_min as f64 * multicast)
                        * ad.attacks_per_second_min,
                ),
                Some(
                    (ad.physical_hit_max as f64 + ad.poison_hit_max as f64 * multicast)
                        * ad.attacks_per_second_max,
                ),
                Some(ad.combined_avg_min as f64 * ad.attacks_per_second_min),
                Some(ad.combined_avg_max as f64 * ad.attacks_per_second_max),
            )
        } else {
            (
                damage
                    .as_ref()
                    .and_then(|d| eff_cast_min.map(|c| d.final_min as f64 * multicast * c)),
                damage
                    .as_ref()
                    .and_then(|d| eff_cast_max.map(|c| d.final_max as f64 * multicast * c)),
                damage
                    .as_ref()
                    .and_then(|d| eff_cast_min.map(|c| d.avg_min as f64 * c)),
                damage
                    .as_ref()
                    .and_then(|d| eff_cast_max.map(|c| d.avg_max as f64 * c)),
            )
        };
    let hit_dps_min = hit_dps_min.map(|v| v * count_min * hits_min * projectiles);
    let hit_dps_max = hit_dps_max.map(|v| v * count_max * hits_max * projectiles);
    let avg_hit_dps_min = avg_hit_dps_min.map(|v| v * count_min * hits_min);
    let avg_hit_dps_max = avg_hit_dps_max.map(|v| v * count_max * hits_max);

    let modeled_actions = match attack_damage.as_ref() {
        Some(ad) => (ad.attacks_per_second_min, ad.attacks_per_second_max),
        None if active_rank > 0 => (eff_cast_min.unwrap_or(0.0), eff_cast_max.unwrap_or(0.0)),
        None => (0.0, 0.0),
    };
    let extra_projectiles = rg(main_scoped, "expected_additional_projectiles");
    let expected_projectiles = (
        projectiles + extra_projectiles.0.max(0.0),
        projectiles + extra_projectiles.1.max(0.0),
    );
    let modeled_contacts = (
        modeled_actions.0 * count_min * hits_min * expected_projectiles.0 * multicast,
        modeled_actions.1 * count_max * hits_max * expected_projectiles.1 * multicast,
    );
    // An entity's attacks do not cast the player's summoning skill again.
    // Global on-cast events use the player's deployment/recast cadence.
    let player_actions = if is_entity {
        active_skill
            .filter(|_| active_rank > 0)
            .and_then(|skill| computed.skill_costs.get(&skill.id))
            .map(|cost| (cost.cast_rate_min.unwrap_or(0.0), cost.cast_rate_max.unwrap_or(0.0)))
            .unwrap_or((0.0, 0.0))
    } else {
        modeled_actions
    };
    if is_entity {
        calculation.push(CalculationStep::new(
            "Player skill uses per second",
            "Player deployment/recast rate for on-cast effects; entity attacks have their own rate",
            player_actions,
        ));
    }
    // With no selected skill, on-hit item/passive procs can still accompany
    // basic weapon attacks. An unlearned selected skill supplies no contacts.
    let proc_contacts = if active_skill.is_none() && deps.main_skill_id.is_none() {
        let speed = combine_additive_and_more(
            stat("increased_attack_speed"),
            stat("increased_attack_speed_more"),
        );
        let base = stat("attacks_per_second");
        (
            base.0.max(0.0) * (1.0 + speed.0 / 100.0).max(0.0),
            base.1.max(0.0) * (1.0 + speed.1 / 100.0).max(0.0),
        )
    } else {
        modeled_contacts
    };
    calculation.push(CalculationStep::new(
        "Proc trigger contacts per second",
        if active_skill.is_none() && deps.main_skill_id.is_none() {
            "No main skill selected: basic weapon attacks supply outgoing on-hit events".into()
        } else {
            format!(
                "{} actions/s × {} entities × {} contacts/cast × {} projectiles × {} multicast",
                range(modeled_actions),
                range((count_min, count_max)),
                range((hits_min, hits_max)),
                range(expected_projectiles),
                number(multicast)
            )
        },
        proc_contacts,
    ));

    let ctx = ProcContext {
        computed: &computed,
        deps: *deps,
        skill_ranks_by_name: &skill_ranks_by_name,
        skills_by_name: &skills_by_name,
        all_class_skills,
        empty_scoped: &empty_scoped,
    };
    let mut proc_dps_min: f64 = 0.0;
    let mut proc_dps_max: f64 = 0.0;
    if let Some(spec) = active_skill.filter(|_| is_blender && active_rank > 0) {
        let cadence = blender_cast_rate.unwrap_or((0.0, 0.0));
        let blades = effective_projectiles.unwrap_or(1).max(1) as f64;
        (proc_dps_min, proc_dps_max) = super::blender::secondary(
            spec,
            deps.subskill_ranks,
            (
                cadence.0 * count_min * hits_min * blades,
                cadence.1 * count_max * hits_max * blades,
            ),
            (
                avg_hit_dps_min.unwrap_or(0.0),
                avg_hit_dps_max.unwrap_or(0.0),
            ),
            deps.kills_per_sec,
            &mut calculation,
        );
    }
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
        let Some(target_dmg) = proc_target_damage(&ctx, &target_name, None) else {
            continue;
        };
        let rate = proc_trigger_rate(
            &proc.trigger,
            player_actions,
            proc_contacts,
            deps.kills_per_sec,
        );
        let chance = (proc.chance / 100.0).clamp(0.0, 1.0);
        let factor = (rate.0 * chance, rate.1 * chance);
        for step in target_dmg.calculation() {
            calculation.push(CalculationStep::new(
                format!("Proc {} · {}", proc_skill.name, step.label()),
                step.expression(),
                step.value(),
            ));
        }
        calculation.push(CalculationStep::new(
            format!("Proc · {}", proc_skill.name),
            format!(
                "{} triggers/s{} × {}% chance / 100 × {} target average damage",
                range(rate),
                proc_trigger_assumption(&proc.trigger),
                number(proc.chance),
                range((target_dmg.avg_min as f64, target_dmg.avg_max as f64))
            ),
            (
                factor.0 * target_dmg.avg_min as f64,
                factor.1 * target_dmg.avg_max as f64,
            ),
        ));
        proc_dps_min += factor.0 * target_dmg.avg_min as f64;
        proc_dps_max += factor.1 * target_dmg.avg_max as f64;
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
            if !deps.proc_toggles.get(&toggle_key).copied().unwrap_or(false) {
                continue;
            }
            let sub_rank = deps.subskill_ranks.get(&toggle_key).copied().unwrap_or(0);
            if sub_rank == 0 {
                continue;
            }
            let target_name = normalize_skill_name(sub_target);
            let Some(target_dmg) = proc_target_damage(&ctx, &target_name, None) else {
                continue;
            };
            let chance = sub_proc.chance.base.unwrap_or(0.0)
                + sub_proc.chance.per_rank.unwrap_or(0.0) * sub_rank as f64;
            // An active skill's on-hit/on-cast subtree belongs to that skill;
            // allocating it must not make a different main skill trigger it.
            let owner_active =
                deps.main_skill_id == Some(owner_skill.id.as_str()) && active_rank > 0;
            let actions = if owner_active {
                player_actions
            } else {
                (0.0, 0.0)
            };
            let contacts = if owner_active {
                modeled_contacts
            } else {
                (0.0, 0.0)
            };
            let rate = proc_trigger_rate(&sub_proc.trigger, actions, contacts, deps.kills_per_sec);
            let chance_fraction = (chance / 100.0).clamp(0.0, 1.0);
            let factor = (rate.0 * chance_fraction, rate.1 * chance_fraction);
            for step in target_dmg.calculation() {
                calculation.push(CalculationStep::new(
                    format!("Proc {} / {} · {}", owner_skill.name, sub.id, step.label()),
                    step.expression(),
                    step.value(),
                ));
            }
            calculation.push(CalculationStep::new(
                format!("Proc · {} / {}", owner_skill.name, sub.id),
                format!(
                    "{} triggers/s{} × {}% chance / 100 × {} target average damage",
                    range(rate),
                    proc_trigger_assumption(&sub_proc.trigger),
                    number(chance),
                    range((target_dmg.avg_min as f64, target_dmg.avg_max as f64))
                ),
                (
                    factor.0 * target_dmg.avg_min as f64,
                    factor.1 * target_dmg.avg_max as f64,
                ),
            ));
            proc_dps_min += factor.0 * target_dmg.avg_min as f64;
            proc_dps_max += factor.1 * target_dmg.avg_max as f64;
        }
    }

    // Item procs fire on this ICD even when their listed cooldown is shorter.
    const ITEM_PROC_ICD_SECS: f64 = 1.5;

    // "18% Chance on Hit to cast Breath of Ice Level 60": the item casts a class
    // skill at its own level, independent of the rank the build put into it.
    for eq in deps.inventory.values() {
        let Some(base) = data::get_item(&eq.base_id) else {
            continue;
        };
        for proc in base.procs.as_deref().unwrap_or(&[]) {
            let (Some(target), Some(level)) = (proc.target.as_ref(), proc.cast_level) else {
                continue;
            };
            let target_name = normalize_skill_name(target);
            let toggle_key = item_cast_toggle_key(&eq.base_id, &target_name);
            if !deps.proc_toggles.get(&toggle_key).copied().unwrap_or(false) {
                continue;
            }
            let Some(target_dmg) = proc_target_damage(&ctx, &target_name, Some(level as f64))
            else {
                continue;
            };
            let rate = proc_trigger_rate(
                &proc.trigger,
                player_actions,
                proc_contacts,
                deps.kills_per_sec,
            );
            let chance = (proc.chance / 100.0).clamp(0.0, 1.0);
            let factor = (
                (rate.0 * chance).min(1.0 / ITEM_PROC_ICD_SECS),
                (rate.1 * chance).min(1.0 / ITEM_PROC_ICD_SECS),
            );
            for step in target_dmg.calculation() {
                calculation.push(CalculationStep::new(
                    format!("Item proc {} / {} · {}", base.name, target, step.label()),
                    step.expression(),
                    step.value(),
                ));
            }
            calculation.push(CalculationStep::new(format!("Item proc · {} / {}", base.name, target), format!("min({} triggers/s{} × {}% / 100, 1 / {}s cooldown) × {} target average damage at rank {}", range(rate), proc_trigger_assumption(&proc.trigger), number(proc.chance), number(ITEM_PROC_ICD_SECS), range((target_dmg.avg_min as f64, target_dmg.avg_max as f64)), level), (factor.0 * target_dmg.avg_min as f64, factor.1 * target_dmg.avg_max as f64)));
            proc_dps_min += factor.0 * target_dmg.avg_min as f64;
            proc_dps_max += factor.1 * target_dmg.avg_max as f64;
        }
    }

    {
        // ponytail: no ignore_res / crit parity with class skills — add when a proc build cares.
        // Granted effects share gear and global buffs, but never inherit the
        // selected class skill's subtree. Build this context only when needed.
        let mut granted_computed = None;
        for granted in data::item_granted_skills().iter() {
            let Some(proc_damage) = granted.proc_damage.as_ref() else {
                continue;
            };
            let toggle_key = format!("granted:{}", granted.id);
            if !deps.proc_toggles.get(&toggle_key).copied().unwrap_or(false) {
                continue;
            }
            let granted_computed =
                granted_computed.get_or_insert_with(|| compute_stats_for_skill(deps, None));
            let key = normalize_skill_name(&granted.name);
            let Some(&(rank_min, rank_max)) = granted_computed.item_granted_ranks.get(&key) else {
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
                let (extra_mult, _) = super::skills::collect_extra_damage(
                    &granted_computed.stats,
                    deps.enemy_conditions,
                    Some(&p.damage_type),
                );
                let add = rg(
                    &granted_computed.stats,
                    &format!("{}_skill_damage", p.damage_type),
                );
                let more = rg(
                    &granted_computed.stats,
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
                    * extra_mult
                    * res_mult;
                let dmg_max = (p.base + p.per_rank * rank_max)
                    * (1.0 + r_max(add) / 100.0)
                    * (1.0 + r_max(more) / 100.0)
                    * extra_mult
                    * res_mult;
                calculation.push(CalculationStep::new(format!("Granted proc · {} / {}", granted.name, p.damage_type), format!("({} base + {} per rank × {} rank) × (1 + {}% increased / 100) × (1 + {}% more / 100) × {} generic/conditional damage × {} resistance / {}s interval", number(p.base), number(p.per_rank), range((rank_min, rank_max)), range(add), range(more), number(extra_mult), number(res_mult), number(interval)), (dmg_min / interval, dmg_max / interval)));
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
    let apply_chances =
        subtree_apply_chances(active_skill, deps.subskill_ranks, deps.enemy_conditions);
    // Volley averages include projectiles and repeated casts; ailments use
    // one contact's mean damage and the same individual-contact rate as procs.
    let ailment_contacts_per_action = projectiles * multicast;
    let (ailment_min, ailment_min_steps) = ailment::ailment_calculation(
        hit_avg_min / ailment_contacts_per_action,
        modeled_contacts.0,
        &computed.stats,
        main_scoped,
        &apply_chances,
    );
    let (ailment_max, ailment_max_steps) = ailment::ailment_calculation(
        hit_avg_max / ailment_contacts_per_action,
        modeled_contacts.1,
        &computed.stats,
        main_scoped,
        &apply_chances,
    );
    let ailment_dps_min = (ailment_min > 0.0).then_some(ailment_min);
    let ailment_dps_max = (ailment_max > 0.0).then_some(ailment_max);

    // Execution shortens the kill by the bottom `t%` of the life bar, so the
    // effective DPS rises by 1/(1 - t). Bosses cannot be executed.
    let execute_below = subtree_stat("execute_below").clamp(0.0, EXECUTE_MAX_PCT);
    let is_boss = deps
        .enemy_conditions
        .get("is_boss")
        .copied()
        .unwrap_or(false);
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

    stat_inputs(&mut calculation, &computed.stats, ["skill_duration"]);
    stat_inputs(
        &mut calculation,
        &computed.stats,
        super::affix_tags::keys_for(super::types::AffixEffect::MaxAmount, entity_tags),
    );
    calculation.push(CalculationStep::new(
        "Entity count",
        if is_single_entity {
            "One massive entity: maximum count scales conversion damage instead"
        } else {
            "1 + matching maximum entity amount; ordinary skills use 1"
        },
        (count_min, count_max),
    ));
    if !is_blender && !circle_of_slugs {
        calculation.push(CalculationStep::new("Hits per cast", active_skill.and_then(|skill| skill.hit_model.as_ref()).and_then(|model| model.lifetime.zip(model.tick_frequency)).filter(|(lifetime, tick)| *lifetime > 0.0 && *tick > 0.0).map(|(lifetime, tick)| format!("max(1, floor({} lifetime × (1 + ({} global + {} subtree)% duration / 100) / {} tick interval) + 1)", number(lifetime), range(duration_global), range(duration_scoped), number(tick))).unwrap_or_else(|| "No positive lifetime/tick interval in skill hit model: one hit per cast".into()), (hits_min, hits_max)));
    }
    if let Some(dps) = avg_hit_dps_min.zip(avg_hit_dps_max) {
        calculation.push(CalculationStep::new(
            "Average hit DPS",
            format!(
                "{} average damage × {} actions/s × {} entities × {} hits per cast",
                range((hit_avg_min, hit_avg_max)),
                range(modeled_actions),
                range((count_min, count_max)),
                range((hits_min, hits_max))
            ),
            dps,
        ));
    } else {
        calculation.push(CalculationStep::new(
            "Average hit DPS unavailable",
            "No active damage or no configured base action rate; no hit DPS is added",
            scalar(0.0),
        ));
    }
    for (bound, steps) in [
        ("Minimum", ailment_min_steps),
        ("Maximum", ailment_max_steps),
    ] {
        for step in steps {
            calculation.push(CalculationStep::new(
                format!("{bound} · {}", step.label()),
                step.expression(),
                step.value(),
            ));
        }
    }
    calculation.push(CalculationStep::new(
        "Proc DPS",
        "Sum of the enabled proc contributions above",
        (proc_dps_min, proc_dps_max),
    ));
    calculation.push(CalculationStep::new(
        "Ailment DPS",
        "Sum of the applicable damage-over-time contributions above",
        (ailment_min, ailment_max),
    ));
    calculation.push(CalculationStep::new(
        "Execute multiplier",
        if is_boss {
            "Boss target: execution disabled (×1)".into()
        } else {
            format!(
                "1 / (1 − {}% execute threshold / 100); threshold clamped to 0–90%",
                number(execute_below)
            )
        },
        scalar(execute_mult),
    ));
    if let Some(dps) = combined_dps_min.zip(combined_dps_max) {
        calculation.push(CalculationStep::new(
            "Combined DPS",
            format!(
                "({} average hit DPS + {} proc DPS + {} ailment DPS) × {} execute",
                range((
                    avg_hit_dps_min.unwrap_or(0.0),
                    avg_hit_dps_max.unwrap_or(0.0)
                )),
                range((proc_dps_min, proc_dps_max)),
                range((ailment_min, ailment_max)),
                number(execute_mult)
            ),
            dps,
        ));
    }

    BuildPerformance {
        calculation,
        calculation_sources: computed.stat_sources,
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
        hits_per_cast: (hits_max > 1.0).then_some((hits_min, hits_max)),
        ehp: computed.ehp,
        defense_insights: computed.defense_insights,
        skill_costs: computed.skill_costs,
    }
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;

impl BuildPerformance {
    pub fn calculation(&self) -> &[CalculationStep] {
        &self.calculation
    }
    pub fn calculation_sources(&self) -> &super::stats::SourceMap {
        &self.calculation_sources
    }
}

#[cfg(test)]
#[path = "build_cadence_tests.rs"]
mod cadence_tests;

#[cfg(test)]
#[path = "build_proc_tests.rs"]
mod proc_tests;
