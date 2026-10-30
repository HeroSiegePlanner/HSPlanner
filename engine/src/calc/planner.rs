use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::{
    build::BuildPerformance,
    commands::{calc_build_performance, calc_build_stats, BuildPerformanceInput},
    data,
    rank::{item_skill_bonus_override, item_skill_rank_range},
    skills::Ranged,
    stats::{ComputedStats, SourceContribution, SourceType},
    types::Inventory,
};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlannerInput {
    #[serde(flatten)]
    pub build: BuildPerformanceInput,
    pub active_skill_ids: Vec<String>,
    pub disabled_potions: HashMap<String, bool>,
    pub merc_inventory: Inventory,
    pub merc_disabled_auras: HashMap<String, bool>,
    pub allocated_ether_nodes: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerPerformance {
    pub current: BuildPerformance,
    pub per_skill: Vec<SkillPerformance>,
    pub computed: ComputedStats,
    pub mercenary: ComputedStats,
    pub ether: Vec<EtherSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPerformance {
    pub skill_id: String,
    pub performance: BuildPerformance,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtherSummary {
    pub key: String,
    pub label: String,
    pub description: String,
    pub count: u32,
    pub value_per: f64,
    pub total: f64,
    pub is_percent: bool,
}

pub fn prepare_input(input: &PlannerInput) -> BuildPerformanceInput {
    let mut build = input.build.clone();
    build
        .inventory
        .retain(|slot, _| !input.disabled_potions.get(slot).copied().unwrap_or(false));
    // An enabled potion is drunk: its granted buff is on.
    for (slot, item) in &build.inventory {
        if !slot.starts_with("potion_") {
            continue;
        }
        let Some(base) = data::get_item(&item.base_id) else {
            continue;
        };
        for (name, _) in data::skill_bonus_entries(base, item) {
            if let Some(condition) = data::get_item_granted_skill_by_name(name)
                .and_then(|skill| skill.condition.clone())
            {
                build.player_conditions.insert(condition, true);
            }
        }
    }
    let mut boosts: HashMap<&str, u32> = HashMap::new();
    for item in build.inventory.values() {
        let Some(skill) = item.subskill_boost_skill_id.as_deref() else {
            continue;
        };
        let amount = item
            .implicit_overrides
            .get("grant_subskills")
            .copied()
            .or_else(|| {
                data::get_item(&item.base_id)?
                    .implicit
                    .as_ref()?
                    .get("grant_subskills")
                    .map(|v| v.as_ranged().1)
            })
            .unwrap_or(0.)
            .floor()
            .max(0.) as u32;
        *boosts.entry(skill).or_default() += amount;
    }
    for (key, rank) in &mut build.subskill_ranks {
        if *rank == 0 {
            continue;
        }
        if let Some((skill, _)) = key.split_once(':') {
            *rank = rank.saturating_add(boosts.get(skill).copied().unwrap_or(0));
        }
    }
    for (slot, item) in &input.merc_inventory {
        let Some(base) = data::get_item(&item.base_id) else {
            continue;
        };
        for (name, rank) in data::skill_bonus_entries(base, item) {
            let Some(_) = data::get_item_granted_skill_by_name(name).filter(|skill| skill.aura)
            else {
                continue;
            };
            if input
                .merc_disabled_auras
                .get(&name.trim().to_lowercase())
                .copied()
                .unwrap_or(false)
            {
                continue;
            }
            let stars = if data::can_star_forge(slot, &base.rarity) {
                item.stars
            } else {
                None
            };
            let rank = item_skill_rank_range(
                name,
                rank.as_ranged(),
                stars,
                item_skill_bonus_override(item, name),
            );
            let entry = build
                .granted_skill_ranks
                .entry(name.clone())
                .or_insert((0., 0.));
            entry.0 = entry.0.max(rank.0);
            entry.1 = entry.1.max(rank.1);
        }
    }
    build.main_skill_id = input
        .active_skill_ids
        .first()
        .cloned()
        .or(build.main_skill_id);
    build
}

/// The stat panel's leading numbers, as min–max ranges.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DpsSummary {
    pub hit_damage: Option<Ranged>,
    pub hit_dps: Option<Ranged>,
    pub combined_dps: Option<Ranged>,
}

pub fn dps_summary(input: &PlannerInput) -> DpsSummary {
    let current = evaluate(input).current;
    DpsSummary {
        hit_damage: current.hit_damage(),
        hit_dps: current.hit_dps_min.zip(current.hit_dps_max),
        combined_dps: current.combined_dps_min.zip(current.combined_dps_max),
    }
}

/// The skills whose damage counts: the selected ones, plus a pulsing aura (Scorching
/// Aura) in the aura slot, which deals its own damage beside them or on its own.
pub fn dps_skill_ids(input: &PlannerInput) -> Vec<String> {
    let build = &input.build;
    let mut ids = input.active_skill_ids.clone();
    let aura = damaging_aura(
        build.class_id.as_deref(),
        build.active_aura_id.as_deref(),
        &build.skill_ranks,
    );
    if let Some(aura) = aura.filter(|aura| !ids.iter().any(|id| id == aura)) {
        if ids.is_empty() {
            ids.extend(build.main_skill_id.clone());
        }
        ids.push(aura.to_string());
    }
    ids
}

/// The aura slot's skill when it is a ranked pulsing aura that deals damage.
pub fn damaging_aura<'a>(
    class_id: Option<&str>,
    active_aura_id: Option<&'a str>,
    skill_ranks: &HashMap<String, u32>,
) -> Option<&'a str> {
    let aura = active_aura_id?;
    let pulses = class_id.is_some_and(|class| {
        data::get_skills_by_class(class)
            .iter()
            .any(|s| s.id == aura && s.pulse_frames.is_some())
    });
    (pulses && skill_ranks.get(aura).copied().unwrap_or(0) > 0).then_some(aura)
}

fn with_damaging_aura(input: &PlannerInput) -> Cow<'_, PlannerInput> {
    let ids = dps_skill_ids(input);
    if ids == input.active_skill_ids {
        return Cow::Borrowed(input);
    }
    let mut out = input.clone();
    if out.build.main_skill_id.is_none() {
        out.build.main_skill_id = ids.first().cloned();
    }
    out.active_skill_ids = ids;
    Cow::Owned(out)
}

/// The combined DPS midpoint `evaluate` reports, without the stat breakdowns; for searches.
pub fn combined_dps(input: &PlannerInput) -> f64 {
    let input = &*with_damaging_aura(input);
    let prepared = prepare_input(input);
    super::commands::combined_dps_mid(&input.active_skill_ids, |main| {
        super::build::compute_build_performance(&super::commands::perf_deps(
            &prepared,
            &prepared.inventory,
            main.or(prepared.main_skill_id.as_deref()),
        ))
    })
}

/// The main skill's Hit DPS midpoint `evaluate` reports, without the stat breakdowns; for searches.
pub fn hit_dps(input: &PlannerInput) -> f64 {
    let prepared = prepare_input(input);
    let performance = super::build::compute_build_performance(&super::commands::perf_deps(
        &prepared,
        &prepared.inventory,
        prepared.main_skill_id.as_deref(),
    ));
    performance
        .hit_dps_min
        .zip(performance.hit_dps_max)
        .map_or(0., |(low, high)| (low + high) / 2.)
}

pub fn evaluate(input: &PlannerInput) -> PlannerPerformance {
    let input = &*with_damaging_aura(input);
    let prepared = prepare_input(input);
    let mut computed = calc_build_stats(prepared.clone());
    let primary = super::build::performance_from_stats(
        &super::commands::perf_deps(
            &prepared,
            &prepared.inventory,
            prepared.main_skill_id.as_deref(),
        ),
        computed.clone(),
    );
    // Crushing Blow stacks follow the main skill's contacts, known only after its pass.
    for key in ["crushing_blow_stacks", "crushing_blow_armor_break"] {
        if let Some(value) = primary.stats.get(key) {
            computed.stats.insert(key.into(), *value);
            let sources = primary.calculation_sources().get(key).cloned();
            computed
                .stat_sources
                .insert(key.into(), sources.unwrap_or_default());
        }
    }
    let mut per_skill = Vec::with_capacity(input.active_skill_ids.len());
    for (index, id) in input.active_skill_ids.iter().enumerate() {
        let mut build = prepared.clone();
        build.main_skill_id = Some(id.clone());
        per_skill.push(SkillPerformance {
            skill_id: id.clone(),
            performance: if index == 0 {
                primary.clone()
            } else {
                calc_build_performance(build)
            },
        });
    }
    let mut current = match per_skill.first() {
        Some(primary) => primary.performance.clone(),
        None => primary,
    };
    if !per_skill.is_empty() {
        merge_skills(&mut current, &per_skill);
    }
    let mercenary = calc_build_stats(BuildPerformanceInput {
        level: 1,
        inventory: input.merc_inventory.clone(),
        kills_per_sec: 1.,
        ..Default::default()
    });
    let ether = summarize_ether(&input.allocated_ether_nodes);
    let ether_mf = ether
        .iter()
        .find(|row| row.key == "etherUnSmall01")
        .map_or(0., |row| row.total);
    let merc_mf = mercenary
        .stats
        .get("magic_find")
        .copied()
        .unwrap_or_default();
    for (key, label, value, source_type) in [
        ("magic_find", "Mercenary", merc_mf, SourceType::Item),
        (
            "magic_find_more",
            "Ether Tree",
            (ether_mf, ether_mf),
            SourceType::Tree,
        ),
    ] {
        if value == (0., 0.) {
            continue;
        }
        let entry = computed.stats.entry(key.into()).or_default();
        entry.0 += value.0;
        entry.1 += value.1;
        computed
            .stat_sources
            .entry(key.into())
            .or_default()
            .push(SourceContribution {
                label: label.into(),
                source_type,
                value,
                forge: None,
            });
    }
    if merc_mf != (0., 0.) || ether_mf != 0. {
        let base = computed
            .stats
            .get("magic_find")
            .copied()
            .unwrap_or_default();
        let more = computed
            .stats
            .get("magic_find_more")
            .copied()
            .unwrap_or_default();
        computed.stats_combined.insert(
            "magic_find".into(),
            (base.0 * (1. + more.0 / 100.), base.1 * (1. + more.1 / 100.)),
        );
    }
    PlannerPerformance {
        current,
        per_skill,
        computed,
        mercenary,
        ether,
    }
}

fn merge_skills(current: &mut BuildPerformance, skills: &[SkillPerformance]) {
    let sum = |pick: fn(&BuildPerformance) -> Option<f64>, execute: bool| {
        skills
            .iter()
            .filter_map(|s| {
                pick(&s.performance).map(|v| {
                    v * if execute {
                        s.performance.execute_mult
                    } else {
                        1.
                    }
                })
            })
            .reduce(|a, b| a + b)
    };
    let combined = |average, ailment, proc| {
        let avg = sum(average, true);
        let ail = sum(ailment, true);
        (avg.is_some() || ail.is_some() || proc > 0.).then(|| {
            avg.unwrap_or(0.) + ail.unwrap_or(0.) + proc * skills[0].performance.execute_mult
        })
    };
    current.combined_dps_min = combined(
        |p| p.avg_hit_dps_min,
        |p| p.ailment_dps_min,
        current.proc_dps_min,
    );
    current.combined_dps_max = combined(
        |p| p.avg_hit_dps_max,
        |p| p.ailment_dps_max,
        current.proc_dps_max,
    );
    current.ailment_dps_min = sum(|p| p.ailment_dps_min, false);
    current.ailment_dps_max = sum(|p| p.ailment_dps_max, false);
    let names = skills
        .iter()
        .filter_map(|s| s.performance.active_skill_name.as_deref())
        .collect::<Vec<_>>()
        .join(" + ");
    current.active_skill_name = (!names.is_empty()).then_some(names);
}

#[derive(Deserialize)]
struct EtherData {
    nodes: Vec<EtherNode>,
    stats: HashMap<String, EtherStat>,
}
#[derive(Deserialize)]
struct EtherNode {
    id: u32,
    key: String,
}
#[derive(Deserialize)]
struct EtherStat {
    label: String,
    desc: String,
    value: String,
}
static ETHER: LazyLock<EtherData> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/ether-tree.json")).expect("valid Ether data")
});

pub fn summarize_ether(ids: &[u32]) -> Vec<EtherSummary> {
    let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    for node in &ETHER.nodes {
        if unique.contains(&node.id) {
            *counts.entry(&node.key).or_default() += 1;
        }
    }
    let mut out: Vec<_> = counts
        .into_iter()
        .filter_map(|(key, count)| {
            let stat = ETHER.stats.get(key)?;
            let number = stat
                .value
                .trim()
                .trim_end_matches('%')
                .parse::<f64>()
                .unwrap_or(0.);
            Some(EtherSummary {
                key: key.into(),
                label: stat.label.clone(),
                description: stat.desc.clone(),
                count,
                value_per: number,
                total: (number * f64::from(count) * 100.).round() / 100.,
                is_percent: stat.value.ends_with('%'),
            })
        })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.label.cmp(&b.label)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dps_mid(performance: &PlannerPerformance) -> f64 {
        let p = &performance.current;
        let value: Ranged = (
            p.combined_dps_min.or(p.hit_dps_min).unwrap_or(0.),
            p.combined_dps_max.or(p.hit_dps_max).unwrap_or(0.),
        );
        (value.0 + value.1) / 2.
    }

    #[test]
    fn mercenary_aura_uses_final_pinned_rank_including_zero() {
        let mut input = PlannerInput::default();
        let base = data::get_item("gloves_satanic_thor_s_battle_gloves").unwrap();
        input.merc_inventory.insert(
            "gloves".into(),
            super::super::types::EquippedItem {
                base_id: base.id.clone(),
                stars: Some(5),
                ..Default::default()
            },
        );
        assert_eq!(
            prepare_input(&input).granted_skill_ranks["Holy Aura"],
            (4., 6.)
        );
        for pin in [4., 0.] {
            input
                .merc_inventory
                .get_mut("gloves")
                .unwrap()
                .skill_bonus_overrides
                .insert("  HOLY AURA  ".into(), pin);
            assert_eq!(
                prepare_input(&input).granted_skill_ranks["Holy Aura"],
                (pin, pin)
            );
        }
        input.merc_disabled_auras.insert("holy aura".into(), true);
        assert!(!prepare_input(&input)
            .granted_skill_ranks
            .contains_key("Holy Aura"));
    }

    #[test]
    fn multiple_skills_keep_execute_and_count_shared_procs_once() {
        let a = BuildPerformance {
            avg_hit_dps_min: Some(100.),
            avg_hit_dps_max: Some(200.),
            ailment_dps_min: Some(10.),
            ailment_dps_max: Some(20.),
            proc_dps_min: 30.,
            proc_dps_max: 40.,
            execute_mult: 2.,
            ..Default::default()
        };
        let b = BuildPerformance {
            avg_hit_dps_min: Some(50.),
            avg_hit_dps_max: Some(100.),
            proc_dps_min: 900.,
            proc_dps_max: 900.,
            execute_mult: 3.,
            ..Default::default()
        };
        let mut result = a.clone();
        merge_skills(
            &mut result,
            &[
                SkillPerformance {
                    skill_id: "a".into(),
                    performance: a,
                },
                SkillPerformance {
                    skill_id: "b".into(),
                    performance: b,
                },
            ],
        );
        assert_eq!(result.combined_dps_min, Some(430.));
        assert_eq!(result.combined_dps_max, Some(820.));
        assert_eq!(result.hit_dps_min, None);
    }
    #[test]
    fn search_measures_match_the_reported_dps() {
        let skills: Vec<String> = data::get_skills_by_class("amazon")
            .iter()
            .filter(|s| s.damage_formula.is_some() || s.damage_per_rank.is_some())
            .take(2)
            .map(|s| s.id.clone())
            .collect();
        let mut input = PlannerInput::default();
        input.build.class_id = Some("amazon".into());
        input.build.level = 60;
        input.build.skill_ranks = skills.iter().map(|id| (id.clone(), 20)).collect();
        input.build.inventory.insert(
            "weapon".into(),
            super::super::types::EquippedItem {
                base_id: "base_mace_ogre_maul".into(),
                ..Default::default()
            },
        );
        for count in [1, 2] {
            input.active_skill_ids = skills[..count].to_vec();
            let reported = dps_mid(&evaluate(&input));
            assert!(reported > 0.);
            assert!((combined_dps(&input) - reported).abs() <= reported * 1e-9);
            let summary = dps_summary(&input);
            let (low, high) = summary.combined_dps.expect("combined DPS");
            assert!(((low + high) / 2. - reported).abs() <= reported * 1e-9);
            assert!(summary.hit_damage.is_some());
            let (low, high) = summary.hit_dps.expect("hit DPS");
            let hit = (low + high) / 2.;
            assert!(hit > 0. && (hit_dps(&input) - hit).abs() <= hit * 1e-9);
        }
    }

    #[test]
    fn preparation_boosts_only_spent_subskills_and_omits_disabled_potions() {
        let mut input = PlannerInput::default();
        let item = super::super::types::EquippedItem {
            base_id: "test".into(),
            subskill_boost_skill_id: Some("skill".into()),
            implicit_overrides: HashMap::from([("grant_subskills".into(), 2.)]),
            ..Default::default()
        };
        input.build.inventory.insert("weapon".into(), item.clone());
        input.build.inventory.insert("potion_1".into(), item);
        input.disabled_potions.insert("potion_1".into(), true);
        input.build.subskill_ranks = HashMap::from([
            ("skill:spent".into(), 1),
            ("skill:empty".into(), 0),
            ("other:spent".into(), 1),
        ]);
        let prepared = prepare_input(&input);
        assert_eq!(prepared.subskill_ranks["skill:spent"], 3);
        assert_eq!(prepared.subskill_ranks["skill:empty"], 0);
        assert_eq!(prepared.subskill_ranks["other:spent"], 1);
        assert!(!prepared.inventory.contains_key("potion_1"));
    }

    #[test]
    fn enabled_potions_activate_their_buffs() {
        let mut input = PlannerInput::default();
        for (slot, base_id) in [
            ("potion_1", "potion_satanic_radbull"),
            ("potion_2", "potion_satanic_witches_potion"),
        ] {
            input.build.inventory.insert(
                slot.into(),
                super::super::types::EquippedItem {
                    base_id: base_id.into(),
                    ..Default::default()
                },
            );
        }
        input.disabled_potions.insert("potion_2".into(), true);
        input
            .build
            .player_conditions
            .insert("hyper_activity_buff".into(), false);
        let prepared = prepare_input(&input);
        assert_eq!(prepared.player_conditions.get("hyper_activity_buff"), Some(&true));
        assert_ne!(
            prepared.player_conditions.get("conjured_replenishment_buff"),
            Some(&true)
        );
    }
}
