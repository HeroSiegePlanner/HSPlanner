//! Gear upgrade search: other items plus sockets, runewords, crystals, augments, random affixes
//! and stars, each kept only when the real calculation says it raises the ranked DPS.
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};

use hsplanner_engine::calc::{
    affix::rolled_affix_value,
    data,
    planner::{DpsSummary, PlannerInput, combined_dps, dps_summary, hit_dps},
    season::SeasonScope,
    skills::Ranged,
    star_scaling,
    types::{
        Affix, AngelicAugment, AugmentCalc, CustomStat, EquippedAffix, EquippedItem, ItemBase,
        StatMap, augment_toggle_key,
    },
};
use hsplanner_engine::suggest_engine::command::suggest_pool;
use hsplanner_engine::task_control::Cancellation;
use rayon::prelude::*;

use crate::{BuildSnapshot, gear, session::Session};

#[path = "gear_suggest_changes.rs"]
mod changes;
pub use changes::GearChange;

/// Slots searched, in order; charms and potions are left alone.
pub const SLOTS: [&str; 15] = [
    "weapon", "offhand", "armor", "helmet", "gloves", "boots", "belt", "amulet", "ring_1",
    "ring_2", "relic_1", "relic_2", "relic_3", "relic_4", "relic_5",
];
/// Full rounds over the slots; the second refines the first with the probes re-measured.
pub const PASSES: usize = 2;
const SHORTLIST: usize = 6;
const VERIFIED: usize = 3;
const MIN_GAIN: f64 = 1e-4;
const ADD_SOCKET: &str = "crystal_add_socket";
const ELEMENTS: [&str; 5] = ["fire", "cold", "lightning", "poison", "arcane"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuggestOptions {
    pub other_items: bool,
    pub sockets: bool,
    pub crystals: bool,
    pub augments: bool,
    pub affixes: bool,
    pub stars: bool,
    pub rank_by: RankBy,
}

impl Default for SuggestOptions {
    fn default() -> Self {
        Self {
            other_items: true,
            sockets: true,
            crystals: true,
            augments: true,
            affixes: true,
            stars: true,
            rank_by: RankBy::CombinedDps,
        }
    }
}

/// The stat panel number the search raises.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RankBy {
    #[default]
    CombinedDps,
    /// The main skill's hits only, so other skills and ailments can't outweigh it.
    HitDps,
}

impl RankBy {
    /// The number's midpoint, as the search compares it.
    fn measure(self, input: &PlannerInput) -> f64 {
        match self {
            Self::CombinedDps => combined_dps(input),
            Self::HitDps => hit_dps(input),
        }
    }

    /// The number among the stat panel's.
    pub fn of(self, summary: &DpsSummary) -> Option<Ranged> {
        match self {
            Self::CombinedDps => summary.combined_dps,
            Self::HitDps => summary.hit_dps,
        }
    }
}

/// One accepted change, measured with every earlier step applied; steps only hold in order.
#[derive(Debug, Clone)]
pub struct GearStep {
    pub slot: String,
    pub pass: usize,
    pub before: Option<EquippedItem>,
    pub item: EquippedItem,
    pub augment_on: bool,
    pub cleared: Vec<String>,
    /// Midpoints of the ranked DPS the search compares.
    pub dps_before: f64,
    pub dps_after: f64,
    /// The stat panel's numbers once this step is applied.
    pub after: DpsSummary,
}

#[derive(Debug, Clone, Default)]
pub struct GearSuggestion {
    pub base_dps: f64,
    pub final_dps: f64,
    pub rank_by: RankBy,
    /// The stat panel's numbers before any step.
    pub before: DpsSummary,
    pub steps: Vec<GearStep>,
    pub evaluations: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Element,
    RandomSkill,
    Subskill,
    Class,
}

impl Choice {
    fn set(self, item: &mut EquippedItem, value: &str) {
        let value = Some(value.to_owned());
        match self {
            Self::Element => item.random_skill_element = value,
            Self::RandomSkill => item.random_skill_id = value,
            Self::Subskill => item.subskill_boost_skill_id = value,
            Self::Class => item.all_skills_class_id = value,
        }
    }
}

impl GearStep {
    /// Saves the step the way the gear editor saves an item.
    pub fn apply(&self, snapshot: &mut BuildSnapshot) -> Result<(), String> {
        equip(snapshot, &self.slot, &self.item, self.augment_on)
    }
}

/// Applies consecutive steps as one undoable edit; nothing changes when one no longer fits.
pub fn apply_steps(session: &mut Session, steps: &[GearStep]) -> Result<(), String> {
    let mut outcome = Ok(());
    session.edit(|draft| {
        let mut next = draft.snapshot.clone();
        outcome = steps.iter().try_for_each(|step| step.apply(&mut next));
        if outcome.is_ok() {
            draft.snapshot = next;
        }
    });
    outcome
}

/// Searches upgrades slot by slot on the shared suggestion workers.
pub fn suggest(
    snapshot: &BuildSnapshot,
    options: SuggestOptions,
    cancellation: &Cancellation,
    progress: impl Fn(u32, u32) + Sync,
) -> Result<GearSuggestion, String> {
    let search = Search {
        start: snapshot,
        options,
        cancellation,
        evaluations: AtomicU32::new(0),
    };
    suggest_pool().install(|| search.run(&progress))
}

fn equip(
    snapshot: &mut BuildSnapshot,
    slot: &str,
    item: &EquippedItem,
    augment_on: bool,
) -> Result<(), String> {
    gear::commit(snapshot, slot, Some(item.clone()), false, false)?;
    let augment = item.augment.as_ref().filter(|_| augment_on);
    let Some(augment) = augment.and_then(|a| data::get_augment(&a.id)) else {
        return Ok(());
    };
    let toggles = match augment.calc {
        AugmentCalc::Toggle => &mut snapshot.player_conditions,
        AugmentCalc::Proc => &mut snapshot.proc_toggles,
        _ => return Ok(()),
    };
    toggles.insert(augment_toggle_key(&augment.id), true);
    Ok(())
}

fn runeword_of(item: &EquippedItem) -> Option<String> {
    let base = data::get_item(&item.base_id)?;
    let socketed: Vec<Option<&str>> = item.socketed.iter().map(Option::as_deref).collect();
    data::detect_runeword(base, &socketed).map(|rw| rw.id.clone())
}

/// Relics worn in the other relic slots; the game never takes the same relic twice.
fn other_relics<'a>(snapshot: &'a BuildSnapshot, slot: &str) -> HashSet<&'a str> {
    snapshot
        .inventory
        .iter()
        .filter(|(key, _)| key.as_str() != slot && gear::slot_group(key) == "relic")
        .map(|(_, item)| item.base_id.as_str())
        .collect()
}

fn gains(dps: f64, than: f64) -> bool {
    dps > than + than.abs() * MIN_GAIN
}

fn better(current: Pick, candidate: Option<Pick>) -> Pick {
    match candidate {
        Some(candidate) if gains(candidate.dps, current.dps) => candidate,
        _ => current,
    }
}

fn adds_socket(item: &EquippedItem) -> bool {
    let mut forged = item.clone();
    gear::set_forge(&mut forged, Some(ADD_SOCKET)).is_ok()
        && gear::max_sockets(&forged) > gear::max_sockets(item)
}

/// The rolls a fresh copy of `base` lets the player pick, with the values worth trying.
fn choices(snapshot: &BuildSnapshot, base: &ItemBase) -> Vec<(Choice, Vec<String>)> {
    let implicit = |key: &str| {
        base.implicit
            .as_ref()
            .is_some_and(|stats| stats.contains_key(key))
    };
    let mut out = Vec::new();
    if implicit("random_skill_element") {
        out.push((Choice::Element, ELEMENTS.map(String::from).to_vec()));
    }
    if let Some(pool) = &base.random_skill_pool {
        let skills = data::get_skills_by_class(&pool.class_id)
            .iter()
            .filter(|skill| skill.tree.as_deref() == Some(pool.tree.as_str()))
            .map(|skill| skill.id.clone())
            .collect();
        out.push((Choice::RandomSkill, skills));
    }
    if implicit("grant_subskills") {
        let mut spent: Vec<String> = snapshot
            .subskill_ranks
            .iter()
            .filter(|(_, rank)| **rank > 0)
            .filter_map(|(key, _)| key.split_once(':').map(|(skill, _)| skill.to_owned()))
            .collect();
        spent.sort();
        spent.dedup();
        out.push((Choice::Subskill, spent));
    }
    if implicit("all_skills_class") {
        out.push((Choice::Class, snapshot.class_id.iter().cloned().collect()));
    }
    out
}

type Stats = Vec<(String, f64)>;

fn stat_list(stats: &StatMap) -> Stats {
    stats
        .iter()
        .map(|(key, value)| (key.clone(), *value))
        .collect()
}

fn affix_stats(affix: &Affix) -> Stats {
    let value = rolled_affix_value(affix, 1.);
    affix
        .stat_key
        .iter()
        .map(|key| (key.clone(), value))
        .collect()
}

struct Search<'a> {
    start: &'a BuildSnapshot,
    options: SuggestOptions,
    cancellation: &'a Cancellation,
    evaluations: AtomicU32,
}

#[derive(Clone)]
struct Pick {
    item: EquippedItem,
    augment_on: bool,
    dps: f64,
}

/// Probe gains of each modification's stats on the build, best first; only gains above zero.
#[derive(Default)]
struct Scores {
    socketables: Vec<(String, f64)>,
    crystals: Vec<(String, f64)>,
    affixes: HashMap<String, Vec<(String, f64)>>,
}

enum Probe<'a> {
    Socketable,
    Crystal,
    Affix(&'a str),
}

impl Search<'_> {
    /// Steps stay in the order they were found, so each one holds in the state it was measured in.
    fn run(&self, progress: &(impl Fn(u32, u32) + Sync)) -> Result<GearSuggestion, String> {
        let _scope = self.scope();
        let total = 1 + (PASSES * SLOTS.len()) as u32;
        progress(0, total);
        let base_dps = self.dps(self.start);
        let mut snapshot = self.start.clone();
        let mut dps = base_dps;
        let mut steps = Vec::new();
        let mut done = 1;
        for pass in 0..PASSES {
            // Each pass measures the probes on the build the earlier changes left.
            let mut scores = self.scores(&snapshot, dps);
            self.cancellation.check()?;
            progress(done, total);
            let found = steps.len();
            for slot in SLOTS {
                let pick = self.best_for(&snapshot, slot, &scores);
                self.cancellation.check()?;
                if let Some(pick) = pick.filter(|pick| gains(pick.dps, dps)) {
                    let mut next = snapshot.clone();
                    equip(&mut next, slot, &pick.item, pick.augment_on)?;
                    let mut cleared: Vec<String> = snapshot
                        .inventory
                        .keys()
                        .filter(|key| !next.inventory.contains_key(*key))
                        .cloned()
                        .collect();
                    cleared.sort();
                    steps.push(GearStep {
                        slot: slot.into(),
                        pass,
                        before: snapshot.inventory.get(slot).cloned(),
                        item: pick.item,
                        augment_on: pick.augment_on,
                        cleared,
                        dps_before: dps,
                        dps_after: pick.dps,
                        after: dps_summary(&next.planner_input()),
                    });
                    snapshot = next;
                    dps = pick.dps;
                    // The weapon decides which stats pay off, so the probes are measured again.
                    if slot == "weapon" {
                        scores = self.scores(&snapshot, dps);
                    }
                }
                done += 1;
                progress(done, total);
            }
            if steps.len() == found {
                break;
            }
        }
        Ok(GearSuggestion {
            base_dps,
            final_dps: dps,
            rank_by: self.options.rank_by,
            before: dps_summary(&self.start.planner_input()),
            steps,
            evaluations: self.evaluations.load(Ordering::Relaxed),
        })
    }

    fn scope(&self) -> SeasonScope {
        SeasonScope::enter(Some(self.start.season.clone()))
    }

    fn dps(&self, snapshot: &BuildSnapshot) -> f64 {
        if self.cancellation.is_cancelled() {
            return 0.;
        }
        self.evaluations.fetch_add(1, Ordering::Relaxed);
        self.options.rank_by.measure(&snapshot.planner_input())
    }

    fn try_item(
        &self,
        snapshot: &BuildSnapshot,
        slot: &str,
        item: EquippedItem,
        augment_on: bool,
    ) -> Option<Pick> {
        let mut next = snapshot.clone();
        equip(&mut next, slot, &item, augment_on).ok()?;
        Some(Pick {
            dps: self.dps(&next),
            item,
            augment_on,
        })
    }

    /// Measures every variant in parallel and keeps the best, the earliest on ties.
    fn best<T: Sync>(
        &self,
        snapshot: &BuildSnapshot,
        slot: &str,
        variants: &[T],
        make: impl Fn(&T) -> Option<(EquippedItem, bool)> + Sync,
    ) -> Option<Pick> {
        let picks: Vec<Option<Pick>> = variants
            .par_iter()
            .map(|variant| {
                let _scope = self.scope();
                let (item, augment_on) = make(variant)?;
                self.try_item(snapshot, slot, item, augment_on)
            })
            .collect();
        picks
            .into_iter()
            .flatten()
            .reduce(|best, pick| if pick.dps > best.dps { pick } else { best })
    }

    fn scores(&self, snapshot: &BuildSnapshot, dps: f64) -> Scores {
        let d = data::data();
        let mut probes: Vec<(Probe, &str, Stats)> = Vec::new();
        if self.options.sockets {
            probes.extend(
                d.gems
                    .values()
                    .map(|gem| (Probe::Socketable, gem.id.as_str(), stat_list(&gem.stats))),
            );
            probes.extend(
                d.runes
                    .values()
                    .map(|rune| (Probe::Socketable, rune.id.as_str(), stat_list(&rune.stats))),
            );
        }
        if self.options.crystals {
            probes.extend(
                d.crystals
                    .values()
                    .map(|crystal| (Probe::Crystal, crystal.id.as_str(), affix_stats(crystal))),
            );
        }
        if self.options.affixes {
            let groups: HashSet<&str> = d
                .items
                .values()
                .filter_map(|base| base.random_affix_group_id.as_deref())
                .collect();
            probes.extend(
                d.affixes
                    .values()
                    .filter(|affix| groups.contains(affix.group_id.as_str()))
                    .map(|affix| {
                        let group = Probe::Affix(affix.group_id.as_str());
                        (group, affix.id.as_str(), affix_stats(affix))
                    }),
            );
        }
        let measured: Vec<f64> = probes
            .par_iter()
            .map(|(_, _, stats)| {
                if stats.is_empty() {
                    return 0.;
                }
                let _scope = self.scope();
                let mut next = snapshot.clone();
                next.custom_stats
                    .extend(stats.iter().map(|(stat_key, value)| CustomStat {
                        stat_key: stat_key.clone(),
                        value: value.to_string(),
                    }));
                self.dps(&next) - dps
            })
            .collect();
        let mut scores = Scores::default();
        for ((probe, id, _), gain) in probes.into_iter().zip(measured) {
            if !gains(dps + gain, dps) {
                continue;
            }
            let entry = (id.to_owned(), gain);
            match probe {
                Probe::Socketable => scores.socketables.push(entry),
                Probe::Crystal => scores.crystals.push(entry),
                Probe::Affix(group) => scores.affixes.entry(group.into()).or_default().push(entry),
            }
        }
        let rank = |list: &mut Vec<(String, f64)>| {
            list.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)))
        };
        rank(&mut scores.socketables);
        rank(&mut scores.crystals);
        scores.affixes.values_mut().for_each(rank);
        scores
    }

    /// What the slot may hold: the current item, the player's own item if an earlier pass
    /// replaced it, and shortlisted other bases.
    fn candidates(
        &self,
        snapshot: &BuildSnapshot,
        slot: &str,
        scores: &Scores,
    ) -> Vec<EquippedItem> {
        let current = snapshot.inventory.get(slot);
        let own = self.start.inventory.get(slot).filter(|own| {
            current.is_none_or(|item| item.base_id != own.base_id)
                && !other_relics(snapshot, slot).contains(own.base_id.as_str())
        });
        let mut items: Vec<EquippedItem> = current.into_iter().chain(own).cloned().collect();
        if self.options.other_items {
            items.extend(self.shortlist(snapshot, slot, scores));
        }
        items
    }

    /// The slot's best item among its candidates, each improved by the enabled modifications.
    fn best_for(&self, snapshot: &BuildSnapshot, slot: &str, scores: &Scores) -> Option<Pick> {
        let picks: Vec<Option<Pick>> = self
            .candidates(snapshot, slot, scores)
            .into_par_iter()
            .map(|item| {
                let _scope = self.scope();
                self.configure(snapshot, slot, item, scores)
            })
            .collect();
        picks.into_iter().flatten().reduce(|best, pick| {
            if gains(pick.dps, best.dps) {
                pick
            } else {
                best
            }
        })
    }

    /// Other bases ranked by measured DPS plus what their missing modifications should add.
    /// The player's own base never comes back as a fresh copy: its real item is a candidate.
    fn shortlist(
        &self,
        snapshot: &BuildSnapshot,
        slot: &str,
        scores: &Scores,
    ) -> Vec<EquippedItem> {
        let worn: HashSet<&str> = [snapshot, self.start]
            .into_iter()
            .filter_map(|build| build.inventory.get(slot))
            .map(|item| item.base_id.as_str())
            .chain(other_relics(snapshot, slot))
            .collect();
        let mut bases: Vec<&ItemBase> = data::data()
            .items
            .values()
            .filter(|base| {
                !worn.contains(base.id.as_str())
                    && base
                        .requires_level
                        .is_none_or(|level| level <= snapshot.level)
                    && gear::accepts(snapshot, slot, base, false)
            })
            .collect();
        bases.sort_by(|a, b| a.id.cmp(&b.id));
        let mut ranked: Vec<(f64, EquippedItem)> = bases
            .par_iter()
            .filter_map(|base| {
                let _scope = self.scope();
                let pick = self.template(snapshot, slot, base)?;
                Some((
                    pick.dps + self.potential(&pick.item, base, scores),
                    pick.item,
                ))
            })
            .collect();
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
        ranked
            .into_iter()
            .take(SHORTLIST)
            .map(|(_, item)| item)
            .collect()
    }

    /// A fresh copy of `base` with full stars, its best rolls and, on a common base, its best runeword.
    fn template(&self, snapshot: &BuildSnapshot, slot: &str, base: &ItemBase) -> Option<Pick> {
        let mut item = gear::make_item(&base.id).ok()?;
        if self.options.stars && data::can_star_forge(slot, &base.rarity) {
            item.stars = Some(star_scaling::max_stars());
        }
        let mut pick = self.try_item(snapshot, slot, item, false)?;
        for (choice, values) in choices(snapshot, base) {
            let next = self.best(snapshot, slot, &values, |value| {
                let mut item = pick.item.clone();
                choice.set(&mut item, value);
                Some((item, false))
            });
            pick = better(pick, next);
        }
        if self.options.sockets {
            let next = self.runeword(snapshot, slot, &pick);
            pick = better(pick, next);
        }
        Some(pick)
    }

    fn potential(&self, item: &EquippedItem, base: &ItemBase, scores: &Scores) -> f64 {
        let gem = scores.socketables.first().map_or(0., |(_, gain)| *gain);
        let mut gain = 0.;
        if self.options.sockets && runeword_of(item).is_none() {
            let filled = item.socketed.iter().flatten().count() as u32;
            gain += f64::from(gear::max_sockets(item).saturating_sub(filled)) * gem;
        }
        if self.options.crystals && data::forge_kind_for(&base.rarity).is_some() {
            let crystal = scores.crystals.first().map_or(0., |(_, gain)| *gain);
            let socket = if self.options.sockets && adds_socket(item) {
                gem
            } else {
                0.
            };
            gain += crystal.max(socket);
        }
        if self.options.affixes
            && let Some(group) = &base.random_affix_group_id
        {
            let ranked = scores.affixes.get(group).into_iter().flatten();
            let slots = base.max_affixes.unwrap_or(0) as usize;
            gain += ranked.take(slots).map(|(_, gain)| gain).sum::<f64>();
        }
        gain
    }

    /// The best version of `item` the enabled modifications reach, one kind at a time.
    fn configure(
        &self,
        snapshot: &BuildSnapshot,
        slot: &str,
        item: EquippedItem,
        scores: &Scores,
    ) -> Option<Pick> {
        let base = data::get_item(&item.base_id)?;
        let mut pick = self.try_item(snapshot, slot, item, false)?;
        let max_stars = star_scaling::max_stars();
        if self.options.stars
            && data::can_star_forge(slot, &base.rarity)
            && pick.item.stars.unwrap_or(0) < max_stars
        {
            let mut item = pick.item.clone();
            item.stars = Some(max_stars);
            let next = self.try_item(snapshot, slot, item, false);
            pick = better(pick, next);
        }
        if self.options.affixes {
            pick = self.affixes(snapshot, slot, base, pick, scores);
        }
        if self.options.sockets {
            pick = self.sockets(snapshot, slot, pick, scores);
        }
        if self.options.crystals && data::forge_kind_for(&base.rarity).is_some() {
            let sockets = pick.item.socket_count;
            pick = self.crystal(snapshot, slot, pick, scores);
            if pick.item.socket_count != sockets {
                pick = self.sockets(snapshot, slot, pick, scores);
            }
        }
        if self.options.augments && slot == "armor" {
            pick = self.augment(snapshot, slot, pick);
        }
        Some(pick)
    }

    /// Fills free random-affix slots, then swaps out weaker affixes, best probe gains first.
    fn affixes(
        &self,
        snapshot: &BuildSnapshot,
        slot: &str,
        base: &ItemBase,
        mut pick: Pick,
        scores: &Scores,
    ) -> Pick {
        let Some(ranked) = base
            .random_affix_group_id
            .as_ref()
            .and_then(|group| scores.affixes.get(group))
        else {
            return pick;
        };
        let options = |item: &EquippedItem| -> Vec<&str> {
            ranked
                .iter()
                .map(|(id, _)| id.as_str())
                .filter(|id| item.affixes.iter().all(|a| a.affix_id != *id))
                .take(VERIFIED)
                .collect()
        };
        while (pick.item.affixes.len() as u32) < base.max_affixes.unwrap_or(0) {
            let next = self.best(snapshot, slot, &options(&pick.item), |id| {
                let mut item = pick.item.clone();
                gear::add_affix(&mut item, id).ok()?;
                Some((item, pick.augment_on))
            });
            match next {
                Some(next) if gains(next.dps, pick.dps) => pick = next,
                _ => break,
            }
        }
        for index in 0..pick.item.affixes.len() {
            let next = self.best(snapshot, slot, &options(&pick.item), |id| {
                let affix = data::get_affix(id)?;
                let mut item = pick.item.clone();
                item.affixes[index] = EquippedAffix {
                    affix_id: affix.id.clone(),
                    tier: affix.tier,
                    roll: 1.,
                    custom_value: None,
                };
                Some((item, pick.augment_on))
            });
            pick = better(pick, next);
        }
        pick
    }

    /// Fills every socket with its best gem or rune, or completes the best runeword on a common base.
    fn sockets(&self, snapshot: &BuildSnapshot, slot: &str, pick: Pick, scores: &Scores) -> Pick {
        if gear::max_sockets(&pick.item) == 0 {
            return pick;
        }
        let mut item = pick.item.clone();
        let max = gear::max_sockets(&item);
        gear::set_socket_count(&mut item, max);
        // Extra sockets can break a runeword, so the wider item is measured before filling it.
        let mut filled = if item.socket_count == pick.item.socket_count {
            Some(pick.clone())
        } else {
            self.try_item(snapshot, slot, item, pick.augment_on)
        };
        if let Some(mut wide) = filled.take() {
            for index in 0..wide.item.socket_count as usize {
                let current = wide.item.socketed.get(index).cloned().flatten();
                let options: Vec<&str> = scores
                    .socketables
                    .iter()
                    .map(|(id, _)| id.as_str())
                    .filter(|id| Some(*id) != current.as_deref())
                    .take(VERIFIED)
                    .collect();
                let next = self.best(snapshot, slot, &options, |id| {
                    let mut item = wide.item.clone();
                    gear::set_socket(&mut item, index, Some(*id)).ok()?;
                    Some((item, wide.augment_on))
                });
                wide = better(wide, next);
            }
            filled = Some(wide);
        }
        let runeword = self.runeword(snapshot, slot, &pick);
        better(better(pick, filled), runeword)
    }

    fn runeword(&self, snapshot: &BuildSnapshot, slot: &str, pick: &Pick) -> Option<Pick> {
        let words: Vec<&str> = data::data()
            .runewords
            .iter()
            .filter(|rw| {
                rw.requires_level
                    .is_none_or(|level| level <= snapshot.level)
            })
            .map(|rw| rw.id.as_str())
            .collect();
        self.best(snapshot, slot, &words, |id| {
            let mut item = pick.item.clone();
            gear::apply_runeword(&mut item, id).ok()?;
            Some((item, pick.augment_on))
        })
    }

    /// Forges the best satanic crystal, counting an extra socket as its best gem.
    fn crystal(&self, snapshot: &BuildSnapshot, slot: &str, pick: Pick, scores: &Scores) -> Pick {
        let current = pick.item.forged_mods.first().map(|m| m.affix_id.as_str());
        let mut options: Vec<&str> = scores
            .crystals
            .iter()
            .map(|(id, _)| id.as_str())
            .filter(|id| Some(*id) != current)
            .take(VERIFIED)
            .collect();
        if self.options.sockets && current != Some(ADD_SOCKET) {
            options.push(ADD_SOCKET);
        }
        let gem = scores.socketables.first().map(|(id, _)| id.as_str());
        let next = self.best(snapshot, slot, &options, |id| {
            let mut item = pick.item.clone();
            gear::set_forge(&mut item, Some(*id)).ok()?;
            // Unpinned crystals count at the middle of their range; pin the top roll.
            if let Some(crystal) = data::get_crystal_mod(id).filter(|c| c.value_max.is_some()) {
                gear::set_forge_value(&mut item, 0, rolled_affix_value(crystal, 1.));
            }
            if *id == ADD_SOCKET {
                let index = item.socket_count as usize;
                gear::set_socket_count(&mut item, index as u32 + 1);
                if item.socket_count as usize == index {
                    return None;
                }
                gear::set_socket(&mut item, index, gem).ok()?;
            }
            Some((item, pick.augment_on))
        });
        better(pick, next)
    }

    /// Tries every angelic augment at its top level; a new one counts with its Config toggle on.
    fn augment(&self, snapshot: &BuildSnapshot, slot: &str, pick: Pick) -> Pick {
        let mut augments: Vec<&AngelicAugment> = data::data().augments.values().collect();
        augments.sort_by(|a, b| a.id.cmp(&b.id));
        let current = pick.item.augment.clone();
        let next = self.best(snapshot, slot, &augments, |augment| {
            let level = augment.levels.len() as u32;
            let same = current.as_ref().filter(|c| c.id == augment.id);
            if same.is_some_and(|c| c.level >= level) {
                return None;
            }
            let mut item = pick.item.clone();
            gear::set_augment(&mut item, Some(&augment.id)).ok()?;
            item.augment.as_mut()?.level = level;
            Some((item, same.is_none()))
        });
        better(pick, next)
    }
}

#[cfg(test)]
#[path = "gear_suggest_tests.rs"]
mod tests;
