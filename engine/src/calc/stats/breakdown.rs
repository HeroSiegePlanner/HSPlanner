use super::*;

// ---------- per-stat breakdown ----------
// PoB-style explainability: surfaces additive/more lists per stat key and
// regroups them by source type for the UI.

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatTypeSubtotal {
    pub source_type: SourceType,
    pub sum: Ranged,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatBreakdown {
    pub stat_key: String,
    pub stat_name: String,
    pub is_percent: bool,
    pub has_more: bool,
    /// Only set for apply_multipliers_pass targets (life/mana/replenishes)
    /// when `increased_X` has contributions; UI uses it to split sections.
    pub has_increased: bool,

    /// Flat-units sources at `stat_key` for multiplied stats; percent
    /// sources for non-multiplied percent stats.
    pub additive_sum: Ranged,
    pub additive_sources: Vec<SourceContribution>,
    pub additive_by_type: Vec<StatTypeSubtotal>,

    pub increased_sum: Ranged,
    pub increased_sources: Vec<SourceContribution>,
    pub increased_by_type: Vec<StatTypeSubtotal>,

    pub more_sum: Ranged,
    pub more_sources: Vec<SourceContribution>,
    pub more_by_type: Vec<StatTypeSubtotal>,

    pub combined: Ranged,
    /// Pre-diminishing-returns total, set only when the curve reduced this stat.
    pub pre_diminish: Option<Ranged>,
}

// For multiplied-flat stats the percent multipliers live under different
// keys than the flat base; mirror apply_multipliers_pass calls.
pub(crate) fn multiplier_keys_for(stat_key: &str) -> (Option<&'static str>, Option<&'static str>) {
    match stat_key {
        "life" => (Some("increased_life"), Some("increased_life_more")),
        "mana" => (Some("increased_mana"), Some("increased_mana_more")),
        "mana_replenish" => (None, Some("mana_replenish_more")),
        "life_replenish" => (None, Some("life_replenish_more")),
        "light_radius" => (Some("light_radius_pct"), None),
        "bleed_duration" => (Some("bleed_duration_pct"), None),
        "burning_duration" => (Some("burning_duration_pct"), None),
        "frostbite_duration" => (Some("frostbite_duration_pct"), None),
        "permafrost_duration" => (Some("permafrost_duration_pct"), None),
        "poisoned_duration" => (Some("poisoned_duration_pct"), None),
        "rabies_duration" => (Some("rabies_duration_pct"), None),
        "shadowburn_duration" => (Some("shadowburn_duration_pct"), None),
        "stasis_duration" => (Some("stasis_duration_pct"), None),
        _ => (None, None),
    }
}

// `_more` → "Total X"; falls back to raw key when no def.
pub(crate) fn resolved_stat_name(stat_key: &str) -> String {
    if let Some(def) = STAT_DEFS_MAP.get(stat_key) {
        return def.name.clone();
    }
    if let Some(base) = stat_key.strip_suffix("_more") {
        if let Some(def) = STAT_DEFS_MAP.get(base) {
            return format!("Total {}", def.name);
        }
    }
    stat_key.to_string()
}

// Sorted by |magnitude| desc; NaN sinks; ties broken by SourceType for
// stable ordering across runs (HashMap iteration is non-deterministic).
pub(crate) fn group_by_source_type(sources: &[SourceContribution]) -> Vec<StatTypeSubtotal> {
    let mut map: HashMap<SourceType, (Ranged, u32)> = HashMap::new();
    for s in sources.iter() {
        let entry = map.entry(s.source_type).or_insert(((0.0, 0.0), 0));
        entry.0 .0 += s.value.0;
        entry.0 .1 += s.value.1;
        entry.1 += 1;
    }
    let mut out: Vec<StatTypeSubtotal> = map
        .into_iter()
        .map(|(source_type, (sum, count))| StatTypeSubtotal {
            source_type,
            sum,
            count,
        })
        .collect();
    let safe_mag = |v: f64| if v.is_nan() { f64::NEG_INFINITY } else { v };
    out.sort_by(|a, b| {
        let a_mag = safe_mag(a.sum.0.abs().max(a.sum.1.abs()));
        let b_mag = safe_mag(b.sum.0.abs().max(b.sum.1.abs()));
        b_mag
            .partial_cmp(&a_mag)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.source_type.cmp(&b.source_type))
    });
    out
}

// Builds a StatBreakdown for `stat_key`; `final_value` becomes `combined` so the
// modal can't disagree with the StatRow. Pass `None` for attributes.
pub fn compute_stat_breakdown(
    stat_sources: &SourceMap,
    stat_key: &str,
    final_value: Option<Ranged>,
) -> StatBreakdown {
    let additive_sources: Vec<SourceContribution> =
        stat_sources.get(stat_key).cloned().unwrap_or_default();

    let (inc_key_opt, more) = multiplier_keys_for(stat_key);
    let more_key_owned = more.map(|s| s.to_string());
    let more_key = more_key_owned.unwrap_or_else(|| format!("{stat_key}_more"));

    let increased_sources: Vec<SourceContribution> = inc_key_opt
        .and_then(|k| stat_sources.get(k))
        .cloned()
        .unwrap_or_default();
    let more_sources: Vec<SourceContribution> =
        stat_sources.get(&more_key).cloned().unwrap_or_default();

    let additive_sum = sum_contributions(&additive_sources);
    let increased_sum = sum_contributions(&increased_sources);
    let more_sum = sum_contributions(&more_sources);
    let has_more = !more_sources.is_empty();
    let has_increased = !increased_sources.is_empty();

    // Prefer the engine's final value so the modal can't diverge from the
    // StatRow; reconstruct only when no final value is supplied.
    let combined_raw: Ranged = if let Some(fv) = final_value {
        fv
    } else if inc_key_opt.is_some() || matches!(stat_key, "mana_replenish" | "life_replenish") {
        let min = additive_sum.0 * (1.0 + increased_sum.0 / 100.0) * (1.0 + more_sum.0 / 100.0);
        let max = additive_sum.1 * (1.0 + increased_sum.1 / 100.0) * (1.0 + more_sum.1 / 100.0);
        (min, max)
    } else if has_more {
        combine_additive_and_more(additive_sum, more_sum)
    } else {
        additive_sum
    };
    // Normalise min/max — negative-spanning factor ranges can flip order
    // (e.g. (-100,-50) × (-50,50)).
    let combined: Ranged = if combined_raw.0 <= combined_raw.1 {
        combined_raw
    } else {
        (combined_raw.1, combined_raw.0)
    };

    // Sources always hold pre-diminish contributions; when the curve reduced
    // this stat, expose their combined total so the modal can show both.
    let pre_diminish = data::game_config()
        .diminishing_returns
        .as_ref()
        .and_then(|defs| defs.get(stat_key))
        .map(|_| combine_additive_and_more(additive_sum, more_sum))
        .filter(|raw| (raw.0 - combined.0).abs() > 1e-6 || (raw.1 - combined.1).abs() > 1e-6);

    let additive_by_type = group_by_source_type(&additive_sources);
    let increased_by_type = group_by_source_type(&increased_sources);
    let more_by_type = group_by_source_type(&more_sources);

    let is_percent = stat_def(stat_key)
        .and_then(|d| d.format.as_deref())
        .map(|f| f == "percent")
        .unwrap_or(false);

    StatBreakdown {
        stat_key: stat_key.to_string(),
        stat_name: resolved_stat_name(stat_key),
        is_percent,
        has_more,
        has_increased,
        additive_sum,
        additive_sources,
        additive_by_type,
        increased_sum,
        increased_sources,
        increased_by_type,
        more_sum,
        more_sources,
        more_by_type,
        combined,
        pre_diminish,
    }
}
