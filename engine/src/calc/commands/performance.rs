use super::*;

// ---------- compute_build_performance command ----------

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildPerformanceInput {
    #[serde(default)]
    pub class_id: Option<String>,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub allocated_attrs: HashMap<String, u32>,
    #[serde(default)]
    pub inventory: Inventory,
    #[serde(default)]
    pub skill_ranks: HashMap<String, u32>,
    #[serde(default)]
    pub subskill_ranks: HashMap<String, u32>,
    #[serde(default)]
    pub active_aura_id: Option<String>,
    #[serde(default)]
    pub active_buffs: HashMap<String, bool>,
    #[serde(default)]
    pub custom_stats: Vec<CustomStat>,
    #[serde(default)]
    pub allocated_tree_nodes: HashSet<u32>,
    #[serde(default)]
    pub tree_socketed: HashMap<u32, TreeSocketContent>,
    #[serde(default)]
    pub main_skill_id: Option<String>,
    #[serde(default)]
    pub enemy_conditions: HashMap<String, bool>,
    #[serde(default)]
    pub player_conditions: HashMap<String, bool>,
    #[serde(default)]
    pub skill_projectiles: HashMap<String, u32>,
    #[serde(default)]
    pub enemy_resistances: HashMap<String, f64>,
    #[serde(default)]
    pub proc_toggles: HashMap<String, bool>,
    #[serde(default)]
    pub kills_per_sec: f64,
    #[serde(default)]
    pub entity_rates: HashMap<String, f64>,
    #[serde(default)]
    pub stack_counts: HashMap<String, u32>,
    #[serde(default)]
    pub season: Option<String>,
    #[serde(default)]
    pub granted_skill_ranks: HashMap<String, calc::Ranged>,
    #[serde(default)]
    pub difficulty: Option<String>,
}

pub(crate) fn perf_deps<'a>(
    input: &'a BuildPerformanceInput,
    inventory: &'a Inventory,
    main_skill_id: Option<&'a str>,
) -> BuildPerformanceDeps<'a> {
    BuildPerformanceDeps {
        class_id: input.class_id.as_deref(),
        level: input.level,
        allocated_attrs: &input.allocated_attrs,
        inventory,
        skill_ranks: &input.skill_ranks,
        subskill_ranks: &input.subskill_ranks,
        active_aura_id: input.active_aura_id.as_deref(),
        active_buffs: &input.active_buffs,
        custom_stats: &input.custom_stats,
        allocated_tree_nodes: &input.allocated_tree_nodes,
        tree_socketed: &input.tree_socketed,
        main_skill_id,
        enemy_conditions: &input.enemy_conditions,
        player_conditions: &input.player_conditions,
        skill_projectiles: &input.skill_projectiles,
        enemy_resistances: &input.enemy_resistances,
        proc_toggles: &input.proc_toggles,
        kills_per_sec: input.kills_per_sec,
        entity_rates: &input.entity_rates,
        stack_counts: &input.stack_counts,
        granted_skill_ranks: Some(&input.granted_skill_ranks),
        difficulty: input.difficulty.as_deref(),
    }
}

// Mirrors frontend mergeCombinedPerformance: per-skill execute multipliers,
// proc DPS counted once from the primary skill, ailment DPS included.
pub(crate) fn combined_dps_mid<F>(active_skill_ids: &[String], run: F) -> f64
where
    F: Fn(Option<&str>) -> BuildPerformance,
{
    if active_skill_ids.len() > 1 {
        let runs: Vec<BuildPerformance> =
            active_skill_ids.iter().map(|sid| run(Some(sid))).collect();
        let exec_sum = |pick: fn(&BuildPerformance) -> Option<f64>| -> f64 {
            runs.iter()
                .filter_map(|r| pick(r).map(|v| v * r.execute_mult))
                .sum()
        };
        let primary_exec = runs[0].execute_mult;
        let min = exec_sum(|r| r.avg_hit_dps_min)
            + runs[0].proc_dps_min * primary_exec
            + exec_sum(|r| r.ailment_dps_min);
        let max = exec_sum(|r| r.avg_hit_dps_max)
            + runs[0].proc_dps_max * primary_exec
            + exec_sum(|r| r.ailment_dps_max);
        (min + max) / 2.0
    } else {
        let p = run(active_skill_ids.first().map(String::as_str));
        let mid = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (Some(x), Some(y)) => Some((x + y) / 2.0),
            _ => None,
        };
        mid(p.combined_dps_min, p.combined_dps_max)
            .or_else(|| mid(p.hit_dps_min, p.hit_dps_max))
            .unwrap_or(0.0)
    }
}

#[tauri::command]
pub fn calc_build_performance(input: BuildPerformanceInput) -> BuildPerformance {
    let _scope = crate::calc::season::SeasonScope::enter(input.season.clone());
    compute_build_performance(&perf_deps(
        &input,
        &input.inventory,
        input.main_skill_id.as_deref(),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankSlotItemsInput {
    pub perf: BuildPerformanceInput,
    pub slot: String,
    pub base_ids: Vec<String>,
    #[serde(default)]
    pub active_skill_ids: Vec<String>,
}

// Ranks candidate bases for one slot by combined-DPS midpoint; multi-skill sums
// avg-hit DPS per active skill and counts proc DPS once (mirrors the frontend).
#[tauri::command]
pub fn rank_slot_items(input: RankSlotItemsInput) -> HashMap<String, f64> {
    let _scope = crate::calc::season::SeasonScope::enter(input.perf.season.clone());
    let mut out: HashMap<String, f64> = HashMap::with_capacity(input.base_ids.len());
    for base_id in &input.base_ids {
        let mut inventory = input.perf.inventory.clone();
        inventory.insert(
            input.slot.clone(),
            EquippedItem {
                base_id: base_id.clone(),
                ..Default::default()
            },
        );
        let dps = combined_dps_mid(&input.active_skill_ids, |main| {
            let main = main.or(input.perf.main_skill_id.as_deref());
            compute_build_performance(&perf_deps(&input.perf, &inventory, main))
        });
        out.insert(base_id.clone(), dps);
    }
    out
}

/// Warm-up progress: `current` of `total` tree nodes parsed. Emitted as
/// "warmup-progress" so the boot splash can drive an honest 0–15% slice.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WarmupProgress {
    current: u32,
    total: u32,
}

/// Warms the data LazyLock and parser caches so the first real calc isn't stuck
/// compiling ~300 regexes on the UI thread. IO-free so tests can drive it.
pub fn run_warmup<F: FnMut(u32, u32)>(mut on_progress: F) -> bool {
    let d = crate::calc::data::data();
    if d.items.is_empty() {
        return false;
    }
    let nodes = crate::calc::data::tree_nodes();
    let total = nodes.len() as u32;
    // Cap emitted ticks so a ~1000-node warm-up doesn't flood the event channel.
    let step = (nodes.len() / 20).max(1);
    for (i, node) in nodes.values().enumerate() {
        for line in &node.lines {
            let _ = crate::calc::tree::parse::parse_tree_node_mod(line);
            let _ = crate::calc::tree::parse::parse_tree_node_meta(line);
        }
        if i % step == 0 {
            on_progress(i as u32, total);
        }
    }
    on_progress(total, total);
    true
}

/// Tauri command: runs the warm-up off the event loop so the webview stays
/// responsive, emitting "warmup-progress" for the boot splash.
#[tauri::command]
pub async fn calc_warmup(app: tauri::AppHandle, season: Option<String>) -> bool {
    // Scope lives inside the blocking closure so it never crosses an .await.
    tauri::async_runtime::spawn_blocking(move || {
        let _scope = crate::calc::season::SeasonScope::enter(season);
        run_warmup(|current, total| {
            let _ = app.emit("warmup-progress", WarmupProgress { current, total });
        })
    })
    .await
    .unwrap_or(false)
}

/// Returns full stats plus per-stat source breakdown rendered by StatsView and
/// the tooltips. Reuses `BuildPerformanceInput`; damage/proc fields are unused here.
#[tauri::command]
pub fn calc_build_stats(input: BuildPerformanceInput) -> ComputedStats {
    let _scope = crate::calc::season::SeasonScope::enter(input.season.clone());
    let stats_input = BuildStatsInput {
        class_id: input.class_id.as_deref(),
        level: input.level,
        allocated_attrs: &input.allocated_attrs,
        inventory: &input.inventory,
        skill_ranks: &input.skill_ranks,
        active_aura_id: input.active_aura_id.as_deref(),
        active_buffs: &input.active_buffs,
        custom_stats: &input.custom_stats,
        allocated_tree_nodes: &input.allocated_tree_nodes,
        tree_socketed: &input.tree_socketed,
        player_conditions: &input.player_conditions,
        subskill_ranks: &input.subskill_ranks,
        enemy_conditions: &input.enemy_conditions,
        stack_counts: &input.stack_counts,
        granted_skill_ranks: Some(&input.granted_skill_ranks),
        main_skill_id: input.main_skill_id.as_deref(),
        difficulty: input.difficulty.as_deref(),
        entity_rates: &input.entity_rates,
    };
    compute_build_stats(&stats_input)
}

/// Backs StatBreakdownModal: re-runs stats and slices out one key's additive/more
/// contributions, subtotals and final value. `kind` picks stat vs attribute sources.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatBreakdownInput {
    #[serde(flatten)]
    pub deps: BuildPerformanceInput,
    pub stat_key: String,
    #[serde(default)]
    pub kind: StatBreakdownKind,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum StatBreakdownKind {
    #[default]
    Stat,
    Attribute,
}

#[tauri::command]
pub fn calc_stat_breakdown(input: StatBreakdownInput) -> StatBreakdown {
    let _scope = crate::calc::season::SeasonScope::enter(input.deps.season.clone());
    let stats_input = BuildStatsInput {
        class_id: input.deps.class_id.as_deref(),
        level: input.deps.level,
        allocated_attrs: &input.deps.allocated_attrs,
        inventory: &input.deps.inventory,
        skill_ranks: &input.deps.skill_ranks,
        active_aura_id: input.deps.active_aura_id.as_deref(),
        active_buffs: &input.deps.active_buffs,
        custom_stats: &input.deps.custom_stats,
        allocated_tree_nodes: &input.deps.allocated_tree_nodes,
        tree_socketed: &input.deps.tree_socketed,
        player_conditions: &input.deps.player_conditions,
        subskill_ranks: &input.deps.subskill_ranks,
        enemy_conditions: &input.deps.enemy_conditions,
        stack_counts: &input.deps.stack_counts,
        granted_skill_ranks: Some(&input.deps.granted_skill_ranks),
        main_skill_id: input.deps.main_skill_id.as_deref(),
        difficulty: input.deps.difficulty.as_deref(),
        entity_rates: &input.deps.entity_rates,
    };
    let computed = compute_build_stats(&stats_input);

    let sources = match input.kind {
        StatBreakdownKind::Stat => &computed.stat_sources,
        StatBreakdownKind::Attribute => &computed.attribute_sources,
    };
    let final_value = match input.kind {
        StatBreakdownKind::Stat => computed.stats.get(&input.stat_key).copied(),
        StatBreakdownKind::Attribute => computed.attributes.get(&input.stat_key).copied(),
    };
    compute_stat_breakdown(sources, &input.stat_key, final_value)
}
