//! OCR'd tooltip lines → equipped item. Sits next to ocr.rs so a screenshot
//! import is one round-trip and the fuzzy matching runs on engine data.
use std::collections::{BTreeMap, HashMap};

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use crate::calc::data;
use crate::calc::types::{Affix, AngelicAugment, CharacterClass, Gem, ItemBase, RangedValue};

const NAME_MATCH_THRESHOLD: f64 = 0.72;
const PHRASE_MATCH_THRESHOLD: f64 = 0.8;
const SKILL_NAME_THRESHOLD: f64 = 0.7;
const CLASS_NAME_THRESHOLD: f64 = 0.55;
const NAME_SCAN_LINES: usize = 4;
const AUGMENT_MAX_LEVEL: i64 = 7;
const MINOR_WORDS: [&str; 5] = ["of", "to", "per", "and", "low"];

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: Lazy<Regex> = Lazy::new(|| Regex::new($pat).expect(stringify!($name)));
    };
}

re!(WS, r"\s+");
re!(CANON_BRACKETS, r"[\[\]|{}()]");
re!(CANON_NUMBERS, r"[+-]?\d+(?:\.\d+)?");
// OCR mangles brackets into 1/|/l/I/), so the range regex accepts them all.
re!(
    TRAILING_RANGE,
    r"[\s\[\]|({](\d{1,6})\s*[-–]\s*(\d{1,6})[\]|)}\s]*$"
);
re!(LEAD_VALUE, r"^([+-]?)(\d+(?:\.\d+)?)");
re!(TAIL_VALUE, r"(\d+(?:\.\d+)?)\s*%?\s*$");
re!(CLASS_SUFFIX, r"^(.*?)[\s(]+([\p{L}?]{3,20})\)\s*$");
re!(TO_PREFIX, r"^to\s+");
re!(PROC_LEVEL, r"(?i)level\s*(\d+)\s*$");
re!(PROC_CHANCE, r"(?i)chance\s+(when|on|while)");
re!(
    AUGMENT_LINE,
    r"(?i)^augment[:.]?\s*(.+?)[\s\[|({l1]*level\s*(\d+)"
);
re!(SOCKETS_LINE, r"(?i)^sockets?\s*\(?(\d+)\)?");
re!(HAS_SIGN, r"^[+-]");

static IGNORED_PREFIXES: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        r"(?i)^defen",
        r"(?i)^damage\s*[:.]",
        r"(?i)^attack speed",
        r"(?i)^block",
        r"(?i)^\(gem",
        r"(?i)^tier\s",
        r"(?i)requires level",
        r"(?i)^currently has",
        r"(?i)^flask cooldown",
        r"(?i)^effect duration",
        r"(?i)^runeword",
    ]
    .iter()
    .map(|p| Regex::new(p).expect("ignored prefix"))
    .collect()
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineStatus {
    Matched,
    Ignored,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TooltipLine {
    pub text: String,
    pub status: LineStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedAffix {
    pub affix_id: String,
    pub tier: u32,
    pub roll: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParsedAugment {
    pub id: String,
    pub level: u32,
}

/// Mirrors the frontend `EquippedItem` shape so the modal can equip it as-is.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedItem {
    pub base_id: String,
    pub affixes: Vec<ParsedAffix>,
    pub socket_count: u32,
    pub socketed: Vec<Option<String>>,
    pub socket_types: Vec<&'static str>,
    pub stars: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forged_mods: Option<Vec<ParsedAffix>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub augment: Option<ParsedAugment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit_overrides: Option<BTreeMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_bonus_overrides: Option<BTreeMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_skills_class_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TooltipParseResult {
    pub base_id: Option<String>,
    pub equipped: Option<ParsedItem>,
    pub lines: Vec<TooltipLine>,
    pub errors: Vec<String>,
}

// ---------- text helpers ----------

fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    if a == b {
        return 0;
    }
    let (m, n) = (a.len(), b.len());
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut prev: Vec<usize> = (0..=n).collect();
    for i in 1..=m {
        let mut cur = vec![0usize; n + 1];
        cur[0] = i;
        for j in 1..=n {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (cur[j - 1] + 1).min(prev[j] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[n]
}

fn similarity(a: &str, b: &str) -> f64 {
    let x: Vec<char> = a.to_lowercase().chars().collect();
    let y: Vec<char> = b.to_lowercase().chars().collect();
    let max = x.len().max(y.len());
    if max == 0 {
        return 1.0;
    }
    1.0 - levenshtein(&x, &y) as f64 / max as f64
}

/// Strip every numeric token, range and % so game/OCR phrasing collapses to one key.
fn canon_phrase(s: &str) -> String {
    let s = CANON_BRACKETS.replace_all(s, " ");
    let s = CANON_NUMBERS.replace_all(&s, " ");
    let s = s.replace('%', " ");
    WS.replace_all(&s, " ").trim().to_lowercase()
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn stat_name(key: &str) -> String {
    let defs = &data::game_config().stats;
    if let Some(def) = defs.iter().find(|d| d.key == key) {
        return def.name.clone();
    }
    if let Some(base) = key.strip_suffix("_more") {
        if let Some(def) = defs.iter().find(|d| d.key == base) {
            return format!("Total {}", def.name);
        }
    }
    key.split('_')
        .enumerate()
        .map(|(i, word)| {
            if i > 0 && MINOR_WORDS.contains(&word) {
                word.to_string()
            } else {
                capitalize(word)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn to_pair(v: RangedValue) -> (f64, f64) {
    match v {
        RangedValue::Scalar(x) => (x, x),
        RangedValue::Range([a, b]) => (a, b),
    }
}

fn extract_trailing_range(line: &str) -> (Vec<(f64, f64)>, String) {
    let Some(m) = TRAILING_RANGE.captures(line) else {
        return (Vec::new(), line.to_string());
    };
    let rest = line[..m.get(0).expect("match").start()].trim().to_string();
    let raw_a = m.get(1).expect("lo").as_str();
    let raw_b = m.get(2).expect("hi").as_str();
    let mut candidates: Vec<(f64, f64)> = Vec::new();
    let mut push = |a: &str, b: &str| {
        if a.is_empty() || b.is_empty() {
            return;
        }
        let (Ok(lo), Ok(hi)) = (a.parse::<f64>(), b.parse::<f64>()) else {
            return;
        };
        if lo > hi || candidates.contains(&(lo, hi)) {
            return;
        }
        candidates.push((lo, hi));
    };
    push(raw_a, raw_b);
    if raw_a.starts_with('1') {
        push(&raw_a[1..], raw_b);
    }
    if raw_b.ends_with('1') {
        push(raw_a, &raw_b[..raw_b.len() - 1]);
    }
    if raw_a.starts_with('1') && raw_b.ends_with('1') {
        push(&raw_a[1..], &raw_b[..raw_b.len() - 1]);
    }
    (candidates, rest)
}

fn extract_value(rest: &str) -> Option<f64> {
    if let Some(m) = LEAD_VALUE.captures(rest) {
        let v: f64 = m[2].parse().ok()?;
        return Some(if &m[1] == "-" { -v } else { v });
    }
    TAIL_VALUE.captures(rest).and_then(|m| m[1].parse().ok())
}

fn roll_for(value: f64, min: f64, max: f64) -> f64 {
    if max <= min {
        return 1.0;
    }
    let r = (value.abs() - min) / (max - min);
    (js_round(r * 1000.0) / 1000.0).clamp(0.0, 1.0)
}

/// Best similarity of `needle` against any same-length word window of `haystack`.
fn best_window_similarity(haystack: &str, needle: &str) -> f64 {
    let words: Vec<&str> = haystack.split(' ').collect();
    let needle_words = needle.split(' ').count();
    let mut best: f64 = 0.0;
    if words.len() < needle_words {
        return best;
    }
    for i in 0..=(words.len() - needle_words) {
        let window = words[i..i + needle_words].join(" ");
        best = best.max(similarity(&window, needle));
    }
    best
}

fn sorted_by_id<'a, T>(values: impl Iterator<Item = &'a T>, id: impl Fn(&T) -> &str) -> Vec<&'a T> {
    let mut out: Vec<&T> = values.collect();
    out.sort_by(|a, b| id(a).cmp(id(b)));
    out
}

// ---------- affix group index ----------

struct AffixGroup<'a> {
    canon: String,
    entries: Vec<&'a Affix>,
}

fn build_group_index(pool: &[&'static Affix]) -> Vec<AffixGroup<'static>> {
    let mut groups: Vec<AffixGroup> = Vec::new();
    let mut by_canon: HashMap<String, usize> = HashMap::new();
    for affix in pool {
        let key = canon_phrase(&affix.description);
        if key.is_empty() {
            continue;
        }
        match by_canon.get(&key) {
            Some(&i) => groups[i].entries.push(affix),
            None => {
                by_canon.insert(key.clone(), groups.len());
                groups.push(AffixGroup {
                    canon: key,
                    entries: vec![affix],
                });
            }
        }
    }
    groups
}

fn best_group<'i, 'a>(index: &'i [AffixGroup<'a>], phrase: &str) -> Option<&'i AffixGroup<'a>> {
    let mut best: Option<(&AffixGroup, f64)> = None;
    for group in index {
        let score = similarity(&group.canon, phrase);
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((group, score));
        }
    }
    best.filter(|(_, s)| *s >= PHRASE_MATCH_THRESHOLD)
        .map(|(g, _)| g)
}

// ---------- matching ----------

enum Apply {
    Implicit(String, f64),
    SkillBonus(String, f64),
    Affix(ParsedAffix),
    Forged(ParsedAffix),
    Augment(String, u32),
    ClassId(String),
}

struct GemCandidate {
    stat_keys: Vec<String>,
    value: f64,
}

struct StatMatch {
    status: LineStatus,
    detail: String,
    apply: Vec<Apply>,
    gem_candidate: Option<GemCandidate>,
}

impl StatMatch {
    fn matched(detail: String, apply: Vec<Apply>) -> Self {
        Self {
            status: LineStatus::Matched,
            detail,
            apply,
            gem_candidate: None,
        }
    }

    fn warning(detail: String, gem_candidate: Option<GemCandidate>) -> Self {
        Self {
            status: LineStatus::Warning,
            detail,
            apply: Vec::new(),
            gem_candidate,
        }
    }
}

#[derive(Default)]
struct Acc {
    implicit_overrides: BTreeMap<String, f64>,
    skill_bonus_overrides: BTreeMap<String, f64>,
    forged_mods: Vec<ParsedAffix>,
    affixes: Vec<ParsedAffix>,
    socket_count: Option<u32>,
    augment: Option<ParsedAugment>,
    all_skills_class_id: Option<String>,
}

impl Acc {
    fn apply(&mut self, apply: Apply) {
        match apply {
            Apply::Implicit(key, value) => {
                self.implicit_overrides.insert(key, value);
            }
            Apply::SkillBonus(key, value) => {
                self.skill_bonus_overrides.insert(key, value);
            }
            Apply::Affix(a) => self.affixes.push(a),
            Apply::Forged(a) => self.forged_mods.push(a),
            Apply::Augment(id, level) => self.augment = Some(ParsedAugment { id, level }),
            Apply::ClassId(id) => self.all_skills_class_id = Some(id),
        }
    }
}

fn find_item_name(lines: &[String]) -> Option<(String, usize)> {
    let items = sorted_by_id(data::data().items.values(), |i: &ItemBase| &i.id);
    let mut best: Option<(&ItemBase, usize, f64)> = None;
    let scan = lines.len().min(NAME_SCAN_LINES);
    for i in 0..scan {
        let single = lines[i].trim().to_string();
        let mut candidates: Vec<(String, usize)> = vec![(single.clone(), i)];
        if i + 1 < lines.len() {
            candidates.push((format!("{single} {}", lines[i + 1].trim()), i + 1));
        }
        for (candidate, end) in &candidates {
            for item in &items {
                let score = similarity(candidate, &item.name);
                if score >= NAME_MATCH_THRESHOLD && best.is_none_or(|(_, _, s)| score > s) {
                    best = Some((item, *end, score));
                }
            }
        }
    }
    best.map(|(item, end, _)| (item.id.clone(), end))
}

/// "(JÖTUNN)" survives OCR as e.g. "U?TUNN)" — fuzzy-match a trailing token vs class names.
fn extract_class_suffix(rest: &str) -> (Option<String>, String) {
    let Some(m) = CLASS_SUFFIX.captures(rest) else {
        return (None, rest.to_string());
    };
    let classes = sorted_by_id(data::data().classes.values(), |c: &CharacterClass| &c.id);
    let mut best: Option<(&CharacterClass, f64)> = None;
    for class in classes {
        let score = similarity(&m[2], &class.name);
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((class, score));
        }
    }
    match best {
        Some((class, score)) if score >= CLASS_NAME_THRESHOLD => {
            (Some(class.id.clone()), m[1].trim().to_string())
        }
        _ => (None, rest.to_string()),
    }
}

fn sorted_implicits(base: &ItemBase) -> Vec<(&String, (f64, f64))> {
    let mut out: Vec<(&String, (f64, f64))> = base
        .implicit
        .iter()
        .flat_map(|m| m.iter())
        .map(|(k, v)| (k, to_pair(*v)))
        .collect();
    out.sort_by(|a, b| a.0.cmp(b.0));
    out
}

fn match_implicit(
    base: &ItemBase,
    phrase: &str,
    value: Option<f64>,
    ranges: &[(f64, f64)],
) -> Option<StatMatch> {
    let implicit = sorted_implicits(base);
    if !ranges.is_empty() {
        let mut best: Option<(&String, f64)> = None;
        for (key, pair) in &implicit {
            if pair.0 == pair.1 || !ranges.contains(pair) {
                continue;
            }
            let score = similarity(&canon_phrase(&stat_name(key)), phrase);
            if best.is_none_or(|(_, s)| score > s) {
                best = Some((key, score));
            }
        }
        return match (best, value) {
            (Some((key, _)), Some(v)) => {
                let pinned = v.abs();
                Some(StatMatch::matched(
                    format!("implicit {key} = {pinned}"),
                    vec![Apply::Implicit(key.clone(), pinned)],
                ))
            }
            _ => None,
        };
    }
    let value = value?;
    let mut best: Option<(&String, (f64, f64), f64)> = None;
    for (key, pair) in &implicit {
        let score = similarity(&canon_phrase(&stat_name(key)), phrase);
        if best.is_none_or(|(_, _, s)| score > s) {
            best = Some((key, *pair, score));
        }
    }
    let (key, pair, score) = best?;
    if score < PHRASE_MATCH_THRESHOLD {
        return None;
    }
    let pinned = value.abs();
    if pair.0 == pair.1 && pair.0 == pinned {
        return Some(StatMatch::matched(
            format!("implicit {key} (base value)"),
            Vec::new(),
        ));
    }
    Some(StatMatch::matched(
        format!("implicit {key} = {pinned}"),
        vec![Apply::Implicit(key.clone(), pinned)],
    ))
}

fn sorted_skill_bonus_keys(base: &ItemBase) -> Vec<&String> {
    let mut keys: Vec<&String> = base.skill_bonuses.iter().flat_map(|m| m.keys()).collect();
    keys.sort();
    keys
}

fn match_skill_bonus(base: &ItemBase, phrase: &str, value: Option<f64>) -> Option<StatMatch> {
    base.skill_bonuses.as_ref()?;
    let value = value?;
    let target = TO_PREFIX.replace(phrase, "");
    let mut best: Option<(&String, f64)> = None;
    for key in sorted_skill_bonus_keys(base) {
        let score = similarity(&canon_phrase(key), &target);
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((key, score));
        }
    }
    let (key, score) = best?;
    if score < SKILL_NAME_THRESHOLD {
        return None;
    }
    let pinned = value.abs();
    Some(StatMatch::matched(
        format!("skill bonus {key} = {pinned}"),
        vec![Apply::SkillBonus(key.clone(), pinned)],
    ))
}

fn match_pool(
    index: &[AffixGroup<'static>],
    forged: bool,
    phrase: &str,
    value: Option<f64>,
    ranges: &[(f64, f64)],
) -> Option<StatMatch> {
    let group = best_group(index, phrase)?;
    let abs = value?.abs();
    let ranged: Vec<(&Affix, f64, f64)> = group
        .entries
        .iter()
        .filter_map(|a| Some((*a, a.value_min?, a.value_max?)))
        .collect();
    let mut tier = ranged
        .iter()
        .find(|(_, lo, hi)| ranges.contains(&(*lo, *hi)))
        .copied();
    if tier.is_none() {
        let mut containing: Vec<(&Affix, f64, f64)> = ranged
            .iter()
            .filter(|(_, lo, hi)| abs >= *lo && abs <= *hi)
            .copied()
            .collect();
        containing.sort_by(|a, b| {
            (a.2 - a.1)
                .partial_cmp(&(b.2 - b.1))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        tier = containing.first().copied();
    }
    let (affix, lo, hi) = tier?;
    let parsed = ParsedAffix {
        affix_id: affix.id.clone(),
        tier: affix.tier,
        roll: roll_for(abs, lo, hi),
    };
    let kind = if forged { "forged" } else { "affix" };
    let apply = if forged {
        Apply::Forged(parsed)
    } else {
        Apply::Affix(parsed)
    };
    Some(StatMatch::matched(
        format!("{kind} {}", affix.id),
        vec![apply],
    ))
}

fn match_proc_line(base: &ItemBase, line: &str) -> Option<StatMatch> {
    let level = PROC_LEVEL.captures(line)?;
    if !PROC_CHANCE.is_match(line) {
        return None;
    }
    if base.skill_bonuses.is_none() {
        return Some(StatMatch::matched(
            "proc line (no granted-skill data)".to_string(),
            Vec::new(),
        ));
    }
    let canon_line = canon_phrase(line);
    let mut best: Option<(&String, f64)> = None;
    for key in sorted_skill_bonus_keys(base) {
        let canon_key = canon_phrase(key);
        let score = if canon_line.contains(&canon_key) {
            1.0
        } else {
            best_window_similarity(&canon_line, &canon_key)
        };
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((key, score));
        }
    }
    let Some((key, _)) = best.filter(|(_, s)| *s >= SKILL_NAME_THRESHOLD) else {
        return Some(StatMatch::matched(
            "proc line (granted skill not recognized)".to_string(),
            Vec::new(),
        ));
    };
    let pinned: f64 = level[1].parse().ok()?;
    Some(StatMatch::matched(
        format!("granted skill {key} level {pinned}"),
        vec![Apply::SkillBonus(key.clone(), pinned)],
    ))
}

fn match_augment_line(line: &str) -> Option<StatMatch> {
    let m = AUGMENT_LINE.captures(line)?;
    let name = m[1].trim();
    let augments = sorted_by_id(data::data().augments.values(), |a: &AngelicAugment| &a.id);
    let mut best: Option<(&AngelicAugment, f64)> = None;
    for augment in augments {
        let score = similarity(name, &augment.name);
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((augment, score));
        }
    }
    let Some((augment, _)) = best.filter(|(_, s)| *s >= SKILL_NAME_THRESHOLD) else {
        return Some(StatMatch::warning(
            format!("unknown augment \"{name}\""),
            None,
        ));
    };
    let level = m[2].parse::<i64>().unwrap_or(1).clamp(1, AUGMENT_MAX_LEVEL) as u32;
    Some(StatMatch::matched(
        format!("augment {} level {level}", augment.id),
        vec![Apply::Augment(augment.id.clone(), level)],
    ))
}

struct Indexes {
    affix: Vec<AffixGroup<'static>>,
    crystal: Vec<AffixGroup<'static>>,
}

fn match_stat_line(base: &ItemBase, line: &str, idx: &Indexes) -> Option<StatMatch> {
    let (candidates, no_range) = extract_trailing_range(line);
    let has_sign = HAS_SIGN.is_match(line.trim());
    if !has_sign && candidates.is_empty() {
        return None;
    }
    let (class_id, rest) = extract_class_suffix(&no_range);
    let value = extract_value(&rest);
    let phrase = canon_phrase(&rest);
    if phrase.is_empty() {
        return None;
    }
    let found = match_implicit(base, &phrase, value, &candidates)
        .or_else(|| match_skill_bonus(base, &phrase, value))
        .or_else(|| match_pool(&idx.affix, false, &phrase, value, &candidates))
        .or_else(|| match_pool(&idx.crystal, true, &phrase, value, &candidates));
    let Some(mut found) = found else {
        let group = best_group(&idx.affix, &phrase).or_else(|| best_group(&idx.crystal, &phrase));
        let mut stat_keys: Vec<String> = Vec::new();
        for entry in group.map(|g| g.entries.as_slice()).unwrap_or(&[]) {
            if let Some(key) = entry.stat_key.as_ref() {
                if !stat_keys.contains(key) {
                    stat_keys.push(key.clone());
                }
            }
        }
        let gem_candidate = match value {
            Some(v) if candidates.is_empty() && !stat_keys.is_empty() => Some(GemCandidate {
                stat_keys,
                value: v.abs(),
            }),
            _ => None,
        };
        let detail = match group {
            Some(g) => format!(
                "\"{}\" value outside known tiers — socketed gems or unsupported source",
                g.canon
            ),
            None => "unrecognized stat line — fix manually after import".to_string(),
        };
        return Some(StatMatch::warning(detail, gem_candidate));
    };
    if let Some(class_id) = class_id {
        if found.status == LineStatus::Matched {
            found.apply.push(Apply::ClassId(class_id));
        }
    }
    Some(found)
}

struct PendingGemLine {
    line_index: usize,
    stat_keys: Vec<String>,
    value: f64,
}

struct GemFill {
    gem_ids: Vec<String>,
    line_details: HashMap<usize, String>,
}

/// Leftover fixed stat lines are usually socketed gems rendered into the stat
/// block. Accept only a full explanation: every pending line consumed by a
/// consistent integer count of gems fitting the socket count.
fn resolve_socketed_gems(pending: Vec<PendingGemLine>, socket_count: u32) -> Option<GemFill> {
    if pending.is_empty() || socket_count == 0 {
        return None;
    }
    let mut remaining: Vec<PendingGemLine> = pending;
    let mut gem_ids: Vec<String> = Vec::new();
    let mut line_details: HashMap<usize, String> = HashMap::new();
    let mut slots = socket_count as f64;
    let mut gems: Vec<&Gem> = data::data().gems.values().collect();
    gems.sort_by(|a, b| {
        b.stats
            .len()
            .cmp(&a.stats.len())
            .then(b.tier.cmp(&a.tier))
            .then(a.id.cmp(&b.id))
    });
    for gem in gems {
        let mut keys: Vec<(&String, f64)> = gem
            .stats
            .iter()
            .filter(|(_, v)| **v != 0.0)
            .map(|(k, v)| (k, *v))
            .collect();
        keys.sort_by(|a, b| a.0.cmp(b.0));
        if keys.is_empty() {
            continue;
        }
        let mut picked: Vec<(usize, f64)> = Vec::new();
        for (key, per) in &keys {
            let found = remaining
                .iter()
                .enumerate()
                .find(|(i, line)| {
                    line.stat_keys.contains(key) && !picked.iter().any(|(p, _)| p == i)
                })
                .map(|(i, _)| i);
            let Some(i) = found else {
                break;
            };
            picked.push((i, *per));
        }
        if picked.len() != keys.len() {
            continue;
        }
        let counts: Vec<f64> = picked
            .iter()
            .map(|(i, per)| remaining[*i].value / per)
            .collect();
        let count = counts[0];
        if count.fract() != 0.0 || count < 1.0 || count > slots {
            continue;
        }
        if !counts.iter().all(|c| *c == count) {
            continue;
        }
        slots -= count;
        for _ in 0..(count as usize) {
            gem_ids.push(gem.id.clone());
        }
        let mut consumed: Vec<usize> = picked.iter().map(|(i, _)| *i).collect();
        for i in &consumed {
            line_details.insert(
                remaining[*i].line_index,
                format!("socketed gems: {}× {}", count as i64, gem.name),
            );
        }
        consumed.sort_unstable_by(|a, b| b.cmp(a));
        for i in consumed {
            remaining.remove(i);
        }
    }
    remaining.is_empty().then_some(GemFill {
        gem_ids,
        line_details,
    })
}

pub fn parse_tooltip(raw_lines: &[String]) -> TooltipParseResult {
    let lines: Vec<String> = raw_lines
        .iter()
        .map(|l| WS.replace_all(l, " ").trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let mut out: Vec<TooltipLine> = Vec::new();

    let Some((base_id, name_end)) = find_item_name(&lines) else {
        return TooltipParseResult {
            base_id: None,
            equipped: None,
            lines: lines
                .into_iter()
                .map(|text| TooltipLine {
                    text,
                    status: LineStatus::Ignored,
                    detail: None,
                })
                .collect(),
            errors: vec![
                "No item name recognized — crop the screenshot to the tooltip".to_string(),
            ],
        };
    };
    let Some(base) = data::get_item(&base_id) else {
        return TooltipParseResult {
            base_id: None,
            equipped: None,
            lines: Vec::new(),
            errors: vec![format!("Unknown base item id: {base_id}")],
        };
    };

    let game = data::data();
    let idx = Indexes {
        affix: build_group_index(&sorted_by_id(game.affixes.values(), |a: &Affix| &a.id)),
        crystal: build_group_index(&sorted_by_id(game.crystals.values(), |a: &Affix| &a.id)),
    };
    let mut acc = Acc::default();
    let mut pending: Vec<PendingGemLine> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let push = |out: &mut Vec<TooltipLine>, status: LineStatus, detail: Option<String>| {
            out.push(TooltipLine {
                text: line.clone(),
                status,
                detail,
            })
        };
        if i <= name_end {
            push(
                &mut out,
                LineStatus::Matched,
                Some(format!("item: {}", base.name)),
            );
            continue;
        }
        if IGNORED_PREFIXES.iter().any(|re| re.is_match(line)) {
            push(&mut out, LineStatus::Ignored, None);
            continue;
        }
        if let Some(m) = SOCKETS_LINE.captures(line) {
            let count: u32 = m[1].parse().unwrap_or(0);
            acc.socket_count = Some(count);
            push(
                &mut out,
                LineStatus::Matched,
                Some(format!("sockets: {count}")),
            );
            continue;
        }
        let found = match_augment_line(line)
            .or_else(|| match_proc_line(base, line))
            .or_else(|| match_stat_line(base, line, &idx));
        let Some(found) = found else {
            push(&mut out, LineStatus::Ignored, None);
            continue;
        };
        for apply in found.apply {
            acc.apply(apply);
        }
        if let Some(gem) = found.gem_candidate {
            pending.push(PendingGemLine {
                line_index: out.len(),
                stat_keys: gem.stat_keys,
                value: gem.value,
            });
        }
        push(&mut out, found.status, Some(found.detail));
    }

    let socket_count = acc.socket_count.or(base.sockets).unwrap_or(0);
    let gem_fill = resolve_socketed_gems(pending, socket_count);
    let mut socketed: Vec<Option<String>> = vec![None; socket_count as usize];
    if let Some(fill) = &gem_fill {
        for (line_index, detail) in &fill.line_details {
            if let Some(line) = out.get_mut(*line_index) {
                line.status = LineStatus::Matched;
                line.detail = Some(detail.clone());
            }
        }
        for (i, id) in fill.gem_ids.iter().enumerate() {
            if i < socketed.len() {
                socketed[i] = Some(id.clone());
            }
        }
    }

    let equipped = ParsedItem {
        base_id: base.id.clone(),
        affixes: acc.affixes,
        socket_count,
        socketed,
        socket_types: vec!["normal"; socket_count as usize],
        stars: 0,
        forged_mods: (!acc.forged_mods.is_empty()).then_some(acc.forged_mods),
        augment: acc.augment,
        implicit_overrides: (!acc.implicit_overrides.is_empty()).then_some(acc.implicit_overrides),
        skill_bonus_overrides: (!acc.skill_bonus_overrides.is_empty())
            .then_some(acc.skill_bonus_overrides),
        all_skills_class_id: acc.all_skills_class_id,
    };
    TooltipParseResult {
        base_id: Some(base.id.clone()),
        equipped: Some(equipped),
        lines: out,
        errors: Vec::new(),
    }
}

#[tauri::command]
pub fn parse_tooltip_lines(lines: Vec<String>, season: Option<String>) -> TooltipParseResult {
    let _scope = crate::calc::season::SeasonScope::enter(season);
    parse_tooltip(&lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> TooltipParseResult {
        parse_tooltip(&text.lines().map(str::to_string).collect::<Vec<_>>())
    }

    fn implicit(result: &TooltipParseResult, key: &str) -> Option<f64> {
        result
            .equipped
            .as_ref()?
            .implicit_overrides
            .as_ref()?
            .get(key)
            .copied()
    }

    fn skill_bonus(result: &TooltipParseResult, key: &str) -> Option<f64> {
        result
            .equipped
            .as_ref()?
            .skill_bonus_overrides
            .as_ref()?
            .get(key)
            .copied()
    }

    fn warnings(result: &TooltipParseResult) -> Vec<&TooltipLine> {
        result
            .lines
            .iter()
            .filter(|l| l.status == LineStatus::Warning)
            .collect()
    }

    fn ignored_matching(result: &TooltipParseResult, needle: &str) -> bool {
        result
            .lines
            .iter()
            .any(|l| l.status == LineStatus::Ignored && l.text.to_lowercase().contains(needle))
    }

    // Real ocrs output for engine/tests/fixtures/tooltips/tooltip1.png (Tundra
    // Hunter's Long Coat) — mangled brackets and OCR noise included on purpose.
    const TOOLTIP1: &str = "TUNDRA HUNTER'S LONG COAT
HEROIC BODY ARMOR
(GEM, GEM. GEM, GEM, GEM)
DEFENSE: 1333 [227] [210-2401
35% CHANCE WHEN STRUCK SET SAIL LEVEL 30
TEMPORARILY INCREASES YOUR COLD SKILL DAMAGE AND
MANA REPLENISH.
COLD SKILL DAMAGE 75%
MANA REPLENISH 65%
+487% ENHANCED DEFENSE 1450-525]
+3 TO ALL SKILLS [2-3]
+3 TO COLD SKILLS |3-51
AUGMENT: LETHAL TEMPO [LEVEL5|
INCREASES YOUR ATTACK SPEED AND CRITICAL STRIKE DAMAGE
FOR A SHORT PERIOD
ATTACK SPEED 80%
CRITICAL STRIKE DAMAGE 40%
EXTRA DAMAGE TO DEEP FROZEN MONSTERS 15% 115-25
+30 TO ALL ATTRIBUTES
+2560 TO ADDITIVE COLD DAMAGE
+100 TO COLD SKILL DAMAGE
COLD SKILL DAMAGE INCREASED BY 25% 125-40)
-23% TO ENEMY COLD RESISTANCE [15-30)
+40% TO COLD RESISTANCE 130-50]
SOCKETS (5) [3-61
IN THE STORMS OF A COLD AND FROZEN TUNDRA. A LONE HUNTER IS
STALKING HIS PREY.
B. Pick up TIER SS. REQUIRES LEVEL 94";

    #[test]
    fn tundra_hunter_matches_base_and_pins_implicits() {
        let result = parse(TOOLTIP1);
        assert_eq!(
            result.base_id.as_deref(),
            Some("body_armor_heroic_tundra_hunter_s_long_coat")
        );
        assert_eq!(implicit(&result, "enhanced_defense"), Some(487.0));
        assert_eq!(implicit(&result, "all_skills"), Some(3.0));
        assert_eq!(implicit(&result, "cold_skills"), Some(3.0));
        assert_eq!(implicit(&result, "extra_dmg_to_deep_frozen"), Some(15.0));
        assert_eq!(implicit(&result, "cold_skill_damage"), Some(25.0));
        assert_eq!(implicit(&result, "ignore_cold_res"), Some(23.0));
        assert_eq!(implicit(&result, "cold_resistance"), Some(40.0));
        assert_eq!(
            implicit(&result, "all_attributes"),
            None,
            "fixed implicit equal to base needs no override"
        );
    }

    #[test]
    fn tundra_hunter_sockets_augment_proc_and_gems() {
        let result = parse(TOOLTIP1);
        let equipped = result.equipped.as_ref().unwrap();
        assert_eq!(equipped.socket_count, 5);
        assert_eq!(equipped.socketed.len(), 5);
        assert_eq!(
            equipped.augment,
            Some(ParsedAugment {
                id: "lethal_tempo".into(),
                level: 5
            })
        );
        assert_eq!(skill_bonus(&result, "Set Sail"), Some(30.0));
        assert!(equipped
            .socketed
            .iter()
            .all(|s| s.as_deref() == Some("gem_pristine_sapphire")));
        let gem_lines: Vec<&TooltipLine> = result
            .lines
            .iter()
            .filter(|l| {
                l.text.contains("ADDITIVE COLD DAMAGE")
                    || l.text.contains("+100 TO COLD SKILL DAMAGE")
            })
            .collect();
        assert_eq!(gem_lines.len(), 2);
        for line in gem_lines {
            assert_eq!(line.status, LineStatus::Matched);
            assert!(line
                .detail
                .as_deref()
                .unwrap_or("")
                .contains("Pristine Sapphire"));
        }
        assert!(warnings(&result).is_empty());
        assert!(
            equipped.affixes.is_empty(),
            "no random affixes on a heroic item"
        );
    }

    #[test]
    fn tundra_hunter_ignores_flavor_and_noise() {
        let result = parse(TOOLTIP1);
        assert!(ignored_matching(&result, "stalking his prey"));
        assert!(ignored_matching(&result, "requires level"));
        assert!(ignored_matching(&result, "temporarily increases"));
    }

    const TOOLTIP2: &str = "GRIMBONE'S VISAGE
HEROIC HELMET
(GEM, GEM, GEM, GEM)
DEFENSF: 135 [63][60-80]
+113% ENHANCED DEFENSE 110-140])
+2 TO ALL SKILLS [2-3]
AILMENT DAMAGE INCREASED BY 17% [15-20]
+20 TO INTFLLIGFNCF [15-25]
+2048 TO ADDITIVE COLD DAMAGF
+80 TO COLD SKILL DAMAGE
MAGIC SKILL DAMAGE INCREASED BY 36% [20-50]
REPLENISH MANA 115% 100-150]
+600 TO MANA (BASED ON LEVEL) [4-8]
+50% TO ALL RESISTANCES
SOCKETS (4) 11-4)
GRIMBONE THE HIVEMIND OF TORMENTED SOULS,
FACE OF TERROR TO ALL THOSE WHO MEET THEIR FATE
IN NIFLHEL.
TIER SS. REQUIRES LEVEL 100
2E";

    #[test]
    fn grimbone_survives_ocr_typos() {
        let result = parse(TOOLTIP2);
        assert_eq!(
            result.base_id.as_deref(),
            Some("helmet_heroic_grimbone_s_visage")
        );
        assert_eq!(implicit(&result, "enhanced_defense"), Some(113.0));
        assert_eq!(implicit(&result, "all_skills"), Some(2.0));
        assert_eq!(implicit(&result, "ailment_damage_all"), Some(17.0));
        assert_eq!(implicit(&result, "to_intelligence"), Some(20.0));
        assert_eq!(implicit(&result, "magic_skill_damage"), Some(36.0));
        assert_eq!(implicit(&result, "all_resistances"), None);
        assert_eq!(implicit(&result, "mana_replenish_pct"), Some(115.0));
        let equipped = result.equipped.as_ref().unwrap();
        assert_eq!(
            equipped.forged_mods,
            Some(vec![ParsedAffix {
                affix_id: "crystal_satanic_mana_based_on_level".into(),
                tier: 1,
                roll: 0.5
            }])
        );
        assert_eq!(equipped.socket_count, 4);
        assert!(equipped
            .socketed
            .iter()
            .all(|s| s.as_deref() == Some("gem_pristine_sapphire")));
        assert!(warnings(&result).is_empty());
    }

    const TOOLTIP3: &str = "GRYPHON'S CLAW
HEROIC AMULET
+12 TO EXECUTF |12-18]
DEAL EXTRA ATTACK DAMAGE TO MONSTERS BELOW 30% LIFE.
ATTACK DAMAGE 72%
+18% INCREASED CRITICAL STRIKE CHANCE [18-25]
+46% NCREASED CRITICAL STRIKE DAMAGF [40-50]
+23% CHANCE TO OPEN WOUNDS [15-25]
+27 TO ALL ATTRIBUTES 120-30]
TIER SS, REQUIRES LEVEL 100";

    #[test]
    fn gryphon_pins_granted_skill_and_implicits() {
        let result = parse(TOOLTIP3);
        assert_eq!(
            result.base_id.as_deref(),
            Some("amulet_heroic_gryphon_s_claw")
        );
        assert_eq!(skill_bonus(&result, "Execute"), Some(12.0));
        assert_eq!(implicit(&result, "crit_chance"), Some(18.0));
        assert_eq!(implicit(&result, "crit_damage"), Some(46.0));
        assert_eq!(implicit(&result, "all_attributes"), Some(27.0));
    }

    const TOOLTIP4: &str = "GHOSTLY POTION
POTION
CURRENTLY HAS 1 CHARGFS OUT OF 1
FLASK COOLDOWN 45 SECONDS
EFFECT DURATION 15 SECONDS
+5 TO PARALLEL DIMENSION
INCREASES YOUR SKILL DAMAGE FOR A SHORT PERIOD OF TIME
MAGIC SKILL DAMAGE 57%
PO
TH
ENHANCES MAGIC SKILL DAMAGE.
TIER S, REQUIRES LEVEL 1
TO";

    #[test]
    fn potion_pins_granted_skill_and_ignores_hud() {
        let result = parse(TOOLTIP4);
        assert_eq!(
            result.base_id.as_deref(),
            Some("potion_satanic_ghostly_potion")
        );
        let bonuses = result
            .equipped
            .as_ref()
            .unwrap()
            .skill_bonus_overrides
            .clone()
            .unwrap();
        assert_eq!(bonuses.len(), 1);
        assert_eq!(bonuses.get("Parallel Dimension"), Some(&5.0));
        assert!(ignored_matching(&result, "chargfs"));
        assert!(ignored_matching(&result, "flask cooldown"));
    }

    // "(JÖTUNN)" arrives as "U?TUNN)".
    const TOOLTIP5: &str = "TORCH OF SHADOWS
HEROIC CHARM (1X2) (UNIQUE EQUIPPED)
5% CHANCE WHEN STRIKING ISHADOWFLAMES LEVEL3
UNLEASH FLAMES OF SHADOW TRAVELING FORWARD DEALING
ARCANE DAMAGE
ARCANE DAMAGE 1573072
+3 TO ALL SKILLS U?TUNN) [1-3]
+23 TO ALL ATTRIBUTES 120-30]
+8 TO LIGHT RADIUS
+20% TO ALL RESISTANCES 110-20)
Napd
d
TORCH FORGED AND LIGHTED IN THE SHADOW REALMS THAT HAS
SOME ODD PROPERTIES, SPREADING DARKNESS IN THE MORTAL WORLD
WHERE THERE SHOULD BE LIGHT:
TIER SS, REQUIRES LEVEL 100
TO";

    #[test]
    fn torch_of_shadow_recovers_class_scoped_skills() {
        let result = parse(TOOLTIP5);
        assert_eq!(
            result.base_id.as_deref(),
            Some("charm_heroic_torch_of_shadow")
        );
        let equipped = result.equipped.as_ref().unwrap();
        assert_eq!(equipped.all_skills_class_id.as_deref(), Some("jotunn"));
        assert_eq!(implicit(&result, "all_skills_class"), Some(3.0));
        assert_eq!(skill_bonus(&result, "Shadowflames"), Some(3.0));
        assert_eq!(implicit(&result, "all_attributes"), Some(23.0));
        assert_eq!(implicit(&result, "all_resistances"), Some(20.0));
        assert_eq!(implicit(&result, "light_radius"), None);
    }

    #[test]
    fn failure_modes() {
        let result = parse("GIBBERISH XYZZY\n+10 TO NOTHING");
        assert!(result.base_id.is_none());
        assert!(result.equipped.is_none());
        assert!(!result.errors.is_empty());
        let empty = parse_tooltip(&[]);
        assert!(empty.base_id.is_none());
        assert!(empty.equipped.is_none());
    }

    #[test]
    fn serializes_like_the_frontend_item_shape() {
        let json = serde_json::to_value(parse(TOOLTIP3)).unwrap();
        let equipped = &json["equipped"];
        assert_eq!(equipped["baseId"], "amulet_heroic_gryphon_s_claw");
        assert_eq!(equipped["socketTypes"], serde_json::json!([]));
        assert_eq!(equipped["skillBonusOverrides"]["Execute"], 12.0);
        assert!(equipped.get("forgedMods").is_none());
        assert_eq!(json["lines"][0]["status"], "matched");
    }
}
