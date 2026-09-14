use std::{env, fs, path::PathBuf};

// Compile the engine's exact classifier without pulling Tauri/OCR into the planner crate.
#[allow(dead_code, clippy::collapsible_if)]
#[rustfmt::skip]
#[path = "../../engine/src/calc/tree/parse/mod.rs"]
mod tree_parse;

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("engine/src/calc/tree/parse").display()
    );
    let nodes_path = root.join("data/incarnation-nodes.json");
    println!("cargo:rerun-if-changed={}", nodes_path.display());
    let nodes: serde_json::Value = serde_json::from_slice(&fs::read(nodes_path).unwrap()).unwrap();
    let mut classified = serde_json::Map::new();
    for (id, node) in nodes.as_object().unwrap() {
        let mut parsed = Vec::new();
        let mut unsupported = Vec::new();
        for line in node["l"].as_array().unwrap() {
            match tree_parse::classify_tree_node_line(line.as_str().unwrap()) {
                tree_parse::TreeLineClass::Stat(_) | tree_parse::TreeLineClass::Meta(_) => {
                    parsed.push(line)
                }
                tree_parse::TreeLineClass::RecognizedNoStat => {}
                tree_parse::TreeLineClass::Unknown => unsupported.push(line),
            }
        }
        classified.insert(
            id.clone(),
            serde_json::json!({"parsed": parsed, "unsupported": unsupported}),
        );
    }
    fs::write(
        output.join("node-lines.json"),
        serde_json::to_vec(&classified).unwrap(),
    )
    .unwrap();
    let icons = root.join("assets/atlas/nodes");
    println!("cargo:rerun-if-changed={}", icons.display());
    let mut paths: Vec<_> = fs::read_dir(icons)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
        .collect();
    paths.sort();
    let mut source = String::from("pub const ICONS: &[(&str, &[u8])] = &[\n");
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        source.push_str(&format!(
            "({:?}, include_bytes!({:?})),\n",
            path.file_stem().unwrap().to_str().unwrap(),
            path.canonicalize().unwrap(),
        ));
    }
    source.push_str("];\n");
    fs::write(output.join("icons.rs"), source).unwrap();
    let items = root.join("assets/items");
    println!("cargo:rerun-if-changed={}", items.display());
    let mut paths: Vec<_> = fs::read_dir(items)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
        .collect();
    paths.sort();
    let mut source = String::from("pub const ITEM_ICONS: &[(&str, &[u8])] = &[\n");
    for path in paths {
        source.push_str(&format!(
            "({:?}, include_bytes!({:?})),\n",
            path.file_stem().unwrap().to_str().unwrap(),
            path.canonicalize().unwrap()
        ));
    }
    source.push_str("];\n");
    fs::write(output.join("item-icons.rs"), source).unwrap();
    // Socketables are keyed by display name ("Pristine Sapphire"), augments by id, like the frontend.
    let mut source = String::new();
    for (constant, directory, key) in [
        ("SOCKETABLE_ICONS", "socketable", true),
        ("AUGMENT_ICONS", "augments", false),
    ] {
        let dir = root.join("assets").join(directory);
        println!("cargo:rerun-if-changed={}", dir.display());
        let mut paths: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
            .collect();
        paths.sort();
        source.push_str(&format!("pub const {constant}: &[(&str, &[u8])] = &[\n"));
        for path in paths {
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let name = if key {
                stem.trim_end_matches("_spr")
                    .replace('_', " ")
                    .to_lowercase()
            } else {
                stem.to_owned()
            };
            source.push_str(&format!(
                "({:?}, include_bytes!({:?})),\n",
                name,
                path.canonicalize().unwrap()
            ));
        }
        source.push_str("];\n");
    }
    fs::write(output.join("socket-icons.rs"), source).unwrap();
    skill_visuals(&root, &output);
}

fn skill_visuals(root: &std::path::Path, output: &std::path::Path) {
    let directory = root.join("data/skills");
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut paths: Vec<_> = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    paths.sort();
    let mut source = String::from("pub const SKILL_POSITIONS: &[(&str, u32, u32)] = &[\n");
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        let skills: Vec<serde_json::Value> =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for skill in skills {
            if let (Some(id), Some(row), Some(col)) = (
                skill["id"].as_str(),
                skill["position"]["row"].as_u64(),
                skill["position"]["col"].as_u64(),
            ) {
                let class = skill["classId"].as_str().unwrap();
                source.push_str(&format!("({:?}, {row}, {col}),\n", format!("{class}/{id}")));
            }
        }
    }
    source.push_str("];\n");
    for (directory, constant) in [("skills", "SKILL_ICONS"), ("subskills", "SUBSKILL_ICONS")] {
        let directory = root.join("assets").join(directory);
        println!("cargo:rerun-if-changed={}", directory.display());
        let mut paths = Vec::new();
        for class in fs::read_dir(directory).unwrap() {
            let class = class.unwrap().path();
            if class.is_dir() {
                paths.extend(
                    fs::read_dir(class)
                        .unwrap()
                        .map(|file| file.unwrap().path())
                        .filter(|path| {
                            path.extension().is_some_and(|extension| extension == "png")
                        }),
                );
            }
        }
        paths.sort();
        source.push_str(&format!("pub const {constant}: &[(&str, &[u8])] = &[\n"));
        for path in paths {
            println!("cargo:rerun-if-changed={}", path.display());
            let key = format!(
                "{}/{}",
                path.parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                path.file_stem().unwrap().to_str().unwrap()
            );
            source.push_str(&format!(
                "({key:?}, include_bytes!({:?})),\n",
                path.canonicalize().unwrap()
            ));
        }
        source.push_str("];\n");
    }
    fs::write(output.join("skill-visuals.rs"), source).unwrap();
}
