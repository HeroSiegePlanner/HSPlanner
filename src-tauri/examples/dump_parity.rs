use serde_json::Value;

const FIXTURE_PATH: &str = "tests/fixtures/parity.json";

fn main() {
    let json = std::fs::read_to_string(FIXTURE_PATH).unwrap_or_else(|e| {
        panic!("cannot read {FIXTURE_PATH}: {e}\n(run from the src-tauri/ directory)")
    });
    let entries: Vec<Value> =
        serde_json::from_str(&json).expect("parity.json must be a valid JSON array");

    let mut out: Vec<Value> = Vec::with_capacity(entries.len());
    for entry in entries {
        let Value::Object(mut obj) = entry else {
            panic!("each parity.json entry must be a JSON object");
        };
        let name = obj
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("<unnamed>")
            .to_string();

        // Skipped scenarios and inputs-without-data pass through unchanged.
        if obj.contains_key("skipped") {
            out.push(Value::Object(obj));
            continue;
        }
        let Some(input_v) = obj.get("input").cloned() else {
            out.push(Value::Object(obj));
            continue;
        };

        let input: app_lib::calc::commands::BuildPerformanceInput =
            serde_json::from_value(input_v).unwrap_or_else(|e| {
                panic!("scenario '{name}': input deserialize failed: {e}")
            });
        let result = app_lib::calc::commands::calc_build_performance(input);
        let output =
            serde_json::to_value(&result).expect("BuildPerformance must be JSON-serialisable");

        obj.insert("output".to_string(), output);
        out.push(Value::Object(obj));
    }

    let rendered =
        serde_json::to_string_pretty(&Value::Array(out)).expect("serialising regenerated fixture");
    println!("{rendered}");
}
