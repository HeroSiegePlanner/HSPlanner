use super::*;

struct Build {
    class: &'static str,
    main: Option<&'static str>,
    ranks: HashMap<String, u32>,
    subs: HashMap<String, u32>,
    stats: Vec<CustomStat>,
    rates: HashMap<String, f64>,
    procs: HashMap<String, bool>,
    projectiles: HashMap<String, u32>,
    kills: f64,
}

impl Build {
    fn new(class: &'static str, skill: &'static str) -> Self {
        Self {
            class,
            main: Some(skill),
            ranks: [(skill.to_string(), 10)].into_iter().collect(),
            subs: HashMap::new(),
            stats: Vec::new(),
            rates: HashMap::new(),
            procs: HashMap::new(),
            projectiles: HashMap::new(),
            kills: 0.0,
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
            inventory: &HashMap::new(),
            skill_ranks: &self.ranks,
            subskill_ranks: &self.subs,
            active_aura_id: None,
            active_buffs: &HashMap::new(),
            custom_stats: &self.stats,
            allocated_tree_nodes: &HashSet::new(),
            tree_socketed: &HashMap::new(),
            main_skill_id: self.main,
            enemy_conditions: &HashMap::new(),
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
