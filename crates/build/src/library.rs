use serde::{Deserialize, Serialize};

use crate::{BuildSnapshot, codec, loadout::Loadouts, notes::Notes};

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4())
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Original native profile payloads retained only for compatibility recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LegacyProfile {
    id: String,
    name: String,
    code: String,
    updated_at: String,
    #[serde(default)]
    snapshot: Option<BuildSnapshot>,
}

impl LegacyProfile {
    fn snapshot(&self) -> Result<BuildSnapshot, String> {
        self.snapshot
            .clone()
            .map(Ok)
            .unwrap_or_else(|| codec::decode(&self.code).map(|v| v.0))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StashEntry {
    pub id: String,
    pub saved_at: u64,
    pub item: hsplanner_engine::calc::types::EquippedItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "SavedBuildWire")]
#[serde(rename_all = "camelCase")]
pub struct SavedBuild {
    pub id: String,
    pub name: String,
    pub class_id: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub native_notes: Option<Notes>,
    pub created_at: String,
    pub updated_at: String,
    pub snapshot: BuildSnapshot,
    pub loadouts: Loadouts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) archived_profiles: Vec<LegacyProfile>,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "current_season")]
    pub season: String,
    #[serde(default)]
    pub stash: Vec<StashEntry>,
}

fn current_season() -> String {
    "s10".into()
}

impl SavedBuild {
    pub fn notes(&self) -> Notes {
        self.native_notes
            .clone()
            .unwrap_or_else(|| Notes::from_html(&self.notes))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedBuildWire {
    id: String,
    name: String,
    class_id: Option<String>,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    native_notes: Option<Notes>,
    created_at: String,
    updated_at: String,
    snapshot: Option<BuildSnapshot>,
    loadouts: Option<Loadouts>,
    #[serde(default)]
    archived_profiles: Vec<LegacyProfile>,
    profiles: Option<Vec<LegacyProfile>>,
    active_profile_id: Option<String>,
    folder_id: Option<String>,
    #[serde(default)]
    favorite: bool,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "current_season")]
    season: String,
    #[serde(default)]
    stash: Vec<StashEntry>,
}

impl TryFrom<SavedBuildWire> for SavedBuild {
    type Error = String;
    fn try_from(wire: SavedBuildWire) -> Result<Self, Self::Error> {
        let (snapshot, loadouts, archived_profiles) = match (wire.snapshot, wire.loadouts) {
            (Some(snapshot), Some(loadouts)) => {
                loadouts.validate()?;
                (snapshot, loadouts, wire.archived_profiles)
            }
            (None, None) => {
                let profiles = wire.profiles.ok_or("Missing build loadouts.")?;
                let active_id = wire.active_profile_id.ok_or("Missing active profile.")?;
                let snapshots: Vec<_> = profiles
                    .iter()
                    .map(|profile| {
                        profile
                            .snapshot()
                            .map(|snapshot| (profile.id.clone(), profile.name.clone(), snapshot))
                    })
                    .collect::<Result<_, _>>()?;
                let snapshot = snapshots
                    .iter()
                    .find(|(id, _, _)| id == &active_id)
                    .ok_or("Invalid active profile.")?
                    .2
                    .clone();
                let loadouts = Loadouts::from_legacy(&snapshots, &active_id)?;
                (snapshot, loadouts, profiles)
            }
            _ => return Err("Incomplete build loadouts.".into()),
        };
        Ok(Self {
            id: wire.id,
            name: wire.name,
            class_id: wire.class_id,
            notes: wire.notes,
            native_notes: wire.native_notes,
            created_at: wire.created_at,
            updated_at: wire.updated_at,
            snapshot,
            loadouts,
            archived_profiles,
            folder_id: wire.folder_id,
            favorite: wire.favorite,
            tags: wire.tags,
            season: wire.season,
            stash: wire.stash,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "LibraryWire")]
pub struct Library {
    pub version: u32,
    pub builds: Vec<SavedBuild>,
    pub folders: Vec<Folder>,
}

#[derive(Deserialize)]
struct LibraryWire {
    version: u32,
    builds: Vec<SavedBuild>,
    folders: Vec<Folder>,
}
impl TryFrom<LibraryWire> for Library {
    type Error = String;
    fn try_from(wire: LibraryWire) -> Result<Self, Self::Error> {
        if !matches!(wire.version, 3 | 4) {
            return Err("Unsupported library version. The original data was not changed.".into());
        }
        let library = Self {
            version: 4,
            builds: wire.builds,
            folders: wire.folders,
        };
        library.validate()?;
        Ok(library)
    }
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 4,
            builds: Vec::new(),
            folders: Vec::new(),
        }
    }
}

impl Library {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 4 {
            return Err("Unsupported library version. The original data was not changed.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for build in &self.builds {
            if build.id.is_empty() || !ids.insert(&build.id) {
                return Err(format!("Invalid library record: {}", build.name));
            }
            build
                .loadouts
                .validate()
                .map_err(|error| format!("{}: {error}", build.name))?;
        }
        let folders: std::collections::HashMap<_, _> =
            self.folders.iter().map(|f| (f.id.as_str(), f)).collect();
        if folders.len() != self.folders.len() {
            return Err("Duplicate folder identifiers.".into());
        }
        for folder in &self.folders {
            let mut seen = std::collections::HashSet::from([folder.id.as_str()]);
            let mut parent = folder.parent_id.as_deref();
            while let Some(id) = parent {
                if !seen.insert(id) {
                    return Err("The folder hierarchy contains a cycle.".into());
                }
                parent = folders
                    .get(id)
                    .ok_or("A parent folder is missing.")?
                    .parent_id
                    .as_deref();
            }
        }
        Ok(())
    }
    pub fn build(&self, id: &str) -> Option<&SavedBuild> {
        self.builds.iter().find(|b| b.id == id)
    }
    pub fn build_mut(&mut self, id: &str) -> Result<&mut SavedBuild, String> {
        self.builds
            .iter_mut()
            .find(|b| b.id == id)
            .ok_or_else(|| "Build no longer exists.".into())
    }
    pub fn create(
        &mut self,
        name: &str,
        snapshot: &BuildSnapshot,
        notes: &Notes,
        stash: &[StashEntry],
        folder_id: Option<String>,
    ) -> Result<String, String> {
        if self.builds.len() >= 1_000 {
            return Err("The library already contains 1,000 builds.".into());
        }
        if folder_id
            .as_ref()
            .is_some_and(|id| !self.folders.iter().any(|f| &f.id == id))
        {
            return Err("Folder no longer exists.".into());
        }
        let loadouts = Loadouts::from_snapshot(snapshot);
        let id = new_id("b");
        self.builds.push(SavedBuild {
            id: id.clone(),
            name: clean_name(name)?,
            class_id: snapshot.class_id.clone(),
            notes: notes.to_html(),
            native_notes: Some(notes.clone()),
            created_at: now(),
            updated_at: now(),
            snapshot: snapshot.clone(),
            loadouts,
            archived_profiles: Vec::new(),
            folder_id,
            favorite: false,
            tags: Vec::new(),
            season: snapshot.season.clone(),
            stash: stash.to_vec(),
        });
        Ok(id)
    }
    pub fn duplicate(&mut self, id: &str) -> Result<String, String> {
        if self.builds.len() >= 1_000 {
            return Err("The library already contains 1,000 builds.".into());
        }
        let mut build = self.build(id).ok_or("Build no longer exists.")?.clone();
        build.id = new_id("b");
        build.name = duplicate_name(&build.name, self.builds.iter().map(|b| b.name.as_str()));
        build.created_at = now();
        build.updated_at = now();
        let id = build.id.clone();
        self.builds.push(build);
        Ok(id)
    }
    pub fn remove(&mut self, id: &str) {
        self.builds.retain(|b| b.id != id);
    }
    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        self.build_mut(id)?.name = clean_name(name)?;
        Ok(())
    }
    pub fn set_tags(&mut self, id: &str, tags: &str) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        self.build_mut(id)?.tags = tags
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.chars().take(40).collect::<String>())
            .filter(|s| seen.insert(s.to_lowercase()))
            .take(24)
            .collect();
        Ok(())
    }
    pub fn create_folder(
        &mut self,
        name: &str,
        parent_id: Option<String>,
    ) -> Result<String, String> {
        if self.folders.len() >= 500 {
            return Err("The library already contains 500 folders.".into());
        }
        if parent_id
            .as_ref()
            .is_some_and(|id| !self.folders.iter().any(|f| &f.id == id))
        {
            return Err("Parent folder no longer exists.".into());
        }
        let id = new_id("f");
        self.folders.push(Folder {
            id: id.clone(),
            name: clean_name(name)?,
            parent_id,
            created_at: now(),
        });
        Ok(id)
    }
    pub fn rename_folder(&mut self, id: &str, name: &str) -> Result<(), String> {
        self.folders
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or("Folder no longer exists.")?
            .name = clean_name(name)?;
        Ok(())
    }
    pub fn remove_folder(&mut self, id: &str, cascade: bool) {
        let parent = self
            .folders
            .iter()
            .find(|f| f.id == id)
            .and_then(|f| f.parent_id.clone());
        let mut removed = std::collections::HashSet::from([id.to_owned()]);
        if cascade {
            loop {
                let len = removed.len();
                for folder in &self.folders {
                    if folder
                        .parent_id
                        .as_ref()
                        .is_some_and(|p| removed.contains(p))
                    {
                        removed.insert(folder.id.clone());
                    }
                }
                if len == removed.len() {
                    break;
                }
            }
            self.builds
                .retain(|b| !b.folder_id.as_ref().is_some_and(|id| removed.contains(id)));
        } else {
            for folder in &mut self.folders {
                if folder.parent_id.as_deref() == Some(id) {
                    folder.parent_id = parent.clone();
                }
            }
            for build in &mut self.builds {
                if build.folder_id.as_deref() == Some(id) {
                    build.folder_id = parent.clone();
                }
            }
        }
        self.folders.retain(|f| !removed.contains(&f.id));
    }
}

pub fn clean_name(name: &str) -> Result<String, String> {
    let value: String = name.trim().chars().take(500).collect();
    if value.is_empty() {
        Err("Enter a name.".into())
    } else {
        Ok(value)
    }
}

pub fn duplicate_name<'a>(name: &str, taken: impl Iterator<Item = &'a str>) -> String {
    let taken: std::collections::HashSet<_> = taken.collect();
    let base = format!("{name} (copy)");
    if !taken.contains(base.as_str()) {
        return base;
    }
    for i in 2.. {
        let value = format!("{name} (copy {i})");
        if !taken.contains(value.as_str()) {
            return value;
        }
    }
    unreachable!()
}
