//! The gear tab laid out like the planner: an equipment doll beside the charm inventory.
use std::fmt::Write;

use hsplanner_build::{BuildSnapshot, gear::charm_layout};
use hsplanner_engine::calc::{data, types::EquippedItem};

use crate::{assets, gear, html::esc, panels::panel};

const RELIC_SLOTS: u32 = 5;
const POTION_SLOTS: u32 = 4;
const CHARM_COLUMNS: u32 = 3;
const CHARM_CELLS: u32 = 33;
/// Cells the game's charm bag never offers.
const BLOCKED_CHARM_CELLS: [u32; 3] = [9, 11, 23];

/// An equipped slot; its position in the board's list pairs its cell with its detail card.
struct Worn {
    key: &'static str,
    name: &'static str,
    item: EquippedItem,
}

struct Board<'a> {
    snapshot: &'a BuildSnapshot,
    worn: Vec<Worn>,
}

/// `relic_3` → `relic`: the name shared by numbered slots of one size.
fn kind(key: &str) -> &str {
    match key.rsplit_once('_') {
        Some((kind, number)) if number.parse::<u32>().is_ok() => kind,
        _ => key,
    }
}

fn slot_name(key: &str) -> &'static str {
    let mut slots = data::game_config().slots.iter().flatten();
    slots
        .find(|slot| slot.key == key)
        .map_or("", |slot| slot.name.as_str())
}

impl Board<'_> {
    fn index(&self, key: &str) -> Option<usize> {
        self.worn.iter().position(|worn| worn.key == key)
    }

    /// The cell of an equipped item: its sprite, stars and socket fill.
    fn cell(&self, index: usize, classes: &str) -> String {
        let Worn {
            name: slot, item, ..
        } = &self.worn[index];
        let base = data::get_item(&item.base_id);
        let rarity = base.map_or_else(String::new, |base| gear::rarity_class(&base.rarity));
        let name = esc(gear::item_name(item));
        let sprite = base
            .map(|base| format!("items/{}.png", base.id))
            .filter(|path| assets::game(path).is_some());
        let mut face = match sprite {
            Some(path) => format!(
                "<img class=\"pixel\" src=\"/assets/{}\" alt=\"\">",
                esc(&path)
            ),
            None => format!("<span class=\"slot-label\">{name}</span>"),
        };
        if let Some(stars) = item.stars.filter(|stars| *stars > 0) {
            let _ = write!(face, "<span class=\"slot-stars\">★{stars}</span>");
        }
        if item.socket_count > 0 {
            let _ = write!(
                face,
                "<span class=\"slot-sockets\">{}/{}◇</span>",
                item.socketed.iter().flatten().count(),
                item.socket_count
            );
        }
        format!(
            "<button type=\"button\" class=\"slot {classes}{rarity}\" data-pick=\"{index}\" \
aria-label=\"{}: {name}\">{face}</button>",
            esc(slot)
        )
    }

    fn offhand_locked(&self) -> bool {
        let weapon = self.snapshot.inventory.get("weapon");
        weapon
            .and_then(|item| data::get_item(&item.base_id))
            .is_some_and(|base| {
                base.two_handed.unwrap_or(false) && !self.snapshot.can_offhand(base)
            })
    }

    /// A doll slot: the item it holds, or its dashed placeholder.
    fn slot(&self, key: &str, empty: &str) -> String {
        let kind = kind(key);
        if let Some(index) = self.index(key) {
            return self.cell(index, &format!("slot-{kind}"));
        }
        let (locked, label) = if key == "offhand" && self.offhand_locked() {
            (" locked", "2H")
        } else {
            ("", empty)
        };
        format!(
            "<span class=\"slot slot-{kind} empty{locked}\">{}</span>",
            esc(label)
        )
    }

    fn potion(&self, number: u32) -> String {
        let key = format!("potion_{number}");
        let disabled = self.snapshot.disabled_potions.get(&key).copied();
        let dot = match (self.index(&key), disabled.unwrap_or(false)) {
            (None, _) => "",
            (Some(_), false) => "<span class=\"potion-dot on\" title=\"Effects applied\"></span>",
            (Some(_), true) => "<span class=\"potion-dot\" title=\"Effects off\"></span>",
        };
        format!(
            "<div class=\"potion\">{}{dot}</div>",
            self.slot(&key, &format!("P{number}"))
        )
    }

    /// The planner's paper doll: relics down the left, then four rows of three columns.
    fn doll(&self) -> String {
        let named = |key: &str| self.slot(key, slot_name(key));
        let row = |class: &str, left: &str, middle: &str, right: &str| {
            format!(
                "<div class=\"doll-row{class}\"><div class=\"doll-side\">{left}</div>\
<div class=\"doll-mid\">{middle}</div><div class=\"doll-side\">{right}</div></div>"
            )
        };
        let relics: String = (1..=RELIC_SLOTS)
            .map(|number| self.slot(&format!("relic_{number}"), &format!("R{number}")))
            .collect();
        let potions: String = (1..=POTION_SLOTS)
            .map(|number| self.potion(number))
            .collect();
        let jewelry = [
            self.slot("ring_1", "Ring"),
            named("belt"),
            self.slot("ring_2", "Ring"),
        ]
        .concat();
        [
            format!("<div class=\"doll\"><div class=\"doll-relics\">{relics}</div><div class=\"doll-body\">"),
            row(" end", "", &named("helmet"), &self.slot("amulet", "Amu")),
            row(" tall", &named("weapon"), &named("armor"), &named("offhand")),
            row("", "", &jewelry, ""),
            row("", &named("gloves"), &potions, &named("boots")),
            "</div></div>".to_owned(),
        ]
        .concat()
    }

    /// The 3 × 11 charm bag with charms where the planner packs them, and the cells they use.
    fn charms(&self) -> (String, u32) {
        let sized: Vec<(String, u32, u32)> = self
            .worn
            .iter()
            .filter(|worn| data::is_charm_slot(worn.key))
            .map(|worn| {
                let base = data::get_item(&worn.item.base_id);
                let width = base.and_then(|base| base.width).unwrap_or(1);
                let height = base.and_then(|base| base.height).unwrap_or(1);
                (worn.key.to_owned(), width, height)
            })
            .collect();
        // ponytail: the extra charm cell is an app setting codes do not carry; assume its default.
        let layout = charm_layout(&sized, true);
        let mut grid = String::new();
        for cell in 0..CHARM_CELLS {
            let blocked = BLOCKED_CHARM_CELLS.contains(&cell);
            if layout.occupied_cells() & (1 << cell) != 0 && !blocked {
                continue;
            }
            let _ = write!(
                grid,
                "<span class=\"charm-cell{} gc{} gr{}\"></span>",
                if blocked { " blocked" } else { "" },
                cell % CHARM_COLUMNS + 1,
                cell / CHARM_COLUMNS + 1
            );
        }
        for placed in layout.placements() {
            let Some(index) = self.index(placed.slot()) else {
                continue;
            };
            let classes = format!(
                "slot-charm gc{} gr{} gw{} gh{}",
                placed.col() + 1,
                placed.row() + 1,
                placed.width(),
                placed.height()
            );
            grid.push_str(&self.cell(index, &classes));
        }
        let overflow: String = layout
            .overflow()
            .iter()
            .filter_map(|key| self.index(key))
            .map(|index| {
                format!(
                    "<div class=\"charm-overflow\">{}<span>Does not fit</span></div>",
                    self.cell(index, "slot-relic")
                )
            })
            .collect();
        let used = sized.iter().map(|(_, width, height)| width * height).sum();
        let html = format!(
            "<div class=\"pad\"><div class=\"charm-bag\"><div class=\"charm-grid\">{grid}</div>\
</div>{overflow}</div>"
        );
        (html, used)
    }
}

/// The equipment doll, charm inventory and detail cards, with how many items they hold.
pub fn board(snapshot: &BuildSnapshot) -> (String, usize) {
    let slots = || data::game_config().slots.iter().flatten();
    let worn = slots()
        .filter_map(|slot| {
            Some(Worn {
                key: slot.key.as_str(),
                name: slot.name.as_str(),
                item: snapshot.inventory.get(&slot.key)?.clone(),
            })
        })
        .collect();
    let board = Board { snapshot, worn };
    let is_equipment = |key: &str| !data::is_charm_slot(key);
    let equipped = board
        .worn
        .iter()
        .filter(|worn| is_equipment(worn.key))
        .count();
    let equipment = slots().filter(|slot| is_equipment(&slot.key)).count();
    let (charms, used) = board.charms();
    let cards: String = board
        .worn
        .iter()
        .enumerate()
        .map(|(index, worn)| gear::card(index, worn.name, &worn.item))
        .collect();
    let html = format!(
        "<div class=\"picker board\" data-hover><div class=\"board-main\">{}<div class=\"pick-panes\">{cards}\
</div></div><div class=\"board-side\">{}</div></div>",
        panel(
            "Equipment",
            &format!("{equipped} / {equipment} equipped"),
            &format!("<div class=\"pad\">{}</div>", board.doll())
        ),
        panel(
            "Charm inventory",
            &format!(
                "{used} / {}",
                CHARM_CELLS as usize - BLOCKED_CHARM_CELLS.len()
            ),
            &charms
        )
    );
    (html, board.worn.len())
}

#[cfg(test)]
mod tests {
    use hsplanner_engine::calc::types::EquippedItem;

    use super::*;

    fn build(items: &[(&str, &str)]) -> BuildSnapshot {
        let inventory = items
            .iter()
            .map(|(slot, base)| {
                let item: EquippedItem =
                    serde_json::from_value(serde_json::json!({ "baseId": base })).unwrap();
                ((*slot).to_owned(), item)
            })
            .collect();
        BuildSnapshot {
            inventory,
            ..BuildSnapshot::default()
        }
    }

    #[test]
    fn worn_items_fill_their_doll_slots_and_the_rest_stay_labelled() {
        let snapshot = build(&[("helmet", "base_mace_ogre_maul")]);

        let (html, count) = board(&snapshot);

        assert_eq!(count, 1);
        assert!(html.contains("1 / 19 equipped"));
        assert!(html.contains("class=\"slot slot-helmet rarity-common\" data-pick=\"0\""));
        assert!(html.contains("<span class=\"slot slot-weapon empty\">Weapon</span>"));
        assert!(html.contains("<span class=\"slot slot-relic empty\">R5</span>"));
        assert!(html.contains("<span class=\"slot slot-potion empty\">P4</span>"));
        assert_eq!(html.matches("data-pane=").count(), 1);
        assert!(html.contains("<div class=\"picker board\" data-hover>"));
    }

    #[test]
    fn a_two_handed_weapon_locks_the_offhand() {
        let snapshot = build(&[("weapon", "base_mace_ogre_maul")]);

        let (html, _) = board(&snapshot);

        assert!(html.contains("<span class=\"slot slot-offhand empty locked\">2H</span>"));
    }

    #[test]
    fn charms_take_the_cells_the_planner_packs_them_into() {
        let snapshot = build(&[("charm_3", "charm_angelic_air_melon")]);

        let (html, count) = board(&snapshot);

        assert_eq!(count, 1);
        assert!(html.contains("4 / 30"));
        assert!(html.contains("slot slot-charm gc1 gr1 gw2 gh2 rarity-angelic"));
        assert_eq!(html.matches("class=\"charm-cell blocked").count(), 3);
        assert_eq!(html.matches("class=\"charm-cell gc").count(), 26);
    }

    #[test]
    fn a_switched_off_potion_shows_a_hollow_dot() {
        let mut snapshot = build(&[("potion_1", "base_mace_ogre_maul")]);
        assert!(board(&snapshot).0.contains("class=\"potion-dot on\""));

        snapshot.disabled_potions.insert("potion_1".into(), true);

        assert!(board(&snapshot).0.contains("class=\"potion-dot\""));
    }
}
