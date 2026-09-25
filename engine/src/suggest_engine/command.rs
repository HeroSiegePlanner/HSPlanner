use std::collections::HashSet;

use super::algo::{suggest_with_oracle, SearchInput};
use super::types::{SuggestInput, SuggestResult};
use crate::calc::build::compute_build_performance;
use crate::calc::commands::{combined_dps_mid, perf_deps, BuildPerformanceInput};
use crate::calc::season::SeasonScope;

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

/// Native searches reserve worker capacity for input and node previews.
pub fn run_suggest_controlled(
    input: &SuggestInput,
    cancellation: &crate::task_control::Cancellation,
    progress: impl Fn(u32, u32) + Sync,
) -> Result<SuggestResult, String> {
    use std::sync::OnceLock;
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    let pool = POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(
                std::thread::available_parallelism()
                    .map_or(1, |n| n.get().saturating_sub(2).clamp(1, 4)),
            )
            .thread_name(|index| format!("hsplanner-suggest-{index}"))
            .build()
            .expect("start suggestion workers")
    });
    cancellation.check()?;
    let season = input.perf.season.clone();
    let oracle = |allocated: &HashSet<u32>| {
        if cancellation.is_cancelled() {
            return 0.;
        }
        let _scope = SeasonScope::enter(season.clone());
        combined_dps_mid(&input.active_skill_ids, |main| {
            let mut deps = perf_deps(
                &input.perf,
                &input.perf.inventory,
                main.or(input.perf.main_skill_id.as_deref()),
            );
            deps.allocated_tree_nodes = allocated;
            compute_build_performance(&deps)
        })
    };
    let mut result = pool.install(|| {
        super::algo::suggest_with_oracle_controlled(
            SearchInput {
                graph: &input.graph,
                allocated: input.perf.allocated_tree_nodes.clone(),
                budget: input.budget,
            },
            oracle,
            &progress,
            || cancellation.is_cancelled(),
        )
    })?;
    let _scope = SeasonScope::enter(season);
    result.unsupported_lines = unsupported_lines_for(&result.added_nodes, &input.perf);
    Ok(result)
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(
            node_ids.len() >= 3,
            "need real stat nodes, got {node_ids:?}"
        );

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
        assert!(
            result.final_dps > result.base_dps,
            "stat nodes must raise dps"
        );

        let final_alloc: HashSet<u32> = result.added_nodes.iter().copied().collect();
        let _scope = SeasonScope::enter(None);
        let direct = combined_dps_mid(&input.active_skill_ids, |main| {
            let main = main.or(input.perf.main_skill_id.as_deref());
            let mut deps = perf_deps(&input.perf, &input.perf.inventory, main);
            deps.allocated_tree_nodes = &final_alloc;
            compute_build_performance(&deps)
        });
        let rel = (result.final_dps - direct).abs() / direct.max(1.0);
        assert!(
            rel < 1e-9,
            "suggester {} vs direct {direct}",
            result.final_dps
        );
    }

    #[test]
    #[ignore = "full Incarnation graph performance smoke test"]
    fn full_incarnation_graph_preserves_budget_and_replay() {
        use super::super::types::TreeGraph;
        use crate::calc::types::EquippedItem;
        let raw: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/incarnation-tree.json")).unwrap();
        let mut graph = TreeGraph::default();
        for node in raw["nodes"].as_array().unwrap() {
            let id = node["id"].as_u64().unwrap() as u32;
            graph.adjacency.insert(id, vec![]);
            if node["t"] == "root" {
                graph.start_ids.push(id);
            }
            if node["r"].as_f64().unwrap() >= 10. {
                graph.valuable_ids.push(id);
            }
        }
        for edge in raw["edges"].as_array().unwrap() {
            let a = edge[0].as_u64().unwrap() as u32;
            let b = edge[1].as_u64().unwrap() as u32;
            graph.adjacency.get_mut(&a).unwrap().push(b);
            graph.adjacency.get_mut(&b).unwrap().push(a);
        }
        let skill = crate::calc::data::get_skills_by_class("amazon")
            .iter()
            .find(|s| s.damage_formula.is_some() || s.damage_per_rank.is_some())
            .unwrap();
        let input = SuggestInput {
            perf: BuildPerformanceInput {
                class_id: Some("amazon".into()),
                level: 100,
                main_skill_id: Some(skill.id.clone()),
                skill_ranks: [(skill.id.clone(), 20)].into(),
                inventory: [(
                    "weapon".into(),
                    EquippedItem {
                        base_id: "base_mace_ogre_maul".into(),
                        ..Default::default()
                    },
                )]
                .into(),
                ..Default::default()
            },
            active_skill_ids: vec![skill.id.clone()],
            graph,
            budget: 200,
        };
        let started = std::time::Instant::now();
        let result = run_suggest_controlled(&input, &Default::default(), |_, _| {}).unwrap();
        eprintln!(
            "full tree: {:?}, {} nodes, DPS {} -> {}",
            started.elapsed(),
            result.budget_used,
            result.base_dps,
            result.final_dps
        );
        assert!(result.budget_used <= input.budget);
        assert!(result.final_dps > result.base_dps);
        let mut paid = HashSet::new();
        for step in &result.sequence {
            assert!(
                input.graph.start_ids.contains(&step.node_id)
                    || input.graph.adjacency[&step.node_id]
                        .iter()
                        .any(|n| paid.contains(n))
            );
            assert!(paid.insert(step.node_id));
        }
        let mut perf = input.perf.clone();
        perf.allocated_tree_nodes = paid;
        let direct = run_suggest(
            &SuggestInput {
                perf,
                budget: 0,
                ..input
            },
            |_, _| {},
        );
        assert_eq!(result.final_dps, direct.base_dps);
    }
}
