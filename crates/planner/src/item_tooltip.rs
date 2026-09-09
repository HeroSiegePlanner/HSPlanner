//! Item tooltip: the reference `itemTooltipModel.ts` model and `ItemTooltip.tsx` look.
use crate::{
    gear::{
        editor::rarity_label,
        presentation::{augment_icon, item_icon, socketable_icon},
    },
    skill_details::{stat_name, units},
};
use gpui_kit::{prelude::*, *};
use hsplanner_engine::calc::{
    affix::{apply_stars_to_ranged_value, rolled_affix_value_with_stars},
    data,
    stats::RAINBOW_MULTIPLIER,
    types::{Affix, AffixFormat, AffixSign, EquippedAffix, EquippedItem, ItemBase, SocketType},
};
use hsplanner_ui::tooltip::CursorTooltipExt;
use hsplanner_ui::{
    theme::{self, TooltipTheme},
    tooltip_text::TooltipText,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, LazyLock},
};

type Ranged = (f64, f64);

// The calculation model intentionally uses maps. Keep the authored display order
// separately: Tauri's Object.entries renders these JSON fields in source order.
#[derive(Default)]
struct OrderedKeys(Vec<String>);

impl<'de> serde::Deserialize<'de> for OrderedKeys {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeysVisitor;
        impl<'de> serde::de::Visitor<'de> for KeysVisitor {
            type Value = OrderedKeys;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an item stat map")
            }

            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let mut keys = Vec::new();
                while let Some(key) = map.next_key()? {
                    keys.push(key);
                    map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(OrderedKeys(keys))
            }
        }
        deserializer.deserialize_map(KeysVisitor)
    }
}

#[derive(serde::Deserialize)]
struct ItemStatOrder {
    id: String,
    #[serde(default)]
    implicit: OrderedKeys,
    #[serde(default, rename = "skillBonuses")]
    skill_bonuses: OrderedKeys,
}

static ITEM_STAT_ORDER: LazyLock<HashMap<String, ItemStatOrder>> = LazyLock::new(|| {
    [
        include_str!("../../../data/items/amulets.json"),
        include_str!("../../../data/items/armors.json"),
        include_str!("../../../data/items/belts.json"),
        include_str!("../../../data/items/boots.json"),
        include_str!("../../../data/items/charms.json"),
        include_str!("../../../data/items/gloves.json"),
        include_str!("../../../data/items/helmets.json"),
        include_str!("../../../data/items/potions.json"),
        include_str!("../../../data/items/relics.json"),
        include_str!("../../../data/items/rings.json"),
        include_str!("../../../data/items/shields.json"),
        include_str!("../../../data/items/weapons.json"),
    ]
    .into_iter()
    .flat_map(|json| {
        serde_json::from_str::<Vec<ItemStatOrder>>(json)
            .expect("embedded item display metadata must be valid")
    })
    .map(|order| (order.id.clone(), order))
    .collect()
});

fn ordered_keys<'a, V>(
    values: &'a HashMap<String, V>,
    authored: Option<&OrderedKeys>,
) -> Vec<&'a String> {
    let mut keys: Vec<_> = values.keys().collect();
    keys.sort_by_key(|key| {
        (
            authored
                .and_then(|order| order.0.iter().position(|authored| authored == *key))
                .unwrap_or(usize::MAX),
            *key,
        )
    });
    keys
}

pub(crate) fn implicit_keys(base: &ItemBase) -> Vec<&String> {
    base.implicit.as_ref().map_or_else(Vec::new, |values| {
        ordered_keys(values, ITEM_STAT_ORDER.get(&base.id).map(|o| &o.implicit))
    })
}

/// Base skill bonuses in authored order, then the completed runeword's, if any.
fn skill_bonus_entries<'a>(
    base: &'a ItemBase,
    equipped: Option<&'a EquippedItem>,
) -> Vec<(&'a String, &'a hsplanner_engine::calc::types::RangedValue)> {
    let mut entries: Vec<_> = skill_bonus_keys(base)
        .into_iter()
        .map(|name| {
            (
                name,
                &base.skill_bonuses.as_ref().expect("keys come from bonuses")[name],
            )
        })
        .collect();
    if let Some(runeword) = equipped
        .and_then(|item| runeword_for(base, Some(item)))
        .and_then(|rw| rw.skill_bonuses.as_ref())
    {
        let mut extra: Vec<_> = runeword.iter().collect();
        extra.sort_by(|a, b| a.0.cmp(b.0));
        entries.extend(extra);
    }
    entries
}

pub(crate) fn skill_bonus_keys(base: &ItemBase) -> Vec<&String> {
    base.skill_bonuses.as_ref().map_or_else(Vec::new, |values| {
        ordered_keys(
            values,
            ITEM_STAT_ORDER.get(&base.id).map(|o| &o.skill_bonuses),
        )
    })
}

const RANDOM_SKILL_NAME: &str = "Random Skill";
const RANDOM_ELEMENT_KEY: &str = "random_skill_element";
const ALL_SKILLS_CLASS_KEY: &str = "all_skills_class";
const SUBSKILL_BOOST_KEY: &str = "subskill_boost";
const UNHOLY_EFFECT: &str = "Unholy";
const UNHOLY_GROUP: &str = "random_unholy";
const BONUS_SOCKET_MOD_ID: &str = "crystal_add_socket";
const EQUIPPED_MARK: &str = "✓";
const NOT_SUPPORTED_FOOTNOTE: &str = "These mods are not yet calculated by the planner.";
const RECOGNIZED_EFFECTS: [&str; 10] = [
    "attacks can hit multiple enemies",
    "cannot be frozen",
    "unholy",
    "movement phasing",
    "piercing attack",
    "half freeze duration",
    "double jump",
    "herobound",
    "all skills class",
    "mirrors your other ring",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineStyle {
    Implicit,
    Affix,
    Unholy,
    UnholyMissing,
    Runeword,
    Forged,
    Socket,
    SetActive,
    SetInactive,
    SetItems,
    Proc,
    Special,
    Unsupported,
    Muted,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeaderTone {
    Gold,
    Orange,
    Red,
    Pink,
    Green,
    Muted,
}

pub(crate) enum Line {
    Row {
        label: String,
        value: String,
    },
    Text {
        text: String,
        style: LineStyle,
        custom: bool,
    },
    Entry {
        title: String,
        style: LineStyle,
        suffix: Option<String>,
        desc: Option<String>,
        icon: Option<Arc<RenderImage>>,
        lines: Vec<String>,
    },
}

pub(crate) struct Section {
    header: Option<(String, HeaderTone, Option<String>)>,
    lines: Vec<Line>,
    footnote: Option<&'static str>,
}

impl Section {
    fn plain(lines: Vec<Line>) -> Self {
        Self {
            header: None,
            lines,
            footnote: None,
        }
    }
    fn titled(text: impl Into<String>, tone: HeaderTone, lines: Vec<Line>) -> Self {
        Self {
            header: Some((text.into(), tone, None)),
            lines,
            footnote: None,
        }
    }
}

pub(crate) struct ItemTooltipModel {
    pub name: String,
    pub tone: String,
    pub type_line: String,
    pub image_id: String,
    pub sections: Vec<Section>,
    pub footer: Option<String>,
}

fn num(value: f64) -> String {
    let rounded = (value.abs() * 100.).round() / 100.;
    if rounded.fract() == 0. {
        format!("{rounded:.0}")
    } else {
        format!("{rounded}")
    }
}

fn plain(value: f64) -> String {
    let rounded = (value * 100.).round() / 100.;
    if rounded.fract() == 0. {
        format!("{rounded:.0}")
    } else {
        format!("{rounded}")
    }
}

fn is_percent(key: &str) -> bool {
    data::game_config()
        .stats
        .iter()
        .find(|s| s.key == key)
        .is_some_and(|s| s.format.as_deref() == Some("percent"))
}

// `formatValue`: "+12%", "+[3-5]" for ranges; an empty key means no suffix.
pub(crate) fn format_ranged(value: Ranged, key: &str) -> String {
    let suffix = if !key.is_empty() && is_percent(key) {
        "%"
    } else {
        ""
    };
    let (min, max) = value;
    let sign = if min >= 0. { "+" } else { "" };
    if min == max {
        return format!("{sign}{}{suffix}", plain(min));
    }
    format!("{sign}[{}-{}]{suffix}", plain(min), plain(max))
}

fn is_zero(value: Ranged) -> bool {
    value.0 == 0. && value.1 == 0.
}

fn range_token(min: f64, max: f64) -> String {
    if num(min) == num(max) {
        num(min)
    } else {
        format!("[{}-{}]", num(min), num(max))
    }
}

fn describe_affix_value(affix: &Affix, value: f64) -> Option<String> {
    let (min, max) = affix.value_min.zip(affix.value_max)?;
    let token = range_token(min, max);
    affix
        .description
        .contains(&token)
        .then(|| affix.description.replacen(&token, &num(value), 1))
}

// `descriptionWithoutValue`: strip a leading "+[1-3]% " or "-12 " style value.
pub(crate) fn description_without_value(description: &str) -> String {
    let rest = description.trim_start_matches(['+', '-']);
    let rest = if let Some(after) = rest.strip_prefix('[') {
        after.split_once(']').map(|(_, tail)| tail).unwrap_or(rest)
    } else if rest.starts_with(|c: char| c.is_ascii_digit()) {
        rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
    } else {
        return description.split_whitespace().collect::<Vec<_>>().join(" ");
    };
    rest.trim_start_matches('%')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn format_affix_value(affix: &Affix, value: f64) -> String {
    let display_minus =
        affix.description.trim_start().starts_with('-') || matches!(affix.sign, AffixSign::Minus);
    let sign = if value < 0. || display_minus {
        "-"
    } else {
        "+"
    };
    let suffix = if matches!(affix.format, AffixFormat::Percent) {
        "%"
    } else {
        ""
    };
    format!("{sign}{}{suffix}", num(value))
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

fn skill_name(id: &str) -> Option<String> {
    data::skill_name_by_id(id).map(str::to_owned)
}

fn random_skill_label(skill_id: Option<&str>) -> String {
    skill_id
        .and_then(skill_name)
        .unwrap_or_else(|| format!("{RANDOM_SKILL_NAME} (not rolled)"))
}

fn random_element_label(element: Option<&str>) -> String {
    match element {
        Some(element) => format!("to {} Skills (random element)", capitalize(element)),
        None => "to Random Element Skills (not rolled)".into(),
    }
}

fn all_skills_class_label(class_id: Option<&str>) -> String {
    match class_id.and_then(data::get_class) {
        Some(class) => format!("to All Skills ({})", class.name),
        None if class_id.is_some() => "to All Skills (Class)".into(),
        None => "to All Skills (Class) (not rolled)".into(),
    }
}

fn subskill_boost_label(skill_id: Option<&str>) -> String {
    match skill_id.and_then(skill_name) {
        Some(name) => format!("to {name} Sub Skills"),
        None if skill_id.is_some() => "to Random Skill Sub Skills".into(),
        None => "to Random Skill Sub Skills (not rolled)".into(),
    }
}

fn trigger_label(trigger: &str) -> Option<&'static str> {
    Some(match trigger {
        "on_hit" => "on Hit",
        "on_attack" => "when Attacking",
        "when_struck" => "when Struck",
        "on_kill" => "on Kill",
        "on_cast" => "on Cast",
        "on_block" => "on Block",
        "on_death" => "on Death",
        "aura" => "Aura:",
        _ => return None,
    })
}

fn stars_for(base: &ItemBase, equipped: Option<&EquippedItem>) -> Option<u32> {
    if !data::can_star_forge(&base.slot, &base.rarity) {
        return None;
    }
    equipped.and_then(|item| item.stars)
}

pub(crate) fn runeword_for(
    base: &ItemBase,
    equipped: Option<&EquippedItem>,
) -> Option<&'static hsplanner_engine::calc::types::Runeword> {
    let socketed: Vec<Option<&str>> = equipped?.socketed.iter().map(|id| id.as_deref()).collect();
    data::detect_runeword(base, &socketed)
}

fn stat_line(key: &str, value: Ranged) -> String {
    format!("{} {}", format_ranged(value, key), stat_name(key))
}

pub(crate) fn build_model(
    base: &ItemBase,
    equipped: Option<&EquippedItem>,
    equipped_ids: &[String],
) -> ItemTooltipModel {
    let runeword = runeword_for(base, equipped);
    let tone = if runeword.is_some() {
        "rare".to_owned()
    } else {
        base.rarity.clone()
    };
    let stars = stars_for(base, equipped);
    let scale_implicit = runeword.is_none();
    let mut sections = Vec::new();

    let mut rows = Vec::new();
    if let Some((min, max)) = base.defense_min.zip(base.defense_max) {
        rows.push(Line::Row {
            label: "Defense".into(),
            value: format!("{}–{}", num(min), num(max)),
        });
    }
    if let Some((min, max)) = base.damage_min.zip(base.damage_max) {
        rows.push(Line::Row {
            label: "Damage".into(),
            value: format!("{}–{}", num(min), num(max)),
        });
    }
    if let Some(block) = base.block_chance {
        rows.push(Line::Row {
            label: "Block".into(),
            value: format!("{}%", num(block)),
        });
    }
    if let Some(speed) = base.attack_speed {
        let (min, max) = speed.as_ranged();
        rows.push(Line::Row {
            label: "Attacks / sec".into(),
            value: if min == max {
                num(min)
            } else {
                format!("{}-{}", num(min), num(max))
            },
        });
    }
    if !rows.is_empty() {
        sections.push(Section::plain(rows));
    }

    let overrides = equipped.map(|item| &item.implicit_overrides);
    let mut implicit_values: Vec<(String, Ranged, bool)> = Vec::new();
    if let Some(implicit) = &base.implicit {
        for key in implicit_keys(base) {
            if let Some(custom) = overrides.and_then(|o| o.get(key)) {
                if *custom != 0. {
                    implicit_values.push((key.clone(), (*custom, *custom), true));
                }
                continue;
            }
            let stat_key = if key == RANDOM_ELEMENT_KEY {
                equipped
                    .and_then(|item| item.random_skill_element.as_deref())
                    .map(|element| format!("{element}_skills"))
                    .unwrap_or_else(|| key.clone())
            } else {
                key.clone()
            };
            let value = implicit[key].as_ranged();
            let shown = if scale_implicit {
                apply_stars_to_ranged_value(value, &stat_key, stars)
            } else {
                value
            };
            if !is_zero(shown) {
                implicit_values.push((key.clone(), shown, false));
            }
        }
    }
    if let Some(overrides) = overrides {
        let mut extra: Vec<(&String, &f64)> = overrides
            .iter()
            .filter(|(key, _)| base.implicit.as_ref().is_none_or(|i| !i.contains_key(*key)))
            .collect();
        extra.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in extra {
            if *value != 0. {
                implicit_values.push((key.clone(), (*value, *value), true));
            }
        }
    }
    let mut implicit_lines: Vec<Line> = implicit_values
        .into_iter()
        .map(|(key, value, custom)| {
            let text = match key.as_str() {
                RANDOM_ELEMENT_KEY => format!(
                    "{} {}",
                    format_ranged(value, ""),
                    random_element_label(equipped.and_then(|i| i.random_skill_element.as_deref()))
                ),
                ALL_SKILLS_CLASS_KEY => format!(
                    "{} {}",
                    format_ranged(value, ""),
                    all_skills_class_label(equipped.and_then(|i| i.all_skills_class_id.as_deref()))
                ),
                SUBSKILL_BOOST_KEY => format!(
                    "{} {}",
                    format_ranged(value, ""),
                    subskill_boost_label(
                        equipped.and_then(|i| i.subskill_boost_skill_id.as_deref())
                    )
                ),
                _ => stat_line(&key, value),
            };
            Line::Text {
                text,
                style: LineStyle::Implicit,
                custom,
            }
        })
        .collect();

    let granted = granted_skill_entries(base, equipped, stars);
    let granted_names: HashSet<String> = granted
        .iter()
        .map(|(skill, _, _)| skill.name.trim().to_lowercase())
        .collect();
    {
        for (name, value) in skill_bonus_entries(base, equipped) {
            if granted_names.contains(&name.trim().to_lowercase()) {
                continue;
            }
            let custom = equipped.and_then(|i| i.skill_bonus_overrides.get(name).copied());
            let label = if name == RANDOM_SKILL_NAME {
                random_skill_label(equipped.and_then(|i| i.random_skill_id.as_deref()))
            } else {
                name.clone()
            };
            let star_locked = data::get_item_granted_skill_by_name(name)
                .is_some_and(|skill| skill.star_rank_locked);
            let shown = match custom {
                Some(value) => (value, value),
                None => apply_stars_to_ranged_value(
                    value.as_ranged(),
                    "item_granted_skill_rank",
                    if star_locked { None } else { stars },
                ),
            };
            if is_zero(shown) {
                continue;
            }
            implicit_lines.push(Line::Text {
                text: format!("{} to {label}", format_ranged(shown, "")),
                style: LineStyle::Implicit,
                custom: custom.is_some(),
            });
        }
    }
    if let Some(runeword) = runeword {
        let mut stats: Vec<(&String, &f64)> =
            runeword.stats.iter().filter(|(_, v)| **v != 0.).collect();
        stats.sort_by(|a, b| a.0.cmp(b.0));
        let lines: Vec<Line> = stats
            .into_iter()
            .map(|(key, value)| Line::Text {
                text: stat_line(key, (*value, *value)),
                style: LineStyle::Runeword,
                custom: false,
            })
            .chain(runeword.description.iter().map(|text| Line::Text {
                text: text.clone(),
                style: LineStyle::Runeword,
                custom: false,
            }))
            .collect();
        if !lines.is_empty() {
            implicit_lines.extend(lines);
        }
    }

    if !implicit_lines.is_empty() {
        sections.push(Section::titled(
            "Implicit",
            HeaderTone::Gold,
            implicit_lines,
        ));
    }

    if !granted.is_empty() {
        sections.push(Section::titled(
            "Granted Skill Effects",
            HeaderTone::Orange,
            granted
                .into_iter()
                .map(|(skill, rank, lines)| Line::Entry {
                    title: skill.name.clone(),
                    style: LineStyle::Implicit,
                    suffix: Some(format!("rank {rank}")),
                    desc: skill
                        .description
                        .clone()
                        .filter(|text| !text.trim().is_empty()),
                    icon: None,
                    lines,
                })
                .collect(),
        ));
    }

    let (supported, unsupported, unholy) = affix_lines(
        equipped.map(|i| i.affixes.as_slice()).unwrap_or_default(),
        stars,
        equipped.and_then(|i| i.random_skill_element.as_deref()),
    );
    if !supported.is_empty() {
        sections.push(Section::titled("Affixes", HeaderTone::Gold, supported));
    }
    let unholy_slots = base
        .unique_effects
        .iter()
        .flatten()
        .filter(|e| e.trim() == UNHOLY_EFFECT)
        .count();
    let unrolled = unholy_slots.saturating_sub(unholy.len());
    if !unholy.is_empty() || unrolled > 0 {
        let mut lines = unholy;
        lines.extend((0..unrolled).map(|_| Line::Text {
            text: "Unholy (not rolled)".into(),
            style: LineStyle::UnholyMissing,
            custom: false,
        }));
        sections.push(Section::titled("Unholy Affixes", HeaderTone::Pink, lines));
    }

    if let Some(item) = equipped
        && data::forge_kind_for(&base.rarity).is_some()
    {
        let forged: Vec<Line> = item
            .forged_mods
            .iter()
            .filter_map(|eq| data::get_crystal_mod(&eq.affix_id).map(|m| (eq, m)))
            .map(|(eq, m)| Line::Text {
                text: eq
                    .custom_value
                    .and_then(|value| describe_affix_value(m, value))
                    .unwrap_or_else(|| m.description.clone()),
                style: LineStyle::Forged,
                custom: eq.custom_value.is_some(),
            })
            .collect();
        if !forged.is_empty() {
            sections.push(Section::titled(
                "Forged · Satanic Crystal",
                HeaderTone::Red,
                forged,
            ));
        }
    }

    if let Some(item) = equipped {
        let groups = socket_groups(item, base);
        if !groups.is_empty() {
            sections.push(Section::titled(
                "From Sockets",
                HeaderTone::Gold,
                groups
                    .into_iter()
                    .map(|(name, count, stats)| Line::Entry {
                        title: if count > 1 {
                            format!("{name} ×{count}")
                        } else {
                            name.clone()
                        },
                        style: LineStyle::Socket,
                        suffix: None,
                        desc: None,
                        icon: socketable_icon(&name),
                        lines: stats
                            .into_iter()
                            .map(|(key, value)| stat_line(&key, (value, value)))
                            .collect(),
                    })
                    .collect(),
            ));
        }
        if let Some(augment_ref) = &item.augment
            && let Some(augment) = data::get_augment(&augment_ref.id)
        {
            let index =
                (augment_ref.level.max(1) as usize - 1).min(augment.levels.len().saturating_sub(1));
            if let Some(tier) = augment.levels.get(index) {
                let mut stats: Vec<(&String, &f64)> =
                    tier.stats.iter().filter(|(_, v)| **v != 0.).collect();
                stats.sort_by(|a, b| a.0.cmp(b.0));
                sections.push(Section::titled(
                    "Angelic Augment",
                    HeaderTone::Gold,
                    vec![Line::Entry {
                        title: augment.name.clone(),
                        style: LineStyle::Implicit,
                        suffix: Some(format!("level {}", augment_ref.level)),
                        desc: None,
                        icon: augment_icon(&augment.id),
                        lines: stats
                            .into_iter()
                            .map(|(key, value)| stat_line(key, (*value, *value)))
                            .collect(),
                    }],
                ));
            }
        }
    }

    if let Some(set) = base.set_id.as_deref().and_then(data::get_set)
        && !set.bonuses.is_empty()
    {
        let equipped_count = equipped_ids
            .iter()
            .filter(|id| data::get_item(id).is_some_and(|b| b.set_id.as_deref() == Some(&set.id)))
            .count() as u32;
        let mut lines: Vec<Line> = set
            .bonuses
            .iter()
            .map(|bonus| {
                let active = equipped_count >= bonus.pieces;
                Line::Entry {
                    title: if active {
                        format!("{}-Set (active)", bonus.pieces)
                    } else {
                        format!("{}-Set", bonus.pieces)
                    },
                    style: if active {
                        LineStyle::SetActive
                    } else {
                        LineStyle::SetInactive
                    },
                    suffix: None,
                    desc: None,
                    icon: None,
                    lines: bonus.descriptions.clone().unwrap_or_default(),
                }
            })
            .collect();
        lines.push(Line::Entry {
            title: "Set items".into(),
            style: LineStyle::SetItems,
            suffix: None,
            desc: None,
            icon: None,
            lines: set
                .items
                .iter()
                .map(|piece| {
                    let mark = if equipped_ids.contains(&piece.item_id) {
                        EQUIPPED_MARK
                    } else {
                        "·"
                    };
                    format!("{mark} {} ({})", piece.name, piece.slot)
                })
                .collect(),
        });
        sections.push(Section {
            header: Some((
                set.name.clone(),
                HeaderTone::Green,
                Some(format!("{equipped_count}/{} pieces", set.items.len())),
            )),
            lines,
            footnote: None,
        });
    }

    if let Some(procs) = base.procs.as_ref().filter(|p| !p.is_empty()) {
        sections.push(Section::plain(
            procs
                .iter()
                .map(|proc| {
                    let mut title = match trigger_label(&proc.trigger) {
                        Some(label) => format!("{}% Chance {label}", num(proc.chance)),
                        None => format!("{}% {}", num(proc.chance), proc.trigger.replace('_', " ")),
                    };
                    if let Some(description) = &proc.description {
                        title = match trigger_label(&proc.trigger) {
                            Some(label) => {
                                format!("{}% Chance {label} to {description}", num(proc.chance))
                            }
                            None => format!("{}% {description}", num(proc.chance)),
                        };
                    } else {
                        match (&proc.target, proc.cast_level) {
                            (Some(target), Some(level)) => {
                                title.push_str(&format!(" to cast level {level} {target}"))
                            }
                            (Some(target), None) => title.push_str(&format!(" to cast {target}")),
                            _ => {}
                        }
                    }
                    Line::Entry {
                        title,
                        style: LineStyle::Proc,
                        suffix: None,
                        desc: proc.details.clone(),
                        icon: None,
                        lines: Vec::new(),
                    }
                })
                .collect(),
        ));
    }

    let effects: Vec<&String> = base
        .unique_effects
        .iter()
        .flatten()
        .filter(|e| e.trim() != UNHOLY_EFFECT)
        .collect();
    let recognized =
        |effect: &str| RECOGNIZED_EFFECTS.contains(&effect.trim().to_lowercase().as_str());
    let special: Vec<Line> = effects
        .iter()
        .filter(|e| recognized(e))
        .map(|e| Line::Text {
            text: (*e).clone(),
            style: LineStyle::Special,
            custom: false,
        })
        .collect();
    if !special.is_empty() {
        sections.push(Section::titled(
            "Special Effects",
            HeaderTone::Gold,
            special,
        ));
    }
    let mut not_supported = unsupported;
    not_supported.extend(
        effects
            .iter()
            .filter(|e| !recognized(e))
            .map(|e| Line::Text {
                text: (*e).clone(),
                style: LineStyle::Unsupported,
                custom: false,
            }),
    );
    if !not_supported.is_empty() {
        sections.push(Section {
            header: Some(("Not Yet Supported".into(), HeaderTone::Muted, None)),
            lines: not_supported,
            footnote: Some(NOT_SUPPORTED_FOOTNOTE),
        });
    }

    let description: Vec<Line> = [&base.description, &base.flavor]
        .into_iter()
        .flatten()
        .map(|text| Line::Text {
            text: text.clone(),
            style: LineStyle::Muted,
            custom: false,
        })
        .collect();
    if !description.is_empty() {
        sections.push(Section::plain(description));
    }

    let mut footer = Vec::new();
    if let Some(level) = runeword
        .and_then(|r| r.requires_level)
        .or(base.requires_level)
    {
        footer.push(format!("Req Level {level}"));
    }
    if let Some(level) = base.item_level {
        footer.push(format!("iLvl {level}"));
    }
    if let Some(grade) = &base.grade {
        footer.push(format!("Tier {grade}"));
    }

    ItemTooltipModel {
        name: display_name(base, equipped, runeword.map(|r| r.name.as_str())),
        tone,
        type_line: type_line(base, equipped, stars, runeword.is_some()),
        image_id: base.id.clone(),
        sections,
        footer: (!footer.is_empty()).then(|| footer.join(" · ")),
    }
}

fn type_line(
    base: &ItemBase,
    equipped: Option<&EquippedItem>,
    stars: Option<u32>,
    runeword: bool,
) -> String {
    let mut line = format!(
        "{} · {}",
        if runeword {
            "Runeword".to_owned()
        } else {
            rarity_label(&base.rarity)
        },
        base.base_type
    );
    if base.slot == "weapon" {
        line.push_str(if base.two_handed.unwrap_or(false) {
            " · 2-Handed"
        } else {
            " · 1-Handed"
        });
    }
    if let Some(stars) = stars.filter(|stars| *stars > 0) {
        line.push_str(&format!(" · {}", "★".repeat(stars as usize)));
    }
    if equipped.is_some_and(|item| {
        item.forged_mods
            .iter()
            .any(|m| m.affix_id == BONUS_SOCKET_MOD_ID)
    }) {
        line.push_str(" · Tinkered");
    }
    line
}

fn display_name(
    base: &ItemBase,
    equipped: Option<&EquippedItem>,
    runeword: Option<&str>,
) -> String {
    if let Some(name) = runeword {
        return name.to_owned();
    }
    let gems: Vec<String> = equipped
        .zip(base.socket_transforms.as_ref())
        .map(|(item, transforms)| {
            item.socketed
                .iter()
                .flatten()
                .filter(|id| transforms.contains_key(*id))
                .filter_map(|id| data::get_gem(id).map(|gem| gem.name.clone()))
                .collect()
        })
        .unwrap_or_default();
    if gems.is_empty() {
        base.name.clone()
    } else {
        format!("{} ({})", base.name, gems.join(" + "))
    }
}

type GrantedEntry = (
    &'static hsplanner_engine::calc::types::ItemGrantedSkill,
    String,
    Vec<String>,
);

fn granted_skill_entries(
    base: &ItemBase,
    equipped: Option<&EquippedItem>,
    stars: Option<u32>,
) -> Vec<GrantedEntry> {
    let mut out = Vec::new();
    for (name, value) in skill_bonus_entries(base, equipped) {
        if name == RANDOM_SKILL_NAME {
            continue;
        }
        let Some(skill) = data::get_item_granted_skill_by_name(name) else {
            continue;
        };
        let (min, max) = match equipped.and_then(|i| i.skill_bonus_overrides.get(name)) {
            Some(value) => (*value, *value),
            None => apply_stars_to_ranged_value(
                value.as_ranged(),
                "item_granted_skill_rank",
                if skill.star_rank_locked { None } else { stars },
            ),
        };
        let (rank_min, rank_max) = (min.round(), max.round());
        if rank_max <= 0. {
            continue;
        }
        let display_rank = if rank_min == rank_max {
            num(rank_min)
        } else {
            format!("{}-{}", num(rank_min), num(rank_max))
        };
        let mut lines = Vec::new();
        if let Some(converts) = &skill.passive_converts {
            for convert in &converts.per_rank {
                let pct =
                    |rank: f64| ((convert.base_pct + convert.pct * rank) * 100.).round() / 100.;
                let (a, b) = (pct(rank_min), pct(rank_max));
                let pct_text = if a == b {
                    format!("{}%", num(a))
                } else {
                    format!("{}–{}%", num(a), num(b))
                };
                let verb = if convert.replaces {
                    "converted to"
                } else {
                    "added as"
                };
                lines.push(format!(
                    "{pct_text} of {} {verb} {}",
                    stat_name(&convert.from),
                    stat_name(&convert.to)
                ));
            }
        }
        if let Some(stats) = &skill.passive_stats {
            let mut totals: BTreeMap<&String, Ranged> = BTreeMap::new();
            for (key, value) in stats.base.iter().flatten() {
                totals.insert(key, (*value, *value));
            }
            for (key, value) in stats.per_rank.iter().flatten() {
                let current = totals.get(key).copied().unwrap_or((0., 0.));
                totals.insert(
                    key,
                    (current.0 + value * rank_min, current.1 + value * rank_max),
                );
            }
            for (key, value) in totals {
                if !is_zero(value) {
                    lines.push(stat_line(key, value));
                }
            }
        }
        out.push((skill, display_rank, lines));
    }
    out
}

fn affix_lines(
    affixes: &[EquippedAffix],
    stars: Option<u32>,
    random_element: Option<&str>,
) -> (Vec<Line>, Vec<Line>, Vec<Line>) {
    let (mut supported, mut unsupported, mut unholy) = (Vec::new(), Vec::new(), Vec::new());
    for eq in affixes {
        let Some(affix) = data::get_affix(&eq.affix_id) else {
            continue;
        };
        let is_unholy = affix.group_id == UNHOLY_GROUP;
        let style = if is_unholy {
            LineStyle::Unholy
        } else if affix.stat_key.is_some() {
            LineStyle::Affix
        } else {
            LineStyle::Unsupported
        };
        let value = eq
            .custom_value
            .unwrap_or_else(|| rolled_affix_value_with_stars(affix, eq.roll, stars));
        let text = if affix.stat_key.as_deref() == Some(RANDOM_ELEMENT_KEY) {
            format!(
                "{} {}",
                format_ranged((value, value), ""),
                random_element_label(random_element)
            )
        } else if let Some(described) = describe_affix_value(affix, value) {
            described
        } else {
            let shown = match &affix.stat_key {
                Some(key) => format_ranged((value, value), key),
                None => format_affix_value(affix, value),
            };
            format!("{shown} {}", description_without_value(&affix.description))
                .trim()
                .to_owned()
        };
        let line = Line::Text {
            text,
            style,
            custom: eq.custom_value.is_some(),
        };
        if is_unholy {
            unholy.push(line);
        } else if affix.stat_key.is_some() {
            supported.push(line);
        } else {
            unsupported.push(line);
        }
    }
    (supported, unsupported, unholy)
}

type SocketGroup = (String, u32, Vec<(String, f64)>);

// Sockets holding the same gem or rune merge into one group, like the reference.
fn socket_groups(item: &EquippedItem, base: &ItemBase) -> Vec<SocketGroup> {
    let mut groups: BTreeMap<&String, (String, u32, BTreeMap<String, f64>)> = BTreeMap::new();
    for (index, id) in item.socketed.iter().enumerate() {
        let Some(id) = id else { continue };
        let (name, stats) = match (data::get_gem(id), data::get_rune(id)) {
            (Some(gem), _) => (&gem.name, &gem.stats),
            (None, Some(rune)) => (&rune.name, &rune.stats),
            _ => continue,
        };
        let rainbow = base
            .rainbow_sockets
            .as_ref()
            .is_some_and(|slots| slots.contains(&(index as u32 + 1)))
            || item.socket_types.get(index) == Some(&SocketType::Rainbow);
        let multiplier = if rainbow { RAINBOW_MULTIPLIER } else { 1. };
        let source = base
            .socket_transforms
            .as_ref()
            .and_then(|t| t.get(id))
            .unwrap_or(stats);
        let group = groups
            .entry(id)
            .or_insert_with(|| (name.clone(), 0, BTreeMap::new()));
        group.1 += 1;
        for (key, value) in source {
            *group.2.entry(key.clone()).or_default() += value * multiplier;
        }
    }
    groups
        .into_values()
        .map(|(name, count, stats)| {
            let stats: Vec<(String, f64)> = stats.into_iter().filter(|(_, v)| *v != 0.).collect();
            (name, count, stats)
        })
        .filter(|(_, _, stats)| !stats.is_empty())
        .collect()
}

pub(crate) fn tone_color(tone: &str, cx: &App) -> Hsla {
    if tone == "neutral" {
        cx.global::<TooltipTheme>().neutral
    } else {
        theme::rarity_color(tone, cx)
    }
}

fn header_tone_color(tone: HeaderTone, cx: &App) -> Hsla {
    let p = cx.global::<TooltipTheme>();
    match tone {
        HeaderTone::Gold => p.accent_hot.opacity(0.85),
        HeaderTone::Orange => p.stat_orange,
        HeaderTone::Red => p.negative,
        HeaderTone::Pink => theme::rarity_color("unholy", cx),
        HeaderTone::Green => p.positive,
        HeaderTone::Muted => p.muted,
    }
}

fn line_color(style: LineStyle, cx: &App) -> Hsla {
    let p = cx.global::<TooltipTheme>();
    match style {
        LineStyle::Implicit | LineStyle::Runeword | LineStyle::Special => p.accent_hot,
        LineStyle::Affix => p.angelic,
        LineStyle::Unholy | LineStyle::UnholyMissing => theme::rarity_color("unholy", cx),
        LineStyle::Forged => p.negative,
        LineStyle::Socket => p.accent,
        LineStyle::SetActive | LineStyle::Proc => p.positive,
        LineStyle::SetInactive | LineStyle::SetItems => p.muted.opacity(0.7),
        LineStyle::Unsupported => p.angelic.opacity(0.7),
        LineStyle::Muted => p.muted,
    }
}

pub(crate) struct ItemTooltip {
    model: ItemTooltipModel,
}

impl ItemTooltip {
    pub(crate) fn new(model: ItemTooltipModel) -> Self {
        Self { model }
    }
}

fn section_header(text: &str, tone: HeaderTone, trailing: Option<&str>, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let color = header_tone_color(tone, cx);
    div()
        .mx(units(-12.))
        .mt(units(-8.))
        .mb_2()
        .px_3()
        .py_1()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .border_b_1()
        .border_color(p.border.opacity(0.4))
        .bg(color.opacity(0.1))
        .text_size(units(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color)
        .child(TooltipText::new(
            SharedString::from(format!("tooltip-section-{text}")),
            text.to_uppercase(),
            0.12,
        ))
        .children(trailing.map(|trailing| {
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .font_weight(FontWeight::NORMAL)
                .text_color(p.text.opacity(0.7))
                .child(trailing.to_owned())
        }))
}

fn render_rows(lines: &[Line], cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .flex()
        .flex_col()
        .gap_0p5()
        .text_size(units(12.))
        .children(lines.iter().filter_map(|line| {
            match line {
                Line::Row { label, value } => Some(
                    div()
                        .min_w_0()
                        .flex()
                        .items_start()
                        .justify_between()
                        .gap_4()
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .whitespace_normal()
                                .text_color(p.muted)
                                .child(label.clone()),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .max_w_full()
                                .whitespace_normal()
                                .text_right()
                                .font_weight(FontWeight::MEDIUM)
                                .font_family(theme::MONO_FONT_FAMILY)
                                .text_color(p.text)
                                .child(value.clone()),
                        ),
                ),
                _ => None,
            }
        }))
}

fn render_text(text: &str, style: LineStyle, custom: bool, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .min_w_0()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap_1()
        .text_size(units(if style == LineStyle::Muted { 11. } else { 12. }))
        .text_color(line_color(style, cx))
        .when(
            matches!(style, LineStyle::Muted | LineStyle::UnholyMissing),
            |v| v.italic(),
        )
        .child(
            div()
                .min_w_0()
                .max_w_full()
                .whitespace_normal()
                .child(text.to_owned()),
        )
        .when(custom, |v| {
            v.child(
                div()
                    .min_w_0()
                    .max_w_full()
                    .whitespace_normal()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(10.))
                    .text_color(p.accent_hot.opacity(0.7))
                    .child("CUSTOM"),
            )
        })
}

fn render_entry(
    title: &str,
    style: LineStyle,
    suffix: Option<&str>,
    desc: Option<&str>,
    icon: Option<Arc<RenderImage>>,
    lines: &[String],
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    let set_items = style == LineStyle::SetItems;
    let set_like = matches!(
        style,
        LineStyle::SetActive | LineStyle::SetInactive | LineStyle::SetItems
    );
    let title_color = match style {
        LineStyle::Socket => p.muted,
        LineStyle::Proc | LineStyle::SetActive | LineStyle::SetInactive | LineStyle::SetItems => {
            line_color(style, cx)
        }
        _ => p.accent_hot,
    };
    let body_color = match style {
        LineStyle::Socket => p.accent_hot,
        LineStyle::SetActive | LineStyle::SetInactive | LineStyle::SetItems => {
            line_color(style, cx)
        }
        _ => p.text.opacity(0.8),
    };
    let sprite = |icon: Arc<RenderImage>| {
        img(icon)
            .flex_shrink_0()
            .size(units(18.))
            .object_fit(ObjectFit::Contain)
    };
    // Socketables sit beside the whole entry; other sprites lead the title, like the reference.
    let (leading, inline) = match (style, icon) {
        (LineStyle::Socket, Some(icon)) => (Some(sprite(icon)), None),
        (_, Some(icon)) => (None, Some(sprite(icon))),
        (_, None) => (None, None),
    };
    let body = div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .child(
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap_1p5()
                .text_size(units(if set_like { 10. } else { 12. }))
                .text_color(title_color)
                .when(set_like, |v| v.font_family(theme::MONO_FONT_FAMILY))
                .children(inline)
                .child(
                    div()
                        .min_w_0()
                        .max_w_full()
                        .whitespace_normal()
                        .child(if set_like {
                            title.to_uppercase()
                        } else {
                            title.to_owned()
                        }),
                )
                .children(suffix.map(|suffix| {
                    div()
                        .min_w_0()
                        .max_w_full()
                        .whitespace_normal()
                        .text_size(units(10.))
                        .text_color(p.muted)
                        .child(suffix.to_owned())
                })),
        )
        .children(desc.map(|desc| {
            div()
                .min_w_0()
                .max_w_full()
                .whitespace_normal()
                .text_size(units(10.))
                .italic()
                .line_height(relative(1.35))
                .text_color(p.muted)
                .child(desc.to_owned())
        }))
        .when(!lines.is_empty(), |v| {
            v.child(
                div()
                    .mt_0p5()
                    .ml(units(if set_like || leading.is_some() {
                        4.
                    } else {
                        8.
                    }))
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .text_size(units(11.))
                    .children(lines.iter().map(|line| {
                        let equipped = set_items && line.starts_with(EQUIPPED_MARK);
                        div()
                            .min_w_0()
                            .max_w_full()
                            .whitespace_normal()
                            .text_color(if equipped { p.positive } else { body_color })
                            .child(line.clone())
                    })),
            )
        });
    div()
        .flex()
        .items_start()
        .gap_1p5()
        .mb_1()
        .when(set_items, |v| {
            v.mt_2()
                .pt_2()
                .border_t_1()
                .border_color(p.text.opacity(0.1))
        })
        .children(leading.map(|icon| icon.mt_px()))
        .child(body)
}

fn render_lines(lines: &[Line], cx: &App) -> Div {
    if matches!(lines.first(), Some(Line::Row { .. })) {
        return render_rows(lines, cx);
    }
    div()
        .flex()
        .flex_col()
        .gap_0p5()
        .children(lines.iter().map(|line| match line {
            Line::Row { .. } => div(),
            Line::Text {
                text,
                style,
                custom,
            } => render_text(text, *style, *custom, cx),
            Line::Entry {
                title,
                style,
                suffix,
                desc,
                icon,
                lines,
            } => render_entry(
                title,
                *style,
                suffix.as_deref(),
                desc.as_deref(),
                icon.clone(),
                lines,
                cx,
            ),
        }))
}

fn render_panel(model: &ItemTooltipModel, embedded: bool, window: &Window, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let rem = window.rem_size();
    let tone = tone_color(&model.tone, cx);
    let mut panel = div()
        .flex()
        .flex_col()
        .when(!embedded, |v| {
            v.min_w(units(220.))
                .max_w(units(360.))
                .max_h(window.viewport_size().height - rem * (16. / 13.))
                .shadow(vec![
                    BoxShadow {
                        color: p.shadow.opacity(0.8),
                        offset: point(px(0.), rem * (8. / 13.)),
                        blur_radius: rem * (32. / 13.),
                        spread_radius: px(0.),
                        inset: false,
                    },
                    BoxShadow {
                        color: tone.opacity(0.3),
                        offset: point(px(0.), px(0.)),
                        blur_radius: rem * (24. / 13.),
                        spread_radius: rem * (-6. / 13.),
                        inset: false,
                    },
                ])
        })
        .when(embedded, |v| v.w_full().min_w_0())
        .bg(p.panel)
        .text_color(p.text)
        .rounded(units(4.))
        .border_1()
        .border_color(tone.opacity(0.6))
        .overflow_hidden()
        .child(
            div()
                .relative()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(p.border.opacity(0.7))
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(tone.opacity(0.14), 0.),
                    linear_color_stop(tone.opacity(0.04), 1.),
                ))
                .child(
                    div()
                        .min_w_0()
                        .w_full()
                        .pr(units(56.))
                        .child(
                            div()
                                .min_w_0()
                                .w_full()
                                .text_size(units(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .line_height(relative(1.25))
                                .text_color(tone)
                                .child({
                                    let text = TooltipText::new(
                                        "item-tooltip-title",
                                        model.name.clone(),
                                        0.02,
                                    )
                                    .glow(Some(tone));
                                    if embedded { text.wrap() } else { text }
                                }),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .w_full()
                                .mt_0p5()
                                .text_size(units(10.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.faint)
                                .child({
                                    let text = TooltipText::new(
                                        "item-tooltip-type",
                                        model.type_line.to_uppercase(),
                                        0.12,
                                    );
                                    if embedded { text.wrap() } else { text }
                                }),
                        ),
                )
                .children(item_icon(&model.image_id).map(|icon| {
                    div()
                        .absolute()
                        .right_2()
                        .top_0()
                        .bottom_0()
                        .w(units(48.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            img(icon)
                                .max_w_full()
                                .max_h(units(48.))
                                .object_fit(ObjectFit::Contain),
                        )
                })),
        );
    for (index, section) in model.sections.iter().enumerate() {
        let mut block = div().px_3().py_2().when(index > 0, |v| {
            v.border_t_1().border_color(p.border.opacity(0.7))
        });
        if let Some((text, tone, trailing)) = &section.header {
            block = block.child(section_header(text, *tone, trailing.as_deref(), cx));
        }
        block = block.child(render_lines(&section.lines, cx));
        if let Some(footnote) = section.footnote {
            block = block.child(
                div()
                    .mt_1()
                    .text_size(units(10.))
                    .italic()
                    .text_color(p.muted.opacity(0.7))
                    .child(footnote),
            );
        }
        panel = panel.child(block);
    }
    if let Some(footer) = &model.footer {
        panel = panel.child(
            div()
                .px_3()
                .py_1p5()
                .border_t_1()
                .border_color(p.border.opacity(0.7))
                .bg(p.panel_secondary)
                .text_size(units(10.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.faint)
                .child({
                    let text = TooltipText::new("item-tooltip-footer", footer.to_uppercase(), 0.12);
                    if embedded { text.wrap() } else { text }
                }),
        );
    }
    panel
}

impl Render for ItemTooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_panel(&self.model, false, window, cx)
    }
}

/// Inline card (no overlay chrome) for the comparison column; an empty slot shows a dashed box.
pub(crate) fn item_card(
    item: Option<&EquippedItem>,
    equipped_ids: &[String],
    window: &Window,
    cx: &App,
) -> Div {
    let p = cx.global::<TooltipTheme>();
    match item.and_then(|item| data::get_item(&item.base_id).map(|base| (item, base))) {
        Some((item, base)) => render_panel(
            &build_model(base, Some(item), equipped_ids),
            true,
            window,
            cx,
        ),
        None => div()
            .w_full()
            .rounded_sm()
            .border_1()
            .border_dashed()
            .border_color(p.border)
            .bg(p.panel)
            .px_3()
            .py_6()
            .flex()
            .justify_center()
            .text_size(units(11.))
            .italic()
            .text_color(p.faint)
            .child("empty slot"),
    }
}

/// Wraps an element so hovering it shows the item tooltip; `equipped_ids` feed set counts.
pub(crate) fn with_item_tooltip(
    id: impl Into<ElementId>,
    item: &EquippedItem,
    equipped_ids: Vec<String>,
    child: impl IntoElement,
) -> Stateful<Div> {
    let item = item.clone();
    div()
        .id(id)
        .flex()
        .cursor_tooltip_view(move |_, cx| {
            let Some(base) = data::get_item(&item.base_id) else {
                return cx.new(|_| EmptyTooltip).into();
            };
            let model = build_model(base, Some(&item), &equipped_ids);
            cx.new(|_| ItemTooltip::new(model)).into()
        })
        .child(child)
}

/// Like [`with_item_tooltip`] for a bare base (picker rows): no rolls, sockets or stars.
pub(crate) fn with_base_tooltip(
    id: impl Into<ElementId>,
    base: &'static ItemBase,
    equipped_ids: Vec<String>,
    child: impl IntoElement,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .cursor_tooltip_view(move |_, cx| {
            let model = build_model(base, None, &equipped_ids);
            cx.new(|_| ItemTooltip::new(model)).into()
        })
        .child(child)
}

struct EmptyTooltip;
impl Render for EmptyTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

pub(crate) fn equipped_ids(inventory: &hsplanner_engine::calc::types::Inventory) -> Vec<String> {
    inventory
        .values()
        .map(|item| item.base_id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn runeword_stats_precede_granted_skills_and_sockets() {
        let base = data::data()
            .items
            .values()
            .find(|base| base.rarity == "common" && base.base_type == "Armor")
            .unwrap();
        let item = EquippedItem {
            base_id: base.id.clone(),
            socketed: vec![Some("rune_io".into()), Some("rune_pul".into())],
            ..Default::default()
        };
        let model = build_model(base, Some(&item), &[]);
        let section_index = |name: &str| {
            model
                .sections
                .iter()
                .position(|section| {
                    section
                        .header
                        .as_ref()
                        .is_some_and(|(title, _, _)| title == name)
                })
                .unwrap()
        };
        let implicit = section_index("Implicit");
        let granted = section_index("Granted Skill Effects");
        let sockets = section_index("From Sockets");
        assert!(implicit < granted && granted < sockets);
        assert!(model.sections[implicit].lines.iter().any(|line| {
            matches!(line, Line::Text { text, .. } if text == "+225% Enhanced Defense")
        }));
        assert!(model.sections[granted].lines.iter().any(|line| {
            matches!(line, Line::Entry { title, suffix, lines, desc, .. }
                if title == "Angel’s Vibrance Aura"
                    && suffix.as_deref() == Some("rank 2-6") && !lines.is_empty() && desc.is_none())
        }));
    }

    #[::core::prelude::v1::test]
    fn deleted_authored_mod_is_hidden_without_restoring_base_value() {
        let base = data::get_item("axe_angelic_st_rexis_sundering_axe").unwrap();
        let mut item = EquippedItem {
            base_id: base.id.clone(),
            ..Default::default()
        };
        let has_damage = |model: ItemTooltipModel| {
            model.sections.iter().flat_map(|s| &s.lines).any(
                |line| matches!(line, Line::Text { text, .. } if text.contains("Enhanced Damage")),
            )
        };
        assert!(has_damage(build_model(base, Some(&item), &[])));
        item.implicit_overrides.insert("enhanced_damage".into(), 0.);
        assert!(!has_damage(build_model(base, Some(&item), &[])));
    }

    #[::core::prelude::v1::test]
    fn item_stat_order_matches_the_authored_axe_and_keeps_unknown_stats() {
        let base = data::get_item("axe_angelic_st_rexis_sundering_axe").unwrap();
        assert_eq!(
            implicit_keys(base)
                .into_iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec![
                "enhanced_damage",
                "all_skills",
                "increased_attack_speed",
                "crushing_blow_chance",
                "life_steal",
                "defense_ignored",
                "to_strength",
                "life_per_kill",
                "max_crushing_blow_stacks",
            ]
        );
        let values = HashMap::from([("extra_z".into(), 1), ("extra_a".into(), 2)]);
        assert_eq!(
            ordered_keys(&values, Some(&ITEM_STAT_ORDER[&base.id].implicit)),
            vec!["extra_a", "extra_z"]
        );
    }

    #[::core::prelude::v1::test]
    fn skill_bonus_metadata_preserves_source_order_without_reading_values() {
        let order: ItemStatOrder = serde_json::from_str(
            r#"{"id":"ordered-skills","skillBonuses":{"Zeal":[1,4],"Arc":2}}"#,
        )
        .unwrap();
        let values = HashMap::from([("Arc".into(), 2), ("Zeal".into(), 3)]);
        assert_eq!(
            ordered_keys(&values, Some(&order.skill_bonuses)),
            vec!["Zeal", "Arc"]
        );
        assert_eq!(ITEM_STAT_ORDER.len(), data::data().items.len());
    }

    #[::core::prelude::v1::test]
    fn value_formatting_matches_the_reference() {
        assert_eq!(format_ranged((12., 12.), ""), "+12");
        assert_eq!(format_ranged((3., 5.), ""), "+[3-5]");
        assert_eq!(format_ranged((-4., -4.), ""), "-4");
        assert_eq!(
            description_without_value("+[1-3]% Chance to Block"),
            "Chance to Block"
        );
        assert_eq!(description_without_value("-12 Life"), "Life");
        assert_eq!(
            description_without_value("Cannot be frozen"),
            "Cannot be frozen"
        );
        assert_eq!(range_token(1., 3.), "[1-3]");
        assert_eq!(range_token(2., 2.), "2");
    }

    #[::core::prelude::v1::test]
    fn affix_descriptions_swap_the_range_token_for_the_roll() {
        let affix = Affix {
            description: "+[10-20]% Enhanced Damage".into(),
            value_min: Some(10.),
            value_max: Some(20.),
            ..Default::default()
        };
        assert_eq!(
            describe_affix_value(&affix, 15.),
            Some("+15% Enhanced Damage".into())
        );
        assert_eq!(format_affix_value(&affix, 15.), "+15");
        let minus = Affix {
            description: "-[5-10]% to Enemy Cold Resistance".into(),
            format: AffixFormat::Percent,
            ..Default::default()
        };
        assert_eq!(format_affix_value(&minus, 7.), "-7%");
    }
}
