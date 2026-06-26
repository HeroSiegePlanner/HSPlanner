use serde_json::{Map, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;

use super::data::SEASON_PATCHES;

// Must match DEFAULT_SEASON_ID in registry.ts; flip both at S10 launch.
pub const DEFAULT_SEASON_ID: &str = "s9";

thread_local! {
    static CURRENT_SEASON: RefCell<Option<String>> = const { RefCell::new(None) };
}

// RAII guard installing the per-command season; !Send so it never crosses an .await.
pub struct SeasonScope {
    _not_send: std::marker::PhantomData<*const ()>,
}

impl SeasonScope {
    #[must_use]
    pub fn enter(season: Option<String>) -> SeasonScope {
        CURRENT_SEASON.with(|c| *c.borrow_mut() = season);
        SeasonScope {
            _not_send: std::marker::PhantomData,
        }
    }
}

impl Drop for SeasonScope {
    fn drop(&mut self) {
        CURRENT_SEASON.with(|c| *c.borrow_mut() = None);
    }
}

pub fn with_current_season<R>(f: impl FnOnce(&str) -> R) -> R {
    CURRENT_SEASON.with(|c| f(c.borrow().as_deref().unwrap_or(DEFAULT_SEASON_ID)))
}

pub fn current_season_id() -> String {
    with_current_season(str::to_string)
}

pub fn has_patches(id: &str) -> bool {
    SEASON_PATCHES.iter().any(|(rel, _)| {
        rel.len() > id.len() && rel.starts_with(id) && rel.as_bytes()[id.len()] == b'/'
    })
}

// All patchless ids share one base-data cache entry so garbage ids can't grow the caches.
pub const BASE_CACHE_KEY: &str = "__base__";

// Sole season-id normalization for the per-season caches: patched ids key under themselves, the rest collapse to BASE_CACHE_KEY.
pub fn cache_key(season_id: &str) -> &str {
    if has_patches(season_id) {
        season_id
    } else {
        BASE_CACHE_KEY
    }
}

// Season id whose patches a cache key loads; "" means pure base data.
pub fn load_id(cache_key: &str) -> &str {
    if cache_key == BASE_CACHE_KEY {
        ""
    } else {
        cache_key
    }
}

// Double-checked-lock cache for leaked per-season singletons.
pub fn cached_per_season<T>(
    cache: &Mutex<HashMap<String, &'static T>>,
    season_id: &str,
    build: impl FnOnce(&str) -> T,
) -> &'static T {
    let key = cache_key(season_id);
    {
        let cache = cache.lock().expect("per-season cache poisoned");
        if let Some(v) = cache.get(key) {
            return v;
        }
    }
    let built = build(load_id(key));
    let mut cache = cache.lock().expect("per-season cache poisoned");
    if let Some(v) = cache.get(key) {
        return v;
    }
    let leaked: &'static T = Box::leak(Box::new(built));
    cache.insert(key.to_string(), leaked);
    leaked
}

// Per-thread memo: one mutex hit per season change, not per call.
pub fn memoized_current_season<T: 'static>(
    cell: &'static std::thread::LocalKey<RefCell<Option<(String, &'static T)>>>,
    lookup: impl Fn(&str) -> &'static T,
) -> &'static T {
    with_current_season(|id| {
        cell.with(|c| {
            if let Some((k, ptr)) = c.borrow().as_ref() {
                if k == id {
                    return *ptr;
                }
            }
            let ptr = lookup(id);
            *c.borrow_mut() = Some((id.to_string(), ptr));
            ptr
        })
    })
}

// rel path "s10/affixes.patch.json" -> key "affixes"
pub fn patches_for(season_id: &str) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    for (rel, content) in SEASON_PATCHES {
        let Some((dir, file)) = rel.split_once('/') else { continue };
        if dir != season_id {
            continue;
        }
        let Some(name) = file.strip_suffix(".patch.json") else { continue };
        match serde_json::from_str::<Value>(content) {
            Ok(v) => {
                out.insert(name.to_string(), v);
            }
            Err(e) => {
                log::error!("season {season_id}: invalid patch {file}: {e}");
            }
        }
    }
    out
}

fn arr<'a>(patch: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    patch
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn obj<'a>(patch: &'a Value, key: &str) -> impl Iterator<Item = (&'a String, &'a Value)> {
    patch
        .get(key)
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
}

// Mirrors the TS zod .strict() schemas: any unknown key fails the whole collection.
fn validate_patch_keys(patch: &Value, allowed: &[&str], label: &str, errors: &mut Vec<String>) {
    let Some(map) = patch.as_object() else { return };
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            errors.push(format!("{label}: unknown patch key \"{key}\""));
        }
    }
}

fn merge_shallow(base: &Value, fields: &Value) -> Value {
    let mut out = base.as_object().cloned().unwrap_or_default();
    if let Some(f) = fields.as_object() {
        for (k, v) in f {
            out.insert(k.clone(), v.clone());
        }
    }
    Value::Object(out)
}

fn value_key(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

// Mirrors applyListPatch in resolve.ts: remove -> change -> add, all-or-nothing per collection.
pub fn apply_list_patch(
    base: &Value,
    patch: &Value,
    label: &str,
    key: &str,
) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();
    validate_patch_keys(patch, &["add", "change", "remove"], label, &mut errors);
    let entries = base.as_array().cloned().unwrap_or_default();
    let mut order: Vec<String> = Vec::with_capacity(entries.len());
    let mut by_key: HashMap<String, Value> = HashMap::with_capacity(entries.len());
    for e in entries {
        let id = e.get(key).map(value_key).unwrap_or_default();
        if !by_key.contains_key(&id) {
            order.push(id.clone());
        }
        by_key.insert(id, e);
    }
    for id in arr(patch, "remove") {
        let id = value_key(id);
        if by_key.remove(&id).is_none() {
            errors.push(format!("{label}: remove unknown id \"{id}\""));
        } else {
            order.retain(|o| o != &id);
        }
    }
    for (id, fields) in obj(patch, "change") {
        match by_key.get(id) {
            Some(cur) => {
                let merged = merge_shallow(cur, fields);
                by_key.insert(id.clone(), merged);
            }
            None => errors.push(format!("{label}: change unknown id \"{id}\"")),
        }
    }
    for entry in arr(patch, "add") {
        let id = entry.get(key).map(value_key).unwrap_or_default();
        if by_key.contains_key(&id) {
            errors.push(format!("{label}: add duplicates id \"{id}\""));
            continue;
        }
        order.push(id.clone());
        by_key.insert(id, entry.clone());
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let out: Vec<Value> = order
        .into_iter()
        .filter_map(|id| by_key.remove(&id))
        .collect();
    Ok(Value::Array(out))
}

// Mirrors applyRecordMergePatch (merge=true) and applyRecordReplacePatch (merge=false).
pub fn apply_record_patch(
    base: &Value,
    patch: &Value,
    label: &str,
    merge: bool,
) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();
    validate_patch_keys(patch, &["add", "change", "remove"], label, &mut errors);
    let mut out: Map<String, Value> = base.as_object().cloned().unwrap_or_default();
    for id in arr(patch, "remove") {
        let id = value_key(id);
        if out.remove(&id).is_none() {
            errors.push(format!("{label}: remove unknown id \"{id}\""));
        }
    }
    for (id, fields) in obj(patch, "change") {
        match out.get(id) {
            Some(cur) => {
                let next = if merge {
                    merge_shallow(cur, fields)
                } else {
                    fields.clone()
                };
                out.insert(id.clone(), next);
            }
            None => errors.push(format!("{label}: change unknown id \"{id}\"")),
        }
    }
    for (id, value) in obj(patch, "add") {
        if out.contains_key(id) {
            errors.push(format!("{label}: add duplicates id \"{id}\""));
            continue;
        }
        out.insert(id.clone(), value.clone());
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Value::Object(out))
}

pub fn apply_game_config_patch(
    base: &Value,
    patch: &Value,
    label: &str,
) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();
    validate_patch_keys(patch, &["change", "stats"], label, &mut errors);
    let mut out = base.as_object().cloned().unwrap_or_default();
    if let Some(change) = patch.get("change").and_then(Value::as_object) {
        for (k, v) in change {
            out.insert(k.clone(), v.clone());
        }
    }
    if let Some(stats_patch) = patch.get("stats") {
        let base_stats = base.get("stats").cloned().unwrap_or(Value::Array(vec![]));
        match apply_list_patch(&base_stats, stats_patch, &format!("{label}.stats"), "key") {
            Ok(stats) => {
                out.insert("stats".to_string(), stats);
            }
            Err(errs) => errors.extend(errs),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARITY: &str = include_str!("../../../src/data/seasons/parity-fixture.json");

    #[test]
    fn list_patch_matches_ts_parity_fixture() {
        let fx: Value = serde_json::from_str(PARITY).unwrap();
        let case = &fx["list"];
        let out = apply_list_patch(&case["base"], &case["patch"], "list", "id").unwrap();
        assert_eq!(out, case["expected"]);
    }

    #[test]
    fn record_patch_matches_ts_parity_fixture() {
        let fx: Value = serde_json::from_str(PARITY).unwrap();
        let case = &fx["record"];
        let out = apply_record_patch(&case["base"], &case["patch"], "record", true).unwrap();
        assert_eq!(out, case["expected"]);
    }

    #[test]
    fn game_config_patch_matches_ts_parity_fixture() {
        let fx: Value = serde_json::from_str(PARITY).unwrap();
        let case = &fx["gameConfig"];
        let out = apply_game_config_patch(&case["base"], &case["patch"], "gameConfig").unwrap();
        assert_eq!(out, case["expected"]);
    }

    #[test]
    fn list_patch_reports_all_error_kinds() {
        let base: Value = serde_json::json!([{ "id": "a", "v": 1 }]);
        let patch: Value = serde_json::json!({
            "remove": ["zz"],
            "change": { "yy": { "v": 0 } },
            "add": [{ "id": "a", "v": 0 }]
        });
        let errs = apply_list_patch(&base, &patch, "aff", "id").unwrap_err();
        assert_eq!(
            errs,
            vec![
                "aff: remove unknown id \"zz\"",
                "aff: change unknown id \"yy\"",
                "aff: add duplicates id \"a\"",
            ]
        );
    }

    #[test]
    fn list_patch_rejects_unknown_keys() {
        let base = serde_json::json!([{ "id": "a" }]);
        let patch = serde_json::json!({ "changee": {} });
        let errs = apply_list_patch(&base, &patch, "aff", "id").unwrap_err();
        assert_eq!(errs, vec!["aff: unknown patch key \"changee\""]);
    }

    #[test]
    fn record_patch_rejects_unknown_keys() {
        let base = serde_json::json!({ "a": { "v": 1 } });
        let patch = serde_json::json!({ "changee": {} });
        let errs = apply_record_patch(&base, &patch, "tree", true).unwrap_err();
        assert_eq!(errs, vec!["tree: unknown patch key \"changee\""]);
    }

    #[test]
    fn game_config_patch_rejects_unknown_keys() {
        let base = serde_json::json!({ "stats": [] });
        let patch = serde_json::json!({ "changee": {} });
        let errs = apply_game_config_patch(&base, &patch, "gc").unwrap_err();
        assert_eq!(errs, vec!["gc: unknown patch key \"changee\""]);
    }

    #[test]
    fn patchless_ids_normalize_to_single_base_key() {
        assert_eq!(cache_key("definitely-unknown"), BASE_CACHE_KEY);
        assert_eq!(cache_key("also-unknown"), BASE_CACHE_KEY);
        assert_eq!(load_id(BASE_CACHE_KEY), "");
        assert_eq!(load_id("s10"), "s10");
        assert!(patches_for("").is_empty());
    }

    #[test]
    fn season_scope_round_trip() {
        assert_eq!(current_season_id(), DEFAULT_SEASON_ID);
        {
            let _scope = SeasonScope::enter(Some("s10".to_string()));
            assert_eq!(current_season_id(), "s10");
        }
        assert_eq!(current_season_id(), DEFAULT_SEASON_ID);
    }

    #[test]
    fn game_config_patch_overrides_scalars_and_stats() {
        let base = serde_json::json!({
            "maxCharacterLevel": 100,
            "stats": [{ "key": "all_skills", "name": "to All Skills" }]
        });
        let patch = serde_json::json!({
            "change": { "maxCharacterLevel": 110 },
            "stats": { "add": [{ "key": "corruption", "name": "Corruption" }] }
        });
        let out = apply_game_config_patch(&base, &patch, "gc").unwrap();
        assert_eq!(out["maxCharacterLevel"], 110);
        assert_eq!(out["stats"].as_array().unwrap().len(), 2);
    }
}
