use app_lib::calc::commands::{calc_build_performance, calc_build_stats, BuildPerformanceInput};
use app_lib::calc::types::{CustomStat, EquippedItem};

fn build(node_rank: u32, chainfueled: bool) -> BuildPerformanceInput {
    let mut input = BuildPerformanceInput {
        class_id: Some("butcher".into()),
        level: 50,
        main_skill_id: Some("chain_rip".into()),
        skill_ranks: [("chain_rip".into(), 10), ("slicing_throw".into(), 10)]
            .into_iter()
            .collect(),
        subskill_ranks: [("chain_rip:the_real_monster".into(), node_rank)]
            .into_iter()
            .collect(),
        inventory: [(
            "weapon".into(),
            EquippedItem {
                base_id: "base_dagger_kris".into(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        custom_stats: [
            ("life_replenish_more", "200"),
            ("mana_replenish_increased", "200"),
            ("life", "901-1101"),
            ("mana", "703-903"),
        ]
        .into_iter()
        .map(|(key, value)| CustomStat {
            stat_key: key.into(),
            value: value.into(),
        })
        .collect(),
        ..Default::default()
    };
    if chainfueled {
        input
            .subskill_ranks
            .insert("chain_rip:chainfueled_hunting".into(), 1);
    }
    input
}

#[test]
fn real_monster_never_adds_permanent_regeneration_or_changes_another_skill() {
    for chainfueled in [false, true] {
        for main in ["chain_rip", "slicing_throw"] {
            let mut baseline_input = build(0, chainfueled);
            baseline_input.main_skill_id = Some(main.into());
            let baseline = calc_build_stats(baseline_input.clone());
            let baseline_performance = calc_build_performance(baseline_input.clone());
            for rank in [1, 2, 3, 5, 7] {
                let mut learned_input = baseline_input.clone();
                learned_input
                    .subskill_ranks
                    .insert("chain_rip:the_real_monster".into(), rank);
                let learned = calc_build_stats(learned_input.clone());
                assert_eq!(learned.stats, baseline.stats, "rank={rank}, main={main}");
                let actual = calc_build_performance(learned_input);
                assert_eq!(actual.avg_hit_dps_max, baseline_performance.avg_hit_dps_max);
                assert_eq!(
                    actual.skill_costs["chain_rip"].mana_regen_max,
                    baseline_performance.skill_costs["chain_rip"].mana_regen_max
                );
                for key in [
                    "skill_life_recovery_from_max_life_pct",
                    "skill_mana_recovery_from_max_mana_pct",
                ] {
                    assert_eq!(
                        learned.skill_scoped["chain_rip"][key],
                        (0.5 * f64::from(rank), 0.5 * f64::from(rank))
                    );
                    assert!(!learned.stats.contains_key(key));
                    assert!(learned
                        .skill_scoped
                        .get("slicing_throw")
                        .is_none_or(|stats| !stats.contains_key(key)));
                }
                if main == "slicing_throw" {
                    assert!(actual
                        .attack_damage
                        .as_ref()
                        .unwrap()
                        .calculation()
                        .iter()
                        .all(|step| !step.label().contains("recovery")));
                }
            }
        }
    }
}

#[test]
fn real_monster_details_show_only_conditional_recovery_inputs_and_mana_portion() {
    for chainfueled in [false, true] {
        for rank in [1, 2, 5, 7] {
            let actual = calc_build_performance(build(rank, chainfueled));
            let pct = 0.5 * f64::from(rank);
            let life = actual.stats["life"];
            let mana = actual.stats["mana"];
            let steps = actual.attack_damage.as_ref().unwrap().calculation();
            let step = |label| {
                steps
                    .iter()
                    .find(|step| step.label() == label)
                    .unwrap_or_else(|| panic!("missing {label}"))
            };
            let life_amount = step("Life recovery before blocking and caps");
            assert_eq!(
                life_amount.value(),
                (
                    (life.0 * (pct * 0.01)).floor(),
                    (life.1 * (pct * 0.01)).floor()
                )
            );
            assert!(life_amount.expression().contains("no recovery rate"));
            let mana_amount = (
                (mana.0 * (pct * 0.01)).floor(),
                (mana.1 * (pct * 0.01)).floor(),
            );
            assert_eq!(
                step("Mana recovery controller base amount").value(),
                mana_amount
            );
            assert_eq!(
                step("Mana recovery controller portion").value(),
                (mana_amount.0 / 8.0, mana_amount.1 / 8.0)
            );
            assert!(step("Mana recovery controller portion")
                .expression()
                .contains("existing mana controller"));
            assert!(actual
                .calculation()
                .iter()
                .any(|step| step.label() == "Model note · The Real Monster"
                    && step.expression().contains("total recovery per cast")));
        }
    }
    let inactive = calc_build_performance(build(0, false));
    assert!(inactive
        .attack_damage
        .as_ref()
        .unwrap()
        .calculation()
        .iter()
        .all(|step| !step.label().contains("recovery")));
}
