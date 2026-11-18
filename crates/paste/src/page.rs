//! The pages: the paste form, a shared build, and plain messages.
use std::{
    fmt::Write,
    panic::{self, AssertUnwindSafe},
};

use hsplanner_build::{
    BuildSnapshot, codec,
    loadout::{LoadoutKind, Loadouts},
};
use hsplanner_engine::calc::{
    data,
    defense::{self, EhpEntry},
    planner::{self, PlannerPerformance},
    skills::Ranged,
};

use crate::html::BUILD_PATH;
use crate::{
    assets, equipment,
    html::{self, Chrome, Meta, VERSION, esc},
    numbers::{compact, ranged},
    panels::{self, active},
};

/// Query parameter of each loadout kind, in the order their choices are stored.
const LOADOUT_PARAMS: [(&str, LoadoutKind); 4] = [
    ("gear", LoadoutKind::Gear),
    ("skills", LoadoutKind::Skills),
    ("tree", LoadoutKind::Incarnation),
    ("ether", LoadoutKind::Ether),
];
const EMPTY_LOADOUT: &str =
    "<section class=\"panel\"><p class=\"pad muted\">This loadout is empty.</p></section>";

// Each extra active skill costs a full damage pass, so shared builds get a fixed budget.
const MAX_ACTIVE_SKILLS: usize = 6;

const RESISTANCES: [&str; 5] = ["fire", "cold", "lightning", "poison", "arcane"];
/// Chat apps cut a link preview at about this many characters.
const MAX_PREVIEW: usize = 340;
const PREVIEW_SKILLS: usize = 3;

/// Which loadout of each kind to show, by position; `None` keeps the one the build was shared with.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Selection([Option<usize>; 4]);

impl Selection {
    /// Reads `gear=1&tree=0`. Unrelated parameters are ignored; a malformed choice is `None`.
    pub fn parse(query: &str) -> Option<Self> {
        let mut chosen = [None; 4];
        for (name, value) in query.split('&').filter_map(|pair| pair.split_once('=')) {
            if let Some(slot) = LOADOUT_PARAMS.iter().position(|(param, _)| *param == name) {
                chosen[slot] = Some(value.parse().ok()?);
            }
        }
        Some(Self(chosen))
    }

    pub fn is_default(self) -> bool {
        self == Self::default()
    }
}

/// Why a build page could not be drawn.
pub enum Failure {
    Unreadable(String),
    NoSuchLoadout,
}

/// The picker of one kind's loadouts; it reloads the page with the choice in the address.
fn loadout_form(
    id: &str,
    tab: &str,
    kind: LoadoutKind,
    loadouts: &Loadouts,
    selection: Selection,
) -> String {
    if loadouts.count(kind) < 2 {
        return String::new();
    }
    let shown = loadouts.active_id(kind);
    let mut options = String::new();
    for (index, (entry, name)) in loadouts.entries(kind).into_iter().enumerate() {
        let selected = if entry == shown { " selected" } else { "" };
        let _ = write!(
            options,
            "<option value=\"{index}\"{selected}>{}</option>",
            esc(name)
        );
    }
    let (mut own, mut others) = ("", String::new());
    for ((param, other), choice) in LOADOUT_PARAMS.iter().zip(selection.0) {
        if *other == kind {
            own = param;
        } else if let Some(choice) = choice {
            let _ = write!(
                others,
                "<input type=\"hidden\" name=\"{param}\" value=\"{choice}\">"
            );
        }
    }
    format!(
        "<form class=\"loadout\" method=\"get\" action=\"{BUILD_PATH}{}#{tab}\"><label>\
<span class=\"eyebrow\">Loadout</span><select name=\"{own}\" data-submit>{options}</select></label>\
{others}<button class=\"btn no-js\" type=\"submit\">Show</button></form>",
        esc(id)
    )
}

pub fn home(error: Option<&str>, draft: &str) -> String {
    let error = error.map_or_else(String::new, |error| {
        format!("<p class=\"error\" role=\"alert\">{}</p>", esc(error))
    });
    let form = format!(
        "<form class=\"paste-form\" method=\"post\" action=\"/\"><label for=\"code\">Build code</label>\
<textarea id=\"code\" name=\"code\" rows=\"8\" required spellcheck=\"false\" autocomplete=\"off\">{}</textarea>\
<button class=\"btn gold large\" type=\"submit\">Create link</button></form>",
        esc(draft)
    );
    let body = format!(
        "<main class=\"landing\"><span class=\"brand-mark\">HSPlanner</span>\
<h1>Share a Hero Siege build as a link</h1>\
<p class=\"lede\">Paste a build code from HSPlanner. Anyone with the link sees the gear, skills, \
trees and damage, and can copy the code back into the planner.</p>{error}{}</main>",
        panels::panel("New link", "", &form)
    );
    let meta = Meta {
        title: "HSPlanner builds",
        description: "Paste an HSPlanner build code and get a short link that shows the gear, skills, trees and damage.",
        image: None,
        site: html::SITE_NAME,
    };
    let chrome = Chrome {
        paste_id: None,
        footnote: &format!("v{VERSION}"),
    };
    html::document(&meta, &chrome, &body)
}

pub fn message(title: &str, detail: &str) -> String {
    let body = format!(
        "<main class=\"landing\"><span class=\"brand-mark\">HSPlanner</span><h1>{}</h1>\
<p class=\"lede\">{}</p><a class=\"btn gold large\" href=\"/\">Share a build</a></main>",
        esc(title),
        esc(detail)
    );
    let meta = Meta {
        title,
        description: detail,
        image: None,
        site: html::SITE_NAME,
    };
    let chrome = Chrome {
        paste_id: None,
        footnote: &format!("v{VERSION}"),
    };
    html::document(&meta, &chrome, &body)
}

struct Tab {
    id: &'static str,
    label: &'static str,
    /// The loadout kind the tab shows, when it has one.
    kind: Option<LoadoutKind>,
    meta: String,
    body: String,
}

fn tabs(
    snapshot: &BuildSnapshot,
    performance: Option<&PlannerPerformance>,
    loadouts: &Loadouts,
) -> Vec<Tab> {
    let mut tabs = Vec::new();
    // A kind with several loadouts keeps its tab even when the shown one is empty.
    let mut add = |id, label, kind: Option<LoadoutKind>, content: Option<(String, String)>| {
        let several = kind.is_some_and(|kind| loadouts.count(kind) > 1);
        let (meta, body) = match content {
            Some(content) => content,
            None if several => (String::new(), EMPTY_LOADOUT.to_owned()),
            None => return,
        };
        tabs.push(Tab {
            id,
            label,
            kind,
            meta,
            body,
        });
    };
    let (gear, items) = equipment::board(snapshot);
    let gear = (items > 0).then(|| (format!("{items} items"), gear));
    add("gear", "Gear", Some(LoadoutKind::Gear), gear);
    let skills = panels::skills(snapshot).map(|(skills, points)| (format!("{points} pts"), skills));
    add("skills", "Skills", Some(LoadoutKind::Skills), skills);
    let nodes = |(tree, nodes): (String, usize)| (format!("{nodes} nodes"), tree);
    let incarnation = panels::incarnation(snapshot).map(nodes);
    add(
        "tree",
        "Incarnation tree",
        Some(LoadoutKind::Incarnation),
        incarnation,
    );
    let ether = panels::ether(snapshot, performance).map(nodes);
    add("ether", "Ether tree", Some(LoadoutKind::Ether), ether);
    if let Some(body) = panels::mercenary(snapshot) {
        tabs.push(Tab {
            id: "mercenary",
            label: "Mercenary",
            // The mercenary's equipment belongs to the gear loadout, as in the planner.
            kind: Some(LoadoutKind::Gear),
            meta: String::new(),
            body,
        });
    }
    tabs
}

fn tab_buttons(tabs: &[Tab]) -> String {
    let mut buttons = String::new();
    for (index, tab) in tabs.iter().enumerate() {
        let meta = if tab.meta.is_empty() {
            String::new()
        } else {
            format!("<span class=\"tab-meta\">{}</span>", esc(&tab.meta))
        };
        let _ = write!(
            buttons,
            "<button type=\"button\" role=\"tab\" class=\"tab{}\" aria-selected=\"{}\" \
data-tab=\"{}\">{}{meta}</button>",
            active(index),
            index == 0,
            tab.id,
            tab.label
        );
    }
    buttons
}

/// The band under the top bar: who the build is, its headline damage, and the section tabs.
fn hero(
    title: &str,
    snapshot: &BuildSnapshot,
    performance: Option<&PlannerPerformance>,
    tabs: &[Tab],
) -> String {
    let class_id = snapshot.class_id.as_deref().unwrap_or_default();
    let portrait = format!("classes/{class_id}.webp");
    let portrait = if assets::game(&portrait).is_some() {
        format!(
            "<span class=\"icon-box hero-icon\"><img class=\"pixel\" src=\"/assets/{}\" \
alt=\"\" width=\"42\" height=\"42\"></span>",
            esc(&portrait)
        )
    } else {
        String::new()
    };
    let skills = data::get_skills_by_class(class_id);
    let mut chips = String::new();
    for (index, id) in snapshot.active_skill_ids.iter().enumerate() {
        if let Some(skill) = skills.iter().find(|skill| skill.id == *id) {
            let gold = if index == 0 { " gold" } else { "" };
            let _ = write!(
                chips,
                "<span class=\"chip{gold}\">{}</span>",
                esc(&skill.name)
            );
        }
    }
    let difficulty = data::get_difficulty(Some(&snapshot.difficulty));
    let facts: Vec<String> = difficulty
        .map(|difficulty| difficulty.name.clone())
        .into_iter()
        .chain(tabs.iter().map(|tab| tab.meta.clone()))
        .filter(|fact| !fact.is_empty())
        .collect();
    let dps = performance
        .and_then(|performance| {
            let current = &performance.current;
            current.combined_dps_min.zip(current.combined_dps_max)
        })
        .map_or_else(String::new, |dps| {
            format!(
                "<div class=\"hero-dps\"><span class=\"eyebrow\">Combined DPS</span>\
<div class=\"num dps\">{}</div></div>",
                esc(&ranged(dps, false))
            )
        });
    format!(
        "<div class=\"hero\"><div class=\"wrap hero-inner\">{portrait}<div class=\"hero-text\">\
<div class=\"hero-title\"><h1>{}</h1>{chips}</div><p class=\"hero-meta\">{}</p></div>{dps}</div>\
<div class=\"wrap\"><div class=\"tabs needs-js\" role=\"tablist\">{}</div></div></div>",
        esc(title),
        esc(&facts.join(" · ")),
        tab_buttons(tabs)
    )
}

/// The one-line summary used for link previews and search results.
/// A stat the way people write it in chat: thousands grouped, the short form from a
/// hundred million up.
pub fn grouped(value: f64) -> String {
    let size = value.abs();
    if !value.is_finite() || !(1e3..1e8).contains(&size) {
        return compact(value);
    }
    let digits = format!("{size:.0}");
    let mut text = String::from(if value < 0. { "-" } else { "" });
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

fn grouped_range((low, high): Ranged) -> String {
    let (low, high) = (grouped(low), grouped(high));
    if low == high {
        low
    } else {
        format!("{low}–{high}")
    }
}

/// A resistance as the game applies it, with the uncapped total when the cap cuts it.
pub fn resistance(value: f64, cap: Option<f64>) -> String {
    match cap {
        Some(cap) if value > cap => format!("{}% ({}%)", compact(cap), compact(value)),
        _ => format!("{}%", compact(value)),
    }
}

/// The link preview: damage, defence, the five resistances and the main skills,
/// one `➤` line each, dropping whole lines that would not fit.
fn description(snapshot: &BuildSnapshot, performance: Option<&PlannerPerformance>) -> String {
    let mut lines = Vec::new();
    if let Some(performance) = performance {
        let current = &performance.current;
        let combined = current.combined_dps_min.zip(current.combined_dps_max);
        let mut damage = vec![format!(
            "DPS: {}",
            combined.map_or_else(|| "0".to_owned(), grouped_range)
        )];
        let hit = current.hit_dps_min.zip(current.hit_dps_max);
        if let Some(hit) = hit.filter(|hit| *hit != (0., 0.) && Some(*hit) != combined) {
            damage.push(format!("Hit DPS: {}", grouped_range(hit)));
        }
        lines.push(damage.join(" · "));

        let effective = |entry: &EhpEntry| entry.ehp.map_or_else(|| "immune".to_owned(), grouped);
        let (physical, elemental): (Vec<_>, Vec<_>) = current
            .ehp
            .entries
            .iter()
            .partition(|entry| entry.damage_type == "physical");
        let weakest = elemental.into_iter().min_by(|a, b| {
            let (a, b) = (
                a.ehp.unwrap_or(f64::INFINITY),
                b.ehp.unwrap_or(f64::INFINITY),
            );
            a.total_cmp(&b)
        });
        let defence: Vec<String> = [
            current
                .stats
                .get("life")
                .map(|life| format!("Life: {}", grouped_range(*life))),
            physical
                .first()
                .map(|entry| format!("Physical eHP: {}", effective(entry))),
            weakest.map(|entry| format!("Elemental eHP: {}", effective(entry))),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !defence.is_empty() {
            lines.push(defence.join(" · "));
        }

        let resistances = RESISTANCES.map(|kind| {
            let key = format!("{kind}_resistance");
            let value = current.stats.get(&key).map_or(0., |value| value.1);
            resistance(value, defense::effective_cap(&key, &current.stats))
        });
        lines.push(format!("Res: {}", resistances.join(" / ")));
    }
    let skills = data::get_skills_by_class(snapshot.class_id.as_deref().unwrap_or_default());
    let names: Vec<&str> = snapshot
        .active_skill_ids
        .iter()
        .filter_map(|id| skills.iter().find(|skill| skill.id == *id))
        .map(|skill| skill.name.as_str())
        .take(PREVIEW_SKILLS)
        .collect();
    if !names.is_empty() {
        lines.push(names.join(" · "));
    }
    let mut text = String::new();
    for line in lines {
        if text.chars().count() + line.chars().count() + 3 > MAX_PREVIEW {
            break;
        }
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str("➤ ");
        text.push_str(&line);
    }
    text
}

pub fn build(
    id: &str,
    code: &str,
    public_url: &str,
    selection: Selection,
) -> Result<String, Failure> {
    let shared = codec::decode_shared(code).map_err(Failure::Unreadable)?;
    let (mut snapshot, notes, mut loadouts) = (shared.snapshot, shared.notes, shared.loadouts);
    for ((_, kind), choice) in LOADOUT_PARAMS.iter().zip(selection.0) {
        let Some(choice) = choice else { continue };
        let entries = loadouts.entries(*kind);
        let (entry, _) = entries.get(choice).ok_or(Failure::NoSuchLoadout)?;
        let entry = (*entry).to_owned();
        loadouts
            .select(*kind, &entry)
            .map_err(Failure::Unreadable)?;
    }
    loadouts.apply_active(&mut snapshot);
    snapshot.active_skill_ids.truncate(MAX_ACTIVE_SKILLS);
    // A build the engine cannot evaluate still shows its gear, skills and trees.
    let performance = panic::catch_unwind(AssertUnwindSafe(|| {
        planner::evaluate(&snapshot.planner_input())
    }))
    .ok();
    let performance = performance.as_ref();
    let class = snapshot.class_id.as_deref().and_then(data::get_class);
    let title = format!(
        "Level {} {}",
        snapshot.level,
        class.map_or("build", |class| class.name.as_str())
    );
    let tabs = tabs(&snapshot, performance, &loadouts);
    let mut body = hero(&title, &snapshot, performance, &tabs);
    let _ = write!(
        body,
        "<div class=\"wrap layout\"><aside class=\"sidebar\">{}</aside><main class=\"main\">",
        panels::sidebar(performance, &snapshot)
    );
    for (index, tab) in tabs.iter().enumerate() {
        let _ = write!(
            body,
            "<div class=\"tab-panel{}\" data-panel=\"{}\">{}{}</div>",
            active(index),
            tab.id,
            tab.kind.map_or_else(String::new, |kind| loadout_form(
                id, tab.id, kind, &loadouts, selection
            )),
            tab.body
        );
    }
    body.push_str(&panels::notes(&notes.to_html()));
    body.push_str(&panels::code(code));
    body.push_str("</main></div>");

    let image = class
        .map(|class| format!("classes/{}.webp", class.id))
        .filter(|path| !public_url.is_empty() && assets::game(path).is_some())
        .map(|path| format!("{public_url}/assets/{path}"));
    let headline = format!(
        "Lv {} {}",
        snapshot.level,
        class.map_or("build", |class| class.name.as_str())
    );
    let host = public_url.split("://").last().unwrap_or_default();
    let meta = Meta {
        title: &shared
            .name
            .map_or(headline.clone(), |name| format!("{name} — {headline}")),
        description: &description(&snapshot, performance),
        image: image.as_deref(),
        site: &if host.is_empty() {
            html::SITE_NAME.to_owned()
        } else {
            format!("{} — {host}", html::SITE_NAME)
        },
    };
    // Share codes are ASCII, so slicing by bytes is safe.
    let short_code = if code.len() > 24 && code.is_ascii() {
        format!("{}…{}", &code[..12], &code[code.len() - 6..])
    } else {
        String::new()
    };
    let chrome = Chrome {
        paste_id: Some(id),
        footnote: &format!("{short_code} · v{VERSION}"),
    };
    Ok(html::document(&meta, &chrome, &body))
}
