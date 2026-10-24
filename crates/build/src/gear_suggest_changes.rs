//! What a suggested step changes on its item, for the step list.
use hsplanner_engine::calc::types::EquippedItem;

use super::{Choice, GearStep, runeword_of};
use crate::gear;

#[derive(Debug, Clone, PartialEq)]
pub enum GearChange {
    Item {
        from: Option<String>,
        to: String,
    },
    Stars {
        from: u32,
        to: u32,
    },
    Choice(Choice, String),
    Affix {
        from: Option<String>,
        to: Option<String>,
    },
    Crystal {
        from: Option<String>,
        to: Option<String>,
    },
    Runeword(String),
    Socket {
        index: usize,
        from: Option<String>,
        to: Option<String>,
    },
    Augment {
        from: Option<(String, u32)>,
        to: Option<(String, u32)>,
    },
}

impl GearStep {
    /// What changes, against the old item or a fresh copy of a new base.
    pub fn changes(&self) -> Vec<GearChange> {
        let mut out = Vec::new();
        let reference = match self
            .before
            .as_ref()
            .filter(|b| b.base_id == self.item.base_id)
        {
            Some(before) => before.clone(),
            None => {
                out.push(GearChange::Item {
                    from: self.before.as_ref().map(|b| b.base_id.clone()),
                    to: self.item.base_id.clone(),
                });
                gear::make_item(&self.item.base_id).unwrap_or_default()
            }
        };
        diff(&reference, &self.item, &mut out);
        out
    }
}

fn diff(from: &EquippedItem, to: &EquippedItem, out: &mut Vec<GearChange>) {
    let stars = (from.stars.unwrap_or(0), to.stars.unwrap_or(0));
    if stars.0 != stars.1 {
        out.push(GearChange::Stars {
            from: stars.0,
            to: stars.1,
        });
    }
    for (choice, old, new) in [
        (
            Choice::Element,
            &from.random_skill_element,
            &to.random_skill_element,
        ),
        (
            Choice::RandomSkill,
            &from.random_skill_id,
            &to.random_skill_id,
        ),
        (
            Choice::Subskill,
            &from.subskill_boost_skill_id,
            &to.subskill_boost_skill_id,
        ),
        (
            Choice::Class,
            &from.all_skills_class_id,
            &to.all_skills_class_id,
        ),
    ] {
        if old != new
            && let Some(value) = new
        {
            out.push(GearChange::Choice(choice, value.clone()));
        }
    }
    let ids = |item: &EquippedItem| -> Vec<String> {
        item.affixes.iter().map(|a| a.affix_id.clone()).collect()
    };
    let (old, new) = (ids(from), ids(to));
    let removed: Vec<&String> = old.iter().filter(|id| !new.contains(id)).collect();
    let added: Vec<&String> = new.iter().filter(|id| !old.contains(id)).collect();
    for index in 0..removed.len().max(added.len()) {
        out.push(GearChange::Affix {
            from: removed.get(index).map(|id| (*id).clone()),
            to: added.get(index).map(|id| (*id).clone()),
        });
    }
    let crystal = |item: &EquippedItem| item.forged_mods.first().map(|m| m.affix_id.clone());
    if crystal(from) != crystal(to) {
        out.push(GearChange::Crystal {
            from: crystal(from),
            to: crystal(to),
        });
    }
    match runeword_of(to).filter(|id| runeword_of(from).as_ref() != Some(id)) {
        Some(id) => out.push(GearChange::Runeword(id)),
        None => {
            for index in 0..from.socketed.len().max(to.socketed.len()) {
                let socket = |item: &EquippedItem| item.socketed.get(index).cloned().flatten();
                if socket(from) != socket(to) {
                    out.push(GearChange::Socket {
                        index,
                        from: socket(from),
                        to: socket(to),
                    });
                }
            }
        }
    }
    let augment = |item: &EquippedItem| item.augment.as_ref().map(|a| (a.id.clone(), a.level));
    if augment(from) != augment(to) {
        out.push(GearChange::Augment {
            from: augment(from),
            to: augment(to),
        });
    }
}
