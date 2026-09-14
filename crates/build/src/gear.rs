//! Item edits shared by native gear, stash, mercenary and import flows.
use crate::{
    BuildSnapshot,
    library::{StashEntry, new_id},
    session::Draft,
};
use hsplanner_engine::calc::{
    data,
    types::{AugmentRef, EquippedAffix, EquippedItem, ItemBase, SocketType},
};

#[path = "gear_affixes.rs"]
mod affix_pools;
pub use affix_pools::{
    affix_allowed, affix_pool_type, affix_roll_bounds, affix_tiers, jewel_affix_allowed,
    set_affix_value, set_forge_value,
};

pub fn make_item(base_id: &str) -> Result<EquippedItem, String> {
    let base = data::get_item(base_id).ok_or("Unknown item.")?;
    let mut item = EquippedItem {
        base_id: base_id.into(),
        stars: Some(0),
        ..Default::default()
    };
    set_socket_count(&mut item, base.sockets.unwrap_or(0));
    Ok(item)
}

pub fn max_sockets(item: &EquippedItem) -> u32 {
    let Some(base) = data::get_item(&item.base_id) else {
        return 0;
    };
    if base.rarity != "common" && base.max_sockets == Some(0) {
        return 0;
    }
    let bonus = u32::from(
        item.forged_mods
            .iter()
            .any(|m| m.affix_id == "crystal_add_socket"),
    );
    (base.max_sockets.or(base.sockets).unwrap_or(0) + bonus).min(6)
}

pub fn set_socket_count(item: &mut EquippedItem, count: u32) {
    item.socket_count = count.min(max_sockets(item));
    item.socketed.resize(item.socket_count as usize, None);
    item.socket_types
        .resize(item.socket_count as usize, SocketType::Normal);
}

pub fn set_socket(item: &mut EquippedItem, index: usize, id: Option<&str>) -> Result<(), String> {
    if index >= item.socket_count as usize {
        return Err("Socket no longer exists.".into());
    }
    if id.is_some_and(|id| data::get_gem(id).is_none() && data::get_rune(id).is_none()) {
        return Err("Unknown gem or rune.".into());
    }
    item.socketed.resize(item.socket_count as usize, None);
    item.socketed[index] = id.map(str::to_owned);
    Ok(())
}

pub fn set_forge(item: &mut EquippedItem, id: Option<&str>) -> Result<(), String> {
    item.forged_mods = match id {
        None => vec![],
        Some(id) => {
            let crystal = data::data().crystals.get(id).ok_or("Unknown crystal.")?;
            vec![EquippedAffix {
                affix_id: id.into(),
                tier: crystal.tier,
                roll: 1.,
                custom_value: None,
            }]
        }
    };
    set_socket_count(item, item.socket_count);
    Ok(())
}

pub const RELIC_MAX_TIER: u32 = 10;

pub fn is_relic(base: &ItemBase) -> bool {
    base.rarity == "relic"
}

/// Ranged relic stats as `(is_skill_bonus, key, (min, max))`.
fn relic_ranges(base: &ItemBase) -> Vec<(bool, String, (f64, f64))> {
    let implicit = base
        .implicit
        .iter()
        .flatten()
        .map(|(k, v)| (false, k, v.as_ranged()));
    let skills = base
        .skill_bonuses
        .iter()
        .flatten()
        .map(|(k, v)| (true, k, v.as_ranged()));
    implicit
        .chain(skills)
        .filter(|(_, _, (lo, hi))| lo != hi)
        .map(|(skill, key, range)| (skill, key.clone(), range))
        .collect()
}

// ponytail: tier interpolates linearly between the data range ends; swap for per-tier tables if the game exposes them.
fn relic_tier_value((lo, hi): (f64, f64), tier: u32) -> f64 {
    let step = (tier.clamp(1, RELIC_MAX_TIER) - 1) as f64 / (RELIC_MAX_TIER - 1) as f64;
    lo + (hi - lo) * step
}

/// Tier 1..=10 read back from the first pinned ranged stat; unpinned relics count as top tier.
pub fn relic_tier(item: &EquippedItem) -> u32 {
    let Some(base) = data::get_item(&item.base_id) else {
        return RELIC_MAX_TIER;
    };
    let mut ranges = relic_ranges(base);
    // Prefer the most precise range: rounded skill ranks can cover several tiers.
    ranges.sort_by(|a, b| {
        (b.2.1 - b.2.0)
            .abs()
            .total_cmp(&(a.2.1 - a.2.0).abs())
            .then(a.1.cmp(&b.1))
    });
    ranges
        .into_iter()
        .find_map(|(skill, key, (lo, hi))| {
            let pins = if skill {
                &item.skill_bonus_overrides
            } else {
                &item.implicit_overrides
            };
            let value = *pins.get(&key)?;
            value.is_finite().then(|| {
                (((value - lo) / (hi - lo) * (RELIC_MAX_TIER - 1) as f64).round() + 1.)
                    .clamp(1., RELIC_MAX_TIER as f64) as u32
            })
        })
        .unwrap_or(RELIC_MAX_TIER)
}

/// Pins every ranged relic stat to the given tier; returns whether anything changed.
pub fn set_relic_tier(item: &mut EquippedItem, tier: u32) -> bool {
    let Some(base) = data::get_item(&item.base_id) else {
        return false;
    };
    let mut changed = false;
    for (skill, key, range) in relic_ranges(base) {
        let value = relic_tier_value(range, tier);
        let value = if skill { value.round() } else { value };
        let pins = if skill {
            &mut item.skill_bonus_overrides
        } else {
            &mut item.implicit_overrides
        };
        changed |= pins.insert(key, value) != Some(value);
    }
    changed
}

/// Only common items and items with a random affix pool take affixes; relics and uniques never do.
pub fn accepts_affixes(base: &ItemBase) -> bool {
    base.rarity == "common" || base.random_affix_group_id.is_some()
}

pub fn add_affix(item: &mut EquippedItem, id: &str) -> Result<(), String> {
    add_affix_with_pool_override(item, id, false)
}

/// Mirrors the reference's explicit "Show all affixes" selection. It never permits
/// socketable affixes or affixes from another item's random pool.
pub fn add_affix_with_pool_override(
    item: &mut EquippedItem,
    id: &str,
    allow_outside_pool: bool,
) -> Result<(), String> {
    let base = data::get_item(&item.base_id).ok_or("Unknown item.")?;
    if base
        .max_affixes
        .is_some_and(|max| item.affixes.len() >= max as usize)
    {
        return Err("This item has no free affix slots.".into());
    }
    let affix = data::get_affix(id).ok_or("Unknown affix.")?;
    if !affix_allowed(base, affix, allow_outside_pool) {
        return Err("This affix is not available for the selected item.".into());
    }
    item.affixes.push(EquippedAffix {
        affix_id: id.into(),
        tier: affix.tier,
        roll: 1.,
        custom_value: None,
    });
    Ok(())
}

pub fn apply_runeword(item: &mut EquippedItem, id: &str) -> Result<(), String> {
    let base = data::get_item(&item.base_id).ok_or("Unknown item.")?;
    let rw = data::data()
        .runewords
        .iter()
        .find(|rw| rw.id == id)
        .ok_or("Unknown runeword.")?;
    // The reference applies runewords using the base capacity, without a crystal bonus.
    let bare = EquippedItem {
        base_id: item.base_id.clone(),
        ..Default::default()
    };
    if base.rarity != "common"
        || !rw.allowed_base_types.contains(&base.base_type)
        || rw.runes.len() > max_sockets(&bare) as usize
    {
        return Err("This runeword does not fit the selected base.".into());
    }
    item.socket_count = rw.runes.len() as u32;
    item.socketed = rw.runes.iter().cloned().map(Some).collect();
    item.socket_types.resize(rw.runes.len(), SocketType::Normal);
    Ok(())
}

pub fn set_augment(item: &mut EquippedItem, id: Option<&str>) -> Result<(), String> {
    item.augment = match id {
        None => None,
        Some(id) => {
            data::get_augment(id).ok_or("Unknown augment.")?;
            Some(AugmentRef {
                id: id.into(),
                level: item
                    .augment
                    .as_ref()
                    .filter(|a| a.id == id)
                    .map_or(1, |a| a.level),
            })
        }
    };
    Ok(())
}

pub fn slot_group(slot: &str) -> &str {
    slot.rsplit_once('_')
        .filter(|(_, suffix)| suffix.parse::<u32>().is_ok())
        .map_or(slot, |(group, _)| group)
}

pub fn accepts(snapshot: &BuildSnapshot, slot: &str, base: &ItemBase, mercenary: bool) -> bool {
    if !data::game_config()
        .slots
        .as_ref()
        .is_some_and(|slots| slots.iter().any(|s| s.key == slot))
    {
        return false;
    }
    if mercenary {
        if !hsplanner_engine::calc::mercenary::data()
            .slots
            .iter()
            .any(|key| key == slot)
        {
            return false;
        }
        if slot == "offhand" {
            return base.slot == "offhand"
                && !snapshot
                    .merc_inventory
                    .get("weapon")
                    .and_then(|item| data::get_item(&item.base_id))
                    .is_some_and(|base| base.two_handed.unwrap_or(false));
        }
    } else if slot == "offhand" {
        return snapshot.can_offhand(base);
    }
    slot_group(slot) == slot_group(&base.slot)
}

pub fn commit(
    snapshot: &mut BuildSnapshot,
    slot: &str,
    item: Option<EquippedItem>,
    mercenary: bool,
    extra_charm_slot: bool,
) -> Result<(), String> {
    if let Some(item) = &item {
        let base = data::get_item(&item.base_id).ok_or("Unknown item.")?;
        if !accepts(snapshot, slot, base, mercenary) {
            return Err("This item cannot be equipped in the selected slot.".into());
        }
        if !mercenary
            && slot.starts_with("charm_")
            && !charm_fits(snapshot, slot, base, extra_charm_slot)
        {
            return Err("This charm will not fit. Free up space first.".into());
        }
    }
    let inventory = if mercenary {
        &mut snapshot.merc_inventory
    } else {
        &mut snapshot.inventory
    };
    if let Some(mut item) = item {
        let base = data::get_item(&item.base_id).unwrap();
        if !mercenary && base.rarity != "common" {
            let count = item.socket_count;
            set_socket_count(&mut item, count);
        }
        if mercenary && slot == "weapon" && base.two_handed.unwrap_or(false) {
            inventory.remove("offhand");
        }
        inventory.insert(slot.into(), item);
    } else {
        inventory.remove(slot);
    }
    if !mercenary {
        snapshot.validate_offhand();
    }
    Ok(())
}

pub fn stash(draft: &mut Draft, item: &EquippedItem) {
    if data::get_item(&item.base_id).is_none_or(is_relic) {
        return;
    }
    let value = serde_json::to_value(item).unwrap();
    if draft
        .stash
        .iter()
        .any(|entry| serde_json::to_value(&entry.item).unwrap() == value)
    {
        return;
    }
    draft.stash.insert(
        0,
        StashEntry {
            id: new_id("stash"),
            saved_at: chrono::Utc::now().timestamp_millis().max(0) as u64,
            item: item.clone(),
        },
    );
    draft.stash.truncate(200);
}

fn charm_fits(snapshot: &BuildSnapshot, slot: &str, base: &ItemBase, extra: bool) -> bool {
    let mut charms: Vec<_> = snapshot
        .inventory
        .iter()
        .filter(|(key, _)| key.starts_with("charm_") && key.as_str() != slot)
        .map(|(key, item)| {
            let base = data::get_item(&item.base_id);
            (
                key.clone(),
                base.and_then(|b| b.width).unwrap_or(1),
                base.and_then(|b| b.height).unwrap_or(1),
            )
        })
        .collect();
    let before = pack_charms(&charms, extra);
    charms.push((
        slot.into(),
        base.width.unwrap_or(1),
        base.height.unwrap_or(1),
    ));
    let after = pack_charms(&charms, extra);
    !after.iter().any(|key| key == slot) && after.len() <= before.len()
}

/// A charm's packed position, in the same 3 × 11 inventory used by the reference.
#[derive(Clone, Debug)]
pub struct CharmPlacement {
    slot: String,
    row: u32,
    col: u32,
    width: u32,
    height: u32,
}
impl CharmPlacement {
    pub fn slot(&self) -> &str {
        &self.slot
    }
    pub fn row(&self) -> u32 {
        self.row
    }
    pub fn col(&self) -> u32 {
        self.col
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
}
/// Presentation and commit validation share this packing result.
pub struct CharmLayout {
    placements: Vec<CharmPlacement>,
    overflow: Vec<String>,
    occupied: u64,
}
impl CharmLayout {
    pub fn placements(&self) -> &[CharmPlacement] {
        &self.placements
    }
    pub fn overflow(&self) -> &[String] {
        &self.overflow
    }
    pub fn occupied_cells(&self) -> u64 {
        self.occupied
    }
}

/// Return overflow IDs using the reference's exact packing order and fallback.
pub fn pack_charms(charms: &[(String, u32, u32)], extra: bool) -> Vec<String> {
    charm_layout(charms, extra).overflow
}

/// Calculate positions without changing the commit validator's ordering or fallback.
pub fn charm_layout(charms: &[(String, u32, u32)], extra: bool) -> CharmLayout {
    let mut sorted = charms.to_vec();
    sorted.sort_by(|a, b| {
        (b.1 * b.2)
            .cmp(&(a.1 * a.2))
            .then(b.2.cmp(&a.2))
            .then(a.0.cmp(&b.0))
    });
    let blocked = (1u64 << 9) | (1u64 << 11) | (1u64 << 23) | if extra { 0 } else { 1u64 << 21 };
    fn masks(w: u32, h: u32) -> Vec<u64> {
        if w == 0 || h == 0 || w > 3 || h > 11 {
            return vec![];
        }
        (0..=11 - h)
            .flat_map(|r| {
                (0..=3 - w).map(move |c| {
                    (0..h)
                        .flat_map(|dr| (0..w).map(move |dc| 1u64 << ((r + dr) * 3 + c + dc)))
                        .fold(0, |a, b| a | b)
                })
            })
            .collect()
    }
    let options: Vec<_> = sorted.iter().map(|(_, w, h)| masks(*w, *h)).collect();
    fn fit(
        index: usize,
        occupied: u64,
        sorted: &[(String, u32, u32)],
        options: &[Vec<u64>],
        failed: &mut std::collections::HashSet<(usize, u64)>,
        positions: &mut [u64],
    ) -> bool {
        if index == sorted.len() {
            return true;
        }
        if sorted[index..].iter().all(|(_, w, h)| *w == 1 && *h == 1) {
            if 33 - (occupied.count_ones() as usize) < sorted.len() - index {
                return false;
            }
            let mut available = (0..33).filter(|cell| occupied & (1u64 << cell) == 0);
            for position in &mut positions[index..] {
                *position = 1u64 << available.next().unwrap();
            }
            return true;
        }
        if !failed.insert((index, occupied)) {
            return false;
        }
        options[index].iter().any(|mask| {
            if occupied & mask != 0 {
                return false;
            }
            positions[index] = *mask;
            fit(
                index + 1,
                occupied | mask,
                sorted,
                options,
                failed,
                positions,
            )
        })
    }
    let area: u32 = sorted.iter().map(|(_, w, h)| w * h).sum();
    let mut positions = vec![0; sorted.len()];
    let complete = area <= 33 - blocked.count_ones()
        && fit(
            0,
            blocked,
            &sorted,
            &options,
            &mut Default::default(),
            &mut positions,
        );
    let mut occupied = blocked;
    if !complete {
        positions.fill(0);
        for (i, options) in options.iter().enumerate() {
            if let Some(mask) = options.iter().find(|mask| occupied & *mask == 0) {
                occupied |= mask;
                positions[i] = *mask;
            }
        }
    }
    let mut placements = Vec::new();
    let mut overflow = Vec::new();
    for ((slot, width, height), mask) in sorted.into_iter().zip(positions) {
        if mask == 0 {
            overflow.push(slot);
            continue;
        }
        occupied |= mask;
        let cell = mask.trailing_zeros();
        placements.push(CharmPlacement {
            slot,
            width,
            height,
            row: cell / 3,
            col: cell % 3,
        });
    }
    CharmLayout {
        placements,
        overflow,
        occupied,
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn positions_preserve_reference_order_and_blocked_cells() {
        let layout = charm_layout(
            &[
                ("charm_3".into(), 1, 1),
                ("charm_2".into(), 1, 3),
                ("charm_1".into(), 2, 3),
            ],
            false,
        );
        let positions = layout
            .placements()
            .iter()
            .map(|p| (p.slot(), p.row(), p.col()))
            .collect::<Vec<_>>();
        assert_eq!(
            positions,
            [("charm_1", 0, 0), ("charm_2", 0, 2), ("charm_3", 3, 1)]
        );
        assert!(layout.overflow().is_empty());
        assert_eq!(layout.occupied_cells().count_ones(), 14);
    }

    #[test]
    fn full_inventory_positions_never_overlap_or_use_locked_cell() {
        let charms = (1..=30)
            .map(|n| (format!("charm_{n}"), 1, 1))
            .collect::<Vec<_>>();
        for extra in [false, true] {
            let layout = charm_layout(&charms, extra);
            let mut occupied =
                (1u64 << 9) | (1u64 << 11) | (1u64 << 23) | if extra { 0 } else { 1u64 << 21 };
            for p in layout.placements() {
                let cell = 1u64 << (p.row() * 3 + p.col());
                assert_eq!(
                    occupied & cell,
                    0,
                    "{} overlaps another charm or a locked cell",
                    p.slot()
                );
                occupied |= cell;
            }
            assert_eq!(occupied, layout.occupied_cells());
            assert_eq!(layout.placements().len(), if extra { 30 } else { 29 });
            assert_eq!(layout.overflow().len(), usize::from(!extra));
        }
    }
}
