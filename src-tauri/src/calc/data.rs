// JSON blobs from src/data/ are inlined at compile time via build.rs into
// `$OUT_DIR/data_includes.rs`, then lazily parsed into per-season GameData (patches applied) on first access.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use once_cell::sync::Lazy;

use super::season;
use super::types::{
    Affix, AngelicAugment, CharacterClass, GameConfig, Gem, ItemBase, ItemGrantedSkill, ItemSet,
    Rune, Runeword, SkillSpec, TreeNodeInfo,
};

#[allow(dead_code)]
mod includes {
    include!(concat!(env!("OUT_DIR"), "/data_includes.rs"));
}

pub(crate) use includes::SEASON_PATCHES;

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

pub fn is_charm_slot(slot: &str) -> bool {
    slot.starts_with("charm_")
}

pub fn charms_allow_stars_forge(season: &str) -> bool {
    season != "s9"
}

pub fn can_star_forge(slot: &str, season: &str) -> bool {
    is_gear_slot(slot) || (is_charm_slot(slot) && charms_allow_stars_forge(season))
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

static GAME_DATA_BY_SEASON: Lazy<Mutex<HashMap<String, &'static GameData>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn index_by_id<T, F: Fn(&T) -> String>(list: Vec<T>, key: F) -> HashMap<String, T> {
    let mut out: HashMap<String, T> = HashMap::with_capacity(list.len());
    for t in list {
        out.insert(key(&t), t);
    }
    out
}

pub(crate) enum PatchKind {
    List(&'static str),
    RecordMerge,
    GameConfig,
}

// All-or-nothing per collection: on patch error, log loudly and fall back to base.
pub(crate) fn patched_value(
    base: serde_json::Value,
    patches: &HashMap<String, serde_json::Value>,
    name: &str,
    kind: PatchKind,
) -> serde_json::Value {
    let Some(patch) = patches.get(name) else { return base };
    let result = match kind {
        PatchKind::List(key) => season::apply_list_patch(&base, patch, name, key),
        PatchKind::RecordMerge => season::apply_record_patch(&base, patch, name, true),
        PatchKind::GameConfig => season::apply_game_config_patch(&base, patch, name),
    };
    match result {
        Ok(v) => v,
        Err(errs) => {
            for e in errs {
                log::error!("season patch error: {e}");
            }
            base
        }
    }
}

fn parse_value(json: &str, ctx: &str) -> serde_json::Value {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("failed to parse {ctx}: {e}"))
}

fn from_value<T: serde::de::DeserializeOwned>(value: serde_json::Value, ctx: &str) -> T {
    serde_json::from_value(value).unwrap_or_else(|e| panic!("invalid {ctx} shape after patch: {e}"))
}

fn load_patched<T: serde::de::DeserializeOwned>(
    json: &str,
    patches: &HashMap<String, serde_json::Value>,
    name: &str,
    kind: PatchKind,
    ctx: &str,
) -> T {
    let value = patched_value(parse_value(json, ctx), patches, name, kind);
    from_value(value, name)
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

fn load_patched_many<T: serde::de::DeserializeOwned>(
    blobs: &[&str],
    patches: &HashMap<String, serde_json::Value>,
    name: &str,
    ctx: &str,
) -> Vec<T> {
    let value = patched_value(concat_values(blobs, ctx), patches, name, PatchKind::List("id"));
    from_value(value, name)
}

fn load_for(season_id: &str) -> GameData {
    use includes::*;

    let patches = season::patches_for(season_id);

    let affixes_vec: Vec<Affix> = load_patched(
        AFFIXES_JSON,
        &patches,
        "affixes",
        PatchKind::List("id"),
        "affixes.json",
    );
    let crystals_vec: Vec<Affix> = load_patched(
        CRYSTALS_JSON,
        &patches,
        "crystals",
        PatchKind::List("id"),
        "crystals.json",
    );
    let runewords_vec: Vec<Runeword> = load_patched(
        RUNEWORDS_JSON,
        &patches,
        "runewords",
        PatchKind::List("id"),
        "runewords.json",
    );
    let sets_vec: Vec<ItemSet> = load_patched(
        SETS_JSON,
        &patches,
        "sets",
        PatchKind::List("id"),
        "sets.json",
    );
    let augments_vec: Vec<AngelicAugment> = load_patched(
        AUGMENTS_JSON,
        &patches,
        "augments",
        PatchKind::List("id"),
        "augments.json",
    );
    let item_granted_vec: Vec<ItemGrantedSkill> = load_patched(
        ITEM_GRANTED_SKILLS_JSON,
        &patches,
        "item-granted-skills",
        PatchKind::List("name"),
        "item-granted-skills.json",
    );
    let game_config: GameConfig = load_patched(
        GAME_CONFIG_JSON,
        &patches,
        "game-config",
        PatchKind::GameConfig,
        "game-config.json",
    );
    let tree_nodes: HashMap<String, TreeNodeInfo> = load_patched(
        TREE_NODES_JSON,
        &patches,
        "tree-nodes",
        PatchKind::RecordMerge,
        "tree-nodes.json",
    );

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

    let items_all: Vec<ItemBase> = load_patched_many(ITEMS_JSON, &patches, "items", "items/*.json");
    let gems_all: Vec<Gem> = load_patched_many(GEMS_JSON, &patches, "gems", "gems/*.json");
    let runes_all: Vec<Rune> = load_patched_many(RUNES_JSON, &patches, "runes", "runes/*.json");
    let classes_all: Vec<CharacterClass> =
        load_patched_many(CLASSES_JSON, &patches, "classes", "classes/*.json");
    let skills_all: Vec<SkillSpec> =
        load_patched_many(SKILLS_JSON, &patches, "skills", "skills/*.json");

    let mut skills_by_class: HashMap<String, Vec<SkillSpec>> = HashMap::new();
    for skill in skills_all {
        skills_by_class
            .entry(skill.class_id.clone())
            .or_default()
            .push(skill);
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

pub fn data_for(season_id: &str) -> &'static GameData {
    season::cached_per_season(&GAME_DATA_BY_SEASON, season_id, load_for)
}

thread_local! {
    static LAST_DATA: RefCell<Option<(String, &'static GameData)>> = const { RefCell::new(None) };
}

/// Reads the SeasonScope thread-local; without a scope, serves DEFAULT_SEASON_ID data.
pub fn data() -> &'static GameData {
    season::memoized_current_season(&LAST_DATA, data_for)
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

// Linear scan stays season-correct; a process-wide index would pin one season's data.
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
    fn data_for_patchless_season_serves_base_data() {
        let a = super::data_for("definitely-unknown") as *const GameData;
        let b = super::data_for("also-unknown") as *const GameData;
        assert_eq!(a, b, "patchless ids must share one base cache entry");
        let base = super::data_for("definitely-unknown");
        let s9 = super::data_for("s9");
        assert_eq!(base.affixes.len(), s9.affixes.len());
    }

    // Every embedded season patch dir must deserialize into GameData without panicking.
    #[test]
    fn sweep_embedded_season_patch_dirs_load() {
        let mut dirs: HashSet<&str> = HashSet::new();
        for (rel, _) in SEASON_PATCHES {
            if let Some((dir, _)) = rel.split_once('/') {
                dirs.insert(dir);
            }
        }
        for dir in dirs {
            let d = super::data_for(dir);
            assert!(!d.affixes.is_empty(), "season {dir} lost all affixes");
        }
        let s9 = super::data_for("s9");
        assert!(!s9.affixes.is_empty());
    }

    #[test]
    fn data_reads_thread_local_season_scope() {
        let default_ptr = super::data() as *const GameData;
        let _scope = crate::calc::season::SeasonScope::enter(Some(
            "scope-unknown-season".to_string(),
        ));
        let scoped_ptr = super::data() as *const GameData;
        assert_eq!(default_ptr, scoped_ptr);
    }

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

    #[test]
    fn can_star_forge_gates_charms_by_season() {
        assert!(super::can_star_forge("weapon", "s9"));
        assert!(super::can_star_forge("weapon", "s10"));
        assert!(!super::can_star_forge("charm_1", "s9"));
        assert!(super::can_star_forge("charm_1", "s10"));
        assert!(super::can_star_forge("charm_30", "s11"));
        assert!(!super::can_star_forge("relic", "s10"));
        assert!(super::is_charm_slot("charm_1"));
        assert!(!super::is_charm_slot("weapon"));
        assert!(!super::charms_allow_stars_forge("s9"));
        assert!(super::charms_allow_stars_forge("s10"));
    }
}
