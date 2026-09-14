//! Re-encode reference codes with the native implementation for decoder parity.
fn main() {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../engine/tests/fixtures/frontend-share-codes.json"
    ))
    .unwrap();
    let output: Vec<_> = fixtures.as_array().unwrap().iter().map(|fixture| {
        let (snapshot, notes) = hsplanner_build::codec::decode(fixture["code"].as_str().unwrap()).unwrap();
        serde_json::json!({"name": fixture["name"], "code": hsplanner_build::codec::encode(&snapshot, &notes).unwrap()})
    }).collect();
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
