//! Which weapon types a class or mercenary may wield, from the game's
//! `CheckWeaponType` and `CanDualWieldClass`.
use hsplanner_engine::calc::{
    data, mercenary,
    stats::weapon_kind_of,
    types::{EquippedItem, ItemBase},
};

/// The game's only item flagged "ignore weapon type" (item field 24).
const IGNORES_WEAPON_TYPE: &str = "wand_satanic_witch_s_wand";
/// Talent 575 `unboundPower`: "Enables equipping any type of weapon".
const UNBOUND_POWER: &str = "unbound power";

const CASTER_WEAPONS: &[&str] = &["Wand", "Book", "Spellblade", "Flask"];
const PHYSICAL_CLASSES: &[&str] = &[
    "viking",
    "marksman",
    "nomad",
    "redneck",
    "samurai",
    "amazon",
    "demon_slayer",
    "marauder",
    "shield_lancer",
    "butcher",
];

const MELEE_WEAPONS: &[&str] = &[
    "Sword", "Dagger", "Mace", "Axe", "Claw", "Polearm", "Chainsaw", "Novelty",
];
const RANGED_WEAPONS: &[&str] = &["Bow", "Gun", "Throwing", "Novelty"];
const SPELL_WEAPONS: &[&str] = &[
    "Staff",
    "Cane",
    "Wand",
    "Book",
    "Spellblade",
    "Flask",
    "Novelty",
];

const DUAL_WIELD_ALL: &[&str] = &["Sword", "Dagger", "Mace", "Axe", "Claw", "Gun"];

fn ignores_weapon_type(base: &ItemBase) -> bool {
    base.id == IGNORES_WEAPON_TYPE
}

/// Whether the player's class may hold this weapon at all.
pub fn class_can_wield(class_id: Option<&str>, base: &ItemBase, unbound_power: bool) -> bool {
    ignores_weapon_type(base)
        || unbound_power
        || !(class_id.is_some_and(|class| PHYSICAL_CLASSES.contains(&class))
            && CASTER_WEAPONS.contains(&weapon_kind_of(base).as_str()))
}

/// Whether a mercenary of this class may hold this weapon; no hired class means no limit.
pub fn mercenary_can_wield(merc_class_id: Option<&str>, base: &ItemBase) -> bool {
    if ignores_weapon_type(base) {
        return true;
    }
    let Some(class) = merc_class_id.and_then(|id| {
        mercenary::data()
            .classes
            .iter()
            .find(|class| class.id == id)
    }) else {
        return true;
    };
    let allowed = match class.role.as_str() {
        "Melee" => MELEE_WEAPONS,
        "Ranged" => RANGED_WEAPONS,
        _ => SPELL_WEAPONS,
    };
    allowed.contains(&weapon_kind_of(base).as_str())
}

/// Weapon types the class may hold in its off hand without tree overrides.
pub fn class_dual_wield_types(class_id: Option<&str>) -> &'static [&'static str] {
    match class_id.unwrap_or_default() {
        "viking" | "nomad" | "redneck" | "samurai" | "demon_slayer" | "marauder" | "jotunn"
        | "butcher" | "bard" | "prophet" => DUAL_WIELD_ALL,
        "amazon" => &["Sword", "Dagger", "Mace", "Axe", "Claw"],
        "marksman" | "pirate" => &["Gun"],
        "shaman" => &["Claw"],
        _ => &[],
    }
}

/// Whether an equipped item grants Unbound Power.
pub fn grants_unbound_power<'a>(items: impl IntoIterator<Item = &'a EquippedItem>) -> bool {
    items.into_iter().any(|item| {
        data::get_item(&item.base_id).is_some_and(|base| {
            data::skill_bonus_entries(base, item)
                .any(|(name, _)| name.trim().eq_ignore_ascii_case(UNBOUND_POWER))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(id: &str) -> &'static ItemBase {
        data::get_item(id).unwrap_or_else(|| panic!("missing item {id}"))
    }

    fn weapon_of(kind: &str) -> &'static ItemBase {
        data::data()
            .items
            .values()
            .find(|base| {
                base.slot == "weapon" && base.base_type == kind && !ignores_weapon_type(base)
            })
            .unwrap_or_else(|| panic!("no {kind}"))
    }

    #[test]
    fn physical_classes_cannot_wield_caster_weapons() {
        for kind in CASTER_WEAPONS {
            assert!(
                !class_can_wield(Some("viking"), weapon_of(kind), false),
                "{kind}"
            );
            assert!(
                class_can_wield(Some("pyromancer"), weapon_of(kind), false),
                "{kind}"
            );
        }
        assert!(class_can_wield(Some("viking"), weapon_of("Staff"), false));
        assert!(class_can_wield(Some("butcher"), weapon_of("Cane"), false));
    }

    #[test]
    fn witchs_wand_and_unbound_power_lift_the_block() {
        assert!(class_can_wield(
            Some("viking"),
            base(IGNORES_WEAPON_TYPE),
            false
        ));
        assert!(class_can_wield(Some("viking"), weapon_of("Wand"), true));
        let ring = EquippedItem {
            base_id: "ring_heroic_the_absence_of_constraint".into(),
            ..Default::default()
        };
        assert!(grants_unbound_power([&ring]));
        assert!(!grants_unbound_power([]));
    }

    #[test]
    fn mercenaries_follow_their_role() {
        assert!(mercenary_can_wield(
            Some("merc_knight"),
            weapon_of("Polearm")
        ));
        assert!(!mercenary_can_wield(Some("merc_knight"), weapon_of("Bow")));
        assert!(mercenary_can_wield(
            Some("merc_archer"),
            weapon_of("Rifle Gun")
        ));
        assert!(!mercenary_can_wield(
            Some("merc_archer"),
            weapon_of("Staff")
        ));
        assert!(mercenary_can_wield(
            Some("merc_magister"),
            weapon_of("Flask")
        ));
        assert!(!mercenary_can_wield(
            Some("merc_magister"),
            weapon_of("Sword")
        ));
        assert!(mercenary_can_wield(
            Some("merc_knight"),
            base(IGNORES_WEAPON_TYPE)
        ));
        assert!(mercenary_can_wield(None, weapon_of("Bow")));
    }

    #[test]
    fn dual_wield_types_follow_the_class_table() {
        assert_eq!(class_dual_wield_types(Some("viking")), DUAL_WIELD_ALL);
        assert_eq!(class_dual_wield_types(Some("marksman")), ["Gun"]);
        assert_eq!(class_dual_wield_types(Some("shaman")), ["Claw"]);
        assert!(!class_dual_wield_types(Some("amazon")).contains(&"Gun"));
        assert!(class_dual_wield_types(Some("pyromancer")).is_empty());
    }
}
