use std::net::{IpAddr, Ipv4Addr};

use hsplanner_build::{BuildSnapshot, codec, loadout::Loadouts, notes::Notes};
use hsplanner_engine::calc::{mercenary, types::CustomStat};
use serde_json::{Value, json};

use super::*;
use crate::store::Store;

const ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

const ATTACK: &str = "<script>alert(1)</script>\"'><img src=x onerror=alert(1)>";

fn fixture_code() -> String {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../../../engine/tests/fixtures/native-share-codes.json"
    ))
    .unwrap();
    fixtures[0]["code"].as_str().unwrap().to_owned()
}

fn app() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let app = App::new(Store::open(dir.path(), u64::MAX).unwrap(), String::new());
    (app, dir)
}

fn send_from(app: &App, address: IpAddr, method: &str, path: &str, body: &[u8]) -> Response {
    app.handle(&Request {
        method,
        path,
        body,
        address,
    })
}

fn send(app: &App, method: &str, path: &str, body: &[u8]) -> Response {
    send_from(app, ADDRESS, method, path, body)
}

fn text(response: &Response) -> String {
    String::from_utf8_lossy(&response.body).into_owned()
}

fn share(app: &App, code: &str) -> String {
    let response = send(app, "POST", "/api/pastes", code.as_bytes());
    assert_eq!(response.status, 201, "{}", text(&response));
    let reply: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(reply["success"], true);
    reply["data"]["id"].as_str().unwrap().to_owned()
}

#[test]
fn pasted_form_code_redirects_to_a_page_that_shows_the_build() {
    let (app, _dir) = app();
    let code = fixture_code();
    let form = format!("code={}", code.replace('+', "%2B"));

    let created = send(&app, "POST", "/", form.as_bytes());

    assert_eq!(created.status, 303, "{}", text(&created));
    let location = created.location.clone().unwrap();
    let page = send(&app, "GET", &location, b"");
    assert_eq!(page.status, 200);
    let html = text(&page);
    for expected in ["Level 60 Stormweaver", "Charged Bolts", "Ogre Maul"] {
        assert!(html.contains(expected), "page is missing {expected}");
    }
    let raw = send(&app, "GET", &format!("{location}/raw"), b"");
    assert_eq!(text(&raw), code);
}

#[test]
fn api_accepts_a_share_link_and_stores_only_the_code() {
    let (app, _dir) = app();
    let code = fixture_code();

    let id = share(&app, &format!("https://example.com/#b={code}"));

    let raw = send(&app, "GET", &format!("/b/{id}/raw"), b"");
    assert_eq!(text(&raw), code);
}

#[test]
fn invalid_codes_are_rejected_and_nothing_is_stored() {
    let (app, dir) = app();

    let api = send(&app, "POST", "/api/pastes", b"not a build");
    let form = send(&app, "POST", "/", b"code=not+a+build");

    assert_eq!(api.status, 400);
    let reply: Value = serde_json::from_slice(&api.body).unwrap();
    assert_eq!(reply["success"], false);
    assert!(
        reply["error"]
            .as_str()
            .is_some_and(|error| !error.is_empty())
    );
    assert_eq!(form.status, 400);
    assert!(text(&form).contains("<form"));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn oversized_bodies_are_refused() {
    let (app, _dir) = app();
    let body = vec![b'A'; MAX_BODY + 1];

    assert_eq!(send(&app, "POST", "/api/pastes", &body).status, 413);
}

#[test]
fn unknown_ids_and_traversal_paths_are_not_found() {
    let (app, _dir) = app();

    for path in [
        "/aaaaaaaaaa",
        "/b/aaaaaaaaaa",
        "/b/short",
        "/b/",
        "/b",
        "/../../etc/passwd",
        "/b/../../etc/passwd",
        "/b/aaaaaaaaaa/other",
        "/assets/../Cargo.toml",
        "/static/nope.css",
    ] {
        assert_eq!(send(&app, "GET", path, b"").status, 404, "{path}");
    }
    assert_eq!(send(&app, "POST", "/aaaaaaaaaa", b"").status, 404);
    assert_eq!(send(&app, "DELETE", "/", b"").status, 405);
}

#[test]
fn sharing_is_limited_per_address() {
    let (app, _dir) = app();
    let code = fixture_code();
    for _ in 0..SHARES_PER_ADDRESS {
        share(&app, &code);
    }

    let limited = send(&app, "POST", "/api/pastes", code.as_bytes());
    let other = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
    let allowed = send_from(&app, other, "POST", "/api/pastes", code.as_bytes());

    assert_eq!(limited.status, 429);
    assert_eq!(allowed.status, 201);
}

#[test]
fn hostile_build_text_never_reaches_the_page_as_markup() {
    let (app, _dir) = app();
    let mut snapshot = BuildSnapshot {
        class_id: Some(ATTACK.into()),
        active_skill_ids: vec![ATTACK.into()],
        active_aura_id: Some(ATTACK.into()),
        merc_class_id: Some(ATTACK.into()),
        ..Default::default()
    };
    snapshot.skill_ranks.insert(ATTACK.into(), 3);
    snapshot
        .subskill_ranks
        .insert(format!("{ATTACK}:{ATTACK}"), 3);
    snapshot.custom_stats.push(CustomStat {
        stat_key: ATTACK.into(),
        value: ATTACK.into(),
    });
    for inventory in [&mut snapshot.inventory, &mut snapshot.merc_inventory] {
        inventory.insert(
            "weapon".into(),
            serde_json::from_value(json!({ "baseId": ATTACK })).unwrap(),
        );
    }
    let notes = Notes {
        markdown: ATTACK.into(),
        original_html: None,
    };
    let id = share(&app, &codec::encode(&snapshot, &notes).unwrap());

    let page = send(&app, "GET", &format!("/b/{id}"), b"");

    assert_eq!(page.status, 200);
    let html = text(&page);
    for markup in ["<script>alert", "<img src=x", "onerror=alert(1)>"] {
        assert!(!html.contains(markup), "page contains {markup}");
    }
}

#[test]
fn embedded_files_are_served_with_their_types() {
    let (app, _dir) = app();

    for (path, content_type) in [
        ("/static/style.css?v=1", "text/css; charset=utf-8"),
        ("/static/app.js", "text/javascript; charset=utf-8"),
        ("/static/tree-incarnation.svg", "image/svg+xml"),
        ("/assets/classes/stormweaver.webp", "image/webp"),
        ("/assets/items/base_mace_ogre_maul.png", "image/png"),
    ] {
        let response = send(&app, "GET", path, b"");
        assert_eq!(response.status, 200, "{path}");
        assert_eq!(response.content_type, content_type, "{path}");
        assert!(!response.body.is_empty(), "{path}");
    }
}

#[test]
fn home_offers_the_paste_form() {
    let (app, _dir) = app();

    let home = send(&app, "GET", "/", b"");

    assert_eq!(home.status, 200);
    assert!(text(&home).contains("name=\"code\""));
}

#[test]
fn failed_attempts_run_out_before_they_can_be_repeated_freely() {
    let (app, _dir) = app();
    for _ in 0..ATTEMPTS_PER_ADDRESS {
        assert_eq!(send(&app, "POST", "/api/pastes", b"junk").status, 400);
    }

    let limited = send(&app, "POST", "/api/pastes", fixture_code().as_bytes());

    assert_eq!(limited.status, 429);
}

#[test]
fn addresses_in_one_ipv6_network_share_a_budget() {
    let first: IpAddr = "2001:db8:1:2::1".parse().unwrap();
    let neighbour: IpAddr = "2001:db8:1:2:ffff::9".parse().unwrap();
    let elsewhere: IpAddr = "2001:db8:1:3::1".parse().unwrap();

    assert_eq!(limit_key(first), limit_key(neighbour));
    assert_ne!(limit_key(first), limit_key(elsewhere));
    assert_eq!(limit_key(ADDRESS), ADDRESS);
}

#[test]
fn a_full_store_refuses_new_builds() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::new(Store::open(dir.path(), 0).unwrap(), String::new());

    let refused = send(&app, "POST", "/api/pastes", fixture_code().as_bytes());

    assert_eq!(refused.status, 507);
}

#[test]
fn builds_holding_more_than_a_character_can_are_refused() {
    let (app, _dir) = app();
    let mut crowded = BuildSnapshot::default();
    for slot in 0..500 {
        crowded.inventory.insert(
            format!("slot_{slot}"),
            serde_json::from_value(json!({ "baseId": "base_mace_ogre_maul" })).unwrap(),
        );
    }
    let mut stuffed = BuildSnapshot::default();
    let affixes: Vec<_> = (0..500)
        .map(|_| json!({ "affixId": "strength", "tier": 1, "roll": 1.0 }))
        .collect();
    stuffed.inventory.insert(
        "weapon".into(),
        serde_json::from_value(json!({ "baseId": "base_mace_ogre_maul", "affixes": affixes }))
            .unwrap(),
    );

    for snapshot in [crowded, stuffed] {
        let code = codec::encode(&snapshot, &Notes::default()).unwrap();
        let refused = send(&app, "POST", "/api/pastes", code.as_bytes());
        assert_eq!(refused.status, 422, "{}", text(&refused));
    }
}

/// A default build with extra loadouts cloned from the first; `gear` gives each an inventory.
fn loadout_code(gear: &[Value], skills: usize) -> String {
    loadout_code_for(&BuildSnapshot::default(), gear, skills)
}

fn loadout_code_for(snapshot: &BuildSnapshot, gear: &[Value], skills: usize) -> String {
    let mut loadouts = serde_json::to_value(Loadouts::from_snapshot(snapshot)).unwrap();
    let mut add = |kind: &str, index: usize, inventory: Option<&Value>| {
        let mut entry = loadouts[kind]["entries"][0].clone();
        entry["id"] = json!(format!("extra-{index}"));
        entry["name"] = json!(format!("Bossing <{index}>"));
        if let Some(inventory) = inventory {
            entry["value"]["inventory"] = inventory.clone();
        }
        loadouts[kind]["entries"]
            .as_array_mut()
            .unwrap()
            .push(entry);
    };
    for (index, inventory) in gear.iter().enumerate() {
        add("gear", index, Some(inventory));
    }
    for index in 0..skills {
        add("skills", index, None);
    }
    let loadouts: Loadouts = serde_json::from_value(loadouts).unwrap();
    codec::encode_loadouts(snapshot, &Notes::default(), &loadouts).unwrap()
}

fn maul() -> Value {
    json!({ "weapon": { "baseId": "base_mace_ogre_maul" } })
}

#[test]
fn a_build_with_several_loadouts_offers_them_and_shows_the_chosen_one() {
    let (app, _dir) = app();
    let id = share(&app, &loadout_code(&[maul()], 0));

    let shared = text(&send(&app, "GET", &format!("/b/{id}"), b""));
    let chosen = text(&send(
        &app,
        "GET",
        &format!("/b/{id}?gear=1&from=chat"),
        b"",
    ));

    assert!(shared.contains(&format!("action=\"/b/{id}#gear\"")));
    assert!(shared.contains("<select name=\"gear\" data-submit>"));
    assert!(shared.contains("<option value=\"0\" selected>Default</option>"));
    assert!(shared.contains("<option value=\"1\">Bossing &lt;0&gt;</option>"));
    assert!(!shared.contains("<select name=\"skills\""));
    assert!(!shared.contains("Ogre Maul"));
    assert!(chosen.contains("<option value=\"1\" selected>Bossing &lt;0&gt;</option>"));
    assert!(chosen.contains("Ogre Maul"));
}

#[test]
fn a_chosen_loadout_carries_over_when_another_kind_is_switched() {
    let (app, _dir) = app();
    let id = share(&app, &loadout_code(&[maul()], 1));

    let page = text(&send(&app, "GET", &format!("/b/{id}?gear=1"), b""));

    let skills = page.split("<select name=\"skills\"").nth(1).unwrap();
    let form = skills.split("</form>").next().unwrap();
    assert!(form.contains("<input type=\"hidden\" name=\"gear\" value=\"1\">"));
}

#[test]
fn builds_with_one_loadout_show_no_picker() {
    let (app, _dir) = app();
    let id = share(&app, &fixture_code());

    let page = text(&send(&app, "GET", &format!("/b/{id}"), b""));

    assert!(!page.contains("class=\"loadout\""));
}

#[test]
fn a_loadout_the_build_does_not_have_is_not_found() {
    let (app, _dir) = app();
    let id = share(&app, &loadout_code(&[maul()], 0));

    for query in ["gear=2", "gear=x", "skills=1", "gear=-1"] {
        let missing = send(&app, "GET", &format!("/b/{id}?{query}"), b"");
        assert_eq!(missing.status, 404, "{query}");
    }
}

#[test]
fn an_oversized_loadout_that_is_not_selected_is_still_refused() {
    let (app, _dir) = app();
    let affixes: Vec<_> = (0..500)
        .map(|_| json!({ "affixId": "strength", "tier": 1, "roll": 1.0 }))
        .collect();
    let stuffed = json!({ "weapon": { "baseId": "base_mace_ogre_maul", "affixes": affixes } });

    let refused = send(
        &app,
        "POST",
        "/api/pastes",
        loadout_code(&[stuffed], 0).as_bytes(),
    );

    assert_eq!(refused.status, 422, "{}", text(&refused));
}

#[test]
fn drawing_other_loadouts_is_limited_per_address_but_drawn_pages_stay_readable() {
    let (app, _dir) = app();
    let gear: Vec<Value> = (0..11).map(|_| maul()).collect();
    let id = share(&app, &loadout_code(&gear, 10));
    let visitor = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9));
    let view = |gear: usize, skills: usize| {
        let path = format!("/b/{id}?gear={gear}&skills={skills}");
        send_from(&app, visitor, "GET", &path, b"").status
    };

    let statuses: Vec<u16> = (0..=ATTEMPTS_PER_ADDRESS as usize)
        .map(|nth| view(nth / 11, nth % 11))
        .collect();

    let (allowed, refused) = statuses.split_at(ATTEMPTS_PER_ADDRESS as usize);
    assert!(allowed.iter().all(|status| *status == 200));
    assert_eq!(refused, [429]);
    assert_eq!(view(0, 0), 200);
}

#[test]
fn the_mercenary_tab_switches_gear_loadouts_and_skills_switch_their_own() {
    let (app, _dir) = app();
    let hired = BuildSnapshot {
        merc_class_id: Some(mercenary::data().classes[0].id.clone()),
        ..BuildSnapshot::default()
    };
    let id = share(&app, &loadout_code_for(&hired, &[maul()], 1));

    let page = text(&send(&app, "GET", &format!("/b/{id}"), b""));
    let unhired = share(&app, &loadout_code(&[maul()], 0));
    let without = text(&send(&app, "GET", &format!("/b/{unhired}"), b""));

    let form = page
        .split(&format!("action=\"/b/{id}#mercenary\""))
        .nth(1)
        .unwrap();
    assert!(
        form.starts_with("><label><span class=\"eyebrow\">Loadout</span><select name=\"gear\"")
    );
    assert!(page.contains(&format!("action=\"/b/{id}#skills\"")));
    assert!(page.contains("<select name=\"skills\" data-submit>"));
    assert!(!without.contains("#mercenary"));
}

#[test]
fn builds_live_under_the_b_prefix_only() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path(), u64::MAX).unwrap();
    let app = App::new(store, "https://example.com".into());
    let reply = send(&app, "POST", "/api/pastes", fixture_code().as_bytes());
    let reply: Value = serde_json::from_slice(&reply.body).unwrap();
    let id = reply["data"]["id"].as_str().unwrap();

    assert_eq!(reply["data"]["url"], format!("https://example.com/b/{id}"));
    assert_eq!(send(&app, "GET", &format!("/b/{id}"), b"").status, 200);
    assert_eq!(send(&app, "GET", &format!("/{id}"), b"").status, 404);
    assert_eq!(send(&app, "GET", &format!("/{id}/raw"), b"").status, 404);
    let page = text(&send(&app, "GET", &format!("/b/{id}"), b""));
    assert!(page.contains(&format!("href=\"/b/{id}/raw\"")));
}

fn meta(page: &str, name: &str) -> String {
    let marker = format!("=\"{name}\" content=\"");
    let start = page
        .find(&marker)
        .unwrap_or_else(|| panic!("no {name} tag"))
        + marker.len();
    let end = page[start..].find('"').unwrap() + start;
    page[start..end].to_owned()
}

fn site() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path(), u64::MAX).unwrap();
    (App::new(store, "https://hsplanner.app".into()), dir)
}

#[test]
fn link_preview_lists_damage_defence_five_resistances_and_skills() {
    let (app, _dir) = site();
    let id = share(&app, &fixture_code());

    let page = text(&send(&app, "GET", &format!("/b/{id}"), b""));

    assert_eq!(meta(&page, "og:title"), "Lv 60 Stormweaver");
    assert_eq!(meta(&page, "og:site_name"), "HSPlanner — hsplanner.app");
    assert_eq!(meta(&page, "twitter:card"), "summary");
    assert!(meta(&page, "og:image").ends_with("/assets/classes/stormweaver.webp"));
    let description = meta(&page, "og:description");
    assert_eq!(description, meta(&page, "twitter:description"));
    let lines: Vec<&str> = description.lines().collect();
    assert_eq!(lines.len(), 4, "{description}");
    assert!(lines[0].starts_with("➤ DPS: 32.09"), "{}", lines[0]);
    assert!(
        lines[1].starts_with("➤ Life: 90 · Physical eHP: "),
        "{}",
        lines[1]
    );
    assert!(lines[1].contains(" · Elemental eHP: "), "{}", lines[1]);
    let resistances = lines[2].strip_prefix("➤ Res: ").expect(lines[2]);
    assert_eq!(resistances.split(" / ").count(), 5, "{resistances}");
    assert!(resistances.split(" / ").all(|value| value.contains('%')));
    assert_eq!(lines[3], "➤ Charged Bolts · Lightning Surge");
}

#[test]
fn link_preview_title_starts_with_the_build_name_when_the_code_has_one() {
    let (app, _dir) = site();
    let snapshot = hsplanner_build::BuildSnapshot::default();
    let code = codec::encode_loadouts_with_stash(
        Some("Balanced <Pyro>"),
        &snapshot,
        &hsplanner_build::notes::Notes::default(),
        &hsplanner_build::loadout::Loadouts::from_snapshot(&snapshot),
        &[],
    )
    .unwrap();
    let id = share(&app, &code);

    let page = text(&send(&app, "GET", &format!("/b/{id}"), b""));

    assert_eq!(
        meta(&page, "og:title"),
        "Balanced &lt;Pyro&gt; — Lv 1 Viking"
    );
    assert!(page.contains("<title>Balanced &lt;Pyro&gt; — Lv 1 Viking</title>"));
    assert!(!meta(&page, "og:description").contains("➤ ➤"));
}

#[test]
fn preview_numbers_group_thousands_and_show_capped_resistances() {
    assert_eq!(page::grouped(19672.6), "19,673");
    assert_eq!(page::grouped(950.4), "950");
    assert_eq!(page::grouped(32.09), "32.09");
    assert_eq!(page::grouped(1_234_567.), "1,234,567");
    assert_eq!(page::grouped(165_400_000.), "165M");
    assert_eq!(page::resistance(578., Some(90.)), "90% (578%)");
    assert_eq!(page::resistance(60., Some(75.)), "60%");
    assert_eq!(page::resistance(-20., None), "-20%");
}
