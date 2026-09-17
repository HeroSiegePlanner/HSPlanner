use serde::{Deserialize, Serialize};

use crate::{
    BuildSnapshot, codec,
    library::{Library, Profile, StashEntry, clean_name, now},
    notes::Notes,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Draft {
    pub build_id: Option<String>,
    pub profile_id: Option<String>,
    pub snapshot: BuildSnapshot,
    pub notes: Notes,
    pub stash: Vec<StashEntry>,
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
#[serde(rename_all = "camelCase")]
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
            version: 1,
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
    pub fn new(state: WorkspaceState) -> Self {
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
        if serde_json::to_value(&previous).ok() == serde_json::to_value(&self.state.draft).ok() {
            return;
        }
        if serde_json::to_value(&previous.snapshot).ok()
            != serde_json::to_value(&self.state.draft.snapshot).ok()
        {
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
            self.calculation_revision += 1;
            self.redo
                .push(std::mem::replace(&mut self.state.draft, previous));
            self.changed();
        }
    }
    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.calculation_revision += 1;
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
            self.state.draft.profile_id = None;
        }
        self.state.library = next;
        self.changed();
        Ok(result)
    }
    pub fn set_settings(&mut self, settings: Settings) {
        self.state.settings = settings;
        self.changed();
    }
    pub fn save_profile(&mut self) -> Result<(), String> {
        let draft = &self.state.draft;
        let (Some(build_id), Some(profile_id)) = (&draft.build_id, &draft.profile_id) else {
            return Ok(());
        };
        let code = codec::encode(&draft.snapshot, &Notes::default())?;
        let build = self.state.library.build_mut(build_id)?;
        let profile = build
            .profiles
            .iter_mut()
            .find(|p| &p.id == profile_id)
            .ok_or("Profile no longer exists.")?;
        profile.snapshot = Some(draft.snapshot.clone());
        profile.code = code;
        profile.updated_at = now();
        build.native_notes = Some(draft.notes.clone());
        build.notes = draft.notes.to_html();
        build.stash = draft.stash.clone();
        build.class_id = draft.snapshot.class_id.clone();
        build.season = draft.snapshot.season.clone();
        build.updated_at = now();
        self.changed();
        Ok(())
    }
    pub fn open(&mut self, build_id: &str, profile_id: Option<&str>) -> Result<(), String> {
        if self.state.settings.auto_save {
            self.save_profile()?;
        }
        let build = self
            .state
            .library
            .build(build_id)
            .ok_or("Build no longer exists.")?;
        let profile = build
            .profile(profile_id.unwrap_or(&build.active_profile_id))
            .ok_or("Profile no longer exists.")?;
        let draft = Draft {
            build_id: Some(build.id.clone()),
            profile_id: Some(profile.id.clone()),
            snapshot: profile.snapshot()?,
            notes: build.notes(),
            stash: build.stash.clone(),
        };
        self.state.library.build_mut(build_id)?.active_profile_id =
            draft.profile_id.clone().unwrap();
        self.state.draft = draft;
        self.calculation_revision += 1;
        self.undo.clear();
        self.redo.clear();
        self.changed();
        Ok(())
    }
    pub fn new_build(&mut self, name: &str) -> Result<String, String> {
        if self.state.settings.auto_save {
            self.save_profile()?;
        }
        let id = self.state.library.create(
            name,
            &BuildSnapshot::default(),
            &Notes::default(),
            &[],
            None,
        )?;
        self.open(&id, None)?;
        Ok(id)
    }
    pub fn save_as(&mut self, name: &str) -> Result<String, String> {
        let draft = &self.state.draft;
        let id =
            self.state
                .library
                .create(name, &draft.snapshot, &draft.notes, &draft.stash, None)?;
        self.open(&id, None)?;
        Ok(id)
    }
    pub fn import_code(&mut self, code: &str) -> Result<String, String> {
        let (snapshot, notes) = codec::decode(code)?;
        let name = snapshot
            .class_id
            .as_deref()
            .and_then(hsplanner_engine::calc::data::get_class)
            .map(|c| format!("Imported {}", c.name))
            .unwrap_or_else(|| "Imported build".into());
        let id = self
            .state
            .library
            .create(&name, &snapshot, &notes, &[], None)?;
        self.open(&id, None)?;
        Ok(id)
    }
    pub fn add_profile(&mut self, name: &str, copy_from: Option<&str>) -> Result<(), String> {
        self.save_profile()?;
        let id = self
            .state
            .draft
            .build_id
            .clone()
            .ok_or("Save this build first.")?;
        let snapshot = match copy_from {
            Some(profile) => self
                .state
                .library
                .build(&id)
                .and_then(|b| b.profile(profile))
                .ok_or("Profile no longer exists.")?
                .snapshot()?,
            None => self.snapshot().clone(),
        };
        let profile = Profile::new(name, &snapshot)?;
        let profile_id = profile.id.clone();
        let build = self.state.library.build_mut(&id)?;
        if build.profiles.len() >= 100 {
            return Err("This build already contains 100 profiles.".into());
        }
        build.profiles.push(profile);
        self.open(&id, Some(&profile_id))
    }
    pub fn rename_profile(&mut self, id: &str, name: &str) -> Result<(), String> {
        let build_id = self
            .state
            .draft
            .build_id
            .clone()
            .ok_or("Save this build first.")?;
        let build = self.state.library.build_mut(&build_id)?;
        build
            .profiles
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or("Profile no longer exists.")?
            .name = clean_name(name)?;
        self.changed();
        Ok(())
    }
    pub fn remove_profile(&mut self, id: &str) -> Result<(), String> {
        let build_id = self
            .state
            .draft
            .build_id
            .clone()
            .ok_or("Save this build first.")?;
        let build = self.state.library.build_mut(&build_id)?;
        if build.profiles.len() <= 1 {
            return Err("Keep at least one profile.".into());
        }
        build.profiles.retain(|p| p.id != id);
        if build.active_profile_id == id {
            build.active_profile_id = build.profiles[0].id.clone();
        }
        if self.state.draft.profile_id.as_deref() == Some(id) {
            self.state.draft.build_id = None;
            self.state.draft.profile_id = None;
            self.open(&build_id, None)?;
        }
        self.changed();
        Ok(())
    }
    pub fn remove_build(&mut self, id: &str) {
        self.state.library.remove(id);
        if self.state.draft.build_id.as_deref() == Some(id) {
            self.state.draft.build_id = None;
            self.state.draft.profile_id = None;
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
                    for profile in build.profiles {
                        if !existing
                            .profiles
                            .iter()
                            .any(|current| current.id == profile.id)
                        {
                            existing.profiles.push(profile);
                        }
                    }
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

    #[test]
    fn profiles_preserve_local_fields_and_history_stops_at_document_boundaries() {
        let mut session = Session::new(WorkspaceState::default());
        let id = session.new_build("Test").unwrap();
        session.edit(|d| {
            d.snapshot.entity_rates.insert("summon".into(), 2.5);
            d.snapshot.allocated_tree_nodes = vec![5, 3, 8];
            d.snapshot.set_max_subskill_points(27);
        });
        session.save_profile().unwrap();
        session.add_profile("Second", None).unwrap();
        session.edit(|d| {
            d.snapshot.level = 40;
            d.snapshot.set_max_subskill_points(30);
        });
        session.undo();
        assert_eq!(session.snapshot().level, 1);
        assert_eq!(session.snapshot().subskill_point_budget(), 27);
        session.redo();
        assert_eq!(session.snapshot().level, 40);
        assert_eq!(session.snapshot().subskill_point_budget(), 30);
        let first = session.state.library.build(&id).unwrap().profiles[0]
            .id
            .clone();
        session.open(&id, Some(&first)).unwrap();
        assert_eq!(session.snapshot().entity_rates["summon"], 2.5);
        assert_eq!(session.snapshot().allocated_tree_nodes, [5, 3, 8]);
        assert_eq!(session.snapshot().subskill_point_budget(), 27);
        assert!(!session.has_undo());
        assert!(!session.has_redo());
    }
}
