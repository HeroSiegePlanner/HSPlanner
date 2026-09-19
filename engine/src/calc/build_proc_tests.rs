use super::*;

fn winters_bite_for_class(
    class: &str,
    nodes: &[(&str, &str, u32)],
    custom: &[(&str, &str)],
    enabled: bool,
) -> BuildPerformance {
    let empty_ranks = HashMap::new();
    let inventory = HashMap::from([(
        "weapon".into(),
        super::super::types::EquippedItem {
            base_id: "axe_heroic_winter_s_bite".into(),
            ..Default::default()
        },
    )]);
    let subskills = nodes
        .iter()
        .map(|(skill, node, rank)| (subskill_key(skill, node), *rank))
        .collect();
    let bools = HashMap::new();
    let custom = custom
        .iter()
        .map(|(key, value)| CustomStat {
            stat_key: (*key).into(),
            value: (*value).into(),
        })
        .collect::<Vec<_>>();
    let tree = HashSet::new();
    let sockets = HashMap::new();
    let projectiles = HashMap::new();
    let numbers = HashMap::new();
    let toggles = HashMap::from([(
        item_cast_toggle_key("axe_heroic_winter_s_bite", "breath of ice"),
        enabled,
    )]);
    compute_build_performance(&BuildPerformanceDeps {
        class_id: Some(class),
        level: 50,
        allocated_attrs: &empty_ranks,
        inventory: &inventory,
        skill_ranks: &empty_ranks,
        subskill_ranks: &subskills,
        active_aura_id: None,
        active_buffs: &bools,
        custom_stats: &custom,
        allocated_tree_nodes: &tree,
        tree_socketed: &sockets,
        main_skill_id: None,
        enemy_conditions: &bools,
        player_conditions: &bools,
        skill_projectiles: &projectiles,
        enemy_resistances: &numbers,
        proc_toggles: &toggles,
        kills_per_sec: 0.0,
        entity_rates: &numbers,
        stack_counts: &empty_ranks,
        granted_skill_ranks: None,
        difficulty: None,
    })
}

#[test]
fn item_cast_resolves_its_skill_for_every_player_class() {
    for class in data::data().skills_by_class.keys() {
        let active = winters_bite_for_class(class, &[], &[], true);
        let disabled = winters_bite_for_class(class, &[], &[], false);
        assert!(active.proc_dps_max > 0.0, "Winter's Bite for {class}");
        assert_eq!(disabled.proc_dps_max, 0.0);
        assert_eq!(active.attributes, disabled.attributes);
        assert_eq!(active.stats, disabled.stats);
    }
}

#[test]
fn foreign_item_cast_uses_player_damage_but_not_foreign_subtree_points() {
    let base = winters_bite_for_class("butcher", &[], &[], true);
    let stronger = winters_bite_for_class("butcher", &[], &[("cold_skill_damage", "100")], true);
    assert!(stronger.proc_dps_max > base.proc_dps_max);
    let foreign_nodes = winters_bite_for_class(
        "butcher",
        &[
            ("breath_of_ice", "fresh_mint", 5),
            ("breath_of_ice", "frosted_throat", 3),
        ],
        &[],
        true,
    );
    assert_eq!(foreign_nodes.proc_dps_max, base.proc_dps_max);
}

#[test]
fn explicit_item_cast_level_is_not_increased_by_skill_rank_bonuses() {
    for class in ["jotunn", "butcher"] {
        let base = winters_bite_for_class(class, &[], &[], true);
        let ranks = winters_bite_for_class(
            class,
            &[],
            &[("all_skills", "50"), ("cold_skills", "20")],
            true,
        );
        let rank = ranks
            .calculation()
            .iter()
            .find(|step| step.label() == "Item proc Winter's Bite / Breath of Ice · Effective rank")
            .expect("explicit item cast has a damage breakdown");
        assert_eq!(rank.value(), (60.0, 60.0), "{class}");
        assert_eq!(base.proc_dps_max, ranks.proc_dps_max, "{class}");
    }
}

fn target_and_direct(
    class: &str,
    main: &str,
    target: &str,
    nodes: &[(&str, &str, u32)],
    custom: &[(&str, &str)],
    projectile_override: Option<u32>,
) -> (SkillDamageBreakdown, SkillDamageBreakdown) {
    let (_, proc, direct) =
        target_and_direct_with_stats(class, main, target, nodes, custom, projectile_override);
    (proc, direct)
}

fn target_and_direct_with_stats(
    class: &str,
    main: &str,
    target: &str,
    nodes: &[(&str, &str, u32)],
    custom: &[(&str, &str)],
    projectile_override: Option<u32>,
) -> (ComputedStats, SkillDamageBreakdown, SkillDamageBreakdown) {
    let (computed, proc, direct) =
        target_and_direct_with_performance(class, main, target, nodes, custom, projectile_override);
    (computed, proc, direct.damage.unwrap())
}

fn target_and_direct_with_performance(
    class: &str,
    main: &str,
    target: &str,
    nodes: &[(&str, &str, u32)],
    custom: &[(&str, &str)],
    projectile_override: Option<u32>,
) -> (ComputedStats, SkillDamageBreakdown, BuildPerformance) {
    let empty_ranks = HashMap::new();
    let inventory = Inventory::new();
    let ranks = HashMap::from([(main.to_string(), 10), (target.to_string(), 10)]);
    let subskills = nodes
        .iter()
        .map(|(skill, node, rank)| (subskill_key(skill, node), *rank))
        .collect();
    let bools = HashMap::new();
    let custom = custom
        .iter()
        .map(|(key, value)| CustomStat {
            stat_key: (*key).into(),
            value: (*value).into(),
        })
        .collect::<Vec<_>>();
    let tree = HashSet::new();
    let sockets = HashMap::new();
    let projectiles = projectile_override
        .map(|count| HashMap::from([(target.to_string(), count)]))
        .unwrap_or_default();
    let numbers = HashMap::new();
    let deps = BuildPerformanceDeps {
        class_id: Some(class),
        level: 50,
        allocated_attrs: &empty_ranks,
        inventory: &inventory,
        skill_ranks: &ranks,
        subskill_ranks: &subskills,
        active_aura_id: None,
        active_buffs: &bools,
        custom_stats: &custom,
        allocated_tree_nodes: &tree,
        tree_socketed: &sockets,
        main_skill_id: Some(main),
        enemy_conditions: &bools,
        player_conditions: &bools,
        skill_projectiles: &projectiles,
        enemy_resistances: &numbers,
        proc_toggles: &bools,
        kills_per_sec: 0.0,
        entity_rates: &numbers,
        stack_counts: &empty_ranks,
        granted_skill_ranks: None,
        difficulty: None,
    };
    let all_class_skills = data::get_skills_by_class(class);
    let by_name = all_class_skills
        .iter()
        .map(|s| (normalize_skill_name(&s.name), skill_spec_to_calc_skill(s)))
        .collect();
    let ranks_by_name = all_class_skills
        .iter()
        .map(|s| {
            (
                normalize_skill_name(&s.name),
                ranks.get(&s.id).copied().unwrap_or(0) as f64,
            )
        })
        .collect();
    let target_spec = all_class_skills.iter().find(|s| s.id == target).unwrap();
    let computed = compute_stats_for_skill(&deps, Some(main));
    let empty_scoped = StatMap::new();
    let ctx = ProcContext {
        computed: &computed,
        deps,
        skill_ranks_by_name: &ranks_by_name,
        skills_by_name: &by_name,
        all_class_skills,
        empty_scoped: &empty_scoped,
    };
    let proc = proc_target_damage(&ctx, &normalize_skill_name(&target_spec.name), None).unwrap();
    let direct_deps = BuildPerformanceDeps {
        main_skill_id: Some(target),
        ..deps
    };
    let direct = compute_build_performance(&direct_deps);
    (computed, proc, direct)
}

fn same_damage(proc: &SkillDamageBreakdown, direct: &SkillDamageBreakdown) {
    assert_eq!(
        (proc.hit_min, proc.hit_max),
        (direct.hit_min, direct.hit_max)
    );
    assert_eq!(
        (proc.avg_min, proc.avg_max),
        (direct.avg_min, direct.avg_max)
    );
    assert_eq!(
        (proc.flat_min, proc.flat_max),
        (direct.flat_min, direct.flat_max)
    );
    assert_eq!(proc.projectile_count, direct.projectile_count);
    assert_eq!(proc.crit_multiplier_avg, direct.crit_multiplier_avg);
}

#[test]
fn proc_target_does_not_inherit_the_main_skills_damage_nodes() {
    let (base, _) = target_and_direct(
        "stormweaver",
        "lightning_surge",
        "storm_cloud",
        &[],
        &[],
        None,
    );
    let (boosted_main, direct) = target_and_direct(
        "stormweaver",
        "lightning_surge",
        "storm_cloud",
        &[("lightning_surge", "amplified_storm", 5)],
        &[],
        None,
    );
    same_damage(&boosted_main, &direct);
    same_damage(&boosted_main, &base);
}

#[test]
fn proc_target_keeps_damage_taken_effect_in_its_own_scope() {
    let (base, _) = target_and_direct("samurai", "shuriken_throw", "smoke_bomb", &[], &[], None);
    let (proc, direct) = target_and_direct(
        "samurai",
        "shuriken_throw",
        "smoke_bomb",
        &[("smoke_bomb", "adjusted_formula", 2)],
        &[],
        None,
    );
    same_damage(&proc, &direct);
    assert!(proc.hit_max > base.hit_max);
    assert_eq!(
        proc.calculation()
            .iter()
            .find(|s| s.label() == "Enemy damage taken multiplier")
            .unwrap()
            .value(),
        (1.4, 1.4)
    );
}

#[test]
fn proc_target_resolves_its_own_conversion_crit_and_projectiles() {
    let (proc, direct) = target_and_direct(
        "white_mage",
        "shadow_bolt",
        "mana_orb",
        &[
            ("mana_orb", "burst_of_energy", 5),
            ("mana_orb", "energy_orbit", 1),
        ],
        &[("to_energy", "200"), ("spell_crit_damage", "100")],
        None,
    );
    same_damage(&proc, &direct);
    assert!(proc.flat_max > 0.0);
    assert!(proc.crit_chance > 0.0);
    assert!(proc.projectile_count > 1);
    assert!(proc
        .calculation()
        .iter()
        .any(|s| s.label() == "conversion_energy"));
}

#[test]
fn proc_target_retains_signed_total_damage_penalties() {
    let (base, _) = target_and_direct("demonspawn", "bone_fragments", "bone_storm", &[], &[], None);
    let (proc, direct) = target_and_direct(
        "demonspawn",
        "bone_fragments",
        "bone_storm",
        &[("bone_storm", "everlasting_storm", 2)],
        &[],
        None,
    );
    same_damage(&proc, &direct);
    assert_eq!(proc.extra_damage_pct, -60.0);
    assert!(proc.hit_max < base.hit_max);
}

#[test]
fn proc_target_honors_base_projectiles_and_explicit_overrides() {
    let (proc, direct) =
        target_and_direct("jotunn", "frozen_boulder", "frost_sunder", &[], &[], None);
    same_damage(&proc, &direct);
    assert_eq!(proc.projectile_count, 4);
    let (proc, direct) = target_and_direct(
        "jotunn",
        "frozen_boulder",
        "frost_sunder",
        &[],
        &[],
        Some(2),
    );
    same_damage(&proc, &direct);
    assert_eq!(proc.projectile_count, 2);
}

#[test]
fn self_target_proc_reuses_own_context_without_doubling_its_subtree() {
    let (base, _) = target_and_direct("pyromancer", "fireball", "fireball", &[], &[], None);
    let (proc, direct) = target_and_direct(
        "pyromancer",
        "fireball",
        "fireball",
        &[("fireball", "critical_burn", 5)],
        &[],
        None,
    );
    same_damage(&proc, &direct);
    assert_eq!(proc.hit_max, base.hit_max);
    assert!((proc.avg_max as f64 - base.avg_max as f64 * 1.3).abs() < 1.3);
}

#[test]
fn target_transformation_uses_its_own_tags_for_damage_and_multicast() {
    let (proc, direct) = target_and_direct(
        "amazon",
        "leaping_ambush",
        "death_from_above",
        &[("death_from_above", "ancient_device", 1)],
        &[("sentry_damage", "100"), ("multicast_chance", "100")],
        None,
    );
    same_damage(&proc, &direct);
    assert_eq!(proc.multicast_multiplier, 1.0);
}

#[test]
fn granted_proc_excludes_main_subtree_damage_but_retains_global_buffs() {
    let run = |blender_node: bool, unholy: bool| {
        let empty_ranks = HashMap::new();
        let inventory = Inventory::new();
        let ranks = HashMap::from([("blender".into(), 20), ("unholy_form".into(), 1)]);
        let subskills = if blender_node {
            HashMap::from([(subskill_key("blender", "it_will_blend"), 5)])
        } else {
            HashMap::new()
        };
        let buffs = HashMap::from([("unholy_form".into(), unholy)]);
        let conditions = HashMap::from([("bleeding".into(), true)]);
        let bools = HashMap::new();
        let custom = Vec::new();
        let tree = HashSet::new();
        let sockets = HashMap::new();
        let projectiles = HashMap::new();
        let numbers = HashMap::new();
        let toggles = HashMap::from([("granted:the_eye".into(), true)]);
        let granted = HashMap::from([("the eye".into(), (10.0, 20.0))]);
        let deps = BuildPerformanceDeps {
            class_id: Some("butcher"),
            level: 50,
            allocated_attrs: &empty_ranks,
            inventory: &inventory,
            skill_ranks: &ranks,
            subskill_ranks: &subskills,
            active_aura_id: None,
            active_buffs: &buffs,
            custom_stats: &custom,
            allocated_tree_nodes: &tree,
            tree_socketed: &sockets,
            main_skill_id: Some("blender"),
            enemy_conditions: &conditions,
            player_conditions: &bools,
            skill_projectiles: &projectiles,
            enemy_resistances: &numbers,
            proc_toggles: &toggles,
            kills_per_sec: 0.0,
            entity_rates: &numbers,
            stack_counts: &empty_ranks,
            granted_skill_ranks: Some(&granted),
            difficulty: None,
        };
        compute_build_performance(&deps)
    };
    let base = run(false, false);
    let node = run(true, false);
    assert_eq!(node.proc_dps_min, base.proc_dps_min);
    assert_eq!(node.proc_dps_max, base.proc_dps_max);
    assert!(node.avg_hit_dps_max > base.avg_hit_dps_max);
    let unholy = run(true, true);
    assert!((unholy.proc_dps_min / base.proc_dps_min - 1.2).abs() < 1e-9);
    assert!((unholy.proc_dps_max / base.proc_dps_max - 1.2).abs() < 1e-9);
}

fn damage_trace(damage: &SkillDamageBreakdown, label: &str) -> Ranged {
    damage
        .calculation()
        .iter()
        .find(|step| step.label() == label)
        .unwrap_or_else(|| panic!("missing damage trace: {label}"))
        .value()
}

fn close_range(actual: Ranged, expected: Ranged) {
    for (actual, expected) in [(actual.0, expected.0), (actual.1, expected.1)] {
        assert!(
            (actual - expected).abs() <= 1e-9 * expected.abs().max(1.0),
            "expected {expected}, got {actual}"
        );
    }
}

fn scale_range(value: Ranged, factor: f64) -> Ranged {
    (value.0 * factor, value.1 * factor)
}

fn assert_scoped_keys_do_not_leak(computed: &ComputedStats, keys: &[&str]) {
    for key in keys {
        assert!(!computed.stats.contains_key(*key), "stats leaked {key}");
        assert!(
            !computed.stats_combined.contains_key(*key),
            "combined stats leaked {key}"
        );
        assert!(
            !computed.stat_sources.contains_key(*key),
            "shared sources leaked {key}"
        );
        for overrides in computed.skill_stat_overrides.values() {
            assert!(!overrides.contains_key(*key), "stat override leaked {key}");
        }
    }
}

#[test]
fn elemental_subtree_pools_scale_the_whole_hit_after_generic_bonuses() {
    for (class, skill, first, second, elemental_key, subtree, intercept) in [
        (
            "pyromancer",
            "fireball",
            "inferno_ball",
            "splitfire",
            "fire_skill_damage",
            70.0,
            4.0,
        ),
        (
            "stormweaver",
            "storm_bolt",
            "bolts_of_devastation",
            "pulsating_bolt",
            "lightning_skill_damage",
            65.0,
            8.0,
        ),
    ] {
        let custom = [(elemental_key, "100"), ("magic_skill_damage", "50")];
        let (base_stats, _, base) =
            target_and_direct_with_stats(class, skill, skill, &[], &custom, None);
        let (computed, proc, direct) = target_and_direct_with_stats(
            class,
            skill,
            skill,
            &[(skill, first, 2), (skill, second, 1)],
            &custom,
            None,
        );
        same_damage(&proc, &direct);
        assert_scoped_keys_do_not_leak(&computed, &["subtree_damage", "subtree_damage_on_proc"]);
        close_range(
            computed.skill_scoped[skill]["subtree_damage"],
            (subtree, subtree),
        );
        assert_eq!(
            computed.stats.get(elemental_key),
            base_stats.stats.get(elemental_key)
        );
        assert_eq!(
            computed.stats.get("magic_skill_damage"),
            base_stats.stats.get("magic_skill_damage")
        );
        let generic = damage_trace(&direct, "Increased skill damage multiplier");
        assert_eq!(
            generic,
            damage_trace(&base, "Increased skill damage multiplier")
        );
        let synergy = damage_trace(&direct, "Synergy multiplier");
        let scaled = damage_trace(&direct, "Damage after synergy and generic bonuses");
        close_range(
            scaled,
            (
                intercept + (direct.base_min - intercept + direct.flat_min) * synergy.0 * generic.0,
                intercept + (direct.base_max - intercept + direct.flat_max) * synergy.1 * generic.1,
            ),
        );
        assert_eq!(
            scaled,
            damage_trace(&base, "Damage after synergy and generic bonuses")
        );
        let factor = 1.0 + subtree / 100.0;
        close_range(
            damage_trace(&direct, "Own subtree damage multiplier"),
            (factor, factor),
        );
        let generic_rounded = damage_trace(&base, "Damage after generic helper rounding");
        assert_eq!(
            damage_trace(&direct, "Damage after generic helper rounding"),
            generic_rounded,
        );
        let before_cast_floor = scale_range(generic_rounded, factor);
        assert_eq!(
            damage_trace(&direct, "Hit before rounding"),
            (before_cast_floor.0.floor(), before_cast_floor.1.floor()),
        );
    }
}

#[test]
fn elemental_fireball_proc_pool_weights_rank_and_chance_only_in_average_damage() {
    let guaranteed = [
        ("fireball", "inferno_ball", 2),
        ("fireball", "splitfire", 1),
    ];
    let custom = [("fire_skill_damage", "100"), ("spell_crit_chance", "20")];
    let (_, baseline) = target_and_direct(
        "pyromancer",
        "fireball",
        "fireball",
        &guaranteed,
        &custom,
        None,
    );
    for (impact, doom, flies, critical) in [(1, 1, 1, 0), (3, 2, 2, 2), (5, 5, 3, 5)] {
        let nodes = [
            guaranteed[0],
            guaranteed[1],
            ("fireball", "paralyzing_impact", impact),
            ("fireball", "mark_of_doom", doom),
            ("fireball", "fireflies", flies),
            ("fireball", "critical_burn", critical),
        ];
        let (computed, proc, direct) = target_and_direct_with_stats(
            "pyromancer",
            "fireball",
            "fireball",
            &nodes,
            &custom,
            None,
        );
        let weighted = 35.0 * impact as f64 * (3.0 * impact as f64 / 100.0)
            + 50.0 * doom as f64 * (doom as f64 / 100.0)
            + 60.0 * flies as f64 * (20.0 * flies as f64 / 100.0);
        close_range(
            computed.skill_scoped["fireball"]["subtree_damage"],
            (70.0, 70.0),
        );
        close_range(
            computed.skill_scoped["fireball"]["subtree_damage_on_proc"],
            (weighted, weighted),
        );
        assert_scoped_keys_do_not_leak(
            &computed,
            &[
                "subtree_damage",
                "subtree_damage_on_proc",
                "double_damage_chance",
                "secondary_projectile_count",
            ],
        );
        same_damage(&proc, &direct);
        assert_eq!(
            (direct.hit_min, direct.hit_max),
            (baseline.hit_min, baseline.hit_max)
        );
        assert_eq!(
            (direct.crit_min, direct.crit_max),
            (baseline.crit_min, baseline.crit_max)
        );
        assert_eq!(
            direct.projectile_count, 1,
            "secondary Fireflies count must not multiply primary casts"
        );
        assert_eq!(
            damage_trace(&direct, "Increased skill damage multiplier"),
            damage_trace(&baseline, "Increased skill damage multiplier")
        );
        let expected_pool = 1.0 + (70.0 + weighted) / 100.0;
        close_range(
            damage_trace(&direct, "Expected own subtree damage multiplier"),
            (expected_pool, expected_pool),
        );
        let expected_hit = scale_range(
            damage_trace(&baseline, "Damage after generic helper rounding"),
            expected_pool,
        );
        close_range(
            damage_trace(&direct, "Expected damage before critical hits"),
            expected_hit,
        );
        let double = 1.0 + 6.0 * critical as f64 / 100.0;
        close_range(
            damage_trace(&direct, "Double damage expectation"),
            (double, double),
        );
        let average_factor = direct.crit_multiplier_avg * double * direct.multicast_multiplier;
        assert_eq!(
            (direct.avg_min, direct.avg_max),
            (
                (expected_hit.0 * average_factor).floor() as i64,
                (expected_hit.1 * average_factor).floor() as i64
            ),
        );
    }
}

#[test]
fn elemental_secondary_damage_coefficients_do_not_increase_the_primary_hit() {
    for (class, skill, nodes, keys) in [
        (
            "pyromancer",
            "fireball",
            vec![("fireball", "path_of_destruction", 3)],
            vec!["trail_damage_percent"],
        ),
        (
            "stormweaver",
            "storm_bolt",
            vec![
                ("storm_bolt", "unleash_discharge", 5),
                ("storm_bolt", "bouncing_charge", 3),
                ("storm_bolt", "storm_claw", 3),
                ("storm_bolt", "magnetize", 3),
            ],
            vec![
                "discharge_damage",
                "bounce_damage_increased",
                "surge_damage",
                "secondary_damage_increased",
                "secondary_projectile_count",
            ],
        ),
    ] {
        let (base_stats, _, base) =
            target_and_direct_with_stats(class, skill, skill, &[], &[], None);
        let (computed, proc, direct) =
            target_and_direct_with_stats(class, skill, skill, &nodes, &[], None);
        same_damage(&proc, &direct);
        same_damage(&direct, &base);
        assert_scoped_keys_do_not_leak(&computed, &keys);
        for key in [
            "fire_skill_damage",
            "lightning_skill_damage",
            "magic_skill_damage",
        ] {
            assert_eq!(computed.stats.get(key), base_stats.stats.get(key));
        }
        close_range(
            damage_trace(&direct, "Own subtree damage multiplier"),
            (1.0, 1.0),
        );
        close_range(
            damage_trace(&direct, "Expected own subtree damage multiplier"),
            (1.0, 1.0),
        );
    }
}

#[test]
fn elemental_main_subtree_pools_cannot_leak_into_a_different_proc_target() {
    for (class, main, target, nodes) in [
        (
            "pyromancer",
            "fireball",
            "comet",
            vec![
                ("fireball", "inferno_ball", 5),
                ("fireball", "splitfire", 3),
                ("fireball", "paralyzing_impact", 5),
                ("fireball", "mark_of_doom", 5),
                ("fireball", "fireflies", 3),
                ("fireball", "critical_burn", 5),
            ],
        ),
        (
            "stormweaver",
            "storm_bolt",
            "lightning_surge",
            vec![
                ("storm_bolt", "bolts_of_devastation", 5),
                ("storm_bolt", "pulsating_bolt", 3),
            ],
        ),
    ] {
        let (base, _) = target_and_direct(class, main, target, &[], &[], None);
        let (computed, proc, direct) =
            target_and_direct_with_stats(class, main, target, &nodes, &[], None);
        assert_scoped_keys_do_not_leak(&computed, &["subtree_damage", "subtree_damage_on_proc"]);
        same_damage(&proc, &base);
        same_damage(&proc, &direct);
        close_range(
            damage_trace(&proc, "Own subtree damage multiplier"),
            (1.0, 1.0),
        );
        close_range(
            damage_trace(&proc, "Expected own subtree damage multiplier"),
            (1.0, 1.0),
        );
    }
}

#[test]
fn elemental_target_subtree_is_applied_once_for_other_and_self_proc_contexts() {
    for (class, target, other, nodes, guaranteed, weighted) in [
        (
            "pyromancer",
            "fireball",
            "comet",
            vec![
                ("fireball", "inferno_ball", 2),
                ("fireball", "splitfire", 1),
                ("fireball", "paralyzing_impact", 1),
                ("fireball", "mark_of_doom", 1),
                ("fireball", "fireflies", 1),
            ],
            70.0,
            13.55,
        ),
        (
            "stormweaver",
            "storm_bolt",
            "lightning_surge",
            vec![
                ("storm_bolt", "bolts_of_devastation", 2),
                ("storm_bolt", "pulsating_bolt", 1),
            ],
            65.0,
            0.0,
        ),
    ] {
        // Keep the secondary skill's rank in both contexts so synergy stays fixed.
        let (from_other, direct) = target_and_direct(class, other, target, &nodes, &[], None);
        same_damage(&from_other, &direct);
        for damage in [&from_other, &direct] {
            close_range(
                damage_trace(damage, "Own subtree damage multiplier"),
                (1.0 + guaranteed / 100.0, 1.0 + guaranteed / 100.0),
            );
            let expected = 1.0 + (guaranteed + weighted) / 100.0;
            close_range(
                damage_trace(damage, "Expected own subtree damage multiplier"),
                (expected, expected),
            );
        }
        let (from_self, self_direct) = target_and_direct(class, target, target, &nodes, &[], None);
        same_damage(&from_self, &self_direct);
        assert_eq!(
            damage_trace(&from_self, "Own subtree damage multiplier"),
            damage_trace(&from_other, "Own subtree damage multiplier")
        );
        assert_eq!(
            damage_trace(&from_self, "Expected own subtree damage multiplier"),
            damage_trace(&from_other, "Expected own subtree damage multiplier")
        );
    }
}

#[test]
fn elemental_storm_bolt_random_projectiles_preserve_fractional_average_and_contacts() {
    let (_, _, base) = target_and_direct_with_performance(
        "stormweaver",
        "storm_bolt",
        "storm_bolt",
        &[],
        &[],
        None,
    );
    let baseline = base.damage.as_ref().unwrap();
    let contacts = |build: &BuildPerformance| {
        build
            .calculation()
            .iter()
            .find(|step| step.label() == "Proc trigger contacts per second")
            .unwrap()
            .value()
    };
    for (tiny, quantity) in [(1, 0), (0, 1), (1, 1), (3, 2), (5, 5)] {
        let nodes = [
            ("storm_bolt", "tiny_storms", tiny),
            ("storm_bolt", "quantity_of_power", quantity),
        ];
        let (computed, proc, build) = target_and_direct_with_performance(
            "stormweaver",
            "storm_bolt",
            "storm_bolt",
            &nodes,
            &[],
            None,
        );
        let direct = build.damage.as_ref().unwrap();
        let additional = 0.05 * (tiny * tiny) as f64 + 0.04 * quantity as f64;
        close_range(
            computed.skill_scoped["storm_bolt"]["expected_additional_projectiles"],
            (additional, additional),
        );
        assert_scoped_keys_do_not_leak(&computed, &["expected_additional_projectiles"]);
        same_damage(&proc, direct);
        assert_eq!(direct.projectile_count, baseline.projectile_count);
        assert_eq!(
            (direct.hit_min, direct.hit_max),
            (baseline.hit_min, baseline.hit_max)
        );
        assert_eq!(
            (direct.crit_min, direct.crit_max),
            (baseline.crit_min, baseline.crit_max)
        );
        assert_eq!(
            damage_trace(direct, "Extra damage multiplier"),
            damage_trace(baseline, "Extra damage multiplier")
        );
        let expected_count = 1.0 + additional;
        close_range(
            damage_trace(direct, "Expected projectiles per cast"),
            (expected_count, expected_count),
        );
        let expected_hit = damage_trace(baseline, "Expected damage before critical hits");
        let factor = direct.crit_multiplier_avg * direct.multicast_multiplier * expected_count;
        assert_eq!(
            (direct.avg_min, direct.avg_max),
            (
                (expected_hit.0 * factor).floor() as i64,
                (expected_hit.1 * factor).floor() as i64
            ),
        );
        close_range(
            contacts(&build),
            scale_range(contacts(&base), expected_count),
        );
    }
}

#[test]
fn elemental_storm_bolt_random_projectiles_use_only_the_proc_targets_scope() {
    let nodes = [
        ("storm_bolt", "tiny_storms", 3),
        ("storm_bolt", "quantity_of_power", 2),
    ];
    let (base_other, _) = target_and_direct(
        "stormweaver",
        "storm_bolt",
        "lightning_surge",
        &[],
        &[],
        None,
    );
    let (other, _) = target_and_direct(
        "stormweaver",
        "storm_bolt",
        "lightning_surge",
        &nodes,
        &[],
        None,
    );
    same_damage(&other, &base_other);
    let (own, direct) = target_and_direct(
        "stormweaver",
        "lightning_surge",
        "storm_bolt",
        &nodes,
        &[],
        None,
    );
    same_damage(&own, &direct);
    close_range(
        damage_trace(&own, "Expected projectiles per cast"),
        (1.53, 1.53),
    );
    assert_eq!(own.projectile_count, 1);
}

#[test]
fn elemental_fireball_orbital_count_replaces_base_without_adding_secondary_counts() {
    let (_, baseline) = target_and_direct("pyromancer", "fireball", "fireball", &[], &[], None);
    for rank in 1..=3 {
        for override_count in [None, Some(7)] {
            let nodes = [
                ("fireball", "orbital_fire", rank),
                ("fireball", "splitfire", 3),
                ("fireball", "fireflies", 3),
            ];
            let (computed, proc, direct) = target_and_direct_with_stats(
                "pyromancer",
                "fireball",
                "fireball",
                &nodes,
                &[],
                override_count,
            );
            same_damage(&proc, &direct);
            assert_eq!(direct.projectile_count, 4 * rank);
            close_range(
                damage_trace(&direct, "Expected projectiles per cast"),
                (4.0 * rank as f64, 4.0 * rank as f64),
            );
            close_range(
                computed.skill_scoped["fireball"]["primary_projectile_count"],
                (4.0 * rank as f64, 4.0 * rank as f64),
            );
            assert_scoped_keys_do_not_leak(
                &computed,
                &["primary_projectile_count", "secondary_projectile_count"],
            );
            // Splitfire's guaranteed 120% scales each hit; its nine children and
            // the Fireflies children never inflate this primary orbital count.
            close_range(
                damage_trace(&direct, "Own subtree damage multiplier"),
                (2.2, 2.2),
            );
            let before_cast_floor = scale_range(
                damage_trace(&baseline, "Damage after generic helper rounding"),
                2.2,
            );
            assert_eq!(
                damage_trace(&direct, "Hit before rounding"),
                (before_cast_floor.0.floor(), before_cast_floor.1.floor()),
            );
        }
    }
}
