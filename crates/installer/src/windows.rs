//! Shell folders, shortcuts and the "Installed apps" entry of the signed-in user.
use super::{EXECUTABLE, Layout, PRODUCT, SHORTCUT, UNINSTALL_FLAG, remove};
use anyhow::Context;
use std::ffi::OsString;
use std::fs;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize, IPersistFile,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_LocalAppData, FOLDERID_Programs, IShellLinkW, KF_FLAG_DEFAULT,
    SHGetKnownFolderPath, ShellLink,
};
use windows::core::{GUID, HSTRING, Interface};
use windows_registry::CURRENT_USER;

const UNINSTALL_KEYS: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
const PUBLISHER: &str = "zium1337";
const EXIT_WAIT_MS: u32 = 15_000;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Waits for this process, then deletes the program file and its empty folder; a
/// program that is still open is retried for a minute. The paths travel in the
/// environment, so no shell ever parses their characters.
const REMOVE_AFTER_EXIT: &str = "Wait-Process -Id $env:HSPLANNER_SETUP_PID -ErrorAction SilentlyContinue; \
     for ($i = 0; $i -lt 120 -and (Test-Path -LiteralPath $env:HSPLANNER_SETUP_EXE); $i++) { \
     Start-Sleep -Milliseconds 500; \
     Remove-Item -LiteralPath $env:HSPLANNER_SETUP_EXE -Force -ErrorAction SilentlyContinue }; \
     [IO.Directory]::Delete($env:HSPLANNER_SETUP_DIR)";

pub fn current_user() -> anyhow::Result<Layout> {
    let default = known_folder(&FOLDERID_LocalAppData)?.join(PRODUCT);
    Ok(Layout {
        directory: installed_directory().unwrap_or(default),
        desktop: known_folder(&FOLDERID_Desktop)?,
        programs: known_folder(&FOLDERID_Programs)?,
        registry_key: PRODUCT.to_owned(),
    })
}

/// The folder an earlier setup chose, so an update never leaves a second copy behind.
fn installed_directory() -> Option<PathBuf> {
    let location = CURRENT_USER
        .open(format!(r"{UNINSTALL_KEYS}\{PRODUCT}"))
        .ok()?
        .get_string("InstallLocation")
        .ok()?;
    let directory = PathBuf::from(location.trim_matches('"'));
    (directory.is_absolute() && directory.join(EXECUTABLE).is_file()).then_some(directory)
}

pub fn installed_version(layout: &Layout) -> Option<String> {
    CURRENT_USER
        .open(uninstall_key(layout))
        .ok()?
        .get_string("DisplayVersion")
        .ok()
}

pub fn register(layout: &Layout, version: &str, desktop_shortcut: bool) -> anyhow::Result<()> {
    let executable = layout.executable();
    // Copying a downloaded file keeps its "from the internet" mark, which would make
    // Windows question every start of the installed program.
    let mut download_mark = executable.clone().into_os_string();
    download_mark.push(":Zone.Identifier");
    let _ = fs::remove_file(download_mark);
    shortcut(&executable, &layout.menu_shortcut())?;
    if desktop_shortcut {
        shortcut(&executable, &layout.desktop_shortcut())?;
    } else {
        remove(&layout.desktop_shortcut())?;
    }
    // The NSIS installer of earlier releases kept its shortcut in a folder.
    let legacy = layout.programs.join(PRODUCT);
    let _ = fs::remove_file(legacy.join(SHORTCUT));
    let _ = fs::remove_dir(legacy);

    let quoted = |path: &Path| format!("\"{}\"", path.display());
    let kilobytes = fs::metadata(&executable).map_or(0, |file| file.len() / 1024);
    // Values of the earlier NSIS entry must not survive next to the new ones.
    let _ = CURRENT_USER.remove_tree(uninstall_key(layout));
    let key = CURRENT_USER
        .create(uninstall_key(layout))
        .context("writing the uninstall entry")?;
    key.set_string("DisplayName", PRODUCT)?;
    key.set_string("DisplayIcon", quoted(&executable))?;
    key.set_string("DisplayVersion", version)?;
    key.set_string("Publisher", PUBLISHER)?;
    key.set_string("InstallLocation", quoted(layout.directory()))?;
    key.set_string(
        "UninstallString",
        format!("{} {UNINSTALL_FLAG}", quoted(&executable)),
    )?;
    key.set_u32("NoModify", 1)?;
    key.set_u32("NoRepair", 1)?;
    key.set_u32(
        "EstimatedSize",
        u32::try_from(kilobytes).unwrap_or(u32::MAX),
    )?;
    Ok(())
}

pub fn unregister(layout: &Layout) -> anyhow::Result<()> {
    remove(&layout.menu_shortcut())?;
    remove(&layout.desktop_shortcut())?;
    let key = uninstall_key(layout);
    if CURRENT_USER.open(&key).is_ok() {
        CURRENT_USER
            .remove_tree(&key)
            .context("removing the uninstall entry")?;
    }
    Ok(())
}

pub fn remove_after_exit(layout: &Layout) -> anyhow::Result<()> {
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            REMOVE_AFTER_EXIT,
        ])
        .env("HSPLANNER_SETUP_PID", std::process::id().to_string())
        .env("HSPLANNER_SETUP_EXE", layout.executable())
        .env("HSPLANNER_SETUP_DIR", layout.directory())
        // A helper started inside the folder would keep it from being removed.
        .current_dir(std::env::temp_dir())
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("scheduling the removal of the program file")?;
    Ok(())
}

pub fn wait_for_exit(pid: u32) {
    // SAFETY: the process handle is owned here and closed after the wait.
    unsafe {
        // A process that is already gone cannot be opened, which is the goal.
        if let Ok(process) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            WaitForSingleObject(process, EXIT_WAIT_MS);
            let _ = CloseHandle(process);
        }
    }
}

fn uninstall_key(layout: &Layout) -> String {
    format!(r"{UNINSTALL_KEYS}\{}", layout.registry_key)
}

fn known_folder(id: &GUID) -> anyhow::Result<PathBuf> {
    // SAFETY: the shell allocates the path; it is copied before being freed.
    unsafe {
        let raw =
            SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).context("locating a user folder")?;
        let path = PathBuf::from(OsString::from_wide(raw.as_wide()));
        CoTaskMemFree(Some(raw.as_ptr().cast_const().cast()));
        Ok(path)
    }
}

fn shortcut(target: &Path, link: &Path) -> anyhow::Result<()> {
    if let Some(folder) = link.parent() {
        fs::create_dir_all(folder).with_context(|| format!("creating {}", folder.display()))?;
    }
    // SAFETY: COM is initialised on this thread for exactly the calls below.
    let saved = unsafe {
        // A thread that already chose another COM mode still serves these calls.
        let initialised = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let saved = save_shortcut(target, link);
        if initialised {
            CoUninitialize();
        }
        saved
    };
    saved.with_context(|| format!("creating {}", link.display()))
}

/// SAFETY: the caller has initialised COM on this thread.
unsafe fn save_shortcut(target: &Path, link: &Path) -> windows::core::Result<()> {
    unsafe {
        let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        shell.SetPath(&HSTRING::from(target))?;
        if let Some(folder) = target.parent() {
            shell.SetWorkingDirectory(&HSTRING::from(folder))?;
        }
        shell
            .cast::<IPersistFile>()?
            .Save(&HSTRING::from(link), true)
    }
}
