use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::calc::commands::BuildPerformanceInput;
use crate::calc::data;
use crate::calc::planner::{evaluate, PlannerInput, PlannerPerformance};
use crate::calc::stats::{ComputedStats, SourceMap};
use crate::calc::types::{
    augment_toggle_key, AugmentCalc, AugmentRef, CustomStat, EquippedAffix, EquippedItem,
    Inventory, SkillKind,
};

const WEAPON_ID: &str = "base_sword_scimitar";
const CARRIER_ID: &str = "amulets_normal_golden_amulet";
const PROBE_VALUE: f64 = 50.0;
// Weapon restrictions off, every ailment able to land and some resistance to convert,
// so the stats that read those have something to act on.
const SCENARIO_STATS: &[(&str, &str)] = &[
    ("skill_restrictions_removed", "100"),
    ("chance_inflict_burning", "50"),
    ("chance_inflict_bleeding", "50"),
    ("chance_inflict_poisoned", "50"),
    ("chance_inflict_stasis", "50"),
    ("cold_resistance", "50"),
    ("poison_resistance", "50"),
];
const ENEMY_CONDITIONS: &[&str] = &[
    "cc_immune",
    "shadow_burn",
    "serrated_chains",
    "lightning_break",
    "arcane_break",
    "poison_break",
    "fire_break",
    "cold_break",
    "deep_frozen",
    "frozenbite",
    "low_life",
    "bleeding",
    "poisoned",
    "stunned",
    "shocked",
    "burning",
    "frozen",
    "is_boss",
    "slow",
];
// Rolled keys the inventory resolves per item before they reach the calculation.
const ROLLED_KEYS: &[&str] = &["random_skill_element", "all_skills_class"];

// ponytail: item stats no formula reads. Drop an entry once the engine learns it;
// a key that starts to matter, or a new inert key, fails the test.
const NOT_IN_CALCULATION: &[(&str, &[&str])] = &[
    (
        "shown on the sheet only: utility, sustain and defence layers outside the EHP model",
        &[
            "attack_rating",
            "block_chance",
            "cannot_be_frozen",
            "damage_recouped_as_life",
            "damage_recouped_as_mana",
            "damage_reduced",
            "damage_return",
            "damage_taken_reduced",
            "decreased_merchant_prices",
            "defense",
            "defense_based_on_level",
            "defense_vs_missiles",
            "experience_gain",
            "extra_jump_count",
            "faster_hit_recovery",
            "gold_find",
            "increased_experience_gain_below_100",
            "jumping_power",
            "life_per_kill",
            "life_replenish",
            "life_replenish_pct",
            "life_steal",
            "light_radius",
            "magic_find",
            "mana_per_kill",
            "mana_steal",
            "movement_speed",
            "phasing",
            "poison_length_reduced",
            "potion_cooldown_recovery",
            "reduced_movement_diminish",
            "suppress_spell_hits",
            "arcane_absorption",
            "cold_absorption",
            "fire_absorption",
            "lightning_absorption",
            "magic_absorption",
            "poison_absorption",
        ],
    ),
    (
        "read only by the planner's hit-area and flight diagrams",
        &[
            "attack_radius",
            "explosion_aoe",
            "melee_range",
            "orbiting_relic_size",
            "projectile_size",
            "projectile_size_increase",
            "projectile_speed",
            "projectile_speed_pct",
            "ranged_range",
            "skill_relic_projectile_size",
            "spell_aoe_radius",
        ],
    ),
    (
        "read only by specific skills, auras or subtrees a rank-10 probe never enables",
        &[
            "buffing_aura_effectiveness",
            "grant_subskills",
            "increased_frost_bite_damage",
            "increased_shadow_burning_damage",
            "summon_life",
        ],
    ),
    (
        "not modelled",
        &[
            "additive_arcane_damage",
            "additive_cold_damage",
            "additive_fire_damage",
            "additive_lightning_damage",
            "additive_poison_damage",
            "ailment_crit_chance",
            "ailment_crit_damage",
            "attacks_hit_multiple_enemies",
            "buffing_aura_range",
            "defense_ignored",
            "dmg_taken_over_four_seconds",
            "effectiveness_of_non_buff_proc_relics",
            "follower_relic_attack_speed",
            "follower_relic_damage",
            "follower_relic_projectiles",
            "guardian_additional_attack",
            "guardian_attack_range",
            "guardian_duration",
            "increased_stasis_frequency",
            "less_dmg_with_cd_skills",
            "open_wounds",
            "orbital_projectile_duration_increased",
            "orbiting_relic_amount",
            "orbiting_relic_damage",
            "orbiting_relic_execute_below_health",
            "orbiting_relic_radius_increase",
            "phys_dmg_taken_as_arcane",
            "phys_dmg_taken_as_cold",
            "proc_non_buff_relic_chance",
            "projectile_random_direction",
            "projectile_return",
            "self_inflicted_chance_when_attacking",
            "self_inflicted_chance_when_casting",
            "self_inflicted_damage_from_attacking",
            "self_inflicted_damage_from_casting",
            "sentry_duration",
            "skill_relic_additional_casts",
            "skill_relic_cooldown",
            "skill_relic_damage",
            "skill_relic_projectiles",
            "slow_target",
            "summon_all_resist",
            "summon_movement_speed",
            "summon_reduced_dmg_taken",
        ],
    ),
];

// Every stat key an item-side source can put on a character, with where it came from.
// Augment level stats are left out: they are probed per augment, through its own switch.
fn item_stat_keys() -> BTreeMap<String, BTreeSet<String>> {
    let d = data::data();
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut add = |key: &str, origin: String| {
        out.entry(key.to_string()).or_default().insert(origin);
    };
    for item in d.items.values() {
        for key in item.implicit.iter().flat_map(|m| m.keys()) {
            add(key, format!("item {}", item.id));
        }
    }
    for (kind, mods) in [("affix", &d.affixes), ("crystal", &d.crystals)] {
        for m in mods.values() {
            if let Some(key) = &m.stat_key {
                add(key, format!("{kind} {}", m.id));
            }
        }
    }
    for rune in d.runes.values() {
        rune.stats
            .keys()
            .for_each(|k| add(k, format!("rune {}", rune.id)));
    }
    for gem in d.gems.values() {
        gem.stats
            .keys()
            .for_each(|k| add(k, format!("gem {}", gem.id)));
    }
    for runeword in &d.runewords {
        runeword
            .stats
            .keys()
            .for_each(|k| add(k, format!("runeword {}", runeword.id)));
    }
    for set in d.sets.values() {
        for key in set.bonuses.iter().flat_map(|b| b.stats.keys()) {
            add(key, format!("set {}", set.id));
        }
    }
    out
}

// Stat definitions the sheet and tooltips name keys by; `_more` variants borrow their base's.
fn is_defined(key: &str) -> bool {
    let config = data::game_config();
    let defined = |k: &str| {
        config.stats.iter().any(|s| s.key == k) || config.attributes.iter().any(|a| a.key == k)
    };
    defined(key)
        || key.strip_suffix("_more").is_some_and(defined)
        || key.strip_prefix("to_").is_some_and(defined)
        || ROLLED_KEYS.contains(&key)
}

struct Scenario {
    class_id: String,
    skill_id: String,
}

// One probe per active skill of every class, so element, tag and skill-type stats each
// meet a skill that reads them.
fn scenarios() -> Vec<Scenario> {
    let d = data::data();
    let mut classes: Vec<_> = d.skills_by_class.keys().cloned().collect();
    classes.sort();
    classes
        .into_iter()
        .flat_map(|class_id| {
            d.skills_by_class[&class_id]
                .iter()
                .filter(|s| matches!(s.kind, SkillKind::Active))
                .map(|s| Scenario {
                    class_id: class_id.clone(),
                    skill_id: s.id.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn equipped(base_id: &str) -> EquippedItem {
    EquippedItem {
        base_id: base_id.into(),
        ..Default::default()
    }
}

// The scenario's skill at rank 10 on a level-100 character holding a plain sword plus `item`.
fn build(scenario: &Scenario, item: EquippedItem) -> BuildPerformanceInput {
    let mut inventory: Inventory = HashMap::from([("weapon".to_string(), equipped(WEAPON_ID))]);
    inventory.insert(data::get_item(&item.base_id).unwrap().slot.clone(), item);
    BuildPerformanceInput {
        class_id: Some(scenario.class_id.clone()),
        level: 100,
        inventory,
        skill_ranks: HashMap::from([(scenario.skill_id.clone(), 10)]),
        main_skill_id: Some(scenario.skill_id.clone()),
        custom_stats: SCENARIO_STATS
            .iter()
            .map(|(key, value)| CustomStat {
                stat_key: key.to_string(),
                value: value.to_string(),
            })
            .collect(),
        enemy_conditions: ENEMY_CONDITIONS
            .iter()
            .map(|c| (c.to_string(), true))
            .collect(),
        kills_per_sec: 1.0,
        entity_rates: ["sentry", "summon", "guardian"]
            .iter()
            .map(|k| (k.to_string(), 1.0))
            .collect(),
        ..Default::default()
    }
}

fn run(build: BuildPerformanceInput) -> PlannerPerformance {
    evaluate(&PlannerInput {
        active_skill_ids: build.main_skill_id.iter().cloned().collect(),
        build,
        ..Default::default()
    })
}

// Everything the planner produced, minus the probed key's own pass-through entries
// and the per-source breakdowns (they always list the probe itself).
fn observable(perf: &PlannerPerformance, key: &str) -> serde_json::Value {
    fn strip(value: &mut serde_json::Value, key: &str) {
        match value {
            serde_json::Value::Object(map) => {
                for drop in [key, "statSources", "attributeSources", "calculationSources"] {
                    map.remove(drop);
                }
                map.values_mut().for_each(|v| strip(v, key));
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(|v| strip(v, key)),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(perf).unwrap();
    strip(&mut value, key);
    value
}

fn stat_reaches_calculation(key: &str, carrier: &str, scenarios: &[Scenario]) -> bool {
    scenarios.iter().any(|s| {
        let probe = match key {
            "random_skill_element" => "fire_skills".to_string(),
            "all_skills_class" => format!("all_skills_{}", s.class_id),
            _ => key.to_string(),
        };
        let with = |value: f64| {
            let mut item = equipped(carrier);
            item.implicit_overrides = HashMap::from([(probe.clone(), value)]);
            observable(&run(build(s, item)), &probe)
        };
        with(0.0) != with(PROBE_VALUE)
    })
}

fn augment_reaches_calculation(id: &str, scenarios: &[Scenario]) -> bool {
    scenarios.iter().any(|s| {
        let plain = run(build(s, equipped(CARRIER_ID)));
        let mut item = equipped(CARRIER_ID);
        item.augment = Some(AugmentRef {
            id: id.into(),
            level: u32::MAX,
        });
        let mut input = build(s, item);
        let toggle = HashMap::from([(augment_toggle_key(id), true)]);
        input.player_conditions = toggle.clone();
        input.proc_toggles = toggle;
        observable(&run(input), "") != observable(&plain, "")
    })
}

fn non_finite_numbers(perf: &PlannerPerformance) -> Vec<String> {
    let current = &perf.current;
    let maps = [
        ("stats", &perf.computed.stats),
        ("attributes", &perf.computed.attributes),
        ("current stats", &current.stats),
    ];
    let mut bad: Vec<String> = maps
        .iter()
        .flat_map(|(name, map)| {
            map.iter()
                .filter(|(_, v)| !v.0.is_finite() || !v.1.is_finite())
                .map(move |(k, v)| format!("{name}.{k} = {v:?}"))
        })
        .collect();
    let dps = [
        ("hit_dps_max", current.hit_dps_max),
        ("avg_hit_dps_max", current.avg_hit_dps_max),
        ("ailment_dps_max", current.ailment_dps_max),
        ("combined_dps_max", current.combined_dps_max),
        ("proc_dps_max", Some(current.proc_dps_max)),
    ];
    bad.extend(
        dps.iter()
            .filter(|(_, v)| v.is_some_and(|v| !v.is_finite() || v < 0.0))
            .map(|(k, v)| format!("{k} = {v:?}")),
    );
    bad
}

fn has_source(sources: &SourceMap, key: &str, label: &str) -> bool {
    sources
        .get(key)
        .is_some_and(|list| list.iter().any(|s| s.label == label))
}

// Attribute stats land on their attribute ("all" on every one); class skill bonuses
// fold into all_skills for that class.
fn credited(computed: &ComputedStats, key: &str, label: &str) -> bool {
    let attribute = data::game_config()
        .stats
        .iter()
        .find(|s| s.key == key)
        .and_then(|s| s.modifies_attribute.as_deref())
        .map(|a| if a == "all" { "strength" } else { a });
    has_source(&computed.stat_sources, key, label)
        || attribute.is_some_and(|a| has_source(&computed.attribute_sources, a, label))
        || key.starts_with("all_skills_") && has_source(&computed.stat_sources, "all_skills", label)
}

// Each item worn alone: the calculation stays finite and every implicit stat is credited to it.
fn item_failures(scenario: &Scenario) -> Vec<String> {
    let mut items: Vec<_> = data::data().items.values().collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    let mut failures = Vec::new();
    for item in items {
        let perf = run(build(scenario, equipped(&item.id)));
        for bad in non_finite_numbers(&perf) {
            failures.push(format!(
                "item {} with {}: {bad}",
                item.id, scenario.skill_id
            ));
        }
        let computed = &perf.computed;
        for key in item.implicit.iter().flat_map(|m| m.keys()) {
            if ROLLED_KEYS.contains(&key.as_str()) || key == "enhanced_defense" {
                continue;
            }
            if !credited(computed, key, &item.name) {
                failures.push(format!(
                    "item {}: implicit {key} never reaches the stats",
                    item.id
                ));
            }
        }
    }
    failures
}

fn forged(base_id: &str, crystal: &str) -> EquippedItem {
    let mut item = equipped(base_id);
    item.forged_mods = vec![EquippedAffix {
        affix_id: crystal.into(),
        roll: 1.0,
        ..Default::default()
    }];
    item
}

// Satanic crystals through the forge path: every crystal on one helmet, and a Life
// crystal on every item the gear editor offers a crystal slot for (#214).
fn crystal_failures(scenario: &Scenario) -> Vec<String> {
    let d = data::data();
    let mut failures = Vec::new();
    let mut crystals: Vec<_> = d.crystals.values().collect();
    crystals.sort_by(|a, b| a.id.cmp(&b.id));
    let helmet = d
        .items
        .values()
        .filter(|i| i.slot == "helmet" && data::forge_kind_for(&i.rarity).is_some())
        .min_by(|a, b| a.id.cmp(&b.id))
        .unwrap();
    for crystal in &crystals {
        let Some(key) = crystal.stat_key.as_deref() else {
            continue;
        };
        let perf = run(build(scenario, forged(&helmet.id, &crystal.id)));
        if !credited(&perf.computed, key, &crystal.name) {
            failures.push(format!("crystal {} never reaches the stats", crystal.id));
        }
    }
    let life = crystals
        .iter()
        .find(|c| c.stat_key.as_deref() == Some("life"))
        .unwrap();
    let mut items: Vec<_> = d
        .items
        .values()
        .filter(|i| data::accepts_crystal(&i.slot, &i.rarity))
        .collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    for item in items {
        let perf = run(build(scenario, forged(&item.id, &life.id)));
        if !credited(&perf.computed, "life", &life.name) {
            failures.push(format!("item {}: its satanic crystal is ignored", item.id));
        }
    }
    failures
}

#[test]
fn every_item_stat_is_defined_and_reaches_the_calculation() {
    let keys = item_stat_keys();
    let scenarios = scenarios();
    let mut failures = Vec::new();

    for (key, origins) in &keys {
        if !is_defined(key) {
            failures.push(format!(
                "{key} has no game-config definition (used by {origins:?})"
            ));
        }
    }

    let exempt: BTreeSet<&str> = NOT_IN_CALCULATION
        .iter()
        .flat_map(|(_, keys)| keys.iter().copied())
        .collect();
    for (key, origins) in &keys {
        let carrier = origins
            .iter()
            .find_map(|o| o.strip_prefix("item "))
            .unwrap_or(CARRIER_ID);
        let reaches = stat_reaches_calculation(key, carrier, &scenarios);
        match (reaches, exempt.contains(key.as_str())) {
            (false, false) => failures.push(format!(
                "{key} changes nothing in any calculation (used by {origins:?})"
            )),
            (true, true) => failures.push(format!(
                "{key} now reaches the calculation; remove it from NOT_IN_CALCULATION"
            )),
            _ => {}
        }
    }
    for key in exempt.iter().filter(|k| !keys.contains_key(**k)) {
        failures.push(format!(
            "{key} is no longer an item stat; remove it from NOT_IN_CALCULATION"
        ));
    }

    let mut augments: Vec<_> = data::data().augments.values().collect();
    augments.sort_by(|a, b| a.id.cmp(&b.id));
    for augment in augments.into_iter().filter(|a| a.calc != AugmentCalc::None) {
        if !augment_reaches_calculation(&augment.id, &scenarios) {
            failures.push(format!(
                "augment {} changes nothing at max level",
                augment.id
            ));
        }
    }

    // One attack and one spell, each with every item worn in turn.
    let fireball = scenarios.iter().find(|s| s.skill_id == "fireball").unwrap();
    for scenario in [&scenarios[0], fireball] {
        failures.extend(item_failures(scenario));
    }
    failures.extend(crystal_failures(&scenarios[0]));

    assert!(
        failures.is_empty(),
        "{} item stat problems:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
