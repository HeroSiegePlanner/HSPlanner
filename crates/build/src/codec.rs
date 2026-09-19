use serde_json::{Value, json};

use crate::{BuildSnapshot, loadout::Loadouts, notes::Notes};

pub const MAX_CODE_LENGTH: usize = 200_000;

const FIELDS: &[(&str, &str)] = &[
    ("c", "classId"),
    ("l", "level"),
    ("a", "allocated"),
    ("i", "inventory"),
    ("s", "skillRanks"),
    ("ss", "subskillRanks"),
    ("msp", "maxSubskillPoints"),
    ("t", "allocatedTreeNodes"),
    ("m", "activeSkillIds"),
    ("u", "activeAuraId"),
    ("buf", "activeBuffs"),
    ("ec", "enemyConditions"),
    ("pc", "playerConditions"),
    ("sp", "skillProjectiles"),
    ("er", "enemyResistances"),
    ("pt", "procToggles"),
    ("dp", "disabledPotions"),
    ("kps", "killsPerSec"),
    ("df", "difficulty"),
    ("ts", "treeSocketed"),
    ("et", "allocatedEtherNodes"),
    ("mc", "mercClassId"),
    ("ms", "mercSkillRanks"),
    ("mi", "mercInventory"),
    ("mda", "mercDisabledAuras"),
    ("se", "season"),
];

pub fn encode(snapshot: &BuildSnapshot, notes: &Notes) -> Result<String, String> {
    let value = serde_json::to_value(snapshot).map_err(|e| e.to_string())?;
    let mut wire = serde_json::Map::new();
    wire.insert("v".into(), json!(2));
    for &(short, long) in FIELDS {
        if let Some(value) = value.get(long) {
            wire.insert(short.into(), value.clone());
        }
    }
    for key in ["t", "et"] {
        if let Some(Value::Array(values)) = wire.get_mut(key) {
            values.sort_by_key(|v| v.as_u64());
            values.dedup();
        }
    }
    for key in ["i", "mi"] {
        if let Some(Value::Object(items)) = wire.get_mut(key) {
            for item in items.values_mut() {
                remove_null_properties(item);
                if let Some(item) = item.as_object_mut() {
                    for key in ["implicitOverrides", "skillBonusOverrides"] {
                        if item
                            .get(key)
                            .and_then(Value::as_object)
                            .is_some_and(|value| value.is_empty())
                        {
                            item.remove(key);
                        }
                    }
                }
            }
        }
    }
    if let Some(sockets) = wire.get_mut("ts") {
        remove_null_properties(sockets);
    }
    wire.insert(
        "cs".into(),
        Value::Array(
            snapshot
                .custom_stats
                .iter()
                .map(|s| json!({"k": s.stat_key, "v": s.value}))
                .collect(),
        ),
    );
    wire.insert("n".into(), json!(notes.to_html()));
    let json = serde_json::to_string(&wire).map_err(|e| e.to_string())?;
    if json.len() > MAX_CODE_LENGTH {
        return Err("This build is too large to share as a code.".into());
    }
    Ok(lz_str::compress_to_encoded_uri_component(json.as_str()))
}

fn remove_null_properties(value: &mut Value) {
    match value {
        Value::Object(object) => {
            object.retain(|_, value| !value.is_null());
            for value in object.values_mut() {
                remove_null_properties(value);
            }
        }
        Value::Array(array) => {
            for value in array {
                remove_null_properties(value);
            }
        }
        _ => {}
    }
}

/// Share the complete build, including inactive loadouts and common settings.
pub fn encode_loadouts(
    snapshot: &BuildSnapshot,
    notes: &Notes,
    loadouts: &Loadouts,
) -> Result<String, String> {
    loadouts.validate()?;
    let mut loadouts = loadouts.clone();
    loadouts.capture_active(snapshot);
    let json = serde_json::to_string(&json!({
        "v": 3, "snapshot": snapshot, "loadouts": loadouts, "n": notes.to_html(),
    }))
    .map_err(|error| error.to_string())?;
    if json.len() > MAX_CODE_LENGTH {
        return Err("This build is too large to share as a code.".into());
    }
    Ok(lz_str::compress_to_encoded_uri_component(json.as_str()))
}

pub fn decode_loadouts(input: &str) -> Result<(BuildSnapshot, Notes, Loadouts), String> {
    let wire = decompress(input)?;
    decode_loadout_value(&wire)
}

fn decode_loadout_value(wire: &Value) -> Result<(BuildSnapshot, Notes, Loadouts), String> {
    if wire["v"].as_u64() != Some(3) {
        let (snapshot, notes) = decode_value(wire)?;
        let loadouts = Loadouts::from_snapshot(&snapshot);
        return Ok((snapshot, notes, loadouts));
    }
    let mut snapshot = wire["snapshot"].clone();
    let fields = snapshot
        .as_object_mut()
        .ok_or("Invalid shared build fields.")?;
    if let Some(level) = fields.get_mut("level") {
        *level = json!(
            level
                .as_f64()
                .ok_or("Invalid character level.")?
                .floor()
                .clamp(1., 10_000.) as u32
        );
    }
    for key in ["inventory", "mercInventory"] {
        if let Some(inventory) = fields.get_mut(key) {
            normalize_loadout_inventory(inventory)?;
        }
    }
    let mut loadouts = wire["loadouts"].clone();
    let gear = loadouts
        .get_mut("gear")
        .and_then(|gear| gear.get_mut("entries"))
        .and_then(Value::as_array_mut)
        .ok_or("Invalid gear loadouts.")?;
    for entry in gear {
        let value = entry
            .get_mut("value")
            .and_then(Value::as_object_mut)
            .ok_or("Invalid gear loadout.")?;
        for key in ["inventory", "mercInventory"] {
            normalize_loadout_inventory(value.get_mut(key).ok_or("Missing equipment list.")?)?;
        }
    }
    let mut snapshot: BuildSnapshot = serde_json::from_value(snapshot)
        .map_err(|error| format!("Invalid build fields: {error}"))?;
    let mut loadouts: Loadouts =
        serde_json::from_value(loadouts).map_err(|error| format!("Invalid loadouts: {error}"))?;
    loadouts.validate()?;
    if snapshot.season != "s10" {
        loadouts.clear_trees();
        snapshot.season = "s10".into();
    }
    loadouts.apply_active(&mut snapshot);
    Ok((
        snapshot,
        Notes::from_html(wire["n"].as_str().unwrap_or("")),
        loadouts,
    ))
}

pub fn decode(input: &str) -> Result<(BuildSnapshot, Notes), String> {
    decode_value(&decompress(input)?)
}

fn decompress(input: &str) -> Result<Value, String> {
    let code = parse_input(input)?;
    if code.len() > MAX_CODE_LENGTH {
        return Err("Build code is too large.".into());
    }
    let utf16 =
        lz_str::decompress_from_encoded_uri_component(&code).ok_or("Invalid build code.")?;
    if utf16.len() > MAX_CODE_LENGTH {
        return Err("Build code expands beyond the supported size.".into());
    }
    let json = String::from_utf16(&utf16).map_err(|_| "Invalid text in build code.")?;
    let wire: Value = serde_json::from_str(&json).map_err(|_| "Invalid build data.")?;
    Ok(wire)
}

pub fn decode_value(wire: &Value) -> Result<(BuildSnapshot, Notes), String> {
    if wire["v"].as_u64() == Some(3) {
        return decode_loadout_value(wire).map(|(snapshot, notes, _)| (snapshot, notes));
    }
    if !matches!(wire["v"].as_u64(), Some(1 | 2)) {
        return Err("Unsupported build code version.".into());
    }
    for key in ["a", "i", "s", "ss", "buf", "ec", "pt"] {
        if !wire[key].is_object() {
            return Err(format!("Missing or invalid build field: {key}"));
        }
    }
    if !wire["t"].is_array() || !wire["l"].is_number() || !wire["kps"].is_number() {
        return Err("Incomplete build code.".into());
    }
    let mut snapshot = serde_json::Map::new();
    for &(short, long) in FIELDS {
        if let Some(value) = wire.get(short) {
            snapshot.insert(long.into(), value.clone());
        }
    }
    snapshot.insert(
        "level".into(),
        json!(wire["l"].as_f64().unwrap_or(1.).floor().clamp(1., 10_000.) as u32),
    );
    snapshot.insert(
        "activeSkillIds".into(),
        match &wire["m"] {
            Value::String(id) => json!([id]),
            Value::Array(ids) => json!(ids),
            _ => json!([]),
        },
    );
    snapshot.insert("customStats".into(), Value::Array(wire["cs"].as_array().into_iter().flatten().map(|s| json!({"statKey": s["k"].as_str().unwrap_or(""), "value": s["v"].as_str().unwrap_or("")})).collect()));
    if let Some(Value::Object(sockets)) = snapshot.get_mut("treeSocketed") {
        sockets.retain(|key, v| key.parse::<u32>().is_ok() && !v.is_null());
    }
    for key in ["inventory", "mercInventory"] {
        if let Some(value) = snapshot.get_mut(key) {
            normalize_inventory(value)?;
        }
    }
    let nodes = snapshot
        .entry("allocatedTreeNodes")
        .or_insert_with(|| json!([]));
    if let Value::Array(nodes) = nodes {
        nodes.extend(wire["it"].as_array().into_iter().flatten().cloned());
        let mut seen = std::collections::HashSet::new();
        nodes.retain(|v| v.as_u64().is_some_and(|id| seen.insert(id)));
    }
    if wire["se"].as_str() != Some("s10") {
        for key in ["allocatedTreeNodes", "allocatedEtherNodes"] {
            snapshot.insert(key.into(), json!([]));
        }
        snapshot.insert("treeSocketed".into(), json!({}));
        snapshot.insert("season".into(), json!("s10"));
    }
    let mut decoded: BuildSnapshot = serde_json::from_value(Value::Object(snapshot))
        .map_err(|e| format!("Invalid build fields: {e}"))?;
    for nodes in [
        &mut decoded.allocated_tree_nodes,
        &mut decoded.allocated_ether_nodes,
    ] {
        if nodes.len() > 10_000 {
            return Err("Too many tree nodes.".into());
        }
    }
    Ok((decoded, Notes::from_html(wire["n"].as_str().unwrap_or(""))))
}

fn normalize_loadout_inventory(value: &mut Value) -> Result<(), String> {
    let unspecified_stars: Vec<_> = value
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, item)| item.get("stars").is_none_or(Value::is_null))
        .map(|(slot, _)| slot.clone())
        .collect();
    normalize_inventory(value)?;
    for slot in unspecified_stars {
        if let Some(item) = value.get_mut(slot).and_then(Value::as_object_mut) {
            item.remove("stars");
        }
    }
    Ok(())
}

fn normalize_inventory(value: &mut Value) -> Result<(), String> {
    let inventory = value.as_object_mut().ok_or("Invalid equipment list.")?;
    if inventory.len() > 5_000 {
        return Err("Too many equipment slots.".into());
    }
    inventory.retain(|_, v| !v.is_null());
    for item in inventory.values_mut() {
        let item = item.as_object_mut().ok_or("Invalid equipped item.")?;
        let count = item
            .get("socketCount")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(32) as usize;
        item.insert("socketCount".into(), json!(count));
        for (key, default) in [("socketed", Value::Null), ("socketTypes", json!("normal"))] {
            let mut values = item
                .get(key)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            values.resize(count, default);
            item.insert(key.into(), json!(values));
        }
        if let Some(augment) = item.get_mut("augment").and_then(Value::as_object_mut) {
            let level = augment
                .get("level")
                .and_then(Value::as_f64)
                .unwrap_or(1.)
                .floor()
                .clamp(1., 7.) as u32;
            augment.insert("level".into(), json!(level));
        }
        let stars = item
            .get("stars")
            .and_then(Value::as_f64)
            .unwrap_or(0.)
            .floor()
            .clamp(0., 5.) as u32;
        item.insert("stars".into(), json!(stars));
        for key in ["affixes", "forgedMods"] {
            if !item.get(key).is_some_and(Value::is_array) {
                item.insert(key.into(), json!([]));
            }
        }
    }
    Ok(())
}

fn parse_input(input: &str) -> Result<String, String> {
    let value = input.trim();
    if value.len() > MAX_CODE_LENGTH {
        return Err("Build code is too large.".into());
    }
    let code = ["#b=", "?b=", "&b="]
        .iter()
        .find_map(|prefix| {
            value
                .split_once(prefix)
                .map(|(_, v)| v.split(['&', ' ', '\n']).next().unwrap_or(v))
        })
        .unwrap_or(value);
    let mut bytes = Vec::with_capacity(code.len());
    let mut chars = code.bytes();
    while let Some(ch) = chars.next() {
        if ch == b'%' {
            let a = chars
                .next()
                .and_then(|b| char::from(b).to_digit(16))
                .ok_or("Invalid escaped build code.")?;
            let b = chars
                .next()
                .and_then(|b| char::from(b).to_digit(16))
                .ok_or("Invalid escaped build code.")?;
            bytes.push((a * 16 + b) as u8);
        } else {
            bytes.push(ch);
        }
    }
    String::from_utf8(bytes).map_err(|_| "Invalid build code text.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn codes_roundtrip_unicode_equipment_and_old_single_skill_shape() {
        let snapshot = BuildSnapshot {
            active_skill_ids: vec!["charged_bolts".into()],
            allocated_tree_nodes: vec![5, 1, 3],
            max_subskill_points: 27,
            ..Default::default()
        };
        let notes = Notes {
            markdown: "# Żółw 🐢\n\n**Fire**".into(),
            original_html: None,
        };
        let code = encode(&snapshot, &notes).unwrap();
        let (read, notes) = decode(&format!("https://example.com/#b={code}")).unwrap();
        assert_eq!(read.active_skill_ids, snapshot.active_skill_ids);
        assert_eq!(read.allocated_tree_nodes, [1, 3, 5]);
        assert_eq!(read.subskill_point_budget(), 27);
        assert!(notes.markdown.contains("Żółw 🐢"));
        let utf16 = lz_str::decompress_from_encoded_uri_component(&code).unwrap();
        let mut wire: Value = serde_json::from_str(&String::from_utf16(&utf16).unwrap()).unwrap();
        wire.as_object_mut().unwrap().remove("msp");
        assert_eq!(decode_value(&wire).unwrap().0.subskill_point_budget(), 20);
        wire["msp"] = json!(99);
        assert_eq!(decode_value(&wire).unwrap().0.subskill_point_budget(), 30);
        wire["v"] = json!(1);
        wire["m"] = json!("charged_bolts");
        assert_eq!(
            decode_value(&wire).unwrap().0.active_skill_ids,
            ["charged_bolts"]
        );
    }
    #[test]
    fn unknown_versions_fail_and_unknown_seasons_clear_only_season_allocations() {
        let snapshot = BuildSnapshot {
            allocated_tree_nodes: vec![5],
            allocated_ether_nodes: vec![8],
            season: "s9".into(),
            ..Default::default()
        };
        let code = encode(&snapshot, &Notes::default()).unwrap();
        let (decoded, _) = decode(&code).unwrap();
        assert!(decoded.allocated_tree_nodes.is_empty());
        assert!(decoded.allocated_ether_nodes.is_empty());
        assert_eq!(decoded.class_id, snapshot.class_id);
        assert!(decode_value(&json!({"v": 999})).is_err());
    }
}
