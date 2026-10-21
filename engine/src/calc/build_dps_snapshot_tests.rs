//! DPS snapshot of every active skill on a fixed reference build.
//!
//! Not an assertion test: run it to compare planner numbers before and after a
//! model change.
//! `HSP_DPS_SNAPSHOT=/path/out.csv cargo test -p HSPlanner dps_snapshot -- --ignored`

use super::*;

const CLASSES: [&str; 24] = [
    "amazon", "bard", "butcher", "demon_slayer", "demonspawn", "exo", "illusionist", "jotunn",
    "marauder", "marksman", "necromancer", "nomad", "paladin", "pirate", "plague_doctor",
    "prophet", "pyromancer", "redneck", "samurai", "shaman", "shield_lancer", "stormweaver",
    "viking", "white_mage",
];

fn step_value(perf: &BuildPerformance, label: &str) -> Option<Ranged> {
    perf.calculation()
        .iter()
        .rev()
        .find(|s| s.label() == label)
        .map(|s| s.value())
}

fn fmt(v: Option<f64>) -> String {
    v.map_or(String::new(), |x| format!("{x:.3}"))
}

#[test]
#[ignore]
fn dps_snapshot() {
    let out = std::env::var("HSP_DPS_SNAPSHOT").unwrap_or_else(|_| "dps_snapshot.csv".into());
    let mut rows = vec![
        "class,skill,name,actions_per_s,hits_per_cast,hit_avg,hit_dps,combined_dps,dummy_conf".to_string(),
    ];
    // Reference build: level 100, skill rank 20, no gear, no subskills; weapon
    // requirements lifted so attack skills still report.
    let stats = vec![CustomStat {
        stat_key: "skill_restrictions_removed".into(),
        value: "100".into(),
    }];
    let empty_f = HashMap::new();
    let empty_b = HashMap::new();
    let empty_u = HashMap::new();
    let inventory: Inventory = HashMap::new();
    for class in CLASSES {
        // HSP_ALL_BUFFS=1: every buff toggle of the class on.
        let buffs: HashMap<String, bool> = if std::env::var("HSP_ALL_BUFFS").is_ok() {
            crate::calc::data::get_skills_by_class(class)
                .iter()
                .map(|s| (s.id.clone(), true))
                .collect()
        } else {
            HashMap::new()
        };
        for spec in crate::calc::data::get_skills_by_class(class) {
            if spec.kind != crate::calc::types::SkillKind::Active {
                continue;
            }
            // HSP_ALL_SKILLS=1: every skill of the class at rank 20 (synergies on).
            let ranks: HashMap<String, u32> = if std::env::var("HSP_ALL_SKILLS").is_ok() {
                crate::calc::data::get_skills_by_class(class)
                    .iter()
                    .map(|s| (s.id.clone(), 20))
                    .collect()
            } else {
                [(spec.id.clone(), 20)].into_iter().collect()
            };
            let perf = compute_build_performance(&BuildPerformanceDeps {
                class_id: Some(class),
                level: 100,
                allocated_attrs: &HashMap::new(),
                inventory: &inventory,
                skill_ranks: &ranks,
                subskill_ranks: &empty_u,
                active_aura_id: None,
                active_buffs: &buffs,
                custom_stats: &stats,
                allocated_tree_nodes: &HashSet::new(),
                tree_socketed: &HashMap::new(),
                main_skill_id: Some(spec.id.as_str()),
                enemy_conditions: &empty_b,
                player_conditions: &HashMap::new(),
                skill_projectiles: &HashMap::new(),
                enemy_resistances: &HashMap::new(),
                proc_toggles: &empty_b,
                kills_per_sec: 0.0,
                entity_rates: &empty_f,
                stack_counts: &HashMap::new(),
                granted_skill_ranks: None,
                difficulty: None,
            });
            let rate = step_value(&perf, "Actions per second")
                .or_else(|| step_value(&perf, "Entity actions per second"))
                .map(|r| r.1)
                .or_else(|| perf.attack_damage.as_ref().map(|a| a.attacks_per_second_max));
            rows.push(format!(
                "{class},{},{},{},{},{},{},{},{}",
                spec.id,
                spec.name.replace(',', " "),
                fmt(rate),
                fmt(step_value(&perf, "Dummy hits per use")
                    .map(|h| h.1)
                    .or(perf.hits_per_cast.map(|h| h.1))
                    .or(Some(1.0))),
                fmt(perf
                    .attack_damage
                    .as_ref()
                    .map(|a| a.combined_avg_max as f64)
                    .or(perf.damage.as_ref().map(|d| d.avg_max as f64))),
                fmt(perf.hit_dps_max),
                fmt(perf.combined_dps_max),
                spec.dummy_hits
                    .as_ref()
                    .map_or("-".to_string(), |d| d.confidence.clone().unwrap_or_default()),
            ));
        }
    }
    std::fs::write(&out, rows.join("\n") + "\n").expect("write snapshot");
    eprintln!("wrote {} rows to {out}", rows.len() - 1);
}

#[test]
fn tick_hits_matches_rule_20_table() {
    // tools/tick_hits.py: (lifetime s, tick s) -> hits, active = life*60 - 2.
    for (life, tick, want) in [
        (1.25, 0.5, 3), (1.0, 0.5, 2), (2.0, 0.5, 4), (3.0, 0.5, 6),
        (4.0, 0.33, 12), (4.5, 0.4, 11), (5.0, 0.25, 19), (2.25, 0.5, 5),
    ] {
        let active = (life * 60.0_f64).round() as u32 - 2;
        assert_eq!(tick_hits(active, tick * 60.0, 0, 0), want, "{life} s / {tick} s");
    }
}

fn dummy_hits_with(class: &str, skill: &str, subskills: &[(&str, u32)]) -> f64 {
    let ranks: HashMap<String, u32> = [(skill.to_string(), 20)].into_iter().collect();
    let sub_ranks: HashMap<String, u32> = subskills
        .iter()
        .map(|(id, r)| (subskill_key(skill, id), *r))
        .collect();
    let perf = compute_build_performance(&BuildPerformanceDeps {
        class_id: Some(class),
        level: 100,
        allocated_attrs: &HashMap::new(),
        inventory: &HashMap::new(),
        skill_ranks: &ranks,
        subskill_ranks: &sub_ranks,
        active_aura_id: None,
        active_buffs: &HashMap::new(),
        custom_stats: &[],
        allocated_tree_nodes: &HashSet::new(),
        tree_socketed: &HashMap::new(),
        main_skill_id: Some(skill),
        enemy_conditions: &HashMap::new(),
        player_conditions: &HashMap::new(),
        skill_projectiles: &HashMap::new(),
        enemy_resistances: &HashMap::new(),
        proc_toggles: &HashMap::new(),
        kills_per_sec: 0.0,
        entity_rates: &HashMap::new(),
        stack_counts: &HashMap::new(),
        granted_skill_ranks: None,
        difficulty: None,
    });
    step_value(&perf, "Dummy hits per use").expect("dummy hits").1
}

#[test]
fn subskill_dummy_effects_change_hits() {
    let base = dummy_hits_with("shaman", "twisters", &[]);
    assert!((base - 1.36).abs() < 1e-9, "{base}");
    // Explosive Gust: an explosion on every hit doubles the hits.
    let gust = dummy_hits_with("shaman", "twisters", &[("explosive_gust", 1)]);
    assert!((gust - 2.0 * base).abs() < 1e-9, "{gust}");
    // Twistering Multitude r3: the card's H(6) = 2.72 replaces the projectile slope.
    let multi = dummy_hits_with("shaman", "twisters", &[("twistering_multitude", 3)]);
    assert!((multi - (base + 1.36)).abs() < 1e-9, "{multi}");
}
