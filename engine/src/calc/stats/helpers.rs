use super::*;

// ---------- inline helpers ----------

#[inline]
pub fn is_zero(v: Ranged) -> bool {
    v.0 == 0.0 && v.1 == 0.0
}

// ---------- stat-def lookup with `_more` suffix fallback ----------

pub(crate) static STAT_DEFS_MAP: Lazy<HashMap<&'static str, &'static StatDef>> = Lazy::new(|| {
    let mut m: HashMap<&'static str, &'static StatDef> = HashMap::new();
    for stat in data::game_config().stats.iter() {
        m.insert(stat.key.as_str(), stat);
    }
    m
});

// `_more` keys fall back to base key (UI formatting overlay stays in TS).
pub fn stat_def(key: &str) -> Option<&'static StatDef> {
    if let Some(def) = STAT_DEFS_MAP.get(key) {
        return Some(*def);
    }
    if let Some(base) = key.strip_suffix("_more") {
        return STAT_DEFS_MAP.get(base).copied();
    }
    None
}

// ---------- contribution helpers ----------

pub fn push_source(map: &mut SourceMap, key: &str, source: SourceContribution) {
    if is_zero(source.value) {
        return;
    }
    map.entry(key.to_string()).or_default().push(source);
}

pub fn sum_contributions(sources: &[SourceContribution]) -> Ranged {
    let mut min = 0.0;
    let mut max = 0.0;
    for s in sources {
        min += s.value.0;
        max += s.value.1;
    }
    (min, max)
}

pub fn sum_ranged_from_map(map: &SourceMap, key: &str) -> Ranged {
    let Some(list) = map.get(key) else {
        return (0.0, 0.0);
    };
    if list.is_empty() {
        return (0.0, 0.0);
    }
    let v = sum_contributions(list);
    (v.0.floor(), v.1.floor())
}

#[allow(clippy::too_many_arguments)]
pub fn apply_contribution(
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
    stat_key: &str,
    value: Ranged,
    label: String,
    source_type: SourceType,
    forge: Option<Forge>,
) {
    if is_zero(value) {
        return;
    }
    let def_opt = stat_def(stat_key);
    if let Some(def) = def_opt {
        if def.item_only.unwrap_or(false) {
            return;
        }
    }
    let contribution = SourceContribution {
        label,
        source_type,
        value,
        forge,
    };
    if let Some(def) = def_opt {
        if let Some(target) = def.modifies_attribute.as_deref() {
            if target == "all" {
                for attr in data::game_config().attributes.iter() {
                    push_source(attr_sources, &attr.key, contribution.clone());
                }
            } else {
                push_source(attr_sources, target, contribution);
            }
            return;
        }
    }
    push_source(stat_sources, stat_key, contribution);
}

// ---------- compute_item_effective_defense ----------

// Returns the Ranged defense after applying enhanced_defense% (caller
// star-scales it), or None when the base has no defense range.
pub fn compute_item_effective_defense(
    defense_min: Option<f64>,
    defense_max: Option<f64>,
    enhanced_defense: Option<Ranged>,
) -> Option<Ranged> {
    let (Some(min_base), Some(max_base)) = (defense_min, defense_max) else {
        return None;
    };
    let (ed_min, ed_max) = enhanced_defense.unwrap_or((0.0, 0.0));
    let min = (min_base * (1.0 + ed_min / 100.0)).floor();
    let max = (max_base * (1.0 + ed_max / 100.0)).floor();
    Some((min, max))
}

// ---------- multiplier helpers ----------

// Collapses additive% and more% into a single equivalent additive%,
// rounded to 6 decimals via JS-style Math.round (`(x + 0.5).floor()`).
pub fn combine_additive_and_more(additive: Ranged, more: Ranged) -> Ranged {
    let round = |n: f64| ((n * 1e6) + 0.5).floor() / 1e6;
    let min = round(((1.0 + additive.0 / 100.0) * (1.0 + more.0 / 100.0) - 1.0) * 100.0);
    let max = round(((1.0 + additive.1 / 100.0) * (1.0 + more.1 / 100.0) - 1.0) * 100.0);
    (min, max)
}

// ---------- diminishing returns ----------

// eff = threshold + (raw - threshold)^power, then hard cap. Formula verified
// against the game: 564 raw skill haste hits the 200 hard cap exactly.
pub fn diminished_value(raw: f64, def: &crate::calc::types::DiminishDef) -> f64 {
    let eff = if raw <= def.threshold {
        raw
    } else {
        def.threshold + (raw - def.threshold).powf(def.power)
    };
    match def.cap {
        Some(cap) => eff.min(cap),
        None => eff,
    }
}

/// Folds each configured stat's `_more` twin into the base key, applies the
/// curve to the combined total and zeroes the twin, so every downstream
/// consumer (rate, DPS, statsCombined) reads post-diminish numbers.
/// Skill-scoped node bonuses are added after this pass and escape the curve.
/// Returns the pre-diminish totals for keys the curve actually reduced.
pub fn apply_diminishing_returns(
    stats: &mut HashMap<String, Ranged>,
    defs: &HashMap<String, crate::calc::types::DiminishDef>,
) -> HashMap<String, Ranged> {
    let mut raw_out = HashMap::new();
    for (key, def) in defs {
        let more_key = format!("{key}_more");
        let base = stats.get(key).copied();
        let more = stats.get(&more_key).copied();
        if base.is_none() && more.is_none() {
            continue;
        }
        let raw = combine_additive_and_more(
            base.unwrap_or((0.0, 0.0)),
            more.unwrap_or((0.0, 0.0)),
        );
        let eff = (diminished_value(raw.0, def), diminished_value(raw.1, def));
        if (eff.0 - raw.0).abs() > 1e-9 || (eff.1 - raw.1).abs() > 1e-9 {
            raw_out.insert(key.clone(), raw);
        }
        stats.insert(key.clone(), eff);
        if more.is_some() {
            stats.insert(more_key, (0.0, 0.0));
        }
    }
    raw_out
}

/// Pre-combined `<key>`+`<key>_more` totals for every base key with a `_more` twin.
/// Exposed as `statsCombined` so views render engine numbers, not re-derived ones.
pub fn stats_combined_map(stats: &HashMap<String, Ranged>) -> HashMap<String, Ranged> {
    let mut out = HashMap::new();
    for (key, more) in stats {
        let Some(base_key) = key.strip_suffix("_more") else {
            continue;
        };
        let additive = stats.get(base_key).copied().unwrap_or((0.0, 0.0));
        out.insert(
            base_key.to_string(),
            combine_additive_and_more(additive, *more),
        );
    }
    out
}

// `floor=false` for replenish stats (preserves fractional regen).
pub fn apply_multiplier(
    stats: &mut HashMap<String, Ranged>,
    flat_key: &str,
    pct_key: Option<&str>,
    more_pct_key: Option<&str>,
    floor: bool,
) {
    let Some(flat) = stats.get(flat_key).copied() else {
        return;
    };
    let pct = pct_key
        .and_then(|k| stats.get(k).copied())
        .unwrap_or((0.0, 0.0));
    let more = more_pct_key
        .and_then(|k| stats.get(k).copied())
        .unwrap_or((0.0, 0.0));
    if is_zero(pct) && is_zero(more) {
        return;
    }
    let raw_min = flat.0 * (1.0 + pct.0 / 100.0) * (1.0 + more.0 / 100.0);
    let raw_max = flat.1 * (1.0 + pct.1 / 100.0) * (1.0 + more.1 / 100.0);
    let (min, max) = if floor {
        (raw_min.floor(), raw_max.floor())
    } else {
        (raw_min, raw_max)
    };
    stats.insert(flat_key.to_string(), (min, max));
}

