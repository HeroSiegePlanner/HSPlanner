use super::*;

struct Build {
    class: &'static str,
    main: Option<&'static str>,
    inventory: Inventory,
    ranks: HashMap<String, u32>,
    subs: HashMap<String, u32>,
    stats: Vec<CustomStat>,
    rates: HashMap<String, f64>,
    procs: HashMap<String, bool>,
    projectiles: HashMap<String, u32>,
    kills: f64,
    enemy_conditions: HashMap<String, bool>,
}

impl Build {
    fn new(class: &'static str, skill: &'static str) -> Self {
        Self {
            class,
            main: Some(skill),
            inventory: HashMap::new(),
            ranks: [(skill.to_string(), 10)].into_iter().collect(),
            subs: HashMap::new(),
            // Cadence fixtures isolate speed/cooldown formulas without gear.
            stats: vec![CustomStat {
                stat_key: "skill_restrictions_removed".into(),
                value: "100".into(),
            }],
            rates: HashMap::new(),
            procs: HashMap::new(),
            projectiles: HashMap::new(),
            kills: 0.0,
            enemy_conditions: HashMap::new(),
        }
    }

    fn stat(&mut self, key: &str, value: &str) {
        self.stats.push(CustomStat {
            stat_key: key.into(),
            value: value.into(),
        });
    }

    fn enable_subproc(&mut self, skill: &str, sub: &str, rank: u32) {
        let key = subskill_key(skill, sub);
        self.subs.insert(key.clone(), rank);
        self.procs.insert(key, true);
    }

    fn run(&self) -> BuildPerformance {
        compute_build_performance(&BuildPerformanceDeps {
            class_id: Some(self.class),
            level: 50,
            allocated_attrs: &HashMap::new(),
            inventory: &self.inventory,
            skill_ranks: &self.ranks,
            subskill_ranks: &self.subs,
            active_aura_id: None,
            active_buffs: &HashMap::new(),
            custom_stats: &self.stats,
            allocated_tree_nodes: &HashSet::new(),
            tree_socketed: &HashMap::new(),
            main_skill_id: self.main,
            enemy_conditions: &self.enemy_conditions,
            player_conditions: &HashMap::new(),
            skill_projectiles: &self.projectiles,
            enemy_resistances: &HashMap::new(),
            proc_toggles: &self.procs,
            kills_per_sec: self.kills,
            entity_rates: &self.rates,
            stack_counts: &HashMap::new(),
            granted_skill_ranks: None,
            difficulty: None,
        })
    }
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
}

#[test]
fn butcher_primary_attacks_preserve_the_neutral_weapon_packet() {
    for (id, base, slope, full_subtree) in [
        ("slicing_throw", 310.0, 30.0, true),
        ("furious_strike", 268.0, 28.0, false),
        ("chain_rip", 116.0, 24.0, true),
        ("brutalizing_slash", 130.0, 20.0, false),
        ("butchers_hook", 125.0, 25.0, true),
        ("chain_swing", 124.0, 26.0, true),
        ("submerged_knives", 142.0, 28.0, true),
    ] {
        let mut build = Build::new("butcher", id);
        build.inventory.insert(
            "weapon".into(),
            super::super::types::EquippedItem {
                base_id: "base_dagger_kris".into(),
                ..Default::default()
            },
        );
        let spec = data::get_skills_by_class("butcher")
            .iter()
            .find(|s| s.id == id)
            .unwrap();
        let mapped = skill_spec_to_calc_skill(spec).attack_scaling.unwrap();
        assert!(mapped.weapon_bonus_scaling);
        assert_eq!(mapped.weapon_subtree_scales_full_hit, full_subtree);
        for rank in [1, 10, 20] {
            build.ranks.insert(id.into(), rank);
            let attack = build.run().attack_damage.unwrap();
            close(attack.weapon_damage_pct_min, base + slope * rank as f64);
            close(attack.weapon_damage_pct_max, base + slope * rank as f64);
            close(attack.skill_flat_phys_min, 0.0);
            close(attack.skill_flat_phys_max, 0.0);
            assert!(attack.physical_hit_max > 0, "{id}, rank {rank}");
        }
    }
}

#[test]
fn butcher_primary_subtree_bonuses_use_their_own_additive_pool() {
    for (id, full_hit, nodes) in [
        (
            "slicing_throw",
            true,
            vec![
                ("cut_in_half", 10.),
                ("splitting_blade", 40.),
                ("slicing_vortex", 50.),
                ("bladesaw_massacre", 35.),
            ],
        ),
        (
            "furious_strike",
            false,
            vec![("strike_of_hate", 12.5), ("sick_to_the_core", 42.5)],
        ),
        (
            "chain_rip",
            true,
            vec![
                ("chains_of_hatred", 15.),
                ("chains_attached", 75.),
                ("chainfueled_hunting", 50.),
                ("carcass_rip", 35.),
            ],
        ),
        (
            "brutalizing_slash",
            false,
            vec![("bonebreaker", 10.), ("pummeling_slashwave", 22.5)],
        ),
        (
            "butchers_hook",
            true,
            vec![
                ("spin_of_death", 8.),
                ("master_of_meat_hooks", 15.),
                ("welcome_to_hell", 40.),
            ],
        ),
        (
            "chain_swing",
            true,
            vec![
                ("demonic_chains", 8.),
                ("prey_of_the_day", 15.),
                ("centrifugal_force", 125.),
            ],
        ),
        (
            "submerged_knives",
            true,
            vec![("agravating_wounds", 11.), ("tidal_knives", 15.)],
        ),
    ] {
        let mut build = Build::new("butcher", id);
        build.stat("physical_skill_damage", "40");
        build.inventory.insert(
            "weapon".into(),
            super::super::types::EquippedItem {
                base_id: "base_dagger_kris".into(),
                ..Default::default()
            },
        );
        let baseline = build.run().attack_damage.unwrap();
        assert!(baseline.physical_hit_max > 0);
        let step = |attack: &AttackSkillDamageBreakdown, label: &str| {
            attack
                .calculation()
                .iter()
                .find(|s| s.label() == label)
                .unwrap()
                .value()
                .1
        };
        let neutral = step(&baseline, "Skill weapon multiplier");
        let mut pool = 0.;
        for (node, per_rank) in nodes {
            build.subs.insert(subskill_key(id, node), 2);
            pool += per_rank * 2.;
            let attack = build.run().attack_damage.unwrap();
            close(
                step(&attack, "Physical skill multiplier"),
                step(&baseline, "Physical skill multiplier"),
            );
            close(
                step(&attack, "Skill weapon multiplier"),
                if full_hit {
                    neutral * (1. + pool / 100.)
                } else {
                    1. + (neutral - 1.) * (1. + pool / 100.)
                },
            );
        }
        build.main = Some("blender");
        build.ranks.insert("blender".into(), 10);
        let other = build.run().attack_damage.unwrap();
        build.subs.clear();
        let other_baseline = build.run().attack_damage.unwrap();
        assert!(other_baseline.physical_hit_max > 0);
        assert_eq!(
            other.physical_hit_max, other_baseline.physical_hit_max,
            "{id}"
        );
    }
}

#[test]
fn butcher_dexterity_scales_its_weapon_bonus_without_adding_flat_damage() {
    let mut build = Build::new("butcher", "brutalizing_slash");
    build.stat("dexterity", "100-300");
    build.inventory.insert(
        "weapon".into(),
        super::super::types::EquippedItem {
            base_id: "base_dagger_kris".into(),
            ..Default::default()
        },
    );
    build
        .subs
        .insert(subskill_key("brutalizing_slash", "bonebreaker"), 2);
    let baseline = build.run();
    build.subs.insert(
        subskill_key("brutalizing_slash", "precision_equals_power"),
        3,
    );
    let boosted = build.run();
    assert_eq!(boosted.attributes, baseline.attributes);
    assert_eq!(boosted.stats, baseline.stats);
    let dex = boosted.attributes["dexterity"];
    let attack = boosted.attack_damage.unwrap();
    assert!(attack.physical_hit_max > 0);
    let value = |attack: &AttackSkillDamageBreakdown, label: &str| {
        attack
            .calculation()
            .iter()
            .find(|s| s.label() == label)
            .unwrap()
            .value()
    };
    let factor = value(&attack, "Skill weapon multiplier");
    close(
        factor.0,
        1. + 2.3 * (1. + 0.2 + (dex.0 * 0.01) * (18. * 0.01)),
    );
    close(
        factor.1,
        1. + 2.3 * (1. + 0.2 + (dex.1 * 0.01) * (18. * 0.01)),
    );
    close(value(&attack, "Converted flat physical damage").1, 0.);
    assert_eq!(
        value(&attack, "Physical base"),
        value(&baseline.attack_damage.unwrap(), "Physical base")
    );
}

#[test]
fn butcher_conditional_subtrees_preserve_ordinary_hits_and_shared_stats() {
    for (id, ordinary_node, conditional_node, conditional_rank, ordinary_mult, expected_mult) in [
        (
            "furious_strike",
            "strike_of_hate",
            "blade_uppercut",
            5,
            6.6,
            8.84,
        ),
        (
            "brutalizing_slash",
            "bonebreaker",
            "meet_daisy",
            3,
            3.76,
            4.91,
        ),
    ] {
        let mut build = Build::new("butcher", id);
        build.inventory.insert(
            "weapon".into(),
            super::super::types::EquippedItem {
                base_id: "base_dagger_kris".into(),
                ..Default::default()
            },
        );
        build.stat("crit_chance", "45");
        build.stat("crit_damage_more", "50");
        build.subs.insert(subskill_key(id, ordinary_node), 2);
        let baseline = build.run();
        build
            .subs
            .insert(subskill_key(id, conditional_node), conditional_rank);
        let boosted = build.run();
        assert_eq!(boosted.stats, baseline.stats, "{conditional_node}");
        assert_eq!(
            boosted.attributes, baseline.attributes,
            "{conditional_node}"
        );
        let plain = baseline.attack_damage.unwrap();
        let attack = boosted.attack_damage.unwrap();
        assert!(plain.physical_hit_max > 0);
        assert_eq!(attack.physical_hit_min, plain.physical_hit_min);
        assert_eq!(attack.physical_hit_max, plain.physical_hit_max);
        assert_eq!(attack.crit_damage_pct, plain.crit_damage_pct);
        assert!(attack.physical_avg_max > plain.physical_avg_max);
        if conditional_node == "meet_daisy" {
            assert_eq!(
                attack
                    .calculation()
                    .iter()
                    .find(|s| s.label() == "Daisy size on every third attack")
                    .unwrap()
                    .value(),
                (1.5, 2.25)
            );
            assert_eq!(attack.projectile_count, plain.projectile_count);
        }
        for (label, expected) in [
            ("Skill weapon multiplier", ordinary_mult),
            ("Expected weapon multiplier", expected_mult),
        ] {
            close(
                attack
                    .calculation()
                    .iter()
                    .find(|s| s.label() == label)
                    .unwrap()
                    .value()
                    .1,
                expected,
            );
        }
        build.procs.insert(subskill_key(id, conditional_node), true);
        assert_eq!(
            build.run().attack_damage.unwrap().physical_avg_max,
            attack.physical_avg_max
        );
        build.main = Some("blender");
        build.ranks.insert("blender".into(), 10);
        let other = build.run().attack_damage.unwrap();
        build.subs.clear();
        assert_eq!(
            other.physical_avg_max,
            build.run().attack_damage.unwrap().physical_avg_max
        );
    }
}
#[test]
fn butcher_local_critical_nodes_preserve_shared_stats_and_ordinary_hits() {
    for (id, node, per_rank) in [
        ("slicing_throw", "one_swift_strike", 20.),
        ("brutalizing_slash", "skullsplitter", 30.),
        ("butchers_hook", "immaculate_butcher", 30.),
        ("chain_swing", "entry_frenzy", 20.),
        ("blender", "obsidian_blades", 32.5),
    ] {
        let mut build = Build::new("butcher", id);
        build.inventory.insert(
            "weapon".into(),
            super::super::types::EquippedItem {
                base_id: "base_dagger_kris".into(),
                ..Default::default()
            },
        );
        build.stat("crit_chance", "45");
        build.stat("crit_damage", "100");
        build.stat("crit_damage_more", "50");
        let baseline = build.run();
        let plain = baseline.attack_damage.as_ref().unwrap();
        let critical = |attack: &AttackSkillDamageBreakdown| {
            attack
                .calculation()
                .iter()
                .find(|s| s.label() == "Critical damage multiplier")
                .unwrap()
                .value()
        };
        for rank in [1, 3, 5] {
            build.subs.insert(subskill_key(id, node), rank);
            let boosted = build.run();
            let mut shared = boosted.stats.clone();
            shared.remove("projectile_speed");
            assert_eq!(shared, baseline.stats, "{node}/{rank}");
            let attack = boosted.attack_damage.as_ref().unwrap();
            assert_eq!(attack.physical_hit_min, plain.physical_hit_min, "{node}");
            assert_eq!(attack.physical_hit_max, plain.physical_hit_max, "{node}");
            close(
                attack.crit_damage_pct,
                plain.crit_damage_pct + per_rank * rank as f64,
            );
            let expected = critical(plain);
            let actual = critical(attack);
            close(actual.0, expected.0 + per_rank * rank as f64 / 100.);
            close(actual.1, expected.1 + per_rank * rank as f64 / 100.);
            assert!(attack.physical_avg_max > plain.physical_avg_max, "{node}");
        }
        build.main = Some("furious_strike");
        build.ranks.insert("furious_strike".into(), 10);
        let other = build.run().attack_damage.unwrap();
        build.subs.clear();
        assert_eq!(
            other.physical_avg_max,
            build.run().attack_damage.unwrap().physical_avg_max,
            "{node}"
        );
    }
}
#[test]
fn blender_variants_inherit_obsidian_critical_points_once() {
    for (nano, traveling) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut build = Build::new("butcher", "blender");
        build.stat("crit_chance", "45");
        build.stat("crit_damage_more", "50");
        if nano {
            build
                .subs
                .insert(subskill_key("blender", "a_i_empowered_nanoblenders"), 1);
        }
        if traveling {
            build.subs.insert(subskill_key("blender", "blenderang"), 1);
        }
        let baseline = build.run();
        build
            .subs
            .insert(subskill_key("blender", "obsidian_blades"), 3);
        let boosted = build.run();
        let plain = baseline.attack_damage.unwrap();
        let attack = boosted.attack_damage.unwrap();
        assert_eq!(attack.physical_hit_max, plain.physical_hit_max);
        assert_eq!(attack.projectile_count, plain.projectile_count);
        close(
            attack.crit_multiplier_avg - plain.crit_multiplier_avg,
            0.5 * 97.5 / 100.,
        );
        assert!(boosted.avg_hit_dps_max.unwrap() > baseline.avg_hit_dps_max.unwrap());
        assert_eq!(boosted.stats, baseline.stats);
    }
}
#[test]
fn hunger_for_blood_requires_a_bleeding_target_and_uses_finalized_global_points() {
    let mut build = Build::new("butcher", "furious_strike");
    let baseline = build.run();
    for (rank, expected) in [(1, 1.), (2, 2.), (3, 3.), (4, 5.)] {
        build.ranks.insert("hunger_for_blood".into(), rank);
        build.enemy_conditions.clear();
        let off = build.run();
        assert_eq!(off.stats["extra_damage_bleeding"], (expected, expected));
        assert_eq!(
            off.attack_damage.as_ref().unwrap().physical_hit_max,
            baseline.attack_damage.as_ref().unwrap().physical_hit_max
        );
        build.enemy_conditions.insert("bleeding".into(), true);
        let on = build.run().attack_damage.unwrap();
        assert_eq!(
            on.extra_damage_sources
                .iter()
                .find(|s| s.stat_key == Some("extra_damage_bleeding"))
                .unwrap()
                .pct,
            expected
        );
    }
}

#[test]
fn butcher_bleeding_subtrees_add_fractional_local_points_after_the_global_floor() {
    for (id, node, local) in [
        ("brutalizing_slash", "bleeding_the_prey", 15.),
        ("chain_swing", "scent_of_prey", 10.),
        ("blender", "it_will_blend", 17.5),
    ] {
        let mut build = Build::new("butcher", id);
        build.ranks.insert("hunger_for_blood".into(), 1);
        build.stat("extra_damage_bleeding", "0.75");
        let baseline = build.run();
        build.subs.insert(subskill_key(id, node), 1);
        let off = build.run();
        assert_eq!(off.stats, baseline.stats, "{node}");
        assert_eq!(off.stats["extra_damage_bleeding"], (2., 2.));
        assert_eq!(
            off.attack_damage.as_ref().unwrap().physical_hit_max,
            baseline.attack_damage.as_ref().unwrap().physical_hit_max
        );
        build.enemy_conditions.insert("bleeding".into(), true);
        let on = build.run().attack_damage.unwrap();
        let source = on
            .extra_damage_sources
            .iter()
            .find(|s| s.stat_key == Some("skill_extra_damage_bleeding"))
            .unwrap();
        assert_eq!(source.pct, local);
        assert_eq!(
            on.extra_damage_sources
                .iter()
                .find(|s| s.stat_key == Some("extra_damage_bleeding"))
                .unwrap()
                .pct,
            2.
        );
        assert!(
            on.physical_avg_max > off.attack_damage.unwrap().physical_avg_max,
            "{node}"
        );
        build.main = Some("slicing_throw");
        build.ranks.insert("slicing_throw".into(), 10);
        let other = build.run().attack_damage.unwrap();
        build.subs.clear();
        assert_eq!(
            other.physical_avg_max,
            build.run().attack_damage.unwrap().physical_avg_max,
            "{node}"
        );
    }
}
#[test]
fn blade_strike_rank_does_not_scale_its_noncritical_weapon_packet() {
    for armed in [false, true] {
        let mut build = Build::new("nomad", "blade_strike");
        if armed {
            build.inventory.insert(
                "weapon".into(),
                super::super::types::EquippedItem {
                    base_id: "gun_satanic_warbringer".into(),
                    ..Default::default()
                },
            );
        }
        let mut ordinary_hit = None;
        for rank in [1, 5, 20] {
            build.ranks.insert("blade_strike".into(), rank);
            let result = build.run();
            let attack = result.attack_damage.unwrap();
            close(attack.weapon_damage_pct_min, 100.);
            close(attack.weapon_damage_pct_max, 100.);
            close(attack.skill_flat_phys_min, 0.);
            close(attack.skill_flat_phys_max, 0.);
            let hit = (attack.physical_hit_min, attack.physical_hit_max);
            assert!(hit.1 > 0);
            if let Some(expected) = ordinary_hit {
                assert_eq!(hit, expected, "rank {rank}, armed {armed}");
            }
            ordinary_hit = Some(hit);
        }
    }
}

#[test]
fn blade_strike_critical_chance_is_added_to_its_own_hit_only() {
    let mut build = Build::new("nomad", "blade_strike");
    build.stat("crit_chance", "10");
    for rank in [1, 5, 20] {
        build.ranks.insert("blade_strike".into(), rank);
        let blade = build.run();
        let attack = blade.attack_damage.unwrap();
        close(blade.stats["crit_chance"].1, 15.);
        close(attack.crit_chance, 65.);
    }
    build.main = Some("flying_scimitar");
    build.ranks.insert("flying_scimitar".into(), 10);
    let other = build.run();
    close(other.stats["crit_chance"].1, 15.);
    close(other.attack_damage.unwrap().crit_chance, 15.);
}

#[test]
fn blade_strike_intrinsic_critical_damage_scales_only_its_own_hit() {
    let mut build = Build::new("nomad", "blade_strike");
    build.stat("crit_damage_more", "50");
    for rank in [1, 5, 20] {
        build.ranks.insert("blade_strike".into(), rank);
        let blade = build.run();
        let global_damage = blade.stats["crit_damage"].1;
        let intrinsic = 67.5 + 22.5 * rank as f64;
        let attack = blade.attack_damage.unwrap();
        close(attack.crit_damage_pct, global_damage + intrinsic);
        let factor = attack
            .calculation()
            .iter()
            .find(|step| step.label() == "Critical damage multiplier")
            .unwrap()
            .value()
            .1;
        close(factor, (1. + global_damage / 100.) * 1.5 + intrinsic / 100.);
    }
    build.main = Some("flying_scimitar");
    build.ranks.insert("flying_scimitar".into(), 10);
    let other = build.run();
    close(
        other.attack_damage.unwrap().crit_damage_pct,
        other.stats["crit_damage"].1,
    );
}

#[test]
fn blade_critical_subtree_scales_local_damage_without_global_or_flat_leaks() {
    for (edge, giant) in [(5, 0), (0, 3), (5, 3)] {
        let mut build = Build::new("nomad", "blade_strike");
        build.inventory.insert(
            "weapon".into(),
            super::super::types::EquippedItem {
                base_id: "base_dagger_kris".into(),
                ..Default::default()
            },
        );
        build.stat("vitality", "100-300");
        build.stat("all_skills", "0-2");
        build.stat("crit_damage_more", "50");
        let baseline = build.run();
        build
            .subs
            .insert(subskill_key("blade_strike", "blades_edge"), edge);
        build
            .subs
            .insert(subskill_key("blade_strike", "giant_slayer"), giant);
        let boosted = build.run();
        assert_eq!(boosted.stats, baseline.stats);
        assert_eq!(boosted.attributes, baseline.attributes);
        let plain = baseline.attack_damage.unwrap();
        let attack = boosted.attack_damage.unwrap();
        assert_eq!(attack.physical_hit_min, plain.physical_hit_min);
        assert_eq!(attack.physical_hit_max, plain.physical_hit_max);
        assert_eq!(attack.skill_flat_phys_min, 0.);
        assert_eq!(attack.skill_flat_phys_max, 0.);
        let vitality = boosted.attributes["vitality"];
        let factor = attack
            .calculation()
            .iter()
            .find(|step| step.label() == "Critical damage multiplier")
            .unwrap()
            .value();
        let global = (1. + boosted.stats["crit_damage"].1 / 100.) * 1.5;
        for (actual, rank, vitality) in [(factor.0, 10., vitality.0), (factor.1, 12., vitality.1)] {
            let intrinsic = (67.5 + 22.5 * rank)
                * (1. + edge as f64 * 5. / 100. + vitality * giant as f64 * 3. / 10000.);
            close(actual, global + intrinsic / 100.);
        }
        assert!(attack.physical_avg_max > plain.physical_avg_max);

        build.main = Some("flying_scimitar");
        build.ranks.insert("flying_scimitar".into(), 10);
        let other = build.run();
        build.subs.clear();
        let other_baseline = build.run();
        assert_eq!(other.stats, other_baseline.stats);
        let other = other.attack_damage.unwrap();
        let other_baseline = other_baseline.attack_damage.unwrap();
        assert_eq!(other.crit_damage_pct, other_baseline.crit_damage_pct);
        assert_eq!(other.physical_hit_max, other_baseline.physical_hit_max);
        assert_eq!(other.physical_avg_max, other_baseline.physical_avg_max);
    }
}

#[test]
fn ferocious_swipe_adds_local_chance_after_global_scaling() {
    let mut build = Build::new("nomad", "blade_strike");
    build.stat("crit_chance", "10");
    let baseline = build.run();
    build
        .subs
        .insert(subskill_key("blade_strike", "ferocious_swipe"), 5);
    let blade = build.run();
    close(
        blade.stats["crit_chance"].1,
        baseline.stats["crit_chance"].1,
    );
    close(blade.attack_damage.as_ref().unwrap().crit_chance, 90.);
    assert_eq!(
        blade.attack_damage.as_ref().unwrap().physical_hit_max,
        baseline.attack_damage.as_ref().unwrap().physical_hit_max
    );
    assert!(
        blade.attack_damage.as_ref().unwrap().physical_avg_max
            > baseline.attack_damage.as_ref().unwrap().physical_avg_max
    );
    build.main = Some("flying_scimitar");
    build.ranks.insert("flying_scimitar".into(), 10);
    let other = build.run();
    close(other.stats["crit_chance"].1, 15.);
    close(other.attack_damage.unwrap().crit_chance, 15.);
}

#[test]
fn blade_master_multiplies_only_blades_intrinsic_attack_rating() {
    let mut build = Build::new("nomad", "blade_strike");
    build.stat("attack_rating_pct", "25");
    let baseline = build.run();
    build
        .subs
        .insert(subskill_key("blade_strike", "blade_master"), 5);
    let blade = build.run();
    assert_eq!(
        blade.stats.get("attack_rating_pct"),
        baseline.stats.get("attack_rating_pct")
    );
    let plain = baseline.attack_damage.unwrap();
    let boosted = blade.attack_damage.unwrap();
    close(
        boosted.attack_rating_pct_max - plain.attack_rating_pct_max,
        25.,
    );
    assert_eq!(boosted.physical_hit_max, plain.physical_hit_max);
    assert_eq!(boosted.physical_avg_max, plain.physical_avg_max);
    build.main = Some("flying_scimitar");
    build.ranks.insert("flying_scimitar".into(), 10);
    let other = build.run();
    build.subs.clear();
    let other_baseline = build.run();
    assert_eq!(
        other.stats.get("attack_rating_pct"),
        other_baseline.stats.get("attack_rating_pct")
    );
    assert_eq!(
        other.attack_damage.unwrap().attack_rating_pct_max,
        other_baseline.attack_damage.unwrap().attack_rating_pct_max
    );
}

#[test]
fn global_cast_and_attack_procs_use_audited_event_types() {
    let all = data::data();
    let mut count = 0;
    for skill in all.skills_by_class.values().flatten() {
        assert!(skill.activation_kind.is_some(), "{}", skill.id);
        count += 1;
    }
    assert_eq!(count, 432, "update the executable audit when skills change");
    for (class, id, cast_rate, attack_rate) in [
        ("butcher", "blender", (1., 2.), (0., 0.)),
        ("amazon", "death_from_above", (0., 0.), (1., 2.)),
        ("viking", "seismic_slam", (0., 0.), (1., 2.)),
        ("amazon", "noxious_strike", (0., 0.), (1., 2.)),
        ("demon_slayer", "trigger_finger", (0., 0.), (1., 2.)),
        ("marauder", "heavy_ball", (0., 0.), (1., 2.)),
        ("marksman", "trickshot", (0., 0.), (1., 2.)),
        ("nomad", "sand_carver", (0., 0.), (1., 2.)),
        ("paladin", "vengeance", (0., 0.), (1., 2.)),
        ("pirate", "buckshot", (0., 0.), (1., 2.)),
        (
            "plague_doctor",
            "surgical_blood_letting",
            (0., 0.),
            (1., 2.),
        ),
        ("samurai", "quickslash", (0., 0.), (1., 2.)),
        ("shield_lancer", "glorious_strike", (0., 0.), (1., 2.)),
        ("butcher", "holy_form", (0., 0.), (0., 0.)),
        ("butcher", "unholy_form", (0., 0.), (0., 0.)),
    ] {
        let spec = data::get_skills_by_class(class)
            .iter()
            .find(|skill| skill.id == id)
            .unwrap();
        for (trigger, expected) in [
            ("on_cast", cast_rate),
            ("on_attack", attack_rate),
            ("on_hit", (3., 6.)),
            ("on_kill", (9., 9.)),
        ] {
            assert_eq!(
                global_proc_trigger_rate(trigger, spec.activation_kind, (1., 2.), (3., 6.), 9.),
                expected,
                "{id}/{trigger}"
            );
        }
    }
}

#[test]
fn dragon_knight_proc_uses_activation_type_instead_of_damage_or_tags() {
    let item = "helmet_heroic_dragon_knight_s_helmet";
    for (class, main, can_trigger) in [
        ("amazon", "death_from_above", true),
        ("viking", "seismic_slam", true),
        ("amazon", "noxious_strike", true),
        ("butcher", "blender", false),
        ("pyromancer", "fireball", false),
        ("butcher", "holy_form", false),
        ("butcher", "unholy_form", false),
    ] {
        let mut build = Build::new(class, main);
        build.inventory.insert(
            "helmet".into(),
            super::super::types::EquippedItem {
                base_id: item.into(),
                ..Default::default()
            },
        );
        let toggle = item_cast_toggle_key(item, "breath of fire");
        build.procs.insert(toggle.clone(), true);
        let result = build.run();
        assert_eq!(result.proc_dps_max > 0., can_trigger, "{class}/{main}");
        let proc = result
            .calculation()
            .iter()
            .find(|step| step.label() == "Item proc · Dragon Knight's Helmet / Breath of Fire")
            .expect("the helmet's explicit rank cast must be modeled");
        assert!(proc.expression().contains("rank 125"));
        close(proc.value().1, result.proc_dps_max);
        build.procs.insert(toggle.clone(), false);
        close(build.run().proc_dps_max, 0.);
        build.procs.insert(toggle, true);
        build.ranks.insert(main.into(), 0);
        close(build.run().proc_dps_max, 0.);
    }
}

#[test]
fn circle_of_slugs_separates_waves_from_angular_projectiles() {
    for rank in 1..=3 {
        let mut build = Build::new("pirate", "buckshot");
        build
            .subs
            .insert(subskill_key("buckshot", "circle_of_slugs"), rank);
        let result = build.run();
        let waves = 1.0 + 2.0 * rank as f64;
        let damage = result.attack_damage.as_ref().unwrap();
        close(damage.attacks_per_second_max, 1.0 / 5.25);
        close(
            result.skill_costs["buckshot"].cast_rate_max.unwrap(),
            1.0 / 5.25,
        );
        assert_eq!(damage.projectile_count, 1);
        close(
            result.avg_hit_dps_max.unwrap(),
            damage.combined_avg_max as f64 * waves / 5.25,
        );
        let contacts = result
            .calculation()
            .iter()
            .find(|step| step.label() == "Proc trigger contacts per second")
            .unwrap();
        close(contacts.value().1, waves / 5.25);
        assert!(result
            .calculation()
            .iter()
            .any(|step| step.label() == "Circle of Slugs contact estimate"));
        assert!(!result.stats.contains_key("wave_count"));
        assert!(!result.stats.contains_key("cooldown_added"));
    }
}

#[test]
fn circle_of_slugs_cooldown_uses_haste_and_survives_other_transformations() {
    let mut build = Build::new("pirate", "buckshot");
    let ordinary = build.run().attack_damage.unwrap().attacks_per_second_max;
    assert!(ordinary > 0.2);
    build
        .subs
        .insert(subskill_key("buckshot", "circle_of_slugs"), 1);
    let base = build.run().avg_hit_dps_max.unwrap();
    build.stat("increased_attack_speed", "100");
    close(build.run().avg_hit_dps_max.unwrap(), base);
    build.stat("skill_haste", "100");
    close(build.run().avg_hit_dps_max.unwrap(), base * 1.5);
    for node in ["big_slugs", "one_slug_to_end_them_all", "slick_slugs"] {
        build.subs.insert(subskill_key("buckshot", node), 1);
        close(
            build.run().attack_damage.unwrap().attacks_per_second_max,
            1.5 / 5.25,
        );
    }
    build
        .subs
        .remove(&subskill_key("buckshot", "circle_of_slugs"));
    assert!(build.run().attack_damage.unwrap().attacks_per_second_max > 0.4);
}

#[test]
fn attack_entities_use_their_own_rate_and_ignore_player_attack_speed() {
    for (class, skill, kind) in [
        ("nomad", "phantom_blade", "sentry"),
        ("bard", "moshpit_massacre", "summon"),
    ] {
        let mut build = Build::new(class, skill);
        build.rates.insert(kind.into(), 1.0);
        let base = build.run().avg_hit_dps_max.unwrap();
        build.rates.insert(kind.into(), 3.0);
        close(build.run().avg_hit_dps_max.unwrap(), base * 3.0);
        build.stat("increased_attack_speed", "100");
        close(build.run().avg_hit_dps_max.unwrap(), base * 3.0);
    }
}

#[test]
fn cooldown_attack_uses_metadata_and_haste_instead_of_weapon_speed() {
    let mut build = Build::new("samurai", "omnislash");
    let cooldown = data::get_skills_by_class("samurai")
        .iter()
        .find(|s| s.id == "omnislash")
        .unwrap()
        .base_cooldown
        .unwrap();
    close(
        build.run().attack_damage.unwrap().attacks_per_second_max,
        1.0 / cooldown,
    );
    build.stat("increased_attack_speed", "100");
    close(
        build.run().attack_damage.unwrap().attacks_per_second_max,
        1.0 / cooldown,
    );
    build
        .subs
        .insert(subskill_key("omnislash", "no_hesitation"), 5);
    close(
        build.run().attack_damage.unwrap().attacks_per_second_max,
        1.175 / cooldown,
    );
}

#[test]
fn on_cast_subproc_tracks_cast_speed_but_not_lifetime_ticks() {
    let mut build = Build::new("stormweaver", "lightning_surge");
    build.enable_subproc("lightning_surge", "lightning_strikes_twice", 5);
    let base = build.run();
    let target = base.damage.as_ref().unwrap().avg_max as f64;
    close(base.proc_dps_max, target * 0.20);
    build.stat("faster_cast_rate", "100");
    close(build.run().proc_dps_max, base.proc_dps_max * 2.0);
}

#[test]
fn on_hit_subproc_counts_projectiles_and_repeated_contacts() {
    let mut build = Build::new("stormweaver", "lightning_surge");
    build.ranks.insert("storm_cloud".into(), 10);
    build.enable_subproc("lightning_surge", "storm_summoner", 5);
    let base = build.run();
    let trace = base
        .calculation()
        .iter()
        .find(|s| s.label() == "Proc trigger contacts per second")
        .unwrap();
    close(trace.value().1, 6.0);
    build.projectiles.insert("lightning_surge".into(), 3);
    close(build.run().proc_dps_max, base.proc_dps_max * 3.0);
    build.stat("faster_cast_rate", "100");
    close(build.run().proc_dps_max, base.proc_dps_max * 6.0);
}

#[test]
fn another_or_unlearned_skill_cannot_trigger_an_allocated_subtree() {
    let mut build = Build::new("stormweaver", "lightning_surge");
    build.enable_subproc("lightning_surge", "lightning_strikes_twice", 5);
    assert!(build.run().proc_dps_max > 0.0);
    build.ranks.insert("storm_cloud".into(), 10);
    build.main = Some("storm_cloud");
    close(build.run().proc_dps_max, 0.0);
    build.main = Some("lightning_surge");
    build.ranks.insert("lightning_surge".into(), 0);
    close(build.run().proc_dps_max, 0.0);
}

#[test]
fn on_kill_rate_stays_independent_of_hit_and_cast_cadence() {
    let mut build = Build::new("stormweaver", "lightning_surge");
    build.ranks.insert("aftershock".into(), 10);
    build.ranks.insert("apocalyptic_thunder".into(), 10);
    build.procs.insert("aftershock".into(), true);
    build.kills = 2.0;
    let base = build.run().proc_dps_max;
    assert!(base > 0.0);
    build.stat("faster_cast_rate", "100");
    close(build.run().proc_dps_max, base);
    build.kills = 4.0;
    close(build.run().proc_dps_max, base * 2.0);
}

#[test]
fn trigger_rates_preserve_both_roll_endpoints() {
    assert_eq!(
        proc_trigger_rate("on_hit", (1.0, 2.0), (3.0, 6.0), 9.0),
        (3.0, 6.0)
    );
    assert_eq!(
        proc_trigger_rate("on_cast", (1.0, 2.0), (3.0, 6.0), 9.0),
        (1.0, 2.0)
    );
    assert_eq!(
        proc_trigger_rate("on_kill", (1.0, 2.0), (3.0, 6.0), 9.0),
        (9.0, 9.0)
    );
}

#[test]
fn bone_altar_cast_procs_follow_player_recasts_not_sentry_attacks() {
    let mut build = Build::new("demonspawn", "bone_fragments");
    build.ranks.insert("blood_demons".into(), 10);
    build.procs.insert("blood_demons".into(), true);
    build
        .subs
        .insert(subskill_key("bone_fragments", "bone_altar"), 1);
    build.rates.insert("sentry".into(), 1.0);
    let base = build.run();
    assert!(base.proc_dps_max > 0.0);
    close(
        base.skill_costs["bone_fragments"].cast_rate_max.unwrap(),
        1.0 / 8.25,
    );
    let casts = base
        .calculation()
        .iter()
        .find(|step| step.label() == "Player skill uses per second")
        .unwrap();
    close(casts.value().1, 1.0 / 8.25);

    build.rates.insert("sentry".into(), 3.0);
    build.stat("sentry_attack_speed", "100");
    let faster_entity = build.run();
    assert!(faster_entity.avg_hit_dps_max > base.avg_hit_dps_max);
    close(faster_entity.proc_dps_max, base.proc_dps_max);
    build.stat("skill_haste", "100");
    close(build.run().proc_dps_max, base.proc_dps_max * 1.5);
    build.ranks.insert("bone_fragments".into(), 0);
    close(build.run().proc_dps_max, 0.0);
}

#[test]
fn haste_cannot_exceed_the_configured_attack_action_speed() {
    let mut spec = data::get_skills_by_class("samurai")
        .iter()
        .find(|s| s.id == "omnislash")
        .unwrap()
        .clone();
    spec.base_cooldown = Some(1.0);
    let stats = [
        ("attacks_per_second".into(), (2.0, 3.0)),
        ("skill_haste".into(), (1000.0, 1000.0)),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        attack_action_rate_override(&spec, &[], &stats, &HashMap::new(), &HashMap::new()),
        Some((2.0, 3.0))
    );
}

#[test]
fn ordinary_buckshot_uses_weapon_speed_without_an_inherited_four_per_second_cap() {
    let mut build = Build::new("pirate", "buckshot");
    build.stat("attacks_per_second", "10");
    let result = build.run();
    assert!(result.attack_damage.unwrap().attacks_per_second_min > 4.0);
    assert!(result.skill_costs["buckshot"].cast_rate_min.unwrap() > 4.0);
}

#[test]
fn attack_speed_spells_preserve_both_weapon_rate_endpoints_for_damage_and_cost() {
    for (class, skill) in [
        ("samurai", "explosive_kunai"),
        ("exo", "scorching_whip"),
        ("amazon", "death_from_above"),
    ] {
        let mut build = Build::new(class, skill);
        build.stat("attacks_per_second", "1-3");
        let result = build.run();
        let weapon_rate = result.stats["attacks_per_second"];
        assert!(weapon_rate.0 < weapon_rate.1);
        let actions = result
            .calculation()
            .iter()
            .find(|step| step.label() == "Actions per second")
            .unwrap()
            .value();
        assert_eq!(actions, weapon_rate, "{skill}");
        let cost = &result.skill_costs[skill];
        assert_eq!(
            (cost.cast_rate_min.unwrap(), cost.cast_rate_max.unwrap()),
            actions,
            "{skill}"
        );
        let damage = result.damage.unwrap();
        close(
            result.avg_hit_dps_min.unwrap(),
            damage.avg_min as f64 * actions.0,
        );
        close(
            result.avg_hit_dps_max.unwrap(),
            damage.avg_max as f64 * actions.1,
        );
    }
}

#[test]
fn declared_spell_cooldown_gates_casts_without_a_separate_haste_flag() {
    let mut build = Build::new("stormweaver", "storm_cloud");
    let spec = data::get_skills_by_class("stormweaver")
        .iter()
        .find(|s| s.id == "storm_cloud")
        .unwrap();
    assert!(!spec.uses_skill_haste);
    let rate = |result: &BuildPerformance| {
        result
            .calculation()
            .iter()
            .find(|s| s.label() == "Actions per second")
            .unwrap()
            .value()
            .1
    };
    let base = build.run();
    let cooldown = spec.base_cooldown.unwrap();
    close(
        rate(&base),
        spec.base_cast_rate.unwrap().min(1.0 / cooldown),
    );
    build.stat("faster_cast_rate", "100");
    close(
        rate(&build.run()),
        (spec.base_cast_rate.unwrap() * 2.0).min(1.0 / cooldown),
    );
    build.stat("skill_haste", "100");
    close(
        rate(&build.run()),
        (spec.base_cast_rate.unwrap() * 2.0).min(1.5 / cooldown),
    );
}
