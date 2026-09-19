//! Independently selected parts of a build. The draft snapshot is the composed,
//! editable view; every session edit captures its four scoped parts here.
use std::collections::{HashMap, HashSet};

use hsplanner_engine::calc::types::{Inventory, TreeSocketContent};
use serde::{Deserialize, Serialize};

use crate::{
    BuildSnapshot,
    library::{clean_name, new_id},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadoutKind {
    Incarnation,
    Ether,
    Gear,
    Skills,
}

impl LoadoutKind {
    pub const ALL: [Self; 4] = [Self::Incarnation, Self::Ether, Self::Gear, Self::Skills];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry<T> {
    id: String,
    name: String,
    value: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Collection<T> {
    active_id: String,
    entries: Vec<Entry<T>>,
}

impl<T: Clone> Collection<T> {
    fn new(value: T) -> Self {
        Self {
            active_id: "default".into(),
            entries: vec![Entry {
                id: "default".into(),
                name: "Default".into(),
                value,
            }],
        }
    }
    fn active(&self) -> &Entry<T> {
        self.entries
            .iter()
            .find(|entry| entry.id == self.active_id)
            .expect("validated loadout selection")
    }
    fn capture(&mut self, value: T) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == self.active_id)
        {
            entry.value = value;
        }
    }
    fn duplicate(&mut self, name: &str) -> Result<(), String> {
        let name = clean_name(name)?;
        if self.entries.len() >= 100 {
            return Err("This category already contains 100 loadouts.".into());
        }
        let entry = Entry {
            id: new_id("l"),
            name,
            value: self.active().value.clone(),
        };
        self.active_id = entry.id.clone();
        self.entries.push(entry);
        Ok(())
    }
    fn select(&mut self, id: &str) -> Result<(), String> {
        if !self.entries.iter().any(|entry| entry.id == id) {
            return Err("Loadout no longer exists.".into());
        }
        self.active_id = id.into();
        Ok(())
    }
    fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        let name = clean_name(name)?;
        self.entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or("Loadout no longer exists.")?
            .name = name;
        Ok(())
    }
    fn remove(&mut self, id: &str) -> Result<(), String> {
        if !self.entries.iter().any(|entry| entry.id == id) {
            return Err("Loadout no longer exists.".into());
        }
        if self.entries.len() <= 1 {
            return Err("Keep at least one loadout in each category.".into());
        }
        self.entries.retain(|entry| entry.id != id);
        if self.active_id == id {
            self.active_id = self.entries[0].id.clone();
        }
        Ok(())
    }
    fn validate(&self) -> Result<(), String> {
        let mut ids = HashSet::new();
        if self.entries.is_empty()
            || self.entries.len() > 100
            || !self.entries.iter().any(|entry| entry.id == self.active_id)
            || self.entries.iter().any(|entry| {
                entry.id.is_empty()
                    || entry.id.len() > 500
                    || !ids.insert(&entry.id)
                    || entry.name.trim().is_empty()
                    || entry.name.chars().count() > 500
            })
        {
            return Err("Invalid loadout collection.".into());
        }
        Ok(())
    }
    fn merge_missing(&mut self, incoming: Self) {
        for entry in incoming.entries {
            if !self.entries.iter().any(|current| current.id == entry.id) {
                self.entries.push(entry);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Incarnation {
    nodes: Vec<u32>,
    sockets: HashMap<u32, TreeSocketContent>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Ether {
    nodes: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Gear {
    inventory: Inventory,
    merc_inventory: Inventory,
    #[serde(default)]
    disabled_potions: HashMap<String, bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Skills {
    ranks: HashMap<String, u32>,
    subskill_ranks: HashMap<String, u32>,
    max_subskill_points: u32,
    active_ids: Vec<String>,
    aura_id: Option<String>,
    #[serde(default)]
    active_buffs: HashMap<String, bool>,
}

/// Four independently selected collections. Character class, level, attributes,
/// combat configuration, mercenary skills, notes and stash belong to the build.
/// Gear includes character and mercenary equipment plus potion activation.
/// Skills includes ranks, subskills, point budget, selected skills/aura and buffs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loadouts {
    incarnation: Collection<Incarnation>,
    ether: Collection<Ether>,
    gear: Collection<Gear>,
    skills: Collection<Skills>,
}

macro_rules! with_collection {
    ($self:expr, $kind:expr, $collection:ident, $body:expr) => {
        match $kind {
            LoadoutKind::Incarnation => {
                let $collection = &$self.incarnation;
                $body
            }
            LoadoutKind::Ether => {
                let $collection = &$self.ether;
                $body
            }
            LoadoutKind::Gear => {
                let $collection = &$self.gear;
                $body
            }
            LoadoutKind::Skills => {
                let $collection = &$self.skills;
                $body
            }
        }
    };
}
macro_rules! with_collection_mut {
    ($self:expr, $kind:expr, $collection:ident, $body:expr) => {
        match $kind {
            LoadoutKind::Incarnation => {
                let $collection = &mut $self.incarnation;
                $body
            }
            LoadoutKind::Ether => {
                let $collection = &mut $self.ether;
                $body
            }
            LoadoutKind::Gear => {
                let $collection = &mut $self.gear;
                $body
            }
            LoadoutKind::Skills => {
                let $collection = &mut $self.skills;
                $body
            }
        }
    };
}

impl Default for Loadouts {
    fn default() -> Self {
        Self::from_snapshot(&BuildSnapshot::default())
    }
}

impl Loadouts {
    pub fn from_snapshot(snapshot: &BuildSnapshot) -> Self {
        Self {
            incarnation: Collection::new(Incarnation::from(snapshot)),
            ether: Collection::new(Ether::from(snapshot)),
            gear: Collection::new(Gear::from(snapshot)),
            skills: Collection::new(Skills::from(snapshot)),
        }
    }
    pub fn entries(&self, kind: LoadoutKind) -> Vec<(&str, &str)> {
        with_collection!(
            self,
            kind,
            collection,
            collection
                .entries
                .iter()
                .map(|entry| (entry.id.as_str(), entry.name.as_str()))
                .collect()
        )
    }
    pub fn active_id(&self, kind: LoadoutKind) -> &str {
        with_collection!(self, kind, collection, collection.active_id.as_str())
    }
    pub fn active_name(&self, kind: LoadoutKind) -> &str {
        with_collection!(self, kind, collection, collection.active().name.as_str())
    }
    pub fn count(&self, kind: LoadoutKind) -> usize {
        with_collection!(self, kind, collection, collection.entries.len())
    }
    pub fn validate(&self) -> Result<(), String> {
        for kind in LoadoutKind::ALL {
            with_collection!(self, kind, collection, collection.validate())?;
        }
        if self
            .incarnation
            .entries
            .iter()
            .any(|entry| entry.value.nodes.len() > 10_000 || entry.value.sockets.len() > 10_000)
            || self
                .ether
                .entries
                .iter()
                .any(|entry| entry.value.nodes.len() > 10_000)
            || self.gear.entries.iter().any(|entry| {
                entry.value.inventory.len() > 5_000 || entry.value.merc_inventory.len() > 5_000
            })
            || self
                .skills
                .entries
                .iter()
                .any(|entry| !(1..=30).contains(&entry.value.max_subskill_points))
        {
            return Err("Invalid or oversized loadout contents.".into());
        }
        Ok(())
    }
    pub(crate) fn capture_active(&mut self, snapshot: &BuildSnapshot) {
        self.incarnation.capture(Incarnation::from(snapshot));
        self.ether.capture(Ether::from(snapshot));
        self.gear.capture(Gear::from(snapshot));
        self.skills.capture(Skills::from(snapshot));
    }
    pub(crate) fn apply(&self, kind: LoadoutKind, snapshot: &mut BuildSnapshot) {
        match kind {
            LoadoutKind::Incarnation => {
                let value = &self.incarnation.active().value;
                snapshot.allocated_tree_nodes = value.nodes.clone();
                snapshot.tree_socketed = value.sockets.clone();
            }
            LoadoutKind::Ether => {
                snapshot.allocated_ether_nodes = self.ether.active().value.nodes.clone()
            }
            LoadoutKind::Gear => {
                let value = &self.gear.active().value;
                snapshot.inventory = value.inventory.clone();
                snapshot.merc_inventory = value.merc_inventory.clone();
                snapshot.disabled_potions = value.disabled_potions.clone();
            }
            LoadoutKind::Skills => {
                let value = &self.skills.active().value;
                snapshot.skill_ranks = value.ranks.clone();
                snapshot.subskill_ranks = value.subskill_ranks.clone();
                snapshot.max_subskill_points = value.max_subskill_points;
                snapshot.active_skill_ids = value.active_ids.clone();
                snapshot.active_aura_id = value.aura_id.clone();
                snapshot.active_buffs = value.active_buffs.clone();
            }
        }
    }
    pub(crate) fn apply_active(&self, snapshot: &mut BuildSnapshot) {
        for kind in LoadoutKind::ALL {
            self.apply(kind, snapshot);
        }
    }
    pub(crate) fn duplicate(&mut self, kind: LoadoutKind, name: &str) -> Result<(), String> {
        with_collection_mut!(self, kind, collection, collection.duplicate(name))
    }
    pub(crate) fn select(&mut self, kind: LoadoutKind, id: &str) -> Result<(), String> {
        with_collection_mut!(self, kind, collection, collection.select(id))
    }
    pub(crate) fn rename(&mut self, kind: LoadoutKind, id: &str, name: &str) -> Result<(), String> {
        with_collection_mut!(self, kind, collection, collection.rename(id, name))
    }
    pub(crate) fn remove(&mut self, kind: LoadoutKind, id: &str) -> Result<(), String> {
        with_collection_mut!(self, kind, collection, collection.remove(id))
    }
    pub(crate) fn class_changed(&mut self) {
        for entry in &mut self.skills.entries {
            entry.value.ranks.clear();
            entry.value.subskill_ranks.clear();
            entry.value.active_ids.clear();
            entry.value.aura_id = None;
            entry.value.active_buffs.clear();
        }
        for entry in &mut self.incarnation.entries {
            entry.value.sockets.clear();
        }
    }
    pub(crate) fn clear_trees(&mut self) {
        for entry in &mut self.incarnation.entries {
            entry.value.nodes.clear();
            entry.value.sockets.clear();
        }
        for entry in &mut self.ether.entries {
            entry.value.nodes.clear();
        }
    }
    pub(crate) fn merge_missing(&mut self, incoming: Self) {
        self.incarnation.merge_missing(incoming.incarnation);
        self.ether.merge_missing(incoming.ether);
        self.gear.merge_missing(incoming.gear);
        self.skills.merge_missing(incoming.skills);
    }
    pub(crate) fn from_legacy(
        profiles: &[(String, String, BuildSnapshot)],
        active_id: &str,
    ) -> Result<Self, String> {
        fn collection<T>(
            profiles: &[(String, String, BuildSnapshot)],
            active_id: &str,
            value: impl Fn(&BuildSnapshot) -> T,
        ) -> Collection<T> {
            Collection {
                active_id: active_id.into(),
                entries: profiles
                    .iter()
                    .map(|(id, name, snapshot)| Entry {
                        id: id.clone(),
                        name: name.clone(),
                        value: value(snapshot),
                    })
                    .collect(),
            }
        }
        let loadouts = Self {
            incarnation: collection(profiles, active_id, |snapshot| Incarnation::from(snapshot)),
            ether: collection(profiles, active_id, |snapshot| Ether::from(snapshot)),
            gear: collection(profiles, active_id, |snapshot| Gear::from(snapshot)),
            skills: collection(profiles, active_id, |snapshot| Skills::from(snapshot)),
        };
        loadouts.validate()?;
        Ok(loadouts)
    }
}

impl From<&BuildSnapshot> for Incarnation {
    fn from(snapshot: &BuildSnapshot) -> Self {
        Self {
            nodes: snapshot.allocated_tree_nodes.clone(),
            sockets: snapshot.tree_socketed.clone(),
        }
    }
}
impl From<&BuildSnapshot> for Ether {
    fn from(snapshot: &BuildSnapshot) -> Self {
        Self {
            nodes: snapshot.allocated_ether_nodes.clone(),
        }
    }
}
impl From<&BuildSnapshot> for Gear {
    fn from(snapshot: &BuildSnapshot) -> Self {
        Self {
            inventory: snapshot.inventory.clone(),
            merc_inventory: snapshot.merc_inventory.clone(),
            disabled_potions: snapshot.disabled_potions.clone(),
        }
    }
}
impl From<&BuildSnapshot> for Skills {
    fn from(snapshot: &BuildSnapshot) -> Self {
        Self {
            ranks: snapshot.skill_ranks.clone(),
            subskill_ranks: snapshot.subskill_ranks.clone(),
            max_subskill_points: snapshot.subskill_point_budget(),
            active_ids: snapshot.active_skill_ids.clone(),
            aura_id: snapshot.active_aura_id.clone(),
            active_buffs: snapshot.active_buffs.clone(),
        }
    }
}
