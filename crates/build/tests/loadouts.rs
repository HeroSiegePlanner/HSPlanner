use hsplanner_build::{
    BuildSnapshot, codec,
    library::StashEntry,
    loadout::{LoadoutKind, Loadouts},
    session::{Session, WorkspaceState},
    storage::{load, write_atomic},
};
use hsplanner_engine::calc::{
    data,
    types::{EquippedItem, TreeSocketContent},
};
use serde_json::{Value, json};

fn value(value: &impl serde::Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}
fn item(id: &str) -> EquippedItem {
    EquippedItem {
        base_id: id.into(),
        ..Default::default()
    }
}
fn socket(id: &str) -> TreeSocketContent {
    TreeSocketContent::Item { id: id.into() }
}

fn populated_session() -> Session {
    let mut state = WorkspaceState::default();
    state.settings.auto_save = false;
    let mut session = Session::new(state);
    session.new_build("Build").unwrap();
    session.edit(|draft| {
        let snapshot = &mut draft.snapshot;
        snapshot.level = 80;
        snapshot.allocated.insert("strength".into(), 10);
        snapshot.allocated_tree_nodes = vec![1, 2];
        snapshot.tree_socketed.insert(2, socket("socket-a"));
        snapshot.allocated_ether_nodes = vec![3, 4];
        snapshot.inventory.insert("weapon".into(), item("weapon-a"));
        snapshot
            .merc_inventory
            .insert("helmet".into(), item("merc-a"));
        snapshot.skill_ranks.insert("skill-a".into(), 3);
        snapshot.subskill_ranks.insert("skill-a:sub".into(), 2);
        snapshot.set_max_subskill_points(27);
        snapshot.active_skill_ids = vec!["skill-a".into()];
        snapshot.active_aura_id = Some("aura-a".into());
        snapshot.active_buffs.insert("buff-a".into(), true);
        snapshot.disabled_potions.insert("potion-a".into(), true);
        snapshot.entity_rates.insert("summon".into(), 2.5);
        snapshot.skill_projectiles.insert("skill-a".into(), 7);
        snapshot.merc_class_id = Some("mercenary".into());
        snapshot.merc_skill_ranks.insert("merc-skill".into(), 4);
        draft.notes.markdown = "Shared notes".into();
        draft.stash.push(StashEntry {
            id: "stash-a".into(),
            item: item("stash-item"),
            ..Default::default()
        });
    });
    session
}

#[test]
fn four_categories_switch_independently_without_saving_and_keep_common_state() {
    let mut session = populated_session();
    let first = session.draft().clone();
    for kind in LoadoutKind::ALL {
        session.add_loadout(kind, "Second").unwrap();
    }
    session.edit(|draft| {
        draft.snapshot.allocated_tree_nodes = vec![11];
        draft.snapshot.tree_socketed.insert(2, socket("socket-b"));
        draft.snapshot.allocated_ether_nodes = vec![12];
        draft
            .snapshot
            .inventory
            .insert("weapon".into(), item("weapon-b"));
        draft
            .snapshot
            .merc_inventory
            .insert("helmet".into(), item("merc-b"));
        draft.snapshot.skill_ranks.insert("skill-a".into(), 9);
        draft
            .snapshot
            .subskill_ranks
            .insert("skill-a:sub".into(), 4);
        draft.snapshot.set_max_subskill_points(30);
        draft.snapshot.active_skill_ids = vec!["skill-b".into()];
        draft.snapshot.active_aura_id = Some("aura-b".into());
        draft.snapshot.active_buffs.clear();
        draft.snapshot.disabled_potions.clear();
        draft.snapshot.level = 90;
    });
    let second = session.draft().clone();
    for kind in LoadoutKind::ALL {
        session
            .switch_loadout(kind, first.loadouts.active_id(kind))
            .unwrap();
        assert_eq!(session.snapshot().level, 90);
        assert_eq!(session.snapshot().entity_rates["summon"], 2.5);
        assert_eq!(session.snapshot().skill_projectiles["skill-a"], 7);
        assert_eq!(session.snapshot().merc_skill_ranks["merc-skill"], 4);
        assert_eq!(session.draft().notes.markdown, "Shared notes");
        assert_eq!(session.draft().stash[0].item.base_id, "stash-item");
    }
    let mut expected = first.snapshot;
    expected.level = 90;
    assert_eq!(value(session.snapshot()), value(&expected));
    for kind in LoadoutKind::ALL {
        session
            .switch_loadout(kind, second.loadouts.active_id(kind))
            .unwrap();
    }
    assert_eq!(value(session.snapshot()), value(&second.snapshot));
    // Auto-save is off: all edits and sets live in the draft until an explicit save.
    assert_eq!(session.state().library.builds[0].snapshot.level, 1);
    let directory = tempfile::tempdir().unwrap();
    write_atomic(directory.path(), session.state()).unwrap();
    let reopened = load(directory.path()).unwrap();
    assert_eq!(value(&reopened.draft), value(session.draft()));
}

#[test]
fn tree_switch_does_not_remove_equipment_that_requires_the_other_tree() {
    let mut session = populated_session();
    let base = data::data()
        .items
        .values()
        .find(|base| base.two_handed == Some(true) && base.base_type == "Sword")
        .unwrap();
    session.edit(|draft| {
        draft
            .snapshot
            .inventory
            .insert("weapon".into(), item(&base.id));
        draft
            .snapshot
            .inventory
            .insert("offhand".into(), item(&base.id));
    });
    session
        .add_loadout(LoadoutKind::Incarnation, "Empty tree")
        .unwrap();
    session.edit(|draft| {
        draft.snapshot.allocated_tree_nodes.clear();
        draft.snapshot.tree_socketed.clear();
    });
    session
        .switch_loadout(LoadoutKind::Incarnation, "default")
        .unwrap();
    assert_eq!(session.snapshot().inventory["offhand"].base_id, base.id);
    let empty = session.draft().loadouts.entries(LoadoutKind::Incarnation)[1]
        .0
        .to_owned();
    session
        .switch_loadout(LoadoutKind::Incarnation, &empty)
        .unwrap();
    assert!(session.snapshot().allocated_tree_nodes.is_empty());
    assert_eq!(session.snapshot().inventory["offhand"].base_id, base.id);
}

#[test]
fn loadout_history_restores_selection_edits_and_deleted_sets_and_metadata_is_calculation_neutral() {
    let mut session = populated_session();
    session.add_loadout(LoadoutKind::Skills, "Second").unwrap();
    let id = session
        .draft()
        .loadouts
        .active_id(LoadoutKind::Skills)
        .to_owned();
    let calculation = session.calculation_revision();
    session
        .rename_loadout(LoadoutKind::Skills, &id, "Boss")
        .unwrap();
    assert_eq!(session.calculation_revision(), calculation);
    session.undo();
    assert_eq!(
        session.draft().loadouts.active_name(LoadoutKind::Skills),
        "Second"
    );
    assert_eq!(session.calculation_revision(), calculation);
    session.redo();
    assert_eq!(
        session.draft().loadouts.active_name(LoadoutKind::Skills),
        "Boss"
    );
    session.edit(|draft| {
        draft.snapshot.skill_ranks.insert("skill-a".into(), 10);
    });
    session
        .switch_loadout(LoadoutKind::Skills, "default")
        .unwrap();
    assert_eq!(session.snapshot().skill_ranks["skill-a"], 3);
    session.undo();
    assert_eq!(session.snapshot().skill_ranks["skill-a"], 10);
    assert_eq!(session.draft().loadouts.active_id(LoadoutKind::Skills), id);
    session.remove_loadout(LoadoutKind::Skills, &id).unwrap();
    assert_eq!(session.snapshot().skill_ranks["skill-a"], 3);
    session.undo();
    assert_eq!(session.snapshot().skill_ranks["skill-a"], 10);
    assert_eq!(session.draft().loadouts.active_id(LoadoutKind::Skills), id);
    session.redo();
    assert_eq!(session.draft().loadouts.count(LoadoutKind::Skills), 1);
}

#[test]
fn selecting_identical_contents_invalidates_identity_consumers_but_no_op_selection_does_not() {
    let mut session = Session::new(WorkspaceState::default());
    session
        .add_loadout(LoadoutKind::Skills, "Identical")
        .unwrap();
    let revision = session.calculation_revision();
    session
        .switch_loadout(LoadoutKind::Skills, "default")
        .unwrap();
    assert_eq!(session.calculation_revision(), revision + 1);
    session
        .switch_loadout(LoadoutKind::Skills, "default")
        .unwrap();
    assert_eq!(session.calculation_revision(), revision + 1);
    session.undo();
    assert_eq!(session.calculation_revision(), revision + 2);
    session.redo();
    assert_eq!(session.calculation_revision(), revision + 3);
}

#[test]
fn failed_loadout_commands_are_transactional_and_last_entry_is_retained() {
    let mut session = populated_session();
    let before = value(session.state());
    for kind in LoadoutKind::ALL {
        assert!(session.remove_loadout(kind, "default").is_err());
        assert!(session.remove_loadout(kind, "missing").is_err());
        assert!(session.rename_loadout(kind, "missing", "Name").is_err());
        assert!(session.rename_loadout(kind, "default", "  ").is_err());
        assert!(session.switch_loadout(kind, "missing").is_err());
        assert!(session.add_loadout(kind, "  ").is_err());
    }
    assert_eq!(value(session.state()), before);
}

#[test]
fn class_change_cannot_restore_previous_class_skills_or_sockets_from_inactive_sets() {
    let mut session = populated_session();
    session.add_loadout(LoadoutKind::Skills, "Second").unwrap();
    session
        .add_loadout(LoadoutKind::Incarnation, "Second")
        .unwrap();
    let before = value(session.draft());
    session.edit(|draft| draft.snapshot.set_class("stormweaver"));
    for kind in [LoadoutKind::Skills, LoadoutKind::Incarnation] {
        session.switch_loadout(kind, "default").unwrap();
    }
    assert_eq!(session.snapshot().class_id.as_deref(), Some("stormweaver"));
    assert!(session.snapshot().skill_ranks.is_empty());
    assert!(session.snapshot().subskill_ranks.is_empty());
    assert!(session.snapshot().active_skill_ids.is_empty());
    assert!(session.snapshot().active_aura_id.is_none());
    assert!(session.snapshot().active_buffs.is_empty());
    assert!(session.snapshot().tree_socketed.is_empty());
    assert_eq!(session.snapshot().inventory["weapon"].base_id, "weapon-a");
    session.undo();
    session.undo();
    session.undo();
    assert_eq!(value(session.draft()), before);
}

#[test]
fn save_as_duplicate_and_share_preserve_all_sets_and_selected_combination() {
    let mut session = populated_session();
    for kind in LoadoutKind::ALL {
        session.add_loadout(kind, "Second").unwrap();
    }
    session.edit(|draft| {
        draft.snapshot.allocated_tree_nodes = vec![71];
        draft.snapshot.allocated_ether_nodes = vec![72];
        draft.snapshot.set_max_subskill_points(30);
    });
    session
        .switch_loadout(LoadoutKind::Gear, "default")
        .unwrap();
    let before = session.draft().clone();
    let saved_id = session.save_as("Saved copy").unwrap();
    assert_eq!(value(&session.draft().loadouts), value(&before.loadouts));
    assert_eq!(value(session.snapshot()), value(&before.snapshot));
    let duplicate = session
        .edit_library(|library| library.duplicate(&saved_id))
        .unwrap();
    session.open(&duplicate).unwrap();
    assert_eq!(value(&session.draft().loadouts), value(&before.loadouts));
    let code = codec::encode_loadouts(
        session.snapshot(),
        &session.draft().notes,
        &session.draft().loadouts,
    )
    .unwrap();
    let (snapshot, notes, loadouts) = codec::decode_loadouts(&code).unwrap();
    assert_eq!(value(&snapshot), value(&before.snapshot));
    assert_eq!(notes.to_html(), before.notes.to_html());
    assert_eq!(value(&loadouts), value(&before.loadouts));
    let imported = session.import_code(&code).unwrap();
    assert_eq!(
        value(&session.state().library.build(&imported).unwrap().loadouts),
        value(&loadouts)
    );
    let legacy_code = codec::encode(&snapshot, &notes).unwrap();
    let (_, _, legacy_loadouts) = codec::decode_loadouts(&legacy_code).unwrap();
    for kind in LoadoutKind::ALL {
        assert_eq!(legacy_loadouts.count(kind), 1);
    }
}

fn native_profiles() -> Value {
    let first = BuildSnapshot {
        level: 50,
        allocated_tree_nodes: vec![1],
        ..Default::default()
    };
    let second = BuildSnapshot {
        level: 70,
        allocated_tree_nodes: vec![2],
        ..Default::default()
    };
    let unsaved = BuildSnapshot {
        level: 99,
        allocated_tree_nodes: vec![3],
        ..second.clone()
    };
    let profiles = json!([
        {"id":"p1","name":"Leveling","code":"unused", "updatedAt":"first", "snapshot":first},
        {"id":"p2","name":"Boss","code":"unused", "updatedAt":"second", "snapshot":second}
    ]);
    json!({"version":1, "revision":42,
        "library":{"version":3, "folders":[], "builds":[{"id":"b1","name":"Old native build",
            "classId":"viking","createdAt":"first","updatedAt":"second",
            "profiles":profiles,"activeProfileId":"p1","notes":"Keep notes"}]},
        "draft":{"buildId":"b1","profileId":"p2","snapshot":unsaved,"notes":{"markdown":"Unsaved notes"}},
        "settings":{"autoSave":false}, "filters":{"version":1,"filters":[]}
    })
}

#[test]
fn native_upgrade_preserves_profiles_unsaved_draft_original_bytes_and_backup_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let original = serde_json::to_vec_pretty(&native_profiles()).unwrap();
    std::fs::write(directory.path().join("state.json"), &original).unwrap();
    let state = load(directory.path()).unwrap();
    assert_eq!(state.version, 2);
    assert_eq!(state.library.version, 4);
    assert_eq!(state.library.builds[0].snapshot.level, 50);
    assert_eq!(state.draft.snapshot.level, 99);
    for kind in LoadoutKind::ALL {
        assert_eq!(state.draft.loadouts.count(kind), 2);
        assert_eq!(state.draft.loadouts.active_id(kind), "p2");
        assert_eq!(state.library.builds[0].loadouts.active_id(kind), "p1");
    }
    assert_eq!(
        value(&state.library.builds[0])["archivedProfiles"],
        native_profiles()["library"]["builds"][0]["profiles"]
    );
    let mut session = Session::new(state);
    session
        .switch_loadout(LoadoutKind::Incarnation, "p1")
        .unwrap();
    assert_eq!(session.snapshot().allocated_tree_nodes, [1]);
    assert_eq!(session.snapshot().level, 99);
    session
        .switch_loadout(LoadoutKind::Incarnation, "p2")
        .unwrap();
    assert_eq!(session.snapshot().allocated_tree_nodes, [3]);
    write_atomic(directory.path(), session.state()).unwrap();
    assert_eq!(
        std::fs::read(directory.path().join("state.before-loadouts.json")).unwrap(),
        original
    );
    assert_eq!(
        std::fs::read(directory.path().join("state.backup.json")).unwrap(),
        original
    );
    assert_eq!(
        value(&load(directory.path()).unwrap().draft),
        value(session.draft())
    );
    std::fs::write(directory.path().join("state.json"), b"incomplete").unwrap();
    let restored = hsplanner_build::storage::restore_backup(directory.path()).unwrap();
    assert_eq!(restored.draft.snapshot.level, 99);
    assert_eq!(restored.draft.loadouts.count(LoadoutKind::Skills), 2);
    assert_eq!(
        std::fs::read(directory.path().join("state.before-loadouts.json")).unwrap(),
        original
    );
}

#[test]
fn invalid_collection_ids_and_selections_are_rejected_without_overwriting_native_data() {
    for invalid in ["missing-selection", "duplicate-id", "empty-collection"] {
        let mut state = value(&WorkspaceState::default());
        let collection = &mut state["draft"]["loadouts"]["skills"];
        match invalid {
            "missing-selection" => collection["activeId"] = json!("missing"),
            "duplicate-id" => {
                let entry = collection["entries"][0].clone();
                collection["entries"].as_array_mut().unwrap().push(entry);
            }
            _ => collection["entries"] = json!([]),
        }
        let directory = tempfile::tempdir().unwrap();
        let bytes = serde_json::to_vec(&state).unwrap();
        std::fs::write(directory.path().join("state.json"), &bytes).unwrap();
        assert!(load(directory.path()).is_err());
        assert!(write_atomic(directory.path(), &WorkspaceState::default()).is_err());
        assert_eq!(
            std::fs::read(directory.path().join("state.json")).unwrap(),
            bytes
        );
    }
    let mut invalid = value(&Loadouts::default());
    invalid["gear"]["activeId"] = json!("missing");
    let wire = json!({"v":3,"snapshot":BuildSnapshot::default(),"loadouts":invalid});
    let code = lz_str::compress_to_encoded_uri_component(&wire.to_string());
    assert!(codec::decode_loadouts(&code).is_err());
}

#[test]
fn shared_loadouts_normalize_inactive_equipment_and_clear_old_season_trees() {
    let mut session = populated_session();
    session.add_loadout(LoadoutKind::Gear, "Inactive").unwrap();
    session
        .switch_loadout(LoadoutKind::Gear, "default")
        .unwrap();
    session
        .add_loadout(LoadoutKind::Incarnation, "Other tree")
        .unwrap();
    session
        .add_loadout(LoadoutKind::Ether, "Other ether")
        .unwrap();
    let mut loadouts = value(&session.draft().loadouts);
    let malicious_item = json!({"baseId":"example", "socketCount":u32::MAX,
        "stars":99, "augment":{"id":"example","level":99}});
    loadouts["gear"]["entries"][1]["value"]["inventory"]["weapon"] = malicious_item;
    let mut snapshot = value(session.snapshot());
    snapshot["season"] = json!("s9");
    snapshot["level"] = json!(20_000);
    let wire = json!({"v":3, "snapshot":snapshot,"loadouts":loadouts});
    let code = lz_str::compress_to_encoded_uri_component(&wire.to_string());
    let (snapshot, _, loadouts) = codec::decode_loadouts(&code).unwrap();
    let normalized = value(&loadouts);
    let weapon = &normalized["gear"]["entries"][1]["value"]["inventory"]["weapon"];
    assert_eq!(weapon["socketCount"], 32);
    assert_eq!(weapon["socketed"].as_array().unwrap().len(), 32);
    assert_eq!(weapon["socketTypes"].as_array().unwrap().len(), 32);
    assert_eq!(weapon["stars"], 5);
    assert_eq!(weapon["augment"]["level"], 7);
    assert_eq!(snapshot.level, 10_000);
    assert_eq!(snapshot.season, "s10");
    for kind in ["incarnation", "ether"] {
        for entry in normalized[kind]["entries"].as_array().unwrap() {
            assert!(entry["value"]["nodes"].as_array().unwrap().is_empty());
            if kind == "incarnation" {
                assert!(entry["value"]["sockets"].as_object().unwrap().is_empty());
            }
        }
    }
}

#[test]
fn malformed_share_collection_shapes_return_errors_without_panicking() {
    for gear in [
        json!("invalid"),
        json!([1]),
        json!({"entries":[{"value":"invalid"}]}),
        json!({"entries":[1]}),
        json!({"entries":[{"value":{}}]}),
    ] {
        let mut loadouts = value(&Loadouts::default());
        loadouts["gear"] = gear;
        let wire = json!({"v":3,"snapshot":BuildSnapshot::default(),"loadouts":loadouts});
        let code = lz_str::compress_to_encoded_uri_component(&wire.to_string());
        assert!(codec::decode_loadouts(&code).is_err());
    }
}
