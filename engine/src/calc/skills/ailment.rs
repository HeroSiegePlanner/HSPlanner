use std::collections::HashMap;

use super::{StatMap, r_max, rg};

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
    if hit_avg <= 0.0 {
        return 0.0;
    }
    let hits = hits_per_second.max(1.0);
    let all_damage = total_pct(stats, scoped, "ailment_damage_all");
    let all_frequency = total_pct(stats, scoped, "increased_ailment_frequency");

    AILMENTS
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
            hit_avg
                * fraction
                * (1.0 + damage_pct / 100.0)
                * (1.0 + frequency_pct / 100.0)
                * uptime
        })
        .sum()
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
    fn burning_dps_is_the_configured_fraction_of_the_hit() {
        let empty = StatMap::new();
        // burning fraction 0.2, 100% apply chance, no damage bonuses.
        let dps = ailment_dps(1000.0, 1.0, &empty, &empty, &chances(&[("burning", 100.0)]));
        assert!((dps - 200.0).abs() < 1e-9, "expected 200, got {dps}");
    }

    #[test]
    fn increased_ailment_damage_scales_the_contribution() {
        let empty = StatMap::new();
        let plain = ailment_dps(1000.0, 1.0, &empty, &empty, &chances(&[("burning", 100.0)]));
        let boosted = ailment_dps(1000.0, 1.0, &stats(&[("increased_burning_damage", 100.0)]),
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
        let shared = ailment_dps(1000.0, 1.0, &stats(&[("increased_rabies_damage", 50.0)]),
            &empty,
            &chances(&[("rabies", 100.0)]),
        );
        let scoped = ailment_dps(1000.0, 1.0, &empty,
            &stats(&[("increased_rabies_damage", 50.0)]),
            &chances(&[("rabies", 100.0)]),
        );
        assert!((shared - scoped).abs() < 1e-9);
    }

    #[test]
    fn frequency_and_apply_chance_scale_linearly() {
        let empty = StatMap::new();
        let base = ailment_dps(1000.0, 1.0, &empty, &empty, &chances(&[("bleeding", 100.0)]));
        let faster = ailment_dps(1000.0, 1.0, &stats(&[("increased_bleeding_frequency", 50.0)]),
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
        let dps = ailment_dps(1000.0, 1.0, &stats(&[("chance_inflict_poisoned", 100.0)]),
            &empty,
            &HashMap::new(),
        );
        assert!(dps > 0.0, "chance_inflict_poisoned must apply poisoned");
    }

    #[test]
    fn apply_chance_is_capped_at_one() {
        let empty = StatMap::new();
        let capped = ailment_dps(1000.0, 1.0, &stats(&[("chance_inflict_burning", 200.0)]),
            &empty,
            &chances(&[("burning", 200.0)]),
        );
        let exact = ailment_dps(1000.0, 1.0, &empty, &empty, &chances(&[("burning", 100.0)]));
        assert!((capped - exact).abs() < 1e-9);
    }

    #[test]
    fn ailments_sum_across_states() {
        let empty = StatMap::new();
        let both = ailment_dps(1000.0, 1.0, &empty,
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
        let sure = ailment_dps(1000.0, 42.5, &empty, &empty, &chances(&[("burning", 100.0)]));
        assert!((sure - 1000.0 * 0.2).abs() < 1e-9, "got {sure}");
    }
}
