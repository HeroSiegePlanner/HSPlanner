//! Effective HP per damage type plus the "cap your resistance" hints the
//! Character view shows. Views only format what comes out of here.
use std::collections::HashMap;

use serde::Serialize;

use super::data;
use super::skills::{r_max, Ranged, ELEMENTS};

pub const DEFAULT_RES_CAP: f64 = 75.0;
const INSIGHT_MIN_GAIN_PCT: f64 = 2.0;
const INSIGHT_MAX_COUNT: usize = 3;
pub const DAMAGE_TYPES: [&str; 6] = ["physical", "fire", "cold", "lightning", "poison", "arcane"];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EhpLayer {
    pub label: String,
    pub pct: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EhpEntry {
    #[serde(rename = "type")]
    pub damage_type: String,
    /// `None` once a layer reaches 100%: the build is immune to this type.
    pub ehp: Option<f64>,
    pub multiplier: f64,
    pub layers: Vec<EhpLayer>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EhpResult {
    pub entries: Vec<EhpEntry>,
    pub worst: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefenseInsight {
    pub text: String,
    /// `None` when capping the resistance makes the build immune.
    pub gain_pct: Option<f64>,
}

type Stats = HashMap<String, Ranged>;

fn stat_pct(stats: &Stats, key: &str) -> f64 {
    stats.get(key).map(|v| r_max(*v)).unwrap_or(0.0)
}

/// Config cap raised by any `max_<stat>` the build carries.
pub fn effective_cap(key: &str, stats: &Stats) -> Option<f64> {
    let base = data::game_config()
        .stats
        .iter()
        .find(|d| d.key == key)?
        .cap?;
    Some(base + stat_pct(stats, &format!("max_{key}")))
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

// JS Math.round semantics: half rounds toward +inf.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn or_inf(v: Option<f64>) -> f64 {
    v.unwrap_or(f64::INFINITY)
}

fn multiplier_for(
    damage_type: &str,
    stats: &Stats,
    res_override: Option<f64>,
) -> (f64, Vec<EhpLayer>) {
    let mut layers: Vec<EhpLayer> = Vec::new();
    let mut multiplier = 1.0;
    let mut apply = |label: String, pct: f64, always_show: bool| {
        if pct != 0.0 || always_show {
            layers.push(EhpLayer { label, pct });
        }
        multiplier *= 1.0 - pct / 100.0;
    };

    if damage_type == "physical" {
        apply(
            "Physical damage reduction".to_string(),
            stat_pct(stats, "physical_damage_reduction"),
            false,
        );
    } else {
        let key = format!("{damage_type}_resistance");
        let cap = effective_cap(&key, stats).unwrap_or(DEFAULT_RES_CAP);
        let raw = res_override.unwrap_or_else(|| stat_pct(stats, &key));
        apply(
            format!("{} resistance", capitalize(damage_type)),
            raw.min(cap),
            true,
        );
        apply(
            "Magic damage reduction".to_string(),
            stat_pct(stats, "magic_damage_reduction"),
            false,
        );
        apply(
            "Magic damage taken reduced".to_string(),
            stat_pct(stats, "magic_damage_taken_reduced"),
            false,
        );
    }
    apply(
        "Damage taken reduced".to_string(),
        stat_pct(stats, "damage_taken_reduced"),
        false,
    );
    apply(
        "All damage taken reduced".to_string(),
        stat_pct(stats, "all_damage_taken_reduced_pct"),
        false,
    );
    (multiplier, layers)
}

pub fn compute_ehp(stats: &Stats) -> EhpResult {
    let life = stat_pct(stats, "life");
    if life <= 0.0 {
        return EhpResult::default();
    }
    let entries: Vec<EhpEntry> = DAMAGE_TYPES
        .iter()
        .map(|damage_type| {
            let (multiplier, layers) = multiplier_for(damage_type, stats, None);
            EhpEntry {
                damage_type: damage_type.to_string(),
                ehp: (multiplier > 0.0).then(|| life / multiplier),
                multiplier,
                layers,
            }
        })
        .collect();
    let mut worst: Option<&EhpEntry> = None;
    for entry in &entries {
        if worst.is_none_or(|w| or_inf(entry.ehp) < or_inf(w.ehp)) {
            worst = Some(entry);
        }
    }
    EhpResult {
        worst: worst.map(|e| e.damage_type.clone()),
        entries,
    }
}

pub fn derive_defense_insights(stats: &Stats) -> Vec<DefenseInsight> {
    let life = stat_pct(stats, "life");
    if life <= 0.0 {
        return Vec::new();
    }
    let mut insights: Vec<DefenseInsight> = Vec::new();
    for element in ELEMENTS {
        let key = format!("{element}_resistance");
        let cap = effective_cap(&key, stats).unwrap_or(DEFAULT_RES_CAP);
        let raw = stat_pct(stats, &key);
        if raw >= cap {
            continue;
        }
        let (now, _) = multiplier_for(element, stats, None);
        if now <= 0.0 {
            continue;
        }
        let (capped, _) = multiplier_for(element, stats, Some(cap));
        let gain_pct = (capped > 0.0).then(|| (now / capped - 1.0) * 100.0);
        if gain_pct.is_some_and(|g| g <= INSIGHT_MIN_GAIN_PCT) {
            continue;
        }
        let gain_label = match gain_pct {
            Some(g) => format!("+{}% EHP", js_round(g)),
            None => "immunity".to_string(),
        };
        insights.push(DefenseInsight {
            text: format!(
                "Cap {element} res ({}→{cap}): {gain_label} vs {element}",
                js_round(raw)
            ),
            gain_pct,
        });
    }
    insights.sort_by(|a, b| {
        or_inf(b.gain_pct)
            .partial_cmp(&or_inf(a.gain_pct))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    insights.truncate(INSIGHT_MAX_COUNT);
    insights
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(pairs: &[(&str, f64)]) -> Stats {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), (*v, *v)))
            .collect()
    }

    fn entry<'a>(result: &'a EhpResult, damage_type: &str) -> &'a EhpEntry {
        result
            .entries
            .iter()
            .find(|e| e.damage_type == damage_type)
            .expect("entry")
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn empty_without_life() {
        assert!(compute_ehp(&stats(&[])).worst.is_none());
        assert!(compute_ehp(&stats(&[("life", 0.0)])).entries.is_empty());
    }

    #[test]
    fn per_type_ehp_and_worst() {
        let result = compute_ehp(&stats(&[("life", 1000.0), ("cold_resistance", 50.0)]));
        assert_eq!(result.entries.len(), 6);
        assert!(close(entry(&result, "cold").ehp.unwrap(), 2000.0));
        assert!(close(entry(&result, "physical").ehp.unwrap(), 1000.0));
        assert_eq!(result.worst.as_deref(), Some("physical"));
    }

    #[test]
    fn stacked_layers_multiply() {
        let result = compute_ehp(&stats(&[
            ("life", 1000.0),
            ("physical_damage_reduction", 30.0),
            ("damage_taken_reduced", 10.0),
            ("all_damage_taken_reduced_pct", 20.0),
        ]));
        let physical = entry(&result, "physical");
        assert!(close(physical.ehp.unwrap(), 1000.0 / 0.504));
        assert_eq!(physical.layers.len(), 3);
    }

    #[test]
    fn resistance_caps_apply_first() {
        let result = compute_ehp(&stats(&[("life", 1000.0), ("fire_resistance", 90.0)]));
        let fire = entry(&result, "fire");
        assert!(close(fire.ehp.unwrap(), 4000.0));
        let res_layer = fire
            .layers
            .iter()
            .find(|l| l.label.contains("resistance"))
            .unwrap();
        assert_eq!(res_layer.pct, 75.0);
    }

    #[test]
    fn negative_resistance_lowers_ehp() {
        let result = compute_ehp(&stats(&[("life", 1000.0), ("poison_resistance", -25.0)]));
        assert!(close(entry(&result, "poison").ehp.unwrap(), 800.0));
    }

    #[test]
    fn full_immunity_is_none() {
        let result = compute_ehp(&stats(&[
            ("life", 1000.0),
            ("all_damage_taken_reduced_pct", 100.0),
        ]));
        assert!(result.entries.iter().all(|e| e.ehp.is_none()));
    }

    #[test]
    fn magic_reduction_skips_physical() {
        let result = compute_ehp(&stats(&[
            ("life", 1000.0),
            ("magic_damage_reduction", 20.0),
        ]));
        assert!(close(entry(&result, "fire").ehp.unwrap(), 1250.0));
        assert!(close(entry(&result, "physical").ehp.unwrap(), 1000.0));
    }

    #[test]
    fn insights_need_life() {
        assert!(derive_defense_insights(&stats(&[("cold_resistance", 10.0)])).is_empty());
    }

    #[test]
    fn insight_suggests_capping_with_gain() {
        let insights = derive_defense_insights(&stats(&[
            ("life", 1000.0),
            ("fire_resistance", 75.0),
            ("lightning_resistance", 75.0),
            ("poison_resistance", 75.0),
            ("arcane_resistance", 75.0),
            ("cold_resistance", 12.0),
        ]));
        assert_eq!(insights.len(), 1);
        assert!(close(insights[0].gain_pct.unwrap(), 252.0));
        assert!(insights[0].text.contains("12→75"));
        assert!(insights[0].text.contains("+252% EHP"));
    }

    #[test]
    fn insight_skips_capped_and_tiny_gains() {
        let insights = derive_defense_insights(&stats(&[
            ("life", 1000.0),
            ("fire_resistance", 75.0),
            ("lightning_resistance", 75.0),
            ("poison_resistance", 75.0),
            ("arcane_resistance", 75.0),
            ("cold_resistance", 74.6),
        ]));
        assert!(insights.is_empty());
    }

    #[test]
    fn missing_resistances_are_zero_holes_capped_at_three() {
        let insights = derive_defense_insights(&stats(&[("life", 1000.0)]));
        assert_eq!(insights.len(), 3);
        assert!(close(insights[0].gain_pct.unwrap(), 300.0));
    }

    #[test]
    fn insights_sorted_by_gain_desc() {
        let insights = derive_defense_insights(&stats(&[
            ("life", 1000.0),
            ("fire_resistance", 0.0),
            ("cold_resistance", 30.0),
            ("lightning_resistance", 50.0),
            ("poison_resistance", 60.0),
            ("arcane_resistance", 70.0),
        ]));
        assert_eq!(insights.len(), 3);
        assert!(insights[0].text.contains("fire"));
        assert!(insights[0].gain_pct.unwrap() > insights[2].gain_pct.unwrap());
    }

    #[test]
    fn immunity_gains_sort_stably() {
        let insights = derive_defense_insights(&stats(&[
            ("life", 1000.0),
            ("fire_resistance", 0.0),
            ("max_fire_resistance", 25.0),
            ("cold_resistance", 0.0),
            ("max_cold_resistance", 25.0),
            ("lightning_resistance", 75.0),
            ("poison_resistance", 75.0),
            ("arcane_resistance", 75.0),
        ]));
        assert_eq!(insights.len(), 2);
        assert!(insights.iter().all(|i| i.gain_pct.is_none()));
        assert!(insights.iter().all(|i| i.text.contains("immunity")));
        assert!(insights[0].text.contains("fire"));
        assert!(insights[1].text.contains("cold"));
    }
}
