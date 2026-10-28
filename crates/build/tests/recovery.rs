use hsplanner_build::{
    codec,
    session::{Session, WorkspaceState},
    storage::{Writer, load, write_atomic},
};

#[test]
fn cast_consumed_count_survives_native_save_and_complete_share_code() {
    let mut session = Session::new(WorkspaceState::default());
    session.new_build("Fuel snapshot").unwrap();
    session.edit(|draft| {
        draft.snapshot.set_class("butcher");
        draft.snapshot.skill_ranks.insert("fuel_to_fire".into(), 1);
        draft
            .snapshot
            .active_buffs
            .insert("fuel_to_fire".into(), true);
        draft
            .snapshot
            .stack_counts
            .insert("fuel_to_fire_consumed_objects".into(), 5);
    });
    session.save_build().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_atomic(temp.path(), session.state()).unwrap();
    let restored = Session::new(load(temp.path()).unwrap());
    let code = codec::encode_loadouts(
        restored.snapshot(),
        &restored.draft().notes,
        &restored.draft().loadouts,
    )
    .unwrap();
    let (snapshot, _, _) = codec::decode_loadouts(&code).unwrap();
    assert_eq!(snapshot.stack_counts["fuel_to_fire_consumed_objects"], 5);
    assert_eq!(snapshot.skill_ranks["fuel_to_fire"], 1);
    assert!(snapshot.active_buffs["fuel_to_fire"]);
}

#[test]
fn butcher_subskill_buffs_survive_native_save_share_and_skill_loadout_switch() {
    use hsplanner_build::loadout::LoadoutKind;
    use hsplanner_engine::calc::{data, subskill::subskill_key};

    let skills = data::get_skills_by_class("butcher");
    let buffs: Vec<_> = [
        ("chain_rip", "prey_on_the_weak"),
        ("butchers_hook", "hellsent"),
        ("chain_swing", "killing_spree"),
        ("chain_rip", "rage_trance"),
    ]
    .into_iter()
    .map(|(parent, id)| {
        let skill = skills.iter().find(|skill| skill.id == parent).unwrap();
        let node = skill
            .subskills
            .as_ref()
            .unwrap()
            .iter()
            .find(|node| node.id == id)
            .unwrap();
        assert!(node.triggered_buff.is_some(), "{parent}:{id}");
        (parent, subskill_key(parent, id))
    })
    .collect();
    let mut session = Session::new(WorkspaceState::default());
    session.new_build("Temporary Butcher buffs").unwrap();
    session.edit(|draft| {
        draft.snapshot.set_class("butcher");
        for (parent, key) in &buffs {
            draft.snapshot.skill_ranks.insert((*parent).into(), 1);
            draft.snapshot.subskill_ranks.insert(key.clone(), 1);
            draft.snapshot.active_buffs.insert(key.clone(), true);
        }
    });
    session
        .add_loadout(LoadoutKind::Skills, "Buff inactive")
        .unwrap();
    let inactive = session
        .draft()
        .loadouts
        .active_id(LoadoutKind::Skills)
        .to_owned();
    session.edit(|draft| {
        for (_, key) in &buffs {
            draft.snapshot.active_buffs.insert(key.clone(), false);
        }
    });
    session
        .switch_loadout(LoadoutKind::Skills, "default")
        .unwrap();
    for (_, key) in &buffs {
        assert!(session.snapshot().active_buffs[key], "{key}");
    }
    session.save_build().unwrap();

    let temp = tempfile::tempdir().unwrap();
    write_atomic(temp.path(), session.state()).unwrap();
    let restored = Session::new(load(temp.path()).unwrap());
    for (_, key) in &buffs {
        assert!(restored.snapshot().active_buffs[key], "{key}");
    }
    let code = codec::encode_loadouts(
        restored.snapshot(),
        &restored.draft().notes,
        &restored.draft().loadouts,
    )
    .unwrap();
    let mut imported = Session::new(WorkspaceState::default());
    imported.import_code(&code).unwrap();
    for (parent, key) in &buffs {
        assert!(imported.snapshot().active_buffs[key], "{key}");
        assert_eq!(imported.snapshot().skill_ranks[*parent], 1);
        assert_eq!(imported.snapshot().subskill_ranks[key], 1);
    }
    imported
        .switch_loadout(LoadoutKind::Skills, &inactive)
        .unwrap();
    for (_, key) in &buffs {
        assert!(!imported.snapshot().active_buffs[key], "{key}");
    }
    imported
        .switch_loadout(LoadoutKind::Skills, "default")
        .unwrap();
    for (_, key) in &buffs {
        assert!(imported.snapshot().active_buffs[key], "{key}");
    }
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
