use super::*;

/// In-combat stacks (Rage today). Count is a Config knob that defaults to the
/// build's own cap, so an unconfigured build reads as full uptime.
pub fn apply_stack_effects(
    stack_counts: &HashMap<String, u32>,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) {
    for def in data::game_config().stack_types.iter() {
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
        }
    }
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
        assert_eq!(stats.get("increased_attack_speed").copied(), Some((30.0, 30.0)));
        assert_eq!(stats.get("enhanced_damage").copied(), Some((6.0, 6.0)));
    }

    #[test]
    fn count_scales_every_per_stack_effect() {
        let stats = run(&[("rage", 2)]);
        assert_eq!(stats.get("increased_attack_speed").copied(), Some((10.0, 10.0)));
        assert_eq!(stats.get("enhanced_damage").copied(), Some((2.0, 2.0)));
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
        assert!(run(&[("rage", 0)]).get("increased_attack_speed").is_none());
    }

    #[test]
    fn no_max_means_no_stacks() {
        let mut attrs = SourceMap::new();
        let mut stats = sources(&[("damage_per_rage_stack", 1.0)]);
        apply_stack_effects(&HashMap::new(), &mut attrs, &mut stats);
        assert!(!stats.contains_key("enhanced_damage"));
    }
}
