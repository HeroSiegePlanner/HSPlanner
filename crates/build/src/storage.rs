use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::{
    BuildSnapshot,
    library::Library,
    notes::Notes,
    session::{Draft, Settings, WorkspaceState},
};

const MAX_STATE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationExport {
    pub version: u32,
    pub exported_at: String,
    pub library: Library,
    pub snapshot: BuildSnapshot,
    pub active_build_id: Option<String>,
    pub active_profile_id: Option<String>,
    pub notes: String,
    #[serde(default)]
    pub stash: Vec<crate::library::StashEntry>,
    pub settings: Settings,
    pub filters: serde_json::Value,
    #[serde(default)]
    pub storage: BTreeMap<String, String>,
}

impl MigrationExport {
    pub fn into_state(self) -> Result<WorkspaceState, String> {
        if self.version != 1 {
            return Err(
                "Unsupported migration version. Install the matching HSPlanner version.".into(),
            );
        }
        self.library.validate()?;
        for build in &self.library.builds {
            for profile in &build.profiles {
                profile.snapshot().map_err(|e| {
                    format!("Cannot migrate {} / {}: {e}", build.name, profile.name)
                })?;
            }
        }
        let mut state = WorkspaceState {
            library: self.library,
            settings: self.settings,
            filters: self.filters,
            legacy_storage: self.storage,
            migrated_from: Some(self.exported_at),
            draft: Draft {
                build_id: self.active_build_id,
                profile_id: self.active_profile_id,
                snapshot: self.snapshot,
                notes: Notes::from_html(&self.notes),
                stash: self.stash,
            },
            ..Default::default()
        };
        let draft = &state.draft;
        if !draft
            .build_id
            .as_deref()
            .and_then(|id| state.library.build(id))
            .is_some_and(|build| {
                draft
                    .profile_id
                    .as_deref()
                    .is_some_and(|id| build.profile(id).is_some())
            })
        {
            state.draft.build_id = None;
            state.draft.profile_id = None;
        }
        Ok(state)
    }
}

pub fn data_directory() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("HSPLANNER_DATA_DIR") {
        return Ok(PathBuf::from(path));
    }
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var_os("HOME").ok_or("Home directory is unavailable.")?)
        .join("Library/Application Support");
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(
        std::env::var_os("APPDATA").ok_or("Application data directory is unavailable.")?,
    );
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        });
    Ok(base.join("com.zium.hsplanner").join("gpui"))
}

pub fn read_migration(directory: &Path) -> Result<MigrationExport, String> {
    let path = directory
        .parent()
        .ok_or("Invalid data directory.")?
        .join("migration-v1.json");
    if !path.exists() {
        return Err(
            "No transfer file found. Open the transitional Tauri release once, then check again."
                .into(),
        );
    }
    read_json(&path)
}

pub fn load(directory: &Path) -> Result<WorkspaceState, String> {
    let path = directory.join("state.json");
    if !path.exists() {
        return Ok(WorkspaceState::default());
    }
    let state = read_json(&path)?;
    validate(&state)?;
    Ok(state)
}

pub fn restore_backup(directory: &Path) -> Result<WorkspaceState, String> {
    let state: WorkspaceState = read_json(&directory.join("state.backup.json"))?;
    validate(&state)?;
    let original = directory.join("state.json");
    if original.exists() {
        fs::copy(
            &original,
            directory.join(format!("state.recovered-{}.json", uuid::Uuid::new_v4())),
        )
        .map_err(|e| e.to_string())?;
    }
    replace_file(
        &original,
        &serde_json::to_vec(&state).map_err(|e| e.to_string())?,
    )?;
    Ok(state)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let file = File::open(path).map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_STATE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err("The saved data exceeds the supported file size.".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| {
        format!(
            "Cannot read {}: {e}. The original file is unchanged.",
            path.display()
        )
    })
}

fn validate(state: &WorkspaceState) -> Result<(), String> {
    if state.version != 1 {
        return Err("Unsupported saved-data version. The original file is unchanged.".into());
    }
    state.library.validate()
}

fn replace_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid data path.")?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| format!("Could not save data: {e}"))?;
    file.persist(path)
        .map_err(|e| format!("Could not replace saved data: {}", e.error))?;
    #[cfg(unix)]
    File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn write_atomic(directory: &Path, state: &WorkspaceState) -> Result<(), String> {
    validate(state)?;
    let bytes = serde_json::to_vec(state).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err("The library is too large to save.".into());
    }
    fs::create_dir_all(directory).map_err(|e| format!("Cannot create data directory: {e}"))?;
    let path = directory.join("state.json");
    if path.exists() {
        let previous: WorkspaceState = read_json(&path)?;
        validate(&previous)?;
        replace_file(
            &directory.join("state.backup.json"),
            &serde_json::to_vec(&previous).map_err(|e| e.to_string())?,
        )?;
    }
    replace_file(&path, &bytes)
}

enum WriteCommand {
    Save,
    Flush(mpsc::SyncSender<Result<u64, String>>),
}

#[derive(Clone)]
pub struct Writer {
    sender: mpsc::Sender<WriteCommand>,
    pending: Arc<Mutex<Option<WorkspaceState>>>,
    results: Arc<Mutex<mpsc::Receiver<Result<u64, String>>>>,
}

impl Writer {
    pub fn open(directory: PathBuf) -> Result<(Self, WorkspaceState), String> {
        Self::open_inner(directory, false)
    }
    pub fn recover(directory: PathBuf) -> Result<(Self, WorkspaceState), String> {
        Self::open_inner(directory, true)
    }
    fn open_inner(directory: PathBuf, recover: bool) -> Result<(Self, WorkspaceState), String> {
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("state.lock"))
            .map_err(|e| e.to_string())?;
        lock.try_lock_exclusive().map_err(|_| {
            "HSPlanner is already using this library. Switch to the open window.".to_owned()
        })?;
        let state = if recover {
            restore_backup(&directory)?
        } else {
            load(&directory)?
        };
        let (sender, receiver) = mpsc::channel();
        let (results, result_receiver) = mpsc::channel();
        let revision = state.revision;
        let pending: Arc<Mutex<Option<WorkspaceState>>> = Arc::new(Mutex::new(None));
        let pending_worker = pending.clone();
        std::thread::Builder::new()
            .name("hsplanner-save".into())
            .spawn(move || {
                let _lock = lock;
                let mut saved_revision = revision;
                let mut last_result = Ok(revision);
                while let Ok(command) = receiver.recv() {
                    match command {
                        WriteCommand::Save => {
                            let Some(state) = pending_worker
                                .lock()
                                .ok()
                                .and_then(|mut pending| pending.take())
                            else {
                                continue;
                            };
                            if state.revision < saved_revision {
                                continue;
                            }
                            last_result = write_atomic(&directory, &state).map(|_| state.revision);
                            if let Ok(revision) = last_result {
                                saved_revision = revision;
                            }
                            let _ = results.send(last_result.clone());
                        }
                        WriteCommand::Flush(reply) => {
                            let _ = reply.send(last_result.clone());
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok((
            Self {
                sender,
                pending,
                results: Arc::new(Mutex::new(result_receiver)),
            },
            state,
        ))
    }
    pub fn save(&self, state: &WorkspaceState) -> Result<(), String> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| "The save worker stopped.")?;
        if pending
            .as_ref()
            .is_some_and(|next| next.revision > state.revision)
        {
            return Ok(());
        }
        let wake = pending.is_none();
        *pending = Some(state.clone());
        if wake {
            self.sender
                .send(WriteCommand::Save)
                .map_err(|_| "The save worker stopped.")?;
        }
        Ok(())
    }
    pub fn flush(&self) -> Result<u64, String> {
        let (send, receive) = mpsc::sync_channel(1);
        self.sender
            .send(WriteCommand::Flush(send))
            .map_err(|_| "The save worker stopped.")?;
        receive.recv().map_err(|_| "The save worker stopped.")?
    }
    pub fn result(&self) -> Option<Result<u64, String>> {
        self.results.lock().ok()?.try_iter().last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saves_are_atomic_ordered_locked_and_recoverable() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("gpui");
        let (writer, mut state) = Writer::open(directory.clone()).unwrap();
        assert!(Writer::open(directory.clone()).is_err());
        state.revision = 1;
        state.draft.snapshot.level = 20;
        writer.save(&state).unwrap();
        writer.flush().unwrap();
        state.revision = 2;
        state.draft.snapshot.level = 30;
        writer.save(&state).unwrap();
        state.revision = 1;
        state.draft.snapshot.level = 10;
        writer.save(&state).unwrap();
        writer.flush().unwrap();
        assert_eq!(load(&directory).unwrap().draft.snapshot.level, 30);
        fs::write(directory.join("state.json"), b"incomplete").unwrap();
        assert!(load(&directory).is_err());
        assert_eq!(restore_backup(&directory).unwrap().draft.snapshot.level, 20);
        assert!(
            fs::read_dir(&directory)
                .unwrap()
                .filter_map(Result::ok)
                .any(|e| e
                    .file_name()
                    .to_string_lossy()
                    .starts_with("state.recovered-"))
        );
    }
    #[test]
    fn legacy_export_is_ignored_and_native_edits_are_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("gpui");
        let export = MigrationExport {
            version: 1,
            exported_at: "first".into(),
            library: Library::default(),
            snapshot: BuildSnapshot::default(),
            active_build_id: None,
            active_profile_id: None,
            notes: "<b>Original</b>".into(),
            stash: Vec::new(),
            settings: Settings::default(),
            filters: serde_json::json!({"version":1,"filters":[]}),
            storage: BTreeMap::new(),
        };
        fs::write(
            temp.path().join("migration-v1.json"),
            serde_json::to_vec(&export).unwrap(),
        )
        .unwrap();
        let mut state = load(&directory).unwrap();
        assert!(state.migrated_from.is_none());
        assert!(state.draft.notes.markdown.is_empty());
        state.draft.snapshot.level = 80;
        state.revision += 1;
        write_atomic(&directory, &state).unwrap();
        assert_eq!(load(&directory).unwrap().draft.snapshot.level, 80);
    }
    #[test]
    fn unknown_schema_never_falls_back_to_an_empty_library() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.json");
        fs::write(&path, br#"{"version":999}"#).unwrap();
        assert!(load(temp.path()).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "{\"version\":999}");
    }
}
