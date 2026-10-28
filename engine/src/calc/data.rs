// JSON blobs from src/data/ are inlined at compile time via build.rs into
// `$OUT_DIR/data_includes.rs`, then lazily parsed into GameData on first access.

use std::collections::{BTreeMap, HashMap, HashSet};

use std::sync::LazyLock;

use super::types::{
    Affix, AffixTag, AngelicAugment, CharacterClass, DifficultyDef, EquippedItem, GameConfig, Gem,
    Inventory, ItemBase, ItemGrantedSkill, ItemSet, Rune, Runeword, SkillSpec, SubskillTagChange,
    TreeNodeInfo,
};

#[allow(dead_code)]
mod includes {
    include!(concat!(env!("OUT_DIR"), "/data_includes.rs"));
}

const GEAR_SLOTS: &[&str] = &[
    "weapon", "offhand", "helmet", "armor", "gloves", "boots", "belt", "amulet", "ring_1", "ring_2",
];

pub fn is_gear_slot(slot: &str) -> bool {
    GEAR_SLOTS.contains(&slot)
}

pub fn is_charm_slot(slot: &str) -> bool {
    slot.starts_with("charm_")
}

/// Only common charms (Small/Large/Grand) take stars; unique charms never do.
pub fn can_star_forge(slot: &str, rarity: &str) -> bool {
    if is_charm_slot(slot) {
        return rarity == "common";
    }
    is_gear_slot(slot)
}

/// Rakhul's Ritual Band carries no stats of its own — it mirrors the other ring.
const MIRROR_RING_ID: &str = "ring_heroic_rakhul_s_ritual_band";

/// Inventory as the stat pipeline should see it: every equipped slot, plus a
/// second pass over the ring that Rakhul's Ritual Band mirrors. Entries are
/// `(slot, item, is_mirror)`; two bands mirror nothing.
pub fn inventory_entries(inventory: &Inventory) -> Vec<(&str, &EquippedItem, bool)> {
    let mut entries: Vec<(&str, &EquippedItem, bool)> = inventory
        .iter()
        .map(|(slot, item)| (slot.as_str(), item, false))
        .collect();
    for (band_slot, other_slot) in [("ring_1", "ring_2"), ("ring_2", "ring_1")] {
        let has_band = inventory
            .get(band_slot)
            .is_some_and(|item| item.base_id == MIRROR_RING_ID);
        let source = inventory
            .get(other_slot)
            .filter(|item| item.base_id != MIRROR_RING_ID);
        if let (true, Some(source)) = (has_band, source) {
            entries.push((band_slot, source, true));
        }
    }
    entries
}

const SATANIC_CRYSTAL_RARITIES: &[&str] =
    &["satanic", "satanic_set", "heroic", "angelic", "unholy"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgeKind {
    SatanicCrystal,
}

pub fn forge_kind_for(rarity: &str) -> Option<ForgeKind> {
    if SATANIC_CRYSTAL_RARITIES.contains(&rarity) {
        Some(ForgeKind::SatanicCrystal)
    } else {
        None
    }
}

pub struct GameData {
    pub affixes: HashMap<String, Affix>,
    pub affix_tags: BTreeMap<String, AffixTag>,
    pub crystals: HashMap<String, Affix>,
    pub items: HashMap<String, ItemBase>,
    pub gems: HashMap<String, Gem>,
    pub runes: HashMap<String, Rune>,
    pub runewords: Vec<Runeword>,
    pub sets: HashMap<String, ItemSet>,
    pub augments: HashMap<String, AngelicAugment>,
    pub item_granted_skills: Vec<ItemGrantedSkill>,
    pub classes: HashMap<String, CharacterClass>,
    pub skills_by_class: HashMap<String, Vec<SkillSpec>>,
    pub game_config: GameConfig,
    pub subskill_tags: HashMap<String, HashMap<String, SubskillTagChange>>,
    pub tree_nodes: HashMap<String, TreeNodeInfo>,
    pub tree_warp_ids: HashSet<u32>,
    pub tree_jewelry_ids: HashSet<u32>,
}

static GAME_DATA: LazyLock<GameData> = LazyLock::new(load);

fn index_by_id<T, F: Fn(&T) -> String>(list: Vec<T>, key: F) -> HashMap<String, T> {
    let mut out: HashMap<String, T> = HashMap::with_capacity(list.len());
    for t in list {
        out.insert(key(&t), t);
    }
    out
}

fn parse_value(json: &str, ctx: &str) -> serde_json::Value {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("failed to parse {ctx}: {e}"))
}

fn from_value<T: serde::de::DeserializeOwned>(value: serde_json::Value, ctx: &str) -> T {
    serde_json::from_value(value).unwrap_or_else(|e| panic!("invalid {ctx} shape: {e}"))
}

fn load_json<T: serde::de::DeserializeOwned>(json: &str, ctx: &str) -> T {
    from_value(parse_value(json, ctx), ctx)
}

// Array files contribute their elements; scalar files contribute themselves.
fn concat_values(blobs: &[&str], ctx: &str) -> serde_json::Value {
    let mut all = Vec::new();
    for blob in blobs {
        match parse_value(blob, ctx) {
            serde_json::Value::Array(items) => all.extend(items),
            other => all.push(other),
        }
    }
    serde_json::Value::Array(all)
}

fn load_json_many<T: serde::de::DeserializeOwned>(blobs: &[&str], ctx: &str) -> Vec<T> {
    from_value(concat_values(blobs, ctx), ctx)
}

fn load() -> GameData {
    use includes::*;

    let affixes_vec: Vec<Affix> = load_json(AFFIXES_JSON, "affixes.json");
    let crystals_vec: Vec<Affix> = load_json(CRYSTALS_JSON, "crystals.json");
    let runewords_vec: Vec<Runeword> = load_json(RUNEWORDS_JSON, "runewords.json");
    let sets_vec: Vec<ItemSet> = load_json(SETS_JSON, "sets.json");
    let augments_vec: Vec<AngelicAugment> = load_json(AUGMENTS_JSON, "augments.json");
    let item_granted_vec: Vec<ItemGrantedSkill> =
        load_json(ITEM_GRANTED_SKILLS_JSON, "item-granted-skills.json");
    let game_config: GameConfig = load_json(GAME_CONFIG_JSON, "game-config.json");
    let affix_tags: BTreeMap<String, AffixTag> = load_json(AFFIX_TAGS_JSON, "affix-tags.json");
    let subskill_tags: HashMap<String, HashMap<String, SubskillTagChange>> =
        load_json(SUBSKILL_TAGS_JSON, "subskill-tags.json");
    let tree_nodes: HashMap<String, TreeNodeInfo> =
        load_json(INCARNATION_NODES_JSON, "incarnation-nodes.json");

    let mut tree_warp_ids: HashSet<u32> = HashSet::new();
    let mut tree_jewelry_ids: HashSet<u32> = HashSet::new();
    for (id_str, info) in tree_nodes.iter() {
        if let Ok(id) = id_str.parse::<u32>() {
            match info.kind.as_str() {
                "warp" => {
                    tree_warp_ids.insert(id);
                }
                "jewelry" => {
                    tree_jewelry_ids.insert(id);
                }
                _ => {}
            }
        }
    }

    let items_all: Vec<ItemBase> = load_json_many(ITEMS_JSON, "items/*.json");
    let gems_all: Vec<Gem> = load_json_many(GEMS_JSON, "gems/*.json");
    let runes_all: Vec<Rune> = load_json_many(RUNES_JSON, "runes/*.json");
    let classes_all: Vec<CharacterClass> = load_json_many(CLASSES_JSON, "classes/*.json");
    let skills_all: Vec<SkillSpec> = load_json_many(SKILLS_JSON, "skills/*.json");

    let mut skills_by_class: HashMap<String, Vec<SkillSpec>> = HashMap::new();
    for skill in skills_all {
        skills_by_class
            .entry(skill.class_id.clone())
            .or_default()
            .push(skill);
    }

    GameData {
        affixes: index_by_id(affixes_vec, |a| a.id.clone()),
        affix_tags,
        crystals: index_by_id(crystals_vec, |a| a.id.clone()),
        items: index_by_id(items_all, |i| i.id.clone()),
        gems: index_by_id(gems_all, |g| g.id.clone()),
        runes: index_by_id(runes_all, |r| r.id.clone()),
        runewords: runewords_vec,
        sets: index_by_id(sets_vec, |s| s.id.clone()),
        augments: index_by_id(augments_vec, |a| a.id.clone()),
        item_granted_skills: item_granted_vec,
        classes: index_by_id(classes_all, |c| c.id.clone()),
        skills_by_class,
        game_config,
        subskill_tags,
        tree_nodes,
        tree_warp_ids,
        tree_jewelry_ids,
    }
}

pub fn data() -> &'static GameData {
    &GAME_DATA
}

// ---------- lookup helpers ----------

pub fn get_affix(id: &str) -> Option<&'static Affix> {
    data().affixes.get(id)
}

pub fn get_crystal_mod(id: &str) -> Option<&'static Affix> {
    data().crystals.get(id)
}

pub fn get_item(id: &str) -> Option<&'static ItemBase> {
    data().items.get(id)
}

pub fn get_gem(id: &str) -> Option<&'static Gem> {
    data().gems.get(id)
}

pub fn get_rune(id: &str) -> Option<&'static Rune> {
    data().runes.get(id)
}

pub fn get_set(id: &str) -> Option<&'static ItemSet> {
    data().sets.get(id)
}

pub fn get_augment(id: &str) -> Option<&'static AngelicAugment> {
    data().augments.get(id)
}

pub fn get_class(id: &str) -> Option<&'static CharacterClass> {
    data().classes.get(id)
}

pub fn skill_name_by_id(skill_id: &str) -> Option<&'static str> {
    data()
        .skills_by_class
        .values()
        .flatten()
        .find(|s| s.id == skill_id)
        .map(|s| s.name.as_str())
}

pub fn get_skills_by_class(class_id: &str) -> &'static [SkillSpec] {
    data()
        .skills_by_class
        .get(class_id)
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

pub fn item_granted_skills() -> &'static [ItemGrantedSkill] {
    &data().item_granted_skills
}

pub fn game_config() -> &'static GameConfig {
    &data().game_config
}

pub fn affix_tags() -> &'static BTreeMap<String, AffixTag> {
    &data().affix_tags
}

pub fn subskill_tag_changes(skill_id: &str) -> Option<&'static HashMap<String, SubskillTagChange>> {
    data().subskill_tags.get(skill_id)
}

pub fn runewords() -> &'static [Runeword] {
    &data().runewords
}

pub fn get_difficulty(id: Option<&str>) -> Option<&'static DifficultyDef> {
    let id = id?;
    game_config().difficulties.iter().find(|d| d.id == id)
}

pub fn tree_nodes() -> &'static HashMap<String, TreeNodeInfo> {
    &data().tree_nodes
}

pub fn tree_warp_ids() -> &'static HashSet<u32> {
    &data().tree_warp_ids
}

pub fn tree_jewelry_ids() -> &'static HashSet<u32> {
    &data().tree_jewelry_ids
}

pub fn get_tree_node(id: u32) -> Option<&'static TreeNodeInfo> {
    data().tree_nodes.get(&id.to_string())
}

pub enum Socketable<'a> {
    Gem(&'a Gem),
    Rune(&'a Rune),
}

pub fn get_socketable_by_id(id: &str) -> Option<Socketable<'static>> {
    if let Some(g) = get_gem(id) {
        return Some(Socketable::Gem(g));
    }
    if let Some(r) = get_rune(id) {
        return Some(Socketable::Rune(r));
    }
    None
}

/// Whether the runeword can form on this base, including its one- or two-handed variant.
pub fn runeword_fits_base(rw: &Runeword, base: &ItemBase) -> bool {
    rw.allowed_base_types.iter().any(|t| t == &base.base_type)
        && rw
            .two_handed
            .is_none_or(|two_handed| two_handed == (base.two_handed == Some(true)))
}

/// Returns the runeword that exactly matches every socket in order, or None.
pub fn detect_runeword(base: &ItemBase, socketed: &[Option<&str>]) -> Option<&'static Runeword> {
    if base.rarity != "common" {
        return None;
    }
    if socketed.iter().any(|s| s.is_none()) {
        return None;
    }
    for rw in data().runewords.iter() {
        if rw.runes.len() != socketed.len() {
            continue;
        }
        if !runeword_fits_base(rw, base) {
            continue;
        }
        let mut all_match = true;
        for (i, rune) in rw.runes.iter().enumerate() {
            if Some(rune.as_str()) != socketed[i] {
                all_match = false;
                break;
            }
        }
        if all_match {
            return Some(rw);
        }
    }
    None
}

/// Skill bonuses granted by the item base plus the runeword completed in its sockets.
pub fn skill_bonus_entries<'a>(
    base: &'a ItemBase,
    item: &'a EquippedItem,
) -> impl Iterator<Item = (&'a String, &'a super::types::RangedValue)> {
    let socketed: Vec<Option<&str>> = item.socketed.iter().map(|s| s.as_deref()).collect();
    let runeword = detect_runeword(base, &socketed).and_then(|rw| rw.skill_bonuses.as_ref());
    base.skill_bonuses
        .iter()
        .flatten()
        .chain(runeword.into_iter().flatten())
}

/// Skills an item's procs cast. Display metadata, except for conditional buffs.
pub fn proc_skill_grants(base: &ItemBase) -> impl Iterator<Item = &super::types::ItemSkillGrant> {
    base.procs
        .iter()
        .flatten()
        .filter_map(|proc| proc.granted_skill.as_ref())
}

pub fn get_item_granted_skill_by_name(name: &str) -> Option<&'static ItemGrantedSkill> {
    let needle = name.trim().to_lowercase();
    data()
        .item_granted_skills
        .iter()
        .find(|s| s.name.trim().to_lowercase() == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_runeword_adds_its_granted_skill_to_the_item() {
        let base = get_item("armors_normal_steel_armor").unwrap();
        let item = EquippedItem {
            base_id: base.id.clone(),
            socketed: vec![Some("rune_io".into()), Some("rune_pul".into())],
            ..Default::default()
        };
        let entries: Vec<_> = skill_bonus_entries(base, &item).collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "Angel\u{2019}s Vibrance Aura");
        assert_eq!(entries[0].1.as_ranged(), (2., 6.));
        let granted = get_item_granted_skill_by_name(entries[0].0).unwrap();
        assert!(granted.aura && granted.passive_stats.is_some());

        let plain = EquippedItem {
            base_id: base.id.clone(),
            ..Default::default()
        };
        assert_eq!(skill_bonus_entries(base, &plain).count(), 0);
    }

    #[test]
    fn grimwalkers_matches_the_game() {
        let rw = data()
            .runewords
            .iter()
            .find(|rw| rw.id == "rw_grimwalkers")
            .unwrap();
        let stat = |key: &str| rw.stats.get(key).map(|v| v.as_ranged());
        assert_eq!(stat("enhanced_defense"), Some((320., 400.)));
        assert_eq!(stat("all_attributes"), Some((30., 40.)));
        assert_eq!(stat("magic_skill_damage"), Some((30., 40.)));
        assert_eq!(stat("movement_speed"), Some((40., 40.)));
        // Flo's +2% Maximum All Resists and Nju's 3% on top of the word's own [10-15]%.
        assert_eq!(stat("max_all_resistances"), Some((2., 2.)));
        assert_eq!(stat("ignore_all_res"), Some((13., 18.)));
        assert_eq!(stat("reduced_movement_diminish"), Some((40., 80.)));
        assert_eq!(stat("defense"), None);
        assert_eq!(stat("all_resistances"), None);
        // Its own name, so a Demon Slayer's Demon's Presence gains no ranks from the proc.
        let ranks = rw.skill_bonuses.as_ref().unwrap()["Demon's Presence (Grimwalkers)"];
        assert_eq!(ranks.as_ranged(), (15., 20.));
        let proc = get_item_granted_skill_by_name("Demon's Presence (Grimwalkers)").unwrap();
        assert!(proc.condition.is_some() && proc.passive_stats.is_some());
    }

    #[test]
    fn angels_vibrance_raises_replenish_by_percent() {
        let aura = get_item_granted_skill_by_name("Angel\u{2019}s Vibrance Aura").unwrap();
        let spec = aura.passive_stats.as_ref().unwrap();
        for stats in [spec.base.as_ref().unwrap(), spec.per_rank.as_ref().unwrap()] {
            assert!(stats.contains_key("life_replenish_increased"));
            assert!(stats.contains_key("mana_replenish_increased"));
            assert!(!stats.contains_key("life_replenish") && !stats.contains_key("mana_replenish"));
        }
    }

    #[test]
    fn known_gear_slots() {
        for slot in [
            "weapon", "offhand", "helmet", "armor", "gloves", "boots", "belt", "amulet", "ring_1",
            "ring_2",
        ] {
            assert!(is_gear_slot(slot), "{slot} should be a gear slot");
        }
    }

    #[test]
    fn non_gear_slots() {
        assert!(!is_gear_slot("relic"));
        assert!(!is_gear_slot("augment"));
        assert!(!is_gear_slot(""));
        assert!(!is_gear_slot("ring_3"));
        assert!(!is_gear_slot("WEAPON")); // case-sensitive
    }

    #[test]
    fn forge_kind_classification() {
        assert_eq!(forge_kind_for("satanic"), Some(ForgeKind::SatanicCrystal));
        assert_eq!(forge_kind_for("heroic"), Some(ForgeKind::SatanicCrystal));
        assert_eq!(forge_kind_for("angelic"), Some(ForgeKind::SatanicCrystal));
        assert_eq!(forge_kind_for("unholy"), Some(ForgeKind::SatanicCrystal));
        assert_eq!(forge_kind_for("relic"), None);
        assert_eq!(forge_kind_for("common"), None);
        assert_eq!(forge_kind_for("rare"), None);
        assert_eq!(forge_kind_for(""), None);
    }

    // Accessing data() forces every JSON file to deserialise, so a schema mismatch
    // panics here with a precise file/error instead of surfacing later.

    #[test]
    fn game_data_loads_without_panic() {
        let d = data();
        // Sanity: at least one affix/item/class/runeword must exist.
        assert!(!d.affixes.is_empty(), "affixes empty");
        assert!(!d.items.is_empty(), "items empty");
        assert!(!d.classes.is_empty(), "classes empty");
        assert!(!d.runewords.is_empty(), "runewords empty");
        assert!(!d.sets.is_empty(), "sets empty");
        assert!(!d.augments.is_empty(), "augments empty");
        assert!(!d.gems.is_empty(), "gems empty");
        assert!(!d.runes.is_empty(), "runes empty");
        assert!(!d.crystals.is_empty(), "crystals empty");
    }

    #[test]
    fn game_config_has_attributes_and_stats() {
        let cfg = game_config();
        assert!(!cfg.attributes.is_empty(), "game config has no attributes");
        assert!(!cfg.stats.is_empty(), "game config has no stats");
        // Six baseline attributes.
        let attr_keys: Vec<&str> = cfg.attributes.iter().map(|a| a.key.as_str()).collect();
        for must_have in [
            "strength",
            "dexterity",
            "intelligence",
            "energy",
            "vitality",
            "armor",
        ] {
            assert!(
                attr_keys.contains(&must_have),
                "missing attribute key: {must_have}"
            );
        }
    }

    #[test]
    fn class_lookup_resolves_known_id() {
        // Sampled directly from `src/data/classes/amazon.json`.
        let amazon = get_class("amazon").expect("amazon class missing");
        assert_eq!(amazon.id, "amazon");
        assert_eq!(amazon.name, "Amazon");
    }

    #[test]
    fn item_lookup_resolves_arbitrary_id() {
        // Take the first item from the loaded list and look it up by its own id.
        // Avoids hardcoding an id that could be renamed in data.
        let any_item_id = data().items.keys().next().expect("no items loaded").clone();
        let resolved = get_item(&any_item_id).expect("item not resolvable after listing");
        assert_eq!(resolved.id, any_item_id);
    }

    #[test]
    fn unknown_id_returns_none() {
        assert!(get_affix("definitely_not_an_affix").is_none());
        assert!(get_item("definitely_not_an_item").is_none());
        assert!(get_class("definitely_not_a_class").is_none());
    }

    #[test]
    fn etherwalk_matches_the_game() {
        let rw = data()
            .runewords
            .iter()
            .find(|rw| rw.id == "rw_etherwalk")
            .unwrap();
        let stat = |key: &str| rw.stats.get(key).map(|v| v.as_ranged());
        assert_eq!(stat("enhanced_defense"), Some((220., 300.)));
        assert_eq!(stat("all_skills"), Some((1., 2.)));
        assert_eq!(stat("movement_speed"), Some((20., 30.)));
        assert_eq!(stat("ignore_all_res"), Some((3., 10.)));
        // Uth adds 5% recouped as mana, and the boots' own defense is not a runeword stat.
        assert_eq!(stat("damage_recouped_as_mana"), Some((5., 5.)));
        assert_eq!(stat("defense"), None);
        let proc = &rw.procs.as_ref().unwrap()[0];
        assert_eq!(
            proc.description.as_deref(),
            Some("cast Mana Devour Level [12-22]")
        );
    }

    #[test]
    fn weapon_runewords_roll_their_two_handed_stats_on_two_handed_bases() {
        let runes = ["rune_um", "rune_gul", "rune_qi", "rune_vex", "rune_old"].map(Some);
        let all_skills = |two_handed: bool| {
            let sword = data()
                .items
                .values()
                .filter(|b| b.rarity == "common" && b.base_type == "Sword")
                .find(|b| (b.two_handed == Some(true)) == two_handed)
                .unwrap();
            detect_runeword(sword, &runes).unwrap().stats["all_skills"].as_ranged()
        };
        assert_eq!(all_skills(false), (2., 2.));
        assert_eq!(all_skills(true), (4., 4.));
    }

    #[test]
    fn detect_runeword_rejects_non_common_or_partial_sockets() {
        // First item we can find — base used purely to exercise the early bails.
        let any_item = data()
            .items
            .values()
            .next()
            .expect("no items loaded")
            .clone();
        // Empty/partial socket list shouldn't match any runeword.
        assert!(detect_runeword(&any_item, &[None]).is_none());
        assert!(detect_runeword(&any_item, &[None, None]).is_none());
    }

    #[test]
    fn item_granted_skill_by_name_is_case_insensitive() {
        // Iterate registered names — case folding must round-trip.
        if let Some(s) = data().item_granted_skills.first() {
            let up = s.name.to_uppercase();
            let down = s.name.to_lowercase();
            let padded = format!("  {}  ", s.name);
            assert!(
                get_item_granted_skill_by_name(&up).is_some(),
                "uppercase lookup failed for '{}'",
                s.name
            );
            assert!(
                get_item_granted_skill_by_name(&down).is_some(),
                "lowercase lookup failed for '{}'",
                s.name
            );
            assert!(
                get_item_granted_skill_by_name(&padded).is_some(),
                "padded lookup failed for '{}'",
                s.name
            );
        }
        assert!(get_item_granted_skill_by_name("nonexistent skill xyz").is_none());
    }

    #[test]
    fn augment_procs_carry_a_chance_and_their_damage() {
        use crate::calc::types::AugmentCalc;
        let procs: Vec<_> = data()
            .augments
            .values()
            .filter(|aug| aug.calc == AugmentCalc::Proc)
            .collect();
        assert_eq!(procs.len(), 32);
        for aug in procs {
            let tier = aug.tier(1).expect(&aug.id);
            assert!(
                tier.proc_chance.is_some_and(|c| c > 0.) || aug.period.is_some(),
                "{} chance",
                aug.id
            );
            assert!(
                tier.stats.keys().any(|k| k == "enhanced_damage"
                    || k == "incoming_damage_as_lightning_pct"
                    || k == "recast_damage_increase"
                    || (k.starts_with("flat_") && k.ends_with("_skill_damage"))),
                "{} damage",
                aug.id
            );
        }
        // Hero_Siege.exe: start + per level × L.
        let staticshot = get_augment("staticshot").unwrap();
        assert_eq!(staticshot.tier(7).unwrap().stats["flat_lightning_skill_damage"], 6500.);
        assert_eq!(get_augment("gut_rippers_spikeball").unwrap().trigger, "on_attack");
    }

    #[test]
    fn potion_buffs_match_game_values_and_need_their_toggle() {
        let at_rank = |name: &str, key: &str, rank: f64| {
            let skill = get_item_granted_skill_by_name(name).expect(name);
            assert!(skill.condition.is_some(), "{name} applies without a toggle");
            let passive = skill.passive_stats.as_ref().expect(name);
            let get = |map: &Option<crate::calc::types::StatMap>| {
                map.as_ref()
                    .and_then(|m| m.get(key).copied())
                    .unwrap_or(0.)
            };
            get(&passive.base) + get(&passive.per_rank) * rank
        };
        assert_eq!(at_rank("Berserker's Rage", "damage", 1.), 27.);
        assert_eq!(at_rank("Berserker's Rage", "damage", 5.), 39.);
        assert_eq!(at_rank("Berserker's Rage", "life_steal", 5.), 0.);
        assert_eq!(at_rank("Drunken Infused Wisdom", "all_skills", 1.), 1.);
        assert_eq!(at_rank("Drunken Infused Wisdom", "all_skills", 5.), 5.);
        assert_eq!(at_rank("Caffeinated", "movement_speed", 1.), 60.);
    }

    #[test]
    fn can_star_forge_covers_gear_and_charms_only() {
        assert!(super::can_star_forge("weapon", "heroic"));
        assert!(super::can_star_forge("charm_1", "common"));
        assert!(!super::can_star_forge("charm_1", "heroic"));
        assert!(!super::can_star_forge("charm_30", "satanic"));
        assert!(!super::can_star_forge("relic", "common"));
        assert!(super::is_charm_slot("charm_1"));
        assert!(!super::is_charm_slot("weapon"));
    }
}
