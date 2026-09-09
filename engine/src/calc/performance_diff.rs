use std::collections::BTreeSet;

use super::{build::BuildPerformance, data, skills::Ranged};

#[derive(Clone, Debug)]
pub struct PerformanceDiff {
    key: String,
    label: String,
    before: Ranged,
    after: Ranged,
    delta: f64,
    is_percent: bool,
    delta_pct: Option<f64>,
}

impl PerformanceDiff {
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn before(&self) -> Ranged {
        self.before
    }
    pub fn after(&self) -> Ranged {
        self.after
    }
    pub fn delta(&self) -> f64 {
        self.delta
    }
    pub fn is_percent(&self) -> bool {
        self.is_percent
    }
    pub fn delta_pct(&self) -> Option<f64> {
        self.delta_pct
    }

    fn between(
        key: String,
        label: String,
        before: Ranged,
        after: Ranged,
        is_percent: bool,
        relative: bool,
    ) -> Option<Self> {
        let before_mid = (before.0 + before.1) / 2.;
        let delta = (after.0 + after.1) / 2. - before_mid;
        if delta.abs() < 0.001 {
            return None;
        }
        Some(Self {
            key,
            label,
            before,
            after,
            delta,
            is_percent,
            delta_pct: (relative && before_mid > 0.).then(|| delta / before_mid * 100.),
        })
    }
}

pub fn compare_performance(
    before: &BuildPerformance,
    after: &BuildPerformance,
) -> Vec<PerformanceDiff> {
    let mut rows = Vec::new();
    let pair = |min: Option<f64>, max: Option<f64>| (min.unwrap_or(0.), max.unwrap_or(0.));
    let average_hit = |perf: &BuildPerformance| {
        perf.damage
            .as_ref()
            .map(|d| (d.avg_min as f64, d.avg_max as f64))
            .unwrap_or_default()
    };
    for (key, label, b, a) in [
        (
            "hit_dps",
            "Hit DPS",
            pair(before.hit_dps_min, before.hit_dps_max),
            pair(after.hit_dps_min, after.hit_dps_max),
        ),
        (
            "combined_dps",
            "Combined DPS",
            pair(before.combined_dps_min, before.combined_dps_max),
            pair(after.combined_dps_min, after.combined_dps_max),
        ),
        (
            "avg_hit",
            "Average Hit",
            average_hit(before),
            average_hit(after),
        ),
    ] {
        rows.extend(PerformanceDiff::between(
            key.into(),
            label.into(),
            b,
            a,
            false,
            true,
        ));
    }
    let config = data::game_config();
    for (is_attribute, b, a) in [
        (true, &before.attributes, &after.attributes),
        (false, &before.stats, &after.stats),
    ] {
        let keys: BTreeSet<_> = b.keys().chain(a.keys()).collect();
        for key in keys {
            let definition = config.stats.iter().find(|stat| stat.key == *key);
            let label = if is_attribute {
                config
                    .attributes
                    .iter()
                    .find(|attr| attr.key == *key)
                    .map(|attr| &attr.name)
            } else {
                definition.map(|stat| &stat.name)
            }
            .unwrap_or(key);
            rows.extend(PerformanceDiff::between(
                format!("{}:{key}", if is_attribute { "attribute" } else { "stat" }),
                label.clone(),
                b.get(key).copied().unwrap_or_default(),
                a.get(key).copied().unwrap_or_default(),
                !is_attribute
                    && definition.is_some_and(|stat| stat.format.as_deref() == Some("percent")),
                false,
            ));
        }
    }
    rows
}

/// Compare the complete planner result, including mercenary and Ether contributions.
pub fn compare_planner(
    before: &super::planner::PlannerPerformance,
    after: &super::planner::PlannerPerformance,
) -> Vec<PerformanceDiff> {
    let mut b = before.current.clone();
    let mut a = after.current.clone();
    b.stats = before.computed.stats.clone();
    a.stats = after.computed.stats.clone();
    let mut rows = compare_performance(&b, &a);
    let keys: BTreeSet<_> = before
        .ether
        .iter()
        .chain(&after.ether)
        .map(|row| &row.key)
        .collect();
    for key in keys {
        let b = before.ether.iter().find(|row| &row.key == key);
        let a = after.ether.iter().find(|row| &row.key == key);
        let description = a.or(b).expect("key comes from an Ether row");
        let b = b.map_or(0., |row| row.total);
        let a = a.map_or(0., |row| row.total);
        rows.extend(PerformanceDiff::between(
            format!("ether:{key}"),
            description.label.clone(),
            (b, b),
            (a, a),
            description.is_percent,
            false,
        ));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::commands::{calc_build_performance, BuildPerformanceInput};

    #[test]
    fn real_root_changes_life_and_strength_and_removal_reverses_it() {
        let input = BuildPerformanceInput {
            level: 40,
            class_id: Some("stormweaver".into()),
            ..Default::default()
        };
        let before = calc_build_performance(input.clone());
        let mut allocated = input;
        allocated.allocated_tree_nodes.insert(0);
        let after = calc_build_performance(allocated);
        let rows = compare_performance(&before, &after);
        assert_eq!(
            rows.iter()
                .find(|r| r.key() == "stat:life")
                .unwrap()
                .delta(),
            25.
        );
        assert_eq!(
            rows.iter()
                .find(|r| r.key() == "attribute:strength")
                .unwrap()
                .delta(),
            5.
        );
        let reversed = compare_performance(&after, &before);
        for row in rows {
            let reverse = reversed.iter().find(|r| r.key() == row.key()).unwrap();
            assert_eq!(reverse.delta(), -row.delta());
            assert_eq!(reverse.before(), row.after());
        }
        assert!(compare_performance(&before, &before).is_empty());
    }

    #[test]
    fn dps_uses_relative_change_and_zero_baseline_has_no_percentage() {
        let before = BuildPerformance {
            hit_dps_min: Some(100.),
            hit_dps_max: Some(200.),
            ..Default::default()
        };
        let after = BuildPerformance {
            hit_dps_min: Some(120.),
            hit_dps_max: Some(240.),
            ..Default::default()
        };
        let rows = compare_performance(&before, &after);
        assert_eq!(rows[0].delta(), 30.);
        assert_eq!(rows[0].delta_pct(), Some(20.));
        let rows = compare_performance(&BuildPerformance::default(), &after);
        assert_eq!(rows[0].delta_pct(), None);
    }
}
