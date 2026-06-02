// Phase 3 of the TS→Rust calc migration. Loads the TS-generated fixture at
// src-tauri/tests/fixtures/parity.json (produced by
// src/utils/__parity-fixtures.gen.test.ts before any TS calc file is touched)
// and reproduces every scenario through the Rust `calc_build_performance`
// command, asserting numerical equality of every output field.
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
    let json = std::fs::read_to_string(FIXTURE_PATH).unwrap_or_else(|e| {
        panic!(
            "missing parity fixture at {FIXTURE_PATH}: {e}\n\
             Regenerate with: npx vitest run src/utils/__parity-fixtures.gen.test.ts"
        )
    });
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
        let Some(expected) = entry.output.as_ref() else {
            continue;
        };

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

        if let Some(diff) = compare_value("", &actual_json, expected) {
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
