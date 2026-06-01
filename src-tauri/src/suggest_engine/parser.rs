use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Mutex;
use std::collections::HashMap;

use super::types::{ParsedMod, ParsedMeta, ParsedConversion, ConversionKind, DisableTarget, SelfCondition};

const ELEMENTS: &[&str] = &["arcane", "cold", "fire", "lightning", "poison"];

fn parse_num(s: &str) -> f64 {
    let cleaned = s.trim_start_matches('+');
    cleaned.parse::<f64>().unwrap_or(0.0)
}

type BuildFn = Box<dyn Fn(&regex::Captures) -> Option<ParsedMod> + Send + Sync>;

pub struct ParseRule {
    pub re: Regex,
    pub build: BuildFn,
}

fn rule_simple(pattern: &str, key: &'static str) -> ParseRule {
    ParseRule {
        re: Regex::new(pattern).expect("invalid regex"),
        build: Box::new(move |caps| {
            Some(ParsedMod {
                key: key.to_string(),
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: None,
            })
        }),
    }
}

fn rule_total(pattern: &str, key: &'static str, key_more: &'static str) -> ParseRule {
    ParseRule {
        re: Regex::new(pattern).expect("invalid regex"),
        build: Box::new(move |caps| {
            let is_more = caps
                .get(2)
                .map(|m| !m.as_str().is_empty())
                .unwrap_or(false);
            Some(ParsedMod {
                key: if is_more { key_more.to_string() } else { key.to_string() },
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: None,
            })
        }),
    }
}

static SELF_CONDITION_CRIT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\s+when\s+critical\s+strike\s+chance\s+is\s+below\s+40%$").unwrap()
});
static SELF_CONDITION_LIFE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\s+(?:when|while)\s+(?:current\s+life\s+is\s+)?below\s+40%(?:\s+of)?\s+maximum\s+life$").unwrap()
});
static WEAPON_CONTEXT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\s+(?:when|while)\s+(?:using|wielding|dual\s+wielding)\s+(?:a\s+|an\s+)?(?:two\s+handed\s+)?(?:melee\s+)?(?:axe[s]?|sword[s]?|bow[s]?|gun[s]?|wand[s]?|staff(?:\s+or\s+a\s+cane)?|cane|shield[s]?|throwing\s+weapon[s]?|two\s+handed\s+weapon|two\s+handed\s+melee\s+weapon)$",
    )
    .unwrap()
});

fn strip_self_condition(line: &str) -> (String, Option<SelfCondition>) {
    if let Some(m) = SELF_CONDITION_CRIT.find(line) {
        let stripped = line[..m.start()].trim_end().to_string();
        return (stripped, Some(SelfCondition::CritChanceBelow40));
    }
    if let Some(m) = SELF_CONDITION_LIFE.find(line) {
        let stripped = line[..m.start()].trim_end().to_string();
        return (stripped, Some(SelfCondition::LifeBelow40));
    }
    (line.to_string(), None)
}

fn strip_weapon_context(line: &str) -> String {
    if let Some(m) = WEAPON_CONTEXT_RE.find(line) {
        line[..m.start()].trim_end().to_string()
    } else {
        line.to_string()
    }
}

static RULES: Lazy<Vec<ParseRule>> = Lazy::new(build_rules);

fn build_rules() -> Vec<ParseRule> {
    let elem_re = ELEMENTS.join("|");
    let mut v: Vec<ParseRule> = Vec::new();

    // ====================== ATTRIBUTES ======================
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Life$", "life"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Mana$", "mana"));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Maximum\s+Life$",
        "increased_life",
        "increased_life_more",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Maximum\s+Mana$",
        "increased_mana",
        "increased_mana_more",
    ));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)%\s+Increased\s+Mana$", "increased_mana"));

    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Strength$", "to_strength"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+Strength$", "to_strength"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Dexterity$", "to_dexterity"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+Dexterity$", "to_dexterity"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Intelligence$", "to_intelligence"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Energy$", "to_energy"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Vitality$", "to_vitality"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Armor$", "to_armor"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+All\s+Attributes$", "all_attributes"));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+All\s+Attributes$",
        "increased_all_attributes",
    ));

    v.push(ParseRule {
        re: Regex::new(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?(Strength|Dexterity|Intelligence|Energy|Vitality|Armor)$",
        )
        .unwrap(),
        build: Box::new(move |caps| {
            let attr = caps.get(3)?.as_str().to_lowercase();
            let is_more = caps.get(2).map(|m| !m.as_str().is_empty()).unwrap_or(false);
            let key = if is_more {
                format!("increased_{}_more", attr)
            } else {
                format!("increased_{}", attr)
            };
            Some(ParsedMod {
                key,
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: None,
            })
        }),
    });

    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Defense$", "defense"));

    // ====================== MOVEMENT / SPEED ======================
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Movement\s+Speed$",
        "movement_speed",
        "movement_speed_more",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Attack\s+Speed$",
        "increased_attack_speed",
        "increased_attack_speed_more",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Faster\s+Cast\s+Rate$",
        "faster_cast_rate",
        "faster_cast_rate_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+(?:to\s+)?Spell\s+Haste$",
        "skill_haste",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+to\s+Faster\s+Cast\s+Rate(?:\s+while\s+wielding\s+a\s+wand)?$",
        "faster_cast_rate",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Faster\s+Cast\s+Rate(?:\s+while\s+wielding\s+a\s+wand)?$",
        "faster_cast_rate",
        "faster_cast_rate_more",
    ));

    // ====================== CRIT ======================
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Critical\s+Strike\s+Damage$",
        "crit_damage",
        "crit_damage_more",
    ));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)%\s+Critical\s+Damage$", "crit_damage"));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+to\s+Critical\s+Strike\s+Chance$",
        "crit_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+Critically\s+Hit\s+with\s+Spells$",
        "spell_crit_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Spell\s+Critical\s+Damage$",
        "spell_crit_damage",
    ));

    // ====================== RESISTANCES ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+(?:to\s+)?All\s+Resistances$",
        "all_resistances",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)\s+to\s+(Total\s+)?All\s+Resistances$",
        "all_resistances",
        "all_resistances_more",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+to\s+(Total\s+)?All\s+Resistances$",
        "all_resistances",
        "all_resistances_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+to\s+Maximum\s+All\s+Resistances$",
        "max_all_resistances",
    ));

    let pat = format!(r"(?i)^([+\-\d.]+)%\s+to\s+({})\s+Resistance$", elem_re);
    v.push(ParseRule {
        re: Regex::new(&pat).unwrap(),
        build: Box::new(move |caps| {
            let el = caps.get(2)?.as_str().to_lowercase();
            Some(ParsedMod {
                key: format!("{}_resistance", el),
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: None,
            })
        }),
    });
    let pat = format!(
        r"(?i)^([+\-\d.]+)%\s+to\s+Maximum\s+({})\s+Resistance$",
        elem_re
    );
    v.push(ParseRule {
        re: Regex::new(&pat).unwrap(),
        build: Box::new(move |caps| {
            let el = caps.get(2)?.as_str().to_lowercase();
            Some(ParsedMod {
                key: format!("max_{}_resistance", el),
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: None,
            })
        }),
    });
    let pat = format!(r"(?i)^([+\-\d.]+)%\s+to\s+({})\s+Absorb$", elem_re);
    v.push(ParseRule {
        re: Regex::new(&pat).unwrap(),
        build: Box::new(move |caps| {
            let el = caps.get(2)?.as_str().to_lowercase();
            Some(ParsedMod {
                key: format!("{}_absorption", el),
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: None,
            })
        }),
    });

    // ====================== ELEMENTAL SKILL DAMAGE ======================
    let pat = format!(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?({})\s+Skill\s+Damage$",
        elem_re
    );
    v.push(ParseRule {
        re: Regex::new(&pat).unwrap(),
        build: Box::new(move |caps| {
            let el = caps.get(3)?.as_str().to_lowercase();
            let is_more = caps.get(2).map(|m| !m.as_str().is_empty()).unwrap_or(false);
            let key = if is_more {
                format!("{}_skill_damage_more", el)
            } else {
                format!("{}_skill_damage", el)
            };
            Some(ParsedMod {
                key,
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: None,
            })
        }),
    });

    let pat = format!(
        r"(?i)^([+\-\d.]+)\s+to\s+({})\s+Skill\s+Damage$",
        elem_re
    );
    v.push(ParseRule {
        re: Regex::new(&pat).unwrap(),
        build: Box::new(move |caps| {
            let el = caps.get(2)?.as_str().to_lowercase();
            Some(ParsedMod {
                key: format!("flat_{}_skill_damage", el),
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: None,
            })
        }),
    });

    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Magic\s+Skill\s+Damage$",
        "magic_skill_damage",
        "magic_skill_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Magic\s+Skill\s+Damage$",
        "flat_skill_damage",
    ));

    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Spell\s+Damage$",
        "spell_damage",
    ));

    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Area\s+of\s+Effect(?:\s+(?:skill\s+)?radius(?:\s+of\s+all\s+skills)?)?$",
        "area_of_effect",
        "area_of_effect_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Spell\s+Duration$",
        "spell_duration_pct",
    ));

    // ====================== SKILLS BONUSES ======================
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+All\s+Skills$", "all_skills"));
    let pat = format!(r"(?i)^([+\-\d.]+)\s+to\s+({})\s+Skills$", elem_re);
    v.push(ParseRule {
        re: Regex::new(&pat).unwrap(),
        build: Box::new(move |caps| {
            let el = caps.get(2)?.as_str().to_lowercase();
            Some(ParsedMod {
                key: format!("{}_skills", el),
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: None,
            })
        }),
    });
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Physical\s+Skills$", "physical_skills"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Explosion\s+Skills$", "explosion_skills"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Summon\s+Skills$", "summon_skills"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Aura\s+Skills$", "aura_skills"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Shield\s+Skills$", "shield_skills"));

    // ====================== ATTACK & DAMAGE ======================
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Attack\s+Rating$", "attack_rating"));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Attack\s+Rating$",
        "attack_rating_pct",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Physical\s+Damage$",
        "additive_physical_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+a\s+deadly\s+blow$",
        "deadly_blow",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+cast\s+an\s+additional\s+time\s+when\s+casting$",
        "multicast_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+while\s+Unarmed$",
        "damage_unarmed",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Explosion\s+(?:area\s+of\s+effect\s+)?[Dd]amage$",
        "explosion_damage",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Enhanced\s+Damage$",
        "enhanced_damage",
        "enhanced_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+to\s+(?:Melee\s+|Ranged\s+)?Enhanced\s+Damage$",
        "enhanced_damage",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Physical\s+Damage$",
        "enhanced_damage",
        "enhanced_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+(?:Maximum|Minimum)\s+Damage(?:\s+when\s+wielding\s+a\s+shield)?$",
        "attack_damage",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Defense$",
        "enhanced_defense",
        "enhanced_defense",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Ranged\s+Projectile\s+Damage$",
        "ranged_projectile_damage",
        "ranged_projectile_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Projectile\s+Damage$",
        "ranged_projectile_damage",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Area\s+of\s+Effect\s+Spell\s+Damage$",
        "spell_aoe_damage",
        "spell_aoe_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+(?:Increased\s+)?Spell\s+Area\s+of\s+Effect\s+Radius$",
        "spell_aoe_radius",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Spell\s+Projectile\s+Damage$",
        "spell_projectile_damage",
        "two_handed_spell_projectile_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+(?:Increased\s+)?Spell\s+Projectile\s+Size$",
        "spell_projectile_size",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+Additional\s+Spell\s+Projectile$",
        "additional_spell_projectile",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Extra\s+Spell\s+Projectiles$",
        "extra_spell_projectiles_pct",
    ));

    // ====================== ELEMENTAL BREAK / ARMOR BREAK ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Elemental\s+Break\s+Inflicted\s+on\s+hit$",
        "elemental_break_on_strike",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Elemental\s+Break\s+Inflicted\s+on\s+spell\s+hit$",
        "elemental_break_on_spell",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+(?:to\s+)?Elemental\s+Break$",
        "elemental_break",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Armor\s+Break(?:\s+Inflicted\s+on\s+hit)?$",
        "armor_break_on_strike",
    ));

    // ====================== STEAL / LEECH ======================
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Life\s+Steal$",
        "life_steal",
        "life_steal_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Mana\s+Steal$",
        "mana_steal",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Rate\s+of\s+Life\s+Steal$",
        "life_steal_rate",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increases\s+Rate\s+of\s+Mana\s+Steal$",
        "mana_steal_rate",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+to\s+Spell\s+Mana\s+Leech$",
        "mana_steal",
    ));

    // ====================== DAMAGE MITIGATION / REDUCTION ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Damage\s+Mitigation$",
        "damage_mitigation",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+of\s+Damage\s+Taken\s+Mitigated$",
        "damage_mitigation",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+of\s+Incoming\s+Damage\s+is\s+mitigated$",
        "damage_mitigation",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+to\s+Physical\s+Damage\s+Reduction$",
        "physical_damage_reduction",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+of\s+Damage\s+Taken\s+returned\s+to\s+the\s+Attacker$",
        "damage_return",
    ));
    v.push(rule_simple(
        r"(?i)^All\s+Damage\s+Taken\s+Reduced\s+by\s+([+\-\d.]+)%$",
        "all_damage_taken_reduced_pct",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Damage\s+Returned$",
        "damage_return",
    ));

    // ====================== REPLENISH ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+Life\s+Replenished\s+per\s+second$",
        "life_replenish",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Maximum\s+Life\s+Replenished\s+(?:Per|per)\s+[Ss]econd$",
        "life_replenish_pct",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+replenished\s+per\s+second$",
        "life_replenish_pct",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+(Total\s+)?(?:to\s+)?Mana\s+Replenish$",
        "mana_replenish",
        "mana_replenish_more",
    ));

    // ====================== AILMENTS ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Hit\s+Recovery$",
        "faster_hit_recovery",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Bleed(?:ing)?\s+Damage$",
        "increased_bleeding_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Bleed(?:ing)?\s+Frequency$",
        "increased_bleeding_frequency",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Poison(?:ed)?\s+Damage$",
        "increased_poisoned_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Burning\s+Damage$",
        "increased_burning_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Stasis\s+Damage$",
        "increased_stasis_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Ailment(?:\s+Tick)?\s+Frequency$",
        "increased_ailment_frequency",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Bleeding\s+on\s+hit$",
        "chance_inflict_bleeding",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Poison(?:ed)?\s+on\s+hit$",
        "chance_inflict_poisoned",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Burning\s+on\s+hit$",
        "chance_inflict_burning",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Stasis\s+on\s+hit$",
        "chance_inflict_stasis",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Bleed(?:ing)?\s+Stacks$",
        "max_bleed_stacks",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Poison(?:ed)?\s+Stacks$",
        "max_poisoned_stacks",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Burning\s+Stacks$",
        "max_burning_stacks",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Stasis\s+Stacks$",
        "max_stasis_stacks",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Rage\s+Stacks$",
        "max_rage_stacks",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Colossus\s+Stacks$",
        "max_colossus_stacks",
    ));

    // ====================== MISC ======================
    v.push(rule_simple(r"(?i)^([+\-\d.]+)\s+to\s+Light\s+Radius$", "light_radius"));
    v.push(rule_simple(r"(?i)^([+\-\d.]+)%\s+to\s+Magic\s+Find$", "magic_find"));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+(?:Increased\s+)?Experience\s+Gain$",
        "experience_gain",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Decreased\s+Crowd\s+Control\s+Diminish$",
        "cc_diminish_decrease",
    ));

    // ====================== SENTRY / SUMMON ======================
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Sentry\s+Damage(?:\s+but\s+you\s+can\s+no\s+longer\s+deal\s+damage\s+your\s+self)?$",
        "sentry_damage",
        "sentry_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Sentry\s+Attack\s+Speed$",
        "sentry_attack_speed",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Sentry\s+Duration$",
        "sentry_duration",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+Sentries\s+to\s+fire\s+an\s+additional\s+projectile$",
        "sentry_extra_projectile_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Damage$",
        "summon_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Attack\s+Speed$",
        "summon_attack_speed",
    ));

    // ====================== AURA / SHIELD ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Damaging\s+Aura\s+Effectiveness$",
        "damaging_aura_effectiveness",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(?:Damaging\s+)?Aura\s+Radius$",
        "damaging_aura_radius",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(?:Total\s+)?(?:Damage\s+by\s+)?Shield\s+Skill(?:\s+|s\s+)Damage$",
        "shield_skill_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+by\s+Shield\s+Skills$",
        "shield_skill_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+radius\s+of\s+Shield\s+Skills$",
        "shield_skill_radius",
    ));

    // ====================== SUMMON DAMAGE EXTRAS ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+to\s+Summon\s+Maximum\s+Life$",
        "summon_life",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Melee\s+Damage$",
        "summon_melee_damage",
    ));
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Summon\s+Projectile\s+Damage$",
        "summon_projectile_damage",
        "summon_projectile_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Total\s+Summon\s+Projectile\s+Damage$",
        "summon_projectile_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Projectile\s+Size$",
        "summon_projectile_size",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Attack\s+Radius$",
        "summon_attack_radius",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Splash\s+Damage(?:\s+around\s+the\s+target)?$",
        "summon_splash_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Summon\s+Amount$",
        "summon_max_amount",
    ));

    // ====================== ORBITAL SKILLS ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Damage$",
        "orbital_skill_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+(?:Increased\s+)?Total\s+Orbital\s+Spell\s+Damage$",
        "orbital_skill_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Orbital\s+Spell\s+Damage$",
        "orbital_skill_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Speed$",
        "orbital_skill_speed",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Duration$",
        "orbital_skill_duration",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Orbital\s+Spell\s+Duration$",
        "orbital_skill_duration",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Size$",
        "orbital_skill_size",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Orbital\s+Spell\s+Size$",
        "orbital_skill_size",
    ));

    // ====================== AILMENT DAMAGE GLOBAL ======================
    v.push(rule_total(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Ailment\s+Damage$",
        "ailment_damage_all",
        "ailment_damage_all_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+of\s+Skills\s+Damage\s+added\s+to\s+the\s+Ailments\s+damage$",
        "skill_damage_to_ailments",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Poison(?:ed)?\s+Frequency$",
        "increased_poisoned_frequency",
    ));

    // ====================== WEAPON-CONDITIONAL DAMAGE ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+when\s+Dual\s+Wielding$",
        "damage_dual_wield",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Damage\s+when\s+Dual\s+Wielding$",
        "damage_dual_wield_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Damage(?:\s+when\s+using\s+a\s+Shield)?$",
        "damage_with_shield",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Damage\s+to\s+Monsters\s+far\s+away\s+from\s+you$",
        "damage_far",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Damage\s+dealt\s+to\s+monsters\s+at\s+long\s+range\s+but\s+deal\s+less\s+damage\s+to\s+monsters\s+close\s+to\s+you$",
        "damage_far",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+to\s+monsters\s+close\s+to\s+you\s+but\s+deal\s+less\s+damage\s+to\s+monsters\s+far\s+away$",
        "damage_close",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+when\s+wielding\s+an\s+Axe$",
        "damage_with_axe",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Attack\s+Rating\s+when\s+wielding\s+an\s+Axe$",
        "attack_rating_with_axe_pct",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(?:Melee\s+Attack\s+Damage|Melee\s+Damage)\s+when\s+using\s+a\s+Two\s+Handed\s+Melee\s+Weapon$",
        "damage_with_two_handed",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+(?:Damage\s+when\s+using\s+a\s+Two\s+Handed\s+Weapon|Ailment\s+Damage\s+when\s+using\s+a\s+Two\s+Handed\s+Weapon)$",
        "damage_with_two_handed",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Spell\s+Projectile\s+Damage\s+when\s+wielding\s+a\s+Staff\s+or\s+a\s+Cane$",
        "two_handed_spell_projectile_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Magic\s+Skill\s+Damage\s+while\s+wielding\s+a\s+Wand$",
        "magic_skill_damage_with_wand",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Critical\s+Strike\s+Damage\s+when\s+using\s+a\s+Shield$",
        "crit_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Projectile\s+Damage\s+when\s+using\s+a\s+Gun$",
        "damage_with_gun",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+when\s+using\s+a\s+Bow$",
        "damage_with_bow",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Projectile\s+Damage\s+when\s+using\s+a\s+Throwing\s+Weapon$",
        "damage_with_throwing",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Minimum\s+Damage\s+when\s+wielding\s+a\s+shield$",
        "min_damage_with_shield",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Maximum\s+Damage\s+when\s+wielding\s+a\s+shield$",
        "max_damage_with_shield",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Attack\s+Range(?:\s+when\s+using\s+a\s+Shield)?$",
        "melee_range",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Attack\s+Radius$",
        "attack_radius",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Attack\s+Speed\s+at\s+Full\s+Life$",
        "attack_speed_full_life",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+When\s+at\s+full\s+Life\s+gain\s+increased\s+total\s+Attack\s+Speed\s+and\s+Damage\s+Dealt\s+but\s+also\s+decreased\s+Damage\s+Reduction\s+and\s+All\s+Resistances$",
        "attack_speed_full_life",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+(?:to\s+)?Increased\s+Total\s+Attack\s+Rating$",
        "attack_rating_pct",
    ));

    // ====================== STAT-CONDITIONAL DAMAGE ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+per\s+stack\s+of\s+rage$",
        "ranged_damage_per_rage_stack",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Damage\s+Increased\s+per\s+Stack\s+of\s+Rage$",
        "damage_per_rage_stack",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Magic\s+Skills?\s+Damage\s+per\s+750\s+points\s+in\s+Mana$",
        "magic_skill_damage_per_750_mana",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Charging\s+Damage\s+per\s+point\s+in\s+Strength$",
        "charging_damage_per_strength",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Faster\s+Cast\s+Rate\s+per\s+Stack\s+of\s+Wizardry$",
        "wizardry_cast_rate",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Colossus\s+damage$",
        "colossus_damage",
    ));

    v.push(ParseRule {
        re: Regex::new(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Ranged\s+)?Physical\s+Damage\s+per\s+500\s+mana$",
        )
        .unwrap(),
        build: Box::new(move |caps| {
            let is_ranged = caps.get(2).map(|m| !m.as_str().is_empty()).unwrap_or(false);
            Some(ParsedMod {
                key: if is_ranged {
                    "ranged_physical_per_500_mana".to_string()
                } else {
                    "additive_physical_per_500_mana".to_string()
                },
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: None,
            })
        }),
    });

    // ====================== EXPLOSION / AOE / AREA ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Explosion\s+(?:area\s+of\s+effect\s+)?radius$",
        "explosion_aoe",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+damage\s+dealt\s+by\s+Area\s+of\s+Effect\s+skills$",
        "area_of_effect",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Spell\s+Area\s+of\s+Effect\s+Radius$",
        "spell_aoe_radius",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Spell\s+Projectile\s+Size$",
        "spell_projectile_size",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s*Seconds?\s+to\s+Spell\s+Duration$",
        "spell_duration_seconds",
    ));

    // ====================== CRUSHING / DEADLY BLOWS ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Crushing\s+Blow\s+Chance$",
        "crushing_blow_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Crushing\s+Blow\s+Effectiveness$",
        "crushing_blow_effectiveness",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+effectiveness\s+of\s+Deadly\s+Blow$",
        "deadly_blow_effectiveness",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+deal\s+area\s+of\s+effect\s+damage\s+with\s+deadly\s+blow$",
        "deadly_blow_aoe_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Deadly\s+Blow\s+Area\s+of\s+Effect\s+Damage$",
        "deadly_blow_aoe_size",
    ));

    // ====================== PROJECTILE / FORK ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+Projectiles\s+to\s+Fork(?:\s+on\s+Hit)?$",
        "fork_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+Extra\s+Forking\s+Projectile$",
        "additional_forking_projectiles",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+Extra\s+Projectile(?:\s+Fired\s+with\s+Ranged\s+Attacks)?$",
        "extra_ranged_projectiles",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+Additional\s+projectile\s+fired\s+when\s+performing\s+a\s+ranged\s+attack$",
        "additional_projectile_fixed",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+fire\s+an\s+additional\s+projectile\s+when\s+performing\s+a\s+ranged\s+attack$",
        "extra_ranged_projectile_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+on\s+ranged\s+attack\s+to\s+perform\s+it\s+an\s+additional\s+time$",
        "ranged_extra_attack_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+unleash\s+an\s+additional\s+attack\s+or\s+projectile\s+on\s+attack$",
        "extra_attack_or_projectile_chance",
    ));

    // ====================== HOMING MISSILE / SPECIAL SKILLS ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+when\s+attacking\s+to\s+fire\s+a\s+homing\s+missile$",
        "homing_missile_attack_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Homing\s+Missile\s+Damage$",
        "homing_missile_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+additional\s+Homing\s+Missile$",
        "homing_missile_count",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Death\s+Explosion\s+damage$",
        "death_explosion_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Void\s+Blast\s+Damage$",
        "void_blast_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Guardian\s+Damage$",
        "guardian_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Guardian\s+Attack\s+Speed$",
        "guardian_attack_speed",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Sand\s+Beam\s+Damage$",
        "sand_beam_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Knock\s+Up\s+Damage$",
        "summon_knock_up_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Weapon\s+Throw\s+Damage$",
        "weapon_throw_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Temporal\s+Echo\s+Damage$",
        "temporal_echo_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Stampede\s+Damage\s+based\s+on\s+your\s+physical\s+damage$",
        "stampede_damage_per_physical",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Desert\s+Ripple\s+Damage$",
        "sand_ripple_damage",
    ));

    // ====================== MISC DAMAGE GLOBAL ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Total\s+Damage\s+Dealt(?:\s+and\s+Damage\s+Taken)?$",
        "total_damage_dealt_and_taken",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+and\s+Increased\s+Damage\s+Taken$",
        "damage_dealt_and_taken_amp",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+dealt\s+to\s+monsters\s+with\s+Crowd\s+Control\s+Immunity$",
        "damage_to_cc_immune",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Elemental\s+Weakness$",
        "elemental_weakness",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+of\s+Target\s+Defense\s+Ignored$",
        "defense_ignored",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+splash\s+damage\s+to\s+monsters\s+around\s+the\s+target$",
        "splash_damage_around_target",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Bleed\s+AoE\s+Damage$",
        "bleed_aoe_damage",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+Lightning\s+Damage\s+scaling\s+from\s+all\s+elemental\s+sources$",
        "lightning_per_element",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Melee\s+Enhanced\s+Damage$",
        "enhanced_damage_melee_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Ranged\s+Enhanced\s+Damage$",
        "enhanced_damage_ranged_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Damage\s+and\s+Decreased\s+Cast\s+Rate\s+from\s+over\s+heat$",
        "enhanced_damage_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Attack\s+Speed\s+&\s+Damage\s+and\s+Increased\s+Damage\s+Reduction\s+&\s+All\s+Resistances$",
        "increased_attack_speed",
    ));

    // ====================== SELF-CONDITION DAMAGE ======================
    v.push(ParseRule {
        re: Regex::new(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+when\s+Critical\s+Strike\s+Chance\s+is\s+below\s+40%$",
        )
        .unwrap(),
        build: Box::new(move |caps| {
            Some(ParsedMod {
                key: "ranged_projectile_damage".to_string(),
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: Some(super::types::SelfCondition::CritChanceBelow40),
            })
        }),
    });
    v.push(ParseRule {
        re: Regex::new(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Physical\s+Damage\s+while\s+below\s+40%\s+Maximum\s+Life$",
        )
        .unwrap(),
        build: Box::new(move |caps| {
            Some(ParsedMod {
                key: "enhanced_damage".to_string(),
                value: parse_num(caps.get(1)?.as_str()),
                self_condition: Some(super::types::SelfCondition::LifeBelow40),
            })
        }),
    });

    // ====================== LIFE / MANA / SUPPRESS / REPLENISH ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Life\s+Replenish(?:\s+when\s+current\s+life\s+is\s+below\s+40%\s+of\s+Maximum\s+Life)?$",
        "life_replenish_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Life\s+Replenish$",
        "life_replenish_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+of\s+Damage\s+Replenished\s+as\s+Life\s+when\s+struck$",
        "damage_to_life_replenish_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+replenished\s+when\s+struck$",
        "damage_to_life_replenish_chance",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Mana\s+regenerated\s+per\s+second$",
        "mana_regen_per_second",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+to\s+Maximum\s+Magic\s+Damage\s+Reduction$",
        "max_magic_damage_reduction",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+to\s+Maximum\s+Physical\s+Damage\s+Reduction$",
        "max_physical_damage_reduction",
    ));

    // ====================== DAMAGE RETURN VARIANTS ======================
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Damage\s+Return$",
        "damage_return_more",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Damage\s+Returned\s+against\s+Bosses$",
        "damage_returned_against_bosses",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)\s+to\s+Damage\s+Returned\s+when\s+wielding\s+a\s+Shield$",
        "damage_return",
    ));
    v.push(rule_simple(
        r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+unleash\s+piercing\s+spikes\s+outwards\s+when\s+struck$",
        "damage_return_more",
    ));

    v
}

static PARSE_CACHE: Lazy<Mutex<HashMap<String, Option<ParsedMod>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn parse_tree_node_mod(line: &str) -> Option<ParsedMod> {
    if let Some(cached) = PARSE_CACHE.lock().ok().and_then(|c| c.get(line).cloned()) {
        return cached;
    }
    let (base, self_condition) = strip_self_condition(line);
    let base = strip_weapon_context(&base);
    let result = RULES.iter().find_map(|r| {
        r.re.captures(&base).and_then(|caps| {
            (r.build)(&caps).map(|mut m| {
                m.self_condition = self_condition;
                m
            })
        })
    });
    if let Ok(mut cache) = PARSE_CACHE.lock() {
        cache.insert(line.to_string(), result.clone());
    }
    result
}

// ====================== CONVERSIONS ======================

type MetaBuildFn = Box<dyn Fn(&regex::Captures) -> Option<ParsedMeta> + Send + Sync>;

pub struct ConversionRule {
    pub re: Regex,
    pub build: MetaBuildFn,
}

fn conv(
    pattern: &str,
    from_key: &'static str,
    from_kind: ConversionKind,
    to_key: &'static str,
    to_kind: ConversionKind,
    pct_group: usize,
) -> ConversionRule {
    ConversionRule {
        re: Regex::new(pattern).expect("invalid regex"),
        build: Box::new(move |caps| {
            Some(ParsedMeta::Convert(ParsedConversion {
                from_key: from_key.to_string(),
                from_kind,
                to_key: to_key.to_string(),
                to_kind,
                pct: parse_num(caps.get(pct_group)?.as_str()),
            }))
        }),
    }
}

static CONVERSION_RULES: Lazy<Vec<ConversionRule>> = Lazy::new(|| {
    vec![
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Attack\s+Speed\s+is\s+added\s+to\s+Magic\s+Skill\s+Damage$",
            "increased_attack_speed",
            ConversionKind::Stat,
            "magic_skill_damage",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+Increased\s+Fire\s+Resistance\s+is\s+added\s+to\s+Fire\s+Skill\s+Damage$",
            "fire_resistance",
            ConversionKind::Stat,
            "fire_skill_damage",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+Increased\s+Cold\s+Resistance\s+is\s+added\s+to\s+Cold\s+Skill\s+Damage$",
            "cold_resistance",
            ConversionKind::Stat,
            "cold_skill_damage",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+Increased\s+Lightning\s+Resistance\s+is\s+added\s+to\s+Lightning\s+Skill\s+Damage$",
            "lightning_resistance",
            ConversionKind::Stat,
            "lightning_skill_damage",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+Increased\s+Poison\s+Resistance\s+is\s+added\s+to\s+Poison\s+Skill\s+Damage$",
            "poison_resistance",
            ConversionKind::Stat,
            "poison_skill_damage",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+Increased\s+Arcane\s+Resistance\s+is\s+added\s+to\s+Arcane\s+Skill\s+Damage$",
            "arcane_resistance",
            ConversionKind::Stat,
            "arcane_skill_damage",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Dexterity\s+is\s+added\s+to\s+Ranged\s+Projectile\s+Damage$",
            "dexterity",
            ConversionKind::Attribute,
            "ranged_projectile_damage",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Strength\s+is\s+added\s+to\s+(?:Weapon\s+)?Damage(?:\s+\(Unarmed\))?$",
            "strength",
            ConversionKind::Attribute,
            "enhanced_damage",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Defense\s+is\s+added\s+to\s+Life$",
            "defense",
            ConversionKind::Stat,
            "life",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Mana\s+is\s+added\s+to\s+Magic\s+Skill\s+Damage$",
            "mana",
            ConversionKind::Stat,
            "magic_skill_damage",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Movement\s+Speed\s+is\s+added\s+to\s+Attack\s+Damage$",
            "movement_speed",
            ConversionKind::Stat,
            "enhanced_damage",
            ConversionKind::Stat,
            1,
        ),
        conv(
            r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Area\s+of\s+Effect\s+is\s+added\s+to\s+Spell\s+Damage$",
            "area_of_effect",
            ConversionKind::Stat,
            "spell_damage",
            ConversionKind::Stat,
            1,
        ),
    ]
});

static DISABLE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^You\s+can\s+no\s+longer\s+Life\s+Replenish$").unwrap());

static META_CACHE: Lazy<Mutex<HashMap<String, Option<ParsedMeta>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn parse_tree_node_meta(line: &str) -> Option<ParsedMeta> {
    if let Some(cached) = META_CACHE.lock().ok().and_then(|c| c.get(line).cloned()) {
        return cached;
    }
    let result = CONVERSION_RULES
        .iter()
        .find_map(|r| r.re.captures(line).and_then(|caps| (r.build)(&caps)))
        .or_else(|| {
            if DISABLE_RE.is_match(line) {
                Some(ParsedMeta::Disable {
                    target: DisableTarget::LifeReplenish,
                })
            } else {
                None
            }
        });
    if let Ok(mut cache) = META_CACHE.lock() {
        cache.insert(line.to_string(), result.clone());
    }
    result
}
