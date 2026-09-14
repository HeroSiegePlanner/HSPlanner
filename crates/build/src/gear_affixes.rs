//! Reference Gear affix-pool restrictions, shared by the picker and edit validation.
use hsplanner_engine::calc::{
    data, season,
    types::{Affix, ItemBase},
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex},
};

struct Pools {
    groups: HashMap<String, Vec<String>>,
    random: HashSet<String>,
}

static POOLS: LazyLock<Mutex<HashMap<String, &'static Pools>>> = LazyLock::new(Default::default);
thread_local! { static CURRENT: RefCell<Option<(String, &'static Pools)>> = const { RefCell::new(None) }; }

fn pools() -> &'static Pools {
    season::memoized_current_season(&CURRENT, |id| {
        season::cached_per_season(&POOLS, id, |id| {
            let base = serde_json::from_str(include_str!("../../../data/affix-pools.json"))
                .expect("valid affix pools");
            let value = season::patches_for(id)
                .get("affix-pools")
                .and_then(|patch| {
                    season::apply_record_patch(&base, patch, "affix-pools", false).ok()
                })
                .unwrap_or(base);
            Pools {
                groups: serde_json::from_value(value).expect("valid affix pool types"),
                random: data::data_for(id)
                    .items
                    .values()
                    .filter_map(|base| base.random_affix_group_id.clone())
                    .collect(),
            }
        })
    })
}

pub fn jewel_affix_allowed(affix: &Affix) -> bool {
    affix.stat_key.is_some()
        && pools()
            .groups
            .get(&affix.group_id)
            .is_some_and(|types| types.iter().any(|kind| kind == "Socketable"))
}

pub fn affix_pool_type(base: &ItemBase) -> Option<&'static str> {
    if base.slot == "weapon" {
        return match base.base_type.as_str() {
            "Sword" | "Mace" | "Dagger" | "Claw" | "Axe" | "Polearm" | "Chainsaw" | "Novelty" => {
                Some("Weapon:Melee")
            }
            "Bow" | "Gun" | "Rifle Gun" | "Throwing" | "1-Handed Throwing Weapon" => {
                Some("Weapon:Ranged")
            }
            "Staff" | "Cane" | "Wand" | "Book" | "Spellblade" | "Flask" => Some("Weapon:Caster"),
            _ => None,
        };
    }
    match super::slot_group(&base.slot) {
        "helmet" => Some("Helmet"),
        "armor" => Some("Chest"),
        "boots" => Some("Boots"),
        "gloves" => Some("Gloves"),
        "belt" => Some("Belt"),
        "amulet" => Some("Amulet"),
        "ring" => Some("Ring"),
        "charm" => Some("Charm"),
        "offhand" => Some("Shield"),
        "potion" => Some("Flask"),
        _ => None,
    }
}

pub fn affix_allowed(base: &ItemBase, affix: &Affix, allow_outside_pool: bool) -> bool {
    if let Some(group) = &base.random_affix_group_id {
        return affix.group_id == *group;
    }
    let pools = pools();
    let group = pools.groups.get(&affix.group_id);
    if base.rarity != "common"
        || pools.random.contains(&affix.group_id)
        || group.is_some_and(|types| types.iter().any(|kind| kind == "Socketable"))
    {
        return false;
    }
    allow_outside_pool
        || affix_pool_type(base)
            .is_none_or(|kind| group.is_none_or(|types| types.iter().any(|value| value == kind)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gear::{add_affix, add_affix_with_pool_override, make_item};

    #[test]
    fn random_items_cannot_accept_normal_or_jewel_affixes_even_with_override() {
        let base = data::data()
            .items
            .values()
            .find(|base| base.random_affix_group_id.as_deref() == Some("random_unholy"))
            .unwrap();
        let own = data::data()
            .affixes
            .values()
            .find(|a| a.group_id == "random_unholy")
            .unwrap();
        let normal = data::data()
            .affixes
            .values()
            .find(|a| a.group_id == "1_increased_experience_gain")
            .unwrap();
        let mut item = make_item(&base.id).unwrap();
        assert!(add_affix(&mut item, &normal.id).is_err());
        assert!(add_affix_with_pool_override(&mut item, &normal.id, true).is_err());
        assert!(item.affixes.is_empty());
        add_affix(&mut item, &own.id).unwrap();
        assert_eq!(item.affixes[0].affix_id, own.id);
    }

    #[test]
    fn regular_pool_override_preserves_reference_behavior_but_excludes_socketables() {
        let base = ItemBase {
            id: String::new(),
            slot: "weapon".into(),
            base_type: "Sword".into(),
            rarity: "common".into(),
            ..Default::default()
        };
        let experience = data::data()
            .affixes
            .values()
            .find(|a| a.group_id == "1_increased_experience_gain")
            .unwrap();
        assert!(!affix_allowed(&base, experience, false));
        assert!(affix_allowed(&base, experience, true));
        let jewel = data::data()
            .affixes
            .values()
            .find(|a| {
                pools()
                    .groups
                    .get(&a.group_id)
                    .is_some_and(|types| types.iter().any(|kind| kind == "Socketable"))
            })
            .unwrap();
        assert!(!affix_allowed(&base, jewel, true));
        let unknown = ItemBase {
            base_type: "Trumpet".into(),
            ..base.clone()
        };
        assert!(affix_allowed(&unknown, experience, false));
        assert_eq!(affix_pool_type(&base), Some("Weapon:Melee"));
    }
}

/// Random pools contain unrelated effects; only ordinary groups form tier ladders.
pub fn affix_tiers(id: &str) -> Vec<&'static Affix> {
    let Some(affix) = data::get_affix(id) else {
        return vec![];
    };
    if pools().random.contains(&affix.group_id) || affix.group_id.is_empty() {
        return vec![affix];
    }
    let mut tiers: Vec<_> = data::data()
        .affixes
        .values()
        .filter(|a| a.group_id == affix.group_id)
        .collect();
    tiers.sort_by(|a, b| a.tier.cmp(&b.tier).then(a.id.cmp(&b.id)));
    tiers
}

pub fn affix_roll_bounds(id: &str, stars: Option<u32>) -> Option<(f64, f64)> {
    affix_tiers(id)
        .into_iter()
        .filter_map(|a| tier_bounds(a, stars))
        .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
}

fn tier_bounds(affix: &Affix, stars: Option<u32>) -> Option<(f64, f64)> {
    affix.value_min?;
    affix.value_max?;
    let a = hsplanner_engine::calc::affix::rolled_affix_value_with_stars(affix, 0., stars).abs();
    let b = hsplanner_engine::calc::affix::rolled_affix_value_with_stars(affix, 1., stars).abs();
    Some((a.min(b), a.max(b)))
}

/// Resolve overlaps to the lowest tier and gaps to the nearest reachable value.
pub fn set_affix_value(item: &mut super::EquippedItem, index: usize, value: f64) -> bool {
    let Some(eq) = item.affixes.get(index) else {
        return false;
    };
    let stars = data::get_item(&item.base_id)
        .filter(|base| data::can_star_forge(&base.slot, &base.rarity))
        .and(item.stars);
    let tiers = affix_tiers(&eq.affix_id);
    apply_affix_value(&mut item.affixes[index], &tiers, stars, value)
}

fn apply_affix_value(
    eq: &mut hsplanner_engine::calc::types::EquippedAffix,
    tiers: &[&Affix],
    stars: Option<u32>,
    value: f64,
) -> bool {
    if !value.is_finite() {
        return false;
    }
    let mut best = None;
    let mut distance = f64::INFINITY;
    for &tier in tiers {
        let Some((lo, hi)) = tier_bounds(tier, stars) else {
            continue;
        };
        let target = value.abs().clamp(lo, hi);
        let delta = (target - value.abs()).abs();
        if delta < distance {
            best = Some((tier, target));
            distance = delta;
        }
    }
    let Some((tier, target)) = best else {
        return false;
    };
    let start = hsplanner_engine::calc::affix::rolled_affix_value_with_stars(tier, 0., stars).abs();
    let end = hsplanner_engine::calc::affix::rolled_affix_value_with_stars(tier, 1., stars).abs();
    let roll = if start == end {
        1.
    } else {
        ((target - start) / (end - start)).clamp(0., 1.)
    };
    eq.affix_id = tier.id.clone();
    eq.tier = tier.tier;
    eq.roll = roll;
    eq.custom_value = None;
    true
}

#[cfg(test)]
mod tier_roll_tests {
    use super::*;
    use hsplanner_engine::calc::{affix::rolled_affix_value_with_stars, types::EquippedAffix};

    #[test]
    fn defense_family_changes_tier_and_roll_and_clears_override() {
        let id = "25_50_enhanced_defense_t1_armorer_s";
        let tiers = affix_tiers(id);
        assert_eq!(tiers.len(), 5);
        assert_eq!(affix_roll_bounds(id, None), Some((20., 225.)));
        let mut item = super::super::EquippedItem::default();
        item.affixes.push(EquippedAffix {
            affix_id: id.into(),
            tier: 1,
            roll: 1.,
            custom_value: Some(999.),
        });
        for (value, tier) in [(40., 1), (20., 2), (70., 3), (100., 4), (225., 5)] {
            assert!(set_affix_value(&mut item, 0, value));
            let eq = &item.affixes[0];
            assert_eq!(eq.tier, tier);
            assert_eq!(eq.custom_value, None);
            assert!(
                (rolled_affix_value_with_stars(
                    data::get_affix(&eq.affix_id).unwrap(),
                    eq.roll,
                    None
                ) - value)
                    .abs()
                    < 0.001
            );
        }
    }

    #[test]
    fn random_pool_effects_do_not_become_tiers_of_other_stats() {
        let affix = data::data()
            .affixes
            .values()
            .find(|a| a.group_id == "random_unholy")
            .unwrap();
        assert_eq!(affix_tiers(&affix.id).len(), 1);
    }

    #[test]
    fn negative_values_and_gaps_resolve_to_reachable_rolls() {
        use hsplanner_engine::calc::types::{AffixFormat, AffixSign};
        let low = Affix {
            id: "low".into(),
            tier: 1,
            value_min: Some(10.),
            value_max: Some(20.),
            sign: AffixSign::Minus,
            format: AffixFormat::Percent,
            ..Default::default()
        };
        let high = Affix {
            id: "high".into(),
            tier: 2,
            value_min: Some(30.),
            value_max: Some(40.),
            ..low.clone()
        };
        let mut eq = EquippedAffix::default();
        for (requested, expected, tier) in [
            (0., -10., 1),
            (25., -20., 1),
            (28., -30., 2),
            (100., -40., 2),
        ] {
            assert!(apply_affix_value(&mut eq, &[&low, &high], None, requested));
            assert_eq!(eq.tier, tier);
            let affix = if tier == 1 { &low } else { &high };
            assert_eq!(
                rolled_affix_value_with_stars(affix, eq.roll, None),
                expected
            );
        }
        assert!(!apply_affix_value(&mut eq, &[&low], None, f64::NAN));
    }
}

/// Pin a crystal's raw value; the engine uses custom_value to replace its range.
pub fn set_forge_value(item: &mut super::EquippedItem, index: usize, value: f64) -> bool {
    let Some(eq) = item.forged_mods.get_mut(index) else {
        return false;
    };
    let Some(affix) = data::get_crystal_mod(&eq.affix_id) else {
        return false;
    };
    if !apply_affix_value(eq, &[affix], None, value) {
        return false;
    }
    eq.custom_value =
        Some(hsplanner_engine::calc::affix::rolled_affix_value_with_stars(affix, eq.roll, None));
    true
}
