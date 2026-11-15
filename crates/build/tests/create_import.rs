use hsplanner_build::{
    BuildSnapshot, codec,
    loadout::Loadouts,
    notes::Notes,
    session::{Session, WorkspaceState},
};
use hsplanner_engine::calc::data;

fn session() -> Session {
    let mut state = WorkspaceState::default();
    state.settings.auto_save = false;
    Session::new(state)
}

fn folder(session: &mut Session, name: &str) -> String {
    session
        .edit_library(|library| library.create_folder(name, None))
        .unwrap()
}

fn code(name: Option<&str>) -> String {
    let snapshot = BuildSnapshot::default();
    codec::encode_loadouts_with_stash(
        name,
        &snapshot,
        &Notes::default(),
        &Loadouts::from_snapshot(&snapshot),
        &[],
    )
    .unwrap()
}

#[test]
fn create_build_sets_class_level_and_folder_and_opens_it() {
    let mut session = session();
    let folder = folder(&mut session, "Leveling");
    let max = data::game_config().max_character_level;

    let id = session
        .create_build("Fire", "pyromancer", max + 50, Some(folder.clone()))
        .unwrap();

    let build = session.state().library.build(&id).unwrap();
    assert_eq!(build.name, "Fire");
    assert_eq!(build.class_id.as_deref(), Some("pyromancer"));
    assert_eq!(build.snapshot.level, max);
    assert_eq!(build.folder_id, Some(folder));
    assert_eq!(session.draft().build_id.as_deref(), Some(id.as_str()));
    assert_eq!(session.snapshot().class_id.as_deref(), Some("pyromancer"));
}

#[test]
fn create_build_rejects_unknown_class_and_missing_folder_without_adding_a_build() {
    let mut session = session();

    assert!(session.create_build("A", "no_such_class", 1, None).is_err());
    assert!(
        session
            .create_build("A", "viking", 1, Some("gone".into()))
            .is_err()
    );
    assert!(session.create_build("  ", "viking", 1, None).is_err());

    assert!(session.state().library.builds.is_empty());
}

#[test]
fn free_name_numbers_a_taken_name() {
    let mut session = session();
    assert_eq!(
        session.state().library.free_name("New Viking"),
        "New Viking"
    );

    session.new_build("New Viking").unwrap();
    assert_eq!(
        session.state().library.free_name("New Viking"),
        "New Viking 2"
    );

    session.new_build("New Viking 2").unwrap();
    assert_eq!(
        session.state().library.free_name(" New Viking "),
        "New Viking 3"
    );
}

#[test]
fn share_code_carries_the_build_name_and_import_keeps_it_unique() {
    let mut session = session();
    let code = code(Some("Żółw 🐢 tank"));

    assert_eq!(
        codec::decode_shared(&code).unwrap().name.as_deref(),
        Some("Żółw 🐢 tank")
    );
    let first = session.import_code(&code).unwrap();
    let second = session.import_code(&code).unwrap();

    let library = &session.state().library;
    assert_eq!(library.build(&first).unwrap().name, "Żółw 🐢 tank");
    assert_eq!(library.build(&second).unwrap().name, "Żółw 🐢 tank 2");
}

#[test]
fn code_without_a_name_imports_under_the_class_name() {
    let mut session = session();
    let legacy = codec::encode(&BuildSnapshot::default(), &Notes::default()).unwrap();

    for code in [code(None), legacy] {
        assert_eq!(codec::decode_shared(&code).unwrap().name, None);
        let id = session.import_code(&code).unwrap();
        assert!(
            session
                .state()
                .library
                .build(&id)
                .unwrap()
                .name
                .starts_with("Imported Viking")
        );
    }
}

#[test]
fn shared_name_is_cleaned_at_the_boundary() {
    let long = "x".repeat(900);
    let cases = [
        ("  Bad\nName\u{7} ", Some("Bad Name".to_owned())),
        (" \t\n ", None),
        ("\u{202E}Evil\u{200B}\u{2066}", Some("Evil".to_owned())),
        (long.as_str(), Some("x".repeat(500))),
    ];

    for (name, expected) in cases {
        assert_eq!(
            codec::decode_shared(&code(Some(name))).unwrap().name,
            expected
        );
    }
}

#[test]
fn import_shared_uses_the_given_name_and_folder() {
    let mut session = session();
    let folder = folder(&mut session, "Friends");
    let shared = codec::decode_shared(&code(Some("Original"))).unwrap();

    let id = session
        .import_shared(shared, "Renamed", Some(folder.clone()))
        .unwrap();

    let build = session.state().library.build(&id).unwrap();
    assert_eq!(build.name, "Renamed");
    assert_eq!(build.folder_id, Some(folder));
    assert_eq!(session.draft().build_id.as_deref(), Some(id.as_str()));
}

#[test]
fn import_shared_fails_cleanly_on_a_blank_name() {
    let mut session = session();
    let shared = codec::decode_shared(&code(None)).unwrap();

    assert!(session.import_shared(shared, " ", None).is_err());
    assert!(session.state().library.builds.is_empty());
}
