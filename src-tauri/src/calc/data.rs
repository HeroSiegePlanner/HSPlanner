// JSON blobs from src/data/ are inlined at compile time via build.rs into
// `$OUT_DIR/data_includes.rs`, then lazily parsed into GameData on first access.

use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;

use super::types::{
    Affix, AngelicAugment, CharacterClass, GameConfig, Gem, ItemBase, ItemGrantedSkill, ItemSet,
    Rune, Runeword, SkillSpec, TreeNodeInfo,
};

#[allow(dead_code)]
mod includes {
    include!(concat!(env!("OUT_DIR"), "/data_includes.rs"));
}

const GEAR_SLOTS: &[&str] = &[
    "weapon",
    "offhand",
    "helmet",
    "armor",
    "gloves",
    "boots",
    "belt",
    "amulet",
    "ring_1",
    "ring_2",
];

pub fn is_gear_slot(slot: &str) -> bool {
    GEAR_SLOTS.contains(&slot)
}

const SATANIC_CRYSTAL_RARITIES: &[&str] = &[
    "satanic",
    "satanic_set",
    "heroic",
    "angelic",
    "unholy",
    "relic",
];

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
    pub tree_nodes: HashMap<String, TreeNodeInfo>,
    pub tree_warp_ids: HashSet<u32>,
    pub tree_jewelry_ids: HashSet<u32>,
}

static GAME_DATA: Lazy<GameData> = Lazy::new(load);

fn parse<T: serde::de::DeserializeOwned>(json: &str, ctx: &str) -> T {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("failed to parse {ctx}: {e}"))
}

fn index_by_id<T, F: Fn(&T) -> String>(list: Vec<T>, key: F) -> HashMap<String, T> {
    let mut out: HashMap<String, T> = HashMap::with_capacity(list.len());
    for t in list {
        out.insert(key(&t), t);
    }
    out
}

fn parse_many<T: serde::de::DeserializeOwned>(blobs: &[&str], ctx: &str) -> Vec<T> {
    let mut all = Vec::new();
    for blob in blobs {
        let chunk: Vec<T> = parse(blob, ctx);
        all.extend(chunk);
    }
    all
}

fn load() -> GameData {
    use includes::*;

    let affixes_vec: Vec<Affix> = parse(AFFIXES_JSON, "affixes.json");
    let crystals_vec: Vec<Affix> = parse(CRYSTALS_JSON, "crystals.json");
    let runewords_vec: Vec<Runeword> = parse(RUNEWORDS_JSON, "runewords.json");
    let sets_vec: Vec<ItemSet> = parse(SETS_JSON, "sets.json");
    let augments_vec: Vec<AngelicAugment> = parse(AUGMENTS_JSON, "augments.json");
    let item_granted_vec: Vec<ItemGrantedSkill> =
        parse(ITEM_GRANTED_SKILLS_JSON, "item-granted-skills.json");
    let game_config: GameConfig = parse(GAME_CONFIG_JSON, "game-config.json");
    let tree_nodes: HashMap<String, TreeNodeInfo> = parse(TREE_NODES_JSON, "tree-nodes.json");

    let mut tree_warp_ids: HashSet<u32> = HashSet::new();
    let mut tree_jewelry_ids: HashSet<u32> = HashSet::new();
    for (id_str, info) in tree_nodes.iter() {
        if let Ok(id) = id_str.parse::<u32>() {
            match info.n.as_str() {
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

    let items_all: Vec<ItemBase> = parse_many(ITEMS_JSON, "items/*.json");
    let gems_all: Vec<Gem> = parse_many(GEMS_JSON, "gems/*.json");
    let runes_all: Vec<Rune> = parse_many(RUNES_JSON, "runes/*.json");

    let mut classes_all: Vec<CharacterClass> = Vec::with_capacity(CLASSES_JSON.len());
    for json in CLASSES_JSON {
        classes_all.push(parse(json, "classes/*.json"));
    }

    let mut skills_by_class: HashMap<String, Vec<SkillSpec>> = HashMap::new();
    for json in SKILLS_JSON {
        let chunk: Vec<SkillSpec> = parse(json, "skills/*.json");
        for skill in chunk {
            skills_by_class
                .entry(skill.class_id.clone())
                .or_default()
                .push(skill);
        }
    }

    GameData {
        affixes: index_by_id(affixes_vec, |a| a.id.clone()),
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

pub fn runewords() -> &'static [Runeword] {
    &data().runewords
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

/// Returns the runeword that exactly matches every socket in order, or None.
pub fn detect_runeword(
    base: &ItemBase,
    socketed: &[Option<&str>],
) -> Option<&'static Runeword> {
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
        if !rw.allowed_base_types.iter().any(|t| t == &base.base_type) {
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

static ITEM_GRANTED_SKILLS_BY_NAME: Lazy<HashMap<String, &'static ItemGrantedSkill>> =
    Lazy::new(|| {
        let mut m: HashMap<String, &'static ItemGrantedSkill> = HashMap::new();
        for s in data().item_granted_skills.iter() {
            m.insert(s.name.trim().to_lowercase(), s);
        }
        m
    });

pub fn get_item_granted_skill_by_name(name: &str) -> Option<&'static ItemGrantedSkill> {
    ITEM_GRANTED_SKILLS_BY_NAME
        .get(name.trim().to_lowercase().as_str())
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_gear_slots() {
        for slot in [
            "weapon", "offhand", "helmet", "armor", "gloves", "boots", "belt", "amulet",
            "ring_1", "ring_2",
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
        assert_eq!(forge_kind_for("relic"), Some(ForgeKind::SatanicCrystal));
        assert_eq!(forge_kind_for("common"), None);
        assert_eq!(forge_kind_for("rare"), None);
        assert_eq!(forge_kind_for(""), None);
    }

    // The remaining tests exercise the Lazy initialiser by accessing data(),
    // which forces every JSON file to deserialise. Any schema mismatch will
    // panic with a precise file/error in the message.

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
        // Six baseline attributes from the TS data file.
        let attr_keys: Vec<&str> = cfg.attributes.iter().map(|a| a.key.as_str()).collect();
        for must_have in ["strength", "dexterity", "intelligence", "energy", "vitality", "armor"] {
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
        let any_item_id = data()
            .items
            .keys()
            .next()
            .expect("no items loaded")
            .clone();
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
}
