use serde::{Deserialize, Serialize};

use crate::{BuildSnapshot, codec, notes::Notes};

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4())
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub code: String,
    pub updated_at: String,
    #[serde(default)]
    pub snapshot: Option<BuildSnapshot>,
}

impl Profile {
    pub fn snapshot(&self) -> Result<BuildSnapshot, String> {
        self.snapshot
            .clone()
            .map(Ok)
            .unwrap_or_else(|| codec::decode(&self.code).map(|v| v.0))
    }
    pub fn new(name: &str, snapshot: &BuildSnapshot) -> Result<Self, String> {
        Ok(Self {
            id: new_id("p"),
            name: clean_name(name)?,
            code: codec::encode(snapshot, &Notes::default())?,
            updated_at: now(),
            snapshot: Some(snapshot.clone()),
        })
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
    pub profiles: Vec<Profile>,
    pub active_profile_id: String,
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
    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
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
pub struct Library {
    pub version: u32,
    pub builds: Vec<SavedBuild>,
    pub folders: Vec<Folder>,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 3,
            builds: Vec::new(),
            folders: Vec::new(),
        }
    }
}

impl Library {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 3 {
            return Err("Unsupported library version. The original data was not changed.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for build in &self.builds {
            if !ids.insert(&build.id)
                || build.profiles.is_empty()
                || build.profile(&build.active_profile_id).is_none()
            {
                return Err(format!("Invalid library record: {}", build.name));
            }
            let mut profiles = std::collections::HashSet::new();
            if build.profiles.iter().any(|p| !profiles.insert(&p.id)) {
                return Err(format!("Duplicate profile in {}", build.name));
            }
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
        let profile = Profile::new("Default", snapshot)?;
        let id = new_id("b");
        self.builds.push(SavedBuild {
            id: id.clone(),
            name: clean_name(name)?,
            class_id: snapshot.class_id.clone(),
            notes: notes.to_html(),
            native_notes: Some(notes.clone()),
            created_at: now(),
            updated_at: now(),
            active_profile_id: profile.id.clone(),
            profiles: vec![profile],
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
        for profile in &mut build.profiles {
            let id = new_id("p");
            if profile.id == build.active_profile_id {
                build.active_profile_id = id.clone();
            }
            profile.id = id;
        }
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
