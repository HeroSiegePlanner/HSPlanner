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

static NO_STACKS: once_cell::sync::Lazy<HashMap<String, u32>> =
    once_cell::sync::Lazy::new(HashMap::new);

static DEFAULT_RATES: once_cell::sync::Lazy<HashMap<String, f64>> =
    once_cell::sync::Lazy::new(|| entity_rates(1.0));

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
    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let mut skill_ranks = HashMap::new();
    skill_ranks.insert(skill_id.to_string(), rank);
    let subskill_ranks: HashMap<String, u32> = subskills
        .iter()
        .map(|(id, r)| (subskill_key(skill_id, id), *r))
        .collect();
    let active_buffs = HashMap::new();
    let custom_stats: Vec<CustomStat> = Vec::new();
    let alloc_tree = HashSet::new();
    let tree_socketed = HashMap::new();
    let enemy_conditions: HashMap<String, bool> =
        enemy.iter().map(|(k, v)| (k.to_string(), *v)).collect();
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
    compute_build_performance(&deps)
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
    assert!((f / b - 2.0).abs() < 1e-9, "sentry AS +100% should double dps");

    deps.custom_stats = &[];
    let doubled = entity_rates(2.0);
    deps.entity_rates = &doubled;
    let two_per_sec = compute_build_performance(&deps);
    let Some(t) = two_per_sec.avg_hit_dps_max else {
        panic!("expected dps");
    };
    assert!((t / b - 2.0).abs() < 1e-9, "2/s base rate should double dps");

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
    assert!((knob - p).abs() < 1e-9, "config knob must not move a pinned rate");
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
    let custom_stats: Vec<CustomStat> = custom
        .iter()
        .map(|(k, v)| CustomStat {
            stat_key: k.to_string(),
            value: v.to_string(),
        })
        .collect();
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
        spell.damage.as_ref().expect("fireball damage").multicast_chance_pct,
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

    let node = perf_with_stats("marksman", "gunner_drone", 10, &[("nanodrones", 1)], &[], 1.0);
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

    let node_1 = perf_with_stats("amazon", "death_from_above", 10, &[("ancient_device", 1)], &[], 1.0);
    let node_3 = perf_with_stats("amazon", "death_from_above", 10, &[("ancient_device", 1)], &[], 3.0);
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
    let with_exec = perf("demonspawn", "spinal_tap", 10, &[("blood_and_gore", 4)], &[]);
    let on_boss = perf(
        "demonspawn",
        "spinal_tap",
        10,
        &[("blood_and_gore", 4)],
        &[("is_boss", true)],
    );
    let (Some(normal), Some(boss)) = (with_exec.combined_dps_max, on_boss.combined_dps_max)
    else {
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
    let off = perf("stormweaver", "charged_bolts", 10, &[("weakening_charge", 5)], &[]);
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
    let a = plain.attack_damage.expect("attack breakdown").combined_avg_max;
    let b = conv.attack_damage.expect("attack breakdown").combined_avg_max;
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
    let pick = data::data().skills_by_class.iter().find_map(|(cid, skills)| {
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
        eprintln!("no active skill with damage formula/table; skipping");
        return;
    };

    let allocated = HashMap::new();
    let inventory = HashMap::new();
    let mut skill_ranks = HashMap::new();
    skill_ranks.insert(skill_id.clone(), 10_u32);
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
    deps.level = 50;
    deps.main_skill_id = Some(&skill_id);

    let perf = compute_build_performance(&deps);
    assert!(
        perf.damage.is_some(),
        "expected damage breakdown for active skill '{skill_id}'"
    );
    assert!(perf.active_skill_name.is_some());
}

#[test]
fn active_skill_without_rank_yields_no_damage() {
    let pick = data::data().skills_by_class.iter().find_map(|(cid, skills)| {
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
        (haste_dps / base_dps - 1.75).abs() < 1e-9,
        "75% skill haste should be 1.75x, got x{}",
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
        (tundra_dps / base_dps - 1.5).abs() < 1e-9,
        "Timeless Tundra's 50% skill haste should be 1.5x, got x{}",
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
        compute_build_performance(&deps).combined_dps_max.unwrap_or(0.0)
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
    let attack = p.attack_damage.as_ref().expect("frost sunder attack breakdown");
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
            &allocated, &inventory, &skill_ranks, &subskill_ranks, &active_buffs,
            &custom_stats, &alloc_tree, &tree_socketed, &enemy_conditions,
            &player_conditions, &skill_projectiles, &enemy_resistances, &proc_toggles,
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
