//! Item lists with detail cards, built from the same text the planner exports for an item.
use std::fmt::Write;

use hsplanner_build::item_text;
use hsplanner_engine::calc::{
    data,
    types::{EquippedItem, SocketType},
};

use crate::{assets, html::esc, numbers::readable, panels::active};

const MAX_STARS: usize = 5;
const MAX_SHOWN_SOCKETS: usize = 12;

pub type Entry<'a> = (&'a str, &'a EquippedItem);

/// A list of items beside the detail card of the selected one.
pub fn picker(entries: &[Entry]) -> String {
    let (mut rows, mut cards) = (String::new(), String::new());
    for (index, (slot, item)) in entries.iter().enumerate() {
        rows.push_str(&row(index, slot, item));
        cards.push_str(&card(index, slot, item));
    }
    format!(
        "<div class=\"picker\"><div class=\"pick-list\">{rows}</div>\
<div class=\"pick-panes\">{cards}</div></div>"
    )
}

pub fn rarity_class(rarity: &str) -> String {
    let token: String = rarity
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();
    format!(" rarity-{token}")
}

/// The runeword the item's runes spell, detected the way the engine does.
fn runeword_name(item: &EquippedItem) -> Option<&'static str> {
    let base = data::get_item(&item.base_id)?;
    let socketed: Vec<Option<&str>> = item.socketed.iter().map(Option::as_deref).collect();
    Some(data::detect_runeword(base, &socketed)?.name.as_str())
}

/// What the item is called: its runeword when its runes spell one, else its base.
pub fn item_name(item: &EquippedItem) -> &'static str {
    runeword_name(item)
        .or(data::get_item(&item.base_id).map(|base| base.name.as_str()))
        .unwrap_or("Unknown item")
}

fn socketable_name(id: &str) -> Option<&'static str> {
    Some(match data::get_socketable_by_id(id)? {
        data::Socketable::Gem(gem) => gem.name.as_str(),
        data::Socketable::Rune(rune) => rune.name.as_str(),
    })
}

/// The sprite of a gem or rune, when one is embedded for its name.
fn socket_icon(name: &str) -> Option<String> {
    let path = format!("socketable/{}.png", assets::slug(name));
    assets::game(&path)?;
    Some(format!(
        "<img class=\"pixel\" src=\"/assets/{path}\" alt=\"\">"
    ))
}

fn row(index: usize, slot: &str, item: &EquippedItem) -> String {
    let base = data::get_item(&item.base_id);
    let name = item_name(item);
    let rarity = base.map_or_else(String::new, |base| rarity_class(&base.rarity));
    let mut extra = String::new();
    if let Some(stars) = item.stars.filter(|stars| *stars > 0) {
        let _ = write!(extra, "<span class=\"stars-mini\">★{stars}</span>");
    }
    for socket in 0..(item.socket_count as usize).min(MAX_SHOWN_SOCKETS) {
        let content = item.socketed.get(socket).and_then(Option::as_deref);
        let rainbow = if item.socket_types.get(socket) == Some(&SocketType::Rainbow) {
            " rainbow"
        } else {
            ""
        };
        let sprite = content
            .and_then(socketable_name)
            .and_then(|name| Some((name, socket_icon(name)?)));
        let _ = match sprite {
            Some((name, icon)) => write!(
                extra,
                "<span class=\"socket has-icon{rainbow}\" title=\"{}\">{icon}</span>",
                esc(name)
            ),
            None => write!(
                extra,
                "<span class=\"socket{}{rainbow}\"></span>",
                if content.is_some() { " filled" } else { "" }
            ),
        };
    }
    format!(
        "<button type=\"button\" class=\"pick-row gear-row{rarity}{}\" data-pick=\"{index}\" \
aria-pressed=\"{}\"><span class=\"gear-slot\">{}</span><span class=\"gear-name\">{}</span>\
<span class=\"gear-extra\">{extra}</span></button>",
        active(index),
        index == 0,
        esc(slot),
        esc(name)
    )
}

pub fn card(index: usize, slot: &str, item: &EquippedItem) -> String {
    let open = |rarity: &str| {
        format!(
            "<article class=\"pick-pane tooltip{rarity}{}\" data-pane=\"{index}\">",
            active(index)
        )
    };
    let Some(base) = data::get_item(&item.base_id) else {
        return format!(
            "{}<div class=\"tooltip-head\"><p class=\"tooltip-name\">Unknown item</p>\
<p class=\"tooltip-type\">{}</p></div><p class=\"tooltip-foot\">{}</p></article>",
            open(""),
            esc(&item.base_id),
            esc(slot)
        );
    };
    let icon = format!("items/{}.png", base.id);
    let icon = if assets::game(&icon).is_some() {
        format!(
            "<img class=\"tooltip-icon pixel\" src=\"/assets/{}\" alt=\"\">",
            esc(&icon)
        )
    } else {
        String::new()
    };
    let stars = (item.stars.unwrap_or(0) as usize).min(MAX_STARS);
    let stars = if stars == 0 {
        String::new()
    } else {
        format!(
            "<p class=\"tooltip-stars\" aria-label=\"{stars} stars\">{}</p>",
            "★".repeat(stars)
        )
    };
    let (name, kind) = match runeword_name(item) {
        Some(runeword) => (runeword, format!("Runeword · {}", base.name)),
        None => (
            base.name.as_str(),
            format!("{} · {}", readable(&base.rarity), base.base_type),
        ),
    };
    let mut html = format!(
        "{}<div class=\"tooltip-head{}\"><p class=\"tooltip-name\">{}</p>\
<p class=\"tooltip-type\">{}</p>{stars}{icon}</div>",
        open(&rarity_class(&base.rarity)),
        if icon.is_empty() { "" } else { " has-icon" },
        esc(name),
        esc(&kind)
    );
    if let Ok(text) = item_text::serialize(item) {
        for part in text.split("\n--------\n").skip(1) {
            html.push_str(&section(part));
        }
    }
    let level = base
        .requires_level
        .map_or_else(String::new, |level| format!(" · Req level {level}"));
    let _ = write!(
        html,
        "<p class=\"tooltip-foot\">{}{level}</p></article>",
        esc(slot)
    );
    html
}

/// One block of the exported item text as a section of the detail card.
fn section(part: &str) -> String {
    let mut lines = part.lines();
    let Some(head) = lines.next() else {
        return String::new();
    };
    let rest: Vec<&str> = lines.collect();
    if head.starts_with("Stars: ") {
        return String::new();
    }
    if let Some(layout) = head.strip_prefix("Sockets: ") {
        if layout.is_empty() {
            return String::new();
        }
        let count = format!("{} / {}", rest.len(), layout.split('-').count());
        return block("Sockets", &count, &socket_lines(&rest));
    }
    if head.starts_with("Augment: ") {
        return block("", "", &[format!("<li>{}</li>", esc(head))]);
    }
    if rest.is_empty() {
        return String::new();
    }
    let rows: Vec<String> = rest.iter().map(|line| mod_line(line)).collect();
    block(head.trim_end_matches(':'), "", &rows)
}

fn block(title: &str, trailing: &str, rows: &[String]) -> String {
    let title = if title.is_empty() {
        String::new()
    } else {
        format!(
            "<h3><span>{}</span><span class=\"trailing\">{}</span></h3>",
            esc(title),
            esc(trailing)
        )
    };
    let rows = if rows.is_empty() {
        String::new()
    } else {
        format!("<ul>{}</ul>", rows.concat())
    };
    format!("<div class=\"tooltip-section\">{title}{rows}</div>")
}

/// `<mod text> [T2, roll 0.5]` → the text, with the bracketed roll as a quiet tag.
fn mod_line(line: &str) -> String {
    let (text, tag) = match line.rfind(" [") {
        Some(start) if line.ends_with(']') => {
            (&line[..start], Some(&line[start + 2..line.len() - 1]))
        }
        _ => (line, None),
    };
    let tag = tag.map_or_else(String::new, |tag| {
        format!("<span class=\"tag\">{}</span>", esc(tag))
    });
    format!("<li>{}{tag}</li>", esc(text))
}

/// `[1] (Rainbow): Amethyst` lines → one row per distinct content, counted and tagged when rainbow.
fn socket_lines(lines: &[&str]) -> Vec<String> {
    let mut counted: Vec<((&str, bool), usize)> = Vec::new();
    for line in lines {
        let (kind, name) = line.split_once("): ").unwrap_or(("", line));
        let key = (
            name.strip_prefix("Rune of ").unwrap_or(name),
            kind.ends_with("Rainbow"),
        );
        match counted.iter_mut().find(|(seen, _)| *seen == key) {
            Some((_, count)) => *count += 1,
            None => counted.push((key, 1)),
        }
    }
    counted
        .iter()
        .map(|((name, rainbow), count)| {
            let icon = socket_icon(name).unwrap_or_default();
            let count = if *count > 1 {
                format!(" ×{count}")
            } else {
                String::new()
            };
            let tag = if *rainbow {
                "<span class=\"tag\">rainbow</span>"
            } else {
                ""
            };
            format!("<li>{icon}{}{count}{tag}</li>", esc(name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_sections_become_titled_blocks_of_the_detail_card() {
        assert_eq!(section("Stars: 2"), "");
        assert_eq!(section("Affixes:"), "");
        let mods = section("Affixes:\n+5 to <Strength> [T1, roll 0.5]");
        assert!(mods.contains("<h3><span>Affixes</span>"));
        assert!(mods.contains("<li>+5 to &lt;Strength&gt;<span class=\"tag\">T1, roll 0.5</span>"));
        assert!(section("Implicit:\n+20% Enhanced Damage").contains("<h3><span>Implicit</span>"));
        let sockets = section("Sockets: N-R\n[2] (Rainbow): Amethyst");
        assert!(sockets.contains("<span class=\"trailing\">1 / 2</span>"));
        assert!(sockets.contains("Amethyst<span class=\"tag\">rainbow</span></li>"));
    }

    #[test]
    fn equal_socket_contents_collapse_into_one_counted_line_with_their_sprite() {
        let sockets = section("Sockets: N-N-N\n[1] (Normal): Rune of Um\n[2] (Normal): Rune of Um");

        assert_eq!(sockets.matches("<li>").count(), 1);
        assert!(sockets.contains("src=\"/assets/socketable/um.png\""));
        assert!(sockets.contains("Um ×2</li>"));
    }

    #[test]
    fn a_runeword_item_goes_by_the_runeword_name_and_shows_its_runes() {
        let item: EquippedItem = serde_json::from_value(serde_json::json!({
            "baseId": "shields_normal_royal_shield",
            "socketCount": 5,
            "socketed": ["rune_vex", "rune_sur", "rune_sur", "rune_jah", "rune_ber"],
        }))
        .unwrap();

        let html = picker(&[("Offhand", &item)]);

        assert!(html.contains("<span class=\"gear-name\">Crematorium&#39;s Pledge</span>"));
        assert!(html.contains("<p class=\"tooltip-name\">Crematorium&#39;s Pledge</p>"));
        assert!(html.contains("Runeword · Royal Shield"));
        assert_eq!(html.matches("class=\"socket has-icon\"").count(), 5);
        assert!(html.contains("title=\"Jah\""));
    }

    #[test]
    fn the_first_item_of_a_list_starts_selected() {
        let item: EquippedItem =
            serde_json::from_value(serde_json::json!({ "baseId": "base_mace_ogre_maul" })).unwrap();

        let html = picker(&[("Weapon", &item), ("Offhand", &item)]);

        assert_eq!(html.matches("active\" data-pick").count(), 1);
        assert_eq!(html.matches("aria-pressed=\"true\"").count(), 1);
        assert_eq!(html.matches("data-pane=").count(), 2);
        assert!(html.contains("Ogre Maul"));
    }
}
