// Stat aggregation: source-tracking parity with TS computeBuildStats in
// src/utils/stats.ts.

use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;
use serde::Serialize;

use super::affix::{
    apply_stars_to_ranged_value, rolled_affix_range, rolled_affix_value, rolled_affix_value_with_stars,
};
use super::custom_stat::parse_custom_stat_value;
use super::data::{self, ForgeKind};
use super::rank::{aggregate_item_skill_bonuses, normalize_skill_name, rank_bonus_for};
use super::skills::Ranged;
use super::tree::parse::{
    DisableTarget, ParsedConversion, ParsedMeta, parse_tree_node_meta, parse_tree_node_mod,
};
use super::types::{CustomStat, Inventory, SkillKind, SocketType, StatDef, TreeSocketContent};

pub const RAINBOW_MULTIPLIER: f64 = 1.5;

// ---------- top-level types ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Class,
    Allocated,
    Level,
    Attribute,
    Item,
    Socket,
    Skill,
    Subskill,
    Custom,
    Tree,
}

pub const CUSTOM_SOURCE_LABEL: &str = "Custom Config";

// Stats whose total fans out to per-element variants
// (e.g. `all_resistances` → each elemental resistance bucket).
pub const STAT_FAN_OUTS: &[(&str, &[&str])] = &[
    (
        "all_resistances",
        &[
            "fire_resistance",
            "cold_resistance",
            "lightning_resistance",
            "poison_resistance",
            "arcane_resistance",
        ],
    ),
    (
        "max_all_resistances",
        &[
            "max_fire_resistance",
            "max_cold_resistance",
            "max_lightning_resistance",
            "max_poison_resistance",
            "max_arcane_resistance",
        ],
    ),
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forge {
    pub item_name: String,
    pub mod_name: String,
    pub kind: ForgeKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceContribution {
    pub label: String,
    pub source_type: SourceType,
    pub value: Ranged,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forge: Option<Forge>,
}

pub type SourceMap = HashMap<String, Vec<SourceContribution>>;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedStats {
    pub attributes: HashMap<String, Ranged>,
    pub stats: HashMap<String, Ranged>,
    pub attribute_sources: SourceMap,
    pub stat_sources: SourceMap,
    pub stats_combined: HashMap<String, Ranged>,
    /// Pre-diminishing-returns totals, only for keys the curve actually reduced.
    pub diminished_raw: HashMap<String, Ranged>,
    pub item_skill_bonuses: HashMap<String, Ranged>,
    /// Ranks of item-granted skills (player inventory + external extras like
    /// merc auras) — the same map the passive/conversion passes consume.
    pub item_granted_ranks: HashMap<String, Ranged>,
    pub rank_bonuses: HashMap<String, Ranged>,
    /// Per skill id: that skill's own subtree values which must never enter
    /// the shared stat map (`projectile_count`, `of_total_damage`, …).
    pub skill_scoped: HashMap<String, HashMap<String, Ranged>>,
    /// Per skill id: keys where that skill's view differs from `stats`; spread over
    /// `stats` to get the map a side skill should be calculated with.
    pub skill_stat_overrides: HashMap<String, HashMap<String, Ranged>>,
}

/// One skill's subtree aggregate, split by where each key is allowed to go.
#[derive(Debug, Clone, Default)]
pub struct SubtreeAgg {
    pub shared: HashMap<String, Ranged>,
    pub scoped: HashMap<String, Ranged>,
}


mod attributes;
mod breakdown;
mod finalize;
mod helpers;
mod inventory;
mod sets;
mod skills;
mod stacks;
mod tree;

pub use attributes::*;
pub use breakdown::*;
pub use finalize::*;
pub use helpers::*;
pub use inventory::*;
pub use sets::*;
pub use skills::*;
pub use stacks::*;
pub use tree::*;

// ---------- orchestrator ----------

#[derive(Debug, Clone, Copy)]
pub struct BuildStatsInput<'a> {
    pub class_id: Option<&'a str>,
    pub level: u32,
    pub allocated_attrs: &'a HashMap<String, u32>,
    pub inventory: &'a Inventory,
    pub skill_ranks: &'a HashMap<String, u32>,
    pub active_aura_id: Option<&'a str>,
    pub active_buffs: &'a HashMap<String, bool>,
    pub custom_stats: &'a [CustomStat],
    pub allocated_tree_nodes: &'a HashSet<u32>,
    pub tree_socketed: &'a HashMap<u32, TreeSocketContent>,
    pub player_conditions: &'a HashMap<String, bool>,
    pub subskill_ranks: &'a HashMap<String, u32>,
    pub enemy_conditions: &'a HashMap<String, bool>,
    /// Active in-combat stacks per stack type; absent means "at cap".
    pub stack_counts: &'a HashMap<String, u32>,
    pub granted_skill_ranks: Option<&'a HashMap<String, Ranged>>,
    /// Scopes subskill aggregation: only this skill's subtree feeds the
    /// shared stat map.
    pub main_skill_id: Option<&'a str>,
}

// A few bases carry their own long type names; tree gates say "Throwing"/"Gun".
fn weapon_kind_of(base: &crate::calc::types::ItemBase) -> String {
    match base.base_type.as_str() {
        "1-Handed Throwing Weapon" => "Throwing".to_string(),
        "Rifle Gun" => "Gun".to_string(),
        _ => base.base_type.clone(),
    }
}

// Single pass of the full stat-aggregation pipeline; compute_build_stats
// wraps it with an automatic crit-below-40 re-run when needed.
pub fn compute_build_stats_core(input: &BuildStatsInput) -> ComputedStats {
    let mut attr_sources: SourceMap = HashMap::new();
    let mut stat_sources: SourceMap = HashMap::new();

    // 1. Base attributes (defaults + class base + allocated)
    apply_base_attributes(input.class_id, input.allocated_attrs, &mut attr_sources);

    // 2. Inventory loop (implicits + affixes + sockets + runeword + augment)
    let weapon_has_aps =
        apply_inventory(input.inventory, &mut attr_sources, &mut stat_sources);

    // 3. Tree contributions (returns deferred conversions + disables);
    //    the incarnation tree's node set is season-patched data.
    let tree_agg = apply_tree_contributions(
        input.allocated_tree_nodes,
        input.player_conditions,
        &mut attr_sources,
        &mut stat_sources,
    );

    // 4. Tree jewelry sockets
    apply_tree_jewelry_sockets(
        input.allocated_tree_nodes,
        input.tree_socketed,
        &mut attr_sources,
        &mut stat_sources,
    );

    // 5. Set bonuses
    apply_set_bonuses(input.inventory, &mut attr_sources, &mut stat_sources);

    // 6. Default/class base stats + per-level
    apply_class_baseline(input.class_id, input.level, weapon_has_aps, &mut stat_sources);

    // 6b. Item mana/life "Based on Level" scaled by character level.
    apply_stats_based_on_level(input.level, &mut attr_sources, &mut stat_sources);

    // 7. Custom user-defined stats
    apply_custom_stats(input.custom_stats, &mut attr_sources, &mut stat_sources);

    // 7b. Weapon-type conditional stats: tree lines gated on a weapon kind
    // fold into their real stats only while a matching base is equipped.
    let weapon_base = input
        .inventory
        .get("weapon")
        .and_then(|item| data::get_item(&item.base_id));
    let kind = weapon_base.map(weapon_kind_of).unwrap_or_default();
    let two_handed = weapon_base.is_some_and(|base| base.two_handed == Some(true));
    let two_handed_melee = two_handed
        && matches!(
            kind.as_str(),
            "Axe" | "Sword" | "Mace" | "Polearm" | "Chainsaw" | "Claw" | "Dagger" | "Spellblade"
        );
    let has_shield = input
        .inventory
        .get("offhand")
        .and_then(|item| data::get_item(&item.base_id))
        .is_some_and(|base| base.base_type == "Shield");
    let weapon_folds = [
        (kind == "Dagger", "physical_damage_with_dagger", "additive_physical_damage", "While wielding a Dagger"),
        (kind == "Dagger", "enhanced_damage_with_dagger", "enhanced_damage", "While wielding a Dagger"),
        (kind == "Wand", "faster_cast_rate_with_wand", "faster_cast_rate", "While wielding a Wand"),
        (kind == "Wand", "faster_cast_rate_more_with_wand", "faster_cast_rate_more", "While wielding a Wand"),
        (kind == "Wand", "magic_skill_damage_with_wand", "magic_skill_damage", "While wielding a Wand"),
        (kind == "Axe", "damage_with_axe", "enhanced_damage", "While wielding an Axe"),
        (kind == "Axe", "attack_rating_with_axe_pct", "attack_rating_pct", "While wielding an Axe"),
        (kind == "Staff" || kind == "Cane", "two_handed_spell_projectile_damage", "spell_projectile_damage", "While wielding a Staff or Cane"),
        (has_shield, "attack_damage_with_shield", "attack_damage", "While using a Shield"),
        (has_shield, "vitality_with_shield_flat", "to_vitality", "While using a Shield"),
        (has_shield, "vitality_with_shield", "increased_vitality", "While using a Shield"),
        (has_shield, "melee_range_with_shield", "melee_range", "While using a Shield"),
        (has_shield, "damage_mitigation_with_shield", "damage_mitigation", "While using a Shield"),
        (has_shield, "crit_damage_more_with_shield", "crit_damage_more", "While using a Shield"),
        (has_shield, "damage_return_with_shield", "damage_return", "While using a Shield"),
        (two_handed, "damage_with_two_handed", "enhanced_damage", "While using a Two Handed Weapon"),
        (two_handed, "ailment_damage_all_with_two_handed", "ailment_damage_all", "While using a Two Handed Weapon"),
        (two_handed, "ailment_damage_all_more_with_two_handed", "ailment_damage_all_more", "While using a Two Handed Weapon"),
        (two_handed, "increased_ailment_frequency_with_two_handed", "increased_ailment_frequency", "While using a Two Handed Weapon"),
        (two_handed_melee, "damage_with_two_handed_melee", "enhanced_damage", "While using a Two Handed Melee Weapon"),
        (kind == "Bow", "damage_with_bow", "ranged_projectile_damage", "While using a Bow"),
        (kind == "Bow", "enhanced_damage_with_bow", "enhanced_damage", "While using a Bow"),
        (kind == "Gun", "damage_with_gun", "ranged_projectile_damage", "While using a Gun"),
        (kind == "Gun", "enhanced_damage_with_gun", "enhanced_damage", "While using a Gun"),
        (kind == "Throwing", "damage_with_throwing", "ranged_projectile_damage", "While using a Throwing Weapon"),
        (kind == "Throwing", "enhanced_damage_with_throwing", "enhanced_damage", "While using a Throwing Weapon"),
    ];
    for (enabled, cond_key, target_key, label) in weapon_folds {
        if !enabled {
            continue;
        }
        let sum = sum_ranged_from_map(&stat_sources, cond_key);
        if sum != (0.0, 0.0) {
            apply_contribution(
                &mut attr_sources,
                &mut stat_sources,
                target_key,
                sum,
                label.to_string(),
                SourceType::Tree,
                None,
            );
        }
    }

    // 7c. Dual wielding: unlocked by incarnation notes (Master of Wands,
    // Hercules Grip), so a weapon in the offhand slot is the only signal here.
    let is_dual_wielding = input
        .inventory
        .get("offhand")
        .and_then(|item| data::get_item(&item.base_id))
        .is_some_and(|base| base.slot == "weapon");
    if is_dual_wielding {
        for (cond_key, target_key) in [
            ("damage_dual_wield", "enhanced_damage"),
            ("damage_dual_wield_more", "enhanced_damage_more"),
        ] {
            let sum = sum_ranged_from_map(&stat_sources, cond_key);
            if sum != (0.0, 0.0) {
                apply_contribution(
                    &mut attr_sources,
                    &mut stat_sources,
                    target_key,
                    sum,
                    "While Dual Wielding".to_string(),
                    SourceType::Tree,
                    None,
                );
            }
        }
    }

    // 7d. In-combat stacks (Rage): count × per-stack effects.
    apply_stack_effects(input.stack_counts, &mut attr_sources, &mut stat_sources);

    // 8. Skill ranks → passive stats
    apply_skill_ranks(
        input.class_id,
        input.skill_ranks,
        input.active_aura_id,
        input.active_buffs,
        input.inventory,
        &mut attr_sources,
        &mut stat_sources,
    );

    // 9. Increased all attributes % (applies to each attribute's flat sum)
    apply_increased_all_attributes(&mut attr_sources, &stat_sources);

    // 10. Increased per-attribute additive + more (compound delta)
    apply_increased_per_attribute(&mut attr_sources, &stat_sources);

    // 11. Stats per attribute (e.g. strength → enhanced_damage)
    apply_stats_per_attribute(input.class_id, &attr_sources, &mut stat_sources);

    // 12. Subskill aggregation (gates skill-scoped stats out)
    let subtree = apply_subskill_aggregation(
        input.class_id,
        input.main_skill_id,
        input.subskill_ranks,
        Some(input.enemy_conditions),
        &mut attr_sources,
        &mut stat_sources,
    );

    // 13. Compute attribute totals
    let mut attributes = compute_final_attributes(&attr_sources);

    // 14. Attribute-divided stats (e.g. vitality/8 → life_replenish)
    apply_attribute_divided_stats(&attributes, &mut stat_sources);

    // 15. Item-granted skill bonuses → passive stats; ranks reused in step 19.
    let item_granted_ranks = apply_item_granted_passive_stats(
        input.inventory,
        input.granted_skill_ranks,
        input.player_conditions,
        &mut attr_sources,
        &mut stat_sources,
    );

    // 15b. Class-scoped "+X to All Skills (Class)" folds into all_skills.
    apply_class_scoped_all_skills(input.class_id, &mut stat_sources);

    // 16. Stat fan-outs (all_resistances → per-element variants)
    apply_stat_fan_outs(&mut stat_sources);

    // 16b. Damage from Resistances → enhanced_damage (needs fanned-out resists).
    apply_damage_per_resist(&mut stat_sources);

    // 17. Compute stat totals from sources
    let mut stats = compute_final_stats(&stat_sources);

    // 18. Multiplier pass (life/mana/replenishes)
    apply_multipliers_pass(&mut stats);

    // 19. Item-granted skill conversions
    let touched_item = apply_item_granted_conversions(
        &item_granted_ranks,
        &stats,
        input.player_conditions,
        &mut stat_sources,
    );

    // 20. Tree conversions (can target attributes or stats)
    let touched_tree = apply_tree_conversions(
        &tree_agg.conversions,
        &mut attributes,
        &stats,
        &mut attr_sources,
        &mut stat_sources,
    );

    // 21. Re-sum touched stat keys after conversions injected new sources.
    for k in touched_item.iter().chain(touched_tree.iter()) {
        if let Some(list) = stat_sources.get(k) {
            stats.insert(k.clone(), sum_contributions(list));
        }
    }

    // 22. Tree disables (zero out life_replenish if flagged)
    apply_tree_disables(&tree_agg.disables, &mut stats);

    // 22b. Diminishing returns on final totals (config-driven, season-aware).
    let diminished_raw = match data::game_config().diminishing_returns.as_ref() {
        Some(defs) => apply_diminishing_returns(&mut stats, defs),
        None => HashMap::new(),
    };

    // 23. UI-facing derivations so views read engine output: combined `_more`
    // totals, item skill bonuses, per-skill rank bonuses.
    let stats_combined = stats_combined_map(&stats);
    let item_skill_bonuses = aggregate_item_skill_bonuses(input.inventory, &data::data().items);
    let rank_bonuses: HashMap<String, Ranged> = match input.class_id {
        Some(cid) => data::get_skills_by_class(cid)
            .iter()
            .map(|s| {
                let tags = crate::calc::subskill::effective_skill_tags(
                    &s.id,
                    s.tags.as_deref().unwrap_or(&[]),
                    input.subskill_ranks,
                );
                (
                    normalize_skill_name(&s.name),
                    rank_bonus_for(
                        &s.name,
                        s.damage_type.as_deref(),
                        &tags,
                        &stats,
                        &item_skill_bonuses,
                    ),
                )
            })
            .collect(),
        None => HashMap::new(),
    };

    let skill_stat_overrides =
        per_skill_stat_overrides(input.class_id, input.main_skill_id, &subtree, &stats);
    let skill_scoped: HashMap<String, HashMap<String, Ranged>> = subtree
        .into_iter()
        .filter(|(_, agg)| !agg.scoped.is_empty())
        .map(|(id, agg)| (id, agg.scoped))
        .collect();

    ComputedStats {
        attributes,
        stats,
        attribute_sources: attr_sources,
        stat_sources,
        stats_combined,
        diminished_raw,
        item_skill_bonuses,
        item_granted_ranks,
        rank_bonuses,
        skill_scoped,
        skill_stat_overrides,
    }
}

// Re-runs the pipeline with `crit_chance_below_40` flipped on when the
// baseline crit < 40% and tree nodes are allocated.
pub fn compute_build_stats(input: &BuildStatsInput) -> ComputedStats {
    let baseline = compute_build_stats_core(input);
    let already_set = input
        .player_conditions
        .get("crit_chance_below_40")
        .copied()
        .unwrap_or(false);
    if !already_set && !input.allocated_tree_nodes.is_empty() {
        let crit = baseline
            .stats
            .get("crit_chance")
            .copied()
            .unwrap_or((0.0, 0.0));
        if crit.0 < 40.0 {
            let mut conds = input.player_conditions.clone();
            conds.insert("crit_chance_below_40".to_string(), true);
            let new_input = BuildStatsInput {
                player_conditions: &conds,
                ..*input
            };
            return compute_build_stats_core(&new_input);
        }
    }
    baseline
}


#[cfg(test)]
mod tests;
