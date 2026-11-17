//! Per-user setup of the single HSPlanner executable. The setup program is the
//! application itself: it copies its own file, then registers the shortcuts and the
//! uninstall entry. Saved builds live in the data directory and are never touched.
use anyhow::Context;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

#[cfg(not(windows))]
mod platform {
    use super::Layout;

    pub fn current_user() -> anyhow::Result<Layout> {
        anyhow::bail!("Setup is available on Windows only.")
    }
    pub fn installed_version(_: &Layout) -> Option<String> {
        None
    }
    pub fn register(_: &Layout, _: &str, _: bool) -> anyhow::Result<()> {
        Ok(())
    }
    pub fn unregister(_: &Layout) -> anyhow::Result<()> {
        Ok(())
    }
    pub fn remove_after_exit(_: &Layout) -> anyhow::Result<()> {
        anyhow::bail!("the program file is still in use")
    }
    pub fn wait_for_exit(_: u32) {}
}

pub const PRODUCT: &str = "HSPlanner";
pub const UPDATE_FLAG: &str = "--update";
pub const WAIT_FLAG: &str = "--wait-pid";
const INSTALL_FLAG: &str = "--install";
const UNINSTALL_FLAG: &str = "--uninstall";
const EXECUTABLE: &str = "hsplanner.exe";
const REPLACED_PREFIX: &str = "hsplanner.exe.old";
const SHORTCUT: &str = "HSPlanner.lnk";
const LICENSE_DIRECTORY: &str = "licenses";
const LICENSES: [(&str, &str); 3] = [
    ("HSPlanner.txt", include_str!("../../../LICENSE")),
    ("Inter.txt", include_str!("../../ui/assets/fonts/OFL.txt")),
    (
        "JetBrainsMono.txt",
        include_str!("../../ui/assets/fonts/OFL-JetBrainsMono.txt"),
    ),
];
/// Left by the NSIS installer of earlier releases; its uninstall entry is replaced.
const LEGACY_UNINSTALLER: &str = "uninstall.exe";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Install,
    /// Sent by the running application: replace it without questions once it exits.
    Update {
        wait_for: Option<u32>,
    },
    Uninstall,
}

/// A downloaded `…-setup.exe` installs when opened; the installed program needs a flag.
pub fn request(executable: &Path, args: impl Iterator<Item = OsString>) -> Option<Request> {
    let args: Vec<OsString> = args.collect();
    let has = |flag: &str| args.iter().any(|arg| arg == flag);
    if has(UNINSTALL_FLAG) {
        return Some(Request::Uninstall);
    }
    if has(UPDATE_FLAG) {
        let wait_for = args
            .iter()
            .position(|arg| arg == WAIT_FLAG)
            .and_then(|flag| args.get(flag + 1))
            .and_then(|pid| pid.to_str()?.parse().ok());
        return Some(Request::Update { wait_for });
    }
    let named_setup = executable.file_stem().is_some_and(|stem| {
        stem.to_string_lossy()
            .to_ascii_lowercase()
            .contains("setup")
    });
    (has(INSTALL_FLAG) || named_setup).then_some(Request::Install)
}

/// Where one installation lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    directory: PathBuf,
    desktop: PathBuf,
    programs: PathBuf,
    registry_key: String,
}

impl Layout {
    /// The signed-in user's installation, in the folders earlier releases used.
    pub fn current_user() -> anyhow::Result<Self> {
        platform::current_user()
    }

    /// Everything inside one folder: tests and the debug preview.
    pub fn under(root: &Path, registry_key: &str) -> Self {
        Self {
            directory: root.join(PRODUCT),
            desktop: root.join("Desktop"),
            programs: root.join("Programs"),
            registry_key: registry_key.to_owned(),
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn executable(&self) -> PathBuf {
        self.directory.join(EXECUTABLE)
    }

    pub fn desktop_shortcut(&self) -> PathBuf {
        self.desktop.join(SHORTCUT)
    }

    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    fn menu_shortcut(&self) -> PathBuf {
        self.programs.join(SHORTCUT)
    }

    pub fn is_installed(&self) -> bool {
        self.executable().is_file()
    }

    pub fn installed_version(&self) -> Option<String> {
        platform::installed_version(self)
    }
}

/// Copies `source` into place and registers it; an existing installation is replaced.
pub fn install(
    source: &Path,
    layout: &Layout,
    version: &str,
    desktop_shortcut: bool,
) -> anyhow::Result<()> {
    let directory = layout.directory();
    fs::create_dir_all(directory).with_context(|| format!("creating {}", directory.display()))?;
    place(source, &layout.executable())?;
    let licenses = directory.join(LICENSE_DIRECTORY);
    fs::create_dir_all(&licenses).with_context(|| format!("creating {}", licenses.display()))?;
    for (name, text) in LICENSES {
        fs::write(licenses.join(name), text).with_context(|| format!("writing {name}"))?;
    }
    remove(&directory.join(LEGACY_UNINSTALLER))?;
    platform::register(layout, version, desktop_shortcut)
}

/// The headless install behind an in-app update: same shortcuts as before.
pub fn update(
    source: &Path,
    layout: &Layout,
    version: &str,
    wait_for: Option<u32>,
) -> anyhow::Result<()> {
    if let Some(pid) = wait_for {
        platform::wait_for_exit(pid);
    }
    let desktop_shortcut = layout.desktop_shortcut().exists();
    install(source, layout, version, desktop_shortcut)
}

/// Removes the program, its shortcuts and its uninstall entry, and nothing else.
pub fn uninstall(layout: &Layout) -> anyhow::Result<()> {
    let directory = layout.directory();
    let executable = layout.executable();
    let licenses = directory.join(LICENSE_DIRECTORY);
    // Leftovers may resist (a file still in use) without stopping the uninstall.
    for (name, _) in LICENSES {
        let _ = fs::remove_file(licenses.join(name));
    }
    // Only an empty folder goes: files the user put there stay.
    let _ = fs::remove_dir(&licenses);
    let _ = fs::remove_file(staged(&executable));
    clear_replaced(directory);
    let _ = fs::remove_file(directory.join(LEGACY_UNINSTALLER));
    // The entry goes last, so an uninstall that failed earlier can be run again.
    platform::unregister(layout)?;
    if remove(&executable).is_err() {
        // Windows cannot delete the running uninstaller; a helper does once it exits.
        return platform::remove_after_exit(layout);
    }
    let _ = fs::remove_dir(directory);
    Ok(())
}

/// Swaps the program file through renames: Windows lets a running executable be
/// renamed but not overwritten, and a failed copy must leave the old program intact.
fn place(source: &Path, target: &Path) -> anyhow::Result<()> {
    if is_same_file(source, target) {
        return Ok(());
    }
    let staged = staged(target);
    let swapped = swap(source, target, &staged);
    if swapped.is_err() {
        let _ = fs::remove_file(&staged);
    }
    if let Some(directory) = target.parent() {
        clear_replaced(directory);
    }
    swapped
}

fn swap(source: &Path, target: &Path, staged: &Path) -> anyhow::Result<()> {
    fs::copy(source, staged).with_context(|| format!("copying to {}", staged.display()))?;
    let replaced = replaced(target);
    let had_previous = target.exists();
    if had_previous {
        fs::rename(target, &replaced).with_context(|| format!("replacing {}", target.display()))?;
    }
    if let Err(error) = fs::rename(staged, target) {
        if had_previous {
            let _ = fs::rename(&replaced, target);
        }
        return Err(error).with_context(|| format!("replacing {}", target.display()));
    }
    Ok(())
}

/// Removes the programs earlier swaps set aside. One that is still running keeps its
/// file until a later setup run, which is why every swap uses a name of its own.
fn clear_replaced(directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(REPLACED_PREFIX)
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn staged(executable: &Path) -> PathBuf {
    executable.with_extension("exe.new")
}

fn replaced(executable: &Path) -> PathBuf {
    executable.with_file_name(format!("{REPLACED_PREFIX}.{}", std::process::id()))
}

fn is_same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn remove(path: &Path) -> anyhow::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != ErrorKind::NotFound => {
            Err(error).with_context(|| format!("removing {}", path.display()))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("hsplanner-installer-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn layout(&self, name: &str) -> Layout {
            Layout::under(
                &self.0,
                &format!("HSPlanner-test-{}-{name}", std::process::id()),
            )
        }

        fn setup(&self, content: &str) -> PathBuf {
            let path = self.0.join("hsplanner_9.9.9_x64-setup.exe");
            fs::write(&path, content).unwrap();
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn args(list: &[&str]) -> impl Iterator<Item = OsString> {
        list.iter()
            .map(OsString::from)
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn a_setup_named_file_installs_and_the_installed_program_needs_a_flag() {
        let setup = Path::new("C:/Downloads/hsplanner_1.3.0_x64-setup (1).exe");
        let installed = Path::new("C:/Users/a/AppData/Local/HSPlanner/hsplanner.exe");
        assert_eq!(request(setup, args(&[])), Some(Request::Install));
        assert_eq!(request(installed, args(&[])), None);
        assert_eq!(request(installed, args(&["--crash-report", "x"])), None);
        assert_eq!(
            request(installed, args(&["--install"])),
            Some(Request::Install)
        );
        assert_eq!(
            request(installed, args(&["--uninstall"])),
            Some(Request::Uninstall)
        );
    }

    #[test]
    fn an_update_request_keeps_the_process_to_wait_for() {
        let setup = Path::new("hsplanner_1.3.0_x64-setup.exe");
        assert_eq!(
            request(setup, args(&["--update", "--wait-pid", "4242"])),
            Some(Request::Update {
                wait_for: Some(4242)
            })
        );
        assert_eq!(
            request(setup, args(&["--update", "--wait-pid", "soon"])),
            Some(Request::Update { wait_for: None })
        );
        assert_eq!(
            request(setup, args(&["--update"])),
            Some(Request::Update { wait_for: None })
        );
    }

    #[test]
    fn install_places_the_program_and_licenses_and_uninstall_removes_them() {
        let scratch = Scratch::new("round-trip");
        let layout = scratch.layout("round-trip");
        let setup = scratch.setup("first");
        assert!(!layout.is_installed());

        install(&setup, &layout, "9.9.9", true).unwrap();

        assert_eq!(fs::read_to_string(layout.executable()).unwrap(), "first");
        assert!(layout.directory().join("licenses/HSPlanner.txt").is_file());
        assert!(layout.is_installed());
        #[cfg(windows)]
        {
            assert!(layout.desktop_shortcut().is_file());
            assert!(layout.menu_shortcut().is_file());
            assert_eq!(layout.installed_version().as_deref(), Some("9.9.9"));
        }

        uninstall(&layout).unwrap();

        assert!(!layout.directory().exists());
        assert!(!layout.desktop_shortcut().exists());
        assert!(!layout.menu_shortcut().exists());
        assert_eq!(layout.installed_version(), None);
        assert!(setup.is_file());
    }

    #[test]
    fn reinstall_replaces_the_program_and_clears_what_nsis_left() {
        let scratch = Scratch::new("replace");
        let layout = scratch.layout("replace");
        install(&scratch.setup("old"), &layout, "9.9.8", false).unwrap();
        let legacy = layout.directory().join(LEGACY_UNINSTALLER);
        fs::write(&legacy, "nsis").unwrap();
        let kept_by_an_earlier_update = layout.directory().join("hsplanner.exe.old.1");
        fs::write(&kept_by_an_earlier_update, "older").unwrap();

        update(&scratch.setup("new"), &layout, "9.9.9", None).unwrap();

        let executable = layout.executable();
        assert_eq!(fs::read_to_string(&executable).unwrap(), "new");
        assert!(!legacy.exists());
        assert!(!staged(&executable).exists());
        assert!(!replaced(&executable).exists());
        assert!(!kept_by_an_earlier_update.exists());
        assert!(!layout.desktop_shortcut().exists());
        uninstall(&layout).unwrap();
    }

    #[test]
    fn installing_from_the_installed_program_keeps_the_file() {
        let scratch = Scratch::new("in-place");
        let layout = scratch.layout("in-place");
        install(&scratch.setup("program"), &layout, "9.9.9", false).unwrap();

        install(&layout.executable(), &layout, "9.9.9", false).unwrap();

        assert_eq!(fs::read_to_string(layout.executable()).unwrap(), "program");
        uninstall(&layout).unwrap();
    }

    #[test]
    fn uninstall_keeps_files_it_did_not_install() {
        let scratch = Scratch::new("foreign");
        let layout = scratch.layout("foreign");
        install(&scratch.setup("program"), &layout, "9.9.9", false).unwrap();
        let foreign = layout.directory().join("notes.txt");
        fs::write(&foreign, "mine").unwrap();

        uninstall(&layout).unwrap();

        assert!(!layout.is_installed());
        assert_eq!(fs::read_to_string(foreign).unwrap(), "mine");
    }
}
