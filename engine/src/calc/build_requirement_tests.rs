use super::*;
use crate::calc::types::{EquippedItem, SkillWeaponRequirement};

fn performance(
    class: &str,
    skill: &str,
    weapon: Option<&str>,
    shield: bool,
    wargod: bool,
    subskill: Option<&str>,
) -> BuildPerformance {
    let mut inventory = Inventory::new();
    for (slot, kind) in [("weapon", weapon), ("offhand", shield.then_some("Shield"))] {
        if let Some(kind) = kind {
            let base = data::data()
                .items
                .values()
                .filter(|base| base.base_type == kind && base.rarity == "common")
                .min_by_key(|base| &base.id)
                .unwrap();
            inventory.insert(
                slot.into(),
                EquippedItem {
                    base_id: base.id.clone(),
                    ..Default::default()
                },
            );
        }
    }
    let nodes = if wargod {
        HashSet::from([737])
    } else {
        HashSet::new()
    };
    let subs = subskill
        .map(|id| HashMap::from([(subskill_key(skill, id), 1)]))
        .unwrap_or_default();
    compute_build_performance(&BuildPerformanceDeps {
        class_id: Some(class),
        level: 50,
        allocated_attrs: &HashMap::new(),
        inventory: &inventory,
        skill_ranks: &HashMap::from([(skill.into(), 20)]),
        subskill_ranks: &subs,
        active_aura_id: None,
        active_buffs: &HashMap::new(),
        custom_stats: &[],
        allocated_tree_nodes: &nodes,
        tree_socketed: &HashMap::new(),
        main_skill_id: Some(skill),
        enemy_conditions: &HashMap::new(),
        player_conditions: &HashMap::new(),
        skill_projectiles: &HashMap::new(),
        enemy_resistances: &HashMap::new(),
        proc_toggles: &HashMap::new(),
        kills_per_sec: 0.0,
        entity_rates: &HashMap::new(),
        stack_counts: &HashMap::new(),
        granted_skill_ranks: None,
        difficulty: None,
    })
}

#[test]
fn all_75_extracted_requirements_are_enforced_and_wargod_unlocks_them() {
    let mut counts = [0; 3];
    for (class, skills) in &data::data().skills_by_class {
        for skill in skills {
            let (index, good_weapon, bad_weapon, shield) = match skill.weapon_requirement {
                SkillWeaponRequirement::None => continue,
                SkillWeaponRequirement::Melee => (0, "Sword", "Bow", false),
                SkillWeaponRequirement::Ranged => (1, "Bow", "Sword", false),
                SkillWeaponRequirement::Shield => (2, "Sword", "Sword", true),
            };
            counts[index] += 1;
            let good = performance(class, &skill.id, Some(good_weapon), shield, false, None);
            let bad = performance(class, &skill.id, Some(bad_weapon), false, false, None);
            let unlocked = performance(class, &skill.id, Some(bad_weapon), false, true, None);
            assert!(
                good.skill_costs[&skill.id].unavailable_reason.is_none(),
                "{}",
                skill.id
            );
            let cost = &bad.skill_costs[&skill.id];
            assert!(cost.unavailable_reason.is_some(), "{}", skill.id);
            assert_eq!(cost.cast_rate_max, Some(0.0));
            assert_eq!(cost.mana_per_sec_max, Some(0.0));
            assert_eq!(bad.avg_hit_dps_max.unwrap_or(0.0), 0.0);
            assert!(
                unlocked.skill_costs[&skill.id].unavailable_reason.is_none(),
                "{}",
                skill.id
            );
        }
    }
    assert_eq!(counts, [53, 19, 3]);
}

#[test]
fn requirements_use_equipment_including_wands_and_preserve_subskill_exceptions() {
    for (class, skill, weapon, shield, subskill, usable) in [
        ("viking", "zeal", None, false, None, true),
        ("viking", "zeal", Some("Staff"), false, None, true),
        ("viking", "zeal", Some("Wand"), false, None, false),
        ("amazon", "rebound", Some("Wand"), false, None, true),
        ("viking", "zeal", Some("Throwing"), false, None, false),
        (
            "viking",
            "zeal",
            Some("Throwing"),
            false,
            Some("weapon_marksman"),
            true,
        ),
        (
            "viking",
            "zeal",
            Some("Bow"),
            false,
            Some("weapon_marksman"),
            false,
        ),
        (
            "viking",
            "berserk",
            Some("Bow"),
            false,
            Some("deadly_advantage"),
            true,
        ),
        (
            "shield_lancer",
            "shield_slam",
            Some("Wand"),
            true,
            None,
            false,
        ),
        ("shield_lancer", "counter", Some("Wand"), true, None, true),
    ] {
        let result = performance(class, skill, weapon, shield, false, subskill);
        assert_eq!(
            result.skill_costs[skill].unavailable_reason.is_none(),
            usable,
            "{skill} {weapon:?} {subskill:?}"
        );
    }
}

#[test]
fn invalid_cast_has_no_hits_or_trigger_contacts_and_wargod_restores_them() {
    for (class, skill, weapon) in [
        ("amazon", "death_from_above", "Sword"),
        ("viking", "zeal", "Bow"),
    ] {
        let blocked = performance(class, skill, Some(weapon), false, false, None);
        let allowed = performance(class, skill, Some(weapon), false, true, None);
        assert!(blocked.damage.is_none() && blocked.attack_damage.is_none());
        let contacts = |p: &BuildPerformance| {
            p.calculation()
                .iter()
                .find(|s| s.label() == "Proc trigger contacts per second")
                .unwrap()
                .value()
        };
        assert_eq!(contacts(&blocked), (0.0, 0.0));
        assert!(contacts(&allowed).1 > 0.0);
        assert!(allowed.avg_hit_dps_max.unwrap() > 0.0);
        assert!(blocked
            .calculation()
            .iter()
            .any(|s| s.label() == "Skill unavailable"));
    }
}
