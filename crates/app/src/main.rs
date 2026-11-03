#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod bug_report;
mod bug_report_issues;
mod bug_report_transport;
mod changelog;
mod chrome;
mod crash;
mod loadouts;
mod release_notes;
mod settings;
mod shell;
mod startup;
mod update;
mod update_dialog;

fn main() {
    #[cfg(target_os = "linux")]
    if gpui_kit::guess_compositor() != "Wayland" {
        eprintln!("HSPlanner requires a native Wayland session.");
        std::process::exit(1);
    }
    if let Some((log, language)) = crash::report_request(std::env::args_os().skip(1)) {
        crash::show(log, language);
        return;
    }
    #[cfg(debug_assertions)]
    hsplanner_ui::debug_log::install();
    let directory = hsplanner_build::storage::data_directory().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1)
    });
    crash::install(directory.join(crash::LOG_NAME));
    let loaded = hsplanner_build::storage::Writer::open(directory.clone());
    shell::run(directory, loaded);
}
