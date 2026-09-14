use hsplanner_build::library::SavedBuild;

pub(super) fn matches(
    build: &SavedBuild,
    search: &str,
    favorites: bool,
    unfiled: bool,
    folder: Option<&str>,
    tag: Option<&str>,
    high_level: bool,
) -> bool {
    (!favorites || build.favorite)
        && (!unfiled || build.folder_id.is_none())
        && folder.is_none_or(|id| build.folder_id.as_deref() == Some(id))
        && tag.is_none_or(|tag| build.tags.iter().any(|value| value == tag))
        && (!high_level || profile_summary(build, false).is_some_and(|s| s.0 >= 90))
        && (search.is_empty()
            || build.name.to_lowercase().contains(search)
            || build
                .class_id
                .as_ref()
                .is_some_and(|class| class.to_lowercase().contains(search))
            || build
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(search)))
}

/// Read row metadata without cloning inventories, trees and every skill in a profile.
/// Encoded-only imported profiles keep their existing decoding fallback.
pub(super) fn profile_summary(build: &SavedBuild, fallback: bool) -> Option<(u32, usize)> {
    let profile = build
        .profile(&build.active_profile_id)
        .or_else(|| fallback.then(|| build.profiles.first()).flatten())?;
    if let Some(snapshot) = &profile.snapshot {
        Some((snapshot.level, snapshot.allocated_tree_nodes.len()))
    } else {
        profile
            .snapshot()
            .ok()
            .map(|snapshot| (snapshot.level, snapshot.allocated_tree_nodes.len()))
    }
}

#[derive(PartialEq, Eq)]
pub(super) struct Filter {
    pub search: String,
    pub favorites: bool,
    pub unfiled: bool,
    pub folder: Option<String>,
    pub tag: Option<String>,
    pub high_level: bool,
    pub recent: bool,
    pub sort: super::LibrarySort,
}

#[derive(Default)]
pub(super) struct Cache {
    filter: Option<Filter>,
    indices: Vec<usize>,
}

impl Cache {
    pub fn invalidate(&mut self) {
        self.filter = None;
    }

    /// The owner invalidates this cache when the session changes. Hover, scroll,
    /// preview completion and paging reuse these lightweight ordered indices.
    pub fn indices(&mut self, builds: &[SavedBuild], filter: Filter) -> &[usize] {
        if self.filter.as_ref() != Some(&filter) {
            self.indices = builds
                .iter()
                .enumerate()
                .filter(|(_, build)| {
                    matches(
                        build,
                        &filter.search,
                        filter.favorites,
                        filter.unfiled,
                        filter.folder.as_deref(),
                        filter.tag.as_deref(),
                        filter.high_level,
                    )
                })
                .map(|(i, _)| i)
                .collect();
            filter
                .sort
                .apply_indices(&mut self.indices, builds, filter.recent);
            self.filter = Some(filter);
        }
        &self.indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsplanner_build::{BuildSnapshot, library::Library, notes::Notes};
    fn build() -> SavedBuild {
        let mut library = Library::default();
        let snapshot = BuildSnapshot {
            class_id: Some("stormweaver".into()),
            level: 90,
            ..Default::default()
        };
        library
            .create("Lightning", &snapshot, &Notes::default(), &[], None)
            .unwrap();
        let mut build = library.builds.remove(0);
        build.tags = vec!["Budget".into()];
        build
    }
    fn filter() -> Filter {
        Filter {
            search: String::new(),
            favorites: false,
            unfiled: false,
            folder: None,
            tag: None,
            high_level: false,
            recent: false,
            sort: super::super::LibrarySort {
                column: super::super::SortColumn::Name,
                direction: super::super::SortDirection::Ascending,
            },
        }
    }

    #[test]
    fn cached_indices_follow_filters_and_same_length_library_edits() {
        let mut builds = vec![build(), build()];
        builds[0].name = "Zulu".into();
        builds[1].name = "Alpha".into();
        let mut cache = Cache::default();
        assert_eq!(cache.indices(&builds, filter()), &[1, 0]);
        let pointer = cache.indices.as_ptr();
        assert_eq!(cache.indices(&builds, filter()), &[1, 0]);
        assert_eq!(
            pointer,
            cache.indices.as_ptr(),
            "unchanged query reuses its allocation"
        );
        builds[0].name = "Aardvark".into();
        cache.invalidate();
        assert_eq!(cache.indices(&builds, filter()), &[0, 1]);
        let mut search = filter();
        search.search = "alpha".into();
        assert_eq!(cache.indices(&builds, search), &[1]);
        builds.remove(0);
        cache.invalidate();
        assert_eq!(cache.indices(&builds, filter()), &[0]);
        builds.clear();
        cache.invalidate();
        assert!(cache.indices(&builds, filter()).is_empty());
    }

    #[test]
    fn row_summary_preserves_encoded_profiles_and_invalid_active_profile_fallback() {
        let mut build = build();
        let expected = profile_summary(&build, true);
        assert_eq!(expected, Some((90, 0)));
        build.profiles[0].snapshot = None;
        assert_eq!(profile_summary(&build, true), expected);
        build.active_profile_id = "missing".into();
        assert_eq!(profile_summary(&build, true), expected);
        assert_eq!(profile_summary(&build, false), None);
        build.profiles[0].code = "invalid".into();
        assert_eq!(profile_summary(&build, true), None);
    }

    #[test]
    fn search_covers_name_class_and_tags() {
        let build = build();
        for query in ["light", "storm", "budget", ""] {
            assert!(matches(&build, query, false, false, None, None, false));
        }
        assert!(!matches(&build, "missing", false, false, None, None, false));
    }
    #[test]
    fn scopes_compose_with_tags_and_level() {
        let mut build = build();
        assert!(matches(&build, "", false, true, None, Some("Budget"), true));
        assert!(!matches(&build, "", true, false, None, None, false));
        build.favorite = true;
        build.folder_id = Some("folder".into());
        assert!(matches(
            &build,
            "",
            true,
            false,
            Some("folder"),
            Some("Budget"),
            true
        ));
        assert!(!matches(&build, "", false, true, None, None, false));
        assert!(!matches(
            &build,
            "",
            false,
            false,
            Some("other"),
            None,
            false
        ));
        build.profiles[0].snapshot.as_mut().unwrap().level = 89;
        assert!(!matches(&build, "", false, false, None, None, true));
    }
}
