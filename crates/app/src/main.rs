#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod bug_report;
mod bug_report_transport;
mod changelog;
mod chrome;
mod loadouts;
mod settings;
mod shell;
mod startup;
mod update;

fn main() {
    #[cfg(target_os = "linux")]
    if gpui_kit::guess_compositor() != "Wayland" {
        eprintln!("HSPlanner requires a native Wayland session.");
        std::process::exit(1);
    }
    #[cfg(debug_assertions)]
    hsplanner_ui::debug_log::install();
    let directory = hsplanner_build::storage::data_directory().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1)
    });
    let loaded = hsplanner_build::storage::Writer::open(directory.clone());
    shell::run(directory, loaded);
}
