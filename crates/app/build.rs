//! Carry the existing Tauri report destination into native packages without logging it.
use std::{env, fs, path::PathBuf};
fn main() {
    for key in ["HSPLANNER_BUG_REPORT_URL", "VITE_BUG_REPORT_URL"] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let file = root.join(".env");
    println!("cargo:rerun-if-changed={}", file.display());
    let dotenv = fs::read_to_string(file).unwrap_or_default();
    let from_file = |key: &str| {
        dotenv
            .lines()
            .find_map(|line| {
                let (name, value) = line
                    .trim()
                    .strip_prefix("export ")
                    .unwrap_or(line.trim())
                    .split_once('=')?;
                (name.trim() == key).then(|| value.trim().trim_matches(['\'', '"']).to_owned())
            })
            .filter(|value| !value.is_empty())
    };
    let url = env::var("HSPLANNER_BUG_REPORT_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            env::var("VITE_BUG_REPORT_URL")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .or_else(|| from_file("HSPLANNER_BUG_REPORT_URL"))
        .or_else(|| from_file("VITE_BUG_REPORT_URL"));
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("bug-report-config.rs");
    fs::write(
        out,
        format!(
            "const COMPILED_ENDPOINT: Option<&str> = {};",
            url.map_or("None".into(), |v| format!("Some({v:?})"))
        ),
    )
    .unwrap();
}
