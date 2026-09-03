use super::*;

static NO_STACKS: Lazy<HashMap<String, u32>> = Lazy::new(HashMap::new);
static NO_RATES: Lazy<HashMap<String, f64>> = Lazy::new(HashMap::new);

fn contrib(value: Ranged) -> SourceContribution {
    SourceContribution {
        label: "test".to_string(),
        source_type: SourceType::Item,
        value,
        forge: None,
    }
}

#[allow(clippy::too_many_arguments)] // mirrors BuildStatsInput fields
fn empty_input<'a>(
    allocated: &'a HashMap<String, u32>,
    inventory: &'a Inventory,
    skill_ranks: &'a HashMap<String, u32>,
    active_buffs: &'a HashMap<String, bool>,
    custom_stats: &'a [CustomStat],
    allocated_tree_nodes: &'a HashSet<u32>,
    tree_socketed: &'a HashMap<u32, TreeSocketContent>,
    player_conditions: &'a HashMap<String, bool>,
    subskill_ranks: &'a HashMap<String, u32>,
    enemy_conditions: &'a HashMap<String, bool>,
) -> BuildStatsInput<'a> {
    BuildStatsInput {
        class_id: None,
        level: 1,
        allocated_attrs: allocated,
        inventory,
        skill_ranks,
        active_aura_id: None,
        active_buffs,
        custom_stats,
        allocated_tree_nodes,
        tree_socketed,
        player_conditions,
        subskill_ranks,
        enemy_conditions,
        stack_counts: &NO_STACKS,
        entity_rates: &NO_RATES,
        granted_skill_ranks: None,
        main_skill_id: None,
        difficulty: None,
    }
}

mod engine;
mod orchestrator;
mod pipeline;
mod units;
