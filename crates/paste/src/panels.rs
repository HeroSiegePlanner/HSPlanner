//! The panels of a build page: stat sidebar, skills, trees, mercenary, notes and code.
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Write,
};

use hsplanner_build::BuildSnapshot;
use hsplanner_engine::calc::{
    data, mercenary,
    planner::PlannerPerformance,
    types::{SkillKind, SkillSpec, StatDef, TreeSocketContent},
};

use crate::{
    assets, gear,
    html::esc,
    numbers::{compact, ranged, readable},
    tree,
};

const OFFENSE_ROWS: [&str; 8] = [
    "enhanced_damage",
    "attacks_per_second",
    "increased_attack_speed",
    "faster_cast_rate",
    "crit_chance",
    "crit_damage",
    "life_steal",
    "mana_steal",
];
const DEFENSE_ROWS: [&str; 9] = [
    "life_replenish",
    "mana_replenish",
    "defense",
    "block_chance",
    "dodge_chance",
    "physical_damage_reduction",
    "magic_damage_reduction",
    "movement_speed",
    "magic_find",
];
const RESISTANCE_ROWS: [&str; 5] = [
    "fire_resistance",
    "cold_resistance",
    "lightning_resistance",
    "poison_resistance",
    "arcane_resistance",
];
/// Substrings of stat keys and the colour channel the planner gives them.
const TONES: [(&str, &str); 8] = [
    ("fire", "tone-orange"),
    ("cold", "tone-cyan"),
    ("lightning", "tone-yellow"),
    ("poison", "tone-green"),
    ("arcane", "tone-purple"),
    ("life", "tone-red"),
    ("mana", "tone-blue"),
    ("magic_find", "tone-gold"),
];

pub fn panel(title: &str, meta: &str, body: &str) -> String {
    let meta = if meta.is_empty() {
        String::new()
    } else {
        format!("<span class=\"panel-meta\">{}</span>", esc(meta))
    };
    format!(
        "<section class=\"panel\"><header class=\"panel-head\"><span class=\"diamond\"></span>\
<h2 class=\"section-label\">{}</h2>{meta}</header>{body}</section>",
        esc(title)
    )
}

fn section_bar(title: &str, count: &str) -> String {
    format!(
        "<div class=\"section-bar\"><span>{}</span><span>{}</span></div>",
        esc(title),
        esc(count)
    )
}

/// A label, its min–max value when the build has one, and the colour class.
type Headline = (&'static str, Option<(f64, f64)>, &'static str);

struct Row {
    label: String,
    value: String,
    tone: &'static str,
}

impl Row {
    fn new(label: impl Into<String>, value: impl Into<String>, tone: &'static str) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            tone,
        }
    }
}

fn tone(key: &str) -> &'static str {
    TONES
        .iter()
        .find(|(needle, _)| key.contains(needle))
        .map_or("", |(_, tone)| tone)
}

fn stat_panel(title: &str, rows: &[Row]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let rows: String = rows
        .iter()
        .map(|row| {
            format!(
                "<div class=\"stat-row\"><span>{}</span><span class=\"num {}\">{}</span></div>",
                esc(&row.label),
                row.tone,
                esc(&row.value)
            )
        })
        .collect();
    panel(
        title,
        "",
        &format!("<div class=\"panel-body\">{rows}</div>"),
    )
}

fn stat_defs() -> HashMap<&'static str, &'static StatDef> {
    data::game_config()
        .stats
        .iter()
        .map(|def| (def.key.as_str(), def))
        .collect()
}

/// The left column: the numbers the planner calculated for this build.
pub fn sidebar(performance: Option<&PlannerPerformance>, snapshot: &BuildSnapshot) -> String {
    let defs = stat_defs();
    let custom: Vec<Row> = snapshot
        .custom_stats
        .iter()
        .map(|stat| {
            let label = defs
                .get(stat.stat_key.as_str())
                .map_or_else(|| stat.stat_key.clone(), |def| def.name.clone());
            Row::new(label, stat.value.clone(), "")
        })
        .collect();
    let custom = stat_panel("Custom stats", &custom);
    let Some(performance) = performance else {
        let rows = [Row::new("Damage and defence", "not available", "")];
        return stat_panel("Summary", &rows) + &custom;
    };
    let current = &performance.current;
    let headline = |rows: [Headline; 2]| -> Vec<Row> {
        rows.into_iter()
            .filter_map(|(label, value, tone)| {
                let value = value.filter(|value| *value != (0., 0.))?;
                Some(Row::new(label, ranged(value, false), tone))
            })
            .collect()
    };
    let stat_rows = |keys: &[&str]| -> Vec<Row> {
        keys.iter()
            .filter_map(|key| {
                let value = current
                    .stats
                    .get(*key)
                    .filter(|value| **value != (0., 0.))?;
                let (label, percent) = defs.get(key).map_or_else(
                    || (readable(key), false),
                    |def| (def.name.clone(), def.format.as_deref() == Some("percent")),
                );
                Some(Row::new(label, ranged(*value, percent), tone(key)))
            })
            .collect()
    };
    let mut offense = headline([
        (
            "Combined DPS",
            current.combined_dps_min.zip(current.combined_dps_max),
            "tone-gold glow",
        ),
        (
            "Hit DPS",
            current.hit_dps_min.zip(current.hit_dps_max),
            "tone-gold",
        ),
    ]);
    offense.extend(stat_rows(&OFFENSE_ROWS));
    let mut defense = headline([
        ("Life", current.stats.get("life").copied(), "tone-red"),
        ("Mana", current.stats.get("mana").copied(), "tone-blue"),
    ]);
    defense.extend(stat_rows(&DEFENSE_ROWS));
    let attributes: Vec<Row> = data::game_config()
        .attributes
        .iter()
        .filter_map(|attribute| {
            let value = performance.computed.attributes.get(&attribute.key)?;
            Some(Row::new(attribute.name.clone(), ranged(*value, false), ""))
        })
        .collect();
    let effective_life: Vec<Row> = current
        .ehp
        .entries
        .iter()
        .map(|entry| {
            let value = entry.ehp.map_or_else(|| "immune".to_owned(), compact);
            Row::new(
                readable(&entry.damage_type),
                value,
                tone(&entry.damage_type),
            )
        })
        .collect();
    [
        stat_panel("Offense", &offense),
        stat_panel("Defense", &defense),
        stat_panel("Resistances", &stat_rows(&RESISTANCE_ROWS)),
        stat_panel("Attributes", &attributes),
        stat_panel("Effective life", &effective_life),
        custom,
    ]
    .concat()
}

/// The class suffix that marks the initially selected row, card or tab.
pub fn active(index: usize) -> &'static str {
    if index == 0 { " active" } else { "" }
}

fn skill_type(skill: &SkillSpec) -> String {
    let kind = match skill.kind {
        SkillKind::Active => "Active",
        SkillKind::Passive => "Passive",
        SkillKind::Aura => "Aura",
        SkillKind::Buff => "Buff",
    };
    match &skill.damage_type {
        Some(damage) => format!("{kind} · {}", readable(damage)),
        None => kind.to_owned(),
    }
}

fn skill_icon(class_id: &str, skill: &SkillSpec, size: u32) -> String {
    let path = format!("skills/{class_id}/{}.png", skill.id);
    if assets::game(&path).is_none() {
        return String::new();
    }
    format!(
        "<img class=\"pixel\" src=\"/assets/{}\" alt=\"\" width=\"{size}\" height=\"{size}\">",
        esc(&path)
    )
}

fn skill_card(snapshot: &BuildSnapshot, skill: &SkillSpec, rank: u32, index: usize) -> String {
    let class_id = snapshot.class_id.as_deref().unwrap_or_default();
    let capped = if rank >= skill.max_rank {
        " capped"
    } else {
        ""
    };
    let mut html = format!(
        "<article class=\"pick-pane tooltip gold{}\" data-pane=\"{index}\">\
<div class=\"tooltip-head has-icon\">\
<p class=\"tooltip-name\">{}</p><p class=\"tooltip-type\">{}</p>\
<span class=\"tooltip-icon icon-box\">{}</span></div>\
<div class=\"tooltip-section\"><div class=\"detail-row\"><span>Rank</span>\
<span class=\"num{capped}\">{rank} / {}</span></div></div>",
        active(index),
        esc(&skill.name),
        esc(&skill_type(skill)),
        skill_icon(class_id, skill, 32),
        skill.max_rank
    );
    let subskills: Vec<(String, u32)> = skill
        .subskills
        .iter()
        .flatten()
        .filter_map(|node| {
            let key = format!("{}:{}", skill.id, node.id);
            let rank = snapshot
                .subskill_ranks
                .get(&key)
                .filter(|rank| **rank > 0)?;
            Some((node.name.clone(), *rank))
        })
        .collect();
    if !subskills.is_empty() {
        let points: u32 = subskills.iter().map(|(_, rank)| rank).sum();
        let _ = write!(
            html,
            "<div class=\"tooltip-section\"><h3><span>Subskills</span>\
<span class=\"trailing\">{points} pts</span></h3>"
        );
        for (name, rank) in subskills {
            let _ = write!(
                html,
                "<div class=\"detail-row\"><span>{}</span><span class=\"num tone-gold\">{rank}</span></div>",
                esc(&name)
            );
        }
        html.push_str("</div>");
    }
    if let Some(description) = skill.description.as_deref().filter(|text| !text.is_empty()) {
        let _ = write!(
            html,
            "<div class=\"tooltip-section tooltip-desc\">{}</div>",
            esc(description)
        );
    }
    html.push_str("</article>");
    html
}

/// The skills the build uses, and how many points they hold.
pub fn skills(snapshot: &BuildSnapshot) -> Option<(String, u32)> {
    let class_id = snapshot.class_id.as_deref().unwrap_or_default();
    let mut used: Vec<(&SkillSpec, u32, usize)> = data::get_skills_by_class(class_id)
        .iter()
        .filter_map(|skill| {
            let rank = snapshot.skill_ranks.get(&skill.id).copied().unwrap_or(0);
            let order = snapshot
                .active_skill_ids
                .iter()
                .position(|id| *id == skill.id);
            let aura = snapshot.active_aura_id.as_deref() == Some(skill.id.as_str());
            (rank > 0 || order.is_some() || aura).then_some((
                skill,
                rank,
                order.unwrap_or(usize::MAX),
            ))
        })
        .collect();
    if used.is_empty() {
        return None;
    }
    used.sort_by_key(|(_, _, order)| *order);
    let points = used.iter().map(|(_, rank, _)| rank).sum();
    let (mut rows, mut cards) = (String::new(), String::new());
    for (index, (skill, rank, order)) in used.iter().enumerate() {
        let role = match *order {
            0 => "main",
            usize::MAX if snapshot.active_aura_id.as_deref() == Some(skill.id.as_str()) => "aura",
            usize::MAX => "",
            _ => "active",
        };
        let role = if role.is_empty() {
            String::new()
        } else {
            format!("<span class=\"mini-chip\">{role}</span>")
        };
        let capped = if *rank >= skill.max_rank {
            " capped"
        } else {
            ""
        };
        let _ = write!(
            rows,
            "<button type=\"button\" class=\"pick-row skill-row{}\" data-pick=\"{index}\" \
aria-pressed=\"{}\"><span class=\"icon-box skill-icon\">{}</span><span>\
<span class=\"skill-name\">{}{role}</span><span class=\"skill-type\">{}</span></span>\
<span class=\"num skill-rank{capped}\">{rank} / {}</span></button>",
            active(index),
            index == 0,
            skill_icon(class_id, skill, 28),
            esc(&skill.name),
            esc(&skill_type(skill)),
            skill.max_rank
        );
        cards.push_str(&skill_card(snapshot, skill, *rank, index));
    }
    let body = format!(
        "<div class=\"picker\"><div class=\"pick-list\">{rows}</div>\
<div class=\"pick-panes\">{cards}</div></div>"
    );
    Some((panel("Skills", &format!("{points} pts"), &body), points))
}

fn node_card(kind: &str, title: &str, line: &str) -> String {
    format!(
        "<div class=\"node{kind}\"><span class=\"diamond\"></span><span class=\"node-text\">\
<strong>{}</strong><span class=\"node-line\">{}</span></span></div>",
        esc(title),
        esc(line)
    )
}

fn minor_card(text: &str, count: &str, hint: &str) -> String {
    format!(
        "<div class=\"node minor\" title=\"{}\"><span class=\"diamond\"></span>\
<span class=\"node-text\">{}</span><span class=\"num node-count\">{}</span></div>",
        esc(hint),
        esc(text),
        esc(count)
    )
}

fn socket_name(content: &TreeSocketContent) -> (String, String) {
    match content {
        TreeSocketContent::Uncut { affixes } => (
            "Uncut jewel".to_owned(),
            format!("{} affixes", affixes.len()),
        ),
        TreeSocketContent::Item { id } => {
            let name = data::get_item(id)
                .map(|item| item.name.clone())
                .or_else(|| data::get_gem(id).map(|gem| gem.name.clone()))
                .or_else(|| data::get_rune(id).map(|rune| format!("Rune of {}", rune.name)))
                .unwrap_or_else(|| id.clone());
            (name, "Socketed in the tree".to_owned())
        }
    }
}

/// The incarnation tree picture and node lists, and how many nodes are allocated.
pub fn incarnation(snapshot: &BuildSnapshot) -> Option<(String, usize)> {
    let describe = |id: u32| {
        let node = data::get_tree_node(id)?;
        let lines = std::iter::once(&node.title).chain(&node.lines);
        Some(lines.map(String::as_str).collect::<Vec<_>>().join("\n"))
    };
    let picture = tree::INCARNATION.overlay(
        &snapshot.allocated_tree_nodes,
        "Allocated nodes on the incarnation tree",
        describe,
    )?;
    let mut ids = snapshot.allocated_tree_nodes.clone();
    ids.sort_unstable();
    ids.dedup();
    let mut notables = Vec::new();
    let mut minors: BTreeMap<String, u32> = BTreeMap::new();
    for node in ids.iter().filter_map(|id| data::get_tree_node(*id)) {
        let line = node.lines.join(", ");
        match node.kind.as_str() {
            "big" => notables.push(node_card("", &node.title, &line)),
            "jewelry" => {}
            _ if line.is_empty() => {}
            _ => *minors.entry(line).or_default() += 1,
        }
    }
    let mut minors: Vec<(String, u32)> = minors.into_iter().collect();
    minors.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let minor_total: u32 = minors.iter().map(|(_, count)| count).sum();
    let minors: Vec<String> = minors
        .iter()
        .map(|(text, count)| minor_card(text, &format!("× {count}"), text))
        .collect();
    let mut sockets: Vec<_> = snapshot.tree_socketed.iter().collect();
    sockets.sort_by_key(|(id, _)| **id);
    let sockets: Vec<String> = sockets
        .into_iter()
        .map(|(_, content)| {
            let (name, line) = socket_name(content);
            node_card(" jewel", &name, &line)
        })
        .collect();
    let mut body = format!("<div class=\"tree-wrap\">{picture}</div>");
    for (title, count, cards) in [
        ("Notables", notables.len().to_string(), notables),
        ("Minor nodes", format!("{minor_total} allocated"), minors),
        ("Jewelry sockets", sockets.len().to_string(), sockets),
    ] {
        if !cards.is_empty() {
            body.push_str(&section_bar(title, &count));
            let _ = write!(body, "<div class=\"node-grid\">{}</div>", cards.concat());
        }
    }
    let html = panel("Incarnation tree", &format!("{} nodes", ids.len()), &body);
    Some((html, ids.len()))
}

/// The ether tree picture and its summed bonuses, and how many nodes are allocated.
pub fn ether(
    snapshot: &BuildSnapshot,
    performance: Option<&PlannerPerformance>,
) -> Option<(String, usize)> {
    let picture = tree::ETHER.overlay(
        &snapshot.allocated_ether_nodes,
        "Allocated nodes on the ether tree",
        |_| None,
    )?;
    let bonuses: Vec<String> = performance
        .iter()
        .flat_map(|performance| &performance.ether)
        .map(|row| {
            let unit = if row.is_percent { "%" } else { "" };
            let text = format!("{}: {}", row.label, row.description);
            minor_card(&text, &format!("{}{unit}", compact(row.total)), &text)
        })
        .collect();
    let mut ids = snapshot.allocated_ether_nodes.clone();
    ids.sort_unstable();
    ids.dedup();
    let mut body = format!("<div class=\"tree-wrap\">{picture}</div>");
    if !bonuses.is_empty() {
        body.push_str(&section_bar("Bonuses", &bonuses.len().to_string()));
        let _ = write!(body, "<div class=\"node-grid\">{}</div>", bonuses.concat());
    }
    let html = panel("Ether tree", &format!("{} nodes", ids.len()), &body);
    Some((html, ids.len()))
}

pub fn mercenary(snapshot: &BuildSnapshot) -> Option<String> {
    let mercenaries = mercenary::data();
    let class = mercenaries
        .classes
        .iter()
        .find(|class| Some(class.id.as_str()) == snapshot.merc_class_id.as_deref())?;
    let skills: String = class
        .skills
        .iter()
        .filter_map(|skill| {
            let rank = snapshot
                .merc_skill_ranks
                .get(&skill.id)
                .filter(|rank| **rank > 0)?;
            Some(format!(
                "<li class=\"chip\">{} <span class=\"num tone-gold\">{rank}</span></li>",
                esc(&skill.name)
            ))
        })
        .collect();
    let names: HashMap<_, _> = data::game_config()
        .slots
        .iter()
        .flatten()
        .map(|slot| (slot.key.as_str(), slot.name.as_str()))
        .collect();
    let entries: Vec<gear::Entry> = mercenaries
        .slots
        .iter()
        .filter_map(|slot| {
            let item = snapshot.merc_inventory.get(slot)?;
            Some((*names.get(slot.as_str()).unwrap_or(&"Item"), item))
        })
        .collect();
    let mut body = String::new();
    if !skills.is_empty() {
        let _ = write!(body, "<ul class=\"chips\">{skills}</ul>");
    }
    if entries.is_empty() {
        body.push_str("<p class=\"pad hint\">No gear equipped.</p>");
    } else {
        body.push_str(&gear::picker(&entries));
    }
    let meta = format!("{} · {}", class.name, class.role);
    Some(panel("Mercenary", &meta, &body))
}

/// `notes` is markup already sanitised by `Notes::to_html`.
pub fn notes(notes: &str) -> String {
    if notes.trim().is_empty() {
        return String::new();
    }
    panel("Notes", "", &format!("<div class=\"notes\">{notes}</div>"))
}

pub fn code(code: &str) -> String {
    let body = format!(
        "<div class=\"pad code-panel\"><p class=\"hint\">Copy this code and import it in the \
HSPlanner library to open the build.</p><textarea id=\"build-code\" readonly rows=\"4\" \
spellcheck=\"false\" aria-label=\"Build code\">{}</textarea></div>",
        esc(code)
    );
    panel("Build code", &format!("{} characters", code.len()), &body)
}
