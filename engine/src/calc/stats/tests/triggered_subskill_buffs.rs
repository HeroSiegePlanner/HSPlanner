use super::super::*;
use super::empty_input;
use crate::calc::skills::collect_extra_damage;
use crate::calc::subskill::subskill_key;

fn prey_stats(
    parent_rank: u32,
    node_rank: u32,
    active: Option<bool>,
    main: Option<&str>,
    chainfueled: bool,
) -> ComputedStats {
    let allocated = HashMap::new();
    let inventory = Inventory::new();
    let ranks = HashMap::from([
        ("chain_rip".into(), parent_rank),
        ("slicing_throw".into(), 1),
    ]);
    let key = subskill_key("chain_rip", "prey_on_the_weak");
    let buffs = active.map(|on| (key.clone(), on)).into_iter().collect();
    let mut subskills = HashMap::from([(key, node_rank)]);
    if chainfueled {
        subskills.insert(subskill_key("chain_rip", "chainfueled_hunting"), 1);
    }
    let custom = vec![CustomStat {
        stat_key: "all_skills".into(),
        value: "5-10".into(),
    }];
    let tree = HashSet::new();
    let sockets = HashMap::new();
    let conditions = HashMap::new();
    let mut input = empty_input(
        &allocated,
        &inventory,
        &ranks,
        &buffs,
        &custom,
        &tree,
        &sockets,
        &conditions,
        &subskills,
        &conditions,
    );
    input.class_id = Some("butcher");
    input.main_skill_id = main;
    compute_build_stats(&input)
}

#[test]
fn prey_buff_requires_learned_parent_node_and_explicit_active_snapshot() {
    for parent in [0, 1] {
        for rank in [0, 1, 2, 3, 5] {
            for active in [None, Some(false), Some(true)] {
                for main in [None, Some("chain_rip"), Some("slicing_throw")] {
                    let result = prey_stats(parent, rank, active, main, false);
                    let baseline = prey_stats(parent, 0, None, main, false);
                    let expected = if parent > 0 && rank > 0 && active == Some(true) {
                        15.0 * f64::from(rank)
                    } else {
                        0.0
                    };
                    assert_eq!(
                        result
                            .stats
                            .get("extra_damage_stunned")
                            .copied()
                            .unwrap_or_default(),
                        (expected, expected),
                        "parent={parent}, rank={rank}, active={active:?}, main={main:?}"
                    );
                    assert_eq!(
                        result.stats.get("crit_damage"),
                        baseline.stats.get("crit_damage"),
                        "metadata-only critical damage must not be granted"
                    );
                    let sources: Vec<_> = result
                        .stat_sources
                        .values()
                        .flatten()
                        .filter(|source| source.label.contains("Prey on the Weak"))
                        .collect();
                    assert_eq!(sources.len(), usize::from(expected > 0.0));
                    assert!(sources
                        .iter()
                        .all(|source| source.source_type == SourceType::Skill));
                    for key in ["extra_damage_stunned", "crit_damage"] {
                        assert!(result
                            .skill_scoped
                            .values()
                            .all(|stats| !stats.contains_key(key)));
                        assert!(
                            result
                                .skill_stat_overrides
                                .values()
                                .all(|stats| !stats.contains_key(key)),
                            "temporary global buff must not enter per-skill subtree overrides"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn prey_buff_global_bonus_survives_skill_variant_changes_and_only_scales_stunned_targets() {
    for chainfueled in [false, true] {
        let result = prey_stats(1, 3, Some(true), Some("slicing_throw"), chainfueled);
        for damage_type in ["physical", "fire", "cold", "lightning", "poison", "arcane"] {
            for stunned in [false, true] {
                let conditions = HashMap::from([("stunned".into(), stunned)]);
                let (factor, sources) =
                    collect_extra_damage(&result.stats, &conditions, Some(damage_type));
                assert_eq!(factor, if stunned { 1.45 } else { 1.0 });
                let prey = sources
                    .iter()
                    .find(|source| source.stat_key == Some("extra_damage_stunned"));
                assert_eq!(prey.map(|source| source.pct), stunned.then_some(45.0));
            }
        }
    }
}

#[test]
fn prey_buff_global_floor_preserves_local_stunned_fractions() {
    let mut sources = SourceMap::new();
    for (source_type, value) in [
        (SourceType::Skill, (15.0, 15.0)),
        (SourceType::Item, (0.6, 1.1)),
        (SourceType::Custom, (0.6, 1.1)),
        (SourceType::Subskill, (17.5, 19.25)),
    ] {
        push_source(
            &mut sources,
            "extra_damage_stunned",
            SourceContribution {
                label: "Test source".into(),
                source_type,
                value,
                forge: None,
            },
        );
    }
    let mut stats = compute_final_stats(&sources);
    stats.insert("crit_damage".into(), (25.5, 35.5));
    stats.insert("extra_damage_bleeding".into(), (4.5, 6.5));
    finalize_extra_damage_stunned(&sources, &mut stats);
    assert_eq!(stats["extra_damage_stunned"], (33.5, 36.25));
    assert_eq!(stats["crit_damage"], (25.5, 35.5));
    assert_eq!(stats["extra_damage_bleeding"], (4.5, 6.5));
    assert_eq!(sources["extra_damage_stunned"].len(), 4);
}

#[test]
fn prey_buff_pipeline_floors_after_other_global_sources_join() {
    let allocated = HashMap::new();
    let inventory = Inventory::new();
    let ranks = HashMap::from([("chain_rip".into(), 1)]);
    let key = subskill_key("chain_rip", "prey_on_the_weak");
    let buffs = HashMap::from([(key.clone(), true)]);
    let subskills = HashMap::from([(key, 1)]);
    let custom = vec![
        CustomStat {
            stat_key: "extra_damage_stunned".into(),
            value: "0.6-1.1".into(),
        };
        2
    ];
    let tree = HashSet::new();
    let sockets = HashMap::new();
    let conditions = HashMap::new();
    let mut input = empty_input(
        &allocated,
        &inventory,
        &ranks,
        &buffs,
        &custom,
        &tree,
        &sockets,
        &conditions,
        &subskills,
        &conditions,
    );
    input.class_id = Some("butcher");
    let result = compute_build_stats(&input);
    assert_eq!(result.stats["extra_damage_stunned"], (16.0, 17.0));
    assert_eq!(
        sum_contributions(&result.stat_sources["extra_damage_stunned"]),
        (16.2, 17.2)
    );
}

#[test]
fn prey_buff_effects_never_enter_owner_subtree_on_stunned_targets() {
    let ranks = HashMap::from([(subskill_key("chain_rip", "prey_on_the_weak"), 3)]);
    let conditions = HashMap::from([("stunned".into(), true)]);
    let mut attributes = SourceMap::new();
    let mut sources = SourceMap::new();
    let subtree = apply_subskill_aggregation(
        Some("butcher"),
        Some("chain_rip"),
        &ranks,
        Some(&conditions),
        &mut attributes,
        &mut sources,
        "",
    );
    for key in ["extra_damage_stunned", "crit_damage"] {
        assert!(!sources.contains_key(key));
        assert!(!subtree["chain_rip"].shared.contains_key(key));
        assert!(!subtree["chain_rip"].scoped.contains_key(key));
    }
}

struct TemporaryNodeBuff {
    skill: &'static str,
    node: &'static str,
    name: &'static str,
    effects: &'static [(&'static str, f64, f64)],
    trigger: &'static str,
    duration: f64,
}

const HELLSENT: TemporaryNodeBuff = TemporaryNodeBuff {
    skill: "butchers_hook",
    node: "hellsent",
    name: "Hellsent",
    effects: &[("movement_speed", 0.0, 15.0), ("phasing", 1.0, 0.0)],
    trigger: "on_cast",
    duration: 1.75,
};
const KILLING_SPREE: TemporaryNodeBuff = TemporaryNodeBuff {
    skill: "chain_swing",
    node: "killing_spree",
    name: "Killing Spree",
    effects: &[("increased_attack_speed", 0.0, 8.0)],
    trigger: "on_skill_complete",
    duration: 5.0,
};
const RAGE_TRANCE: TemporaryNodeBuff = TemporaryNodeBuff {
    skill: "chain_rip",
    node: "rage_trance",
    name: "Rage Trance",
    effects: &[
        ("physical_damage_reduction", 0.0, 8.0),
        ("magic_damage_reduction", 0.0, 8.0),
    ],
    trigger: "on_cast",
    duration: 3.0,
};

#[allow(clippy::too_many_arguments)]
fn temporary_node_stats(
    node: &TemporaryNodeBuff,
    parent_rank: u32,
    node_rank: u32,
    active: Option<bool>,
    namespaced: bool,
    main: Option<&str>,
    form: Option<&str>,
    variant_and_target: bool,
) -> ComputedStats {
    let allocated = HashMap::new();
    let inventory = Inventory::new();
    let mut ranks = HashMap::from([(node.skill.into(), parent_rank)]);
    let key = subskill_key(node.skill, node.node);
    let mut buffs = HashMap::new();
    if let Some(on) = active {
        buffs.insert(
            if namespaced {
                key.clone()
            } else {
                node.node.into()
            },
            on,
        );
    }
    if let Some(form) = form {
        ranks.insert(form.into(), 1);
        buffs.insert(form.into(), true);
    }
    let mut subskills = HashMap::from([(key, node_rank)]);
    if variant_and_target {
        for (parent, variant) in [
            ("chain_rip", "chainfueled_hunting"),
            ("chain_swing", "hookstorm"),
        ] {
            subskills.insert(subskill_key(parent, variant), 1);
        }
    }
    let custom = vec![CustomStat {
        stat_key: "all_skills".into(),
        value: "5-10".into(),
    }];
    let tree = HashSet::new();
    let sockets = HashMap::new();
    let conditions = HashMap::from([("phasing".into(), variant_and_target)]);
    let enemy = HashMap::from([
        ("stunned".into(), variant_and_target),
        ("bleeding".into(), variant_and_target),
    ]);
    let mut input = empty_input(
        &allocated,
        &inventory,
        &ranks,
        &buffs,
        &custom,
        &tree,
        &sockets,
        &conditions,
        &subskills,
        &enemy,
    );
    input.class_id = Some("butcher");
    input.main_skill_id = main;
    compute_build_stats(&input)
}

fn assert_temporary_node_gate(node: &TemporaryNodeBuff) {
    for parent in [0, 1] {
        for rank in [0, 1, 2, 3, 5] {
            for (active, namespaced) in [
                (None, true),
                (Some(false), true),
                (Some(true), true),
                (Some(true), false),
            ] {
                let result = temporary_node_stats(
                    node,
                    parent,
                    rank,
                    active,
                    namespaced,
                    Some(node.skill),
                    None,
                    false,
                );
                let baseline = temporary_node_stats(
                    node,
                    parent,
                    0,
                    None,
                    true,
                    Some(node.skill),
                    None,
                    false,
                );
                let enabled = parent > 0 && rank > 0 && active == Some(true) && namespaced;
                for &(key, base, per_rank) in node.effects {
                    let expected = if enabled {
                        base + per_rank * f64::from(rank)
                    } else {
                        0.0
                    };
                    let before = baseline.stats.get(key).copied().unwrap_or_default();
                    let after = result.stats.get(key).copied().unwrap_or_default();
                    assert_eq!(
                        (after.0 - before.0, after.1 - before.1), (expected, expected),
                        "{} {key}: parent={parent}, rank={rank}, active={active:?}, namespaced={namespaced}",
                        node.name,
                    );
                    assert!(result
                        .skill_scoped
                        .values()
                        .all(|stats| !stats.contains_key(key)));
                    assert!(result
                        .skill_stat_overrides
                        .values()
                        .all(|stats| !stats.contains_key(key)));
                }
                assert_eq!(
                    result.stats.get("all_damage_taken_reduced_pct"),
                    baseline.stats.get("all_damage_taken_reduced_pct"),
                    "{} must not add a separate all-damage mitigation layer",
                    node.name,
                );
                let sources: Vec<_> = result
                    .stat_sources
                    .values()
                    .flatten()
                    .filter(|source| source.label.contains(node.name))
                    .collect();
                assert_eq!(sources.len(), if enabled { node.effects.len() } else { 0 });
                assert!(sources
                    .iter()
                    .all(|source| source.source_type == SourceType::Skill));
            }
        }
    }
}

#[test]
fn hellsent_buff_requires_learned_parent_node_and_namespaced_active_flag() {
    assert_temporary_node_gate(&HELLSENT);
}

#[test]
fn killing_spree_buff_requires_learned_parent_node_and_namespaced_active_flag() {
    assert_temporary_node_gate(&KILLING_SPREE);
}

#[test]
fn rage_trance_buff_requires_learned_parent_node_and_namespaced_active_flag() {
    assert_temporary_node_gate(&RAGE_TRANCE);
}

#[test]
fn temporary_subtree_buff_metadata_describes_acquisition_not_uptime() {
    for expected in [&HELLSENT, &KILLING_SPREE, &RAGE_TRANCE] {
        let skill = data::get_skills_by_class("butcher")
            .into_iter()
            .find(|skill| skill.id == expected.skill)
            .unwrap();
        let node = skill
            .subskills
            .as_ref()
            .unwrap()
            .iter()
            .find(|node| node.id == expected.node)
            .unwrap();
        let buff = node.triggered_buff.as_ref().expect(expected.name);
        assert_eq!(buff.trigger, expected.trigger);
        assert_eq!(buff.chance, 100.0);
        assert_eq!(buff.duration, expected.duration);
    }
}

#[test]
fn active_temporary_subtree_buffs_survive_main_variant_form_and_target_changes() {
    for node in [&HELLSENT, &KILLING_SPREE, &RAGE_TRANCE] {
        for main in [None, Some(node.skill), Some("slicing_throw")] {
            for form in [
                None,
                Some("holy_form"),
                Some("unholy_form"),
                Some("insatiable_hunger"),
            ] {
                for changed in [false, true] {
                    let result =
                        temporary_node_stats(node, 1, 3, Some(true), true, main, form, changed);
                    let baseline =
                        temporary_node_stats(node, 1, 0, None, true, main, form, changed);
                    for &(key, base, per_rank) in node.effects {
                        let before = baseline.stats.get(key).copied().unwrap_or_default();
                        let after = result.stats.get(key).copied().unwrap_or_default();
                        let expected = base + per_rank * 3.0;
                        assert_eq!(
                            (after.0 - before.0, after.1 - before.1),
                            (expected, expected),
                            "{} {key}: main={main:?}, form={form:?}, variant/target={changed}",
                            node.name,
                        );
                        let sources: Vec<_> = result
                            .stat_sources
                            .get(key)
                            .into_iter()
                            .flatten()
                            .filter(|source| source.label.contains(node.name))
                            .collect();
                        assert_eq!(sources.len(), 1);
                        assert_eq!(sources[0].source_type, SourceType::Skill);
                    }
                }
            }
        }
    }
}

#[test]
fn hellsent_buff_enables_real_phasing_tree_condition_without_mutating_input() {
    let allocated = HashMap::new();
    let inventory = Inventory::new();
    let sockets = HashMap::new();
    let enemy = HashMap::new();
    let tree = HashSet::from([221]);
    let key = subskill_key("butchers_hook", "hellsent");
    for parent_rank in [1, 0] {
        for node_rank in [1, 0] {
            for active in [Some(true), Some(false), None] {
                for manual in [Some(false), Some(true), None] {
                    let ranks = HashMap::from([("butchers_hook".into(), parent_rank)]);
                    let subskills = HashMap::from([(key.clone(), node_rank)]);
                    let buffs = active.map(|on| (key.clone(), on)).into_iter().collect();
                    let conditions: HashMap<_, _> = manual
                        .map(|on| ("phasing".into(), on))
                        .into_iter()
                        .collect();
                    let original_conditions = conditions.clone();
                    let mut input = empty_input(
                        &allocated,
                        &inventory,
                        &ranks,
                        &buffs,
                        &[],
                        &tree,
                        &sockets,
                        &conditions,
                        &subskills,
                        &enemy,
                    );
                    input.class_id = Some("butcher");
                    input.main_skill_id = Some("slicing_throw");
                    let result = compute_build_stats(&input);
                    let enabled = manual == Some(true)
                        || (parent_rank > 0 && node_rank > 0 && active == Some(true));
                    let expected = if enabled { (15.0, 15.0) } else { (0.0, 0.0) };
                    assert_eq!(
                        result.stats.get("damage_mitigation").copied().unwrap_or_default(),
                        expected,
                        "parent={parent_rank}, node={node_rank}, active={active:?}, manual={manual:?}",
                    );
                    assert_eq!(conditions, original_conditions);
                    assert_eq!(input.player_conditions, &original_conditions);
                }
            }
        }
    }

    let ranks = HashMap::new();
    let buffs = HashMap::new();
    let subskills = HashMap::new();
    let conditions = HashMap::from([("phasing".into(), false)]);
    let custom = vec![CustomStat {
        stat_key: "phasing".into(),
        value: "1".into(),
    }];
    let mut input = empty_input(
        &allocated,
        &inventory,
        &ranks,
        &buffs,
        &custom,
        &tree,
        &sockets,
        &conditions,
        &subskills,
        &enemy,
    );
    input.class_id = Some("butcher");
    let result = compute_build_stats(&input);
    assert_eq!(result.stats["phasing"], (1.0, 1.0));
    assert_eq!(
        result
            .stats
            .get("damage_mitigation")
            .copied()
            .unwrap_or_default(),
        (0.0, 0.0)
    );
}

#[test]
fn rage_trance_adds_to_existing_reduction_pools_before_separate_ehp_layers() {
    let allocated = HashMap::new();
    let inventory = Inventory::new();
    let sockets = HashMap::new();
    let enemy = HashMap::new();
    let tree = HashSet::from([221]);
    let ranks = HashMap::from([
        ("chain_rip".into(), 1),
        ("butchers_hook".into(), 1),
        ("holy_form".into(), 1),
    ]);
    let rage = subskill_key("chain_rip", "rage_trance");
    let hellsent = subskill_key("butchers_hook", "hellsent");
    let subskills = HashMap::from([(rage.clone(), 3), (hellsent.clone(), 1)]);
    let buffs = HashMap::from([(rage, true), (hellsent, true), ("holy_form".into(), true)]);
    let custom = [
        ("physical_damage_reduction", "10"),
        ("magic_damage_reduction", "20"),
        ("all_damage_taken_reduced_pct", "5"),
        ("fire_resistance", "25"),
    ]
    .map(|(key, value)| CustomStat {
        stat_key: key.into(),
        value: value.into(),
    });

    for manual_phasing in [false, true] {
        let conditions = HashMap::from([("phasing".into(), manual_phasing)]);
        let mut input = empty_input(
            &allocated,
            &inventory,
            &ranks,
            &buffs,
            &custom,
            &tree,
            &sockets,
            &conditions,
            &subskills,
            &enemy,
        );
        input.class_id = Some("butcher");
        input.main_skill_id = Some("slicing_throw");
        let result = compute_build_stats(&input);
        assert_eq!(result.stats["physical_damage_reduction"], (34.0, 34.0));
        assert_eq!(result.stats["magic_damage_reduction"], (44.0, 44.0));
        assert_eq!(result.stats["all_damage_taken_reduced_pct"], (5.0, 5.0));
        assert_eq!(result.stats["damage_mitigation"], (15.0, 15.0));
        assert_eq!(result.stats["phasing"], (1.0, 1.0));
        assert_eq!(result.stats["movement_speed"], (30.0, 30.0));
        for key in ["physical_damage_reduction", "magic_damage_reduction"] {
            let sources = &result.stat_sources[key];
            assert_eq!(
                sources.len(),
                2,
                "one custom source and one Rage source: {key}"
            );
            assert_eq!(
                sources
                    .iter()
                    .filter(|s| s.label.contains("Rage Trance"))
                    .count(),
                1
            );
        }
        assert_eq!(result.stat_sources["damage_mitigation"].len(), 1);
        assert_eq!(result.stat_sources["phasing"].len(), 1);
        assert_eq!(
            result.stat_sources["movement_speed"]
                .iter()
                .filter(|s| s.label.contains("Hellsent"))
                .count(),
            1
        );

        let ehp = crate::calc::defense::compute_ehp(&result.stats);
        let life = result.stats["life"].1;
        for (damage_type, expected_multiplier) in [
            ("physical", 0.66 * 0.85 * 0.95),
            ("fire", 0.75 * 0.56 * 0.85 * 0.95),
        ] {
            let entry = ehp
                .entries
                .iter()
                .find(|entry| entry.damage_type == damage_type)
                .unwrap();
            assert!((entry.multiplier - expected_multiplier).abs() < 1e-12);
            assert!((entry.ehp.unwrap() - life / expected_multiplier).abs() < 1e-9);
            assert_eq!(
                entry
                    .layers
                    .iter()
                    .filter(|layer| layer.label == "All damage taken reduced")
                    .map(|layer| layer.pct)
                    .collect::<Vec<_>>(),
                vec![5.0]
            );
        }
        assert_eq!(conditions["phasing"], manual_phasing);
    }
}
