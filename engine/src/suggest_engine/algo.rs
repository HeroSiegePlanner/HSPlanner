use std::collections::{HashMap, HashSet, VecDeque};

use rayon::prelude::*;

use super::types::{SuggestResult, SuggestStep, TreeGraph};

pub struct SearchInput<'a> {
    pub graph: &'a TreeGraph,
    pub allocated: HashSet<u32>,
    pub budget: u32,
}

// Bounds apply to expensive full-build evaluations, not the tree's point budget.
const BEAM_WIDTH: usize = 6;
const SEARCH_PROBES: usize = 12_000;
const EPSILON: f64 = 1e-6;

#[derive(Clone)]
struct Allocation {
    nodes: HashSet<u32>,
    order: Vec<u32>,
    dps: f64,
}

fn key(nodes: &HashSet<u32>) -> Vec<u32> {
    let mut ids: Vec<_> = nodes.iter().copied().collect();
    ids.sort_unstable();
    ids
}

/// All shortest paid paths in one BFS. Roots and neighbours of already paid
/// nodes both cost one point. Sorting makes equal-cost routes reproducible.
fn paths_from(nodes: &HashSet<u32>, graph: &TreeGraph, budget: usize) -> Vec<Vec<u32>> {
    let mut frontier = graph.start_ids.clone();
    for id in nodes {
        frontier.extend(graph.adjacency.get(id).into_iter().flatten().copied());
    }
    frontier.sort_unstable();
    frontier.dedup();
    let mut paths = HashMap::<u32, Vec<u32>>::new();
    let mut queue = VecDeque::new();
    for id in frontier {
        if budget > 0 && !nodes.contains(&id) && graph.adjacency.contains_key(&id) {
            paths.insert(id, vec![id]);
            queue.push_back(id);
        }
    }
    while let Some(id) = queue.pop_front() {
        let path = paths[&id].clone();
        if path.len() >= budget {
            continue;
        }
        let mut neighbours = graph.adjacency.get(&id).cloned().unwrap_or_default();
        neighbours.sort_unstable();
        for next in neighbours {
            if nodes.contains(&next)
                || paths.contains_key(&next)
                || !graph.adjacency.contains_key(&next)
            {
                continue;
            }
            let mut extended = path.clone();
            extended.push(next);
            paths.insert(next, extended);
            queue.push_back(next);
        }
    }
    let mut paths: Vec<_> = paths.into_values().collect();
    paths.sort();
    paths
}

// Single-node probes rank routes only; they never veto a node. Keep short and
// notable routes too, including neutral/negative setup for nonlinear synergies.
fn candidate_paths(
    state: &Allocation,
    graph: &TreeGraph,
    budget: usize,
    impact: &HashMap<u32, f64>,
) -> Vec<Vec<u32>> {
    let mut paths = paths_from(&state.nodes, graph, budget);
    if paths.len() <= 64 {
        return paths;
    }
    let estimate = |p: &Vec<u32>| {
        p.iter()
            .map(|id| impact.get(id).copied().unwrap_or(0.))
            .sum::<f64>()
    };
    paths.sort_by(|a, b| {
        (estimate(b) / b.len() as f64)
            .total_cmp(&(estimate(a) / a.len() as f64))
            .then(a.cmp(b))
    });
    let mut selected: Vec<_> = paths.iter().take(24).cloned().collect();
    paths.sort_by(|a, b| estimate(b).total_cmp(&estimate(a)).then(a.cmp(b)));
    selected.extend(paths.iter().take(8).cloned());
    let notable: HashSet<_> = graph
        .valuable_ids
        .iter()
        .chain(&graph.jewelry_ids)
        .collect();
    selected.extend(
        paths
            .iter()
            .filter(|p| notable.contains(p.last().unwrap()))
            .take(8)
            .cloned(),
    );
    paths.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));
    selected.extend(paths.into_iter().take(8));
    selected.sort();
    selected.dedup();
    selected
}

fn evaluate<O, C>(
    candidates: &mut [Allocation],
    cache: &mut HashMap<Vec<u32>, f64>,
    oracle: &O,
    cancelled: &C,
) -> Result<usize, String>
where
    O: Fn(&HashSet<u32>) -> f64 + Sync,
    C: Fn() -> bool + Sync,
{
    let mut missing = HashMap::new();
    for state in candidates.iter() {
        let ids = key(&state.nodes);
        if !cache.contains_key(&ids) {
            missing.entry(ids).or_insert(&state.nodes);
        }
    }
    let values: Result<Vec<_>, String> = missing
        .into_par_iter()
        .map(|(ids, nodes)| {
            if cancelled() {
                return Err("Operation cancelled.".into());
            }
            let dps = oracle(nodes);
            if !dps.is_finite() {
                return Err("Cannot suggest nodes: calculated DPS is not finite.".into());
            }
            Ok((ids, dps))
        })
        .collect();
    let values = values?;
    let count = values.len();
    cache.extend(values);
    for state in candidates {
        state.dps = cache[&key(&state.nodes)];
    }
    Ok(count)
}

fn expand(state: &Allocation, paths: Vec<Vec<u32>>) -> Vec<Allocation> {
    paths
        .into_iter()
        .map(|path| {
            let mut next = state.clone();
            next.nodes.extend(path.iter().copied());
            next.order.extend(path);
            next
        })
        .collect()
}

fn better(a: &Allocation, b: &Allocation) -> bool {
    a.dps > b.dps + EPSILON
        || ((a.dps - b.dps).abs() <= EPSILON
            && (a.order.len(), &a.order) < (b.order.len(), &b.order))
}

fn retain_state(layer: &mut Vec<Allocation>, state: Allocation) {
    if layer.iter().any(|s| s.nodes == state.nodes) {
        return;
    }
    layer.push(state);
    layer.sort_by(|a, b| b.dps.total_cmp(&a.dps).then(a.order.cmp(&b.order)));
    layer.truncate(BEAM_WIDTH);
}

/// Budget-layer beam search with a complete gain-per-point rollout as incumbent.
/// Exact build DPS ranks allocations; bounded route screening and beam width
/// make this a heuristic, not a guarantee of the global optimum.
pub fn suggest_with_oracle<O, P>(input: SearchInput, oracle: O, progress: P) -> SuggestResult
where
    O: Fn(&HashSet<u32>) -> f64 + Sync,
    P: Fn(u32, u32),
{
    suggest_with_oracle_controlled(input, oracle, progress, || false).unwrap_or_default()
}

pub fn suggest_with_oracle_controlled<O, P, C>(
    input: SearchInput,
    oracle: O,
    progress: P,
    cancelled: C,
) -> Result<SuggestResult, String>
where
    O: Fn(&HashSet<u32>) -> f64 + Sync,
    P: Fn(u32, u32),
    C: Fn() -> bool + Sync,
{
    if cancelled() {
        return Err("Operation cancelled.".into());
    }
    let initial = input.allocated;
    let budget = (input.budget as usize).min(input.graph.adjacency.len());
    let total = budget as u32 * 2 + 1;
    progress(0, total);
    let mut cache = HashMap::new();
    let mut baseline = vec![Allocation {
        nodes: initial.clone(),
        order: vec![],
        dps: 0.,
    }];
    evaluate(&mut baseline, &mut cache, &oracle, &cancelled)?;
    let start = baseline.pop().unwrap();
    let base_dps = start.dps;
    let mut best = start.clone();
    let mut layers = vec![Vec::new(); budget + 1];
    layers[0].push(start.clone());

    // Restrict screening to nodes reachable within the budget; disconnected
    // islands and unaffordable destinations must not consume oracle work.
    let mut singles: Vec<_> = paths_from(&initial, input.graph, budget)
        .iter()
        .map(|p| {
            let id = *p.last().unwrap();
            let mut state = start.clone();
            state.nodes.insert(id);
            state.order.push(id);
            state
        })
        .collect();
    evaluate(&mut singles, &mut cache, &oracle, &cancelled)?;
    let impact: HashMap<_, _> = singles
        .iter()
        .map(|s| (s.order[0], s.dps - base_dps))
        .collect();

    // Complete a cheap rollout first, so the bounded alternative search always
    // has a usable allocation even when its evaluation allowance is exhausted.
    let mut current = start;
    while current.order.len() < budget {
        if cancelled() {
            return Err("Operation cancelled.".into());
        }
        let paths = candidate_paths(&current, input.graph, budget - current.order.len(), &impact);
        let mut candidates = expand(&current, paths);
        evaluate(&mut candidates, &mut cache, &oracle, &cancelled)?;
        candidates.retain(|s| s.dps > current.dps + EPSILON);
        candidates.sort_by(|a, b| {
            let score = |s: &Allocation| {
                (s.dps - current.dps) / (s.order.len() - current.order.len()) as f64
            };
            score(b).total_cmp(&score(a)).then(a.order.cmp(&b.order))
        });
        let Some(next) = candidates.into_iter().next() else {
            break;
        };
        current = next;
        retain_state(&mut layers[current.order.len()], current.clone());
        if better(&current, &best) {
            best = current.clone();
        }
        progress(current.order.len() as u32, total);
    }
    let mut probes = 0;
    for spent in 0..budget {
        if cancelled() {
            return Err("Operation cancelled.".into());
        }
        progress(budget as u32 + spent as u32, total);
        let states = std::mem::take(&mut layers[spent]);
        for state in states {
            if probes >= SEARCH_PROBES {
                break;
            }
            let paths = candidate_paths(&state, input.graph, budget - spent, &impact);
            let mut candidates = expand(&state, paths);
            candidates.truncate(SEARCH_PROBES - probes);
            probes += evaluate(&mut candidates, &mut cache, &oracle, &cancelled)?;
            for next in candidates {
                if better(&next, &best) {
                    best = next.clone();
                }
                let cost = next.order.len();
                retain_state(&mut layers[cost], next);
            }
        }
        if probes >= SEARCH_PROBES {
            break;
        }
    }

    // Replay the winning legal order, never reuse deltas from discarded paths.
    let mut nodes = initial;
    let mut previous = base_dps;
    let mut sequence = Vec::new();
    for &id in &best.order {
        if cancelled() {
            return Err("Operation cancelled.".into());
        }
        nodes.insert(id);
        let mut replay = vec![Allocation {
            nodes: nodes.clone(),
            order: vec![],
            dps: 0.,
        }];
        evaluate(&mut replay, &mut cache, &oracle, &cancelled)?;
        let dps = replay[0].dps;
        sequence.push(SuggestStep {
            node_id: id,
            dps_before: previous,
            dps_after: dps,
            gain: dps - previous,
            is_filler: dps - previous <= EPSILON,
        });
        previous = dps;
    }
    if cancelled() {
        return Err("Operation cancelled.".into());
    }
    let mut added_nodes = best.order.clone();
    added_nodes.sort_unstable();
    let mut used_starts: Vec<_> = input
        .graph
        .start_ids
        .iter()
        .copied()
        .filter(|id| nodes.contains(id))
        .collect();
    used_starts.sort_unstable();
    used_starts.dedup();
    progress(total, total);
    Ok(SuggestResult {
        budget_used: added_nodes.len() as u32,
        budget_requested: input.budget,
        added_nodes,
        sequence,
        base_dps,
        final_dps: previous,
        used_starts,
        unsupported_lines: vec![],
    })
}

#[cfg(test)]
fn reachable_from_starts(
    starts: &HashSet<u32>,
    allowed: &HashSet<u32>,
    graph: &TreeGraph,
) -> HashSet<u32> {
    let mut seen: HashSet<_> = starts.intersection(allowed).copied().collect();
    let mut queue: VecDeque<_> = seen.iter().copied().collect();
    while let Some(id) = queue.pop_front() {
        for &next in graph.adjacency.get(&id).into_iter().flatten() {
            if allowed.contains(&next) && seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    seen
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
        TreeGraph {
            adjacency,
            start_ids: vec![0],
            ..Default::default()
        }
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
            SearchInput {
                graph,
                allocated: HashSet::new(),
                budget,
            },
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
            SearchInput {
                graph: &graph,
                allocated: HashSet::new(),
                budget: 3,
            },
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
            assert!(
                reachable.contains(n),
                "node {n} detached: {:?}",
                result.added_nodes
            );
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

    #[test]
    fn chooses_total_gain_over_a_greedy_dead_end() {
        // Taking +6 first strands the budget; the other branch costs three for +15.
        let graph = TreeGraph {
            adjacency: [
                (0, vec![1, 2]),
                (1, vec![0]),
                (2, vec![0, 3]),
                (3, vec![2, 4]),
                (4, vec![3]),
            ]
            .into(),
            start_ids: vec![0],
            ..Default::default()
        };
        let result = suggest_with_oracle(
            SearchInput {
                graph: &graph,
                allocated: [0].into(),
                budget: 3,
            },
            value_oracle([(1, 6.), (4, 15.)].into()),
            |_, _| {},
        );
        assert_eq!(result.added_nodes, vec![2, 3, 4]);
        assert_eq!(result.final_dps, 115.);
    }

    #[test]
    fn finds_synergy_across_neutral_and_negative_nodes() {
        let graph = TreeGraph {
            adjacency: [(0, vec![1, 2]), (1, vec![0]), (2, vec![0])].into(),
            start_ids: vec![0],
            ..Default::default()
        };
        let oracle = |nodes: &HashSet<u32>| {
            if nodes.contains(&1) && nodes.contains(&2) {
                150.
            } else if nodes.contains(&1) {
                90.
            } else {
                100.
            }
        };
        let result = suggest_with_oracle(
            SearchInput {
                graph: &graph,
                allocated: [0].into(),
                budget: 2,
            },
            oracle,
            |_, _| {},
        );
        assert_eq!(result.added_nodes, vec![1, 2]);
        let mut nodes = HashSet::from([0]);
        let mut dps = result.base_dps;
        for step in result.sequence {
            assert_eq!(step.dps_before, dps);
            nodes.insert(step.node_id);
            assert_eq!(step.dps_after, oracle(&nodes));
            assert_eq!(step.gain, step.dps_after - dps);
            dps = step.dps_after;
        }
        assert_eq!(dps, 150.);
    }

    #[test]
    fn charges_roots_and_preserves_existing_allocations() {
        let graph = TreeGraph {
            adjacency: [
                (0, vec![1]),
                (1, vec![0]),
                (10, vec![11]),
                (11, vec![10]),
                (99, vec![]),
            ]
            .into(),
            start_ids: vec![0, 10],
            ..Default::default()
        };
        let oracle = value_oracle([(1, 1.), (11, 100.), (99, 1000.)].into());
        let result = suggest_with_oracle(
            SearchInput {
                graph: &graph,
                allocated: [0, 1].into(),
                budget: 1,
            },
            &oracle,
            |_, _| {},
        );
        assert!(result.added_nodes.is_empty());
        let result = suggest_with_oracle(
            SearchInput {
                graph: &graph,
                allocated: [0, 1].into(),
                budget: 2,
            },
            oracle,
            |_, _| {},
        );
        assert_eq!(
            result
                .sequence
                .iter()
                .map(|s| s.node_id)
                .collect::<Vec<_>>(),
            vec![10, 11]
        );
        assert_eq!(result.final_dps, 201.);
    }

    #[test]
    fn shortest_route_prefers_paid_node_over_unpaid_root() {
        let graph = TreeGraph {
            adjacency: [(0, vec![2]), (1, vec![2]), (2, vec![0, 1])].into(),
            start_ids: vec![0, 1],
            ..Default::default()
        };
        let result = suggest_with_oracle(
            SearchInput {
                graph: &graph,
                allocated: [1].into(),
                budget: 1,
            },
            value_oracle([(2, 10.)].into()),
            |_, _| {},
        );
        assert_eq!(result.added_nodes, vec![2]);
    }

    #[test]
    fn reports_monotonic_progress_and_honours_cancellation() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let graph = chain_graph(5);
        let reports = std::cell::RefCell::new(Vec::new());
        let result = suggest_with_oracle_controlled(
            SearchInput {
                graph: &graph,
                allocated: HashSet::new(),
                budget: 3,
            },
            |_| 100.,
            |current, total| reports.borrow_mut().push((current, total)),
            || false,
        )
        .unwrap();
        assert_eq!(result.budget_used, 0);
        let reports = reports.into_inner();
        assert!(reports
            .windows(2)
            .all(|p| p[0].0 <= p[1].0 && p[0].1 == p[1].1));
        assert_eq!(reports.last().unwrap().0, reports.last().unwrap().1);
        let calls = AtomicUsize::new(0);
        let result = suggest_with_oracle_controlled(
            SearchInput {
                graph: &graph,
                allocated: HashSet::new(),
                budget: 3,
            },
            |_| {
                calls.fetch_add(1, Ordering::Relaxed);
                100.
            },
            |_, _| {},
            || calls.load(Ordering::Relaxed) > 1,
        );
        assert!(result.unwrap_err().contains("cancelled"));
    }

    #[test]
    fn zero_budget_and_nonfinite_scores_are_safe() {
        let graph = chain_graph(2);
        let result = run(&graph, 0, [(0, 10.)].into());
        assert_eq!(result.budget_used, 0);
        assert_eq!(result.final_dps, result.base_dps);
        assert!(suggest_with_oracle_controlled(
            SearchInput {
                graph: &graph,
                allocated: HashSet::new(),
                budget: 1,
            },
            |_| f64::NAN,
            |_, _| {},
            || false
        )
        .is_err());
    }
}
