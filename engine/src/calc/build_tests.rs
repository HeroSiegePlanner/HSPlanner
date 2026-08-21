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
        granted_skill_ranks: None,
    }
}

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
