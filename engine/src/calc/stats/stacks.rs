use super::*;
use crate::calc::skills::calculation::range;

/// In-combat stacks (Rage today). Count is a Config knob that defaults to the
/// build's own cap, so an unconfigured build reads as full uptime.
pub fn apply_stack_effects(
    stack_counts: &HashMap<String, u32>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) {
    for def in data::game_config().stack_types.iter() {
        // Buff 279 has a fixed 5% payload; its chance enables the effect but
        // does not scale the payload. Extra capacity cannot enable it alone.
        if def.key == "ramping_pulse"
            && stat_sources
                .get("ramping_pulse_chance")
                .map(|s| sum_contributions(s).1)
                .unwrap_or(0.0)
                <= 0.0
        {
            continue;
        }
        let activation = match def.key.as_str() {
            "colossus" => Some("colossus_damage"),
            "combat_mitigation" => Some("damage_mitigation_when_struck"),
            _ => None,
        };
        if let Some(rate) = activation {
            // PlayerTakeDamage creates buffs 253/252 only for positive rates
            // and passes 1 + their extra capacity as the cap. See audit evidence.
            if stat_sources
                .get(rate)
                .map(|s| sum_contributions(s).1)
                .unwrap_or(0.0)
                <= 0.0
            {
                continue;
            }
            apply_contribution(
                attr_sources,
                stat_sources,
                &def.max_stat,
                (1.0, 1.0),
                format!("{} base stack", def.name),
                SourceType::Tree,
                None,
            );
        }
        let max = sum_ranged_from_map(stat_sources, &def.max_stat).1;
        if max <= 0.0 {
            continue;
        }
        let count = stack_counts
            .get(&def.key)
            .map(|c| f64::from(*c))
            .unwrap_or(max)
            .clamp(0.0, max);
        if count <= 0.0 {
            continue;
        }
        let label = format!("{} ({count} stacks)", def.name);
        for (target, per_stack) in def.per_stack.iter() {
            let value = per_stack * count;
            apply_contribution(
                attr_sources,
                stat_sources,
                target,
                (value, value),
                label.clone(),
                SourceType::Tree,
                None,
            );
        }
        for (rate_key, target) in def.per_stack_stats.iter() {
            let rate = stat_sources
                .get(rate_key)
                .map(|list| sum_contributions(list))
                .unwrap_or((0.0, 0.0));
            apply_contribution(
                attr_sources,
                stat_sources,
                target,
                (rate.0 * count, rate.1 * count),
                label.clone(),
                SourceType::Tree,
                None,
            );
            if rate_key == "damage_dealt_and_taken_amp" {
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    "damage_taken_increased",
                    (rate.0 * count, rate.1 * count),
                    label.clone(),
                    SourceType::Tree,
                    None,
                );
            }
        }
    }
}

pub const CRUSHING_BLOW_KEY: &str = "crushing_blow";
const CRUSHING_BLOW_BASE_STACKS: f64 = 5.0;
const CRUSHING_BLOW_BASE_BREAK: f64 = 10.0;
const CRUSHING_BLOW_DURATION_SECS: f64 = 3.0;

thread_local! {
    static CRUSHING_BLOW_CONTACTS: std::cell::RefCell<Option<Ranged>> =
        const { std::cell::RefCell::new(None) };
}

/// Main-skill contacts per second for the stats rerun that estimates Crushing Blow stacks.
pub struct CrushingBlowScope {
    _not_send: std::marker::PhantomData<*const ()>,
}

impl CrushingBlowScope {
    #[must_use]
    pub fn enter(contacts_per_sec: Ranged) -> Self {
        CRUSHING_BLOW_CONTACTS.with(|c| *c.borrow_mut() = Some(contacts_per_sec));
        Self {
            _not_send: std::marker::PhantomData,
        }
    }

    pub fn active() -> bool {
        CRUSHING_BLOW_CONTACTS.with(|c| c.borrow().is_some())
    }
}

impl Drop for CrushingBlowScope {
    fn drop(&mut self) {
        CRUSHING_BLOW_CONTACTS.with(|c| *c.borrow_mut() = None);
    }
}

/// irandom(99) < chance: an integer roll, so a fractional chance rounds up.
pub fn crushing_blow_proc_chance(chance_pct: f64) -> f64 {
    chance_pct.ceil().clamp(0.0, 100.0) / 100.0
}

/// One stack breaks floor(10 × (1 + Armor Break% / 100)) of physical damage.
pub fn crushing_blow_stack_value(effectiveness_pct: f64) -> f64 {
    (CRUSHING_BLOW_BASE_BREAK * (1.0 + effectiveness_pct / 100.0))
        .floor()
        .max(0.0)
}

/// Stacks refresh 3 s per proc, so P(k or more up) = q^k with q = 1 − e^(−3 s × procs/s).
pub fn crushing_blow_expected_stacks(contacts_per_sec: f64, chance_pct: f64, cap: f64) -> f64 {
    let procs_per_sec = contacts_per_sec.max(0.0) * crushing_blow_proc_chance(chance_pct);
    let q = 1.0 - (-procs_per_sec * CRUSHING_BLOW_DURATION_SECS).exp();
    (1..=cap.max(0.0) as i32).map(|k| q.powi(k)).sum()
}

/// Crushing Blow (enemy buff 6): a Config count, else the stacks the rerun's contact rate keeps up.
pub fn apply_crushing_blow(
    stack_counts: &HashMap<String, u32>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) {
    let chance = sum_ranged_from_map(stat_sources, "crushing_blow_chance");
    if chance.1 <= 0.0 {
        return;
    }
    apply_contribution(
        attr_sources,
        stat_sources,
        "max_crushing_blow_stacks",
        (CRUSHING_BLOW_BASE_STACKS, CRUSHING_BLOW_BASE_STACKS),
        "Crushing Blow base stacks".to_string(),
        SourceType::Class,
        None,
    );
    let cap = sum_ranged_from_map(stat_sources, "max_crushing_blow_stacks");
    let round = |v: Ranged| ((v.0 * 100.0).round() / 100.0, (v.1 * 100.0).round() / 100.0);
    let (stacks, source_type, origin) = match stack_counts.get(CRUSHING_BLOW_KEY) {
        Some(count) => {
            let count = f64::from(*count);
            (
                (count.min(cap.0), count.min(cap.1)),
                SourceType::Custom,
                "Config".to_string(),
            )
        }
        None => {
            let Some(contacts) = CRUSHING_BLOW_CONTACTS.with(|c| *c.borrow()) else {
                return;
            };
            (
                (
                    crushing_blow_expected_stacks(contacts.0, chance.0, cap.0),
                    crushing_blow_expected_stacks(contacts.1, chance.1, cap.1),
                ),
                SourceType::Skill,
                format!("{} contacts/s, 3 s refresh", range(round(contacts))),
            )
        }
    };
    let effectiveness = sum_ranged_from_map(stat_sources, "crushing_blow_effectiveness");
    let value = (
        crushing_blow_stack_value(effectiveness.0),
        crushing_blow_stack_value(effectiveness.1),
    );
    apply_contribution(
        attr_sources,
        stat_sources,
        "crushing_blow_stacks",
        stacks,
        format!("Crushing Blow ({origin})"),
        source_type,
        None,
    );
    apply_contribution(
        attr_sources,
        stat_sources,
        "crushing_blow_armor_break",
        (stacks.0 * value.0, stacks.1 * value.1),
        format!(
            "Crushing Blow: {} stacks × {}% ({origin})",
            range(round(stacks)),
            range(value)
        ),
        source_type,
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources(pairs: &[(&str, f64)]) -> SourceMap {
        let mut map = SourceMap::new();
        for (key, value) in pairs {
            push_source(
                &mut map,
                key,
                SourceContribution {
                    label: "test".to_string(),
                    source_type: SourceType::Tree,
                    value: (*value, *value),
                    forge: None,
                },
            );
        }
        map
    }

    fn run(counts: &[(&str, u32)]) -> HashMap<String, Ranged> {
        let counts: HashMap<String, u32> =
            counts.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mut attrs = SourceMap::new();
        let mut stats = sources(&[("max_rage_stacks", 6.0), ("damage_per_rage_stack", 1.0)]);
        apply_stack_effects(&counts, &mut attrs, &mut stats);
        compute_final_stats(&stats)
    }

    #[test]
    fn unset_count_assumes_the_cap() {
        let stats = run(&[]);
        assert_eq!(
            stats.get("increased_attack_speed").copied(),
            Some((30.0, 30.0))
        );
        assert_eq!(stats.get("damage").copied(), Some((6.0, 6.0)));
    }

    #[test]
    fn count_scales_every_per_stack_effect() {
        let stats = run(&[("rage", 2)]);
        assert_eq!(
            stats.get("increased_attack_speed").copied(),
            Some((10.0, 10.0))
        );
        assert_eq!(stats.get("damage").copied(), Some((2.0, 2.0)));
    }

    #[test]
    fn count_above_the_cap_clamps_to_it() {
        assert_eq!(
            run(&[("rage", 99)]).get("increased_attack_speed").copied(),
            Some((30.0, 30.0))
        );
    }

    #[test]
    fn zero_count_contributes_nothing() {
        assert!(!run(&[("rage", 0)]).contains_key("increased_attack_speed"));
    }

    #[test]
    fn no_max_means_no_stacks() {
        let mut attrs = SourceMap::new();
        let mut stats = sources(&[("damage_per_rage_stack", 1.0)]);
        apply_stack_effects(&HashMap::new(), &mut attrs, &mut stats);
        assert!(!stats.contains_key("damage"));
    }

    fn crushing(
        counts: &[(&str, u32)],
        contacts: Option<Ranged>,
        pairs: &[(&str, f64)],
    ) -> HashMap<String, Ranged> {
        let counts: HashMap<String, u32> =
            counts.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mut attrs = SourceMap::new();
        let mut stats = sources(pairs);
        let _scope = contacts.map(CrushingBlowScope::enter);
        apply_crushing_blow(&counts, &mut attrs, &mut stats);
        compute_final_stats(&stats)
    }

    #[test]
    fn crushing_blow_chance_rounds_up_like_the_integer_roll() {
        assert_eq!(crushing_blow_proc_chance(10.5), 0.11);
        assert_eq!(crushing_blow_proc_chance(0.5), 0.01);
        assert_eq!(crushing_blow_proc_chance(10.0), 0.1);
        assert_eq!(crushing_blow_proc_chance(0.0), 0.0);
        assert_eq!(crushing_blow_proc_chance(150.0), 1.0);
    }

    #[test]
    fn crushing_blow_stack_value_floors_the_armor_break_bonus() {
        assert_eq!(crushing_blow_stack_value(0.0), 10.0);
        assert_eq!(crushing_blow_stack_value(25.0), 12.0);
        assert_eq!(crushing_blow_stack_value(50.0), 15.0);
    }

    #[test]
    fn crushing_blow_caps_at_five_plus_extra_stacks() {
        let base = crushing(
            &[("crushing_blow", 99)],
            None,
            &[("crushing_blow_chance", 10.0)],
        );
        assert_eq!(base["max_crushing_blow_stacks"], (5.0, 5.0));
        assert_eq!(base["crushing_blow_stacks"], (5.0, 5.0));
        assert_eq!(base["crushing_blow_armor_break"], (50.0, 50.0));
        let extra = crushing(
            &[("crushing_blow", 99)],
            None,
            &[
                ("crushing_blow_chance", 10.0),
                ("max_crushing_blow_stacks", 1.0),
            ],
        );
        assert_eq!(extra["max_crushing_blow_stacks"], (6.0, 6.0));
        assert_eq!(extra["crushing_blow_armor_break"], (60.0, 60.0));
    }

    #[test]
    fn config_count_overrides_the_estimate() {
        let stats = crushing(
            &[("crushing_blow", 2)],
            Some((100.0, 100.0)),
            &[("crushing_blow_chance", 25.0)],
        );
        assert_eq!(stats["crushing_blow_armor_break"], (20.0, 20.0));
    }

    #[test]
    fn expected_crushing_blow_stacks_follow_the_proc_rate_up_to_the_cap() {
        assert!((crushing_blow_expected_stacks(100.0, 25.0, 6.0) - 6.0).abs() < 1e-9);
        // A rare proc almost never meets the next one within 3 s: about 3 s × procs/s.
        assert!((crushing_blow_expected_stacks(1.0, 1.0, 5.0) - 0.03).abs() < 0.001);
        let q = 1.0 - (-3.0f64).exp();
        let one_proc_per_second: f64 = (1..=5).map(|k| q.powi(k)).sum();
        assert!(
            (crushing_blow_expected_stacks(4.0, 25.0, 5.0) - one_proc_per_second).abs() < 1e-12
        );
        assert_eq!(crushing_blow_expected_stacks(10.0, 0.0, 5.0), 0.0);
    }

    #[test]
    fn spec_example_keeps_ninety_percent_physical_break_up() {
        // 25% chance, +50% effectiveness, +1 stack, a proc far inside every 3 s.
        let stats = crushing(
            &[],
            Some((40.0, 40.0)),
            &[
                ("crushing_blow_chance", 25.0),
                ("crushing_blow_effectiveness", 50.0),
                ("max_crushing_blow_stacks", 1.0),
            ],
        );
        assert_eq!(stats["max_crushing_blow_stacks"], (6.0, 6.0));
        assert!((stats["crushing_blow_armor_break"].1 - 90.0).abs() < 1e-6);
    }

    #[test]
    fn crushing_blow_needs_a_chance_and_a_stack_source() {
        let no_chance = crushing(
            &[("crushing_blow", 5)],
            Some((10.0, 10.0)),
            &[("max_crushing_blow_stacks", 1.0)],
        );
        assert_eq!(no_chance["max_crushing_blow_stacks"], (1.0, 1.0));
        assert!(!no_chance.contains_key("crushing_blow_armor_break"));
        let no_rate = crushing(&[], None, &[("crushing_blow_chance", 10.0)]);
        assert_eq!(no_rate["max_crushing_blow_stacks"], (5.0, 5.0));
        assert!(!no_rate.contains_key("crushing_blow_armor_break"));
    }
}
