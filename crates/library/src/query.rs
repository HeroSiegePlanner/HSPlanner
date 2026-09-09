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
        && (!high_level
            || build
                .profile(&build.active_profile_id)
                .and_then(|p| p.snapshot().ok())
                .is_some_and(|s| s.level >= 90))
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
