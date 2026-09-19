// Phase 3 of the TS→Rust calc migration. Loads the TS-generated fixture at
// engine/tests/fixtures/parity.json and reproduces every scenario through the
// Rust `calc_build_performance` command, asserting numerical equality of every
// output field. Each scenario carries the season it was dumped under, so the
// fixture stays valid when the default season moves on.
//
// Comparison tolerates:
//   * scalar vs `[min, min]` tuple equivalence (TS legacy RangedValue shape
//     vs Rust `[f64, f64]` tuples).
//   * TS-side missing key for `undefined` Option fields where Rust emits null.
//   * f64 fields within 1e-9 (integers compared exactly via f64 conversion).

use serde_json::Value;

#[derive(serde::Deserialize)]
struct FixtureEntry {
    name: String,
    #[serde(default)]
    skipped: Option<String>,
    #[serde(default)]
    input: Option<Value>,
    #[serde(default)]
    output: Option<Value>,
}

const FIXTURE_PATH: &str = "tests/fixtures/parity.json";
const EPSILON: f64 = 1e-9;

#[test]
fn parity_with_ts_fixtures() {
    let json = std::fs::read_to_string(FIXTURE_PATH)
        .unwrap_or_else(|e| panic!("missing parity fixture at {FIXTURE_PATH}: {e}"));
    let entries: Vec<FixtureEntry> =
        serde_json::from_str(&json).expect("parity.json must be a valid JSON array");

    let mut diffs: Vec<String> = Vec::new();
    let mut compared = 0;
    let mut skipped = 0;

    for entry in &entries {
        if let Some(reason) = entry.skipped.as_deref() {
            eprintln!("  skipped: {} ({reason})", entry.name);
            skipped += 1;
            continue;
        }
        let Some(input_v) = entry.input.as_ref() else {
            continue;
        };
        let Some(archived_expected) = entry.output.as_ref() else {
            continue;
        };
        let mut expected = archived_expected.clone();
        // Keep the historical TS snapshot intact. Its three-projectile Charged
        // Bolts case counted all three only in average DPS, but just one in
        // noncritical DPS. This named correction is independently covered by
        // noncritical_dps_counts_every_projectile_contact.
        if entry.name == "class_with_active_skill_damage" {
            assert_eq!(input_v["skillProjectiles"]["charged_bolts"], 3);
            for field in ["hitDpsMin", "hitDpsMax"] {
                assert_eq!(archived_expected[field], 15.0);
                expected[field] = serde_json::json!(45.0); // 15 damage × 3 projectiles × 1 cast/s.
            }
        }
        correct_archived_cost_cadence(input_v, archived_expected, &mut expected);

        let input: app_lib::calc::commands::BuildPerformanceInput =
            match serde_json::from_value(input_v.clone()) {
                Ok(v) => v,
                Err(e) => {
                    diffs.push(format!(
                        "scenario '{}': input deserialize failed: {e}",
                        entry.name
                    ));
                    continue;
                }
            };

        let actual = app_lib::calc::commands::calc_build_performance(input);
        let actual_json =
            serde_json::to_value(&actual).expect("BuildPerformance must be JSON-serialisable");

        if let Some(diff) = compare_value("", &actual_json, &expected) {
            diffs.push(format!("scenario '{}': {diff}", entry.name));
        }
        compared += 1;
    }

    eprintln!(
        "parity: {compared} scenarios compared, {skipped} skipped, {} divergence(s)",
        diffs.len()
    );

    assert!(
        compared >= 5,
        "expected at least 5 fixture scenarios; got {compared}"
    );
    if !diffs.is_empty() {
        for d in &diffs {
            eprintln!("  ✗ {d}");
        }
        panic!(
            "{} parity divergence(s) — see output above. Either the Rust calc \
             diverged from TS, or the fixture is stale (re-run the dump).",
            diffs.len()
        );
    }
}

// The archived UI cost path ignored declared cooldowns without usesSkillHaste
// and returned no cost cadence for Attack-kind skills. Keep the archive intact:
// correct only these named rows using fixed rates and its unchanged mana/regen.
// Damage, ranks, per-use costs and every unrelated field retain TS expectations.
fn correct_archived_cost_cadence(input: &Value, archived: &Value, expected: &mut Value) {
    let class = input["classId"].as_str().unwrap_or("");
    let mut rows = Vec::new();
    if class == "amazon" {
        let attack_rate = if input["inventory"]["weapon"].is_object() {
            1.75
        } else {
            1.5
        };
        assert_eq!(
            archived["stats"]["attacks_per_second"],
            serde_json::json!([attack_rate, attack_rate])
        );
        for id in [
            "astropes_gift",
            "caustic_spearhead",
            "noxious_strike",
            "rebound",
            "spearnage",
            "thunder_fury",
        ] {
            assert!(archived["skillCosts"][id]["baseRate"].is_null());
            expected["skillCosts"][id]["baseRate"] = serde_json::json!(attack_rate);
            rows.push((id, attack_rate));
        }
        rows.extend([
            ("astropes_battle_maiden", 1.0 / 70.0),
            ("jungle_camouflage", 1.0 / 70.0),
        ]);
    } else if class == "stormweaver" {
        rows.extend([
            ("static_shock", 1.0 / 3.0),
            ("storm_cloud", 1.0 / 3.0),
            ("symphony_of_thunder", 1.0 / 70.0),
        ]);
    }
    for (id, rate) in rows {
        let old = &archived["skillCosts"][id];
        assert_eq!(old["speedMax"], 0.0);
        assert!(old["castRateMax"].is_null() || old["castRateMax"] == 1.0);
        let mana_min = old["manaMin"].as_f64().unwrap() * rate;
        let mana_max = old["manaMax"].as_f64().unwrap() * rate;
        let regen_min = old["manaRegenMin"].as_f64().unwrap();
        let regen_max = old["manaRegenMax"].as_f64().unwrap();
        let cost = &mut expected["skillCosts"][id];
        for field in ["castRateMin", "castRateMax"] {
            cost[field] = serde_json::json!(rate);
        }
        cost["manaPerSecMin"] = serde_json::json!(mana_min);
        cost["manaPerSecMax"] = serde_json::json!(mana_max);
        cost["netMin"] = serde_json::json!(regen_min - mana_max);
        cost["netMax"] = serde_json::json!(regen_max - mana_min);
        cost["sustainable"] = serde_json::json!(mana_max <= regen_min);
        cost["unsustainable"] = serde_json::json!(mana_min > regen_max);
        cost["uptimeMin"] = serde_json::json!((regen_min / mana_max * 100.0).min(100.0));
        cost["uptimeMax"] = serde_json::json!((regen_max / mana_min * 100.0).min(100.0));
    }
}

// Recursive structural comparison with the migration-specific tolerances baked
// in. Returns None on match, Some(error_message_path) on first divergence.
fn compare_value(path: &str, rust: &Value, ts: &Value) -> Option<String> {
    match (rust, ts) {
        (Value::Null, Value::Null) => None,
        (Value::Bool(a), Value::Bool(b)) if a == b => None,
        (Value::String(a), Value::String(b)) if a == b => None,
        (Value::Number(a), Value::Number(b)) => compare_numbers(path, a, b),
        (Value::Array(arr), Value::Number(n)) if arr.len() == 2 => {
            compare_ranged_tuple_vs_scalar(path, arr, n)
        }
        (Value::Number(n), Value::Array(arr)) if arr.len() == 2 => {
            compare_ranged_tuple_vs_scalar(path, arr, n)
        }

        (Value::Array(rust_arr), Value::Array(ts_arr)) => {
            if rust_arr.len() != ts_arr.len() {
                return Some(format!(
                    "{path}: array length {} != {} (rust vs ts)",
                    rust_arr.len(),
                    ts_arr.len()
                ));
            }
            for (i, (r, t)) in rust_arr.iter().zip(ts_arr.iter()).enumerate() {
                if let Some(e) = compare_value(&format!("{path}[{i}]"), r, t) {
                    return Some(e);
                }
            }
            None
        }

        (Value::Object(rust_obj), Value::Object(ts_obj)) => {
            for (k, r_val) in rust_obj.iter() {
                if !ts_obj.contains_key(k) && !r_val.is_null() {
                    return Some(format!(
                        "{path}.{k}: rust value {r_val} present but missing in TS"
                    ));
                }
            }
            for (k, t_val) in ts_obj.iter() {
                let r_val = rust_obj.get(k).unwrap_or(&Value::Null);
                if let Some(e) = compare_value(&format!("{path}.{k}"), r_val, t_val) {
                    return Some(e);
                }
            }
            None
        }

        _ => Some(format!("{path}: type mismatch — rust={rust} ts={ts}")),
    }
}

fn compare_numbers(path: &str, a: &serde_json::Number, b: &serde_json::Number) -> Option<String> {
    let af = a.as_f64().unwrap_or(0.0);
    let bf = b.as_f64().unwrap_or(0.0);
    if (af - bf).abs() <= EPSILON {
        None
    } else {
        Some(format!("{path}: number {af} != {bf}"))
    }
}

fn compare_ranged_tuple_vs_scalar(
    path: &str,
    arr: &[Value],
    n: &serde_json::Number,
) -> Option<String> {
    let a = arr[0].as_f64().unwrap_or(0.0);
    let b = arr[1].as_f64().unwrap_or(0.0);
    let v = n.as_f64().unwrap_or(0.0);
    if (a - v).abs() <= EPSILON && (b - v).abs() <= EPSILON {
        None
    } else {
        Some(format!("{path}: tuple [{a}, {b}] != scalar {v}"))
    }
}
