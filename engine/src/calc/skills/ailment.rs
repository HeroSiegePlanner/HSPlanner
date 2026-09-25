use super::calculation::{number, scalar, CalculationStep};
use std::collections::HashMap;

use super::{r_max, rg, StatMap};

pub struct Ailment {
    pub state: &'static str,
    pub damage_key: &'static str,
    pub frequency_key: Option<&'static str>,
    pub chance_key: Option<&'static str>,
}

/// Ailments the engine models as damage over time; `state` matches the
/// `proc.appliesStates` spelling used by subskill trees and game-config.
pub const AILMENTS: &[Ailment] = &[
    Ailment {
        state: "burning",
        damage_key: "increased_burning_damage",
        frequency_key: None,
        chance_key: Some("chance_inflict_burning"),
    },
    Ailment {
        state: "bleeding",
        damage_key: "increased_bleeding_damage",
        frequency_key: Some("increased_bleeding_frequency"),
        chance_key: Some("chance_inflict_bleeding"),
    },
    Ailment {
        state: "poisoned",
        damage_key: "increased_poisoned_damage",
        frequency_key: Some("increased_poisoned_frequency"),
        chance_key: Some("chance_inflict_poisoned"),
    },
    Ailment {
        state: "stasis",
        damage_key: "increased_stasis_damage",
        frequency_key: None,
        chance_key: Some("chance_inflict_stasis"),
    },
    Ailment {
        state: "frostbite",
        damage_key: "increased_frost_bite_damage",
        frequency_key: None,
        chance_key: None,
    },
    Ailment {
        state: "shadow_burn",
        damage_key: "increased_shadow_burning_damage",
        frequency_key: None,
        chance_key: None,
    },
    Ailment {
        state: "permafrost",
        damage_key: "increased_permafrost_damage",
        frequency_key: None,
        chance_key: None,
    },
    Ailment {
        state: "rabies",
        damage_key: "increased_rabies_damage",
        frequency_key: None,
        chance_key: None,
    },
];

/// Per-second share of the triggering hit that a freshly applied ailment deals.
/// Calibrated in `game-config.json` -> `ailmentBaseFraction`.
fn base_fraction(state: &str) -> f64 {
    crate::calc::data::game_config()
        .ailment_base_fraction
        .as_ref()
        .and_then(|m| m.get(state))
        .copied()
        .unwrap_or(0.0)
}

fn total_pct(stats: &StatMap, scoped: &StatMap, key: &str) -> f64 {
    r_max(rg(stats, key)) + r_max(rg(scoped, key))
}

/// DPS from every ailment this build can inflict on the target it already hits.
/// `apply_chances` holds subtree proc chances; `chance_inflict_*` stats add to them.
/// `hits_per_second` (cast/attack rate times the entity count) turns a per-hit
/// chance into uptime: 17 drones firing twice a second keep a 4% burn up far
/// more than 4% of the time.
pub fn ailment_dps(
    hit_avg: f64,
    hits_per_second: f64,
    stats: &StatMap,
    scoped: &StatMap,
    apply_chances: &HashMap<String, f64>,
) -> f64 {
    ailment_calculation(hit_avg, hits_per_second, stats, scoped, apply_chances, false).0
}

/// Same-skill sources share application chances and replace the active ailment.
/// This preserves the existing one-second uptime estimate. Contact-rate weighting
/// estimates the latest source; it does not simulate the echo's arrival timeline.
pub(crate) fn replacing_ailment_calculation(
    sources: &[(f64, f64)],
    stats: &StatMap,
    scoped: &StatMap,
    apply_chances: &HashMap<String, f64>,
    target_dot_immune: bool,
) -> (f64, Vec<CalculationStep>) {
    let (weighted_hit, contacts) = sources
        .iter()
        .filter(|(hit, rate)| *hit > 0.0 && *rate > 0.0)
        .fold((0.0, 0.0), |(damage, rate), (hit, next_rate)| {
            (damage + hit * next_rate, rate + next_rate)
        });
    let hit = if contacts > 0.0 {
        weighted_hit / contacts
    } else {
        0.0
    };
    let (dps, mut steps) = ailment_calculation(
        hit,
        contacts,
        stats,
        scoped,
        apply_chances,
        target_dot_immune,
    );
    if contacts > 0.0 {
        steps.insert(0, CalculationStep::new(
            "Replacing ailment source estimate",
            format!("Latest application replaces the previous one; shared uptime uses {} total contacts/s. Source damage is weighted by contact rate ({} / {}); exact arrival order is not simulated", number(contacts), number(weighted_hit), number(contacts)),
            scalar(hit),
        ));
    }
    (dps, steps)
}

pub(crate) fn ailment_calculation(
    hit_avg: f64,
    hits_per_second: f64,
    stats: &StatMap,
    scoped: &StatMap,
    apply_chances: &HashMap<String, f64>,
    target_dot_immune: bool,
) -> (f64, Vec<CalculationStep>) {
    let mut trace = Vec::new();
    if hit_avg <= 0.0 || hits_per_second <= 0.0 {
        return (0.0, trace);
    }
    let hits = hits_per_second;
    let all_damage = total_pct(stats, scoped, "ailment_damage_all");
    let all_frequency = total_pct(stats, scoped, "increased_ailment_frequency");
    let skill_damage_added = total_pct(stats, scoped, "skill_damage_to_ailments") / 100.0;

    let total: f64 = AILMENTS
        .iter()
        .map(|a| {
            let fraction = base_fraction(a.state);
            if fraction <= 0.0 {
                return 0.0;
            }
            let from_stat = a
                .chance_key
                .map(|k| total_pct(stats, scoped, k))
                .unwrap_or(0.0);
            let from_procs = apply_chances.get(a.state).copied().unwrap_or(0.0);
            let chance = (from_stat + from_procs).clamp(0.0, 100.0) / 100.0;
            if chance <= 0.0 {
                return 0.0;
            }
            let uptime = 1.0 - (1.0 - chance).powf(hits);
            let damage_pct = total_pct(stats, scoped, a.damage_key) + all_damage;
            let frequency_pct = a
                .frequency_key
                .map(|k| total_pct(stats, scoped, k))
                .unwrap_or(0.0)
                + all_frequency;
            let contribution = hit_avg * (fraction + skill_damage_added) * (1.0 + damage_pct / 100.0) * (1.0 + frequency_pct / 100.0) * uptime;
            trace.push(CalculationStep::new(format!("{} uptime", a.state), format!("1 − (1 − clamp({}% stat + {}% subtree, 0, 100) / 100)^({} hits/s)", number(from_stat), number(from_procs), number(hits_per_second)), scalar(uptime)));
            trace.push(CalculationStep::new(format!("{} DPS", a.state), format!("{} average hit × ({} base fraction + {} skill damage added) × (1 + {}% damage / 100) × (1 + {}% frequency / 100) × {} uptime", number(hit_avg), number(fraction), number(skill_damage_added), number(damage_pct), number(frequency_pct), number(uptime)), scalar(contribution)));
            contribution
        })
        .sum();
    // ProjectileCollision00Skills subtracts shatter from full DoT immunity
    // before multiplying by (1 - immunity). See dot-shatter-evidence.md.
    // The note's half-value bonus belongs only to non-immune targets.
    let shatter = total_pct(stats, scoped, "monster_dot_immunity_shattered").max(0.0);
    let target_multiplier = if target_dot_immune {
        shatter / 100.0
    } else {
        1.0 + shatter / 200.0
    };
    if target_dot_immune {
        trace.push(CalculationStep::new(
            "DoT against an immune target",
            format!(
                "{} ailment DPS × max(0, 1 − (100% immunity − {}% immunity shatter) / 100)",
                number(total),
                number(shatter)
            ),
            scalar(total * target_multiplier),
        ));
    } else if shatter > 0.0 {
        trace.push(CalculationStep::new(
            "DoT against a non-immune target",
            format!(
                "{} ailment DPS × (1 + {}% immunity shatter / 2 / 100)",
                number(total),
                number(shatter)
            ),
            scalar(total * target_multiplier),
        ));
    }
    (total * target_multiplier, trace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(pairs: &[(&str, f64)]) -> StatMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), (*v, *v)))
            .collect()
    }

    fn chances(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn replacing_sources_share_uptime_and_never_sum_two_active_ailments() {
        let empty = StatMap::new();
        for ailment in AILMENTS {
            for chance in [25.0, 100.0] {
                let apply = chances(&[(ailment.state, chance)]);
                let (actual, _) = replacing_ailment_calculation(
                    &[(100.0, 6.0), (240.0, 1.0)],
                    &empty,
                    &empty,
                    &apply,
                    false,
                );
                let expected = ailment_dps(120.0, 7.0, &empty, &empty, &apply);
                assert!((actual - expected).abs() < 1e-10, "{}", ailment.state);
                let (same, _) = replacing_ailment_calculation(
                    &[(100.0, 6.0), (100.0, 1.0)],
                    &empty,
                    &empty,
                    &apply,
                    false,
                );
                assert_eq!(same, ailment_dps(100.0, 7.0, &empty, &empty, &apply));
                let (disabled, _) = replacing_ailment_calculation(
                    &[(100.0, 6.0), (240.0, 0.0)],
                    &empty,
                    &empty,
                    &apply,
                    false,
                );
                assert_eq!(disabled, ailment_dps(100.0, 6.0, &empty, &empty, &apply));
                let shatter = stats(&[("monster_dot_immunity_shattered", 50.0)]);
                let (immune, _) = replacing_ailment_calculation(
                    &[(100.0, 6.0), (240.0, 1.0)],
                    &shatter,
                    &empty,
                    &apply,
                    true,
                );
                assert!((immune - expected * 0.5).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn zero_contacts_cannot_inflict_damage_and_sparse_hits_are_not_promoted() {
        let empty = StatMap::new();
        assert_eq!(
            ailment_dps(
                1000.0,
                0.0,
                &empty,
                &empty,
                &chances(&[("bleeding", 100.0)])
            ),
            0.0
        );
        assert_eq!(
            ailment_dps(
                1000.0,
                -1.0,
                &empty,
                &empty,
                &chances(&[("bleeding", 100.0)])
            ),
            0.0
        );
        let chances = chances(&[("bleeding", 25.0)]);
        let sparse = ailment_dps(1000.0, 0.25, &empty, &empty, &chances);
        let once = ailment_dps(1000.0, 1.0, &empty, &empty, &chances);
        assert!(sparse > 0.0 && sparse < once);
    }

    #[test]
    fn every_ailment_has_a_configured_base_fraction() {
        for a in AILMENTS {
            assert!(
                base_fraction(a.state) > 0.0,
                "game-config ailmentBaseFraction is missing {}",
                a.state
            );
        }
    }

    #[test]
    fn no_applier_means_no_ailment_dps() {
        let s = stats(&[("increased_burning_damage", 500.0)]);
        let empty = StatMap::new();
        assert_eq!(ailment_dps(1000.0, 1.0, &s, &empty, &HashMap::new()), 0.0);
    }

    #[test]
    fn full_dot_immunity_uses_shatter_without_the_nonimmune_bonus() {
        let empty = StatMap::new();
        for ailment in AILMENTS {
            let apply = chances(&[(ailment.state, 100.0)]);
            let damage = stats(&[(ailment.damage_key, 100.0)]);
            let baseline = ailment_dps(1000.0, 1.0, &damage, &empty, &apply);
            assert!(baseline > 0.0, "{}", ailment.state);
            for (shatter, expected) in [
                (0.0, 0.0),
                (25.0, 0.25),
                (50.0, 0.5),
                (100.0, 1.0),
                (150.0, 1.5),
            ] {
                let scoped = stats(&[("monster_dot_immunity_shattered", shatter)]);
                let (dps, trace) = ailment_calculation(1000.0, 1.0, &damage, &scoped, &apply, true);
                assert!(
                    (dps - baseline * expected).abs() < 1e-9,
                    "{} / {shatter}: {dps}",
                    ailment.state
                );
                assert!(trace
                    .iter()
                    .any(|step| step.label() == "DoT against an immune target"));
            }
        }
    }

    #[test]
    fn burning_dps_is_the_configured_fraction_of_the_hit() {
        let empty = StatMap::new();
        // burning fraction 0.2, 100% apply chance, no damage bonuses.
        let dps = ailment_dps(1000.0, 1.0, &empty, &empty, &chances(&[("burning", 100.0)]));
        assert!((dps - 200.0).abs() < 1e-9, "expected 200, got {dps}");
    }

    #[test]
    fn skill_damage_added_to_ailments_stacks_with_increased_ailment_damage() {
        let s = stats(&[
            ("skill_damage_to_ailments", 140.0),
            ("ailment_damage_all", 25.0),
        ]);
        let dps = ailment_dps(
            1000.0,
            1.0,
            &s,
            &StatMap::new(),
            &chances(&[("burning", 100.0)]),
        );
        assert!((dps - 2000.0).abs() < 1e-9, "expected 2000, got {dps}");
    }

    #[test]
    fn increased_ailment_damage_scales_the_contribution() {
        let empty = StatMap::new();
        let plain = ailment_dps(1000.0, 1.0, &empty, &empty, &chances(&[("burning", 100.0)]));
        let boosted = ailment_dps(
            1000.0,
            1.0,
            &stats(&[("increased_burning_damage", 100.0)]),
            &empty,
            &chances(&[("burning", 100.0)]),
        );
        assert!(
            (boosted - plain * 2.0).abs() < 1e-9,
            "+100% burning damage must double the burning DPS: {plain} -> {boosted}"
        );
    }

    #[test]
    fn scoped_subtree_value_counts_like_a_shared_one() {
        let empty = StatMap::new();
        let shared = ailment_dps(
            1000.0,
            1.0,
            &stats(&[("increased_rabies_damage", 50.0)]),
            &empty,
            &chances(&[("rabies", 100.0)]),
        );
        let scoped = ailment_dps(
            1000.0,
            1.0,
            &empty,
            &stats(&[("increased_rabies_damage", 50.0)]),
            &chances(&[("rabies", 100.0)]),
        );
        assert!((shared - scoped).abs() < 1e-9);
    }

    #[test]
    fn frequency_and_apply_chance_scale_linearly() {
        let empty = StatMap::new();
        let base = ailment_dps(
            1000.0,
            1.0,
            &empty,
            &empty,
            &chances(&[("bleeding", 100.0)]),
        );
        let faster = ailment_dps(
            1000.0,
            1.0,
            &stats(&[("increased_bleeding_frequency", 50.0)]),
            &empty,
            &chances(&[("bleeding", 100.0)]),
        );
        assert!((faster - base * 1.5).abs() < 1e-9);
        let halved = ailment_dps(1000.0, 1.0, &empty, &empty, &chances(&[("bleeding", 50.0)]));
        assert!((halved - base * 0.5).abs() < 1e-9);
    }

    #[test]
    fn chance_inflict_stat_applies_the_ailment_on_its_own() {
        let empty = StatMap::new();
        let dps = ailment_dps(
            1000.0,
            1.0,
            &stats(&[("chance_inflict_poisoned", 100.0)]),
            &empty,
            &HashMap::new(),
        );
        assert!(dps > 0.0, "chance_inflict_poisoned must apply poisoned");
    }

    #[test]
    fn apply_chance_is_capped_at_one() {
        let empty = StatMap::new();
        let capped = ailment_dps(
            1000.0,
            1.0,
            &stats(&[("chance_inflict_burning", 200.0)]),
            &empty,
            &chances(&[("burning", 200.0)]),
        );
        let exact = ailment_dps(1000.0, 1.0, &empty, &empty, &chances(&[("burning", 100.0)]));
        assert!((capped - exact).abs() < 1e-9);
    }

    #[test]
    fn ailments_sum_across_states() {
        let empty = StatMap::new();
        let both = ailment_dps(
            1000.0,
            1.0,
            &empty,
            &empty,
            &chances(&[("burning", 100.0), ("bleeding", 100.0)]),
        );
        let one = ailment_dps(1000.0, 1.0, &empty, &empty, &chances(&[("burning", 100.0)]));
        assert!((both - one * 2.0).abs() < 1e-9);
    }

    #[test]
    fn a_per_hit_chance_becomes_uptime_over_many_hits() {
        let empty = StatMap::new();
        let c = chances(&[("burning", 4.0)]);
        // One hit a second is the old behaviour: 4% chance, 4% of the burn.
        let slow = ailment_dps(1000.0, 1.0, &empty, &empty, &c);
        assert!((slow - 1000.0 * 0.2 * 0.04).abs() < 1e-9, "got {slow}");

        // 17 drones firing 2.5x a second land ~42 hits: the target burns most
        // of the time, so the same 4% node is worth far more.
        let swarm = ailment_dps(1000.0, 42.5, &empty, &empty, &c);
        let uptime = swarm / (1000.0 * 0.2);
        assert!(uptime > 0.8 && uptime < 0.9, "uptime {uptime}");

        // A guaranteed apply cannot exceed full uptime.
        let sure = ailment_dps(
            1000.0,
            42.5,
            &empty,
            &empty,
            &chances(&[("burning", 100.0)]),
        );
        assert!((sure - 1000.0 * 0.2).abs() < 1e-9, "got {sure}");
    }
}
