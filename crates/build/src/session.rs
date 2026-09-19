use serde::{Deserialize, Serialize};

use crate::{
    BuildSnapshot, codec,
    library::{Library, StashEntry, now},
    loadout::{LoadoutKind, Loadouts},
    notes::Notes,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Draft {
    pub build_id: Option<String>,
    pub snapshot: BuildSnapshot,
    pub loadouts: Loadouts,
    pub notes: Notes,
    pub stash: Vec<StashEntry>,
}

impl Draft {
    fn calculation_changed(&self, other: &Self) -> bool {
        serde_json::to_value(&self.snapshot).ok() != serde_json::to_value(&other.snapshot).ok()
            || LoadoutKind::ALL
                .into_iter()
                .any(|kind| self.loadouts.active_id(kind) != other.loadouts.active_id(kind))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub language: String,
    pub auto_save: bool,
    pub number_scale: String,
    pub extra_charm_slot: bool,
    pub ui_zoom: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            auto_save: true,
            number_scale: "billions".into(),
            extra_charm_slot: true,
            ui_zoom: 1.,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "WorkspaceWire")]
pub struct WorkspaceState {
    pub version: u32,
    #[serde(default)]
    pub revision: u64,
    pub library: Library,
    #[serde(default)]
    pub draft: Draft,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub filters: serde_json::Value,
    #[serde(default)]
    pub legacy_storage: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub migrated_from: Option<String>,
}
impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            version: 2,
            revision: 0,
            library: Library::default(),
            draft: Draft::default(),
            settings: Settings::default(),
            filters: serde_json::json!({"version":1,"filters":[]}),
            legacy_storage: Default::default(),
            migrated_from: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceWire {
    version: u32,
    #[serde(default)]
    revision: u64,
    library: Library,
    #[serde(default)]
    draft: serde_json::Value,
    #[serde(default)]
    settings: Settings,
    #[serde(default)]
    filters: serde_json::Value,
    #[serde(default)]
    legacy_storage: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    migrated_from: Option<String>,
}
impl TryFrom<WorkspaceWire> for WorkspaceState {
    type Error = String;
    fn try_from(wire: WorkspaceWire) -> Result<Self, Self::Error> {
        if !matches!(wire.version, 1 | 2) {
            return Err("Unsupported saved-data version. The original file is unchanged.".into());
        }
        let mut draft: Draft = if wire.draft.is_null() {
            Draft::default()
        } else {
            serde_json::from_value(wire.draft.clone()).map_err(|error| error.to_string())?
        };
        if wire.draft.get("loadouts").is_none() {
            if wire.version == 2 && !wire.draft.is_null() {
                return Err("Missing draft loadouts. The original file is unchanged.".into());
            }
            if let Some(build) = draft
                .build_id
                .as_deref()
                .and_then(|id| wire.library.build(id))
            {
                draft.loadouts = build.loadouts.clone();
                if let Some(id) = wire.draft["profileId"].as_str()
                    && !LoadoutKind::ALL
                        .into_iter()
                        .all(|kind| draft.loadouts.select(kind, id).is_ok())
                {
                    draft.build_id = None;
                    draft.loadouts = Loadouts::from_snapshot(&draft.snapshot);
                }
            } else {
                draft.build_id = None;
                draft.loadouts = Loadouts::from_snapshot(&draft.snapshot);
            }
        }
        draft.loadouts.validate()?;
        // The in-progress document may be newer than the last saved build.
        draft.loadouts.capture_active(&draft.snapshot);
        Ok(Self {
            version: 2,
            revision: wire.revision,
            library: wire.library,
            draft,
            settings: wire.settings,
            filters: wire.filters,
            legacy_storage: wire.legacy_storage,
            migrated_from: wire.migrated_from,
        })
    }
}

impl WorkspaceState {
    pub fn is_pristine(&self) -> bool {
        self.revision == 0
            && self.migrated_from.is_none()
            && self.library.builds.is_empty()
            && self.library.folders.is_empty()
            && serde_json::to_value(&self.draft).ok() == serde_json::to_value(Draft::default()).ok()
            && serde_json::to_value(&self.settings).ok()
                == serde_json::to_value(Settings::default()).ok()
            && self.legacy_storage.is_empty()
            && self.filters == WorkspaceState::default().filters
    }
}

pub struct Session {
    state: WorkspaceState,
    undo: Vec<Draft>,
    redo: Vec<Draft>,
    dirty: bool,
    calculation_revision: u64,
}
impl Session {
    pub fn new(mut state: WorkspaceState) -> Self {
        state.draft.loadouts.capture_active(&state.draft.snapshot);
        Self {
            calculation_revision: state.revision,
            state,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
        }
    }
    pub fn state(&self) -> &WorkspaceState {
        &self.state
    }
    pub fn snapshot(&self) -> &BuildSnapshot {
        &self.state.draft.snapshot
    }
    pub fn draft(&self) -> &Draft {
        &self.state.draft
    }
    pub fn calculation_revision(&self) -> u64 {
        self.calculation_revision
    }
    pub fn revision(&self) -> u64 {
        self.state.revision
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn has_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn has_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    fn changed(&mut self) {
        self.state.revision += 1;
        self.dirty = true;
    }
    pub fn edit(&mut self, edit: impl FnOnce(&mut Draft)) {
        let previous = self.state.draft.clone();
        edit(&mut self.state.draft);
        if previous.snapshot.class_id != self.state.draft.snapshot.class_id {
            self.state.draft.loadouts.class_changed();
        }
        self.state
            .draft
            .loadouts
            .capture_active(&self.state.draft.snapshot);
        if serde_json::to_value(&previous).ok() == serde_json::to_value(&self.state.draft).ok() {
            return;
        }
        if previous.calculation_changed(&self.state.draft) {
            self.calculation_revision += 1;
        }
        if self.undo.len() == 50 {
            self.undo.remove(0);
        }
        self.undo.push(previous);
        self.redo.clear();
        self.changed();
    }
    pub fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            if previous.calculation_changed(&self.state.draft) {
                self.calculation_revision += 1;
            }
            self.redo
                .push(std::mem::replace(&mut self.state.draft, previous));
            self.changed();
        }
    }
    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            if next.calculation_changed(&self.state.draft) {
                self.calculation_revision += 1;
            }
            self.undo
                .push(std::mem::replace(&mut self.state.draft, next));
            self.changed();
        }
    }
    pub fn edit_library<T>(
        &mut self,
        edit: impl FnOnce(&mut Library) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut next = self.state.library.clone();
        let result = edit(&mut next)?;
        next.validate()?;
        if self
            .state
            .draft
            .build_id
            .as_deref()
            .is_some_and(|id| next.build(id).is_none())
        {
            self.state.draft.build_id = None;
        }
        self.state.library = next;
        self.changed();
        Ok(result)
    }
    pub fn set_settings(&mut self, settings: Settings) {
        self.state.settings = settings;
        self.changed();
    }
    pub fn save_build(&mut self) -> Result<(), String> {
        let draft = &self.state.draft;
        let Some(build_id) = &draft.build_id else {
            return Ok(());
        };
        let build = self.state.library.build_mut(build_id)?;
        let previous = serde_json::to_value(&*build).map_err(|error| error.to_string())?;
        build.snapshot = draft.snapshot.clone();
        build.loadouts = draft.loadouts.clone();
        build.native_notes = Some(draft.notes.clone());
        build.notes = draft.notes.to_html();
        build.stash = draft.stash.clone();
        build.class_id = draft.snapshot.class_id.clone();
        build.season = draft.snapshot.season.clone();
        if serde_json::to_value(&*build).map_err(|error| error.to_string())? != previous {
            build.updated_at = now();
            self.changed();
        }
        Ok(())
    }
    pub fn open(&mut self, build_id: &str) -> Result<(), String> {
        if self.state.settings.auto_save {
            self.save_build()?;
        }
        let build = self
            .state
            .library
            .build(build_id)
            .ok_or("Build no longer exists.")?;
        self.state.draft = Draft {
            build_id: Some(build.id.clone()),
            snapshot: build.snapshot.clone(),
            loadouts: build.loadouts.clone(),
            notes: build.notes(),
            stash: build.stash.clone(),
        };
        self.calculation_revision += 1;
        self.undo.clear();
        self.redo.clear();
        self.changed();
        Ok(())
    }
    pub fn new_build(&mut self, name: &str) -> Result<String, String> {
        if self.state.settings.auto_save {
            self.save_build()?;
        }
        let id = self.state.library.create(
            name,
            &BuildSnapshot::default(),
            &Notes::default(),
            &[],
            None,
        )?;
        self.open(&id)?;
        Ok(id)
    }
    pub fn save_as(&mut self, name: &str) -> Result<String, String> {
        let draft = &self.state.draft;
        let archive = draft
            .build_id
            .as_deref()
            .and_then(|id| self.state.library.build(id))
            .map(|build| build.archived_profiles.clone())
            .unwrap_or_default();
        let id =
            self.state
                .library
                .create(name, &draft.snapshot, &draft.notes, &draft.stash, None)?;
        let saved = self.state.library.build_mut(&id)?;
        saved.loadouts = draft.loadouts.clone();
        saved.archived_profiles = archive;
        self.open(&id)?;
        Ok(id)
    }
    pub fn import_code(&mut self, code: &str) -> Result<String, String> {
        let (snapshot, notes, loadouts) = codec::decode_loadouts(code)?;
        let name = snapshot
            .class_id
            .as_deref()
            .and_then(hsplanner_engine::calc::data::get_class)
            .map(|class| format!("Imported {}", class.name))
            .unwrap_or_else(|| "Imported build".into());
        let id = self
            .state
            .library
            .create(&name, &snapshot, &notes, &[], None)?;
        self.state.library.build_mut(&id)?.loadouts = loadouts;
        self.open(&id)?;
        Ok(id)
    }
    pub fn add_loadout(&mut self, kind: LoadoutKind, name: &str) -> Result<(), String> {
        let mut loadouts = self.state.draft.loadouts.clone();
        loadouts.duplicate(kind, name)?;
        self.edit(|draft| draft.loadouts = loadouts);
        Ok(())
    }
    pub fn switch_loadout(&mut self, kind: LoadoutKind, id: &str) -> Result<(), String> {
        let mut loadouts = self.state.draft.loadouts.clone();
        loadouts.select(kind, id)?;
        self.edit(|draft| {
            loadouts.apply(kind, &mut draft.snapshot);
            draft.loadouts = loadouts;
        });
        Ok(())
    }
    pub fn rename_loadout(
        &mut self,
        kind: LoadoutKind,
        id: &str,
        name: &str,
    ) -> Result<(), String> {
        let mut loadouts = self.state.draft.loadouts.clone();
        loadouts.rename(kind, id, name)?;
        self.edit(|draft| draft.loadouts = loadouts);
        Ok(())
    }
    pub fn remove_loadout(&mut self, kind: LoadoutKind, id: &str) -> Result<(), String> {
        let mut loadouts = self.state.draft.loadouts.clone();
        loadouts.remove(kind, id)?;
        self.edit(|draft| {
            loadouts.apply(kind, &mut draft.snapshot);
            draft.loadouts = loadouts;
        });
        Ok(())
    }
    pub fn remove_build(&mut self, id: &str) {
        self.state.library.remove(id);
        if self.state.draft.build_id.as_deref() == Some(id) {
            self.state.draft.build_id = None;
        }
        self.changed();
    }
    pub fn import_transfer(
        &mut self,
        export: crate::storage::MigrationExport,
    ) -> Result<(), String> {
        if self.state.migrated_from.is_some() {
            return Err("This library has already imported its transfer.".into());
        }
        let mut incoming = export.into_state()?;
        if self.state.is_pristine() {
            incoming.revision = self.state.revision + 1;
            self.state = incoming;
        } else {
            let mut next = self.state.clone();
            for folder in incoming.library.folders {
                if !next
                    .library
                    .folders
                    .iter()
                    .any(|current| current.id == folder.id)
                {
                    next.library.folders.push(folder);
                }
            }
            for build in incoming.library.builds {
                if let Some(existing) = next
                    .library
                    .builds
                    .iter_mut()
                    .find(|current| current.id == build.id)
                {
                    existing.loadouts.merge_missing(build.loadouts);
                    existing.archived_profiles.extend(build.archived_profiles);
                } else {
                    next.library.builds.push(build);
                }
            }
            if let Some(filters) = incoming
                .filters
                .get("filters")
                .and_then(serde_json::Value::as_array)
                && let Some(current) = next
                    .filters
                    .get_mut("filters")
                    .and_then(serde_json::Value::as_array_mut)
            {
                for filter in filters {
                    if !current.iter().any(|item| item["id"] == filter["id"]) {
                        current.push(filter.clone());
                    }
                }
            }
            next.legacy_storage.extend(incoming.legacy_storage);
            next.legacy_storage.insert(
                "hsplanner.migration.unsavedDocument".into(),
                serde_json::to_string(&incoming.draft).map_err(|e| e.to_string())?,
            );
            next.library.create(
                "Recovered Tauri document",
                &incoming.draft.snapshot,
                &incoming.draft.notes,
                &incoming.draft.stash,
                None,
            )?;
            next.library.validate()?;
            next.migrated_from = incoming.migrated_from;
            next.revision += 1;
            self.state = next;
        }
        self.calculation_revision += 1;
        self.undo.clear();
        self.redo.clear();
        self.dirty = true;
        Ok(())
    }

    pub fn persisted(&mut self, revision: u64) {
        if revision == self.revision() {
            self.dirty = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn older_settings_default_to_english_and_language_survives_storage() {
        let old: Settings = serde_json::from_str(r#"{"autoSave":false,"uiZoom":1.25}"#).unwrap();
        assert_eq!(old.language, "en");
        assert!(!old.auto_save);
        assert_eq!(old.ui_zoom, 1.25);
        let directory = tempfile::tempdir().unwrap();
        let (writer, state) = crate::storage::Writer::open(directory.path().into()).unwrap();
        let mut session = Session::new(state);
        let draft = serde_json::to_value(session.draft()).unwrap();
        let mut settings = session.state().settings.clone();
        settings.language = "ko".into();
        session.set_settings(settings);
        assert_eq!(serde_json::to_value(session.draft()).unwrap(), draft);
        assert!(!session.has_undo());
        writer.save(session.state()).unwrap();
        writer.flush().unwrap();
        drop(writer);
        let (_, reopened) = crate::storage::Writer::open(directory.path().into()).unwrap();
        assert_eq!(reopened.settings.language, "ko");
    }
}
