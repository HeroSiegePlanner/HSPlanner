//! OCR'd tooltip lines → equipped item. Sits next to ocr.rs so a screenshot
//! import is one round-trip and the fuzzy matching runs on engine data.
use std::collections::{BTreeMap, HashMap, HashSet};

use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

use crate::calc::data;
use crate::calc::types::{
    Affix, AngelicAugment, CharacterClass, Gem, ItemBase, RangedValue, Runeword,
};

const NAME_MATCH_THRESHOLD: f64 = 0.72;
const PHRASE_MATCH_THRESHOLD: f64 = 0.8;
const SKILL_NAME_THRESHOLD: f64 = 0.7;
const CLASS_NAME_THRESHOLD: f64 = 0.55;
const NAME_SCAN_LINES: usize = 4;
const RUNEWORD_SCAN_LINES: usize = 6;
const RUNEWORD_WORD_THRESHOLD: f64 = 0.75;
const RUNES_TRIM: &str = "()[]{}| ";
const AUGMENT_MAX_LEVEL: i64 = 7;
const MINOR_WORDS: [&str; 5] = ["of", "to", "per", "and", "low"];

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> =
            LazyLock::new(|| Regex::new($pat).expect(stringify!($name)));
    };
}

re!(WS, r"\s+");
re!(CANON_BRACKETS, r"[\[\]|{}()]");
re!(CANON_NUMBERS, r"[+-]?\d+(?:\.\d+)?");
// "+[2-5] to X" leaves a lone "+" once the range is gone; drop it so tiers share one group.
re!(CANON_SIGNS, r"(?:^|\s)[+\-–]+(?:\s|$)");
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
re!(HANDEDNESS, r"(?i)\b(1|2|i|l|one|two)[\s-]*handed\b");
// "+5 to All Skills [2-5] Tier S" — the tier tag follows the range on base affixes.
re!(TRAILING_TIER, r"(?i)\s+tier\s+\S+\s*$");

static IGNORED_PREFIXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)^defen",
        r"(?i)^damage\s*[:.]",
        r"(?i)^attack damage\s*[:.]",
        r"(?i)^attacks per second",
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runeword_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TooltipParseResult {
    pub base_id: Option<String>,
    pub equipped: Option<ParsedItem>,
    pub lines: Vec<TooltipLine>,
    pub errors: Vec<String>,
}

impl From<ParsedAffix> for crate::calc::types::EquippedAffix {
    fn from(affix: ParsedAffix) -> Self {
        Self {
            affix_id: affix.affix_id,
            tier: affix.tier,
            roll: affix.roll,
            custom_value: None,
        }
    }
}

impl From<ParsedItem> for crate::calc::types::EquippedItem {
    fn from(item: ParsedItem) -> Self {
        use crate::calc::types::{AugmentRef, SocketType};
        Self {
            base_id: item.base_id,
            stars: Some(item.stars),
            affixes: item.affixes.into_iter().map(Into::into).collect(),
            socket_count: item.socket_count,
            socketed: item.socketed,
            socket_types: item
                .socket_types
                .into_iter()
                .map(|kind| {
                    if kind == "rainbow" {
                        SocketType::Rainbow
                    } else {
                        SocketType::Normal
                    }
                })
                .collect(),
            forged_mods: item
                .forged_mods
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            augment: item.augment.map(|augment| AugmentRef {
                id: augment.id,
                level: augment.level,
            }),
            implicit_overrides: item
                .implicit_overrides
                .unwrap_or_default()
                .into_iter()
                .collect(),
            skill_bonus_overrides: item
                .skill_bonus_overrides
                .unwrap_or_default()
                .into_iter()
                .collect(),
            all_skills_class_id: item.all_skills_class_id,
            runeword_id: item.runeword_id,
            ..Default::default()
        }
    }
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
    let s = crate::calc::resistance::canonical_text(s);
    let s = CANON_BRACKETS.replace_all(&s, " ");
    let s = CANON_NUMBERS.replace_all(&s, " ");
    let s = CANON_SIGNS.replace_all(&s, " ");
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
    let a_stripped = raw_a.strip_prefix('1');
    let b_stripped = raw_b.strip_suffix('1');
    if let Some(a) = a_stripped {
        push(a, raw_b);
    }
    if let Some(b) = b_stripped {
        push(raw_a, b);
    }
    if let (Some(a), Some(b)) = (a_stripped, b_stripped) {
        push(a, b);
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

struct RunewordHeader {
    rw: &'static Runeword,
    base: &'static ItemBase,
    runes: Vec<String>,
    name_end: usize,
    details: Vec<String>,
}

/// Runeword tooltips name the word, then "Runeword <handedness> <base type>", then the runes.
/// The exact base is not shown, so the first common base of that type stands in.
fn detect_runeword(lines: &[String]) -> Option<RunewordHeader> {
    let scan = lines.len().min(RUNEWORD_SCAN_LINES);
    let (header_ix, base_text) = (0..scan).find_map(|i| {
        let (first, rest) = lines[i].split_once(' ').unwrap_or((&lines[i], ""));
        (similarity(first, "runeword") >= RUNEWORD_WORD_THRESHOLD)
            .then(|| (i, rest.trim().to_string()))
    })?;
    let rune_pool = sorted_by_id(data::data().runes.values(), |r| &r.id);
    // Rune names are 2-4 letters, so OCR's 1/l/|→i and 0→o swaps matter more than distance.
    let ocr_normal = |s: &str| {
        s.to_lowercase()
            .chars()
            .map(|c| match c {
                '1' | 'l' | '|' => 'i',
                '0' => 'o',
                c => c,
            })
            .collect::<String>()
    };
    let rune_id = |name: &str| {
        let wanted = ocr_normal(name);
        rune_pool
            .iter()
            .map(|rune| {
                let normal = ocr_normal(&rune.name);
                let score = if normal == wanted {
                    1.0
                } else {
                    similarity(&wanted, &normal)
                };
                (rune, score)
            })
            .filter(|(_, score)| *score >= SKILL_NAME_THRESHOLD)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(rune, _)| rune.id.clone())
    };
    // "(Ymn, Co, Qi)" — OCR may drop or mangle the parentheses, so accept the bare list.
    let runes_line = (header_ix + 1..lines.len().min(header_ix + 4)).find_map(|i| {
        let names: Vec<String> = lines[i]
            .trim_matches(|c: char| RUNES_TRIM.contains(c))
            .split(',')
            .map(|s| {
                s.trim()
                    .trim_matches(|c: char| RUNES_TRIM.contains(c))
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();
        let ids: Vec<String> = names.iter().filter_map(|n| rune_id(n)).collect();
        (!names.is_empty() && names.len() <= 6 && ids.len() * 2 >= names.len())
            .then_some((i, names, ids))
    });
    let (runes_ix, rune_names, runes) = match runes_line {
        Some((i, names, ids)) => (Some(i), names, ids),
        None => (None, Vec::new(), Vec::new()),
    };
    let words = data::runewords();
    let mut names: Vec<String> = lines[..header_ix].to_vec();
    names.push(lines[..header_ix].join(" "));
    let rw = words
        .iter()
        .find(|rw| !runes.is_empty() && rw.runes == runes)
        .or_else(|| {
            words
                .iter()
                .flat_map(|rw| names.iter().map(move |n| (rw, similarity(n, &rw.name))))
                .filter(|(_, score)| *score >= NAME_MATCH_THRESHOLD)
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(rw, _)| rw)
        })?;
    let two_handed = HANDEDNESS
        .captures(&base_text)
        .is_some_and(|m| matches!(m[1].to_lowercase().as_str(), "2" | "two"));
    let type_text = canon_phrase(&HANDEDNESS.replace_all(&base_text, " "));
    let bases = sorted_by_id(data::data().items.values(), |i: &ItemBase| &i.id);
    let base = bases
        .iter()
        .filter(|item| item.rarity == "common")
        .filter(|item| {
            rw.allowed_base_types
                .iter()
                .any(|t| t.eq_ignore_ascii_case(&item.base_type))
        })
        .filter(|item| item.two_handed.unwrap_or(false) == two_handed)
        .map(|item| (item, similarity(&type_text, &item.base_type.to_lowercase())))
        .filter(|(_, score)| *score >= PHRASE_MATCH_THRESHOLD)
        .fold(None::<(&&ItemBase, f64)>, |best, next| match best {
            Some((_, score)) if score >= next.1 => best,
            _ => Some(next),
        })
        .map(|(item, _)| *item)?;
    let name_end = runes_ix.unwrap_or(header_ix);
    let details = (0..=name_end)
        .map(|i| {
            if i < header_ix {
                format!("runeword: {}", rw.name)
            } else if i == header_ix {
                format!(
                    "base type: {} → {} (pick the exact base in the editor)",
                    base.base_type, base.name
                )
            } else {
                format!("runes: {}", rune_names.join(", "))
            }
        })
        .collect();
    let rw: &'static Runeword = rw;
    Some(RunewordHeader {
        rw,
        base,
        runes,
        name_end,
        details,
    })
}

fn strip_trailing_tier(line: &str) -> String {
    TRAILING_TIER.replace(line, "").to_string()
}

fn word_overlap(phrase: &str, needle: &str) -> f64 {
    let words: HashSet<&str> = phrase.split(' ').collect();
    let needle: Vec<&str> = needle.split(' ').collect();
    needle.iter().filter(|w| words.contains(*w)).count() as f64 / needle.len().max(1) as f64
}

/// Runeword stats print the rolled value plus `[min-max]` (data keeps the max) or a fixed value;
/// granted skills print "<Skill> Level N [lo-hi]". Neither carries a `Tier`.
fn match_runeword_stat(rw: &Runeword, line: &str) -> Option<StatMatch> {
    let (candidates, rest) = extract_trailing_range(&strip_trailing_tier(line));
    let value = extract_value(&rest);
    let phrase = canon_phrase(&rest);
    if phrase.is_empty() {
        return None;
    }
    let mut stats: Vec<(&String, &f64)> = rw.stats.iter().collect();
    stats.sort_by(|a, b| a.0.cmp(b.0));
    let mut best: Option<(&String, f64)> = None;
    for (key, expected) in stats {
        let name = canon_phrase(&stat_name(key));
        let sim = best_window_similarity(&phrase, &name);
        let overlap = word_overlap(&phrase, &name);
        let value_ok = candidates.iter().any(|(_, hi)| (hi - expected).abs() < 0.5)
            || value.is_some_and(|v| (v.abs() - expected).abs() < 0.5);
        let ok = value_ok && (sim >= PHRASE_MATCH_THRESHOLD || overlap >= 0.6);
        if ok && best.is_none_or(|(_, s)| sim > s) {
            best = Some((key, sim));
        }
    }
    if let Some((key, _)) = best {
        return Some(StatMatch::matched(format!("runeword stat: {key}"), vec![]));
    }
    let skill = rw.skill_bonuses.as_ref()?.keys().find(|skill| {
        best_window_similarity(&phrase, &canon_phrase(skill)) >= SKILL_NAME_THRESHOLD
    })?;
    Some(StatMatch::matched(
        format!("runeword skill: {skill}"),
        vec![],
    ))
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
    let (candidates, no_range) = extract_trailing_range(&strip_trailing_tier(line));
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
    // Red "(Based on Level)" lines are crystal forged mods; the affix pool has look-alikes.
    let (first, second) = if phrase.contains("based on level") {
        ((&idx.crystal, true), (&idx.affix, false))
    } else {
        ((&idx.affix, false), (&idx.crystal, true))
    };
    let found = match_implicit(base, &phrase, value, &candidates)
        .or_else(|| match_skill_bonus(base, &phrase, value))
        .or_else(|| match_pool(first.0, first.1, &phrase, value, &candidates))
        .or_else(|| match_pool(second.0, second.1, &phrase, value, &candidates));
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

    let header = detect_runeword(&lines);
    let found_name = header
        .as_ref()
        .map(|h| (h.base.id.clone(), h.name_end))
        .or_else(|| find_item_name(&lines));
    let Some((base_id, name_end)) = found_name else {
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
            let detail = header
                .as_ref()
                .and_then(|h| h.details.get(i).cloned())
                .unwrap_or_else(|| format!("item: {}", base.name));
            push(&mut out, LineStatus::Matched, Some(detail));
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
        let found = header
            .as_ref()
            .and_then(|h| match_runeword_stat(h.rw, line))
            .or_else(|| match_augment_line(line))
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

    let socket_count = acc
        .socket_count
        .or(base.sockets)
        .unwrap_or(0)
        .max(header.as_ref().map_or(0, |h| h.runes.len() as u32));
    let gem_fill = resolve_socketed_gems(pending, socket_count);
    let mut socketed: Vec<Option<String>> = vec![None; socket_count as usize];
    if let Some(h) = &header {
        for (i, rune) in h.runes.iter().enumerate() {
            socketed[i] = Some(rune.clone());
        }
    }
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
        runeword_id: header.as_ref().map(|h| h.rw.id.clone()),
    };
    TooltipParseResult {
        base_id: Some(base.id.clone()),
        equipped: Some(equipped),
        lines: out,
        errors: Vec::new(),
    }
}

#[cfg_attr(feature = "desktop", tauri::command)]
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

    const AURORA: &str = "\
AURORA'S MIGHT
RUNEWORD 1-HANDED MACE
(YMN, CO, QI)
ATTACK DAMAGE: 234 TO 335 [54]
ATTACKS PER SECOND: 1.50
+520% ENHANCED DAMAGE [430-550]
+5 TO ALL SKILLS [2-5] TIER S
+7% LIFE STOLEN PER HIT
+31% OF TARGET DEFENSE IGNORED [30-40]
+10 TO DEXTERITY
+288 TO ADDITIVE ARCANE DAMAGE [250-320]
+242 TO ADDITIVE COLD DAMAGE [230-280]
ATTACK DAMAGE INCREASED BY 20%
+25% FASTER HIT RECOVERY
+15% EXTRA GOLD DROPPED FROM KILLS
LUNAR AURA LEVEL 27 [12-28]
INCREASES YOUR ATTACK RATING, ADDITIVE ARCANE DAMAGE
AND ALL RESISTANCES
ATTACK RATING 54%
ARCANE DAMAGE 270
ALL RESISTANCES 54%
SOCKETS (3) [1-5]
AURORA, OF OF THE PUREST SOULS TO EVER WALK ON
EARTH WAS BRUTALLY SLAIN, BUT HER SOUL WAS
INFUSED INTO THIS WEAPON SO SHE CAN LIVE ON
AND AVENGE HER DEATH.
TIER S, REQUIRES LEVEL 57";

    #[test]
    fn runeword_tooltip_maps_runes_base_type_and_stats() {
        let result = parse(AURORA);
        let item = result.equipped.as_ref().expect("runeword item");
        assert!(
            item.base_id.starts_with("base_mace_"),
            "a common mace base, got {}",
            item.base_id
        );
        assert_eq!(item.runeword_id.as_deref(), Some("rw_aurora_s_might"));
        assert_eq!(item.socket_count, 3);
        assert_eq!(
            item.socketed,
            vec![
                Some("rune_ymn".to_string()),
                Some("rune_co".to_string()),
                Some("rune_qi".to_string())
            ]
        );
        assert!(
            item.affixes
                .iter()
                .any(|a| a.affix_id == "1_to_all_skills_t5_archangel_s"),
            "the Tier S all-skills line is a base affix: {:?}",
            item.affixes
        );
        assert!(
            warnings(&result).is_empty(),
            "runeword stats must not need manual review: {:?}",
            warnings(&result)
        );
        let matched = |needle: &str| {
            result.lines.iter().any(|line| {
                line.status == LineStatus::Matched && line.text.to_lowercase().contains(needle)
            })
        };
        assert!(matched("defense ignored"), "ranged runeword stat");
        assert!(matched("attack damage increased"), "unsigned runeword stat");
        assert!(matched("lunar aura"), "runeword skill bonus");
        assert!(ignored_matching(&result, "attack rating 54%"));
        assert!(ignored_matching(&result, "purest souls"));
    }

    #[test]
    fn runeword_detection_survives_ocr_noise() {
        let noisy = AURORA
            .replacen("AURORA'S MIGHT", "T4 MJ\nAURORA'S MICHT", 1)
            .replacen("RUNEWORD 1-HANDED MACE", "RUNEW0RD I-HANDED MACE", 1)
            .replacen("(YMN, CO, QI)", "1YMN, CO, Q1)", 1);
        let result = parse(&noisy);
        let item = result.equipped.as_ref().expect("runeword item");
        assert_eq!(item.runeword_id.as_deref(), Some("rw_aurora_s_might"));
        assert!(item.base_id.starts_with("base_mace_"));
        assert_eq!(item.socketed.len(), 3);
    }

    #[test]
    fn parsed_item_converts_into_an_equipped_item() {
        let parsed = ParsedItem {
            base_id: "helmet_angelic_lucifers_crown".into(),
            affixes: vec![ParsedAffix {
                affix_id: "affix_a".into(),
                tier: 3,
                roll: 0.5,
            }],
            socket_count: 2,
            socketed: vec![None, Some("gem_x".into())],
            socket_types: vec!["normal", "rainbow"],
            stars: 0,
            forged_mods: Some(vec![ParsedAffix {
                affix_id: "crystal_b".into(),
                tier: 1,
                roll: 1.,
            }]),
            augment: Some(ParsedAugment {
                id: "augment_c".into(),
                level: 4,
            }),
            implicit_overrides: Some(BTreeMap::from([("strength".to_string(), 42.)])),
            skill_bonus_overrides: None,
            all_skills_class_id: Some("viking".into()),
            runeword_id: Some("rw_x".into()),
        };
        let item = crate::calc::types::EquippedItem::from(parsed);
        assert_eq!(item.runeword_id.as_deref(), Some("rw_x"));
        assert_eq!(item.base_id, "helmet_angelic_lucifers_crown");
        assert_eq!(item.stars, Some(0));
        assert_eq!(item.affixes[0].affix_id, "affix_a");
        assert_eq!(item.affixes[0].tier, 3);
        assert_eq!(item.affixes[0].roll, 0.5);
        assert_eq!(item.affixes[0].custom_value, None);
        assert_eq!(item.socket_count, 2);
        assert_eq!(item.socketed, vec![None, Some("gem_x".to_string())]);
        assert_eq!(
            item.socket_types,
            vec![
                crate::calc::types::SocketType::Normal,
                crate::calc::types::SocketType::Rainbow
            ]
        );
        assert_eq!(item.forged_mods[0].affix_id, "crystal_b");
        assert_eq!(
            item.augment.as_ref().map(|a| (a.id.as_str(), a.level)),
            Some(("augment_c", 4))
        );
        assert_eq!(item.implicit_overrides["strength"], 42.);
        assert!(item.skill_bonus_overrides.is_empty());
        assert_eq!(item.all_skills_class_id.as_deref(), Some("viking"));
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
