use app_lib::calc::tree::parse::{classify_tree_node_line, TreeLineClass};
use serde_json::{json, Map, Value};

fn main() {
    let mut map = Map::new();
    for (id, node) in app_lib::calc::data::tree_nodes() {
        let (mut parsed, mut unsupported) = (Vec::new(), Vec::new());
        for line in &node.lines {
            match classify_tree_node_line(line) {
                TreeLineClass::Stat(_) | TreeLineClass::Meta(_) => parsed.push(line),
                TreeLineClass::RecognizedNoStat => {}
                TreeLineClass::Unknown => unsupported.push(line),
            }
        }
        map.insert(
            id.clone(),
            json!({"parsed": parsed, "unsupported": unsupported}),
        );
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&Value::Object(map)).unwrap()
    );
}
