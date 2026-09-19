use hsplanner_build::{
    BuildSnapshot, codec,
    library::Library,
    session::{Session, Settings, WorkspaceState},
    storage::{MigrationExport, Writer, load, write_atomic},
};

fn transfer(library: Library) -> MigrationExport {
    MigrationExport {
        version: 1,
        exported_at: "transfer-1".into(),
        library,
        snapshot: BuildSnapshot {
            level: 60,
            ..Default::default()
        },
        active_build_id: None,
        active_profile_id: None,
        notes: "<h1>Original notes</h1><p><b>Keep me</b></p>".into(),
        stash: vec![],
        settings: Settings::default(),
        filters: serde_json::json!({"version":1,"filters":[{"id":"legacy-filter"}]}),
        storage: [("heroplanner.savedBuilds.v1".into(), "original bytes".into())].into(),
    }
}

#[test]
fn legacy_transfer_is_ignored_even_after_empty_first_launch() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("gpui");
    write_atomic(&directory, &WorkspaceState::default()).unwrap();
    let export_path = temp.path().join("migration-v1.json");
    let bytes = b"invalid old export must not block native startup";
    std::fs::write(&export_path, bytes).unwrap();
    let state = load(&directory).unwrap();
    assert!(state.is_pristine());
    assert!(state.migrated_from.is_none());
    assert_eq!(std::fs::read(export_path).unwrap(), bytes);
}

#[test]
fn late_transfer_preserves_native_edits_ids_and_the_unsaved_tauri_document() {
    let mut session = Session::new(WorkspaceState::default());
    let id = session.new_build("Shared build").unwrap();
    session.edit(|draft| draft.snapshot.level = 40);
    session.save_build().unwrap();
    let legacy = session.state().library.clone();
    session.edit(|draft| draft.snapshot.level = 80);
    session.save_build().unwrap();
    session.import_transfer(transfer(legacy.clone())).unwrap();
    assert_eq!(session.snapshot().level, 80);
    assert_eq!(
        session.state().library.build(&id).unwrap().snapshot.level,
        80
    );
    assert_eq!(session.state().library.builds.len(), 2);
    let recovered = session
        .state()
        .library
        .builds
        .iter()
        .find(|build| build.id != id)
        .unwrap();
    assert_eq!(recovered.snapshot.level, 60);
    assert!(recovered.notes().original_html.is_some());
    assert_eq!(
        session.state().legacy_storage["heroplanner.savedBuilds.v1"],
        "original bytes"
    );
    let before = serde_json::to_value(session.state()).unwrap();
    assert!(session.import_transfer(transfer(legacy)).is_err());
    assert_eq!(serde_json::to_value(session.state()).unwrap(), before);
}

#[test]
fn failed_transfer_is_transactional_and_custom_filters_are_not_pristine() {
    let state = WorkspaceState {
        filters: serde_json::json!({"version":1,"filters":[{"id":"native"}]}),
        ..Default::default()
    };
    assert!(!state.is_pristine());
    let mut session = Session::new(state);
    let mut bad = transfer(Library::default());
    bad.version = 999;
    let before = serde_json::to_value(session.state()).unwrap();
    assert!(session.import_transfer(bad).is_err());
    assert_eq!(serde_json::to_value(session.state()).unwrap(), before);
}

#[test]
fn metadata_and_notes_do_not_invalidate_calculations_and_stale_save_cannot_clear_dirty() {
    let mut session = Session::new(WorkspaceState::default());
    session.new_build("Test").unwrap();
    let calculation = session.calculation_revision();
    let revision = session.revision();
    session.edit(|draft| draft.notes.markdown = "New notes".into());
    session.save_build().unwrap();
    assert_eq!(session.calculation_revision(), calculation);
    session.persisted(revision);
    assert!(session.is_dirty());
    session.edit(|draft| draft.snapshot.level = 20);
    assert_eq!(session.calculation_revision(), calculation + 1);
    session.persisted(session.revision());
    assert!(!session.is_dirty());
}

#[test]
fn writer_coalesces_to_newest_revision_and_recovery_respects_the_instance_lock() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("gpui");
    let (writer, mut state) = Writer::open(directory.clone()).unwrap();
    for revision in 1..=150 {
        state.revision = revision;
        state.draft.snapshot.level = revision as u32;
        writer.save(&state).unwrap();
    }
    assert_eq!(writer.flush().unwrap(), 150);
    assert_eq!(load(&directory).unwrap().draft.snapshot.level, 150);
    assert!(Writer::recover(directory).is_err());
}

#[test]
fn reference_and_native_share_codes_decode_to_the_same_document() {
    let front: serde_json::Value = serde_json::from_str(include_str!(
        "../../../engine/tests/fixtures/frontend-share-codes.json"
    ))
    .unwrap();
    let native: serde_json::Value = serde_json::from_str(include_str!(
        "../../../engine/tests/fixtures/native-share-codes.json"
    ))
    .unwrap();
    for (before, after) in front
        .as_array()
        .unwrap()
        .iter()
        .zip(native.as_array().unwrap())
    {
        let a = codec::decode(before["code"].as_str().unwrap()).unwrap();
        let b = codec::decode(after["code"].as_str().unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(a.0).unwrap(),
            serde_json::to_value(b.0).unwrap()
        );
    }
}
