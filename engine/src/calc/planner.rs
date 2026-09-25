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
    let _scope = super::season::SeasonScope::enter(build.season.clone());
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

pub fn evaluate(input: &PlannerInput) -> PlannerPerformance {
    let prepared = prepare_input(input);
    let _scope = super::season::SeasonScope::enter(prepared.season.clone());
    let mut computed = calc_build_stats(prepared.clone());
    let primary = super::build::performance_from_stats(
        &super::commands::perf_deps(
            &prepared,
            &prepared.inventory,
            prepared.main_skill_id.as_deref(),
        ),
        computed.clone(),
    );
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
        season: prepared.season,
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

pub fn dps_mid(performance: &PlannerPerformance) -> f64 {
    let p = &performance.current;
    let value: Ranged = (
        p.combined_dps_min.or(p.hit_dps_min).unwrap_or(0.),
        p.combined_dps_max.or(p.hit_dps_max).unwrap_or(0.),
    );
    (value.0 + value.1) / 2.
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
