//! Game loot-filter codec (base64 JSON delta the game exports) and the
//! "generate from build" mapping of gear stats onto the filter's affix ids.
use std::collections::BTreeMap;

use base64::engine::general_purpose::GeneralPurpose;
use base64::engine::{DecodePaddingMode, GeneralPurposeConfig};
use base64::Engine as _;
use std::sync::LazyLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::data;
use super::types::{AffixFormat, Inventory};

const FILTER_VERSION: i64 = 2;
const TIER_COUNT: usize = 5;
pub const DEFAULT_RS: i64 = 0b11111100000;
pub const DEFAULT_SOC: i64 = 0b111111;
pub const DEFAULT_SOCH: i64 = 0;
pub const DEFAULT_WTC: i64 = 0b111111111111111111;
pub const FILTER_TYPE_IDS: [u32; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 15, 18];

// atob() tolerates missing padding; the game always pads, players' pastes may not.
static BASE64: LazyLock<GeneralPurpose> = LazyLock::new(|| {
    GeneralPurpose::new(
        &base64::alphabet::STANDARD,
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
    )
});

#[derive(Debug, Clone, Deserialize)]
pub struct FilterStat {
    pub id: i64,
    pub name: String,
    pub col: u32,
}

pub static FILTER_STATS: LazyLock<Vec<FilterStat>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/lootfilter-stats.json"))
        .expect("data/lootfilter-stats.json")
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootFilterTier {
    pub rs: i64,
    pub hidden: Vec<i64>,
    pub highlighted: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootFilterType {
    pub tiers: Vec<LootFilterTier>,
    pub soc: i64,
    pub soch: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootFilter {
    pub version: i64,
    pub types: BTreeMap<u32, LootFilterType>,
    pub wtc: i64,
}

fn default_tier() -> LootFilterTier {
    LootFilterTier {
        rs: DEFAULT_RS,
        hidden: Vec::new(),
        highlighted: Vec::new(),
    }
}

fn default_type() -> LootFilterType {
    LootFilterType {
        tiers: (0..TIER_COUNT).map(|_| default_tier()).collect(),
        soc: DEFAULT_SOC,
        soch: DEFAULT_SOCH,
    }
}

pub fn default_filter() -> LootFilter {
    LootFilter {
        version: FILTER_VERSION,
        types: FILTER_TYPE_IDS
            .iter()
            .map(|id| (*id, default_type()))
            .collect(),
        wtc: DEFAULT_WTC,
    }
}

// ---------- decode ----------

// JS Number() coercion: null → 0, bools → 0/1, numeric strings parse, else NaN.
fn js_number(value: &Value) -> Option<f64> {
    match value {
        Value::Null => Some(0.0),
        Value::Bool(b) => Some(f64::from(u8::from(*b))),
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                Some(0.0)
            } else {
                t.parse().ok()
            }
        }
        _ => None,
    }
}

fn to_int(value: Option<&Value>, fallback: i64) -> i64 {
    value
        .and_then(js_number)
        .filter(|n| n.is_finite())
        .map(|n| n.trunc() as i64)
        .unwrap_or(fallback)
}

fn to_id_list(value: Option<&Value>) -> Vec<i64> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(js_number)
        .filter(|n| n.is_finite() && *n >= 0.0)
        .map(|n| n.trunc() as i64)
        .collect()
}

fn parse_tier(raw: Option<&Value>) -> LootFilterTier {
    let Some(Value::Object(t)) = raw else {
        return default_tier();
    };
    LootFilterTier {
        rs: to_int(t.get("rs"), DEFAULT_RS),
        hidden: to_id_list(t.get("hs")),
        highlighted: to_id_list(t.get("hls")),
    }
}

fn parse_type(raw: &Value) -> LootFilterType {
    let Value::Object(t) = raw else {
        return default_type();
    };
    LootFilterType {
        tiers: (0..TIER_COUNT)
            .map(|i| {
                let key = format!("tr{i}");
                if t.contains_key(&key) {
                    parse_tier(t.get(&key))
                } else {
                    default_tier()
                }
            })
            .collect(),
        soc: to_int(t.get("soc"), DEFAULT_SOC),
        soch: to_int(t.get("soch"), DEFAULT_SOCH),
    }
}

pub fn decode(code: &str) -> Option<LootFilter> {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return None;
    }
    let bytes = BASE64.decode(trimmed).ok()?;
    let parsed: Value = serde_json::from_slice(&bytes).ok()?;
    let Value::Object(obj) = parsed else {
        return None;
    };
    let mut types: BTreeMap<u32, LootFilterType> = default_filter().types;
    for (key, value) in &obj {
        if let Some(id) = key.strip_prefix('t').and_then(|s| s.parse::<u32>().ok()) {
            types.insert(id, parse_type(value));
        }
    }
    Some(LootFilter {
        version: to_int(obj.get("version"), FILTER_VERSION),
        types,
        wtc: to_int(obj.get("wtc"), DEFAULT_WTC),
    })
}

// ---------- encode ----------

// GameMaker quirk: the last array element is written as a float ("75.0").
fn serialize_ids(ids: &[i64]) -> String {
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    let parts: Vec<String> = sorted
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if i == sorted.len() - 1 {
                format!("{n}.0")
            } else {
                n.to_string()
            }
        })
        .collect();
    format!("[{}]", parts.join(","))
}

fn serialize_tier(tier: &LootFilterTier) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if tier.rs != DEFAULT_RS {
        parts.push(format!("\"rs\":{}", tier.rs));
    }
    if !tier.hidden.is_empty() {
        parts.push(format!("\"hs\":{}", serialize_ids(&tier.hidden)));
    }
    if !tier.highlighted.is_empty() {
        parts.push(format!("\"hls\":{}", serialize_ids(&tier.highlighted)));
    }
    (!parts.is_empty()).then(|| format!("{{{}}}", parts.join(",")))
}

fn serialize_type(t: &LootFilterType) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for (i, tier) in t.tiers.iter().enumerate() {
        if let Some(body) = serialize_tier(tier) {
            parts.push(format!("\"tr{i}\":{body}"));
        }
    }
    if t.soc != DEFAULT_SOC {
        parts.push(format!("\"soc\":{}", t.soc));
    }
    if t.soch != DEFAULT_SOCH {
        parts.push(format!("\"soch\":{}", t.soch));
    }
    (!parts.is_empty()).then(|| format!("{{{}}}", parts.join(",")))
}

pub fn encode(filter: &LootFilter) -> String {
    let mut parts: Vec<String> = vec![format!("\"version\":{}", filter.version)];
    for (id, t) in &filter.types {
        if let Some(body) = serialize_type(t) {
            parts.push(format!("\"t{id}\":{body}"));
        }
    }
    if filter.wtc != DEFAULT_WTC {
        parts.push(format!("\"wtc\":{}", filter.wtc));
    }
    BASE64.encode(format!("{{{}}}", parts.join(",")))
}

// ---------- generate from build ----------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuildAffixStat {
    pub stat_key: String,
    pub format: AffixFormat,
}

// Filter names that differ from the stat catalog; a pair splits flat vs percent.
const STAT_ALIASES: &[(&str, &str, &str)] = &[
    (
        "fire_skill_damage",
        "Flat Fire Skill Damage",
        "Fire Skill Damage %",
    ),
    (
        "cold_skill_damage",
        "Flat Cold Skill Damage",
        "Cold Skill Damage %",
    ),
    (
        "lightning_skill_damage",
        "Flat Lightning Skill Damage",
        "Lightning Skill Damage %",
    ),
    (
        "poison_skill_damage",
        "Flat Poison Skill Damage",
        "Poison Skill Damage %",
    ),
    (
        "arcane_skill_damage",
        "Flat Arcane Skill Damage",
        "Arcane Skill Damage %",
    ),
    (
        "additive_physical_damage",
        "Flat Physical Damage",
        "Physical Damage %",
    ),
    ("attack_rating_pct", "Attack Rating %", "Attack Rating %"),
    (
        "all_damage_taken_reduced_pct",
        "All damage reduction",
        "All damage reduction",
    ),
    ("damage_taken_reduced", "Damage reduced", "Damage reduced"),
    (
        "magic_damage_reduction_pct",
        "Magic damage reduction",
        "Magic damage reduction",
    ),
    (
        "extra_damage_burning",
        "Extra damage to Burning",
        "Extra damage to Burning",
    ),
    ("life_replenish_pct", "Life Replenish", "Life Replenish"),
    ("mana_replenish_pct", "Mana Replenish", "Mana Replenish"),
    ("ranged_range", "Attack Range", "Attack Range"),
];

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).expect(stringify!($name)));
    };
}
re!(NORM_PREFIX, r"^(to|increased|extra)\s+");
re!(NORM_MAX, r"\bmaximum\b");
re!(NORM_RES, r"\bresists?\b");
re!(NORM_DEF, r"\bdefence\b");
re!(NORM_SEC, r"\bper sec(ond)?\b");
re!(NORM_FREQ, r"\bfreq(uency)?\b");
re!(NORM_JUNK, r"[^a-z0-9]+");

fn normalize(name: &str) -> String {
    let s = name.to_lowercase();
    let s = NORM_PREFIX.replace(s.trim(), "");
    let s = s.replace('%', " percent");
    let s = NORM_MAX.replace_all(&s, "max");
    let s = NORM_RES.replace_all(&s, "resistance");
    let s = NORM_DEF.replace_all(&s, "defense");
    let s = NORM_SEC.replace_all(&s, "per second");
    let s = NORM_FREQ.replace_all(&s, "frequency");
    NORM_JUNK.replace_all(&s, " ").trim().to_string()
}

static FILTER_ID_BY_NAME: LazyLock<BTreeMap<String, i64>> = LazyLock::new(|| {
    let mut map = BTreeMap::new();
    for stat in FILTER_STATS.iter() {
        map.entry(normalize(&stat.name)).or_insert(stat.id);
    }
    map
});

fn stat_catalog_name(key: &str) -> String {
    data::game_config()
        .stats
        .iter()
        .find(|d| d.key == key)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| key.to_string())
}

fn catalog_format(key: &str) -> AffixFormat {
    let is_percent = data::game_config()
        .stats
        .iter()
        .find(|d| d.key == key)
        .and_then(|d| d.format.as_deref())
        == Some("percent");
    if is_percent {
        AffixFormat::Percent
    } else {
        AffixFormat::Flat
    }
}

fn resolve_filter_stat_id(stat: &BuildAffixStat) -> Option<i64> {
    if let Some((_, flat, percent)) = STAT_ALIASES.iter().find(|(k, _, _)| *k == stat.stat_key) {
        let alias = if stat.format == AffixFormat::Percent {
            percent
        } else {
            flat
        };
        return FILTER_ID_BY_NAME.get(&normalize(alias)).copied();
    }
    let base = normalize(&stat_catalog_name(&stat.stat_key));
    let candidates = if stat.format == AffixFormat::Percent {
        vec![format!("{base} percent"), base]
    } else {
        vec![base]
    };
    candidates
        .iter()
        .find_map(|c| FILTER_ID_BY_NAME.get(c).copied())
}

pub fn filter_stat_ids_for(stats: &[BuildAffixStat]) -> Vec<i64> {
    let mut ids: Vec<i64> = Vec::new();
    for stat in stats {
        if let Some(id) = resolve_filter_stat_id(stat) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    ids
}

/// Every stat the gear rolls: affixes, forged mods and unique implicits.
pub fn collect_build_stats(inventory: &Inventory) -> Vec<BuildAffixStat> {
    let mut seen: std::collections::BTreeSet<BuildAffixStat> = std::collections::BTreeSet::new();
    for equipped in inventory.values() {
        for eq in equipped.affixes.iter().chain(equipped.forged_mods.iter()) {
            let Some(affix) =
                data::get_affix(&eq.affix_id).or_else(|| data::get_crystal_mod(&eq.affix_id))
            else {
                continue;
            };
            if let Some(key) = affix.stat_key.as_ref() {
                seen.insert(BuildAffixStat {
                    stat_key: key.clone(),
                    format: affix.format,
                });
            }
        }
        let Some(base) = data::get_item(&equipped.base_id) else {
            continue;
        };
        for key in base.implicit.iter().flat_map(|m| m.keys()) {
            seen.insert(BuildAffixStat {
                stat_key: key.clone(),
                format: catalog_format(key),
            });
        }
    }
    seen.into_iter().collect()
}

pub fn build_filter_for_stats(stat_ids: &[i64], hide_rest: bool) -> LootFilter {
    let mut highlighted: Vec<i64> = Vec::new();
    for id in stat_ids {
        if !highlighted.contains(id) {
            highlighted.push(*id);
        }
    }
    let hidden: Vec<i64> = if hide_rest && !highlighted.is_empty() {
        FILTER_STATS
            .iter()
            .map(|s| s.id)
            .filter(|id| !highlighted.contains(id))
            .collect()
    } else {
        Vec::new()
    };
    let mut filter = default_filter();
    for t in filter.types.values_mut() {
        for tier in &mut t.tiers {
            tier.hidden = hidden.clone();
            tier.highlighted = highlighted.clone();
        }
    }
    filter
}

// ---------- commands ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildFilterStats {
    pub stat_ids: Vec<i64>,
    /// Gear stats with no counterpart in the game's filter list.
    pub unmatched: usize,
}

#[tauri::command]
pub fn lootfilter_decode(code: String) -> Option<LootFilter> {
    decode(&code)
}

#[tauri::command]
pub fn lootfilter_encode(filter: LootFilter) -> String {
    encode(&filter)
}

#[tauri::command]
pub fn lootfilter_build_stats(inventory: Inventory, season: Option<String>) -> BuildFilterStats {
    let _scope = super::season::SeasonScope::enter(season);
    let stats = collect_build_stats(&inventory);
    let stat_ids = filter_stat_ids_for(&stats);
    BuildFilterStats {
        unmatched: stats.len() - stat_ids.len(),
        stat_ids,
    }
}

#[tauri::command]
pub fn lootfilter_code_for_stats(stat_ids: Vec<i64>, hide_rest: bool) -> String {
    encode(&build_filter_for_stats(&stat_ids, hide_rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::types::{EquippedAffix, EquippedItem};

    const HELL_FILTER_S9: &str = include_str!("../../tests/fixtures/lootfilter/hell_filter_s9.b64");
    const GAME_AFFIX_CODES: &str =
        include_str!("../../tests/fixtures/lootfilter/game_affix_codes.json");

    fn b64(json: &str) -> String {
        BASE64.encode(json)
    }

    fn json_of(code: &str) -> Value {
        serde_json::from_slice(&BASE64.decode(code).unwrap()).unwrap()
    }

    fn normalized(mut filter: LootFilter) -> LootFilter {
        for t in filter.types.values_mut() {
            for tier in &mut t.tiers {
                tier.hidden.sort_unstable();
                tier.highlighted.sort_unstable();
            }
        }
        filter
    }

    fn id_of(name: &str) -> i64 {
        FILTER_STATS.iter().find(|s| s.name == name).unwrap().id
    }

    fn name_of(id: i64) -> &'static str {
        &FILTER_STATS.iter().find(|s| s.id == id).unwrap().name
    }

    #[test]
    fn decodes_a_real_game_export() {
        let f = decode(HELL_FILTER_S9).expect("decodes");
        assert_eq!(f.version, 2);
        assert_eq!(f.wtc, 212031);
        let helmet = &f.types[&0];
        assert_eq!(helmet.tiers.len(), 5);
        assert_eq!(helmet.tiers[3].rs, 2016);
        assert_eq!(helmet.tiers[3].hidden.len(), 155);
        assert!(helmet.tiers[3].highlighted.contains(&201));
        assert_eq!(helmet.tiers[4].rs, 2017);
        assert_eq!(helmet.soc, 62);
        assert_eq!(helmet.soch, 62);
        assert_eq!(f.types[&4].soc, DEFAULT_SOC);
        for id in FILTER_TYPE_IDS {
            assert!(f.types.contains_key(&id));
        }
    }

    #[test]
    fn tolerates_gamemaker_floats_and_delta() {
        let json = r#"{"version":2,"t0":{"tr4":{"hls":[75.0]},"soc":59,"tr1":{"hs":[75.0]}},"wtc":260959}"#;
        let f = decode(&b64(json)).unwrap();
        assert_eq!(f.types[&0].tiers[4].highlighted, vec![75]);
        assert_eq!(f.types[&0].tiers[1].hidden, vec![75]);
        assert_eq!(f.types[&0].tiers[0].rs, DEFAULT_RS);
        assert!(f.types[&0].tiers[0].hidden.is_empty());
        assert_eq!(f.types[&0].soc, 59);
        assert_eq!(f.wtc, 260959);
        assert_eq!(f.types[&3].tiers[2].rs, DEFAULT_RS);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode("").is_none());
        assert!(decode("nie-base64 !!!").is_none());
        assert!(decode(&b64("[]")).is_none());
        assert!(decode(&b64("\"tekst\"")).is_none());
        assert!(decode(&b64("{zepsuty json")).is_none());
    }

    #[test]
    fn round_trip_keeps_semantics() {
        let original = decode(HELL_FILTER_S9).unwrap();
        let round = decode(&encode(&original)).unwrap();
        assert_eq!(normalized(round), normalized(original));
    }

    #[test]
    fn default_filter_encodes_to_version_only() {
        assert_eq!(
            json_of(&encode(&default_filter())),
            serde_json::json!({ "version": 2 })
        );
    }

    #[test]
    fn last_array_element_is_a_float() {
        let mut filter = default_filter();
        filter.types.get_mut(&0).unwrap().tiers[1].hidden = vec![26, 75];
        let json = String::from_utf8(BASE64.decode(encode(&filter)).unwrap()).unwrap();
        assert!(json.contains("\"hs\":[26,75.0]"), "{json}");
    }

    #[test]
    fn skips_default_values() {
        let mut filter = default_filter();
        filter.wtc = 260959;
        let t3 = filter.types.get_mut(&3).unwrap();
        t3.soc = 59;
        t3.tiers[4].rs = 2017;
        let parsed = json_of(&encode(&filter));
        let mut keys: Vec<&String> = parsed.as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, vec!["t3", "version", "wtc"]);
        assert_eq!(
            parsed["t3"],
            serde_json::json!({ "tr4": { "rs": 2017 }, "soc": 59 })
        );
        assert_eq!(parsed["wtc"], 260959);
        assert!(json_of(&encode(&default_filter())).get("wtc").is_none());
    }

    #[derive(Deserialize)]
    struct GameAffixCode {
        name: String,
        id: i64,
        code: String,
    }

    fn hide_affix(stat_id: i64) -> LootFilter {
        let mut filter = default_filter();
        filter.types.get_mut(&0).unwrap().tiers[0].hidden = vec![stat_id];
        filter
    }

    #[test]
    fn affix_ids_calibrated_on_game_exports() {
        let codes: Vec<GameAffixCode> = serde_json::from_str(GAME_AFFIX_CODES).unwrap();
        assert_eq!(codes.len(), 66);
        let unique: std::collections::HashSet<i64> = codes.iter().map(|c| c.id).collect();
        assert_eq!(unique.len(), 66);
        for c in &codes {
            let decoded = decode(&c.code).unwrap_or_else(|| panic!("{}", c.name));
            assert_eq!(decoded.types[&0].tiers[0].hidden, vec![c.id], "{}", c.name);
            assert_eq!(name_of(c.id), c.name);
            assert_eq!(encode(&hide_affix(c.id)), c.code, "{}", c.name);
        }
    }

    #[test]
    fn filter_stat_list_matches_the_game_layout() {
        assert_eq!(FILTER_STATS.len(), 165);
        let unique: std::collections::HashSet<i64> = FILTER_STATS.iter().map(|s| s.id).collect();
        assert_eq!(unique.len(), 165);
        let cols: std::collections::HashSet<u32> = FILTER_STATS.iter().map(|s| s.col).collect();
        assert_eq!(cols, [1, 2, 3, 4].into_iter().collect());
        for col in 1..=4 {
            let ids: Vec<i64> = FILTER_STATS
                .iter()
                .filter(|s| s.col == col)
                .map(|s| s.id)
                .collect();
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            assert_eq!(ids, sorted, "col {col}");
        }
    }

    fn stat(key: &str, format: AffixFormat) -> BuildAffixStat {
        BuildAffixStat {
            stat_key: key.into(),
            format,
        }
    }

    #[test]
    fn maps_build_stats_onto_filter_affixes() {
        let names = |ids: Vec<i64>| ids.into_iter().map(name_of).collect::<Vec<_>>();
        assert_eq!(
            names(filter_stat_ids_for(&[stat(
                "fire_resistance",
                AffixFormat::Percent
            )])),
            vec!["Fire Resistance"]
        );
        assert_eq!(
            names(filter_stat_ids_for(&[stat("life", AffixFormat::Flat)])),
            vec!["Life"]
        );
        assert_eq!(
            names(filter_stat_ids_for(&[stat(
                "increased_life",
                AffixFormat::Percent
            )])),
            vec!["Life %"]
        );
        assert_eq!(
            names(filter_stat_ids_for(&[stat(
                "fire_skill_damage",
                AffixFormat::Flat
            )])),
            vec!["Flat Fire Skill Damage"]
        );
        assert_eq!(
            names(filter_stat_ids_for(&[stat(
                "fire_skill_damage",
                AffixFormat::Percent
            )])),
            vec!["Fire Skill Damage %"]
        );
        assert!(filter_stat_ids_for(&[stat("nie_ma_takiego", AffixFormat::Flat)]).is_empty());
        let dup = filter_stat_ids_for(&[
            stat("fire_resistance", AffixFormat::Percent),
            stat("fire_resistance", AffixFormat::Percent),
        ]);
        assert_eq!(dup.len(), 1);
    }

    fn item(base_id: &str, affix_ids: &[&str]) -> EquippedItem {
        EquippedItem {
            base_id: base_id.into(),
            affixes: affix_ids
                .iter()
                .map(|id| EquippedAffix {
                    affix_id: id.to_string(),
                    tier: 1,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn collects_gear_stats_and_dedupes() {
        let inventory: Inventory = [
            (
                "helmet".to_string(),
                item(
                    "whatever",
                    &[
                        "7_15_to_fire_resistance_t1_firecloaking",
                        "15_30_to_life_t1_bear",
                    ],
                ),
            ),
            (
                "armor".to_string(),
                item("whatever", &["7_15_to_fire_resistance_t2_fireproof"]),
            ),
        ]
        .into_iter()
        .collect();
        let stats = collect_build_stats(&inventory);
        assert_eq!(stats.len(), 2);
        assert!(stats.contains(&stat("fire_resistance", AffixFormat::Percent)));
        assert!(stats.contains(&stat("life", AffixFormat::Flat)));
        assert!(collect_build_stats(&Inventory::new()).is_empty());
    }

    #[test]
    fn unique_implicits_count_with_catalog_format() {
        let crown: Inventory = [(
            "helmet".to_string(),
            item("helmet_angelic_lucifers_crown", &[]),
        )]
        .into_iter()
        .collect();
        let keys: Vec<String> = collect_build_stats(&crown)
            .into_iter()
            .map(|s| s.stat_key)
            .collect();
        for key in ["enhanced_defense", "all_skills", "all_resistances"] {
            assert!(keys.contains(&key.to_string()), "{key}");
        }
        let mask: Inventory = [(
            "helmet".to_string(),
            item("helmet_angelic_mask_of_the_celestial", &[]),
        )]
        .into_iter()
        .collect();
        let stats = collect_build_stats(&mask);
        assert!(stats.contains(&stat("life", AffixFormat::Flat)));
        assert!(stats.contains(&stat("increased_life", AffixFormat::Percent)));
        let names: Vec<&str> = filter_stat_ids_for(&stats)
            .into_iter()
            .map(name_of)
            .collect();
        assert!(names.contains(&"Life"));
        assert!(names.contains(&"Life %"));
    }

    #[test]
    fn build_filter_highlights_everywhere_and_hides_the_rest() {
        let ids = vec![id_of("Life"), id_of("Fire Resistance")];
        let filter = build_filter_for_stats(&ids, false);
        for t in filter.types.values() {
            for tier in &t.tiers {
                assert_eq!(tier.highlighted, ids);
                assert!(tier.hidden.is_empty());
            }
        }
        let hiding = build_filter_for_stats(&ids, true);
        let tier = &hiding.types[&0].tiers[0];
        assert_eq!(tier.hidden.len(), FILTER_STATS.len() - ids.len());
        for id in &ids {
            assert!(!tier.hidden.contains(id));
        }
        let decoded = decode(&encode(&hiding)).unwrap();
        let mut got = decoded.types[&0].tiers[0].highlighted.clone();
        got.sort_unstable();
        let mut want = ids.clone();
        want.sort_unstable();
        assert_eq!(got, want);
        let empty = build_filter_for_stats(&[], false);
        assert!(empty.types[&0].tiers[0].highlighted.is_empty());
        assert!(empty.types[&0].tiers[0].hidden.is_empty());
    }
}
