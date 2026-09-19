use std::{collections::HashSet, rc::Rc, sync::Arc, time::Instant};

#[cfg(test)]
use hsplanner_engine::calc::build::BuildPerformance;
use hsplanner_engine::calc::{
    performance_diff::{PerformanceDiff, compare_planner},
    planner::{PlannerPerformance, evaluate},
};

use crate::tree::{Graph, TreeKind};
use hsplanner_build::{BuildSnapshot, loadout::LoadoutKind, session::Session};

#[cfg(test)]
use hsplanner_engine::calc::commands::BuildPerformanceInput;

#[cfg(test)]
pub fn example_input() -> BuildSnapshot {
    let input: BuildPerformanceInput =
        serde_json::from_str(include_str!("../assets/example-build.json"))
            .expect("valid example build");
    let mut value = serde_json::to_value(&input).unwrap();
    value["allocated"] = serde_json::to_value(input.allocated_attrs).unwrap();
    value["activeSkillIds"] =
        serde_json::json!(input.main_skill_id.into_iter().collect::<Vec<_>>());
    value
        .as_object_mut()
        .unwrap()
        .retain(|_, value| !value.is_null());
    serde_json::from_value(value).unwrap()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentKey {
    build: Option<String>,
    loadouts: [String; 4],
    revision: u64,
}
impl DocumentKey {
    pub fn from_session(session: &Session) -> Self {
        Self {
            build: session.draft().build_id.clone(),
            loadouts: LoadoutKind::ALL
                .map(|kind| session.draft().loadouts.active_id(kind).to_owned()),
            revision: session.calculation_revision(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CalculationRequest {
    pub kind: TreeKind,
    pub document: DocumentKey,
    pub selected: Vec<u32>,
    pub preview: Option<PreviewRequest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewRequest {
    pub node_id: u32,
    pub single: Vec<u32>,
    pub path: Vec<u32>,
    pub added: usize,
    pub removed: usize,
}

fn node_ids(graph: &Graph, indices: &HashSet<usize>) -> Vec<u32> {
    let mut ids: Vec<_> = indices
        .iter()
        .map(|&ix| graph.nodes[ix].id as u32)
        .collect();
    ids.sort_unstable();
    ids
}

impl CalculationRequest {
    pub fn new(graph: &Graph, selected: &HashSet<usize>, hovered: Option<usize>) -> Self {
        let preview = hovered.map(|ix| {
            let mut single = selected.clone();
            if !single.remove(&ix) {
                single.insert(ix);
            }
            let mut path = selected.clone();
            graph.toggle(&mut path, ix);
            PreviewRequest {
                node_id: graph.nodes[ix].id as u32,
                single: node_ids(graph, &single),
                path: node_ids(graph, &path),
                added: path.difference(selected).count(),
                removed: selected.difference(&path).count(),
            }
        });
        Self {
            kind: graph.kind,
            document: DocumentKey::default(),
            selected: node_ids(graph, selected),
            preview,
        }
    }
}

#[derive(Clone)]
pub struct PreviewResult {
    pub single: Vec<PerformanceDiff>,
    pub path: Vec<PerformanceDiff>,
    pub added: usize,
    pub removed: usize,
}

pub struct CalculationResult {
    pub request: CalculationRequest,
    pub current: Arc<PlannerPerformance>,
    pub preview: Option<PreviewResult>,
    pub milliseconds: f64,
}

pub fn calculate(
    input: &BuildSnapshot,
    request: CalculationRequest,
    cached: Option<Arc<PlannerPerformance>>,
) -> CalculationResult {
    let start = Instant::now();
    let compute = |nodes: &[u32]| {
        let mut input = input.clone();
        request.kind.apply(&mut input, nodes);
        evaluate(&input.planner_input())
    };
    let current = cached.unwrap_or_else(|| Arc::new(compute(&request.selected)));
    let preview = request.preview.as_ref().map(|preview| {
        let single = compute(&preview.single);
        let single_diff = compare_planner(&current, &single);
        let path_diff = if preview.path == preview.single {
            single_diff.clone()
        } else {
            compare_planner(&current, &compute(&preview.path))
        };
        PreviewResult {
            single: single_diff,
            path: path_diff,
            added: preview.added,
            removed: preview.removed,
        }
    });
    CalculationResult {
        request,
        current,
        preview,
        milliseconds: start.elapsed().as_secs_f64() * 1000.,
    }
}

#[derive(Default)]
pub struct BuildSession {
    pub request: CalculationRequest,
    pub result: Option<Rc<CalculationResult>>,
    pub in_flight: bool,
    pub error: Option<String>,
}

impl BuildSession {
    #[cfg(test)]
    pub fn current(&self) -> Option<&BuildPerformance> {
        self.result
            .as_ref()
            .filter(|result| {
                result.request.document == self.request.document
                    && result.request.selected == self.request.selected
            })
            .map(|result| &result.current.current)
    }
    pub fn performance(&self) -> Option<Arc<PlannerPerformance>> {
        self.result
            .as_ref()
            .filter(|result| {
                result.request.document == self.request.document
                    && result.request.selected == self.request.selected
            })
            .map(|result| result.current.clone())
    }

    pub fn preview(&self) -> Option<&PreviewResult> {
        self.result
            .as_ref()
            .filter(|result| result.request == self.request)
            .and_then(|result| result.preview.as_ref())
    }

    pub fn is_current(&self) -> bool {
        self.result
            .as_ref()
            .is_some_and(|result| result.request == self.request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ether_preview_matches_commit_and_keeps_incarnation_and_equipment() {
        let graph = Graph::load_ether();
        let mut input = example_input();
        input.allocated_tree_nodes = vec![0];
        let request = CalculationRequest::new(&graph, &HashSet::new(), Some(20));
        let result = calculate(&input, request.clone(), None);
        let mut committed = input.clone();
        committed.allocated_ether_nodes = request.preview.unwrap().path;
        let after = evaluate(&committed.planner_input());
        assert_eq!(committed.allocated_tree_nodes, [0]);
        let expected = compare_planner(&result.current, &after);
        assert!(!expected.is_empty());
        let actual = result.preview.unwrap().path;
        assert_eq!(
            actual
                .iter()
                .map(|r| (r.key(), r.delta()))
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|r| (r.key(), r.delta()))
                .collect::<Vec<_>>()
        );
        assert!(actual.iter().any(|r| r.key().starts_with("ether:")));
    }

    #[test]
    fn preview_matches_the_click_including_disconnected_branch_removal() {
        let graph = Graph::load();
        let input = example_input();
        let target = graph.nodes.len() - 1;
        let empty = HashSet::new();
        let request = CalculationRequest::new(&graph, &empty, Some(target));
        let before = calculate(&input, request.clone(), None);
        assert!(
            before
                .current
                .current
                .combined_dps_min
                .is_some_and(|dps| dps > 0.)
        );
        let mut selected = empty;
        graph.toggle(&mut selected, target);
        let clicked = CalculationRequest::new(&graph, &selected, None);
        assert_eq!(clicked.selected, request.preview.unwrap().path);
        let after = calculate(&input, clicked, None);
        let expected = compare_planner(&before.current, &after.current);
        let preview = before.preview.unwrap();
        assert_eq!(
            preview
                .path
                .iter()
                .map(|r| (r.key(), r.delta()))
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|r| (r.key(), r.delta()))
                .collect::<Vec<_>>()
        );
        let root = *graph.roots.iter().find(|ix| selected.contains(ix)).unwrap();
        let remove = CalculationRequest::new(&graph, &selected, Some(root));
        let preview = remove.preview.unwrap();
        assert!(preview.path.is_empty());
        assert!(preview.removed > 1);
        assert!(!preview.single.is_empty());
    }

    #[test]
    fn stale_hover_or_selection_never_supplies_new_tooltip_numbers() {
        let graph = Graph::load();
        let first = CalculationRequest::new(&graph, &HashSet::new(), Some(0));
        let result = calculate(&example_input(), first.clone(), None);
        let mut session = BuildSession {
            request: first,
            result: Some(Rc::new(result)),
            ..Default::default()
        };
        assert!(session.preview().is_some());
        session.request = CalculationRequest::new(&graph, &HashSet::new(), Some(1));
        assert!(session.current().is_some());
        assert!(session.preview().is_none());
        assert!(!session.is_current());
        session.request = CalculationRequest::new(&graph, &HashSet::from([0]), Some(1));
        assert!(session.current().is_none());
        assert!(session.preview().is_none());
    }

    #[test]
    fn lightning_node_changes_the_example_skills_dps() {
        let graph = Graph::load();
        let ix = graph.nodes.iter().position(|node| node.id == 1025).unwrap();
        let request = CalculationRequest::new(&graph, &HashSet::new(), Some(ix));
        let result = calculate(&example_input(), request, None);
        let preview = result.preview.unwrap();
        assert!(
            preview
                .single
                .iter()
                .any(|row| row.key() == "combined_dps" && row.delta() > 0.)
        );
        assert!(preview.added > 1);
    }

    #[test]
    fn calculations_use_domain_ids_instead_of_canvas_indices() {
        let mut graph = Graph::load();
        graph.nodes.swap(0, 1);
        let request = CalculationRequest::new(&graph, &HashSet::from([0]), None);
        assert_eq!(request.selected, [1]);
    }
}
