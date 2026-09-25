//! Cast requirements from TalentRequirement, distinct from equipment damage
//! bonuses. See ../weapon-requirements-evidence.md for the executable evidence.
use std::collections::HashMap;

use crate::calc::{
    data,
    subskill::subskill_key,
    types::{Inventory, SkillSpec, SkillWeaponRequirement},
};

pub fn unmet_requirement(
    skill: &SkillSpec,
    inventory: &Inventory,
    subskills: &HashMap<String, u32>,
    restrictions_removed: bool,
) -> Option<&'static str> {
    use SkillWeaponRequirement::*;
    if restrictions_removed || skill.weapon_requirement == None {
        return Option::None;
    }
    let weapon = inventory
        .get("weapon")
        .and_then(|item| data::get_item(&item.base_id));
    let kind = weapon.map(|item| item.base_type.as_str()).unwrap_or("");
    // Unlike Enhanced Damage's weapon grouping, cast requirements include Wand.
    let ranged = matches!(
        kind,
        "Wand" | "Bow" | "Gun" | "Rifle Gun" | "Flask" | "Throwing" | "1-Handed Throwing Weapon"
    );
    let allocated = |id| {
        subskills
            .get(&subskill_key(&skill.id, id))
            .copied()
            .unwrap_or(0)
            > 0
    };
    if skill.class_id == "viking" {
        if skill.id == "berserk" && allocated("deadly_advantage") {
            return Option::None;
        }
        if skill.id == "zeal"
            && allocated("weapon_marksman")
            && matches!(kind, "Throwing" | "1-Handed Throwing Weapon")
        {
            return Option::None;
        }
    }
    // Shield Slam additionally rejects ranged main-hand weapons before the
    // common shield check. Counter and Shield Wall only require the shield.
    if skill.class_id == "shield_lancer" && skill.id == "shield_slam" && ranged {
        return Some("Requires a melee main-hand weapon and a shield");
    }
    match skill.weapon_requirement {
        Melee if ranged => Some("Requires a melee main-hand weapon"),
        Ranged if !ranged => Some("Requires a ranged main-hand weapon"),
        Shield
            if !inventory
                .get("offhand")
                .and_then(|item| data::get_item(&item.base_id))
                .is_some_and(|item| item.base_type == "Shield") =>
        {
            Some("Requires a shield")
        }
        _ => Option::None,
    }
}
