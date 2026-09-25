use super::*;
use crate::calc::types::EquippedItem;

#[allow(clippy::too_many_arguments)]
fn empty_deps<'a>(
    allocated: &'a HashMap<String, u32>,
    inventory: &'a Inventory,
    skill_ranks: &'a HashMap<String, u32>,
    subskill_ranks: &'a HashMap<String, u32>,
    active_buffs: &'a HashMap<String, bool>,
    custom_stats: &'a [CustomStat],
    alloc_tree: &'a HashSet<u32>,
    tree_socketed: &'a HashMap<u32, TreeSocketContent>,
    enemy_conditions: &'a HashMap<String, bool>,
    player_conditions: &'a HashMap<String, bool>,
    skill_projectiles: &'a HashMap<String, u32>,
    enemy_resistances: &'a HashMap<String, f64>,
    proc_toggles: &'a HashMap<String, bool>,
) -> BuildPerformanceDeps<'a> {
    BuildPerformanceDeps {
        class_id: None,
        level: 1,
        allocated_attrs: allocated,
        inventory,
        skill_ranks,
        subskill_ranks,
        active_aura_id: None,
        active_buffs,
        custom_stats,
        allocated_tree_nodes: alloc_tree,
        tree_socketed,
        main_skill_id: None,
        enemy_conditions,
        player_conditions,
        skill_projectiles,
        enemy_resistances,
        proc_toggles,
        kills_per_sec: 0.0,
        entity_rates: &DEFAULT_RATES,
        stack_counts: &NO_STACKS,
        granted_skill_ranks: None,
        difficulty: None,
    }
}

static NO_STACKS: std::sync::LazyLock<HashMap<String, u32>> =
    std::sync::LazyLock::new(HashMap::new);

static DEFAULT_RATES: std::sync::LazyLock<HashMap<String, f64>> =
    std::sync::LazyLock::new(|| entity_rates(1.0));

fn entity_rates(rate: f64) -> HashMap<String, f64> {
    ["sentry", "summon", "guardian"]
        .iter()
        .map(|k| (k.to_string(), rate))
        .collect()
}

fn perf(
    class_id: &str,
    skill_id: &str,
    rank: u32,
    subskills: &[(&str, u32)],
    enemy: &[(&str, bool)],
) -> BuildPerformance {
    perf_with_tree(class_id, skill_id, rank, subskills, enemy, &[])
}

fn perf_with_tree(
    class_id: &str,
    skill_id: &str,
    rank: u32,
    subskills: &[(&str, u32)],
    enemy: &[(&str, bool)],
    nodes: &[u32],
) -> BuildPerformance {
    perf_with_tree_conditions(class_id, skill_id, rank, subskills, enemy, nodes, &[])
}

fn perf_with_tree_conditions(
    class_id: &str,
    skill_id: &str,
    rank: u32,
    subskills: &[(&str, u32)],
    enemy: &[(&str, bool)],
    nodes: &[u32],
    player: &[(&str, bool)],
) -> BuildPerformance {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let mut skill_ranks = HashMap::new();
    skill_ranks.insert(skill_id.to_string(), rank);
    let subskill_ranks: HashMap<String, u32> = subskills
        .iter()
        .map(|(id, r)| (subskill_key(skill_id, id), *r))
        .collect();
    let active_buffs = HashMap::new();
    // Formula fixtures intentionally have no equipment. Weapon eligibility is
    // exercised separately with real inventory in the Wargod regressions.
    let custom_stats = vec![CustomStat {
        stat_key: "skill_restrictions_removed".into(),
        value: "100".into(),
    }];
    let alloc_tree = nodes.iter().copied().collect();
    let tree_socketed = HashMap::new();
    let enemy_conditions: HashMap<String, bool> =
        enemy.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    let player_conditions = player
        .iter()
        .map(|(key, value)| (key.to_string(), *value))
        .collect();
    let skill_projectiles = HashMap::new();
    let enemy_resistances = HashMap::new();
    let proc_toggles = HashMap::new();

    let mut deps = empty_deps(
        &allocated,
        &inventory,
        &skill_ranks,
        &subskill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &enemy_conditions,
        &player_conditions,
        &skill_projectiles,
        &enemy_resistances,
        &proc_toggles,
    );
    deps.class_id = Some(class_id);
    deps.level = 50;
    deps.main_skill_id = Some(skill_id);
    compute_build_performance(&deps)
}

#[test]
fn wizards_wrath_changes_real_spell_damage_and_cast_cadence() {
    let base = perf("pyromancer", "fireball", 10, &[], &[])
        .avg_hit_dps_max
        .unwrap();
    for (overheated, expected_ratio) in [(false, 1.5), (true, 0.75)] {
        let result = perf_with_tree_conditions(
            "pyromancer",
            "fireball",
            10,
            &[],
            &[],
            &[992],
            &[("overheated", overheated)],
        );
        let ratio = result.avg_hit_dps_max.unwrap() / base;
        assert!(
            (ratio - expected_ratio).abs() < 0.01,
            "overheated={overheated}: {ratio}"
        );
    }
}

// marksman:gunner_drone "Multitude" grants sentry_max_amount 2/rank. Drone
// count multiplies DPS; the note's projectile_count must be gone from data.
#[test]
fn sentry_amount_multiplies_drone_dps() {
    let base = perf("marksman", "gunner_drone", 10, &[], &[]);
    let with_count = perf("marksman", "gunner_drone", 10, &[("multitude", 1)], &[]);
    let (Some(one), Some(three)) = (base.avg_hit_dps_max, with_count.avg_hit_dps_max) else {
        panic!("expected dps for gunner_drone");
    };
    assert!(
        (three / one - 3.0).abs() < 1e-9,
        "1 + 2 drones should triple dps, got x{}",
        three / one
    );
    // Maxed subtree: +16 drones, so 17x the DPS and 17 in the exported count.
    let maxed = perf("marksman", "gunner_drone", 10, &[("multitude", 8)], &[]);
    assert_eq!(maxed.entity_count, Some((17.0, 17.0)));
    let Some(all) = maxed.avg_hit_dps_max else {
        panic!("expected dps for gunner_drone");
    };
    assert!(
        (all / one - 17.0).abs() < 1e-9,
        "1 + 16 drones should be 17x dps, got x{}",
        all / one
    );
}

// Sentry, Summon and Guardian each own their rate knob; a summon knob must
// leave a sentry skill alone.
#[test]
fn each_entity_kind_reads_its_own_rate() {
    let base = perf_with_stats("marksman", "gunner_drone", 10, &[], &[], 1.0);
    let Some(one) = base.avg_hit_dps_max else {
        panic!("expected dps for gunner_drone");
    };

    let per_kind = |sentry: f64, summon: f64| {
        let mut rates = entity_rates(1.0);
        rates.insert("sentry".into(), sentry);
        rates.insert("summon".into(), summon);
        rates
    };
    let mut skill_ranks = HashMap::new();
    skill_ranks.insert("gunner_drone".to_string(), 10u32);
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let subskill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let enemy_conditions = HashMap::new();
    let player_conditions = HashMap::new();
    let skill_projectiles = HashMap::new();
    let enemy_resistances = HashMap::new();
    let proc_toggles = HashMap::new();
    let mut deps = empty_deps(
        &allocated,
        &inventory,
        &skill_ranks,
        &subskill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &enemy_conditions,
        &player_conditions,
        &skill_projectiles,
        &enemy_resistances,
        &proc_toggles,
    );
    deps.class_id = Some("marksman");
    deps.level = 50;
    deps.main_skill_id = Some("gunner_drone");

    let summon_only = per_kind(1.0, 5.0);
    deps.entity_rates = &summon_only;
    assert_eq!(
        compute_build_performance(&deps).avg_hit_dps_max,
        Some(one),
        "a summon rate must not touch a Sentry skill"
    );

    let sentry_only = per_kind(3.0, 1.0);
    deps.entity_rates = &sentry_only;
    let Some(tripled) = compute_build_performance(&deps).avg_hit_dps_max else {
        panic!("expected dps for gunner_drone");
    };
    assert!(
        (tripled / one - 3.0).abs() < 1e-9,
        "sentry rate 3/s should triple drone dps, got x{}",
        tripled / one
    );
}

#[test]
fn entity_rate_config_scales_dps_and_player_fcr_does_not() {
    let base = perf("marksman", "gunner_drone", 10, &[], &[]);

    let mut skill_ranks = HashMap::new();
    skill_ranks.insert("gunner_drone".to_string(), 10);
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let subskill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let fcr_stats = vec![CustomStat {
        stat_key: "faster_cast_rate".to_string(),
        value: "100".to_string(),
    }];
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let enemy_conditions = HashMap::new();
    let player_conditions = HashMap::new();
    let skill_projectiles = HashMap::new();
    let enemy_resistances = HashMap::new();
    let proc_toggles = HashMap::new();
    let mut deps = empty_deps(
        &allocated,
        &inventory,
        &skill_ranks,
        &subskill_ranks,
        &active_buffs,
        &fcr_stats,
        &alloc_tree,
        &tree_socketed,
        &enemy_conditions,
        &player_conditions,
        &skill_projectiles,
        &enemy_resistances,
        &proc_toggles,
    );
    deps.class_id = Some("marksman");
    deps.level = 50;
    deps.main_skill_id = Some("gunner_drone");

    let with_fcr = compute_build_performance(&deps);
    assert_eq!(
        with_fcr.avg_hit_dps_max, base.avg_hit_dps_max,
        "player FCR must not speed up the drone"
    );

    let sentry_as = vec![CustomStat {
        stat_key: "sentry_attack_speed".to_string(),
        value: "100".to_string(),
    }];
    deps.custom_stats = &sentry_as;
    let faster = compute_build_performance(&deps);
    let (Some(b), Some(f)) = (base.avg_hit_dps_max, faster.avg_hit_dps_max) else {
        panic!("expected dps");
    };
    assert!(
        (f / b - 2.0).abs() < 1e-9,
        "sentry AS +100% should double dps"
    );

    deps.custom_stats = &[];
    let doubled = entity_rates(2.0);
    deps.entity_rates = &doubled;
    let two_per_sec = compute_build_performance(&deps);
    let Some(t) = two_per_sec.avg_hit_dps_max else {
        panic!("expected dps");
    };
    assert!(
        (t / b - 2.0).abs() < 1e-9,
        "2/s base rate should double dps"
    );

    // Config rate is the flat base; "increased sentry attack speed" multiplies
    // it rather than adding to it: 2/s at +100% swings four times as often.
    deps.custom_stats = &sentry_as;
    let Some(both) = compute_build_performance(&deps).avg_hit_dps_max else {
        panic!("expected dps");
    };
    assert!(
        (both / b - 4.0).abs() < 1e-9,
        "flat 2/s x (1 + 100%) should quadruple dps, got x{}",
        both / b
    );
}

// A pinned entity rate (C.Y.C.L.O.P.S. lasers) ignores both the config knob and
// sentry attack speed, so a maxed Rapidfire buys the laser build nothing.
#[test]
fn pinned_entity_rate_ignores_config_knob_and_sentry_speed() {
    let pin: &[(&str, &str)] = &[("sentry_attack_rate_fixed", "4")];
    let plain = perf_with_stats("marksman", "gunner_drone", 10, &[], &[], 1.0);
    let pinned = perf_with_stats("marksman", "gunner_drone", 10, &[], pin, 1.0);
    let pinned_fast_knob = perf_with_stats("marksman", "gunner_drone", 10, &[], pin, 3.0);
    let pinned_hasted = perf_with_stats(
        "marksman",
        "gunner_drone",
        10,
        &[],
        &[
            ("sentry_attack_rate_fixed", "4"),
            ("sentry_attack_speed", "100"),
        ],
        1.0,
    );

    let (Some(base), Some(p), Some(knob), Some(hasted)) = (
        plain.avg_hit_dps_max,
        pinned.avg_hit_dps_max,
        pinned_fast_knob.avg_hit_dps_max,
        pinned_hasted.avg_hit_dps_max,
    ) else {
        panic!("expected dps");
    };
    assert!(
        (p / base - 4.0).abs() < 1e-9,
        "a 4/s pin should quadruple 1/s dps, got x{}",
        p / base
    );
    assert!(
        (knob - p).abs() < 1e-9,
        "config knob must not move a pinned rate"
    );
    assert!(
        (hasted - p).abs() < 1e-9,
        "sentry attack speed must not move a pinned rate"
    );
}

// The S10 node carries the pin; without it the drone stays on the config knob.
#[test]
fn cyclops_pins_the_drone_to_four_ticks_a_second() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let laser: &[(&str, u32)] = &[("c_y_c_l_o_p_s", 1)];
    let at_one = perf_with_stats("marksman", "gunner_drone", 10, laser, &[], 1.0);
    let at_three = perf_with_stats("marksman", "gunner_drone", 10, laser, &[], 3.0);
    let (Some(one), Some(three)) = (at_one.avg_hit_dps_max, at_three.avg_hit_dps_max) else {
        panic!("expected dps");
    };
    assert!(
        (one - three).abs() < 1e-9,
        "C.Y.C.L.O.P.S. must pin the rate, got {one} vs {three}"
    );
}

#[allow(clippy::too_many_arguments)]
fn perf_with_stats(
    class_id: &str,
    skill_id: &str,
    rank: u32,
    subskills: &[(&str, u32)],
    custom: &[(&str, &str)],
    entity_rate: f64,
) -> BuildPerformance {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let mut skill_ranks = HashMap::new();
    skill_ranks.insert(skill_id.to_string(), rank);
    let subskill_ranks: HashMap<String, u32> = subskills
        .iter()
        .map(|(id, r)| (subskill_key(skill_id, id), *r))
        .collect();
    let active_buffs = HashMap::new();
    let mut custom_stats: Vec<CustomStat> = custom
        .iter()
        .map(|(k, v)| CustomStat {
            stat_key: k.to_string(),
            value: v.to_string(),
        })
        .collect();
    // Isolate numerical formulas from the equipment needed to cast a skill.
    custom_stats.push(CustomStat {
        stat_key: "skill_restrictions_removed".into(),
        value: "100".into(),
    });
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let enemy_conditions = HashMap::new();
    let player_conditions = HashMap::new();
    let skill_projectiles = HashMap::new();
    let enemy_resistances = HashMap::new();
    let proc_toggles = HashMap::new();
    let mut deps = empty_deps(
        &allocated,
        &inventory,
        &skill_ranks,
        &subskill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &enemy_conditions,
        &player_conditions,
        &skill_projectiles,
        &enemy_resistances,
        &proc_toggles,
    );
    deps.class_id = Some(class_id);
    deps.level = 50;
    deps.main_skill_id = Some(skill_id);
    let rates = entity_rates(entity_rate);
    deps.entity_rates = &rates;
    compute_build_performance(&deps)
}

// Multicast re-casts a spell; sentries, summons and guardians are spawned, not
// multicast, so the stat must not reach their damage.
#[test]
fn multicast_skips_entity_skills() {
    let drone = perf_with_stats(
        "marksman",
        "gunner_drone",
        10,
        &[],
        &[("multicast_chance", "50")],
        1.0,
    );
    let d = drone.damage.as_ref().expect("drone damage");
    assert_eq!(
        d.multicast_chance_pct, 0.0,
        "a Sentry skill must not multicast"
    );

    let spell = perf_with_stats(
        "pyromancer",
        "fireball",
        10,
        &[],
        &[("multicast_chance", "50")],
        1.0,
    );
    assert_eq!(
        spell
            .damage
            .as_ref()
            .expect("fireball damage")
            .multicast_chance_pct,
        50.0,
        "a plain Spell still multicasts"
    );
}

// Burning Shot is a 4%/rank on-hit proc. A lone caster burns the target 4% of
// the time; a swarm of drones lands enough hits to keep it burning.
#[test]
fn ailment_uptime_grows_with_the_number_of_entities() {
    let lone = perf_with_stats(
        "marksman",
        "gunner_drone",
        10,
        &[("burning_shot", 1)],
        &[],
        1.0,
    );
    let swarm = perf_with_stats(
        "marksman",
        "gunner_drone",
        10,
        &[("burning_shot", 1), ("multitude", 8)],
        &[],
        1.0,
    );
    let (Some(one), Some(many)) = (lone.ailment_dps_max, swarm.ailment_dps_max) else {
        panic!("expected burning dps");
    };
    assert!(
        many / one > 10.0,
        "17 drones should keep the burn up far longer than one, got x{}",
        many / one
    );
}

// "+X to Sentry Skills" must show up in the displayed rank too, not just
// silently in the damage math.
#[test]
fn sentry_skills_bonus_lands_in_rank_bonuses() {
    let plus = perf_with_stats(
        "marksman",
        "gunner_drone",
        10,
        &[],
        &[("sentry_skills", "5")],
        1.0,
    );
    assert_eq!(
        plus.rank_bonuses.get("gunner drone").copied(),
        Some((5.0, 5.0)),
        "sentry_skills should appear in the skill's rank bonus"
    );
}

// "+X to Explosion Skills" was dead before affix-tags.json: explosion is a tag,
// never a damage type, so the element path could not see it.
#[test]
fn explosion_skills_bonus_reaches_an_explosion_tagged_skill() {
    let plus = perf_with_stats(
        "pyromancer",
        "volcano",
        10,
        &[],
        &[("explosion_skills", "4")],
        1.0,
    );
    assert_eq!(
        plus.rank_bonuses.get("volcano").copied(),
        Some((4.0, 4.0)),
        "explosion_skills should raise an Explosion-tagged skill"
    );
    assert_eq!(
        plus.rank_bonuses.get("fire enchant").copied(),
        Some((0.0, 0.0)),
        "a skill without the Explosion tag must not pick it up"
    );
}

// Orange Grayon (satanic charm): implicit sentry AS / damage / skills /
// amount. The whole chain item -> stats -> entity branch must move the DPS.
#[test]
fn sentry_charm_implicits_raise_drone_dps() {
    let base = perf_with_stats("marksman", "gunner_drone", 10, &[], &[], 1.0);

    let allocated = HashMap::new();
    let mut inventory: Inventory = HashMap::new();
    inventory.insert(
        "charm_1".to_string(),
        EquippedItem {
            base_id: "charm_satanic_orange_grayon".to_string(),
            ..Default::default()
        },
    );
    let mut skill_ranks = HashMap::new();
    skill_ranks.insert("gunner_drone".to_string(), 10);
    let subskill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let enemy_conditions = HashMap::new();
    let player_conditions = HashMap::new();
    let skill_projectiles = HashMap::new();
    let enemy_resistances = HashMap::new();
    let proc_toggles = HashMap::new();
    let mut deps = empty_deps(
        &allocated,
        &inventory,
        &skill_ranks,
        &subskill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &enemy_conditions,
        &player_conditions,
        &skill_projectiles,
        &enemy_resistances,
        &proc_toggles,
    );
    deps.class_id = Some("marksman");
    deps.level = 100;
    deps.main_skill_id = Some("gunner_drone");
    let with_charm = compute_build_performance(&deps);

    let (Some(b), Some(f)) = (base.avg_hit_dps_max, with_charm.avg_hit_dps_max) else {
        panic!("expected dps");
    };
    // AS +25%, damage +45% additive, +8 sentry skills (rank 18), +2% amount:
    // every one of them must push the number up.
    assert!(
        f / b > 1.5,
        "sentry charm implicits should visibly raise dps, got x{}",
        f / b
    );
}

// "+X to Sentry Skills" raises the rank of Sentry-tagged skills, exactly
// like projectile_skills does for Projectile.
#[test]
fn sentry_skills_rank_bonus_raises_drone_damage() {
    let base = perf_with_stats("marksman", "gunner_drone", 10, &[], &[], 1.0);
    let plus = perf_with_stats(
        "marksman",
        "gunner_drone",
        10,
        &[],
        &[("sentry_skills", "5")],
        1.0,
    );
    let rank15 = perf_with_stats("marksman", "gunner_drone", 15, &[], &[], 1.0);
    assert!(
        plus.avg_hit_dps_max > base.avg_hit_dps_max,
        "+5 sentry skills must raise dps"
    );
    assert_eq!(
        plus.avg_hit_dps_max, rank15.avg_hit_dps_max,
        "rank 10 with +5 sentry skills should hit like allocated rank 15"
    );
}

// Tree/gear "+Maximum Summon Amount" is a summon stat; sentries count only
// their own sentry_max_amount.
#[test]
fn gear_summon_amount_does_not_multiply_sentry_dps() {
    let base = perf_with_stats("marksman", "gunner_drone", 10, &[], &[], 1.0);
    let with_gear = perf_with_stats(
        "marksman",
        "gunner_drone",
        10,
        &[],
        &[("summon_max_amount", "5")],
        1.0,
    );
    assert_eq!(
        base.avg_hit_dps_max, with_gear.avg_hit_dps_max,
        "summon amount must not scale a sentry"
    );
}

// data/subskill-tags.json: Nanodrones adds the Explosion tag to Gunner Drone,
// so explosion_damage starts counting only once the node is taken.
#[test]
fn subskill_tag_change_enables_archetype_damage() {
    let plain = perf_with_stats("marksman", "gunner_drone", 10, &[], &[], 1.0);
    let plain_stat = perf_with_stats(
        "marksman",
        "gunner_drone",
        10,
        &[],
        &[("explosion_damage", "40")],
        1.0,
    );
    assert_eq!(
        plain.avg_hit_dps_max, plain_stat.avg_hit_dps_max,
        "without Nanodrones the drone is not an Explosion skill"
    );

    let node = perf_with_stats(
        "marksman",
        "gunner_drone",
        10,
        &[("nanodrones", 1)],
        &[],
        1.0,
    );
    let node_stat = perf_with_stats(
        "marksman",
        "gunner_drone",
        10,
        &[("nanodrones", 1)],
        &[("explosion_damage", "40")],
        1.0,
    );
    let (Some(b), Some(f)) = (node.avg_hit_dps_max, node_stat.avg_hit_dps_max) else {
        panic!("expected dps");
    };
    // Nanodrones itself grants arcane_skill_damage 15, sharing the additive
    // pool with explosion_damage: (100+15+40)/(100+15).
    assert!(
        (f / b - 155.0 / 115.0).abs() < 1e-9,
        "with Nanodrones explosion_damage 40% should join the additive pool, got x{}",
        f / b
    );
}

// data/subskill-tags.json: Ancient Device turns Death from Above into a
// Sentry, so its DPS switches to the entity rate.
#[test]
fn subskill_tag_change_switches_skill_to_entity_rate() {
    let no_node_1 = perf_with_stats("amazon", "death_from_above", 10, &[], &[], 1.0);
    let no_node_3 = perf_with_stats("amazon", "death_from_above", 10, &[], &[], 3.0);
    assert_eq!(
        no_node_1.avg_hit_dps_max, no_node_3.avg_hit_dps_max,
        "without Ancient Device the entity rate must not matter"
    );

    let node_1 = perf_with_stats(
        "amazon",
        "death_from_above",
        10,
        &[("ancient_device", 1)],
        &[],
        1.0,
    );
    let node_3 = perf_with_stats(
        "amazon",
        "death_from_above",
        10,
        &[("ancient_device", 1)],
        &[],
        3.0,
    );
    let (Some(b), Some(f)) = (node_1.avg_hit_dps_max, node_3.avg_hit_dps_max) else {
        panic!("expected dps");
    };
    assert!(
        (f / b - 3.0).abs() < 1e-9,
        "as a Sentry the skill should scale with entity rate, got x{}",
        f / b
    );
}

// marksman:gunner_drone "Rapidfire" is the drone's own rate of fire
// (sentry_attack_speed 12.5/rank), not player FCR.
#[test]
fn rapidfire_scales_the_drone_rate() {
    let base = perf("marksman", "gunner_drone", 10, &[], &[]);
    let fast = perf("marksman", "gunner_drone", 10, &[("rapidfire", 4)], &[]);
    let (Some(b), Some(f)) = (base.avg_hit_dps_max, fast.avg_hit_dps_max) else {
        panic!("expected dps for gunner_drone");
    };
    assert!(
        (f / b - 1.5).abs() < 1e-9,
        "rank 4 rapidfire = +50% drone rate, got x{}",
        f / b
    );
}

// demonspawn:spinal_tap#11 "Blood and Gore" grants execute_below 2.5/rank.
#[test]
fn execute_below_from_the_subtree_raises_combined_dps() {
    let with_exec = perf(
        "demonspawn",
        "spinal_tap",
        10,
        &[("blood_and_gore", 4)],
        &[],
    );
    let on_boss = perf(
        "demonspawn",
        "spinal_tap",
        10,
        &[("blood_and_gore", 4)],
        &[("is_boss", true)],
    );
    let (Some(normal), Some(boss)) = (with_exec.combined_dps_max, on_boss.combined_dps_max) else {
        panic!("expected combined dps for spinal_tap");
    };
    // rank 4 -> execute_below 10% -> 1 / (1 - 0.10)
    assert!(
        (normal / boss - 1.0 / 0.9).abs() < 1e-9,
        "execute multiplier missing: {normal} vs {boss}"
    );
}

// samurai:explosive_kunai is thrown at weapon attack speed, not cast rate.
#[test]
fn attack_speed_skill_rates_off_attacks_per_second() {
    let p = perf("samurai", "explosive_kunai", 20, &[], &[]);
    let damage = p.damage.as_ref().expect("kunai damage");
    let dps = p.hit_dps_max.expect("kunai hit dps");
    // 1.5 base attacks/s from game-config, no IAS on a bare build.
    assert!(
        (dps - damage.final_max as f64 * 1.5).abs() < 1e-6,
        "expected hit dps at 1.5 throws/s, got {dps}"
    );
}

#[test]
fn scorching_whip_deals_spell_damage_at_every_learned_rank() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    for rank in 1..=20 {
        let p = perf("exo", "scorching_whip", rank, &[], &[]);
        let damage = p.damage.as_ref().expect("Scorching Whip damage");
        assert!(damage.final_min > 0, "rank {rank}");
        assert!((damage.base_min - (5.2 + 10.0 * rank as f64)).abs() < 1e-9);
        assert!(
            p.attack_damage.is_none(),
            "the whip has no weapon damage component"
        );
        assert!(p.hit_dps_min.is_some_and(|dps| dps > 0.0), "rank {rank}");
        assert_eq!(p.skill_costs["scorching_whip"].cast_rate_min, Some(1.5));
    }
    let unlearned = perf("exo", "scorching_whip", 0, &[], &[]);
    assert!(unlearned.damage.is_none());
    assert!(unlearned.hit_dps_min.is_none());
}

#[test]
fn scorching_whip_rate_and_dps_scale_with_attack_speed_not_cast_rate() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let base = perf("exo", "scorching_whip", 20, &[], &[]);
    let base_dps = base.hit_dps_max.expect("Scorching Whip DPS");
    let base_mana = base.skill_costs["scorching_whip"].mana_per_sec_max.unwrap();
    for (stat, multiplier) in [("increased_attack_speed", 2.0), ("faster_cast_rate", 1.0)] {
        let fast = perf_with_stats("exo", "scorching_whip", 20, &[], &[(stat, "100")], 1.0);
        assert_eq!(
            fast.damage.as_ref().unwrap().final_max,
            base.damage.as_ref().unwrap().final_max
        );
        assert!(
            (fast.hit_dps_max.unwrap() - base_dps * multiplier).abs() < 1e-9,
            "{stat}"
        );
        let cost = &fast.skill_costs["scorching_whip"];
        assert_eq!(cost.cast_rate_max, Some(1.5 * multiplier), "{stat}");
        assert!(
            (cost.mana_per_sec_max.unwrap() - base_mana * multiplier).abs() < 1e-9,
            "{stat}"
        );
    }
}

#[test]
fn scorching_whip_uses_equipped_weapon_speed_and_its_subtree_bonus() {
    use crate::calc::commands::{calc_build_performance, BuildPerformanceInput};

    for (weapon, base_rate) in [
        ("base_spell_gnarled_staff", 1.25),
        ("base_sword_scimitar", 1.8),
    ] {
        for (chosen_rank, multiplier) in [(0, 1.0), (5, 1.5)] {
            let input = BuildPerformanceInput {
                class_id: Some("exo".into()),
                level: 100,
                main_skill_id: Some("scorching_whip".into()),
                skill_ranks: HashMap::from([("scorching_whip".into(), 20)]),
                subskill_ranks: HashMap::from([(
                    "scorching_whip:sun_gods_chosen".into(),
                    chosen_rank,
                )]),
                inventory: Inventory::from([(
                    "weapon".into(),
                    EquippedItem {
                        base_id: weapon.into(),
                        ..Default::default()
                    },
                )]),
                season: Some("s10".into()),
                ..Default::default()
            };
            let p = calc_build_performance(input);
            let rate = base_rate * multiplier;
            let dps = p.hit_dps_max.expect("Scorching Whip DPS");
            assert!(
                (dps - p.damage.as_ref().unwrap().final_max as f64 * rate).abs() < 1e-9,
                "{weapon}, chosen {chosen_rank}"
            );
            assert_eq!(p.skill_costs["scorching_whip"].cast_rate_max, Some(rate));
        }
    }
}

// demon_slayer:demons_calling#6 inflicts burning and boosts its damage.
#[test]
fn ailment_dps_adds_on_top_of_hit_dps() {
    let plain = perf("demon_slayer", "demons_calling", 10, &[], &[]);
    let burning = perf(
        "demon_slayer",
        "demons_calling",
        10,
        &[("the_fiery_layer_of_hell", 5)],
        &[],
    );
    assert_eq!(plain.ailment_dps_max, None);
    let Some(ailment) = burning.ailment_dps_max else {
        panic!("expected burning dps once the subtree applies burning");
    };
    assert!(ailment > 0.0);
    let combined = burning.combined_dps_max.expect("combined dps");
    let hit = burning.avg_hit_dps_max.expect("hit dps");
    assert!(
        (combined - (hit + burning.proc_dps_max + ailment)).abs() < 1e-6,
        "combined dps must carry the ailment term",
    );
}

#[test]
fn asphyxiating_touch_keeps_ailments_but_suppresses_direct_damage() {
    let baseline = perf(
        "demon_slayer",
        "demons_calling",
        10,
        &[("the_fiery_layer_of_hell", 5)],
        &[],
    );
    let baseline_ailment = baseline.ailment_dps_max.expect("baseline burning");
    for node_id in [312, 1736] {
        let allocated = HashMap::new();
        let inventory = HashMap::new();
        let skill_ranks = HashMap::from([("demons_calling".to_string(), 10)]);
        let subskill_ranks =
            HashMap::from([(subskill_key("demons_calling", "the_fiery_layer_of_hell"), 5)]);
        let active_buffs = HashMap::new();
        let custom_stats = Vec::new();
        let tree_nodes = HashSet::from([node_id]);
        let tree_socketed = HashMap::new();
        let enemy_conditions = HashMap::new();
        let player_conditions = HashMap::new();
        let skill_projectiles = HashMap::new();
        let enemy_resistances = HashMap::new();
        let proc_toggles = HashMap::new();
        let mut deps = empty_deps(
            &allocated,
            &inventory,
            &skill_ranks,
            &subskill_ranks,
            &active_buffs,
            &custom_stats,
            &tree_nodes,
            &tree_socketed,
            &enemy_conditions,
            &player_conditions,
            &skill_projectiles,
            &enemy_resistances,
            &proc_toggles,
        );
        deps.class_id = Some("demon_slayer");
        deps.level = 50;
        deps.main_skill_id = Some("demons_calling");
        let result = compute_build_performance(&deps);
        assert_eq!(result.avg_hit_dps_max, Some(0.0), "node {node_id}");
        assert_eq!(result.hit_dps_max, Some(0.0), "node {node_id}");
        assert_eq!(result.proc_dps_max, 0.0, "node {node_id}");
        assert!(result.damage.is_some() || result.attack_damage.is_some());
        if let Some(damage) = &result.damage {
            assert_eq!(damage.final_max, 0, "node {node_id}");
            assert_eq!(damage.avg_max, 0, "node {node_id}");
        }
        if let Some(damage) = &result.attack_damage {
            assert_eq!(damage.combined_hit_max, 0, "node {node_id}");
            assert_eq!(damage.combined_avg_max, 0, "node {node_id}");
        }
        let ailment = result.ailment_dps_max.expect("burning still deals damage");
        assert!(ailment > baseline_ailment, "node {node_id}");
        assert_eq!(result.combined_dps_max, Some(ailment), "node {node_id}");
    }
}

// "Heat Combustion" inflicts burning with no amount — dropping amount-less
// states zeroed the ailment DPS of nodes whose whole point is the ailment.
#[test]
fn bare_state_without_an_amount_still_applies_the_ailment() {
    let plain = perf("pyromancer", "breath_of_fire", 10, &[], &[]);
    let burning = perf(
        "pyromancer",
        "breath_of_fire",
        10,
        &[("heat_combustion", 5)],
        &[],
    );
    assert_eq!(plain.ailment_dps_max, None);
    let Some(ailment) = burning.ailment_dps_max else {
        panic!("a bare `burning` state must still apply burning");
    };
    assert!(ailment > 0.0);
}

// Subtree multicast must work regardless of the Spell tag: multicast_chance is
// not skillScoped, so odins_fury's 15/rank lands shared and dies at the gate.
#[test]
fn verify_subtree_multicast_reaches_non_spell_odins_fury() {
    let p = perf("viking", "odins_fury", 10, &[("echo_of_duality", 2)], &[]);
    let d = p.damage.expect("odins_fury breakdown");
    assert_eq!(
        d.multicast_chance_pct, 30.0,
        "echo_of_duality rank 2 (2 x 15%) must multicast a non-Spell skill"
    );
}

// On a Spell skill the shared routing works, but the value must count exactly
// once, not shared+scoped twice. fireball:multicast 10/rank -> 50% at rank 5.
#[test]
fn verify_spell_subtree_multicast_counted_once_on_fireball() {
    let p = perf("pyromancer", "fireball", 10, &[("multicast", 5)], &[]);
    let d = p.damage.expect("fireball breakdown");
    assert_eq!(d.multicast_chance_pct, 50.0);
}

// lightning_break is skillScoped, so a subtree value once never reached the hit.
// weakening_charge 30/rank -> 150% at rank 5, only when the enemy toggle is on.
#[test]
fn verify_subtree_lightning_break_reaches_build_hit() {
    let off = perf(
        "stormweaver",
        "charged_bolts",
        10,
        &[("weakening_charge", 5)],
        &[],
    );
    let on = perf(
        "stormweaver",
        "charged_bolts",
        10,
        &[("weakening_charge", 5)],
        &[("lightning_break", true)],
    );
    let h_off = off.damage.expect("breakdown off").hit_max as f64;
    let h_on = on.damage.expect("breakdown on").hit_max as f64;
    assert!(
        (h_on / h_off - 2.5).abs() < 0.02,
        "150% lightning break should scale the hit 2.5x: {h_off} -> {h_on}"
    );
}

#[test]
fn critical_break_node_reaches_real_skill_dps_only_with_matching_break() {
    let calculate = |nodes: &[u32], active: bool| {
        perf_with_tree(
            "stormweaver",
            "charged_bolts",
            20,
            &[("weakening_charge", 5)],
            &[("lightning_break", active)],
            nodes,
        )
    };
    let base = calculate(&[], true);
    let critical = calculate(&[1824], true);
    let plain = base.damage.as_ref().unwrap();
    let changed = critical.damage.as_ref().unwrap();
    assert_eq!(plain.hit_max, changed.hit_max);
    // 150% Break: (1 + 1.5 × (1 + .15 × .25)) / (1 + 1.5).
    let expected_ratio = 1.0225;
    assert!((changed.avg_max as f64 - plain.avg_max as f64 * expected_ratio).abs() < 2.0);
    assert!(critical.avg_hit_dps_max.unwrap() > base.avg_hit_dps_max.unwrap());
    assert_eq!(
        calculate(&[1824], false).avg_hit_dps_max,
        calculate(&[], false).avg_hit_dps_max
    );
    assert_eq!(
        calculate(&[1782], true).avg_hit_dps_max,
        base.avg_hit_dps_max
    );
    for node in [704, 705, 706, 707, 708, 709, 1972, 1973, 2090] {
        let combined = calculate(&[node, 1824], true);
        let damage = combined.damage.as_ref().unwrap();
        assert_eq!(damage.elemental_break_pct, 5.0);
        // (1 + .05 universal + 1.5 typed × (1 + .15 × .25)) / 2.5 baseline.
        assert!((damage.avg_max as f64 - plain.avg_max as f64 * 1.0425).abs() < 2.0);
        assert!((damage.hit_max as f64 - plain.hit_max as f64 * 1.02).abs() < 2.0);
        assert_eq!(combined.hits_per_cast, base.hits_per_cast);
        assert!(combined.avg_hit_dps_max.unwrap() > critical.avg_hit_dps_max.unwrap());
    }
}

#[test]
fn elemental_break_uses_the_owning_skills_damage_types_even_for_hybrid_spells() {
    let base = perf_with_tree("samurai", "blade_barrier", 20, &[], &[], &[]);
    let plain = base.damage.as_ref().unwrap();
    for node in [522, 550] {
        let on_hit = perf_with_tree("samurai", "blade_barrier", 20, &[], &[], &[node]);
        let changed = on_hit.damage.as_ref().unwrap();
        assert_eq!(changed.elemental_break_pct, 20.0);
        assert!((changed.hit_max as f64 - plain.hit_max as f64 * 1.2).abs() < 2.0);
        assert_eq!(
            on_hit.attack_damage.as_ref().unwrap().physical_hit_max,
            base.attack_damage.as_ref().unwrap().physical_hit_max,
        );
        let fireball = perf_with_tree("pyromancer", "fireball", 20, &[], &[], &[node]);
        assert_eq!(fireball.damage.unwrap().elemental_break_pct, 0.0);
    }
    let spell_node = perf_with_tree("samurai", "blade_barrier", 20, &[], &[], &[704]);
    assert_eq!(spell_node.damage.unwrap().elemental_break_pct, 0.0);
    let fireball = perf_with_tree("pyromancer", "fireball", 20, &[], &[], &[704]);
    assert_eq!(fireball.damage.unwrap().elemental_break_pct, 5.0);
}

#[test]
fn incarnation_leap_nodes_reach_confirmed_skills_without_affecting_other_movement() {
    let recipients = [
        ("amazon", "leaping_ambush"),
        ("amazon", "storm_dash"),
        ("demon_slayer", "fast_slices"),
        ("illusionist", "link_of_sand"),
        ("jotunn", "freezing_leap"),
        ("marauder", "crazy_grapple"),
        ("marksman", "vault"),
        ("paladin", "ball_lightning"),
        ("pirate", "grenade_jump"),
        ("prophet", "leaping_charge"),
    ];
    for (class, id) in recipients {
        let skill = data::get_skills_by_class(class)
            .iter()
            .find(|s| s.id == id)
            .unwrap();
        assert!(
            skill
                .tags
                .as_deref()
                .unwrap_or_default()
                .iter()
                .any(|tag| tag == "Leap"),
            "{class}/{id}"
        );
    }

    // The six direct elemental spells exercise both flat and percentage nodes.
    // Link of Sand has no direct damage; its tag must not manufacture a hit.
    for (class, id) in [
        ("amazon", "leaping_ambush"),
        ("amazon", "storm_dash"),
        ("jotunn", "freezing_leap"),
        ("marauder", "crazy_grapple"),
        ("paladin", "ball_lightning"),
        ("pirate", "grenade_jump"),
    ] {
        let base = perf(class, id, 20, &[], &[]);
        for node in [1887, 1888, 1889, 1891, 1966, 1967, 1968, 1969, 1890, 1970] {
            let changed = perf_with_tree(class, id, 20, &[], &[], &[node]);
            assert!(
                changed.damage.as_ref().unwrap().hit_max > base.damage.as_ref().unwrap().hit_max,
                "{id}, node {node}: hit"
            );
            assert!(
                changed.avg_hit_dps_max.unwrap() > base.avg_hit_dps_max.unwrap(),
                "{id}, node {node}: DPS"
            );
            assert_eq!(changed.hits_per_cast, base.hits_per_cast);
        }
    }
    for (class, id) in [
        ("pyromancer", "fireball"),
        ("pyromancer", "phoenix_flight"),
        ("illusionist", "link_of_sand"),
        ("illusionist", "dimensional_displacement"),
    ] {
        let base = perf(class, id, 20, &[], &[]);
        let changed = perf_with_tree(class, id, 20, &[], &[], &[1887, 1890]);
        assert_eq!(changed.avg_hit_dps_max, base.avg_hit_dps_max, "{id}");
    }
    // Arena Master changes Crazy Grapple's movement, but the game calculates
    // its damage from the same Leap-tagged talent before applying the variant.
    let arena_ranks = HashMap::from([(subskill_key("crazy_grapple", "arena_master"), 3)]);
    let grapple = data::get_skills_by_class("marauder")
        .iter()
        .find(|skill| skill.id == "crazy_grapple")
        .unwrap();
    let tags = super::super::subskill::effective_skill_tags(
        "crazy_grapple",
        grapple.tags.as_deref().unwrap(),
        &arena_ranks,
    );
    assert!(!tags.iter().any(|tag| tag == "Movement"));
    assert!(tags.iter().any(|tag| tag == "Leap"));
    let arena = perf("marauder", "crazy_grapple", 20, &[("arena_master", 3)], &[]);
    for node in [1887, 1890] {
        let changed = perf_with_tree(
            "marauder",
            "crazy_grapple",
            20,
            &[("arena_master", 3)],
            &[],
            &[node],
        );
        assert!(
            changed.damage.as_ref().unwrap().hit_max > arena.damage.as_ref().unwrap().hit_max,
            "Arena Master, Leap node {node}"
        );
        assert_eq!(changed.hits_per_cast, arena.hits_per_cast);
    }

    // The physical weapon-coefficient branch does not consume the flat pool.
    for (class, id) in [
        ("demon_slayer", "fast_slices"),
        ("marksman", "vault"),
        ("prophet", "leaping_charge"),
    ] {
        let base = perf(class, id, 20, &[], &[]);
        let changed = perf_with_tree(class, id, 20, &[], &[], &[1887]);
        assert_eq!(
            changed.attack_damage.as_ref().unwrap().physical_hit_max,
            base.attack_damage.as_ref().unwrap().physical_hit_max,
            "{id}"
        );
        assert_eq!(changed.avg_hit_dps_max, base.avg_hit_dps_max, "{id}");
        let percent = perf_with_tree(class, id, 20, &[], &[], &[1890]);
        assert!(
            percent.attack_damage.as_ref().unwrap().physical_hit_max
                > base.attack_damage.as_ref().unwrap().physical_hit_max,
            "{id}: percentage bonus"
        );
    }
}

// brutalizing_slash is an attack with no elemental breakdown, so its subtree
// conversion_strength has nowhere to land — the swing must not stay flat.
#[test]
fn verify_conversion_feeds_attack_skill_without_elemental_formula() {
    let plain = perf("butcher", "brutalizing_slash", 10, &[], &[]);
    let conv = perf(
        "butcher",
        "brutalizing_slash",
        10,
        &[("gutting_frenzy", 3)],
        &[],
    );
    let a = plain
        .attack_damage
        .expect("attack breakdown")
        .combined_avg_max;
    let b = conv
        .attack_damage
        .expect("attack breakdown")
        .combined_avg_max;
    assert!(
        b > a,
        "conversion_strength (30% proc x 45% of strength) must raise the swing: {a} vs {b}"
    );
}

#[test]
fn empty_build_produces_no_damage_no_proc() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let subskill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let enemy_conditions = HashMap::new();
    let player_conditions = HashMap::new();
    let skill_projectiles = HashMap::new();
    let enemy_resistances = HashMap::new();
    let proc_toggles = HashMap::new();
    let deps = empty_deps(
        &allocated,
        &inventory,
        &skill_ranks,
        &subskill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &enemy_conditions,
        &player_conditions,
        &skill_projectiles,
        &enemy_resistances,
        &proc_toggles,
    );
    let perf = compute_build_performance(&deps);
    assert!(perf.damage.is_none());
    assert_eq!(perf.proc_dps_min, 0.0);
    assert_eq!(perf.proc_dps_max, 0.0);
    assert_eq!(perf.hit_dps_min, None);
    assert_eq!(perf.combined_dps_min, None);
    assert_eq!(perf.active_skill_name, None);
    assert!(!perf.stats.is_empty(), "default base stats should populate");
}

#[test]
fn class_with_active_skill_produces_damage() {
    // This empty-inventory smoke test needs a skill without a weapon requirement.
    // HashMap order could previously choose Buckshot and correctly reject its cast.
    let class_id = "pyromancer";
    let skill_id = "fireball";

    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let mut skill_ranks = HashMap::new();
    skill_ranks.insert(skill_id.to_string(), 10_u32);
    let subskill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let enemy_conditions = HashMap::new();
    let player_conditions = HashMap::new();
    let skill_projectiles = HashMap::new();
    let enemy_resistances = HashMap::new();
    let proc_toggles = HashMap::new();

    let mut deps = empty_deps(
        &allocated,
        &inventory,
        &skill_ranks,
        &subskill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &enemy_conditions,
        &player_conditions,
        &skill_projectiles,
        &enemy_resistances,
        &proc_toggles,
    );
    deps.class_id = Some(class_id);
    deps.level = 50;
    deps.main_skill_id = Some(skill_id);

    let perf = compute_build_performance(&deps);
    assert!(
        perf.damage.is_some(),
        "expected damage breakdown for active skill '{skill_id}'"
    );
    assert!(perf.active_skill_name.is_some());
}

#[test]
fn active_skill_without_rank_yields_no_damage() {
    let pick = data::data()
        .skills_by_class
        .iter()
        .find_map(|(cid, skills)| {
            skills.iter().find_map(|s| {
                if s.kind != SkillKind::Active {
                    return None;
                }
                if s.damage_formula.is_none() && s.damage_per_rank.is_none() {
                    return None;
                }
                Some((cid.clone(), s.id.clone()))
            })
        });
    let Some((class_id, skill_id)) = pick else {
        eprintln!("no active skill; skipping");
        return;
    };

    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks: HashMap<String, u32> = HashMap::new();
    let subskill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let enemy_conditions = HashMap::new();
    let player_conditions = HashMap::new();
    let skill_projectiles = HashMap::new();
    let enemy_resistances = HashMap::new();
    let proc_toggles = HashMap::new();

    let mut deps = empty_deps(
        &allocated,
        &inventory,
        &skill_ranks,
        &subskill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &enemy_conditions,
        &player_conditions,
        &skill_projectiles,
        &enemy_resistances,
        &proc_toggles,
    );
    deps.class_id = Some(&class_id);
    deps.main_skill_id = Some(&skill_id);

    let perf = compute_build_performance(&deps);
    assert!(perf.damage.is_none());
    assert!(perf.hit_dps_min.is_none());
}

// Item-granted procs (charms like The Eye): flat typed damage per rank on
// an internal cooldown, gated by a `granted:{id}` proc toggle.
#[test]
fn item_granted_proc_damage_adds_proc_dps() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let skill_ranks = HashMap::new();
    let subskill_ranks = HashMap::new();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let enemy_conditions = HashMap::new();
    let player_conditions = HashMap::new();
    let skill_projectiles = HashMap::new();
    let enemy_resistances = HashMap::new();
    let proc_toggles: HashMap<String, bool> = [("granted:the_eye".to_string(), true)]
        .into_iter()
        .collect();
    let granted_ranks: HashMap<String, Ranged> = [("the eye".to_string(), (10.0, 20.0))]
        .into_iter()
        .collect();

    let mut deps = empty_deps(
        &allocated,
        &inventory,
        &skill_ranks,
        &subskill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &enemy_conditions,
        &player_conditions,
        &skill_projectiles,
        &enemy_resistances,
        &proc_toggles,
    );
    deps.granted_skill_ranks = Some(&granted_ranks);

    let perf = compute_build_performance(&deps);
    // item-granted-skills.json The Eye: procDamage arcane 19.5/rank,
    // interval max(procCooldown 0.25, ICD 1.5) = 1.5s, no stats/resists.
    let expected_min = 19.5 * 10.0 / 1.5;
    let expected_max = 19.5 * 20.0 / 1.5;
    assert!(
        (perf.proc_dps_min - expected_min).abs() < 1e-6,
        "proc_dps_min = {}, expected {expected_min}",
        perf.proc_dps_min
    );
    assert!(
        (perf.proc_dps_max - expected_max).abs() < 1e-6,
        "proc_dps_max = {}, expected {expected_max}",
        perf.proc_dps_max
    );

    let toggles_off: HashMap<String, bool> = HashMap::new();
    deps.proc_toggles = &toggles_off;
    let perf_off = compute_build_performance(&deps);
    assert_eq!(perf_off.proc_dps_min, 0.0);
    assert_eq!(perf_off.proc_dps_max, 0.0);

    let asphyxiating = HashSet::from([312]);
    deps.allocated_tree_nodes = &asphyxiating;
    deps.proc_toggles = &proc_toggles;
    let ailments_only = compute_build_performance(&deps);
    assert_eq!(ailments_only.proc_dps_min, 0.0);
    assert_eq!(ailments_only.proc_dps_max, 0.0);
    assert_eq!(ailments_only.combined_dps_min, None);
    assert_eq!(ailments_only.combined_dps_max, None);
}

#[test]
fn orb_of_frost_rate_follows_cooldown_and_skill_haste() {
    let base = perf_with_stats("jotunn", "orb_of_frost", 10, &[], &[], 1.0);
    let with_fcr = perf_with_stats(
        "jotunn",
        "orb_of_frost",
        10,
        &[],
        &[("faster_cast_rate", "100")],
        1.0,
    );
    let with_haste = perf_with_stats(
        "jotunn",
        "orb_of_frost",
        10,
        &[],
        &[("skill_haste", "75")],
        1.0,
    );

    let (Some(base_dps), Some(fcr_dps), Some(haste_dps), Some(hit)) = (
        base.avg_hit_dps_max,
        with_fcr.avg_hit_dps_max,
        with_haste.avg_hit_dps_max,
        base.damage.as_ref().map(|d| d.avg_max as f64),
    ) else {
        panic!("expected dps for orb_of_frost");
    };

    assert!(
        (base_dps / hit - 1.0 / 1.75).abs() < 1e-9,
        "base rate should be one cast per 1.75 s cooldown, got {}",
        base_dps / hit
    );
    assert_eq!(fcr_dps, base_dps, "faster cast rate must not touch the orb");
    assert!(
        (haste_dps / base_dps - 1.375).abs() < 1e-9,
        "75 skill haste gives 1 + 75 × 0.005 = 1.375x, got x{}",
        haste_dps / base_dps
    );

    let tundra = perf_with_stats(
        "jotunn",
        "orb_of_frost",
        10,
        &[("timeless_tundra", 5)],
        &[],
        1.0,
    );
    let Some(tundra_dps) = tundra.avg_hit_dps_max else {
        panic!("expected dps for orb_of_frost");
    };
    assert!(
        (tundra_dps / base_dps - 1.25).abs() < 1e-9,
        "Timeless Tundra's 50 skill haste gives 1.25x, got x{}",
        tundra_dps / base_dps
    );
}

// Blazing Trail's fire lives 2.5 s and re-arms every 0.5 s, so one cast lands
// floor(2.5 / 0.5) + 1 = 6 hits on a target that stays in it.
#[test]
fn hit_model_multiplies_dps_by_hits_per_cast() {
    let trail = perf("pyromancer", "blazing_trail", 10, &[], &[]);
    assert_eq!(trail.hits_per_cast, Some((6.0, 6.0)));
    let (Some(dps), Some(damage)) = (trail.avg_hit_dps_max, trail.damage.as_ref()) else {
        panic!("expected dps for blazing_trail");
    };
    // baseCastRate 1/s, no FCR: the whole DPS is damage x hits.
    assert!(
        (dps / damage.avg_max as f64 - 6.0).abs() < 1e-9,
        "6 hits per cast should be 6x the hit damage, got x{}",
        dps / damage.avg_max as f64
    );

    let fireball = perf("pyromancer", "fireball", 10, &[], &[]);
    assert_eq!(
        fireball.hits_per_cast, None,
        "a skill without a hit model hits once"
    );
}

// Timed Fire adds 10% Skill Duration per rank: 2.5 s -> 3.75 s buys two more ticks.
#[test]
fn skill_duration_buys_extra_ticks() {
    let base = perf("pyromancer", "blazing_trail", 10, &[], &[]);
    let longer = perf("pyromancer", "blazing_trail", 10, &[("timed_fire", 5)], &[]);
    assert_eq!(longer.hits_per_cast, Some((8.0, 8.0)));
    let (Some(base_dps), Some(long_dps)) = (base.avg_hit_dps_max, longer.avg_hit_dps_max) else {
        panic!("expected dps for blazing_trail");
    };
    assert!(
        (long_dps / base_dps - 8.0 / 6.0).abs() < 1e-9,
        "8 hits over 6 should be 1.33x, got x{}",
        long_dps / base_dps
    );
}

// "Increased Damage when wielding an Axe" (node 683) multiplies the whole
// physical hit, so its worth must not decay as flat physical is stacked.
#[test]
fn axe_damage_node_is_independent_of_flat_physical() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let dps = |nodes: &HashSet<u32>, add_phys: f64| -> f64 {
        let allocated = HashMap::new();
        let mut inventory: Inventory = HashMap::new();
        inventory.insert(
            "weapon".to_string(),
            EquippedItem {
                base_id: "base_melee_hand_axe".to_string(),
                ..Default::default()
            },
        );
        let mut skill_ranks = HashMap::new();
        skill_ranks.insert("furious_strike".to_string(), 20u32);
        let subskill_ranks = HashMap::new();
        let active_buffs = HashMap::new();
        let custom_stats: Vec<CustomStat> = vec![CustomStat {
            stat_key: "additive_physical_damage".to_string(),
            value: format!("{add_phys}"),
        }];
        let tree_socketed = HashMap::new();
        let enemy_conditions = HashMap::new();
        let player_conditions = HashMap::new();
        let skill_projectiles = HashMap::new();
        let enemy_resistances = HashMap::new();
        let proc_toggles = HashMap::new();
        let mut deps = empty_deps(
            &allocated,
            &inventory,
            &skill_ranks,
            &subskill_ranks,
            &active_buffs,
            &custom_stats,
            nodes,
            &tree_socketed,
            &enemy_conditions,
            &player_conditions,
            &skill_projectiles,
            &enemy_resistances,
            &proc_toggles,
        );
        deps.class_id = Some("butcher");
        deps.level = 50;
        deps.main_skill_id = Some("furious_strike");
        compute_build_performance(&deps)
            .combined_dps_max
            .unwrap_or(0.0)
    };

    let node: HashSet<u32> = [683].into_iter().collect();
    let none: HashSet<u32> = HashSet::new();
    let gain = |add_phys: f64| dps(&node, add_phys) / dps(&none, add_phys) - 1.0;

    let bare = gain(0.0);
    let geared = gain(5000.0);
    assert!(
        bare > 0.10,
        "node 683 should be worth well over 10% on a bare build, got {:.3}%",
        bare * 100.0
    );
    assert!(
        (bare - geared).abs() < 0.01,
        "node 683 must not decay with flat physical: {:.3}% bare vs {:.3}% with +5000 flat",
        bare * 100.0,
        geared * 100.0
    );
}

// Frost Sunder throws a fan of 4 icicles before any subskill adds more.
#[test]
fn frost_sunder_starts_at_four_projectiles() {
    let p = perf("jotunn", "frost_sunder", 20, &[], &[]);
    let attack = p
        .attack_damage
        .as_ref()
        .expect("frost sunder attack breakdown");
    assert_eq!(attack.projectile_count, 4);
    // Frost Shrapnel adds 2 per rank on top of the base.
    let boosted = perf("jotunn", "frost_sunder", 20, &[("frost_shrapnel", 3)], &[]);
    let boosted_attack = boosted.attack_damage.as_ref().expect("boosted breakdown");
    assert_eq!(boosted_attack.projectile_count, 4 + 6);
}

// Frost Sunder Onslaught is an explosion layered on the icicle, not a bigger
// icicle: its damage sits on top instead of joining the cold skill damage pool,
// so a build already stacking cold damage gets the same multiplier out of it.
#[test]
fn frost_sunder_onslaught_lands_on_top_of_the_cold_pool() {
    let _scope = crate::calc::season::SeasonScope::enter(Some("s10".to_string()));
    let cold_avg = |rank: u32, cold_pool: f64| -> f64 {
        let allocated = HashMap::new();
        let inventory = HashMap::new();
        let mut skill_ranks = HashMap::new();
        skill_ranks.insert("frost_sunder".to_string(), 20u32);
        let mut subskill_ranks = HashMap::new();
        if rank > 0 {
            subskill_ranks.insert(subskill_key("frost_sunder", "frost_sunder_onslaught"), rank);
        }
        let active_buffs = HashMap::new();
        let custom_stats: Vec<CustomStat> = vec![CustomStat {
            stat_key: "cold_skill_damage".to_string(),
            value: format!("{cold_pool}"),
        }];
        let alloc_tree = HashSet::new();
        let tree_socketed = HashMap::new();
        let enemy_conditions = HashMap::new();
        let player_conditions = HashMap::new();
        let skill_projectiles = HashMap::new();
        let enemy_resistances = HashMap::new();
        let proc_toggles = HashMap::new();
        let mut deps = empty_deps(
            &allocated,
            &inventory,
            &skill_ranks,
            &subskill_ranks,
            &active_buffs,
            &custom_stats,
            &alloc_tree,
            &tree_socketed,
            &enemy_conditions,
            &player_conditions,
            &skill_projectiles,
            &enemy_resistances,
            &proc_toggles,
        );
        deps.class_id = Some("jotunn");
        deps.level = 50;
        deps.main_skill_id = Some("frost_sunder");
        compute_build_performance(&deps)
            .attack_damage
            .map(|a| a.poison_avg_max as f64)
            .unwrap_or(0.0)
    };

    // Rank 3 is 3 x 75% of total damage on top of the hit.
    for pool in [0.0, 500.0] {
        let ratio = cold_avg(3, pool) / cold_avg(0, pool);
        assert!(
            (ratio - 3.25).abs() < 0.02,
            "onslaught must be worth 3.25x with a {pool}% cold pool, got {ratio:.3}"
        );
    }
}

fn winters_bite_perf(toggled: bool, subskills: &[(&str, u32)]) -> BuildPerformance {
    let allocated = HashMap::new();
    let mut inventory: Inventory = HashMap::new();
    inventory.insert(
        "weapon".to_string(),
        EquippedItem {
            base_id: "axe_heroic_winter_s_bite".to_string(),
            ..Default::default()
        },
    );
    let mut skill_ranks = HashMap::new();
    skill_ranks.insert("frost_sunder".to_string(), 10);
    let subskill_ranks: HashMap<String, u32> = subskills
        .iter()
        .map(|(id, r)| (subskill_key("breath_of_ice", id), *r))
        .collect();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let enemy_conditions = HashMap::new();
    let player_conditions = HashMap::new();
    let skill_projectiles = HashMap::new();
    let enemy_resistances = HashMap::new();
    let mut proc_toggles = HashMap::new();
    if toggled {
        proc_toggles.insert(
            "cast:axe_heroic_winter_s_bite:breath of ice".to_string(),
            true,
        );
    }

    let mut deps = empty_deps(
        &allocated,
        &inventory,
        &skill_ranks,
        &subskill_ranks,
        &active_buffs,
        &custom_stats,
        &alloc_tree,
        &tree_socketed,
        &enemy_conditions,
        &player_conditions,
        &skill_projectiles,
        &enemy_resistances,
        &proc_toggles,
    );
    deps.class_id = Some("jotunn");
    deps.level = 50;
    deps.main_skill_id = Some("frost_sunder");
    compute_build_performance(&deps)
}

// Winter's Bite: "18% Chance on Hit to cast Breath of Ice Level 60". The build
// never learned Breath of Ice; the item casts it at its own level.
#[test]
fn item_proc_casts_a_class_skill_and_its_subtree_counts() {
    let off = winters_bite_perf(false, &[]);
    assert_eq!(off.proc_dps_max, 0.0, "an untoggled item proc pays nothing");

    let on = winters_bite_perf(true, &[]);
    assert!(
        on.proc_dps_max > 0.0,
        "the cast must reach proc dps even at rank 0 of Breath of Ice"
    );

    let with_subtree = winters_bite_perf(true, &[("fresh_mint", 5)]);
    assert!(
        with_subtree.proc_dps_max > on.proc_dps_max,
        "points spent in Breath of Ice's subtree lift the skill the item casts"
    );
}

#[test]
fn effective_projectile_count_caps_then_repeats_volleys() {
    let stats = |cap: f64, volleys: f64| {
        move |k: &str| match k {
            "projectile_count" => 6.0,
            "single_target_hit_cap" => cap,
            "extra_volleys_pct" => volleys,
            _ => 0.0,
        }
    };
    assert_eq!(effective_projectile_count(4, &stats(0.0, 0.0)), 10);
    assert_eq!(effective_projectile_count(4, &stats(2.0, 0.0)), 2);
    // 10 projectiles, 45% chance of 3 extra waves: 10 * 2.35 = 23.5 -> 24.
    assert_eq!(effective_projectile_count(4, &stats(0.0, 135.0)), 24);
    assert_eq!(effective_projectile_count(4, &stats(2.0, 135.0)), 5);
}

// jotunn:frozen_boulder "Boulder Barrage": 5 boulders in an arc, but only 2
// reach one target (single_target_hit_cap), on top of +105% cold damage.
#[test]
fn arc_hit_cap_limits_projectiles_on_one_target() {
    let base = perf("jotunn", "frozen_boulder", 10, &[], &[]);
    let barrage = perf(
        "jotunn",
        "frozen_boulder",
        10,
        &[("boulder_barrage", 3)],
        &[],
    );
    let (Some(one), Some(arc)) = (base.damage, barrage.damage) else {
        panic!("expected a breakdown for frozen_boulder");
    };
    assert_eq!(one.projectile_count, 1);
    assert_eq!(
        arc.projectile_count, 2,
        "5 boulders in an arc, 2 reach one target"
    );
    assert!(
        arc.avg_max > 2 * one.avg_max,
        "+105% cold damage rides on top of the 2 hits"
    );
}

// jotunn:frost_sunder "Gelid Riptide": 45% chance of 3 extra waves repeats the
// whole 10-icicle volley, so hits go x2.35 (rounded to 24 of 10), not +1.
#[test]
fn wave_proc_repeats_the_whole_volley() {
    let volley = perf("jotunn", "frost_sunder", 10, &[("frost_shrapnel", 3)], &[]);
    let waves = perf(
        "jotunn",
        "frost_sunder",
        10,
        &[("frost_shrapnel", 3), ("gelid_riptide", 3)],
        &[],
    );
    let (Some(one), Some(rip)) = (volley.attack_damage, waves.attack_damage) else {
        panic!("expected an attack breakdown for frost_sunder");
    };
    assert_eq!(one.projectile_count, 10);
    assert_eq!(rip.projectile_count, 24, "10 icicles x 2.35 waves, rounded");
}

// pyromancer:hydra "Scorching Kraken": one massive hydra instead of a pack,
// +45% damage per hydra the build could have fielded.
#[test]
fn single_entity_note_pins_the_count_and_scales_with_max_amount() {
    let pack = perf(
        "pyromancer",
        "hydra",
        10,
        &[("more_heads_more_fire", 5)],
        &[],
    );
    assert_eq!(pack.entity_count, Some((11.0, 11.0)));

    let kraken = perf("pyromancer", "hydra", 10, &[("scorching_kraken", 3)], &[]);
    let big_kraken = perf(
        "pyromancer",
        "hydra",
        10,
        &[("more_heads_more_fire", 5), ("scorching_kraken", 3)],
        &[],
    );
    assert_eq!(kraken.entity_count, Some((1.0, 1.0)));
    assert_eq!(big_kraken.entity_count, Some((1.0, 1.0)));
    let (Some(one), Some(eleven)) = (kraken.avg_hit_dps_max, big_kraken.avg_hit_dps_max) else {
        panic!("expected dps for hydra");
    };
    assert!(
        eleven > one,
        "max hydra amount must feed the kraken's damage"
    );
}

// jotunn:avalanche has a 3 s cooldown; skill haste (Ephemeral Flurry +35%)
// must scale its cast rate, which only the cooldown-gated path does.
#[test]
fn avalanche_is_cooldown_gated_and_scales_with_skill_haste() {
    let base = perf("jotunn", "avalanche", 10, &[], &[]);
    let hasted = perf("jotunn", "avalanche", 10, &[("ephemeral_flurry", 5)], &[]);
    let (Some(slow), Some(fast)) = (base.avg_hit_dps_max, hasted.avg_hit_dps_max) else {
        panic!("expected dps for avalanche");
    };
    assert!(
        (fast / slow - 1.175).abs() < 1e-9,
        "35 skill haste gives 1 + 35 × 0.005 = 1.175x, got x{}",
        fast / slow
    );
}

#[test]
fn native_calculation_steps_reconcile_real_spell_attack_and_entity_dps() {
    for (class, skill) in [
        ("stormweaver", "charged_bolts"),
        ("jotunn", "frost_sunder"),
        ("marksman", "gunner_drone"),
    ] {
        let result = perf(class, skill, 20, &[], &[]);
        let combined = result
            .calculation()
            .iter()
            .find(|step| step.label() == "Combined DPS")
            .expect(skill);
        assert_eq!(
            combined.value(),
            (
                result.combined_dps_min.unwrap(),
                result.combined_dps_max.unwrap()
            )
        );
        let hit = result
            .calculation()
            .iter()
            .find(|step| step.label() == "Average hit DPS")
            .unwrap();
        assert_eq!(
            hit.value(),
            (
                result.avg_hit_dps_min.unwrap(),
                result.avg_hit_dps_max.unwrap()
            )
        );
        assert!(!result.calculation_sources().is_empty());
        let json = serde_json::to_value(result).unwrap();
        assert!(json.get("calculation").is_none());
        assert!(json.get("calculationSources").is_none());
    }
}

#[test]
fn blender_uses_game_weapon_coefficient_without_invented_flat_damage() {
    for rank in [1, 20] {
        let result = perf("butcher", "blender", rank, &[], &[]);
        let attack = result.attack_damage.as_ref().unwrap();
        // TalentsButcher: projDamage * (1 + (75 + 15 * rank) / 100).
        assert_eq!(attack.weapon_damage_pct_min, 175.0 + 15.0 * rank as f64);
        assert_eq!(attack.weapon_damage_pct_max, attack.weapon_damage_pct_min);
        assert_eq!(attack.skill_flat_phys_min, 0.0);
        assert_eq!(attack.skill_flat_phys_max, 0.0);
    }
}

#[test]
fn nanoblenders_average_contacts_and_separate_blade_count() {
    let base = perf(
        "butcher",
        "blender",
        20,
        &[("a_i_empowered_nanoblenders", 1)],
        &[],
    );
    let attack = base.attack_damage.as_ref().unwrap();
    assert_eq!(
        attack.projectile_count, 6,
        "3 nanoblenders with 2 blades each"
    );
    assert_eq!(base.hits_per_cast, Some((2.5, 2.5)));
    assert!((base.avg_hit_dps_max.unwrap() / attack.combined_avg_max as f64 - 0.3125).abs() < 1e-9);
    let extra = perf(
        "butcher",
        "blender",
        20,
        &[
            ("a_i_empowered_nanoblenders", 3),
            ("extra_cleaver_addon", 2),
        ],
        &[],
    );
    assert_eq!(extra.attack_damage.as_ref().unwrap().projectile_count, 30);
    assert!(base
        .calculation()
        .iter()
        .any(|s| s.label() == "Nanoblender contact estimate"));
}

#[test]
fn nanoblenders_use_duration_and_haste_not_weapon_or_cast_speed() {
    let nodes = [("a_i_empowered_nanoblenders", 1)];
    let base = perf_with_stats("butcher", "blender", 20, &nodes, &[], 1.0);
    for key in ["increased_attack_speed", "faster_cast_rate"] {
        let faster = perf_with_stats("butcher", "blender", 20, &nodes, &[(key, "100")], 1.0);
        assert_eq!(base.avg_hit_dps_max, faster.avg_hit_dps_max, "{key}");
    }
    for key in [
        "skill_duration",
        "skill_haste",
        "orbital_skill_duration",
        "orbital_skill_speed",
    ] {
        let boosted = perf_with_stats("butcher", "blender", 20, &nodes, &[(key, "100")], 1.0);
        let expected = if key == "skill_haste" { 1.5 } else { 2.0 };
        assert!(
            (boosted.avg_hit_dps_max.unwrap() / base.avg_hit_dps_max.unwrap() - expected).abs()
                < 1e-9,
            "{key}"
        );
    }
    let unlearned = perf("butcher", "blender", 0, &nodes, &[]);
    assert!(unlearned.avg_hit_dps_max.is_none());
}

#[test]
fn blender_every_node_has_an_explicit_calculation_effect() {
    let base = perf("butcher", "blender", 20, &[], &[]);
    assert_eq!(base.attack_damage.as_ref().unwrap().projectile_count, 2);
    for (node, label) in [
        ("sharpened_cleavers", "Sharpened Cleavers"),
        ("industrial_sized", "Industrial Sized"),
        ("no_bits_nor_pieces", "No Bits Nor Pieces"),
        ("ragefueled_energy", "Ragefueled Energy"),
        ("extra_cleaver_addon", "Extra Cleaver Addon"),
        ("obsidian_blades", "Obsidian Blades"),
        ("attachment_malfunction", "Attachment Malfunction"),
        ("it_will_blend", "It Will Blend"),
        (
            "blood_thirsting_killing_machine",
            "Blood Thirsting Killing Machine",
        ),
        ("will_it_blend", "Will it Blend?"),
        ("a_i_empowered_nanoblenders", "A.I. Empowered Nanoblenders"),
        ("blenderang", "Blenderang"),
        ("attachable_microblades", "Attachable Microblades"),
        ("blending_blood_pact", "Blending Blood Pact"),
    ] {
        let changed = perf("butcher", "blender", 20, &[(node, 1)], &[]);
        assert!(
            changed
                .calculation()
                .iter()
                .any(|s| s.label().starts_with("Blender · ") && s.label().contains(label)),
            "missing node {node}: {:?}",
            changed.calculation()
        );
        match node {
            "attachment_malfunction"
            | "blood_thirsting_killing_machine"
            | "no_bits_nor_pieces"
            | "it_will_blend" => {
                assert_eq!(base.avg_hit_dps_max, changed.avg_hit_dps_max, "{node}")
            }
            "attachable_microblades" => {
                assert_eq!(base.avg_hit_dps_max, changed.avg_hit_dps_max);
                assert!(changed.proc_dps_max > 0.0);
            }
            "blenderang" | "obsidian_blades" => {}
            _ => assert!(changed.avg_hit_dps_max > base.avg_hit_dps_max, "{node}"),
        }
    }
    let accuracy = perf("butcher", "blender", 20, &[("no_bits_nor_pieces", 5)], &[]);
    assert_eq!(
        accuracy.attack_damage.unwrap().attack_rating_pct_max
            - base.attack_damage.unwrap().attack_rating_pct_max,
        50.0
    );
}

#[test]
fn blender_conditional_critical_and_additive_damage_bonuses() {
    let base = perf("butcher", "blender", 20, &[], &[("bleeding", true)]);
    let bleeding = perf(
        "butcher",
        "blender",
        20,
        &[("it_will_blend", 5)],
        &[("bleeding", true)],
    );
    assert!(bleeding.avg_hit_dps_max > base.avg_hit_dps_max);
    let crit = perf_with_stats(
        "butcher",
        "blender",
        20,
        &[],
        &[("crit_chance", "100")],
        1.0,
    );
    let obsidian = perf_with_stats(
        "butcher",
        "blender",
        20,
        &[("obsidian_blades", 5)],
        &[("crit_chance", "100")],
        1.0,
    );
    assert!(obsidian.avg_hit_dps_max > crit.avg_hit_dps_max);
    let stacked = perf(
        "butcher",
        "blender",
        20,
        &[
            ("sharpened_cleavers", 5),
            ("will_it_blend", 5),
            ("blending_blood_pact", 3),
        ],
        &[],
    );
    let mult = stacked
        .attack_damage
        .as_ref()
        .unwrap()
        .calculation()
        .iter()
        .find(|s| s.label() == "Physical skill multiplier")
        .unwrap()
        .value();
    assert_eq!(mult, (3.9, 3.9)); // 40 + 25 + 225%, additive.
}

#[test]
fn blender_microblades_are_separate_non_recursive_and_scale_with_contacts() {
    let nodes = [
        ("a_i_empowered_nanoblenders", 1),
        ("attachable_microblades", 3),
    ];
    let result = perf("butcher", "blender", 20, &nodes, &[]);
    let expected = 0.15 * 2.05 * 3.0 * ((240.0 * 2.0 + 17.1 * 4.0) / 360.0) * 0.75;
    assert!((result.proc_dps_max / result.avg_hit_dps_max.unwrap() - expected).abs() < 1e-9);
    let extra = perf(
        "butcher",
        "blender",
        20,
        &[nodes[0], nodes[1], ("extra_cleaver_addon", 1)],
        &[],
    );
    assert!((extra.proc_dps_max / result.proc_dps_max - 2.0).abs() < 0.03);
    let unlearned = perf("butcher", "blender", 0, &nodes, &[]);
    assert_eq!(unlearned.proc_dps_max, 0.0);
    let zero = perf_with_stats(
        "butcher",
        "blender",
        20,
        &nodes,
        &[("skill_duration", "-100")],
        1.0,
    );
    assert_eq!(zero.proc_dps_max, 0.0);
}

#[test]
fn projectile_damage_affix_follows_blender_transformation_tags() {
    for (nodes, expected_multiplier) in [
        (vec![], 4.75),
        (vec![("blenderang", 1)], 8.5),
        (vec![("a_i_empowered_nanoblenders", 1)], 4.75),
        (
            vec![("blenderang", 1), ("a_i_empowered_nanoblenders", 1)],
            4.75,
        ),
    ] {
        let result = perf_with_stats(
            "butcher",
            "blender",
            20,
            &nodes,
            &[("projectile_damage_increase", "100")],
            1.0,
        );
        let step = result
            .attack_damage
            .as_ref()
            .unwrap()
            .calculation()
            .iter()
            .find(|step| step.label() == "Skill weapon multiplier")
            .unwrap();
        // Generic projectile damage boosts the 375% bonus only when Projectile
        // is present. Nano's earlier replacement tags take precedence.
        assert_eq!(step.value(), (expected_multiplier, expected_multiplier));
    }
}

#[test]
fn nanoblender_tags_take_precedence_over_blenderang() {
    for (nodes, projectile_bonus) in [
        (vec![("blenderang", 1)], 10.0),
        (vec![("a_i_empowered_nanoblenders", 1)], 0.0),
        (
            vec![("blenderang", 1), ("a_i_empowered_nanoblenders", 1)],
            0.0,
        ),
    ] {
        let result = perf_with_stats(
            "butcher",
            "blender",
            20,
            &nodes,
            &[("projectile_skills", "10")],
            1.0,
        );
        assert_eq!(
            result.attack_damage.unwrap().effective_rank_max,
            20.0 + projectile_bonus,
            "{nodes:?}"
        );
    }
}

#[test]
fn blenderang_haste_and_hybrid_orbit_are_accounted_for() {
    let returning = perf("butcher", "blender", 20, &[("blenderang", 3)], &[]);
    assert_eq!(returning.hits_per_cast, Some((1.5, 1.5)));
    assert_eq!(
        returning
            .attack_damage
            .as_ref()
            .unwrap()
            .attacks_per_second_max,
        0.171875 // (1 + 75 × 0.005) / 8 seconds.
    );
    let hybrid = perf(
        "butcher",
        "blender",
        20,
        &[("blenderang", 3), ("a_i_empowered_nanoblenders", 1)],
        &[],
    );
    assert_eq!(hybrid.hits_per_cast, Some((2.5, 2.5)));
    assert_eq!(hybrid.attack_damage.as_ref().unwrap().projectile_count, 6);
    let plain = perf("butcher", "blender", 20, &[], &[]);
    let fast = perf_with_stats(
        "butcher",
        "blender",
        20,
        &[],
        &[("increased_attack_speed", "100")],
        1.0,
    );
    assert_eq!(plain.avg_hit_dps_max, fast.avg_hit_dps_max);
}

#[test]
fn unholy_form_stats_and_damage_follow_rank_and_activation() {
    let run = |form_rank: u32, enabled: bool, main: &str| {
        let allocated = HashMap::new();
        let inventory = HashMap::new();
        let skill_ranks =
            HashMap::from([(main.to_string(), 20), ("unholy_form".into(), form_rank)]);
        let subskills = HashMap::new();
        let buffs = HashMap::from([("unholy_form".into(), enabled)]);
        let custom = vec![CustomStat {
            stat_key: "life_steal".into(),
            value: "10".into(),
        }];
        let tree = HashSet::new();
        let sockets = HashMap::new();
        let enemy = HashMap::new();
        let player = HashMap::new();
        let projectiles = HashMap::new();
        let resistances = HashMap::new();
        let procs = HashMap::from([("granted:the_eye".into(), true)]);
        let granted_ranks = HashMap::from([("the eye".into(), (10.0, 20.0))]);
        let mut deps = empty_deps(
            &allocated,
            &inventory,
            &skill_ranks,
            &subskills,
            &buffs,
            &custom,
            &tree,
            &sockets,
            &enemy,
            &player,
            &projectiles,
            &resistances,
            &procs,
        );
        deps.class_id = Some("butcher");
        deps.level = 50;
        deps.main_skill_id = Some(main);
        deps.granted_skill_ranks = Some(&granted_ranks);
        compute_build_performance(&deps)
    };
    for skill in ["blender", "brutalizing_slash"] {
        let off = run(1, false, skill);
        let on = run(1, true, skill);
        let high = run(20, true, skill);
        let unlearned = run(0, true, skill);
        assert_eq!(on.stats.get("damage"), Some(&(20.0, 20.0)));
        assert_eq!(high.stats.get("damage"), Some(&(48.5, 48.5)));
        for key in ["life_replenish", "life_steal"] {
            let factor = on.stats.get(key).unwrap().0 / off.stats.get(key).unwrap().0;
            assert!((factor - 0.05).abs() < 1e-9, "{key}: {factor}");
            assert_eq!(on.stats.get(&format!("{key}_more")), Some(&(-95.0, -95.0)));
            assert_eq!(on.stats.get(key), high.stats.get(key));
        }
        let ratio = on.avg_hit_dps_max.unwrap() / off.avg_hit_dps_max.unwrap();
        assert!((ratio - 1.2).abs() < 0.01, "{skill}: {ratio}");
        assert!(high.avg_hit_dps_max > on.avg_hit_dps_max);
        assert_eq!(off.avg_hit_dps_max, unlearned.avg_hit_dps_max);
        assert!((on.proc_dps_min / off.proc_dps_min - 1.2).abs() < 1e-9);
        assert!((on.proc_dps_max / off.proc_dps_max - 1.2).abs() < 1e-9);
        assert_eq!(off.proc_dps_max, unlearned.proc_dps_max);
    }
}

#[test]
fn all_attack_skills_scale_physical_hits_and_projectile_ranks() {
    let mut checked = 0;
    for (class, skills) in &data::data().skills_by_class {
        for skill in skills.iter().filter(|s| {
            matches!(s.kind, crate::calc::types::SkillKind::Active)
                && matches!(
                    s.attack_kind,
                    Some(crate::calc::types::AttackKindSpec::Attack)
                )
        }) {
            let base = perf(class, &skill.id, 20, &[], &[]);
            let boosted = perf_with_stats(
                class,
                &skill.id,
                20,
                &[],
                &[("physical_skill_damage", "100")],
                1.0,
            );
            let original = base.attack_damage.as_ref().expect(&skill.id);
            let changed = boosted.attack_damage.as_ref().expect(&skill.id);
            // Integer rounding may leave one point below twice the rounded hit.
            assert!(
                (changed.physical_hit_max - 2 * original.physical_hit_max).abs() <= 1,
                "{class}/{}",
                skill.id
            );
            assert_eq!(
                changed.poison_hit_max, original.poison_hit_max,
                "Physical bonus must not raise elemental damage: {class}/{}",
                skill.id
            );
            if skill
                .tags
                .as_ref()
                .is_some_and(|t| t.iter().any(|t| t == "Projectile"))
            {
                let ranked = perf_with_stats(
                    class,
                    &skill.id,
                    20,
                    &[],
                    &[("projectile_skills", "3")],
                    1.0,
                );
                assert_eq!(
                    ranked.attack_damage.unwrap().effective_rank_max,
                    original.effective_rank_max + 3.0,
                    "{class}/{}",
                    skill.id
                );
            }
            checked += 1;
        }
    }
    assert!(
        checked > 50,
        "cross-class attack coverage unexpectedly missing"
    );
}

#[test]
fn blood_demons_proc_resolves_its_own_damage_skill() {
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let ranks = HashMap::from([("blood_surge".into(), 20), ("blood_demons".into(), 10)]);
    let subskills = HashMap::new();
    let buffs = HashMap::new();
    let custom = Vec::new();
    let tree = HashSet::new();
    let sockets = HashMap::new();
    let enemy = HashMap::new();
    let player = HashMap::new();
    let projectiles = HashMap::new();
    let resistances = HashMap::new();
    let procs = HashMap::from([("blood_demons".into(), true)]);
    let mut deps = empty_deps(
        &allocated,
        &inventory,
        &ranks,
        &subskills,
        &buffs,
        &custom,
        &tree,
        &sockets,
        &enemy,
        &player,
        &projectiles,
        &resistances,
        &procs,
    );
    deps.class_id = Some("demonspawn");
    deps.level = 50;
    deps.main_skill_id = Some("blood_surge");
    let result = compute_build_performance(&deps);
    assert!(result.proc_dps_min > 0.0);
    assert!(result
        .calculation()
        .iter()
        .any(|s| s.label() == "Proc · Blood Demons"));
}

#[test]
fn noncritical_dps_counts_every_projectile_contact() {
    let result = perf(
        "butcher",
        "blender",
        20,
        &[("a_i_empowered_nanoblenders", 1)],
        &[],
    );
    let attack = result.attack_damage.as_ref().unwrap();
    let expected = attack.combined_hit_max as f64 * 6.0 * 2.5 / 8.0;
    assert!((result.hit_dps_max.unwrap() - expected).abs() < 1e-9);
}

#[test]
fn projectile_count_changes_ailment_application_rate_not_single_contact_damage() {
    let nodes = [("a_i_empowered_nanoblenders", 1)];
    let stats = [("chance_inflict_bleeding", "100")];
    let single = perf_with_stats("butcher", "blender", 20, &nodes, &stats, 1.0);
    let double = perf_with_stats(
        "butcher",
        "blender",
        20,
        &[nodes[0], ("extra_cleaver_addon", 1)],
        &stats,
        1.0,
    );
    // Both already guarantee application. More blades cannot double an
    // individual bleed merely by being included in a volley average.
    assert!((double.ailment_dps_max.unwrap() / single.ailment_dps_max.unwrap() - 1.0).abs() < 0.01);
}

#[test]
fn frost_sunder_ailment_magnitude_uses_one_icicle() {
    let result = perf_with_stats(
        "jotunn",
        "frost_sunder",
        20,
        &[],
        &[("chance_inflict_bleeding", "100")],
        1.0,
    );
    let attack = result.attack_damage.as_ref().unwrap();
    assert_eq!(attack.projectile_count, 4);
    let expected = attack.combined_avg_max as f64 / 4.0 * 0.2;
    assert!((result.ailment_dps_max.unwrap() - expected).abs() < 1e-9);
}

#[test]
fn hybrid_multicast_does_not_repeat_the_physical_component() {
    let result = perf_with_stats(
        "samurai",
        "blade_barrier",
        20,
        &[],
        &[("multicast_chance", "100")],
        1.0,
    );
    let attack = result.attack_damage.as_ref().unwrap();
    let damage = result.damage.as_ref().unwrap();
    assert_eq!(damage.multicast_multiplier, 2.0);
    let expected = (attack.physical_hit_max as f64 + attack.poison_hit_max as f64 * 2.0)
        * attack.projectile_count as f64
        * attack.attacks_per_second_max
        * result.hits_per_cast.unwrap_or((1.0, 1.0)).1;
    assert!((result.hit_dps_max.unwrap() - expected).abs() < 1e-9);
}

#[test]
fn fireball_critical_burn_is_independent_expected_double_damage() {
    let base = perf("pyromancer", "fireball", 20, &[], &[]);
    let doubled = perf("pyromancer", "fireball", 20, &[("critical_burn", 5)], &[]);
    let plain = base.damage.as_ref().unwrap();
    let changed = doubled.damage.as_ref().unwrap();
    assert_eq!(plain.hit_max, changed.hit_max);
    assert_eq!(plain.projectile_count, changed.projectile_count);
    assert!(
        (changed.avg_max as f64 - 1.3 * plain.avg_max as f64).abs() < 1.3,
        "separate rounding of base and boosted expectations"
    );
    assert_eq!(base.hits_per_cast, doubled.hits_per_cast);
    let step = changed
        .calculation()
        .iter()
        .find(|s| s.label() == "Double damage expectation")
        .unwrap();
    assert_eq!(step.value(), (1.3, 1.3));
}

#[path = "amazon_tests.rs"]
mod amazon;

#[test]
fn cull_the_weak_requires_bleeding_and_never_executes_bosses() {
    for (bleeding, boss, expected) in [
        (false, false, 1.0),
        (true, false, 1.0 / 0.9),
        (true, true, 1.0),
    ] {
        let p = perf_with_tree(
            "demonspawn",
            "spinal_tap",
            10,
            &[],
            &[("bleeding", bleeding), ("is_boss", boss)],
            &[2130],
        );
        assert!((p.execute_mult - expected).abs() < 1e-9);
    }
}

#[test]
fn mechanical_engineering_blocks_player_hits_but_preserves_sentries() {
    for (class, skill) in [("pyromancer", "fireball"), ("demonspawn", "spinal_tap")] {
        let base = perf(class, skill, 10, &[], &[]);
        let blocked = perf_with_tree(class, skill, 10, &[], &[], &[328]);
        assert!(base.combined_dps_max.unwrap() > 0.0, "{skill}");
        assert_eq!(blocked.combined_dps_max, Some(0.0), "{skill}");
        assert_eq!(blocked.avg_hit_dps_max, Some(0.0), "{skill}");
        assert_eq!(blocked.ailment_dps_max.unwrap_or(0.0), 0.0, "{skill}");
    }
    let base = perf("marksman", "gunner_drone", 10, &[], &[]);
    let sentry = perf_with_tree("marksman", "gunner_drone", 10, &[], &[], &[328]);
    let ratio = sentry.avg_hit_dps_max.unwrap() / base.avg_hit_dps_max.unwrap();
    assert!((ratio - 1.5).abs() < 0.02, "sentry ratio {ratio}");
}

#[test]
fn incarnation_conversion_reaches_attack_dps_but_never_flat_spell_damage() {
    let base = perf("viking", "zeal", 10, &[], &[]);
    let converted = perf_with_tree("viking", "zeal", 10, &[], &[], &[1097, 1091]);
    let physical = base.attack_damage.as_ref().unwrap();
    let mixed = converted.attack_damage.as_ref().unwrap();
    assert_eq!(mixed.physical_hit_max, physical.physical_hit_max);
    assert_eq!(mixed.converted_elements.len(), 2);
    assert!(mixed.combined_hit_max > physical.combined_hit_max);
    assert!(converted.hit_dps_max.unwrap() > base.hit_dps_max.unwrap());
    assert!(converted.avg_hit_dps_max.unwrap() > base.avg_hit_dps_max.unwrap());
    let spell = perf_with_tree("pyromancer", "fireball", 10, &[], &[], &[1097, 1091]);
    assert_eq!(
        spell.avg_hit_dps_max,
        perf("pyromancer", "fireball", 10, &[], &[]).avg_hit_dps_max
    );
    let hybrid_base = perf("pyromancer", "inferno_slash", 10, &[], &[]);
    let hybrid = perf_with_tree("pyromancer", "inferno_slash", 10, &[], &[], &[1097]);
    assert_eq!(
        hybrid.damage.as_ref().unwrap().hit_max,
        hybrid_base.damage.as_ref().unwrap().hit_max
    );
    assert_eq!(
        hybrid.attack_damage.as_ref().unwrap().poison_hit_max,
        hybrid_base.attack_damage.as_ref().unwrap().poison_hit_max
    );
    assert!(hybrid.avg_hit_dps_max.unwrap() > hybrid_base.avg_hit_dps_max.unwrap());
    let blocked = perf_with_tree("viking", "zeal", 10, &[], &[], &[1097, 328]);
    assert_eq!(blocked.avg_hit_dps_max, Some(0.0));
    assert!(blocked
        .attack_damage
        .unwrap()
        .converted_elements
        .iter()
        .all(|c| c.hit_max == 0 && c.avg_max == 0));
}

#[test]
fn incarnation_area_and_charge_bonuses_reach_real_skills() {
    let area_base = perf("pyromancer", "fire_nova", 10, &[], &[]);
    let area = perf_with_tree("pyromancer", "fire_nova", 10, &[], &[], &[1161]);
    assert!(area.avg_hit_dps_max.unwrap() > area_base.avg_hit_dps_max.unwrap());
    let charge_base = perf("pyromancer", "phoenix_flight", 10, &[], &[]);
    let charge = perf_with_tree("pyromancer", "phoenix_flight", 10, &[], &[], &[2046]);
    assert!(charge.avg_hit_dps_max.unwrap() > charge_base.avg_hit_dps_max.unwrap());
    let unrelated = perf_with_tree("pyromancer", "fireball", 10, &[], &[], &[2046]);
    assert_eq!(
        unrelated.avg_hit_dps_max,
        perf("pyromancer", "fireball", 10, &[], &[]).avg_hit_dps_max
    );
}

#[test]
fn crowd_control_immunity_damage_requires_an_immune_target() {
    for (class, skill) in [("pyromancer", "fireball"), ("demonspawn", "spinal_tap")] {
        let off = perf_with_tree(class, skill, 10, &[], &[], &[1044]);
        let on = perf_with_tree(class, skill, 10, &[], &[("cc_immune", true)], &[1044]);
        assert_eq!(
            off.avg_hit_dps_max,
            perf(class, skill, 10, &[], &[]).avg_hit_dps_max
        );
        let ratio = on.avg_hit_dps_max.unwrap() / off.avg_hit_dps_max.unwrap();
        assert!((ratio - 1.25).abs() < 0.01, "{skill}: {ratio}");
    }
}

#[test]
fn immunity_shatter_notes_raise_dot_against_nonimmune_targets() {
    let base = perf_with_tree("pyromancer", "fireball", 10, &[], &[], &[472]);
    for (nodes, multiplier) in [(&[472, 813][..], 1.25), (&[472, 813, 825][..], 1.5)] {
        let result = perf_with_tree("pyromancer", "fireball", 10, &[], &[], nodes);
        assert_eq!(result.avg_hit_dps_max, base.avg_hit_dps_max);
        assert!(
            (result.ailment_dps_max.unwrap() / base.ailment_dps_max.unwrap() - multiplier).abs()
                < 1e-9
        );
    }
}

#[test]
fn immunity_shatter_nodes_restore_dot_against_full_immunity() {
    let baseline = perf_with_tree("pyromancer", "fireball", 10, &[], &[], &[472]);
    let base_dot = baseline.ailment_dps_max.unwrap();
    assert!(base_dot > 0.0);
    for (nodes, restored) in [
        (&[472][..], 0.0),
        (&[472, 813][..], 0.5),
        (&[472, 825][..], 0.5),
        (&[472, 813, 825][..], 1.0),
    ] {
        let immune = perf_with_tree(
            "pyromancer",
            "fireball",
            10,
            &[],
            &[("dot_immune", true)],
            nodes,
        );
        assert_eq!(immune.avg_hit_dps_max, baseline.avg_hit_dps_max);
        assert!((immune.ailment_dps_max.unwrap_or(0.0) - base_dot * restored).abs() < 1e-9);
        assert!(
            (immune.combined_dps_max.unwrap()
                - immune.avg_hit_dps_max.unwrap()
                - base_dot * restored)
                .abs()
                < 1e-9
        );
        let disabled = perf_with_tree(
            "pyromancer",
            "fireball",
            10,
            &[],
            &[("dot_immune", false)],
            nodes,
        );
        let absent = perf_with_tree("pyromancer", "fireball", 10, &[], &[], nodes);
        assert_eq!(disabled.ailment_dps_max, absent.ailment_dps_max);
    }
}

#[test]
fn wargod_total_damage_penalty_reaches_spell_and_weapon_hits() {
    for (class, skill) in [("pyromancer", "fireball"), ("viking", "zeal")] {
        for nodes in [&[][..], &[638, 643][..]] {
            let baseline = perf_with_tree(class, skill, 20, &[], &[], nodes);
            let mut with_wargod = nodes.to_vec();
            with_wargod.push(737);
            let wargod = perf_with_tree(class, skill, 20, &[], &[], &with_wargod);
            let hit = |p: &BuildPerformance| {
                p.attack_damage
                    .as_ref()
                    .map(|a| a.combined_hit_max)
                    .or_else(|| p.damage.as_ref().map(|d| d.hit_max))
                    .unwrap() as f64
            };
            // Final integer rounding can differ by one point; the penalty
            // multiplies the whole hit even with existing increased damage.
            assert!(
                (hit(&wargod) - hit(&baseline) * 0.85).abs() <= 1.0,
                "{skill}"
            );
        }
    }
}

#[test]
fn raging_titan_damage_affects_spells_and_complete_weapon_hits() {
    for (class, skill) in [("pyromancer", "fireball"), ("viking", "zeal")] {
        let base = perf_with_tree(class, skill, 10, &[], &[], &[149, 150]);
        let boosted = perf_with_tree(class, skill, 10, &[], &[], &[149, 150, 155]);
        // Compare hits: node 155 independently grants Total Attack Speed.
        let hit = |p: &BuildPerformance| {
            p.attack_damage
                .as_ref()
                .map(|a| a.combined_hit_max)
                .or_else(|| p.damage.as_ref().map(|d| d.hit_max))
                .unwrap()
        };
        assert!(hit(&boosted) > hit(&base), "{skill}");
        let delta = boosted.stats.get("damage").copied().unwrap_or_default().0
            - base.stats.get("damage").copied().unwrap_or_default().0;
        assert_eq!(delta, 2.0);
        assert_eq!(
            boosted.stats.get("enhanced_damage"),
            base.stats.get("enhanced_damage")
        );
    }
}

#[test]
fn power_of_weakness_increases_spell_and_weapon_damage() {
    for (class, skill) in [("pyromancer", "fireball"), ("viking", "zeal")] {
        // Lost Resistances supplies -7 to each of the five elements.
        let base = perf_with_tree(class, skill, 20, &[], &[], &[638]);
        let boosted = perf_with_tree(class, skill, 20, &[], &[], &[638, 643]);
        assert!(
            boosted.avg_hit_dps_max.unwrap() > base.avg_hit_dps_max.unwrap(),
            "{skill}"
        );
        assert_eq!(
            boosted.stats.get("enhanced_damage"),
            base.stats.get("enhanced_damage")
        );
        let extra = boosted.stats.get("damage").copied().unwrap_or_default().0
            - base.stats.get("damage").copied().unwrap_or_default().0;
        assert!((extra - 35.0 * 0.04).abs() < 1e-9);
    }
}

#[test]
fn colossus_total_damage_reaches_spells_and_weapon_hits() {
    for (class, skill) in [("pyromancer", "fireball"), ("viking", "zeal")] {
        let base = perf(class, skill, 20, &[], &[]);
        let boosted = perf_with_tree(class, skill, 20, &[], &[], &[579, 580, 581, 582]);
        // Four 1% per-stack nodes, five stacks including the base capacity.
        assert_eq!(boosted.stats["damage"], (20.0, 20.0));
        let ratio = boosted.avg_hit_dps_max.unwrap() / base.avg_hit_dps_max.unwrap();
        assert!((ratio - 1.2).abs() < 0.02, "{skill}: {ratio}");
        assert_eq!(
            boosted.stats.get("enhanced_damage"),
            base.stats.get("enhanced_damage")
        );
    }
}
