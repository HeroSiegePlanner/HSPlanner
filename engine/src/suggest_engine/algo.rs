use std::collections::{HashMap, HashSet, VecDeque};

use rayon::prelude::*;
use serde::Serialize;

use super::types::{SuggestResult, SuggestStep, TreeGraph};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub current: u32,
    pub total: u32,
}

fn reachable_from_starts(
    starts: &HashSet<u32>,
    allowed: &HashSet<u32>,
    graph: &TreeGraph,
) -> HashSet<u32> {
    let mut seen: HashSet<u32> = HashSet::new();
    let mut queue: VecDeque<u32> = VecDeque::new();
    for &s in starts {
        if !allowed.contains(&s) {
            continue;
        }
        if seen.insert(s) {
            queue.push_back(s);
        }
    }
    while let Some(cur) = queue.pop_front() {
        let Some(nbrs) = graph.adjacency.get(&cur) else { continue };
        for &nb in nbrs {
            if !allowed.contains(&nb) {
                continue;
            }
            if seen.insert(nb) {
                queue.push_back(nb);
            }
        }
    }
    seen
}

/// Shortest path from `allocated ∪ virtual_starts` to `target`; traversed virtual
/// starts are included so the caller pays for them from the budget.
fn find_path_to(
    allocated: &HashSet<u32>,
    virtual_starts: &HashSet<u32>,
    target: u32,
    graph: &TreeGraph,
) -> Option<Vec<u32>> {
    if allocated.contains(&target) {
        return Some(Vec::new());
    }
    let mut parent: HashMap<u32, Option<u32>> = HashMap::new();
    let mut queue: VecDeque<u32> = VecDeque::new();
    for &s in allocated.iter().chain(virtual_starts.iter()) {
        if parent.contains_key(&s) {
            continue;
        }
        parent.insert(s, None);
        queue.push_back(s);
    }
    while let Some(cur) = queue.pop_front() {
        let Some(nbrs) = graph.adjacency.get(&cur) else { continue };
        for &nb in nbrs {
            if parent.contains_key(&nb) {
                continue;
            }
            parent.insert(nb, Some(cur));
            if nb == target {
                let mut path: Vec<u32> = Vec::new();
                let mut node = target;
                loop {
                    path.push(node);
                    if allocated.contains(&node) {
                        path.pop();
                        break;
                    }
                    match parent.get(&node).copied().flatten() {
                        Some(p) => node = p,
                        None => break,
                    }
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(nb);
        }
    }
    None
}

pub struct SearchInput<'a> {
    pub graph: &'a TreeGraph,
    pub allocated: HashSet<u32>,
    pub budget: u32,
}

/// Search core: impact filter -> greedy path scoring -> top-up -> 2-opt swap.
/// The oracle is the single source of DPS truth; probes run on rayon workers.
pub fn suggest_with_oracle<O, P>(input: SearchInput, oracle: O, progress: P) -> SuggestResult
where
    O: Fn(&HashSet<u32>) -> f64 + Sync,
    P: Fn(u32, u32),
{
    let graph = input.graph;
    let jewelry_set: HashSet<u32> = graph.jewelry_ids.iter().copied().collect();
    let valuable_set: HashSet<u32> = graph.valuable_ids.iter().copied().collect();
    let start_set: HashSet<u32> = graph.start_ids.iter().copied().collect();

    let mut allocated = input.allocated;
    let initial = allocated.clone();

    let base_dps = oracle(&allocated);
    let mut current_dps = base_dps;
    let mut sequence: Vec<SuggestStep> = Vec::new();

    // Drop notables that can't move DPS (jewelry passes: a jewel may be slotted later).
    let candidates: Vec<u32> = valuable_set
        .iter()
        .copied()
        .filter(|v| !allocated.contains(v))
        .collect();
    let mut valuable_with_impact: HashSet<u32> = candidates
        .par_iter()
        .copied()
        .filter(|&v| {
            if jewelry_set.contains(&v) {
                return true;
            }
            let mut probe = allocated.clone();
            probe.insert(v);
            oracle(&probe) > base_dps + 1e-6
        })
        .collect();

    // Greedy: best gain-per-node along the cheapest path, so a notable 5 filler
    // hops away can beat a locally-best 1-hop pick.
    let mut remaining_budget = input.budget;
    loop {
        progress(sequence.len() as u32, input.budget);
        if remaining_budget == 0 {
            break;
        }
        let targets: Vec<u32> = valuable_with_impact
            .iter()
            .copied()
            .filter(|t| !allocated.contains(t))
            .collect();
        let scored: Vec<(u32, Vec<u32>, f64)> = targets
            .par_iter()
            .filter_map(|&t| {
                let path = find_path_to(&allocated, &start_set, t, graph)?;
                if path.is_empty() || (path.len() as u32) > remaining_budget {
                    return None;
                }
                let mut probe = allocated.clone();
                probe.extend(path.iter().copied());
                let dps = oracle(&probe);
                if dps - current_dps <= 1e-6 {
                    return None;
                }
                Some((t, path, dps))
            })
            .collect();
        let best = scored.into_iter().max_by(|a, b| {
            let sa = (a.2 - current_dps) / a.1.len() as f64;
            let sb = (b.2 - current_dps) / b.1.len() as f64;
            sa.partial_cmp(&sb)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.0.cmp(&a.0))
        });
        let Some((target, path, final_dps)) = best else { break };

        // Walk the path one node at a time so filler steps show their tiny gain.
        let mut step_dps = current_dps;
        for &node in &path {
            allocated.insert(node);
            let d = oracle(&allocated);
            sequence.push(SuggestStep {
                node_id: node,
                dps_before: step_dps,
                dps_after: d,
                gain: d - step_dps,
                is_filler: d - step_dps <= 1e-6,
            });
            step_dps = d;
        }
        for &node in &path {
            valuable_with_impact.remove(&node);
        }
        valuable_with_impact.remove(&target);
        remaining_budget = remaining_budget.saturating_sub(path.len() as u32);
        current_dps = final_dps;
    }

    // Top-up: spend stranded budget greedily on frontier nodes that still gain.
    while remaining_budget > 0 {
        let mut frontier: HashSet<u32> = start_set.difference(&allocated).copied().collect();
        for id in &allocated {
            if let Some(nbrs) = graph.adjacency.get(id) {
                for &nb in nbrs {
                    if !allocated.contains(&nb) {
                        frontier.insert(nb);
                    }
                }
            }
        }
        let frontier: Vec<u32> = frontier.into_iter().collect();
        let best = frontier
            .par_iter()
            .filter_map(|&cand| {
                let mut probe = allocated.clone();
                probe.insert(cand);
                let dps = oracle(&probe);
                if dps > current_dps + 1e-6 { Some((cand, dps)) } else { None }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .max_by(|a, b| {
                a.1.partial_cmp(&b.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(b.0.cmp(&a.0))
            });
        let Some((cand, dps)) = best else { break };
        sequence.push(SuggestStep {
            node_id: cand,
            dps_before: current_dps,
            dps_after: dps,
            gain: dps - current_dps,
            is_filler: false,
        });
        allocated.insert(cand);
        valuable_with_impact.remove(&cand);
        remaining_budget -= 1;
        current_dps = dps;
        progress(sequence.len() as u32, input.budget);
    }

    // 2-opt: swap any added node for a frontier neighbour while DPS improves.
    const SWAP_MAX_PASSES: u32 = 60;
    for pass in 0..SWAP_MAX_PASSES {
        progress(sequence.len() as u32 + pass, sequence.len() as u32 + SWAP_MAX_PASSES);
        let removable: Vec<u32> = allocated.difference(&initial).copied().collect();
        if removable.is_empty() {
            break;
        }
        let swaps: Vec<(u32, u32)> = removable
            .iter()
            .flat_map(|&rm| {
                let mut without = allocated.clone();
                without.remove(&rm);
                // Removing `rm` must not detach the rest from a PAID start.
                let reachable = reachable_from_starts(&start_set, &without, graph);
                if !without.iter().all(|n| reachable.contains(n)) {
                    return Vec::new();
                }
                let mut frontier: HashSet<u32> =
                    start_set.difference(&without).copied().collect();
                for id in &without {
                    if let Some(nbrs) = graph.adjacency.get(id) {
                        for &nb in nbrs {
                            if !without.contains(&nb) {
                                frontier.insert(nb);
                            }
                        }
                    }
                }
                frontier
                    .into_iter()
                    .filter(|&add| add != rm && !allocated.contains(&add))
                    .map(|add| (rm, add))
                    .collect::<Vec<_>>()
            })
            .collect();
        let best_swap = swaps
            .par_iter()
            .filter_map(|&(rm, add)| {
                let mut new_alloc = allocated.clone();
                new_alloc.remove(&rm);
                new_alloc.insert(add);
                let dps = oracle(&new_alloc);
                if dps > current_dps + 1e-6 { Some((rm, add, dps)) } else { None }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .max_by(|a, b| {
                a.2.partial_cmp(&b.2)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(b.1.cmp(&a.1))
            });
        match best_swap {
            None => break,
            Some((rm, add, new_dps)) => {
                let gain = new_dps - current_dps;
                allocated.remove(&rm);
                allocated.insert(add);
                current_dps = new_dps;
                sequence.retain(|s| s.node_id != rm);
                sequence.push(SuggestStep {
                    node_id: add,
                    dps_before: current_dps - gain,
                    dps_after: current_dps,
                    gain,
                    is_filler: false,
                });
            }
        }
    }

    progress(sequence.len() as u32, input.budget);
    let mut added_nodes: Vec<u32> = allocated.difference(&initial).copied().collect();
    added_nodes.sort_unstable();
    let budget_used = sequence.len() as u32;
    let mut used_starts: Vec<u32> = allocated.intersection(&start_set).copied().collect();
    used_starts.sort_unstable();
    SuggestResult {
        added_nodes,
        sequence,
        base_dps,
        final_dps: current_dps,
        budget_used,
        budget_requested: input.budget,
        unsupported_lines: Vec::new(),
        used_starts,
    }
}

#[cfg(test)]
mod oracle_search_tests {
    use super::*;

    fn chain_graph(n: u32) -> TreeGraph {
        let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();
        for i in 0..n {
            let mut nbrs = Vec::new();
            if i > 0 {
                nbrs.push(i - 1);
            }
            if i + 1 < n {
                nbrs.push(i + 1);
            }
            adjacency.insert(i, nbrs);
        }
        TreeGraph { adjacency, start_ids: vec![0], ..Default::default() }
    }

    // Oracle: dps = 100 + sum of per-node values over the allocation.
    fn value_oracle(values: HashMap<u32, f64>) -> impl Fn(&HashSet<u32>) -> f64 + Sync {
        move |alloc| {
            100.0
                + alloc
                    .iter()
                    .map(|id| values.get(id).copied().unwrap_or(0.0))
                    .sum::<f64>()
        }
    }

    fn run(graph: &TreeGraph, budget: u32, values: HashMap<u32, f64>) -> SuggestResult {
        suggest_with_oracle(
            SearchInput { graph, allocated: HashSet::new(), budget },
            value_oracle(values),
            |_, _| {},
        )
    }

    #[test]
    fn spends_full_budget_on_gaining_fillers() {
        let graph = chain_graph(5);
        let values: HashMap<u32, f64> = (0..5).map(|i| (i, 5.0)).collect();
        let result = run(&graph, 4, values);
        assert_eq!(result.budget_used, 4);
        assert_eq!(result.added_nodes.len(), 4);
        assert!(result.final_dps > result.base_dps);
    }

    #[test]
    fn routes_through_dead_fillers_to_reach_a_notable() {
        // 0 and 1 are worthless; 2 is a notable worth 50.
        let mut graph = chain_graph(6);
        graph.valuable_ids = vec![2];
        let mut values: HashMap<u32, f64> = HashMap::new();
        values.insert(2, 50.0);
        for i in [3u32, 4, 5] {
            values.insert(i, 5.0);
        }
        let result = run(&graph, 5, values);
        // Path 0,1,2 costs 3; the leftover 2 points must go into the +5 fillers.
        assert_eq!(result.budget_used, 5);
        assert!(result.added_nodes.contains(&2));
    }

    #[test]
    fn keeps_budget_when_nothing_improves_dps() {
        let graph = chain_graph(5);
        let result = run(&graph, 4, HashMap::new());
        assert_eq!(result.budget_used, 0);
        assert!(result.added_nodes.is_empty());
    }

    #[test]
    fn never_suggests_nodes_detached_from_paid_starts() {
        // Second component 10-11 behind an unpaid start with a huge node.
        let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();
        adjacency.insert(0, vec![1]);
        adjacency.insert(1, vec![0, 2]);
        adjacency.insert(2, vec![1]);
        adjacency.insert(10, vec![11]);
        adjacency.insert(11, vec![10]);
        let graph = TreeGraph {
            adjacency,
            start_ids: vec![0, 10],
            ..Default::default()
        };
        let mut values: HashMap<u32, f64> = HashMap::new();
        for i in [0u32, 1, 2] {
            values.insert(i, 5.0);
        }
        values.insert(11, 100.0);
        let result = suggest_with_oracle(
            SearchInput { graph: &graph, allocated: HashSet::new(), budget: 3 },
            value_oracle(values),
            |_, _| {},
        );
        let alloc: HashSet<u32> = result.added_nodes.iter().copied().collect();
        let starts: HashSet<u32> = result
            .used_starts
            .iter()
            .copied()
            .filter(|s| alloc.contains(s))
            .collect();
        let reachable = reachable_from_starts(&starts, &alloc, &graph);
        for n in &alloc {
            assert!(reachable.contains(n), "node {n} detached: {:?}", result.added_nodes);
        }
    }

    #[test]
    fn deterministic_across_runs() {
        let mut graph = chain_graph(8);
        graph.valuable_ids = vec![3, 6];
        let values: HashMap<u32, f64> = (0..8).map(|i| (i, (i % 3) as f64)).collect();
        let a = run(&graph, 5, values.clone());
        let b = run(&graph, 5, values);
        assert_eq!(a.added_nodes, b.added_nodes);
        assert_eq!(a.sequence, b.sequence);
    }
}
