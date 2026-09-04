use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::calc::commands::BuildPerformanceInput;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeGraph {
    pub adjacency: HashMap<u32, Vec<u32>>,
    pub start_ids: Vec<u32>,
    pub warp_ids: Vec<u32>,
    pub valuable_ids: Vec<u32>,
    pub jewelry_ids: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestInput {
    pub perf: BuildPerformanceInput,
    #[serde(default)]
    pub active_skill_ids: Vec<String>,
    pub graph: TreeGraph,
    pub budget: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestStep {
    pub node_id: u32,
    pub dps_before: f64,
    pub dps_after: f64,
    pub gain: f64,
    pub is_filler: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestResult {
    pub added_nodes: Vec<u32>,
    pub sequence: Vec<SuggestStep>,
    pub base_dps: f64,
    pub final_dps: f64,
    pub budget_used: u32,
    pub budget_requested: u32,
    pub unsupported_lines: Vec<String>,
    pub used_starts: Vec<u32>,
}
