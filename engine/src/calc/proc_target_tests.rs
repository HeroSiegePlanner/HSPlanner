use super::*;

struct Fixture<'a> {
    class: &'a str,
    main: &'a str,
    target: &'a str,
    rank_override: Option<f64>,
    weapon: Option<&'a str>,
    nodes: Vec<(&'a str, &'a str, u32)>,
    custom: Vec<(&'a str, &'a str)>,
    projectiles: Option<u32>,
    target_rank: u32,
    proc_enabled: bool,
}

impl<'a> Fixture<'a> {
    fn new(class: &'a str, main: &'a str, target: &'a str) -> Self {
        Self {
            class,
            main,
            target,
            rank_override: None,
            weapon: Some("gun_satanic_warbringer"),
            nodes: Vec::new(),
            custom: Vec::new(),
            projectiles: None,
            target_rank: 10,
            proc_enabled: false,
        }
    }

    fn run(&self) -> (ProcTargetDamage, BuildPerformance) {
        self.evaluate(false)
    }

    fn run_selected(&self) -> (ProcTargetDamage, BuildPerformance) {
        self.evaluate(true)
    }

    fn evaluate(&self, selected: bool) -> (ProcTargetDamage, BuildPerformance) {
        let ranks = HashMap::from([
            (self.main.into(), 10),
            (self.target.into(), self.target_rank),
        ]);
        let empty_ranks = HashMap::new();
        let inventory = self
            .weapon
            .map(|id| {
                HashMap::from([(
                    "weapon".into(),
                    super::super::types::EquippedItem {
                        base_id: id.into(),
                        ..Default::default()
                    },
                )])
            })
            .unwrap_or_default();
        let subs = self
            .nodes
            .iter()
            .map(|(skill, node, rank)| (subskill_key(skill, node), *rank))
            .collect();
        let mut custom = self
            .custom
            .iter()
            .map(|(key, value)| CustomStat {
                stat_key: (*key).into(),
                value: (*value).into(),
            })
            .collect::<Vec<_>>();
        custom.push(CustomStat {
            stat_key: "skill_restrictions_removed".into(),
            value: "100".into(),
        });
        let bools = HashMap::new();
        let tree = HashSet::new();
        let sockets = HashMap::new();
        let numbers = HashMap::new();
        let projectiles = self
            .projectiles
            .map(|count| HashMap::from([(self.target.into(), count)]))
            .unwrap_or_default();
        let target = data::data()
            .skills_by_class
            .values()
            .flatten()
            .find(|s| s.id == self.target)
            .unwrap();
        let toggles = self
            .weapon
            .map(|weapon| {
                HashMap::from([(
                    item_cast_toggle_key(weapon, &normalize_skill_name(&target.name)),
                    self.proc_enabled,
                )])
            })
            .unwrap_or_default();
        let deps = BuildPerformanceDeps {
            class_id: Some(self.class),
            level: 50,
            allocated_attrs: &empty_ranks,
            inventory: &inventory,
            skill_ranks: &ranks,
            subskill_ranks: &subs,
            active_aura_id: None,
            active_buffs: &bools,
            custom_stats: &custom,
            allocated_tree_nodes: &tree,
            tree_socketed: &sockets,
            main_skill_id: Some(self.main),
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
        };
        let all_class_skills = data::get_skills_by_class(self.class);
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
        let computed = compute_stats_for_skill(&deps, Some(self.main));
        let empty_scoped = StatMap::new();
        let ctx = ProcContext {
            computed: &computed,
            deps,
            skill_ranks_by_name: &ranks_by_name,
            skills_by_name: &by_name,
            all_class_skills,
            empty_scoped: &empty_scoped,
        };
        let proc = proc_target_damage(
            &ctx,
            &normalize_skill_name(&target.name),
            self.rank_override,
        )
        .expect("attack target has a payload");
        let direct = compute_build_performance(&BuildPerformanceDeps {
            main_skill_id: Some(if selected { self.main } else { self.target }),
            ..deps
        });
        (proc, direct)
    }
}

fn attack(target: &ProcTargetDamage) -> &AttackSkillDamageBreakdown {
    match target {
        ProcTargetDamage::Attack(attack) => attack,
        _ => panic!("expected physical/hybrid attack target"),
    }
}

fn same_attack(target: &ProcTargetDamage, direct: &BuildPerformance) {
    let expected = direct.attack_damage.as_ref().unwrap();
    assert_eq!(target.avg_min(), expected.combined_avg_min);
    assert_eq!(target.avg_max(), expected.combined_avg_max);
    assert_eq!(attack(target).physical_hit_min, expected.physical_hit_min);
    assert_eq!(attack(target).physical_hit_max, expected.physical_hit_max);
}

#[test]
fn butcher_secondary_coefficients_preserve_primary_packets_and_other_skills() {
    for (skill, node, coefficients) in [
        (
            "butchers_hook",
            "concussive_blow",
            [
                ("secondary_damage_per_captured_enemy", 2.5),
                ("secondary_size_per_captured_enemy", 1.25),
            ],
        ),
        (
            "chain_swing",
            "hookstorm",
            [
                ("secondary_damage_increased", 30.0),
                ("secondary_range_increased", 15.0),
            ],
        ),
    ] {
        for rank in 1..=3 {
            let mut fixture = Fixture::new("butcher", skill, skill);
            fixture.weapon = None;
            fixture.custom.push(("physical_skill_damage", "50"));
            let (_, baseline) = fixture.run();
            fixture.nodes.push((skill, node, rank));
            let (target, changed) = fixture.run();
            same_attack(&target, &changed);
            let a = baseline.attack_damage.as_ref().unwrap();
            let b = changed.attack_damage.as_ref().unwrap();
            assert_eq!(b.physical_hit_min, a.physical_hit_min, "{node} r{rank}");
            assert_eq!(b.physical_hit_max, a.physical_hit_max, "{node} r{rank}");
            assert_eq!(b.physical_avg_max, a.physical_avg_max, "{node} r{rank}");
            // Hookstorm's own contacts come from the dps_cards dummy model.
            if node == "hookstorm" {
                assert!(changed.avg_hit_dps_max > baseline.avg_hit_dps_max);
            } else {
                assert_eq!(changed.avg_hit_dps_max, baseline.avg_hit_dps_max);
            }
            assert_eq!(
                changed.stats, baseline.stats,
                "primary stats changed for {node}"
            );
            assert_eq!(changed.stats["physical_skill_damage"], (50.0, 50.0));
            for (key, per_rank) in coefficients {
                let step = changed
                    .calculation()
                    .iter()
                    .find(|step| step.label() == format!("Subtree · {}", key.replace('_', " ")))
                    .unwrap_or_else(|| panic!("missing {key}"));
                assert_eq!(
                    step.value(),
                    (per_rank * rank as f64, per_rank * rank as f64)
                );
                assert!(!changed.stats.contains_key(key), "leaked {key}");
            }
            assert!(changed.calculation().iter().any(|step| {
                step.label().starts_with("Model note ·")
                    && step.expression().contains("excluded from displayed DPS")
            }));

            fixture.main = "slicing_throw";
            fixture.target = "slicing_throw";
            let (_, other) = fixture.run();
            fixture.nodes.clear();
            let (_, other_baseline) = fixture.run();
            assert_eq!(other.stats, other_baseline.stats);
            assert_eq!(other.avg_hit_dps_max, other_baseline.avg_hit_dps_max);
        }
    }
}

#[test]
fn butcher_local_rating_nodes_scale_intrinsic_rank_ranges_before_shared_bonus() {
    for (skill, node, local_crit_per_rank) in [
        ("brutalizing_slash", "bucthering_skill", 3.0),
        ("submerged_knives", "sharp_edges", 0.0),
    ] {
        for rank in 1..=5 {
            let mut fixture = Fixture::new("butcher", skill, skill);
            fixture.weapon = None;
            fixture.custom.extend([
                ("all_skills", "0-2"),
                ("attack_rating_pct", "25"),
                ("crit_chance", "10"),
                ("crit_damage_more", "200"),
            ]);
            let (_, baseline) = fixture.run();
            fixture.nodes.push((skill, node, rank));
            let (target, changed) = fixture.run();
            same_attack(&target, &changed);
            let a = baseline.attack_damage.as_ref().unwrap();
            let b = changed.attack_damage.as_ref().unwrap();
            assert_eq!((b.effective_rank_min, b.effective_rank_max), (10.0, 12.0));
            let multiplier = 1.0 + 0.10 * rank as f64;
            assert_eq!(
                b.attack_rating_pct_min,
                50.0 * multiplier + 25.0,
                "{node} r{rank}"
            );
            assert_eq!(
                b.attack_rating_pct_max,
                60.0 * multiplier + 25.0,
                "{node} r{rank}"
            );
            assert_eq!(
                b.crit_chance,
                a.crit_chance + local_crit_per_rank * rank as f64
            );
            assert_eq!(b.physical_hit_max, a.physical_hit_max);
            assert_eq!(changed.stats, baseline.stats);

            fixture.main = "slicing_throw";
            let (other, _) = fixture.run();
            assert_eq!(
                attack(&other).attack_rating_pct_max,
                b.attack_rating_pct_max
            );
            assert_eq!(attack(&other).crit_chance, b.crit_chance);
        }
    }
}

#[test]
fn physical_proc_target_uses_equipped_weapon_and_existing_attack_formula() {
    let mut fixture = Fixture::new("demon_slayer", "bullet_hell", "trigger_finger");
    let (armed, direct) = fixture.run();
    same_attack(&armed, &direct);
    assert!(armed.avg_max() > 0);
    fixture.weapon = None;
    let (unarmed, direct) = fixture.run();
    same_attack(&unarmed, &direct);
    assert!(armed.avg_max() > unarmed.avg_max());
}

#[test]
fn hybrid_proc_target_counts_physical_elemental_and_conversion_once() {
    let mut fixture = Fixture::new("marksman", "multishot", "homing_missile");
    fixture.custom.push(("physical_to_fire", "30"));
    let (target, direct) = fixture.run();
    same_attack(&target, &direct);
    let result = attack(&target);
    assert!(result.physical_avg_max > 0);
    assert!(result.poison_avg_max > 0);
    let converted = result
        .converted_elements
        .iter()
        .map(|d| d.avg_max)
        .sum::<i64>();
    assert!(converted > 0);
    assert_eq!(
        target.avg_max(),
        result.physical_avg_max + result.poison_avg_max + converted
    );
    assert_eq!(result.poison_avg_max, direct.damage.unwrap().avg_max);
}

#[test]
fn physical_proc_target_keeps_its_own_subtree_and_projectile_count() {
    let mut fixture = Fixture::new("demon_slayer", "bullet_hell", "trigger_finger");
    let (plain, _) = fixture.run();
    fixture.nodes.push(("bullet_hell", "hellflame_bullets", 5));
    fixture.nodes.push(("bullet_hell", "more_bullets", 5));
    let (main_boost, direct) = fixture.run();
    same_attack(&main_boost, &direct);
    assert_eq!(main_boost.avg_max(), plain.avg_max());
    fixture.nodes.push(("trigger_finger", "triggerheavy", 5));
    fixture
        .nodes
        .push(("trigger_finger", "gunslinging_slayer", 2));
    let (target_boost, direct) = fixture.run();
    same_attack(&target_boost, &direct);
    assert!(target_boost.avg_max() > main_boost.avg_max());
    assert_eq!(attack(&target_boost).projectile_count, 3);
    fixture.projectiles = Some(3);
    let (extra, direct) = fixture.run();
    same_attack(&extra, &direct);
    assert_eq!(attack(&extra).projectile_count, 5);
}

#[test]
fn explicit_attack_proc_level_ignores_rank_bonuses_and_foreign_subtrees() {
    for class in ["demon_slayer", "butcher"] {
        let main = if class == "demon_slayer" {
            "bullet_hell"
        } else {
            "furious_strike"
        };
        let mut fixture = Fixture::new(class, main, "trigger_finger");
        fixture.rank_override = Some(8.0);
        let (plain, _) = fixture.run();
        fixture
            .custom
            .extend([("all_skills", "40"), ("projectile_skills", "20")]);
        let (boosted, _) = fixture.run();
        assert_eq!(attack(&boosted).effective_rank_min, 8.0);
        assert_eq!(attack(&boosted).effective_rank_max, 8.0);
        // Trigger Finger: W·(1 + V1/100), V1 = 35 + 15·L (game natives) → 135 + 15·8.
        assert_eq!(attack(&boosted).weapon_damage_pct_max, 255.0);
        assert_eq!(
            attack(&plain).weapon_damage_pct_max,
            attack(&boosted).weapon_damage_pct_max
        );
        if class == "butcher" {
            assert_eq!(plain.avg_max(), boosted.avg_max());
            fixture.nodes.push(("trigger_finger", "triggerheavy", 5));
            fixture
                .nodes
                .push(("trigger_finger", "gunslinging_slayer", 5));
            let (foreign_nodes, _) = fixture.run();
            assert_eq!(foreign_nodes.avg_max(), boosted.avg_max());
        }
    }
}

#[test]
fn explicit_hybrid_proc_level_is_shared_by_both_components() {
    let mut fixture = Fixture::new("butcher", "furious_strike", "homing_missile");
    fixture.rank_override = Some(80.0);
    fixture
        .custom
        .extend([("all_skills", "40"), ("arcane_skills", "20")]);
    let (target, _) = fixture.run();
    assert_eq!(attack(&target).effective_rank_max, 80.0);
    // Homing Missile: W·(1 + V2/100), V2 = 28·L (game natives) → 100 + 28·80.
    assert_eq!(attack(&target).weapon_damage_pct_max, 2340.0);
    let elemental_rank = target
        .calculation()
        .iter()
        .find(|step| step.label() == "Effective rank")
        .unwrap();
    assert_eq!(elemental_rank.value(), (80.0, 80.0));
}

#[test]
fn proc_payload_does_not_inherit_target_attack_cadence() {
    let mut fixture = Fixture::new("demon_slayer", "bullet_hell", "trigger_finger");
    let (plain, _) = fixture.run();
    fixture.custom.push(("increased_attack_speed", "250"));
    let (faster, direct) = fixture.run();
    assert_eq!(plain.avg_max(), faster.avg_max());
    assert_eq!(attack(&faster).attacks_per_second_max, 1.0);
    assert!(direct.attack_damage.unwrap().attacks_per_second_max > 1.0);
}

#[test]
fn hybrid_proc_target_honors_base_projectiles_and_explicit_overrides() {
    let mut fixture = Fixture::new("jotunn", "frozen_boulder", "frost_sunder");
    for (override_count, expected) in [(None, 4), (Some(2), 2)] {
        fixture.projectiles = override_count;
        let (target, direct) = fixture.run();
        same_attack(&target, &direct);
        assert_eq!(attack(&target).projectile_count, expected);
        assert_eq!(
            attack(&target).poison_avg_max,
            direct.damage.unwrap().avg_max
        );
    }
}

#[test]
fn every_active_attack_target_reuses_its_direct_base_payload() {
    let mut checked = 0;
    for (class, skills) in &data::data().skills_by_class {
        for skill in skills.iter().filter(|skill| {
            skill.kind == SkillKind::Active
                && matches!(
                    skill.attack_kind,
                    Some(super::super::types::AttackKindSpec::Attack)
                )
        }) {
            let fixture = Fixture::new(class, &skill.id, &skill.id);
            let (target, direct) = fixture.run();
            let direct = direct.attack_damage.as_ref().unwrap();
            assert_eq!(
                (target.avg_min(), target.avg_max()),
                (direct.combined_avg_min, direct.combined_avg_max),
                "{class}/{}",
                skill.id,
            );
            checked += 1;
        }
    }
    assert!(
        checked > 70,
        "expected coverage across active attack classes"
    );
}

#[test]
fn warbringer_casts_rank_eight_physical_damage_on_eligible_player_attacks() {
    let item = data::get_item("gun_satanic_warbringer").unwrap();
    let proc = &item.procs.as_ref().unwrap()[0];
    assert_eq!(proc.trigger, "on_attack");
    assert_eq!(proc.target.as_deref(), Some("Trigger Finger"));
    assert_eq!(proc.cast_level, Some(8));
    assert_eq!(proc.chance, 25.0);

    for (class, main, eligible) in [
        ("demon_slayer", "bullet_hell", true),
        ("marksman", "multishot", true),
        ("amazon", "death_from_above", true),
        ("pyromancer", "fireball", false),
    ] {
        let mut fixture = Fixture::new(class, main, "trigger_finger");
        fixture.rank_override = Some(8.0);
        fixture.target_rank = 0;
        fixture.proc_enabled = true;
        let (target, result) = fixture.run_selected();
        assert_eq!(result.proc_dps_max > 0.0, eligible, "{class}/{main}");
        let rank = result
            .calculation()
            .iter()
            .find(|step| {
                step.label() == "Item proc Warbringer / Trigger Finger · Effective attack rank"
            })
            .expect("equipped item proc exposes its physical payload");
        assert_eq!(rank.value(), (8.0, 8.0));
        let payload = result
            .calculation()
            .iter()
            .find(|step| {
                step.label() == "Item proc Warbringer / Trigger Finger · Average damage per swing"
            })
            .unwrap();
        assert_eq!(payload.value().1, target.avg_max() as f64);
        fixture.proc_enabled = false;
        assert_eq!(fixture.run_selected().1.proc_dps_max, 0.0);
        fixture.proc_enabled = true;
        fixture.weapon = None;
        assert_eq!(fixture.run_selected().1.proc_dps_max, 0.0);
    }
}

#[test]
fn warbringer_proc_uses_player_attack_rate_and_existing_item_cooldown_once() {
    let mut fixture = Fixture::new("marksman", "multishot", "trigger_finger");
    fixture.rank_override = Some(8.0);
    fixture.target_rank = 0;
    fixture.proc_enabled = true;
    let (target, result) = fixture.run_selected();
    let actions = result.attack_damage.as_ref().unwrap();
    for (actual, payload, rate) in [
        (
            result.proc_dps_min,
            target.avg_min(),
            actions.attacks_per_second_min,
        ),
        (
            result.proc_dps_max,
            target.avg_max(),
            actions.attacks_per_second_max,
        ),
    ] {
        let expected = payload as f64 * (rate * 0.25).min(1.0 / 1.5);
        assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
    }
    fixture.custom.push(("increased_attack_speed", "500"));
    let (faster_target, faster) = fixture.run_selected();
    assert_eq!(faster_target.avg_max(), target.avg_max());
    assert!((faster.proc_dps_max - target.avg_max() as f64 / 1.5).abs() < 1e-8);
    assert!(faster.proc_dps_max > result.proc_dps_max);
}

#[test]
fn warbringer_does_not_trigger_trigger_finger_from_itself() {
    let mut fixture = Fixture::new("demon_slayer", "trigger_finger", "trigger_finger");
    fixture.rank_override = Some(8.0);
    fixture.proc_enabled = true;
    let (_, result) = fixture.run_selected();
    assert!(result.avg_hit_dps_max.unwrap() > 0.0);
    assert_eq!(result.proc_dps_min, 0.0);
    assert_eq!(result.proc_dps_max, 0.0);
    fixture.main = "bullet_hell";
    assert!(fixture.run_selected().1.proc_dps_max > 0.0);
}
