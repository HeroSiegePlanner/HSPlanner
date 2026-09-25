//! Human-readable item editing. Parsing never mutates the caller's draft.
use crate::gear;
use hsplanner_engine::calc::{
    affix::apply_stars_to_ranged_value, data, rank::item_skill_rank_range, types::*,
};
use regex::Regex;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::LazyLock,
};

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub line: usize,
    pub message: String,
    pub warning: bool,
}
pub struct ParseResult {
    pub item: Option<EquippedItem>,
    pub diagnostics: Vec<Diagnostic>,
}
static VALUE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([+-]?)(?:\[([0-9.]+)-([0-9.]+)\]|([0-9.]+))%?\s+(.+)$").unwrap()
});
static SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(.+?)\s+\[T(\d+)(?:,\s*(roll\s+([0-9.]+)|custom))?\]$").unwrap()
});
static SOCKET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\[(\d+)\]\s*\((Normal|Rainbow)\):\s*(.+)$").unwrap());

pub fn custom_stats() -> Vec<(String, String, bool)> {
    let mut by_name = BTreeMap::new();
    for s in &data::game_config().stats {
        by_name.insert(
            s.name.to_lowercase(),
            (
                s.key.clone(),
                s.name.clone(),
                s.format.as_deref() == Some("percent"),
            ),
        );
    }
    let mut by_key = HashMap::new();
    for row in by_name.into_values() {
        by_key.insert(row.0.clone(), row);
    }
    let mut rows: Vec<_> = by_key.into_values().collect();
    rows.sort_by(|a, b| a.1.cmp(&b.1));
    rows
}
fn stat_label(key: &str) -> String {
    data::game_config()
        .stats
        .iter()
        .rev()
        .find(|s| s.key == key)
        .map_or_else(|| key.to_owned(), |s| s.name.clone())
}
fn percent(key: &str) -> bool {
    data::game_config()
        .stats
        .iter()
        .rev()
        .find(|s| s.key == key)
        .is_some_and(|s| s.format.as_deref() == Some("percent"))
}
fn number(v: f64) -> String {
    let v = (v * 10000.).round() / 10000.;
    format!("{v}")
}
fn value_text((a, b): (f64, f64), percent: bool) -> String {
    let sign = if a < 0. && b <= 0. { "-" } else { "+" };
    let value = if a == b {
        number(a.abs())
    } else {
        format!(
            "[{}-{}]",
            number(a.abs().min(b.abs())),
            number(a.abs().max(b.abs()))
        )
    };
    format!("{sign}{value}{}", if percent { "%" } else { "" })
}
fn prefix(line: &str) -> Option<(f64, bool, String)> {
    let c = VALUE.captures(line)?;
    let range = c.get(3).is_some();
    if let Some(lo) = c.get(2)
        && !lo.as_str().parse::<f64>().ok()?.is_finite()
    {
        return None;
    }
    let n: f64 = c.get(3).or_else(|| c.get(4))?.as_str().parse().ok()?;
    n.is_finite()
        .then(|| (if &c[1] == "-" { -n } else { n }, range, c[5].to_owned()))
}
fn description_key(text: &str) -> String {
    let text = hsplanner_engine::calc::resistance::canonical_text(text);
    static NUMBERS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[+-]?(?:\[[0-9.]+-[0-9.]+\]|[0-9.]+)%?").unwrap());
    NUMBERS
        .replace_all(&text, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn displayed(
    base: &ItemBase,
    item: &EquippedItem,
    key: &str,
    value: RangedValue,
    skill: bool,
) -> (f64, f64) {
    let sockets: Vec<_> = item.socketed.iter().map(|v| v.as_deref()).collect();
    if !skill && data::detect_runeword(base, &sockets).is_some() {
        return value.as_ranged();
    }
    let stars = data::can_star_forge(&base.slot, &base.rarity)
        .then_some(item.stars)
        .flatten();
    if skill {
        item_skill_rank_range(key, value.as_ranged(), stars, None)
    } else {
        apply_stars_to_ranged_value(value.as_ranged(), key, stars)
    }
}
fn modifier_text(eq: &EquippedAffix, forged: bool) -> String {
    let def = if forged {
        data::get_crystal_mod(&eq.affix_id)
    } else {
        data::get_affix(&eq.affix_id)
    };
    let Some(def) = def else {
        return format!("{} [T{}, roll {}]", eq.affix_id, eq.tier, eq.roll);
    };
    let body = if let Some(value) = eq.custom_value {
        let label = prefix(&def.description).map_or_else(|| def.name.clone(), |p| p.2);
        format!(
            "{} {label}",
            value_text((value, value), matches!(def.format, AffixFormat::Percent))
        )
    } else {
        def.description.clone()
    };
    let unholy = if def.group_id == "random_unholy" {
        "[Unholy] "
    } else {
        ""
    };
    format!(
        "{unholy}{body} [T{}, {}]",
        eq.tier,
        eq.custom_value
            .map_or_else(|| format!("roll {}", eq.roll), |_| "custom".into())
    )
}

pub fn serialize(item: &EquippedItem) -> Result<String, String> {
    let base = data::get_item(&item.base_id).ok_or("Unknown base item")?;
    let mut lines = vec![
        format!("Rarity: {}", base.rarity.to_uppercase()),
        base.name.clone(),
        base.base_type.clone(),
        "--------".into(),
        format!("Stars: {}", item.stars.unwrap_or(0)),
    ];
    for (section, base_stats, overrides, skill) in [
        (
            "Implicit:",
            base.implicit.as_ref(),
            &item.implicit_overrides,
            false,
        ),
        (
            "Skill Bonuses:",
            base.skill_bonuses.as_ref(),
            &item.skill_bonus_overrides,
            true,
        ),
    ] {
        lines.extend(["--------".into(), section.into()]);
        let mut keys: Vec<_> = base_stats
            .into_iter()
            .flat_map(|s| s.keys())
            .chain(overrides.keys())
            .cloned()
            .collect();
        keys.sort();
        keys.dedup();
        for key in keys {
            // Zero overrides suppress an authored mod; don't resurrect its line.
            if overrides.get(&key) == Some(&0.) {
                continue;
            }
            let shown = overrides
                .get(&key)
                .map(|v| (*v, *v))
                .unwrap_or_else(|| displayed(base, item, &key, base_stats.unwrap()[&key], skill));
            let label = if skill {
                format!("to {key}")
            } else {
                stat_label(&key)
            };
            lines.push(format!(
                "{} {label}{}",
                value_text(shown, !skill && percent(&key)),
                if overrides.contains_key(&key) {
                    " [custom]"
                } else {
                    ""
                }
            ));
        }
    }
    for (section, entries, forged) in [
        ("Affixes:", &item.affixes, false),
        ("Forged Mods:", &item.forged_mods, true),
    ] {
        lines.extend(["--------".into(), section.into()]);
        lines.extend(entries.iter().map(|a| modifier_text(a, forged)));
    }
    lines.extend([
        "--------".into(),
        format!(
            "Sockets: {}",
            (0..item.socket_count as usize)
                .map(
                    |i| if item.socket_types.get(i) == Some(&SocketType::Rainbow) {
                        "R"
                    } else {
                        "N"
                    }
                )
                .collect::<Vec<_>>()
                .join("-")
        ),
    ]);
    for (i, id) in item
        .socketed
        .iter()
        .enumerate()
        .take(item.socket_count as usize)
    {
        if let Some(id) = id {
            let name = data::data()
                .gems
                .get(id)
                .map(|g| g.name.clone())
                .or_else(|| {
                    data::data()
                        .runes
                        .get(id)
                        .map(|r| format!("Rune of {}", r.name))
                })
                .unwrap_or_else(|| id.clone());
            lines.push(format!(
                "[{}] ({}): {name}",
                i + 1,
                if item.socket_types.get(i) == Some(&SocketType::Rainbow) {
                    "Rainbow"
                } else {
                    "Normal"
                }
            ));
        }
    }
    if let Some(augment) = &item.augment {
        lines.extend([
            "--------".into(),
            format!(
                "Augment: {} · Level {}",
                data::get_augment(&augment.id).map_or(augment.id.as_str(), |a| a.name.as_str()),
                augment.level
            ),
        ]);
    }
    Ok(lines.join("\n"))
}

fn parse_modifier(line: &str, forged: bool) -> Result<EquippedAffix, String> {
    let normalized = hsplanner_engine::calc::resistance::canonical_text(line);
    let line = normalized.as_ref();
    let unholy = line.starts_with("[Unholy]");
    let text = line.strip_prefix("[Unholy]").unwrap_or(line).trim();
    let c = SUFFIX
        .captures(text)
        .ok_or("Expected [T<tier>, roll <0..1>] or [T<tier>, custom]")?;
    let tier: u32 = c[2].parse().map_err(|_| "Invalid tier")?;
    let roll: f64 = c
        .get(4)
        .map_or(Ok(1.), |m| m.as_str().parse())
        .map_err(|_| "Invalid roll")?;
    if !roll.is_finite() || !(0.0..=1.0).contains(&roll) {
        return Err("Roll must be between 0 and 1".into());
    }
    let content = c[1].trim();
    let defs = if forged {
        &data::data().crystals
    } else {
        &data::data().affixes
    };
    let mut matches: Vec<_> = defs
        .values()
        .filter(|a| a.tier == tier && (a.group_id == "random_unholy") == unholy)
        .filter(|a| {
            a.description.eq_ignore_ascii_case(content)
                || a.name.eq_ignore_ascii_case(content)
                || a.id == content
                || description_key(&a.description) == description_key(content)
        })
        .collect();
    matches.sort_by(|a, b| {
        let exact = |a: &Affix| {
            a.description.eq_ignore_ascii_case(content)
                || a.name.eq_ignore_ascii_case(content)
                || a.id == content
        };
        exact(b).cmp(&exact(a)).then(a.id.cmp(&b.id))
    });
    let a = matches.first().ok_or("Unknown affix or tier")?;
    let custom = c
        .get(3)
        .is_some_and(|m| m.as_str().eq_ignore_ascii_case("custom"));
    let custom_value = if custom {
        Some(
            prefix(content)
                .filter(|p| !p.1)
                .ok_or("Custom roll needs a single numeric value")?
                .0,
        )
    } else if content != a.description {
        prefix(content).filter(|p| !p.1).map(|p| p.0)
    } else {
        None
    };
    Ok(EquippedAffix {
        affix_id: a.id.clone(),
        tier,
        roll,
        custom_value,
    })
}

pub fn parse(text: &str, original: &EquippedItem) -> ParseResult {
    let mut diagnostics = vec![];
    let mut report = |line, message: String, warning| {
        diagnostics.push(Diagnostic {
            line,
            message,
            warning,
        })
    };
    let Some(base) = data::get_item(&original.base_id) else {
        return ParseResult {
            item: None,
            diagnostics: vec![Diagnostic {
                line: 0,
                message: "Unknown base item".into(),
                warning: false,
            }],
        };
    };
    // Retain fields absent from the reference text format (random skill/class, flags).
    let mut item = original.clone();
    item.affixes.clear();
    item.forged_mods.clear();
    item.augment = None;
    item.socketed.clear();
    item.socket_types.clear();
    item.socket_count = 0;
    item.implicit_overrides.clear();
    item.skill_bonus_overrides.clear();
    let mut kept = HashSet::new();
    let mut skills = HashSet::new();
    let mut skill_section = false;
    let mut section = "header";
    let mut pending_stats = vec![];
    let mut seen_header = 0;
    for (offset, raw) in text.lines().enumerate() {
        let line_no = offset + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.len() >= 4 && line.chars().all(|c| c == '-') {
            section = "";
            continue;
        }
        match line {
            "Implicit:" => {
                section = "implicit";
                continue;
            }
            "Skill Bonuses:" => {
                section = "skills";
                skill_section = true;
                continue;
            }
            "Affixes:" => {
                section = "affixes";
                continue;
            }
            "Forged Mods:" => {
                section = "forged";
                continue;
            }
            _ => {}
        }
        if let Some(v) = line.strip_prefix("Stars:") {
            match v.trim().parse::<u32>() {
                Ok(v) if v <= hsplanner_engine::calc::star_scaling::max_stars() => {
                    item.stars = Some(v)
                }
                _ => report(
                    line_no,
                    format!(
                        "Stars must be 0..{}",
                        hsplanner_engine::calc::star_scaling::max_stars()
                    ),
                    false,
                ),
            };
            section = "metadata";
            continue;
        }
        if let Some(map) = line.strip_prefix("Sockets:") {
            section = "sockets";
            let parts: Vec<_> = map.trim().split('-').filter(|s| !s.is_empty()).collect();
            if parts.len() > 6 || parts.iter().any(|s| !matches!(*s, "N" | "R" | "_")) {
                report(
                    line_no,
                    "Expected at most six sockets, separated by '-' (N/R/_)".into(),
                    false,
                );
                continue;
            }
            item.socket_count = parts.len() as u32;
            item.socketed = vec![None; parts.len()];
            item.socket_types = parts
                .iter()
                .map(|s| {
                    if *s == "R" {
                        SocketType::Rainbow
                    } else {
                        SocketType::Normal
                    }
                })
                .collect();
            continue;
        }
        if let Some(v) = line.strip_prefix("Augment:") {
            section = "augment";
            let result = v.trim().rsplit_once(" · Level ").and_then(|(name, level)| {
                data::data()
                    .augments
                    .values()
                    .find(|a| a.name.eq_ignore_ascii_case(name) || a.id == name)
                    .zip(level.parse::<u32>().ok())
            });
            match result {
                Some((a, level)) if level > 0 && level as usize <= a.levels.len() => {
                    item.augment = Some(AugmentRef {
                        id: a.id.clone(),
                        level,
                    })
                }
                _ => report(
                    line_no,
                    "Expected Augment: <name> · Level <valid level>".into(),
                    false,
                ),
            };
            continue;
        }
        if line.starts_with("Runeword:") {
            section = "metadata";
            continue;
        }
        if let Some(rarity) = line.strip_prefix("Rarity:") {
            if !rarity.trim().eq_ignore_ascii_case(&base.rarity) {
                report(
                    line_no,
                    format!("Rarity is read-only ({})", base.rarity.to_uppercase()),
                    true,
                );
            }
            section = "header";
            seen_header = 0;
            continue;
        }
        if section == "header" && seen_header < 2 {
            seen_header += 1;
            let expected = if seen_header == 1 {
                &base.name
            } else {
                &base.base_type
            };
            if line != expected {
                report(line_no, "Base item identity is read-only".into(), true)
            };
            continue;
        }
        if [
            "Item Level:",
            "Requires Level:",
            "Defense:",
            "Damage:",
            "Attack Speed:",
            "Block:",
        ]
        .iter()
        .any(|p| line.starts_with(p))
        {
            continue;
        }
        match section {
            "implicit" | "skills" => {
                let custom = line.ends_with("[custom]");
                let body = line.trim_end_matches("[custom]").trim();
                let normalized = hsplanner_engine::calc::resistance::canonical_text(body);
                let body = normalized.as_ref();
                let Some((value, range, name)) = prefix(body) else {
                    report(
                        line_no,
                        "Expected a numeric value followed by a stat name".into(),
                        false,
                    );
                    continue;
                };
                let skill = section == "skills";
                let key = if skill {
                    base.skill_bonuses
                        .as_ref()
                        .and_then(|s| {
                            s.keys()
                                .find(|k| k.eq_ignore_ascii_case(name.trim_start_matches("to ")))
                        })
                        .cloned()
                } else {
                    base.implicit
                        .as_ref()
                        .and_then(|stats| {
                            stats.keys().find(|key| {
                                stat_label(key).eq_ignore_ascii_case(&name)
                                    || key.as_str()
                                        == hsplanner_engine::calc::resistance::canonical_key(&name)
                            })
                        })
                        .cloned()
                        .or_else(|| {
                            custom_stats()
                                .into_iter()
                                .find(|(key, label, _)| {
                                    label.eq_ignore_ascii_case(&name)
                                        || key.as_str()
                                            == hsplanner_engine::calc::resistance::canonical_key(
                                                &name,
                                            )
                                })
                                .map(|s| s.0)
                        })
                };
                let Some(key) = key else {
                    report(line_no, format!("Unknown stat: {name}"), false);
                    continue;
                };
                if skill {
                    skills.insert(key.clone());
                } else {
                    kept.insert(key.clone());
                }
                if custom && range {
                    report(
                        line_no,
                        "A custom value must be a single number".into(),
                        false,
                    );
                    continue;
                }
                pending_stats.push((key, value, range, custom, skill));
            }
            "affixes" | "forged" => {
                let result = {
                    let forged = section == "forged";
                    let existing = if forged {
                        &original.forged_mods
                    } else {
                        &original.affixes
                    };
                    existing
                        .iter()
                        .find(|eq| modifier_text(eq, forged) == line)
                        .cloned()
                        .map(Ok)
                        .unwrap_or_else(|| parse_modifier(line, forged))
                };
                match result {
                    Ok(a) => {
                        if section == "forged" {
                            item.forged_mods.push(a)
                        } else {
                            item.affixes.push(a)
                        }
                    }
                    Err(e) => report(line_no, e, false),
                }
            }
            "sockets" => {
                let Some(c) = SOCKET.captures(line) else {
                    report(
                        line_no,
                        "Expected [index] (Normal|Rainbow): <gem or Rune of name>".into(),
                        false,
                    );
                    continue;
                };
                let index = c[1].parse::<usize>().unwrap_or(0).wrapping_sub(1);
                if index >= item.socketed.len() {
                    report(line_no, "Socket index outside socket map".into(), false);
                    continue;
                }
                let name = c[3].trim();
                let id = if let Some(name) = name.strip_prefix("Rune of ") {
                    data::data()
                        .runes
                        .values()
                        .find(|r| r.name.eq_ignore_ascii_case(name))
                        .map(|r| r.id.clone())
                } else {
                    data::data()
                        .gems
                        .values()
                        .find(|g| g.name.eq_ignore_ascii_case(name))
                        .map(|g| g.id.clone())
                };
                match id {
                    Some(id) => item.socketed[index] = Some(id),
                    None => report(line_no, format!("Unknown socketable: {name}"), false),
                };
                item.socket_types[index] = if c[2].eq_ignore_ascii_case("rainbow") {
                    SocketType::Rainbow
                } else {
                    SocketType::Normal
                };
            }
            _ => report(line_no, format!("Unknown section or line: {line}"), false),
        }
    }
    for (key, value, range, custom, skill) in pending_stats {
        let base_value = if skill {
            base.skill_bonuses.as_ref()
        } else {
            base.implicit.as_ref()
        }
        .and_then(|s| s.get(&key));
        if range && !custom {
            continue;
        }
        if !custom
            && base_value.is_some_and(|v| {
                let (a, b) = displayed(base, &item, &key, *v, skill);
                a == b && (a - value).abs() < 0.005
            })
        {
            continue;
        }
        if skill {
            item.skill_bonus_overrides.insert(key, value);
        } else {
            item.implicit_overrides.insert(key, value);
        }
    }
    for key in base.implicit.iter().flat_map(|s| s.keys()) {
        if !kept.contains(key) {
            item.implicit_overrides.insert(key.clone(), 0.);
        }
    }
    if skill_section {
        for key in base.skill_bonuses.iter().flat_map(|s| s.keys()) {
            if !skills.contains(key) {
                item.skill_bonus_overrides.insert(key.clone(), 0.);
            }
        }
    }
    if item.socket_count > gear::max_sockets(&item) {
        report(0, "Too many sockets for this item".into(), false)
    }
    if item.forged_mods.len() > 1 {
        report(0, "An item can have only one forged modifier".into(), false)
    }
    if base
        .max_affixes
        .is_some_and(|max| item.affixes.len() > max as usize)
    {
        report(0, "Too many affixes for this item".into(), false)
    }
    if !item.affixes.is_empty() && !gear::accepts_affixes(base) {
        report(0, "This item cannot have affixes".into(), false)
    }
    if !item.forged_mods.is_empty() && !data::can_star_forge(&base.slot, &base.rarity) {
        report(0, "This item cannot have forged modifiers".into(), false)
    }
    let valid = !diagnostics.iter().any(|d| !d.warning);
    ParseResult {
        item: valid.then_some(item),
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starred_skill_text_preserves_locked_ranks_and_unpinned_ranges() {
        for (id, line) in [
            (
                "armors_heroic_gabriels_broken_wings",
                "+[1-10] to Fallen God's Bloodlust",
            ),
            ("gloves_satanic_thor_s_battle_gloves", "+[4-6] to Holy Aura"),
        ] {
            let mut item = gear::make_item(id).unwrap();
            item.stars = Some(5);
            let text = serialize(&item).unwrap();
            assert!(text.lines().any(|shown| shown == line), "{id}: {text}");
            let result = parse(&text, &item);
            let parsed = result
                .item
                .unwrap_or_else(|| panic!("{:?}", result.diagnostics));
            assert!(parsed.skill_bonus_overrides.is_empty(), "{id}");
        }
    }

    #[test]
    fn every_base_item_text_is_valid_and_preserves_unpinned_stats() {
        let mut bases: Vec<_> = data::data().items.values().collect();
        bases.sort_by(|a, b| a.id.cmp(&b.id));
        for base in bases {
            let item = gear::make_item(&base.id).unwrap();
            let text = serialize(&item).unwrap();
            let result = parse(&text, &item);
            assert!(
                result.item.is_some(),
                "{}: {:?}\n{}",
                base.id,
                result.diagnostics,
                text
            );
            let parsed = result.item.unwrap();
            assert_eq!(
                item.implicit_overrides, parsed.implicit_overrides,
                "{}",
                base.id
            );
            assert_eq!(
                item.skill_bonus_overrides, parsed.skill_bonus_overrides,
                "{}",
                base.id
            );
        }
    }
    fn armor() -> EquippedItem {
        let base = data::data()
            .items
            .values()
            .find(|b| b.slot == "armor" && b.rarity == "common")
            .unwrap();
        gear::make_item(&base.id).unwrap()
    }
    #[test]
    fn text_edits_affixes_forge_custom_stats_and_preserves_unrepresented_fields() {
        let mut item = armor();
        item.all_skills_class_id = Some("viking".into());
        gear::add_affix(&mut item, "25_50_enhanced_defense_t1_armorer_s").unwrap();
        gear::set_forge(&mut item, Some("crystal_satanic_all_attributes")).unwrap();
        gear::set_forge_value(&mut item, 0, 17.);
        let text = serialize(&item).unwrap();
        let parsed = parse(&text, &item).item.unwrap();
        assert_eq!(
            serde_json::to_value(&item).unwrap(),
            serde_json::to_value(&parsed).unwrap()
        );
        let text = text
            .replace("Implicit:\n", "Implicit:\n+123 to Strength [custom]\n")
            .replace("roll 1]", "roll 0.25]");
        let result = parse(&text, &item);
        assert!(result.item.is_some(), "{:?}", result.diagnostics);
        let parsed = result.item.unwrap();
        assert_eq!(parsed.implicit_overrides["to_strength"], 123.);
        assert_eq!(parsed.affixes[0].roll, 0.25);
        assert_eq!(parsed.forged_mods[0].custom_value, Some(17.));
        assert_eq!(parsed.all_skills_class_id, item.all_skills_class_id);
    }
    #[test]
    fn relics_reject_affixes_and_forged_modifiers() {
        let mut item = gear::make_item("relic_relic_1000_kg").unwrap();
        gear::set_forge(&mut item, Some("crystal_satanic_all_attributes")).unwrap();
        let affix = data::get_affix("25_50_enhanced_defense_t1_armorer_s").unwrap();
        item.affixes.push(EquippedAffix {
            affix_id: affix.id.clone(),
            tier: affix.tier,
            roll: 1.,
            custom_value: None,
        });
        let result = parse(&serialize(&item).unwrap(), &item);
        assert!(result.item.is_none(), "{:?}", result.diagnostics);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("affixes"))
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("forged"))
        );
    }
    #[test]
    fn sockets_augment_and_deleted_implicit_lines_follow_text() {
        let base = data::data()
            .items
            .values()
            .find(|b| b.rarity == "common" && b.max_sockets.unwrap_or(0) >= 2)
            .unwrap();
        let mut item = gear::make_item(&base.id).unwrap();
        gear::set_socket_count(&mut item, 2);
        let rune = data::data().runes.values().next().unwrap();
        let gem = data::data().gems.values().next().unwrap();
        gear::set_socket(&mut item, 0, Some(&rune.id)).unwrap();
        gear::set_socket(&mut item, 1, Some(&gem.id)).unwrap();
        item.socket_types[1] = SocketType::Rainbow;
        let augment = data::data()
            .augments
            .values()
            .find(|a| !a.levels.is_empty())
            .unwrap();
        gear::set_augment(&mut item, Some(&augment.id)).unwrap();
        item.implicit_overrides.insert("to_strength".into(), 12.);
        let text = serialize(&item).unwrap();
        let result = parse(&text, &item);
        assert!(result.item.is_some(), "{:?}\n{text}", result.diagnostics);
        let next = result.item.unwrap();
        assert_eq!(next.socketed, item.socketed);
        assert_eq!(next.socket_types, item.socket_types);
        assert_eq!(next.augment.unwrap().id, augment.id);
        let modified = text.replace("+12 to Strength [custom]\n", "");
        assert!(
            !parse(&modified, &item)
                .item
                .unwrap()
                .implicit_overrides
                .contains_key("to_strength")
        );
        assert!(
            parse(&text.replace("Level 1", "Level 999"), &item)
                .item
                .is_none()
        );
    }

    #[test]
    fn deleted_authored_mod_stays_suppressed_after_save_and_reopening_text() {
        let item = gear::make_item("axe_angelic_st_rexis_sundering_axe").unwrap();
        let text = serialize(&item).unwrap();
        let deleted: String = text
            .lines()
            .filter(|line| !line.contains("Enhanced Damage"))
            .map(|line| format!("{line}\n"))
            .collect();
        assert_ne!(deleted, format!("{}\n", text.trim_end()));
        let parsed = parse(&deleted, &item).item.unwrap();
        assert_eq!(parsed.implicit_overrides["enhanced_damage"], 0.);
        let saved: EquippedItem =
            serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
        let reopened = serialize(&saved).unwrap();
        assert!(!reopened.contains("Enhanced Damage"));
        let reparsed = parse(&reopened, &saved).item.unwrap();
        assert_eq!(reparsed.implicit_overrides["enhanced_damage"], 0.);
    }

    #[test]
    fn invalid_text_blocks_save_and_reports_source_line() {
        let item = armor();
        let text = serialize(&item).unwrap().replace("Stars: 0", "Stars: 999");
        let result = parse(&text, &item);
        assert!(result.item.is_none());
        assert!(result.diagnostics.iter().any(|d| d.line == 5 && !d.warning));
        assert!(
            parse(&text.replace("Stars: 999", "Stars: -1"), &item)
                .item
                .is_none()
        );
        let text = serialize(&item)
            .unwrap()
            .replace("Implicit:\n", "Implicit:\n+1 Unknown stat [custom]\n");
        assert!(parse(&text, &item).item.is_none());
    }

    #[test]
    fn legacy_resistance_names_import_as_current_positive_ignore_bonuses() {
        let mut item = gear::make_item("body_armor_angelic_st_jupe_s_plate_of_command").unwrap();
        item.implicit_overrides.insert("ignore_all_res".into(), 23.);
        let old_text = serialize(&item)
            .unwrap()
            .replace("Ignore All Resistance", "Enemy All Resist");
        let parsed = parse(&old_text, &item);
        let imported = parsed
            .item
            .unwrap_or_else(|| panic!("{:?}", parsed.diagnostics));
        assert_eq!(imported.implicit_overrides["ignore_all_res"], 23.);
        assert!(!imported.implicit_overrides.contains_key("enemy_all_resist"));
        let affix = parse_modifier("-7% to Enemy Cold Resistance [T3, custom]", false).unwrap();
        assert_eq!(affix.affix_id, "to_enemy_cold_resistance_t3_coldbreaking");
        assert_eq!(affix.custom_value, Some(7.));
    }
}
