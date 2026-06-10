use std::collections::HashMap;

use super::affix::apply_stars_to_ranged_value;
use super::data::is_gear_slot;
use super::skills::{ItemSkillBonuses, Ranged, Skill, StatMap, r_max, r_min, rg};
use super::types::{Inventory, ItemBase};

pub fn normalize_skill_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Rank bonus shared by every skill-rank consumer: all_skills + element
/// skills + item-granted bonus. Exposed to the UI as `rankBonuses` so views
/// stop re-deriving it from raw stats.
pub fn rank_bonus_for(
    name: &str,
    damage_type: Option<&str>,
    stats: &StatMap,
    item_skill_bonuses: &ItemSkillBonuses,
) -> Ranged {
    let all = rg(stats, "all_skills");
    let elem = match damage_type {
        Some(dt) => rg(stats, &format!("{}_skills", dt)),
        None => (0.0, 0.0),
    };
    let item = item_skill_bonuses
        .get(&normalize_skill_name(name))
        .copied()
        .unwrap_or((0.0, 0.0));
    (
        r_min(all) + r_min(elem) + item.0,
        r_max(all) + r_max(elem) + item.1,
    )
}

pub fn effective_rank_range_for(
    skill: &Skill,
    base_rank: f64,
    stats: &StatMap,
    item_skill_bonuses: &ItemSkillBonuses,
) -> Ranged {
    if base_rank <= 0.0 {
        return (0.0, 0.0);
    }
    let bonus = rank_bonus_for(
        &skill.name,
        skill.damage_type.as_deref(),
        stats,
        item_skill_bonuses,
    );
    (base_rank + bonus.0, base_rank + bonus.1)
}

pub fn aggregate_item_skill_bonuses(
    inventory: &Inventory,
    items: &HashMap<String, ItemBase>,
) -> HashMap<String, Ranged> {
    let mut out: HashMap<String, Ranged> = HashMap::new();
    for (slot_key, item) in inventory {
        let Some(base) = items.get(&item.base_id) else {
            continue;
        };
        let Some(skill_bonuses) = base.skill_bonuses.as_ref() else {
            continue;
        };
        let stars = if is_gear_slot(slot_key) {
            item.stars
        } else {
            None
        };
        for (skill_name, val) in skill_bonuses {
            let scaled = apply_stars_to_ranged_value(
                val.as_ranged(),
                "item_granted_skill_rank",
                stars,
            );
            // No-op on the hot starred path (already floored); rounds away
            // fractional item-data fallback values.
            let min = r_min(scaled).round();
            let max = r_max(scaled).round();
            let key = normalize_skill_name(skill_name);
            let cur = out.entry(key).or_insert((0.0, 0.0));
            cur.0 += min;
            cur.1 += max;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::types::{EquippedItem, RangedValue};
    use super::*;

    fn skill(name: &str, damage_type: Option<&str>) -> Skill {
        Skill {
            name: name.to_string(),
            tags: vec![],
            damage_type: damage_type.map(String::from),
            damage_formula: None,
            damage_per_rank: None,
            bonus_sources: vec![],
            attack_kind: None,
            attack_scaling: None,
        }
    }

    fn item_base(id: &str, slot: &str, bonuses: &[(&str, Ranged)]) -> ItemBase {
        let map: HashMap<String, RangedValue> = bonuses
            .iter()
            .map(|(k, (lo, hi))| {
                let val = if lo == hi {
                    RangedValue::Scalar(*lo)
                } else {
                    RangedValue::Range([*lo, *hi])
                };
                (k.to_string(), val)
            })
            .collect();
        ItemBase {
            id: id.to_string(),
            slot: slot.to_string(),
            skill_bonuses: if map.is_empty() { None } else { Some(map) },
            ..Default::default()
        }
    }

    fn equipped(base_id: &str, stars: Option<u32>) -> EquippedItem {
        EquippedItem {
            base_id: base_id.to_string(),
            stars,
            ..Default::default()
        }
    }

    // ---- normalize_skill_name ----

    #[test]
    fn normalize_trims_and_lowercases() {
        assert_eq!(normalize_skill_name("Fireball"), "fireball");
        assert_eq!(normalize_skill_name("  Frost Nova  "), "frost nova");
        assert_eq!(normalize_skill_name("ALREADY_lower"), "already_lower");
        assert_eq!(normalize_skill_name(""), "");
    }

    // ---- rank_bonus_for ----

    #[test]
    fn rank_bonus_sums_all_element_and_item_components() {
        let mut stats: StatMap = HashMap::new();
        stats.insert("all_skills".into(), (1.0, 2.0));
        stats.insert("fire_skills".into(), (1.0, 1.0));
        let mut bonuses: ItemSkillBonuses = HashMap::new();
        bonuses.insert("fireball".into(), (1.0, 3.0));
        assert_eq!(
            rank_bonus_for("Fireball", Some("fire"), &stats, &bonuses),
            (3.0, 6.0)
        );
    }

    #[test]
    fn rank_bonus_without_damage_type_skips_element() {
        let mut stats: StatMap = HashMap::new();
        stats.insert("all_skills".into(), (2.0, 2.0));
        stats.insert("fire_skills".into(), (99.0, 99.0));
        let bonuses: ItemSkillBonuses = HashMap::new();
        assert_eq!(rank_bonus_for("Berserk", None, &stats, &bonuses), (2.0, 2.0));
    }

    #[test]
    fn rank_bonus_plus_base_matches_effective_rank() {
        let s = skill("Fireball", Some("fire"));
        let mut stats: StatMap = HashMap::new();
        stats.insert("all_skills".into(), (1.0, 2.0));
        stats.insert("fire_skills".into(), (1.0, 1.0));
        let mut bonuses: ItemSkillBonuses = HashMap::new();
        bonuses.insert("fireball".into(), (1.0, 3.0));
        let bonus = rank_bonus_for(&s.name, s.damage_type.as_deref(), &stats, &bonuses);
        let eff = effective_rank_range_for(&s, 10.0, &stats, &bonuses);
        assert_eq!((10.0 + bonus.0, 10.0 + bonus.1), eff);
    }

    // ---- effective_rank_range_for ----

    #[test]
    fn effective_rank_zero_base_returns_zero() {
        let s = skill("Fireball", Some("fire"));
        let stats: StatMap = HashMap::new();
        let bonuses: ItemSkillBonuses = HashMap::new();
        assert_eq!(effective_rank_range_for(&s, 0.0, &stats, &bonuses), (0.0, 0.0));
        assert_eq!(effective_rank_range_for(&s, -3.0, &stats, &bonuses), (0.0, 0.0));
    }

    #[test]
    fn effective_rank_sums_all_skills_and_element() {
        let s = skill("Fireball", Some("fire"));
        let mut stats: StatMap = HashMap::new();
        stats.insert("all_skills".into(), (1.0, 2.0));
        stats.insert("fire_skills".into(), (1.0, 1.0));
        let bonuses: ItemSkillBonuses = HashMap::new();
        assert_eq!(
            effective_rank_range_for(&s, 10.0, &stats, &bonuses),
            (12.0, 13.0)
        );
    }

    #[test]
    fn effective_rank_includes_item_bonus_lookup_by_normalized_name() {
        let s = skill("Fireball", Some("fire"));
        let stats: StatMap = HashMap::new();
        let mut bonuses: ItemSkillBonuses = HashMap::new();
        bonuses.insert("fireball".into(), (1.0, 3.0));
        assert_eq!(
            effective_rank_range_for(&s, 10.0, &stats, &bonuses),
            (11.0, 13.0)
        );
        let s2 = skill("  FIREBALL  ", Some("fire"));
        assert_eq!(
            effective_rank_range_for(&s2, 10.0, &stats, &bonuses),
            (11.0, 13.0)
        );
    }

    #[test]
    fn effective_rank_no_damage_type_skips_element() {
        let s = skill("Berserk", None);
        let mut stats: StatMap = HashMap::new();
        stats.insert("all_skills".into(), (2.0, 2.0));
        stats.insert("fire_skills".into(), (99.0, 99.0)); // must be ignored
        let bonuses: ItemSkillBonuses = HashMap::new();
        assert_eq!(
            effective_rank_range_for(&s, 5.0, &stats, &bonuses),
            (7.0, 7.0)
        );
    }

    // ---- aggregate_item_skill_bonuses ----

    #[test]
    fn aggregate_empty_inventory_returns_empty() {
        let db: HashMap<String, ItemBase> = HashMap::new();
        let inv: Inventory = HashMap::new();
        let out = aggregate_item_skill_bonuses(&inv, &db);
        assert!(out.is_empty());
    }

    #[test]
    fn aggregate_item_without_bonuses_skipped() {
        let mut db: HashMap<String, ItemBase> = HashMap::new();
        db.insert("plain_sword".into(), item_base("plain_sword", "weapon", &[]));
        let mut inv: Inventory = HashMap::new();
        inv.insert("weapon".into(), equipped("plain_sword", None));
        let out = aggregate_item_skill_bonuses(&inv, &db);
        assert!(out.is_empty());
    }

    #[test]
    fn aggregate_missing_base_skipped() {
        let db: HashMap<String, ItemBase> = HashMap::new();
        let mut inv: Inventory = HashMap::new();
        inv.insert("weapon".into(), equipped("nonexistent", Some(3)));
        let out = aggregate_item_skill_bonuses(&inv, &db);
        assert!(out.is_empty());
    }

    #[test]
    fn aggregate_gear_slot_zero_stars_passes_through() {
        let mut db: HashMap<String, ItemBase> = HashMap::new();
        db.insert(
            "fire_amulet".into(),
            item_base("fire_amulet", "amulet", &[("Fireball", (2.0, 2.0))]),
        );
        let mut inv: Inventory = HashMap::new();
        inv.insert("amulet".into(), equipped("fire_amulet", None));
        let out = aggregate_item_skill_bonuses(&inv, &db);
        assert_eq!(out.get("fireball"), Some(&(2.0, 2.0)));
    }

    #[test]
    fn aggregate_gear_slot_with_stars_applies_staircase() {
        // item_granted_skill_rank uses ITEM_SPECIFIC_STAIRCASE: 4 stars → +2 flat.
        // (1,1) + flat=2 → (3,3); .round() keeps 3.
        let mut db: HashMap<String, ItemBase> = HashMap::new();
        db.insert(
            "amulet_4s".into(),
            item_base("amulet_4s", "amulet", &[("Fireball", (1.0, 1.0))]),
        );
        let mut inv: Inventory = HashMap::new();
        inv.insert("amulet".into(), equipped("amulet_4s", Some(4)));
        let out = aggregate_item_skill_bonuses(&inv, &db);
        assert_eq!(out.get("fireball"), Some(&(3.0, 3.0)));
    }

    #[test]
    fn aggregate_non_gear_slot_ignores_stars() {
        // 'relic' is not a gear slot — stars dropped before star scaling.
        let mut db: HashMap<String, ItemBase> = HashMap::new();
        db.insert(
            "relic_x".into(),
            item_base("relic_x", "relic", &[("Fireball", (1.0, 1.0))]),
        );
        let mut inv: Inventory = HashMap::new();
        inv.insert("relic".into(), equipped("relic_x", Some(5)));
        let out = aggregate_item_skill_bonuses(&inv, &db);
        assert_eq!(out.get("fireball"), Some(&(1.0, 1.0)));
    }

    #[test]
    fn aggregate_multiple_items_sum_per_skill() {
        let mut db: HashMap<String, ItemBase> = HashMap::new();
        db.insert(
            "amulet_a".into(),
            item_base("amulet_a", "amulet", &[("Fireball", (1.0, 2.0))]),
        );
        db.insert(
            "ring_a".into(),
            item_base("ring_a", "ring_1", &[("Fireball", (1.0, 1.0))]),
        );
        let mut inv: Inventory = HashMap::new();
        inv.insert("amulet".into(), equipped("amulet_a", None));
        inv.insert("ring_1".into(), equipped("ring_a", None));
        let out = aggregate_item_skill_bonuses(&inv, &db);
        assert_eq!(out.get("fireball"), Some(&(2.0, 3.0)));
    }

    #[test]
    fn aggregate_multiple_skills_per_item() {
        let mut db: HashMap<String, ItemBase> = HashMap::new();
        db.insert(
            "all_amulet".into(),
            item_base(
                "all_amulet",
                "amulet",
                &[("Fireball", (1.0, 1.0)), ("Frost Nova", (2.0, 3.0))],
            ),
        );
        let mut inv: Inventory = HashMap::new();
        inv.insert("amulet".into(), equipped("all_amulet", None));
        let out = aggregate_item_skill_bonuses(&inv, &db);
        assert_eq!(out.get("fireball"), Some(&(1.0, 1.0)));
        assert_eq!(out.get("frost nova"), Some(&(2.0, 3.0)));
    }

    #[test]
    fn aggregate_normalizes_skill_keys() {
        // Two items with differently-cased skill names collapse to one key.
        let mut db: HashMap<String, ItemBase> = HashMap::new();
        db.insert(
            "a".into(),
            item_base("a", "amulet", &[("FIREBALL", (1.0, 1.0))]),
        );
        db.insert(
            "b".into(),
            item_base("b", "ring_1", &[("  fireball  ", (2.0, 2.0))]),
        );
        let mut inv: Inventory = HashMap::new();
        inv.insert("amulet".into(), equipped("a", None));
        inv.insert("ring_1".into(), equipped("b", None));
        let out = aggregate_item_skill_bonuses(&inv, &db);
        assert_eq!(out.len(), 1);
        assert_eq!(out.get("fireball"), Some(&(3.0, 3.0)));
    }
}
