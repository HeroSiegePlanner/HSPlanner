use std::collections::HashMap;

use hsplanner_engine::calc::{
    commands::BuildPerformanceInput,
    data,
    planner::PlannerInput,
    types::{CustomStat, Inventory, TreeSocketContent},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BuildSnapshot {
    pub class_id: Option<String>,
    pub level: u32,
    pub difficulty: String,
    pub allocated: HashMap<String, u32>,
    pub inventory: Inventory,
    pub skill_ranks: HashMap<String, u32>,
    pub subskill_ranks: HashMap<String, u32>,
    #[serde(deserialize_with = "deserialize_max_subskill_points")]
    pub max_subskill_points: u32,
    pub allocated_tree_nodes: Vec<u32>,
    pub tree_socketed: HashMap<u32, TreeSocketContent>,
    pub active_skill_ids: Vec<String>,
    pub active_aura_id: Option<String>,
    pub active_buffs: HashMap<String, bool>,
    pub enemy_conditions: HashMap<String, bool>,
    pub player_conditions: HashMap<String, bool>,
    pub skill_projectiles: HashMap<String, u32>,
    pub enemy_resistances: HashMap<String, f64>,
    pub proc_toggles: HashMap<String, bool>,
    pub disabled_potions: HashMap<String, bool>,
    pub kills_per_sec: f64,
    pub entity_rates: HashMap<String, f64>,
    pub stack_counts: HashMap<String, u32>,
    pub custom_stats: Vec<CustomStat>,
    pub allocated_ether_nodes: Vec<u32>,
    pub merc_class_id: Option<String>,
    pub merc_skill_ranks: HashMap<String, u32>,
    pub merc_inventory: Inventory,
    pub merc_disabled_auras: HashMap<String, bool>,
    pub season: String,
}

impl Default for BuildSnapshot {
    fn default() -> Self {
        Self {
            class_id: Some("viking".into()),
            level: 1,
            difficulty: "normal".into(),
            allocated: HashMap::new(),
            inventory: HashMap::new(),
            skill_ranks: HashMap::new(),
            subskill_ranks: HashMap::new(),
            max_subskill_points: 20,
            allocated_tree_nodes: Vec::new(),
            tree_socketed: HashMap::new(),
            active_skill_ids: Vec::new(),
            active_aura_id: None,
            active_buffs: HashMap::new(),
            enemy_conditions: HashMap::new(),
            player_conditions: HashMap::new(),
            skill_projectiles: HashMap::new(),
            enemy_resistances: ["fire", "cold", "lightning", "poison", "arcane"]
                .into_iter()
                .map(|k| (k.into(), 85.))
                .collect(),
            proc_toggles: HashMap::new(),
            disabled_potions: HashMap::new(),
            kills_per_sec: 1.,
            entity_rates: ["sentry", "summon", "guardian"]
                .into_iter()
                .map(|k| (k.into(), 1.))
                .collect(),
            stack_counts: HashMap::new(),
            custom_stats: Vec::new(),
            allocated_ether_nodes: Vec::new(),
            merc_class_id: None,
            merc_skill_ranks: HashMap::new(),
            merc_inventory: HashMap::new(),
            merc_disabled_auras: HashMap::new(),
            season: "s10".into(),
        }
    }
}

fn deserialize_max_subskill_points<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<u32, D::Error> {
    u32::deserialize(deserializer).map(|points| points.clamp(1, 30))
}

impl BuildSnapshot {
    pub fn subskill_point_budget(&self) -> u32 {
        self.max_subskill_points.clamp(1, 30)
    }

    pub fn set_max_subskill_points(&mut self, points: u32) {
        self.max_subskill_points = points.clamp(1, 30);
    }

    pub fn planner_input(&self) -> PlannerInput {
        PlannerInput {
            build: BuildPerformanceInput {
                class_id: self.class_id.clone(),
                level: self.level,
                allocated_attrs: self.allocated.clone(),
                inventory: self.inventory.clone(),
                skill_ranks: self.skill_ranks.clone(),
                subskill_ranks: self.subskill_ranks.clone(),
                active_aura_id: self.active_aura_id.clone(),
                active_buffs: self.active_buffs.clone(),
                custom_stats: self.custom_stats.clone(),
                allocated_tree_nodes: self.allocated_tree_nodes.iter().copied().collect(),
                tree_socketed: self.tree_socketed.clone(),
                main_skill_id: self.active_skill_ids.first().cloned(),
                enemy_conditions: self.enemy_conditions.clone(),
                player_conditions: self.player_conditions.clone(),
                skill_projectiles: self.skill_projectiles.clone(),
                enemy_resistances: self.enemy_resistances.clone(),
                proc_toggles: self.proc_toggles.clone(),
                kills_per_sec: self.kills_per_sec,
                entity_rates: self.entity_rates.clone(),
                stack_counts: self.stack_counts.clone(),
                season: Some(self.season.clone()),
                granted_skill_ranks: HashMap::new(),
                difficulty: Some(self.difficulty.clone()),
            },
            active_skill_ids: self.active_skill_ids.clone(),
            disabled_potions: self.disabled_potions.clone(),
            merc_inventory: self.merc_inventory.clone(),
            merc_disabled_auras: self.merc_disabled_auras.clone(),
            allocated_ether_nodes: self.allocated_ether_nodes.clone(),
        }
    }

    pub fn set_tree_nodes(&mut self, nodes: &[u32]) {
        let wanted: std::collections::HashSet<_> = nodes.iter().copied().collect();
        self.allocated_tree_nodes.retain(|id| wanted.contains(id));
        for id in nodes {
            if !self.allocated_tree_nodes.contains(id) {
                self.allocated_tree_nodes.push(*id);
            }
        }
        self.validate_offhand();
    }

    pub fn can_offhand(&self, base: &hsplanner_engine::calc::types::ItemBase) -> bool {
        let main = self
            .inventory
            .get("weapon")
            .and_then(|item| data::get_item(&item.base_id));
        let notes = self
            .allocated_tree_nodes
            .iter()
            .filter_map(|id| data::get_tree_node(*id))
            .flat_map(|node| node.lines.iter().chain(node.note.iter()))
            .map(|s| s.to_lowercase())
            .collect::<Vec<_>>();
        let grip = notes.iter().any(|s| {
            s.contains("dual wield")
                && (s.contains("melee weapons") || s.contains("swords, maces and axes"))
        });
        let grip_type = |item: &hsplanner_engine::calc::types::ItemBase| {
            ["Sword", "Mace", "Axe", "Polearm", "Claw"].contains(&item.base_type.as_str())
        };
        if base.two_handed.unwrap_or(false) || main.is_some_and(|b| b.two_handed.unwrap_or(false)) {
            return base.slot == "weapon" && grip && main.is_none_or(grip_type) && grip_type(base);
        }
        base.slot == "offhand"
            || (base.slot == "weapon"
                && (base.base_type != "Wand"
                    || notes.iter().any(|s| {
                        s.contains("dual wield wands") || s.contains("dual wielding wands")
                    })))
    }

    pub fn validate_offhand(&mut self) {
        if self
            .inventory
            .get("offhand")
            .and_then(|item| data::get_item(&item.base_id))
            .is_some_and(|base| !self.can_offhand(base))
        {
            self.inventory.remove("offhand");
        }
    }

    pub fn set_class(&mut self, id: &str) {
        if self.class_id.as_deref() == Some(id) || data::get_class(id).is_none() {
            return;
        }
        self.class_id = Some(id.into());
        self.allocated.clear();
        self.skill_ranks.clear();
        self.active_skill_ids.clear();
        self.active_aura_id = None;
        self.proc_toggles.clear();
        self.active_buffs.clear();
        self.subskill_ranks.clear();
        self.tree_socketed.clear();
        self.skill_projectiles.clear();
    }

    pub fn set_mercenary_class(&mut self, id: Option<&str>) {
        if self.merc_class_id.as_deref() == id {
            return;
        }
        if id.is_some_and(|id| {
            !hsplanner_engine::calc::mercenary::data()
                .classes
                .iter()
                .any(|class| class.id == id)
        }) {
            return;
        }
        self.merc_class_id = id.map(str::to_owned);
        self.merc_skill_ranks.clear();
    }

    pub fn set_mercenary_skill(&mut self, id: &str, rank: u32) {
        let mercenary = hsplanner_engine::calc::mercenary::data();
        if !mercenary
            .classes
            .iter()
            .find(|class| Some(class.id.as_str()) == self.merc_class_id.as_deref())
            .is_some_and(|class| class.skills.iter().any(|skill| skill.id == id))
        {
            return;
        }
        let rank = rank.min(mercenary.max_skill_rank);
        if rank == 0 {
            self.merc_skill_ranks.remove(id);
        } else {
            self.merc_skill_ranks.insert(id.into(), rank);
        }
    }

    pub fn set_level(&mut self, level: u32) {
        self.level = level.clamp(1, data::game_config().max_character_level.max(1));
    }

    pub fn adjust_attribute(&mut self, key: &str, amount: i32) {
        if !data::game_config().attributes.iter().any(|a| a.key == key) {
            return;
        }
        let current = self.allocated.get(key).copied().unwrap_or(0);
        let spent: u32 = self.allocated.values().sum();
        let remaining = self
            .level
            .saturating_mul(data::game_config().attribute_points_per_level)
            .saturating_sub(spent);
        let value = if amount > 0 {
            current.saturating_add((amount as u32).min(remaining))
        } else {
            current.saturating_sub(amount.unsigned_abs())
        };
        self.allocated.insert(key.into(), value);
    }

    pub fn set_skill_rank(&mut self, id: &str, rank: u32) {
        let skills = data::get_skills_by_class(self.class_id.as_deref().unwrap_or(""));
        let Some(skill) = skills.iter().find(|s| s.id == id) else {
            return;
        };
        if rank > 0
            && skill
                .requires_skill
                .as_ref()
                .is_some_and(|required| self.skill_ranks.get(required).copied().unwrap_or(0) == 0)
        {
            return;
        }
        let other: u32 = self
            .skill_ranks
            .iter()
            .filter(|(key, _)| key.as_str() != id)
            .map(|(_, rank)| *rank)
            .sum();
        let rank = rank.min(skill.max_rank).min(
            self.level
                .saturating_mul(data::game_config().skill_points_per_level)
                .saturating_sub(other),
        );
        if rank > 0 {
            self.skill_ranks.insert(id.into(), rank);
            return;
        }
        self.skill_ranks.remove(id);
        let mut queue = vec![id.to_owned()];
        while let Some(removed) = queue.pop() {
            for dependent in skills
                .iter()
                .filter(|s| s.requires_skill.as_deref() == Some(&removed))
            {
                if self.skill_ranks.remove(&dependent.id).is_some() {
                    queue.push(dependent.id.clone());
                }
            }
        }
    }

    pub fn set_subskill_rank(&mut self, skill_id: &str, node_id: &str, rank: u32) {
        let skills = data::get_skills_by_class(self.class_id.as_deref().unwrap_or(""));
        let Some(nodes) = skills
            .iter()
            .find(|s| s.id == skill_id)
            .and_then(|s| s.subskills.as_ref())
        else {
            return;
        };
        let Some(node) = nodes.iter().find(|s| s.id == node_id) else {
            return;
        };
        let max = node.max_rank;
        let key = format!("{skill_id}:{node_id}");
        let prefix = format!("{skill_id}:");
        let current = self.subskill_ranks.get(&key).copied().unwrap_or(0);
        let other: u32 = self
            .subskill_ranks
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix) && **k != key)
            .map(|(_, v)| *v)
            .sum();
        let rank = rank.min(max).min(
            self.subskill_point_budget()
                .saturating_sub(other)
                .max(current),
        );
        if rank == 0 {
            self.subskill_ranks.remove(&key);
        } else {
            self.subskill_ranks.insert(key, rank);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn subskill_budget_defaults_for_old_saves_and_caps_imported_values() {
        let old: BuildSnapshot = serde_json::from_value(json!({"level": 100})).unwrap();
        assert_eq!(old.subskill_point_budget(), 20);
        for (saved, expected) in [(0, 1), (27, 27), (99, 30)] {
            let snapshot: BuildSnapshot =
                serde_json::from_value(json!({"maxSubskillPoints": saved})).unwrap();
            assert_eq!(snapshot.max_subskill_points, expected);
            assert_eq!(snapshot.subskill_point_budget(), expected);
        }
    }

    #[test]
    fn subskill_budget_applies_per_subtree_at_every_level() {
        let mut snapshot = BuildSnapshot {
            class_id: Some("stormweaver".into()),
            ..Default::default()
        };
        let skill = data::get_skills_by_class("stormweaver")
            .iter()
            .find(|skill| skill.id == "charged_bolts")
            .unwrap();
        let nodes = skill.subskills.as_ref().unwrap();
        snapshot.subskill_ranks.insert("other:node".into(), 30);
        for (requested, expected, level) in [(20, 20, 1), (27, 27, 50), (99, 30, 100)] {
            snapshot.set_level(level);
            snapshot.set_max_subskill_points(requested);
            for node in nodes.iter().filter(|node| node.position_index > 0) {
                snapshot.set_subskill_rank(&skill.id, &node.id, u32::MAX);
                assert!(
                    snapshot
                        .subskill_ranks
                        .get(&format!("{}:{}", skill.id, node.id))
                        .copied()
                        .unwrap_or(0)
                        <= node.max_rank
                );
            }
            let spent: u32 = snapshot
                .subskill_ranks
                .iter()
                .filter(|(key, _)| key.starts_with("charged_bolts:"))
                .map(|(_, rank)| rank)
                .sum();
            assert_eq!(spent, expected);
            assert_eq!(snapshot.subskill_ranks["other:node"], 30);
        }
    }

    #[test]
    fn lowering_subskill_budget_preserves_ranks_and_allows_gradual_refunds() {
        let mut snapshot = BuildSnapshot {
            class_id: Some("stormweaver".into()),
            ..Default::default()
        };
        for node in ["halting_storm", "weakening_charge", "efficient_wiring"] {
            snapshot.set_subskill_rank("charged_bolts", node, 5);
        }
        let before = snapshot.subskill_ranks.clone();
        snapshot.set_max_subskill_points(10);
        assert_eq!(snapshot.subskill_ranks, before);
        snapshot.set_subskill_rank("charged_bolts", "static_buildup", 1);
        assert!(
            !snapshot
                .subskill_ranks
                .contains_key("charged_bolts:static_buildup")
        );
        snapshot.set_subskill_rank("charged_bolts", "halting_storm", 4);
        assert_eq!(snapshot.subskill_ranks["charged_bolts:halting_storm"], 4);
        snapshot.set_subskill_rank("charged_bolts", "halting_storm", 5);
        assert_eq!(snapshot.subskill_ranks["charged_bolts:halting_storm"], 4);
        snapshot.set_subskill_rank("charged_bolts", "halting_storm", 0);
        snapshot.set_subskill_rank("charged_bolts", "weakening_charge", 4);
        snapshot.set_subskill_rank("charged_bolts", "static_buildup", 1);
        assert_eq!(snapshot.subskill_ranks["charged_bolts:static_buildup"], 1);
    }
}
