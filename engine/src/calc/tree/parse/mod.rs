use std::collections::HashMap;
use std::sync::Mutex;

use std::sync::LazyLock;
use regex::{Captures, Regex};

#[macro_use]
mod macros;

mod conversion;
mod rules;

pub(crate) use conversion::*;
pub(crate) use rules::RULES;

// ---------- types ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelfConditionKey {
    CritChanceBelow40,
    LifeBelow40,
}

impl SelfConditionKey {
    pub fn as_str(self) -> &'static str {
        match self {
            SelfConditionKey::CritChanceBelow40 => "crit_chance_below_40",
            SelfConditionKey::LifeBelow40 => "life_below_40",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SelfConditionKey::CritChanceBelow40 => "Critical Strike Chance is below 40% (auto)",
            SelfConditionKey::LifeBelow40 => "Current Life is below 40% of Maximum",
        }
    }
}

pub const SELF_CONDITION_KEYS: &[SelfConditionKey] = &[
    SelfConditionKey::CritChanceBelow40,
    SelfConditionKey::LifeBelow40,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisableTarget {
    LifeReplenish,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMod {
    pub key: String,
    pub value: f64,
    pub self_condition: Option<SelfConditionKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConvertKind {
    Stat,
    Attribute,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedConversion {
    pub from_key: String,
    pub from_kind: ConvertKind,
    pub to_key: String,
    pub to_kind: ConvertKind,
    pub pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDisable {
    pub target: DisableTarget,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedMeta {
    Convert(ParsedConversion),
    Disable(ParsedDisable),
}

pub const ELEMENTS: &[&str] = &["arcane", "cold", "fire", "lightning", "poison"];

// ---------- rule shapes ----------

type ModBuild = fn(&Captures<'_>) -> Option<ParsedMod>;
type ConversionBuild = fn(&Captures<'_>) -> Option<ParsedConversion>;

pub(crate) struct ParseRule {
    pub test: Regex,
    pub build: ModBuild,
}

pub(crate) struct ConversionRule {
    pub test: Regex,
    pub build: ConversionBuild,
}

pub(crate) struct DisableRule {
    pub test: Regex,
    pub target: DisableTarget,
}

pub(crate) static CONVERSION_TARGET_STATS: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        [
            ("magic skill damage", "magic_skill_damage"),
            ("ranged projectile damage", "ranged_projectile_damage"),
            ("damage return", "damage_return"),
            ("increased maximum life", "increased_life"),
            ("maximum life", "life"),
            ("maximum mana", "mana"),
            ("physical damage", "additive_physical_damage"),
            ("attack damage", "attack_damage"),
            ("ranged physical damage", "ranged_physical_per_500_mana"),
            ("increased life", "increased_life"),
            ("increased damage", "enhanced_damage"),
        ]
        .into_iter()
        .collect()
    });

// ---------- regex helpers (lazily compiled once) ----------

static SELF_CONDITION_SUFFIXES: LazyLock<Vec<(Regex, SelfConditionKey)>> = LazyLock::new(|| {
    vec![
        (
            Regex::new(r"(?i)\s+when\s+critical\s+strike\s+chance\s+is\s+below\s+40%$").unwrap(),
            SelfConditionKey::CritChanceBelow40,
        ),
        (
            Regex::new(
                r"(?i)\s+(?:when|while)\s+(?:current\s+life\s+is\s+)?below\s+40%(?:\s+of)?\s+maximum\s+life$",
            )
            .unwrap(),
            SelfConditionKey::LifeBelow40,
        ),
    ]
});

static WEAPON_CONTEXT_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\s+(?:when|while)\s+(?:using|wielding|dual\s+wielding)\s+(?:a\s+|an\s+)?(?:two\s+handed\s+)?(?:melee\s+)?(?:axe[s]?|sword[s]?|bow[s]?|gun[s]?|wand[s]?|staff(?:\s+or\s+a\s+cane)?|cane|shield[s]?|throwing\s+weapon[s]?|two\s+handed\s+weapon|two\s+handed\s+melee\s+weapon)$",
    )
    .unwrap()
});

pub(crate) fn strip_weapon_context(line: &str) -> String {
    let stripped = WEAPON_CONTEXT_SUFFIX.replace(line, "");
    stripped.trim_end().to_string()
}

pub(crate) fn strip_self_condition(line: &str) -> (String, Option<SelfConditionKey>) {
    for (re, cond) in SELF_CONDITION_SUFFIXES.iter() {
        if re.is_match(line) {
            let stripped = re.replace(line, "");
            return (stripped.trim_end().to_string(), Some(*cond));
        }
    }
    (line.to_string(), None)
}

pub(crate) fn num(s: &str) -> f64 {
    // Mirror JS Number(): strip leading '+', return NaN on parse failure.
    let cleaned = s.strip_prefix('+').unwrap_or(s);
    cleaned.parse::<f64>().unwrap_or(f64::NAN)
}


// ---------- caches ----------

static MOD_CACHE: LazyLock<Mutex<HashMap<String, Option<ParsedMod>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static META_CACHE: LazyLock<Mutex<HashMap<String, Option<ParsedMeta>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// The optimizer re-classifies the same node lines for every candidate it scores,
// so an uncached classify would rerun every rule regex millions of times per run.
static CLASS_CACHE: LazyLock<Mutex<HashMap<String, TreeLineClass>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ---------- dispatchers ----------

enum ModRuleOutcome {
    Stat(ParsedMod),
    /// A null-rule matched: line is recognized but intentionally carries no stat.
    Reject,
    NoMatch,
}

fn match_mod_rules(trimmed: &str) -> ModRuleOutcome {
    let (base, self_condition) = strip_self_condition(trimmed);
    let mut candidates: Vec<String> = vec![base.clone()];
    let stripped = strip_weapon_context(&base);
    if stripped != base {
        candidates.push(stripped);
    }

    for candidate in &candidates {
        for rule in RULES.iter() {
            let Some(caps) = rule.test.captures(candidate) else {
                continue;
            };
            match (rule.build)(&caps) {
                None => {
                    // Explicit reject: abort, do not try further rules.
                    return ModRuleOutcome::Reject;
                }
                Some(built) if built.value.is_finite() => {
                    return ModRuleOutcome::Stat(ParsedMod {
                        self_condition: self_condition.or(built.self_condition),
                        ..built
                    });
                }
                Some(_) => {
                    // Non-finite value (e.g. NaN from bad capture) — try next rule.
                    continue;
                }
            }
        }
    }
    ModRuleOutcome::NoMatch
}

pub fn parse_tree_node_mod(line: &str) -> Option<ParsedMod> {
    let trimmed = line.trim();
    {
        let cache = MOD_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(trimmed) {
            return cached.clone();
        }
    }
    let out = match match_mod_rules(trimmed) {
        ModRuleOutcome::Stat(m) => Some(m),
        ModRuleOutcome::Reject | ModRuleOutcome::NoMatch => None,
    };
    MOD_CACHE
        .lock()
        .unwrap()
        .insert(trimmed.to_string(), out.clone());
    out
}

#[derive(Debug, Clone)]
pub enum TreeLineClass {
    Stat(ParsedMod),
    Meta(ParsedMeta),
    RecognizedNoStat,
    Unknown,
}

pub fn classify_tree_node_line(line: &str) -> TreeLineClass {
    let trimmed = line.trim();
    {
        let cache = CLASS_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(trimmed) {
            return cached.clone();
        }
    }
    let out = classify_uncached(trimmed);
    CLASS_CACHE
        .lock()
        .unwrap()
        .insert(trimmed.to_string(), out.clone());
    out
}

fn classify_uncached(trimmed: &str) -> TreeLineClass {
    match match_mod_rules(trimmed) {
        ModRuleOutcome::Stat(m) => TreeLineClass::Stat(m),
        // Mod-then-meta flow: a stat-rejected line still gets a meta pass, so
        // meta-only lines (e.g. conversions) classify as parsed, not dropped.
        ModRuleOutcome::Reject => match parse_tree_node_meta(trimmed) {
            Some(meta) => TreeLineClass::Meta(meta),
            None => TreeLineClass::RecognizedNoStat,
        },
        ModRuleOutcome::NoMatch => match parse_tree_node_meta(trimmed) {
            Some(meta) => TreeLineClass::Meta(meta),
            None => TreeLineClass::Unknown,
        },
    }
}

pub fn parse_tree_node_meta(line: &str) -> Option<ParsedMeta> {
    let trimmed = line.trim();
    {
        let cache = META_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(trimmed) {
            return cached.clone();
        }
    }

    for rule in CONVERSION_RULES.iter() {
        if let Some(caps) = rule.test.captures(trimmed) {
            if let Some(out) = (rule.build)(&caps) {
                if out.pct.is_finite() {
                    let meta = ParsedMeta::Convert(out);
                    META_CACHE
                        .lock()
                        .unwrap()
                        .insert(trimmed.to_string(), Some(meta.clone()));
                    return Some(meta);
                }
            }
        }
    }
    for rule in DISABLE_RULES.iter() {
        if rule.test.is_match(trimmed) {
            let meta = ParsedMeta::Disable(ParsedDisable {
                target: rule.target,
            });
            META_CACHE
                .lock()
                .unwrap()
                .insert(trimmed.to_string(), Some(meta.clone()));
            return Some(meta);
        }
    }
    META_CACHE.lock().unwrap().insert(trimmed.to_string(), None);
    None
}


#[cfg(test)]
mod tests;
