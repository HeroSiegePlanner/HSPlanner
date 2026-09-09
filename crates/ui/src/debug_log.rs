//! Dev-build logger: every `log` record goes to stderr and into a ring buffer the debug overlay shows.
use std::{collections::VecDeque, sync::Mutex};

pub const CAPACITY: usize = 200;

static LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

struct DebugLogger;

impl log::Log for DebugLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format_line(record.level(), record.target(), &record.args().to_string());
        eprintln!("{line}");
        push(line);
    }

    fn flush(&self) {}
}

/// Installs the logger once; later calls are no-ops so tests and the app can both call it.
pub fn install() {
    if log::set_logger(&DebugLogger).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

pub fn format_line(level: log::Level, target: &str, message: &str) -> String {
    format!("[{level:<5} {target}] {message}")
}

fn push(line: String) {
    let mut lines = LINES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if lines.len() == CAPACITY {
        lines.pop_front();
    }
    lines.push_back(line);
}

/// Oldest first.
pub fn lines() -> Vec<String> {
    LINES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_keeps_only_the_newest_capacity_lines() {
        for i in 0..(CAPACITY + 50) {
            push(format!("line {i}"));
        }
        let lines = lines();
        assert_eq!(lines.len(), CAPACITY);
        assert_eq!(lines.first().map(String::as_str), Some("line 50"));
        assert_eq!(lines.last().map(String::as_str), Some("line 249"));
    }

    #[test]
    fn line_format_carries_level_and_target() {
        assert_eq!(
            format_line(log::Level::Warn, "gpui", "atlas full"),
            "[WARN  gpui] atlas full"
        );
    }
}
