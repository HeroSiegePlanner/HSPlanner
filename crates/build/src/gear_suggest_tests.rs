use super::*;

fn starter() -> BuildSnapshot {
    let skill = data::get_skills_by_class("amazon")
        .iter()
        .find(|s| s.damage_formula.is_some() || s.damage_per_rank.is_some())
        .expect("amazon has a damage skill");
    let mut snapshot = BuildSnapshot {
        class_id: Some("amazon".into()),
        level: 100,
        active_skill_ids: vec![skill.id.clone()],
        ..Default::default()
    };
    snapshot.skill_ranks.insert(skill.id.clone(), 20);
    let mut commons: Vec<&ItemBase> = data::data()
        .items
        .values()
        .filter(|b| b.rarity == "common" && gear::accepts(&snapshot, "weapon", b, false))
        .collect();
    commons.sort_by(|a, b| a.id.cmp(&b.id));
    for base in commons {
        let mut armed = snapshot.clone();
        equip(
            &mut armed,
            "weapon",
            &gear::make_item(&base.id).unwrap(),
            false,
        )
        .unwrap();
        if combined_dps(&armed.planner_input()) > 0. {
            return armed;
        }
    }
    panic!("no common weapon lets {} deal damage", skill.id);
}

fn only(enable: impl FnOnce(&mut SuggestOptions)) -> SuggestOptions {
    let mut options = SuggestOptions {
        other_items: false,
        sockets: false,
        crystals: false,
        augments: false,
        affixes: false,
        stars: false,
    };
    enable(&mut options);
    options
}

fn step(slot: &str, before: Option<EquippedItem>, item: EquippedItem) -> GearStep {
    GearStep {
        slot: slot.into(),
        pass: 0,
        before,
        item,
        augment_on: false,
        cleared: Vec::new(),
        dps_before: 0.,
        dps_after: 0.,
    }
}

// Wiring guard: replaying the steps through the editor's save path gives every reported DPS.
fn assert_replays(start: &BuildSnapshot, result: &GearSuggestion) -> BuildSnapshot {
    let _scope = SeasonScope::enter(Some(start.season.clone()));
    let close = |a: f64, b: f64| (a - b).abs() <= b.abs().max(1.) * 1e-9;
    let mut snapshot = start.clone();
    assert!(close(
        combined_dps(&snapshot.planner_input()),
        result.base_dps
    ));
    let mut dps = result.base_dps;
    for step in &result.steps {
        assert_eq!(step.dps_before, dps);
        assert!(
            step.dps_after > step.dps_before,
            "{} gains nothing",
            step.slot
        );
        step.apply(&mut snapshot).unwrap();
        let replayed = combined_dps(&snapshot.planner_input());
        assert!(
            close(replayed, step.dps_after),
            "{}: {replayed} vs {}",
            step.slot,
            step.dps_after
        );
        let relics: Vec<&str> = snapshot
            .inventory
            .iter()
            .filter(|(slot, _)| gear::slot_group(slot) == "relic")
            .map(|(_, item)| item.base_id.as_str())
            .collect();
        let unique: HashSet<&str> = relics.iter().copied().collect();
        assert_eq!(relics.len(), unique.len(), "a relic is doubled");
        dps = step.dps_after;
    }
    assert_eq!(dps, result.final_dps);
    snapshot
}

#[test]
fn full_search_steps_replay_to_their_reported_dps() {
    let start = starter();
    let started = std::time::Instant::now();
    let result = suggest(
        &start,
        SuggestOptions::default(),
        &Cancellation::default(),
        |_, _| {},
    )
    .unwrap();
    eprintln!(
        "gear search: {:?}, {} calculations, {} steps, DPS {} -> {}",
        started.elapsed(),
        result.evaluations,
        result.steps.len(),
        result.base_dps,
        result.final_dps
    );
    for step in &result.steps {
        eprintln!(
            "  pass {} {} {:.0} -> {:.0}: {:?}",
            step.pass,
            step.slot,
            step.dps_before,
            step.dps_after,
            step.changes()
        );
    }
    assert!(result.final_dps > result.base_dps);
    assert_replays(&start, &result);
    let crystals = result.steps.iter().flat_map(|s| &s.item.forged_mods);
    for forged in crystals.filter(|m| m.affix_id != ADD_SOCKET) {
        let crystal = data::get_crystal_mod(&forged.affix_id).unwrap();
        assert_eq!(forged.custom_value, Some(rolled_affix_value(crystal, 1.)));
    }
}

#[test]
fn a_later_pass_offers_the_players_own_item_not_a_fresh_copy() {
    let mut start = starter();
    let belt = "belt_satanic_cingulum_of_shifting_destiny";
    let mut own = gear::make_item(belt).unwrap();
    gear::set_forge(&mut own, Some("crystal_satanic_to_strength")).unwrap();
    equip(&mut start, "belt", &own, false).unwrap();
    let cancellation = Cancellation::default();
    let search = Search {
        start: &start,
        options: only(|o| o.other_items = true),
        cancellation: &cancellation,
        evaluations: AtomicU32::new(0),
    };
    let _scope = search.scope();
    let mut later = start.clone();
    let other = data::data()
        .items
        .values()
        .filter(|b| b.id != belt && gear::accepts(&later, "belt", b, false))
        .map(|b| b.id.clone())
        .min()
        .unwrap();
    equip(&mut later, "belt", &gear::make_item(&other).unwrap(), false).unwrap();
    let candidates = search.candidates(&later, "belt", &Scores::default());
    let copies: Vec<&EquippedItem> = candidates.iter().filter(|i| i.base_id == belt).collect();
    assert_eq!(copies.len(), 1);
    assert_eq!(copies[0].forged_mods.len(), 1, "the player's crystal stays");
}

#[test]
fn random_affixes_fill_only_the_items_unholy_pool() {
    let mut start = starter();
    let boots = "boots_heroic_soulflaming_marchers";
    equip(&mut start, "boots", &gear::make_item(boots).unwrap(), false).unwrap();
    let result = suggest(
        &start,
        only(|o| o.affixes = true),
        &Cancellation::default(),
        |_, _| {},
    )
    .unwrap();
    let base = data::get_item(boots).unwrap();
    let step = result
        .steps
        .iter()
        .find(|s| s.slot == "boots")
        .expect("affixes raise DPS");
    assert!(step.item.affixes.len() <= base.max_affixes.unwrap() as usize);
    assert!(
        step.item
            .affixes
            .iter()
            .all(|a| { gear::affix_allowed(base, data::get_affix(&a.affix_id).unwrap(), false) })
    );
    assert!(result.steps.iter().all(|s| s.slot == "boots"));
    assert_replays(&start, &result);
}

#[test]
fn sockets_fill_the_current_weapon() {
    let start = starter();
    let result = suggest(
        &start,
        only(|o| o.sockets = true),
        &Cancellation::default(),
        |_, _| {},
    )
    .unwrap();
    let step = result
        .steps
        .iter()
        .find(|s| s.slot == "weapon")
        .expect("sockets raise DPS");
    assert_eq!(step.item.base_id, start.inventory["weapon"].base_id);
    assert!(step.item.socketed.iter().any(Option::is_some));
    assert_replays(&start, &result);
}

#[test]
fn nothing_changes_with_every_kind_of_change_off() {
    let result = suggest(
        &starter(),
        only(|_| {}),
        &Cancellation::default(),
        |_, _| {},
    )
    .unwrap();
    assert!(result.steps.is_empty());
    assert_eq!(result.final_dps, result.base_dps);
}

#[test]
fn a_cancelled_search_reports_an_error() {
    let cancellation = Cancellation::default();
    cancellation.cancel();
    assert!(
        suggest(
            &starter(),
            SuggestOptions::default(),
            &cancellation,
            |_, _| {}
        )
        .is_err()
    );
}

#[test]
fn a_new_augment_switches_on_where_the_calculation_reads_it() {
    let first = |calc: AugmentCalc| {
        let ids = data::data().augments.values().filter(|a| a.calc == calc);
        ids.map(|a| a.id.clone()).min().unwrap()
    };
    let armor = data::data().items.values().filter(|b| b.slot == "armor");
    let mut item = gear::make_item(&armor.map(|b| b.id.clone()).min().unwrap()).unwrap();
    let mut snapshot = BuildSnapshot::default();
    let toggle = first(AugmentCalc::Toggle);
    gear::set_augment(&mut item, Some(&toggle)).unwrap();
    equip(&mut snapshot, "armor", &item, true).unwrap();
    assert_eq!(
        snapshot.player_conditions.get(&augment_toggle_key(&toggle)),
        Some(&true)
    );
    let proc = first(AugmentCalc::Proc);
    gear::set_augment(&mut item, Some(&proc)).unwrap();
    equip(&mut snapshot, "armor", &item, false).unwrap();
    assert_eq!(snapshot.proc_toggles.get(&augment_toggle_key(&proc)), None);
    equip(&mut snapshot, "armor", &item, true).unwrap();
    assert_eq!(
        snapshot.proc_toggles.get(&augment_toggle_key(&proc)),
        Some(&true)
    );
}

#[test]
fn changes_name_a_new_base_its_stars_and_a_completed_runeword() {
    let mut commons: Vec<&ItemBase> = data::data()
        .items
        .values()
        .filter(|b| b.rarity == "common")
        .collect();
    commons.sort_by(|a, b| a.id.cmp(&b.id));
    let (base, mut item, word) = data::data()
        .runewords
        .iter()
        .find_map(|rw| {
            commons.iter().find_map(|base| {
                let mut item = gear::make_item(&base.id).ok()?;
                gear::apply_runeword(&mut item, &rw.id).ok()?;
                Some((*base, item, rw.id.clone()))
            })
        })
        .unwrap();
    item.stars = Some(3);
    assert_eq!(
        step(&base.slot, None, item).changes(),
        vec![
            GearChange::Item {
                from: None,
                to: base.id.clone()
            },
            GearChange::Stars { from: 0, to: 3 },
            GearChange::Runeword(word),
        ]
    );
}

#[test]
fn changes_list_crystal_socket_and_augment_edits_on_the_same_item() {
    let base = data::data()
        .items
        .values()
        .filter(|b| b.slot == "armor" && data::forge_kind_for(&b.rarity).is_some())
        .filter(|b| gear::max_sockets(&gear::make_item(&b.id).unwrap()) > 0)
        .min_by(|a, b| a.id.cmp(&b.id))
        .unwrap();
    let mut before = gear::make_item(&base.id).unwrap();
    gear::set_socket_count(&mut before, 1);
    let gem = data::data().gems.keys().min().unwrap().clone();
    let crystal = data::data()
        .crystals
        .keys()
        .filter(|id| *id != ADD_SOCKET)
        .min();
    let crystal = crystal.unwrap().clone();
    let augment = data::data().augments.keys().min().unwrap().clone();
    let mut item = before.clone();
    gear::set_socket(&mut item, 0, Some(&gem)).unwrap();
    gear::set_forge(&mut item, Some(&crystal)).unwrap();
    gear::set_augment(&mut item, Some(&augment)).unwrap();
    item.augment.as_mut().unwrap().level = 7;
    assert_eq!(
        step("armor", Some(before), item).changes(),
        vec![
            GearChange::Crystal {
                from: None,
                to: Some(crystal)
            },
            GearChange::Socket {
                index: 0,
                from: None,
                to: Some(gem)
            },
            GearChange::Augment {
                from: None,
                to: Some((augment, 7))
            },
        ]
    );
}

// The dialog's path: "Apply up to here" then "Apply all" through the session, one undo each.
#[test]
fn applying_through_the_session_matches_each_prefix_and_undoes_per_apply() {
    let mut state = crate::session::WorkspaceState::default();
    state.settings.auto_save = false;
    let mut session = Session::new(state);
    session.new_build("Build").unwrap();
    let start = starter();
    session.edit(|draft| draft.snapshot = start.clone());
    let result = suggest(
        session.snapshot(),
        SuggestOptions::default(),
        &Cancellation::default(),
        |_, _| {},
    )
    .unwrap();
    assert!(result.steps.len() >= 3);
    let dps = |session: &Session| {
        let _scope = SeasonScope::enter(Some(session.snapshot().season.clone()));
        combined_dps(&session.snapshot().planner_input())
    };
    let close = |a: f64, b: f64| (a - b).abs() <= b.abs().max(1.) * 1e-9;
    let middle = result.steps.len() / 2;
    apply_steps(&mut session, &result.steps[..middle]).unwrap();
    assert!(close(dps(&session), result.steps[middle - 1].dps_after));
    apply_steps(&mut session, &result.steps[middle..]).unwrap();
    assert!(close(dps(&session), result.final_dps));
    session.undo();
    assert!(close(dps(&session), result.steps[middle - 1].dps_after));
    session.undo();
    assert!(close(dps(&session), result.base_dps));
}
