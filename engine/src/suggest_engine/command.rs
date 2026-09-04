use std::collections::HashSet;

use super::algo::{suggest_with_oracle, ProgressPayload, SearchInput};
use super::types::{SuggestInput, SuggestResult};
use crate::calc::build::compute_build_performance;
use crate::calc::commands::{combined_dps_mid, perf_deps, BuildPerformanceInput};
use crate::calc::season::SeasonScope;
use tauri::Emitter;

// SeasonScope is thread_local and Drop clears it, so the oracle re-enters the
// scope on every call: rayon workers get the right season, and no outer scope
// exists for a nested Drop to clobber.
pub fn run_suggest(input: &SuggestInput, progress: impl Fn(u32, u32) + Sync) -> SuggestResult {
    let season = input.perf.season.clone();
    let oracle = |alloc: &HashSet<u32>| -> f64 {
        let _scope = SeasonScope::enter(season.clone());
        combined_dps_mid(&input.active_skill_ids, |main| {
            let main = main.or(input.perf.main_skill_id.as_deref());
            let mut deps = perf_deps(&input.perf, &input.perf.inventory, main);
            deps.allocated_tree_nodes = alloc;
            compute_build_performance(&deps)
        })
    };
    let allocated: HashSet<u32> = input.perf.allocated_tree_nodes.iter().copied().collect();
    let mut result = suggest_with_oracle(
        SearchInput {
            graph: &input.graph,
            allocated,
            budget: input.budget,
        },
        oracle,
        progress,
    );
    let _scope = SeasonScope::enter(season);
    result.unsupported_lines = unsupported_lines_for(&result.added_nodes, &input.perf);
    result
}

fn unsupported_lines_for(added: &[u32], perf: &BuildPerformanceInput) -> Vec<String> {
    use crate::calc::tree::parse::{classify_tree_node_line, TreeLineClass};
    added
        .iter()
        .chain(perf.allocated_tree_nodes.iter())
        .filter_map(|id| crate::calc::data::get_tree_node(*id))
        .flat_map(|node| node.lines.iter())
        .filter(|line| matches!(classify_tree_node_line(line), TreeLineClass::Unknown))
        .cloned()
        .collect()
}

// A panic inside the search must degrade to "no suggestions" instead of
// re-panicking on the IPC runtime thread.
async fn join_or_default(
    task: tauri::async_runtime::JoinHandle<SuggestResult>,
) -> SuggestResult {
    task.await.unwrap_or_else(|e| {
        eprintln!("suggest_tree_nodes task panicked: {e}");
        SuggestResult::default()
    })
}

#[tauri::command]
pub async fn suggest_tree_nodes(
    app: tauri::AppHandle,
    input: SuggestInput,
) -> SuggestResult {
    join_or_default(tauri::async_runtime::spawn_blocking(move || {
        run_suggest(&input, |current, total| {
            let _ = app.emit("suggest-progress", ProgressPayload { current, total });
        })
    }))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panicked_task_returns_default_result() {
        let result = tauri::async_runtime::block_on(async {
            let task = tauri::async_runtime::spawn_blocking(|| -> SuggestResult {
                panic!("boom")
            });
            join_or_default(task).await
        });
        assert_eq!(result, SuggestResult::default());
    }

    // Wiring guard: the suggester's final_dps must equal a direct recompute of
    // the returned allocation through the same real-calc oracle.
    #[test]
    fn real_data_suggest_matches_direct_recompute() {
        use crate::calc::types::EquippedItem;
        use std::collections::HashMap;

        let _scope = SeasonScope::enter(None);
        let skills = crate::calc::data::get_skills_by_class("amazon");
        let skill = skills
            .iter()
            .find(|s| s.damage_formula.is_some() || s.damage_per_rank.is_some())
            .expect("amazon has a damage skill");

        let mut node_ids: Vec<u32> = crate::calc::data::tree_nodes()
            .iter()
            .filter(|(_, n)| {
                n.lines
                    .iter()
                    .any(|l| l.contains("Enhanced Damage") || l.contains("to Strength"))
            })
            .filter_map(|(id, _)| id.parse::<u32>().ok())
            .collect();
        node_ids.sort_unstable();
        node_ids.truncate(5);
        assert!(node_ids.len() >= 3, "need real stat nodes, got {node_ids:?}");

        let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();
        for (i, &id) in node_ids.iter().enumerate() {
            let mut nbrs = Vec::new();
            if i > 0 {
                nbrs.push(node_ids[i - 1]);
            }
            if i + 1 < node_ids.len() {
                nbrs.push(node_ids[i + 1]);
            }
            adjacency.insert(id, nbrs);
        }
        let graph = super::super::types::TreeGraph {
            adjacency,
            start_ids: vec![node_ids[0]],
            valuable_ids: node_ids.clone(),
            ..Default::default()
        };

        let mut skill_ranks = HashMap::new();
        skill_ranks.insert(skill.id.clone(), 20u32);
        let mut inventory = crate::calc::types::Inventory::new();
        inventory.insert(
            "weapon".to_string(),
            EquippedItem {
                base_id: "base_mace_ogre_maul".to_string(),
                ..Default::default()
            },
        );
        let perf = BuildPerformanceInput {
            class_id: Some("amazon".to_string()),
            level: 60,
            skill_ranks,
            inventory,
            main_skill_id: Some(skill.id.clone()),
            ..Default::default()
        };
        let input = super::super::types::SuggestInput {
            perf,
            active_skill_ids: vec![skill.id.clone()],
            graph,
            budget: 3,
        };

        let started = std::time::Instant::now();
        let result = run_suggest(&input, |_, _| {});
        eprintln!("suggest on real data took {:?}", started.elapsed());

        assert!(result.base_dps > 0.0, "baseline dps missing");
        assert!(result.final_dps > result.base_dps, "stat nodes must raise dps");

        let final_alloc: HashSet<u32> = result.added_nodes.iter().copied().collect();
        let _scope = SeasonScope::enter(None);
        let direct = combined_dps_mid(&input.active_skill_ids, |main| {
            let main = main.or(input.perf.main_skill_id.as_deref());
            let mut deps = perf_deps(&input.perf, &input.perf.inventory, main);
            deps.allocated_tree_nodes = &final_alloc;
            compute_build_performance(&deps)
        });
        let rel = (result.final_dps - direct).abs() / direct.max(1.0);
        assert!(rel < 1e-9, "suggester {} vs direct {direct}", result.final_dps);
    }
}
