//! Embeds the game icons the pages link to, so the server ships as one file.
use std::{
    collections::hash_map::DefaultHasher,
    env, fs,
    hash::{Hash, Hasher},
    path::Path,
};

const FOLDERS: [&str; 4] = ["classes", "items", "skills", "socketable"];
/// Files whose content decides the cache-busting revision in page links.
const REVISIONED: [&str; 4] = [
    "static/style.css",
    "static/app.js",
    "../../data/incarnation-tree.json",
    "../../data/ether-tree.json",
];

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut hasher = DefaultHasher::new();
    for file in REVISIONED {
        println!("cargo:rerun-if-changed={file}");
        fs::read(Path::new(&manifest).join(file))
            .unwrap()
            .hash(&mut hasher);
    }
    println!("cargo:rustc-env=HSPASTE_ASSET_REV={:x}", hasher.finish());
    let root = Path::new(&manifest).join("../../assets");
    let mut files = Vec::new();
    for folder in FOLDERS {
        println!("cargo:rerun-if-changed={}", root.join(folder).display());
        collect(&root.join(folder), folder, &mut files);
    }
    files.sort();
    let rows: String = files
        .iter()
        .map(|(name, path)| format!("({name:?}, include_bytes!({path:?}) as &[u8]),\n"))
        .collect();
    let table = format!("pub static FILES: &[(&str, &[u8])] = &[\n{rows}];\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("assets.rs"),
        table,
    )
    .unwrap();
}

fn collect(dir: &Path, name: &str, files: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let name = format!("{name}/{}", entry.file_name().to_string_lossy());
        if path.is_dir() {
            collect(&path, &name, files);
        } else if name.ends_with(".png") || name.ends_with(".webp") {
            let path = path.canonicalize().unwrap();
            files.push((published(&name), path.to_string_lossy().into_owned()));
        }
    }
}

/// Socket sprites are looked up by gem or rune name, so they publish under `assets::slug`.
fn published(name: &str) -> String {
    let stem = name
        .strip_prefix("socketable/")
        .and_then(|file| file.strip_suffix("_spr.png"));
    let Some(stem) = stem else {
        return name.to_owned();
    };
    let slug: String = stem
        .chars()
        .map(|character| match character {
            character if character.is_ascii_alphanumeric() => character.to_ascii_lowercase(),
            _ => '_',
        })
        .collect();
    format!("socketable/{slug}.png")
}
