use std::collections::{HashMap, HashSet, VecDeque};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::engine::{compute_final_state, EngineInputs, FinalState};
use super::types::{
    PrecomputedInput, SuggestResult, SuggestStep, TreeGraph, SkillRef, BonusSource,
};

use crate::calc::skills as calc;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub current: u32,
    pub total: u32,
}

fn skill_ref_to_calc(skill: &SkillRef) -> calc::Skill {
    calc::Skill {
        name: skill.name.trim().to_lowercase(),
        tags: skill.tags.clone(),
        damage_type: skill.damage_type.clone(),
        damage_formula: skill.damage_formula.as_ref().map(|f| calc::DamageFormula {
            base: f.base,
            per_level: f.per_level,
        }),
        damage_per_rank: skill
            .damage_per_rank
            .as_ref()
            .map(|rows| rows.iter().map(|r| calc::DamageRow { min: r.min, max: r.max }).collect()),
        bonus_sources: skill
            .bonus_sources
            .iter()
            .map(|b| match b {
                BonusSource::AttributePoint { source, value } => calc::BonusSource::AttributePoint {
                    source: source.trim().to_lowercase(),
                    value: *value,
                },
                BonusSource::SkillLevel { source, value } => calc::BonusSource::SkillLevel {
                    source: source.trim().to_lowercase(),
                    value: *value,
                },
            })
            .collect(),
        attack_kind: match skill.attack_kind.as_deref() {
            Some("attack") => Some(calc::AttackKind::Attack),
            Some("spell") => Some(calc::AttackKind::Spell),
            _ => None,
        },
        attack_scaling: skill.attack_scaling.as_ref().map(|s| {
            let f = |x: &Option<super::types::DamageFormula>| {
                x.as_ref().map(|d| calc::DamageFormula { base: d.base, per_level: d.per_level })
            };
            calc::AttackSkillScaling {
                weapon_damage_pct: f(&s.weapon_damage_pct),
                flat_physical_min: f(&s.flat_physical_min),
                flat_physical_max: f(&s.flat_physical_max),
                attack_rating_pct: f(&s.attack_rating_pct),
            }
        }),
    }
}

fn normalize_keys<V: Clone>(map: &HashMap<String, V>) -> HashMap<String, V> {
    map.iter()
        .map(|(k, v)| (k.trim().to_lowercase(), v.clone()))
        .collect()
}

/// Immutable lookup tables built once per `suggest()` call; reused for every DPS probe.
struct DpsContext<'a> {
    active: &'a SkillRef,
    active_calc: calc::Skill,
    skills_by_name: HashMap<String, calc::Skill>,
    id_by_normalized_name: HashMap<String, String>,
    skill_ranks_norm: HashMap<String, f64>,
    item_bonuses_norm: HashMap<String, super::types::Ranged>,
    /// Equipped weapon for attack-kind skills; constant across candidates.
    weapon: Option<calc::Weapon>,
}

fn build_dps_context<'a>(input: &'a PrecomputedInput) -> Option<DpsContext<'a>> {
    let active = input.active_skill.as_ref()?;
    if input.active_skill_rank == 0 {
        return None;
    }
    let mut skills_by_name = HashMap::with_capacity(input.all_skills.len());
    let mut id_by_normalized_name = HashMap::with_capacity(input.all_skills.len());
    for s in &input.all_skills {
        let norm = s.name.trim().to_lowercase();
        skills_by_name.insert(norm.clone(), skill_ref_to_calc(s));
        id_by_normalized_name.insert(norm, s.id.clone());
    }
    let weapon = input
        .inventory
        .get("weapon")
        .and_then(|eq| crate::calc::data::get_item(&eq.base_id))
        .and_then(|base| match (base.damage_min, base.damage_max) {
            (Some(min), Some(max)) => Some(calc::Weapon {
                name: base.name.clone(),
                damage_min: min,
                damage_max: max,
            }),
            _ => None,
        });
    Some(DpsContext {
        active,
        active_calc: skill_ref_to_calc(active),
        skills_by_name,
        id_by_normalized_name,
        skill_ranks_norm: normalize_keys(&input.skill_ranks_by_name),
        item_bonuses_norm: crate::calc::rank::aggregate_item_skill_bonuses(
            &input.inventory,
            &crate::calc::data::data().items,
        ),
        weapon,
    })
}

fn compute_dps(state: &FinalState, input: &PrecomputedInput, ctx: &DpsContext) -> f64 {
    let attrs_norm = normalize_keys(&state.attrs);
    // Subtree ranks are fixed while the optimizer varies tree nodes, so skill-scoped
    // terms are constant multipliers and cannot reorder the candidates.
    let no_scoped = calc::StatMap::new();

    let hit_input = calc::SkillInput {
        skill: &ctx.active_calc,
        allocated_rank: input.active_skill_rank as f64,
        attributes: &attrs_norm,
        stats: &state.stats,
        skill_ranks_by_name: &ctx.skill_ranks_norm,
        item_skill_bonuses: &ctx.item_bonuses_norm,
        enemy_conditions: &input.enemy_conditions,
        enemy_resistances: &input.enemy_resistances,
        skills_by_name: &ctx.skills_by_name,
        projectile_count: input.projectile_count.unwrap_or(1),
        of_total_damage: 0.0,
        scoped: &no_scoped,
        conversion_flat: 0.0,
        conversion_skill_damage_pct: 0.0,
    };
    let hit_breakdown = calc::compute_skill_damage(&hit_input);

    let mut avg_hit_min = 0.0;
    let mut avg_hit_max = 0.0;
    if ctx.active_calc.attack_kind == Some(calc::AttackKind::Attack) {
        // Mirror of the attack branch in calc/build.rs: weapon physical +
        // elemental part, with attacks-per-second baked into the breakdown.
        let attack_input = calc::AttackSkillInput {
            skill: &ctx.active_calc,
            allocated_rank: input.active_skill_rank as f64,
            stats: &state.stats,
            item_skill_bonuses: &ctx.item_bonuses_norm,
            enemy_conditions: &input.enemy_conditions,
            weapon: ctx.weapon.as_ref(),
            poison_breakdown: hit_breakdown.as_ref(),
            scoped: &no_scoped,
            conversion_flat: 0.0,
        };
        if let Some(ad) = calc::compute_attack_skill_damage(&attack_input) {
            avg_hit_min = ad.combined_avg_min as f64 * ad.attacks_per_second_min;
            avg_hit_max = ad.combined_avg_max as f64 * ad.attacks_per_second_max;
        }
    } else if let Some(b) = hit_breakdown.as_ref() {
        let stat = |key: &str| state.stats.get(key).copied().unwrap_or((0.0, 0.0));
        // Mirror of the entity branch in calc/build.rs. The config base rate is
        // a constant scale, so 1.0 cannot reorder candidates.
        let entity_tags = ctx.active_calc.tags.as_slice();
        if crate::calc::affix_tags::is_entity(entity_tags) {
            let add = crate::calc::affix_tags::sum_for(
                crate::calc::types::AffixEffect::AttackSpeed,
                entity_tags,
                &state.stats,
            );
            let extra = crate::calc::affix_tags::sum_for(
                crate::calc::types::AffixEffect::MaxAmount,
                entity_tags,
                &state.stats,
            );
            avg_hit_min =
                b.avg_min as f64 * (1.0 + add.0 / 100.0) * (1.0 + extra.0.max(0.0));
            avg_hit_max =
                b.avg_max as f64 * (1.0 + add.1 / 100.0) * (1.0 + extra.1.max(0.0));
        } else {
            let (speed_key, base_cast) = if ctx.active.uses_attack_speed {
                ("increased_attack_speed", stat("attacks_per_second").1)
            } else {
                ("faster_cast_rate", ctx.active.base_cast_rate.unwrap_or(0.0))
            };
            if base_cast > 0.0 {
                let add = stat(speed_key);
                let more = stat(&format!("{speed_key}_more"));
                let combined_min = ((1.0 + add.0 / 100.0) * (1.0 + more.0 / 100.0) - 1.0) * 100.0;
                let combined_max = ((1.0 + add.1 / 100.0) * (1.0 + more.1 / 100.0) - 1.0) * 100.0;
                let eff_cast_min = base_cast * (1.0 + combined_min / 100.0);
                let eff_cast_max = base_cast * (1.0 + combined_max / 100.0);
                avg_hit_min = b.avg_min as f64 * eff_cast_min;
                avg_hit_max = b.avg_max as f64 * eff_cast_max;
            } else {
                avg_hit_min = b.avg_min as f64;
                avg_hit_max = b.avg_max as f64;
            }
        }
    }

    let mut proc_min = 0.0;
    let mut proc_max = 0.0;
    for proc_skill in &input.all_skills {
        let Some(proc) = proc_skill.proc.as_ref() else { continue };
        if !input.proc_toggles.get(&proc_skill.id).copied().unwrap_or(false) {
            continue;
        }
        let proc_rank = input
            .skill_ranks_by_id
            .get(&proc_skill.id)
            .copied()
            .unwrap_or(0.0);
        if proc_rank <= 0.0 {
            continue;
        }
        let target_norm = proc.target.trim().to_lowercase();
        let Some(target_id) = ctx.id_by_normalized_name.get(&target_norm) else { continue };
        let Some(target_ref) = input.all_skills.iter().find(|s| s.id == *target_id) else {
            continue;
        };
        let target_rank = input
            .skill_ranks_by_id
            .get(target_id)
            .copied()
            .unwrap_or(0.0);
        if target_rank <= 0.0 {
            continue;
        }
        let target_calc = skill_ref_to_calc(target_ref);
        let projectile_count = input
            .skill_projectiles
            .get(target_id)
            .copied()
            .unwrap_or(1);
        let target_input = calc::SkillInput {
            skill: &target_calc,
            allocated_rank: target_rank,
            attributes: &attrs_norm,
            stats: &state.stats,
            skill_ranks_by_name: &ctx.skill_ranks_norm,
            item_skill_bonuses: &ctx.item_bonuses_norm,
            enemy_conditions: &input.enemy_conditions,
            enemy_resistances: &input.enemy_resistances,
            skills_by_name: &ctx.skills_by_name,
            projectile_count,
            of_total_damage: 0.0,
            scoped: &no_scoped,
            conversion_flat: 0.0,
            conversion_skill_damage_pct: 0.0,
        };
        let Some(target_break) = calc::compute_skill_damage(&target_input) else { continue };
        let rate = if proc.trigger == "on_kill" {
            input.kills_per_sec
        } else {
            1.0
        };
        let factor = rate * (proc.chance / 100.0);
        proc_min += factor * target_break.avg_min as f64;
        proc_max += factor * target_break.avg_max as f64;
    }

    let combined_min = avg_hit_min + proc_min;
    let combined_max = avg_hit_max + proc_max;
    (combined_min + combined_max) * 0.5
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

pub fn suggest(input: &PrecomputedInput, app: Option<&AppHandle>) -> SuggestResult {
    let jewelry_set: HashSet<u32> = input.graph.jewelry_ids.iter().copied().collect();
    let valuable_set: HashSet<u32> = input.graph.valuable_ids.iter().copied().collect();
    let start_set: HashSet<u32> = input.graph.start_ids.iter().copied().collect();

    let mut allocated: HashSet<u32> = input.allocated_tree_nodes.iter().copied().collect();
    let initial = allocated.clone();

    let game_cfg = &input.game_config;

    let compute_for_alloc = |alloc: &HashSet<u32>| -> FinalState {
        let alloc_vec: Vec<u32> = alloc.iter().copied().collect();
        let inputs = EngineInputs {
            attr_contributions: &input.attr_contributions,
            stat_contributions: &input.stat_contributions,
            allocated_tree_nodes: &alloc_vec,
            tree_node_info: &input.tree_nodes,
            player_conditions: &input.player_conditions,
            jewelry_ids: &jewelry_set,
            game_config: game_cfg,
        };
        compute_final_state(&inputs)
    };

    let initial_state = compute_for_alloc(&allocated);
    let unsupported_total = initial_state.unsupported_lines.clone();
    let ctx = build_dps_context(input);
    let dps_of = |state: &FinalState| -> f64 {
        match ctx.as_ref() {
            Some(c) => compute_dps(state, input, c),
            None => 0.0,
        }
    };
    let base_dps = dps_of(&initial_state);
    let mut current_dps = base_dps;
    let mut sequence: Vec<SuggestStep> = Vec::new();

    // Drop notables that don't change DPS, else the path-distance tie-breaker picks
    // irrelevant ones. Jewelry sockets pass anyway — a jewel may be slotted later.
    let mut valuable_with_impact: HashSet<u32> = HashSet::new();
    for &v in &valuable_set {
        if allocated.contains(&v) {
            continue;
        }
        if jewelry_set.contains(&v) {
            valuable_with_impact.insert(v);
            continue;
        }
        let mut probe = allocated.clone();
        probe.insert(v);
        let state = compute_for_alloc(&probe);
        let dps = dps_of(&state);
        if dps > base_dps + 1e-6 {
            valuable_with_impact.insert(v);
        }
    }

    let mut remaining_budget = input.budget;
    loop {
        if let Some(app) = app {
            let _ = app.emit(
                "suggest-progress",
                ProgressPayload {
                    current: sequence.len() as u32,
                    total: input.budget,
                },
            );
        }
        if remaining_budget == 0 {
            break;
        }

        // Score targets by gain-per-node along their cheapest path so a notable
        // 5 filler hops away beats a locally-best 1-hop pick.
        let mut best_target: Option<u32> = None;
        let mut best_score = f64::NEG_INFINITY;
        let mut best_path: Vec<u32> = Vec::new();
        let mut best_final_dps = current_dps;

        for &target in &valuable_with_impact {
            if allocated.contains(&target) {
                continue;
            }
            let Some(path) = find_path_to(&allocated, &start_set, target, &input.graph)
            else { continue };
            if path.is_empty() || (path.len() as u32) > remaining_budget {
                continue;
            }
            let mut probe = allocated.clone();
            for p in &path {
                probe.insert(*p);
            }
            let state = compute_for_alloc(&probe);
            let dps = dps_of(&state);
            let gain = dps - current_dps;
            if gain <= 1e-6 {
                continue;
            }
            let score = gain / (path.len() as f64);
            if score > best_score + 1e-9 {
                best_target = Some(target);
                best_score = score;
                best_path = path;
                best_final_dps = dps;
            }
        }

        let Some(target) = best_target else { break };

        // Walk path one node at a time so filler steps show their tiny per-step gain.
        let mut step_dps = current_dps;
        let mut step_alloc = allocated.clone();
        for &node in &best_path {
            step_alloc.insert(node);
            let s = compute_for_alloc(&step_alloc);
            let d = dps_of(&s);
            let g = d - step_dps;
            sequence.push(SuggestStep {
                node_id: node,
                dps_before: step_dps,
                dps_after: d,
                gain: g,
                is_filler: g <= 1e-6,
            });
            step_dps = d;
        }
        for &node in &best_path {
            allocated.insert(node);
            valuable_with_impact.remove(&node);
        }
        valuable_with_impact.remove(&target);
        remaining_budget = remaining_budget.saturating_sub(best_path.len() as u32);
        current_dps = best_final_dps;
    }

    // ====================== BUDGET TOP-UP ======================
    // The main loop only targets notables, stranding budget once none is reachable
    // with a gain; spend the rest greedily on frontier nodes that still raise DPS.
    while remaining_budget > 0 {
        let mut frontier: HashSet<u32> =
            start_set.difference(&allocated).copied().collect();
        for id in &allocated {
            if let Some(nbrs) = input.graph.adjacency.get(id) {
                for &nb in nbrs {
                    if !allocated.contains(&nb) {
                        frontier.insert(nb);
                    }
                }
            }
        }

        let mut best: Option<(u32, f64)> = None;
        for &cand in &frontier {
            let mut probe = allocated.clone();
            probe.insert(cand);
            let state = compute_for_alloc(&probe);
            let dps = dps_of(&state);
            if dps > current_dps + 1e-6 && best.is_none_or(|(_, b)| dps > b) {
                best = Some((cand, dps));
            }
        }
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
        if let Some(app) = app {
            let _ = app.emit(
                "suggest-progress",
                ProgressPayload {
                    current: sequence.len() as u32,
                    total: input.budget,
                },
            );
        }
    }

    // ====================== LOCAL SEARCH SWAP REFINEMENT ======================
    // 2-opt: swap any allocated node for a frontier neighbour while DPS improves.
    const SWAP_MAX_PASSES: u32 = 60;
    for pass in 0..SWAP_MAX_PASSES {
        if let Some(app) = app {
            let _ = app.emit(
                "suggest-progress",
                ProgressPayload {
                    current: sequence.len() as u32 + pass,
                    total: sequence.len() as u32 + SWAP_MAX_PASSES,
                },
            );
        }

        let removable: Vec<u32> = allocated.difference(&initial).copied().collect();
        if removable.is_empty() {
            break;
        }

        let mut best_swap: Option<(u32, u32, f64)> = None;

        for &rm in &removable {
            let mut without = allocated.clone();
            without.remove(&rm);

            // Removing `rm` must not detach the rest from a PAID start —
            // unpurchased starts are not free transit, they cost budget.
            let reachable = reachable_from_starts(&start_set, &without, &input.graph);
            if !without.iter().all(|n| reachable.contains(n)) {
                continue;
            }

            // Candidates: unpurchased starts, or neighbours of the remaining
            // allocation. Warps stay in — they are valid transit nodes.
            let mut frontier: HashSet<u32> =
                start_set.difference(&without).copied().collect();
            for id in &without {
                if let Some(nbrs) = input.graph.adjacency.get(id) {
                    for &nb in nbrs {
                        if !without.contains(&nb) {
                            frontier.insert(nb);
                        }
                    }
                }
            }

            for &add in &frontier {
                if add == rm || allocated.contains(&add) {
                    continue;
                }
                let mut new_alloc = without.clone();
                new_alloc.insert(add);
                let state = compute_for_alloc(&new_alloc);
                let dps = dps_of(&state);
                if dps > current_dps + 1e-6 {
                    let gain = dps - current_dps;
                    match best_swap.as_ref() {
                        None => best_swap = Some((rm, add, dps)),
                        Some(prev) if gain > prev.2 - current_dps => {
                            best_swap = Some((rm, add, dps))
                        }
                        _ => {}
                    }
                }
            }
        }

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

    if let Some(app) = app {
        let _ = app.emit(
            "suggest-progress",
            ProgressPayload {
                current: sequence.len() as u32,
                total: input.budget,
            },
        );
    }
    let added_nodes: Vec<u32> = allocated.difference(&initial).copied().collect();
    let budget_used = sequence.len() as u32;
    let used_starts: Vec<u32> = allocated.intersection(&start_set).copied().collect();
    SuggestResult {
        added_nodes,
        sequence,
        base_dps,
        final_dps: current_dps,
        budget_used,
        budget_requested: input.budget,
        unsupported_lines: unsupported_total,
        used_starts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{DamageFormula, TreeNodeInfo};

    /// Chain graph 0-1-2-...-(n-1); node 0 is the start.
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

    fn node(kind: &str, line: &str) -> TreeNodeInfo {
        TreeNodeInfo {
            title: String::new(),
            kind: kind.to_string(),
            lines: vec![line.to_string()],
        }
    }

    /// Active skill dealing 100 base damage, +1% per point of strength.
    fn strength_skill() -> SkillRef {
        SkillRef {
            id: "s1".to_string(),
            name: "Test Skill".to_string(),
            damage_formula: Some(DamageFormula { base: 100.0, per_level: 0.0 }),
            bonus_sources: vec![BonusSource::AttributePoint {
                source: "strength".to_string(),
                value: 1.0,
            }],
            ..Default::default()
        }
    }

    fn base_input(graph: TreeGraph, nodes: HashMap<u32, TreeNodeInfo>, budget: u32) -> PrecomputedInput {
        let skill = strength_skill();
        PrecomputedInput {
            graph,
            tree_nodes: nodes,
            active_skill: Some(skill.clone()),
            active_skill_rank: 1,
            all_skills: vec![skill],
            budget,
            ..Default::default()
        }
    }

    #[test]
    fn attack_skill_gets_nonzero_dps_and_enhanced_damage_fillers() {
        let nodes: HashMap<u32, TreeNodeInfo> = (0..5)
            .map(|i| (i, node("minor", "+10% to Enhanced Damage")))
            .collect();
        let skill = SkillRef {
            id: "a1".to_string(),
            name: "Heavy Swing".to_string(),
            attack_kind: Some("attack".to_string()),
            attack_scaling: Some(super::super::types::AttackScalingRef {
                weapon_damage_pct: Some(DamageFormula { base: 100.0, per_level: 0.0 }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut inventory: crate::calc::types::Inventory = HashMap::new();
        inventory.insert(
            "weapon".to_string(),
            crate::calc::types::EquippedItem {
                base_id: "base_mace_ogre_maul".to_string(),
                ..Default::default()
            },
        );
        let mut input = PrecomputedInput {
            graph: chain_graph(5),
            tree_nodes: nodes,
            active_skill: Some(skill.clone()),
            active_skill_rank: 1,
            all_skills: vec![skill],
            budget: 4,
            inventory,
            ..Default::default()
        };
        input
            .stat_contributions
            .insert("attacks_per_second".to_string(), vec![(1.0, 1.0)]);

        let result = suggest(&input, None);

        assert!(
            result.base_dps > 0.0,
            "unarmed attack skill must have baseline dps (got {})",
            result.base_dps
        );
        assert_eq!(result.budget_used, 4, "enhanced damage nodes are dps gains for attacks");
        assert!(result.final_dps > result.base_dps);
    }

    #[test]
    fn spends_full_budget_on_dps_fillers_without_notables() {
        let nodes: HashMap<u32, TreeNodeInfo> =
            (0..5).map(|i| (i, node("minor", "+5 to Strength"))).collect();
        let input = base_input(chain_graph(5), nodes, 4);

        let result = suggest(&input, None);

        assert_eq!(result.budget_used, 4, "every point buys +5 str = +5% dps");
        assert_eq!(result.added_nodes.len(), 4);
        assert!(result.final_dps > result.base_dps);
    }

    #[test]
    fn spends_leftover_budget_after_notables_exhausted() {
        let mut nodes: HashMap<u32, TreeNodeInfo> =
            (0..6).map(|i| (i, node("minor", "+5 to Strength"))).collect();
        nodes.insert(2, node("big", "+50 to Strength"));
        let mut graph = chain_graph(6);
        graph.valuable_ids = vec![2];
        let input = base_input(graph, nodes, 5);

        let result = suggest(&input, None);

        // Path to the notable costs 3 (nodes 0,1,2); the remaining 2 points
        // must go into +5 str fillers instead of being dropped.
        assert_eq!(result.budget_used, 5);
        assert_eq!(result.added_nodes.len(), 5);
    }

    #[test]
    fn swap_never_adds_nodes_detached_from_purchased_starts() {
        // Start 10 guards a huge filler (11) behind a dead node: the swap pass must
        // not trade a chain node for it, since reaching it means paying for start 10.
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
        let mut nodes: HashMap<u32, TreeNodeInfo> = HashMap::new();
        for i in [0u32, 1, 2] {
            nodes.insert(i, node("minor", "+5 to Strength"));
        }
        nodes.insert(10, TreeNodeInfo { title: String::new(), kind: "minor".to_string(), lines: vec![] });
        nodes.insert(11, node("minor", "+100 to Strength"));
        let input = base_input(graph, nodes, 3);

        let result = suggest(&input, None);

        // Every suggested node must be reachable from a start the result pays for.
        let alloc: HashSet<u32> = result.added_nodes.iter().copied().collect();
        let starts: HashSet<u32> = result
            .used_starts
            .iter()
            .copied()
            .filter(|s| alloc.contains(s))
            .collect();
        let reachable = reachable_from_starts(&starts, &alloc, &input.graph);
        for n in &alloc {
            assert!(
                reachable.contains(n),
                "node {n} is detached from purchased starts: {:?}",
                result.added_nodes
            );
        }
    }

    #[test]
    fn keeps_budget_when_no_node_improves_dps() {
        // Life nodes don't feed the damage formula: spending on them is waste.
        let nodes: HashMap<u32, TreeNodeInfo> =
            (0..5).map(|i| (i, node("minor", "+5 to Maximum Life"))).collect();
        let input = base_input(chain_graph(5), nodes, 4);

        let result = suggest(&input, None);

        assert_eq!(result.budget_used, 0);
        assert!(result.added_nodes.is_empty());
    }
}
