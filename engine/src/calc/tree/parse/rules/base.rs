use super::super::*;

pub(super) fn rules() -> Vec<ParseRule> {
    vec![
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Life$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "life".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Mana$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "mana".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Maximum\s+Life$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: if m.get(2).is_some() {
                        "increased_life_more".to_string()
                    } else {
                        "increased_life".to_string()
                    },
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Maximum\s+Mana$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: if m.get(2).is_some() {
                        "increased_mana_more".to_string()
                    } else {
                        "increased_mana".to_string()
                    },
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)%\s+Increased\s+Mana$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "increased_mana".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+to\s+Strength$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "to_strength".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+Strength$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "to_strength".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+to\s+Dexterity$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "to_dexterity".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+Dexterity$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "to_dexterity".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+to\s+Intelligence$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "to_intelligence".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+to\s+Energy$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "to_energy".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+to\s+Vitality$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "to_vitality".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+to\s+Armor$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "to_armor".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+to\s+All\s+Attributes$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "all_attributes".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)%\s+Increased\s+All\s+Attributes$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "increased_all_attributes".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?(Strength|Dexterity|Intelligence|Energy|Vitality|Armor)$",
            )
            .unwrap(),
            build: |m| {
                let attr = m[3].to_lowercase();
                let key = if m.get(2).is_some() {
                    format!("increased_{}_more", attr)
                } else {
                    format!("increased_{}", attr)
                };
                Some(ParsedMod {
                    key,
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)\s+to\s+Defense$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "defense".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Movement\s+Speed$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: if m.get(2).is_some() {
                        "movement_speed_more".to_string()
                    } else {
                        "movement_speed".to_string()
                    },
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Attack\s+Speed$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: if m.get(2).is_some() {
                        "increased_attack_speed_more".to_string()
                    } else {
                        "increased_attack_speed".to_string()
                    },
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Faster\s+Cast\s+Rate$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: if m.get(2).is_some() {
                        "faster_cast_rate_more".to_string()
                    } else {
                        "faster_cast_rate".to_string()
                    },
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)%\s+(?:to\s+)?Spell\s+Haste$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "skill_haste".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Critical\s+Strike\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: if m.get(2).is_some() {
                        "crit_damage_more".to_string()
                    } else {
                        "crit_damage".to_string()
                    },
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)%\s+Critical\s+Damage$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "crit_damage".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(r"(?i)^([+\-\d.]+)%\s+to\s+Critical\s+Strike\s+Chance$").unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "crit_chance".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+Critically\s+Hit\s+with\s+Spells$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "spell_crit_chance".to_string(),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Spell\s+Critical\s+Damage$",
            "spell_crit_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+(?:to\s+)?All\s+Resistances$",
            "all_resistances"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+(Total\s+)?All\s+Resistances$",
            "all_resistances",
            "all_resistances_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+(Total\s+)?All\s+Resistances$",
            "all_resistances",
            "all_resistances_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Maximum\s+All\s+Resistances$",
            "max_all_resistances"
        ),
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+to\s+(arcane|cold|fire|lightning|poison)\s+Resistance$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: format!("{}_resistance", m[2].to_ascii_lowercase()),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+to\s+Maximum\s+(arcane|cold|fire|lightning|poison)\s+Resistance$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: format!("max_{}_resistance", m[2].to_ascii_lowercase()),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+to\s+(arcane|cold|fire|lightning|poison)\s+Absorb$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: format!("{}_absorption", m[2].to_ascii_lowercase()),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?(arcane|cold|fire|lightning|poison)\s+Skill\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                let element = m[3].to_ascii_lowercase();
                Some(ParsedMod {
                    key: if m.get(2).is_some() {
                        format!("{}_skill_damage_more", element)
                    } else {
                        format!("{}_skill_damage", element)
                    },
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)\s+to\s+(arcane|cold|fire|lightning|poison)\s+Skill\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: format!("flat_{}_skill_damage", m[2].to_ascii_lowercase()),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Magic\s+Skill\s+Damage$",
            "magic_skill_damage",
            "magic_skill_damage_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Magic\s+Skill\s+Damage$",
            "flat_skill_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Spell\s+Damage$",
            "spell_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Area\s+of\s+Effect(?:\s+(?:skill\s+)?radius(?:\s+of\s+all\s+skills)?)?$",
            "area_of_effect",
            "area_of_effect_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Spell\s+Duration$",
            "spell_duration_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Hit\s+Recovery$",
            "faster_hit_recovery"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Bleed(?:ing)?\s+Damage$",
            "increased_bleeding_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Bleed(?:ing)?\s+Frequency$",
            "increased_bleeding_frequency"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Poison(?:ed)?\s+Damage$",
            "increased_poisoned_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Burning\s+Damage$",
            "increased_burning_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Stasis\s+Damage$",
            "increased_stasis_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Ailment(?:\s+Tick)?\s+Frequency$",
            "increased_ailment_frequency"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Bleeding\s+on\s+hit$",
            "chance_inflict_bleeding"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Poison(?:ed)?\s+on\s+hit$",
            "chance_inflict_poisoned"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Burning\s+on\s+hit$",
            "chance_inflict_burning"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Stasis\s+on\s+hit$",
            "chance_inflict_stasis"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Bleed(?:ing)?\s+Stacks$",
            "max_bleed_stacks"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Poison(?:ed)?\s+Stacks$",
            "max_poisoned_stacks"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Burning\s+Stacks$",
            "max_burning_stacks"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Stasis\s+Stacks$",
            "max_stasis_stacks"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Rage\s+Stacks$",
            "max_rage_stacks"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Colossus\s+Stacks$",
            "max_colossus_stacks"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Life\s+Steal$",
            "life_steal",
            "life_steal_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Mana\s+Steal$",
            "mana_steal"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Rate\s+of\s+Life\s+Steal$",
            "life_steal_rate"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increases\s+Rate\s+of\s+Mana\s+Steal$",
            "mana_steal_rate"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Damage\s+Mitigation$",
            "damage_mitigation"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Damage\s+Taken\s+Mitigated$",
            "damage_mitigation"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Incoming\s+Damage\s+is\s+mitigated$",
            "damage_mitigation"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Physical\s+Damage\s+Reduction$",
            "physical_damage_reduction"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%?\s+of\s+Damage\s+Taken\s+returned\s+to\s+the\s+Attacker$",
            "damage_return"
        ),
        mod_rule!(
            r"(?i)^All\s+Damage\s+Taken\s+Reduced\s+by\s+([+\-\d.]+)%$",
            "all_damage_taken_reduced_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Damage\s+Returned$",
            "damage_return"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Life\s+Replenished\s+per\s+second$",
            "life_replenish"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Maximum\s+Life\s+Replenished\s+(?:Per|per)\s+[Ss]econd$",
            "life_replenish_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+replenished\s+per\s+second$",
            "life_replenish_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+(Total\s+)?(?:to\s+)?Mana\s+Replenish$",
            "mana_replenish",
            "mana_replenish_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+All\s+Skills$",
            "all_skills"
        ),
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)\s+to\s+(arcane|cold|fire|lightning|poison)\s+Skills$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: format!("{}_skills", m[2].to_ascii_lowercase()),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Physical\s+Skills$",
            "physical_skills"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Explosion\s+Skills$",
            "explosion_skills"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Summon\s+Skills$",
            "summon_skills"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Light\s+Radius$",
            "light_radius"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Magic\s+Find$",
            "magic_find"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+(?:Increased\s+)?Experience\s+Gain$",
            "experience_gain"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Decreased\s+Crowd\s+Control\s+Diminish$",
            "cc_diminish_decrease"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Attack\s+Rating$",
            "attack_rating"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Attack\s+Rating$",
            "attack_rating_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Physical\s+Damage$",
            "additive_physical_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+a\s+deadly\s+blow$",
            "deadly_blow"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+cast\s+an\s+additional\s+time\s+when\s+casting$",
            "multicast_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+while\s+Unarmed$",
            "damage_unarmed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Explosion\s+(?:area\s+of\s+effect\s+)?[Dd]amage$",
            "explosion_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Enhanced\s+Damage$",
            "enhanced_damage",
            "enhanced_damage_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+(?:Melee\s+|Ranged\s+)?Enhanced\s+Damage$",
            "enhanced_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Faster\s+Cast\s+Rate(?:\s+while\s+wielding\s+a\s+wand)?$",
            "faster_cast_rate"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Faster\s+Cast\s+Rate(?:\s+while\s+wielding\s+a\s+wand)?$",
            "faster_cast_rate",
            "faster_cast_rate_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Spell\s+Mana\s+Leech$",
            "mana_steal"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Elemental\s+Break\s+Inflicted\s+on\s+hit$",
            "elemental_break_on_strike"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Elemental\s+Break\s+Inflicted\s+on\s+spell\s+hit$",
            "elemental_break_on_spell"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+(?:to\s+)?Elemental\s+Break$",
            "elemental_break"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Armor\s+Break(?:\s+Inflicted\s+on\s+hit)?$",
            "armor_break_on_strike"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+(?:Maximum|Minimum)\s+Damage(?:\s+when\s+wielding\s+a\s+shield)?$",
            "attack_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%?\s+Increased\s+(Total\s+)?Physical\s+Damage$",
            "enhanced_damage",
            "enhanced_damage_more"
        ),
        // Both branches intentionally map to `enhanced_defense` (TS parity).
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Defense$",
            "enhanced_defense",
            "enhanced_defense"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Ranged\s+Projectile\s+Damage$",
            "ranged_projectile_damage",
            "ranged_projectile_damage_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Projectile\s+Damage$",
            "ranged_projectile_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Area\s+of\s+Effect\s+Spell\s+Damage$",
            "spell_aoe_damage",
            "spell_aoe_damage_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+(?:Increased\s+)?Spell\s+Area\s+of\s+Effect\s+Radius$",
            "spell_aoe_radius"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(?:Total\s+)?Area\s+of\s+Effect\s+Spell\s+Damage$",
            "spell_aoe_damage"
        ),
        // "_more" branch intentionally uses `two_handed_spell_projectile_damage` (TS parity).
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Spell\s+Projectile\s+Damage$",
            "spell_projectile_damage",
            "two_handed_spell_projectile_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+(?:Increased\s+)?Spell\s+Projectile\s+Size$",
            "spell_projectile_size"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Additional\s+Spell\s+Projectile$",
            "additional_spell_projectile"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Extra\s+Spell\s+Projectiles$",
            "extra_spell_projectiles_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Damaging\s+Aura\s+Effectiveness$",
            "damaging_aura_effectiveness"
        ),
        ParseRule {
            test: Regex::new(
                r"(?i)^(?:([+\-\d.]+)%\s+)?Increased\s+Damaging\s+Aura\s+Radius$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: "damaging_aura_radius".to_string(),
                    value: match m.get(1) {
                        Some(g) => num(g.as_str()),
                        None => 1.0,
                    },
                    self_condition: None,
                })
            },
        },
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Damaging\s+)?Aura\s+Radius$",
            "damaging_aura_radius",
            "damaging_aura_radius"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Aura\s+Skills$",
            "aura_skills"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Shield\s+Skills$",
            "shield_skills"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(?:Total\s+)?(?:Damage\s+by\s+)?Shield\s+Skill(?:\s+|s\s+)Damage$",
            "shield_skill_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+by\s+Shield\s+Skills$",
            "shield_skill_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+radius\s+of\s+Shield\s+Skills$",
            "shield_skill_radius"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Sentry\s+Damage(?:\s+but\s+you\s+can\s+no\s+longer\s+deal\s+damage\s+your\s+self)?$",
            "sentry_damage",
            "sentry_damage_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Sentry\s+Attack\s+Speed$",
            "sentry_attack_speed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Sentry\s+Duration$",
            "sentry_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+Sentries\s+to\s+fire\s+an\s+additional\s+projectile$",
            "sentry_extra_projectile_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Damage$",
            "summon_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Attack\s+Speed$",
            "summon_attack_speed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Life$",
            "summon_life"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Summon\s+Maximum\s+Life$",
            "summon_life"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Melee\s+Damage$",
            "summon_melee_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Summon\s+Projectile\s+Damage$",
            "summon_projectile_damage",
            "summon_projectile_damage_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Projectile\s+Size$",
            "summon_projectile_size"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Attack\s+Radius$",
            "summon_attack_radius"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Summon\s+Splash\s+Damage(?:\s+around\s+the\s+target)?$",
            "summon_splash_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Summon\s+Amount$",
            "summon_max_amount"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+summon\s+projectiles?\s+to\s+chain\s+on\s+hit$",
            "summon_chain_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+summon\s+projectile\s+to\s+fork\s+on\s+hit$",
            "summon_fork_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+on\s+summon\s+hit\s+to\s+knock\s+monsters\s+up\s+dealing\s+increased\s+damage$",
            "summon_knock_up_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Knock\s+Up\s+Damage$",
            "summon_knock_up_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Void\s+Blast\s+Damage$",
            "void_blast_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Guardian\s+Damage$",
            "guardian_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Guardian\s+Attack\s+Speed$",
            "guardian_attack_speed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Guardian\s+Duration$",
            "guardian_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+when\s+summoning\s+guardian\s+to\s+spawn\s+an\s+extra\s+guardian$",
            "extra_guardian_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Summoned\s+Guardians$",
            "max_summoned_guardians"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Sand\s+Beam\s+Damage$",
            "sand_beam_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Damage$",
            "orbital_skill_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+(?:Increased\s+)?Total\s+Orbital\s+Spell\s+Damage$",
            "orbital_skill_damage_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Orbital\s+Spell\s+Damage$",
            "orbital_skill_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Speed$",
            "orbital_skill_speed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Duration$",
            "orbital_skill_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Orbital\s+Spell\s+Duration$",
            "orbital_skill_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Size$",
            "orbital_skill_size"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Orbital\s+Spell\s+Size$",
            "orbital_skill_size"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+suppress\s+incoming\s+elemental\s+damage$",
            "elemental_suppression_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+elemental\s+suppression\s+effectiveness$",
            "elemental_suppression_effectiveness"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+Evade\s+incoming\s+damage$",
            "evade_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+Evade\s+projectiles$",
            "evade_projectile_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+evade\s+elemental\s+damage$",
            "evade_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+Dodge\s+Physical\s+Damage$",
            "dodge_physical_damage_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Ailment\s+Damage$",
            "ailment_damage_all",
            "ailment_damage_all_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Skills\s+Damage\s+added\s+to\s+the\s+Ailments\s+damage$",
            "skill_damage_to_ailments"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Poison(?:ed)?\s+Frequency$",
            "increased_poisoned_frequency"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Knockback\s+Force$",
            "knockback_force"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Knockback\s+Damage$",
            "knockback_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+when\s+knocking\s+monsters\s+into\s+terrain$",
            "knockback_terrain_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+when\s+attacking\s+to\s+fire\s+a\s+homing\s+missile$",
            "homing_missile_attack_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Homing\s+Missile\s+Damage$",
            "homing_missile_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+additional\s+Homing\s+Missile$",
            "homing_missile_count"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Homing\s+Missile\s+Explosion\s+area\s+of\s+effect\s+radius$",
            "homing_missile_explosion_radius"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+homing\s+missiles\s+to\s+unleash\s+a\s+shockwave\s+of\s+explosions$",
            "homing_missile_shockwave_chance"
        ),
        null_rule!(r"(?i)^([+\-\d.]+)\s+Socketable\s+Slot$"),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Explosion\s+(?:area\s+of\s+effect\s+)?radius$",
            "explosion_aoe"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+when\s+Dual\s+Wielding$",
            "damage_dual_wield"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Damage\s+when\s+Dual\s+Wielding$",
            "damage_dual_wield_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Damage(?:\s+when\s+using\s+a\s+Shield)?$",
            "damage_with_shield"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Damage\s+to\s+Monsters\s+far\s+away\s+from\s+you$",
            "damage_far"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Damage\s+dealt\s+to\s+monsters\s+at\s+long\s+range\s+but\s+deal\s+less\s+damage\s+to\s+monsters\s+close\s+to\s+you$",
            "damage_far"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+to\s+monsters\s+close\s+to\s+you\s+but\s+deal\s+less\s+damage\s+to\s+monsters\s+far\s+away$",
            "damage_close"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Damage\s+Reduction\s+when\s+wielding\s+a\s+Staff\s+or\s+a\s+Cane$",
            "two_handed_damage_reduction"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+when\s+wielding\s+an\s+Axe$",
            "damage_with_axe"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Attack\s+Rating\s+when\s+wielding\s+an\s+Axe$",
            "attack_rating_with_axe_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Physical\s+Damage\s+when\s+dual\s+wielding\s+Axes$",
            "damage_to_terrain_flat_with_axes"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Attack\s+Radius\s+when\s+dual\s+wielding\s+Axes$",
            "attack_radius_dual_axes"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Attack\s+Radius$",
            "attack_radius"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(?:Melee\s+Attack\s+Damage|Melee\s+Damage)\s+when\s+using\s+a\s+Two\s+Handed\s+Melee\s+Weapon$",
            "damage_with_two_handed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+(?:Damage\s+when\s+using\s+a\s+Two\s+Handed\s+Weapon|Ailment\s+Damage\s+when\s+using\s+a\s+Two\s+Handed\s+Weapon)$",
            "damage_with_two_handed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Spell\s+Projectile\s+Damage\s+when\s+wielding\s+a\s+Staff\s+or\s+a\s+Cane$",
            "two_handed_spell_projectile_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Magic\s+Skill\s+Damage\s+while\s+wielding\s+a\s+Wand$",
            "magic_skill_damage_with_wand"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+block\s+Physical\s+Damage\s+when\s+wielding\s+a\s+Staff\s+or\s+a\s+Cane$",
            "block_chance_physical_two_handed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Attack\s+Range\s+when\s+using\s+a\s+Shield$",
            "melee_range"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Damage\s+Mitigation\s+when\s+using\s+a\s+Shield$",
            "damage_mitigation"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Critical\s+Strike\s+Damage\s+when\s+using\s+a\s+Shield$",
            "crit_damage_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Vitality\s+when\s+wielding\s+a\s+shield$",
            "vitality_with_shield_flat"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Vitality\s+when\s+wielding\s+a\s+shield$",
            "vitality_with_shield"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Minimum\s+Damage\s+when\s+wielding\s+a\s+shield$",
            "min_damage_with_shield"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Maximum\s+Damage\s+when\s+wielding\s+a\s+shield$",
            "max_damage_with_shield"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Damage\s+Returned\s+when\s+wielding\s+a\s+Shield$",
            "damage_return"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Projectile\s+Damage\s+when\s+using\s+a\s+Gun$",
            "damage_with_gun"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+when\s+using\s+a\s+Bow$",
            "damage_with_bow"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Projectile\s+Damage\s+when\s+using\s+a\s+Throwing\s+Weapon$",
            "damage_with_throwing"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+All\s+Resistances$",
            "all_resistances"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Sentry\s+Duration$",
            "sentry_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Attack\s+Speed\s+at\s+Full\s+Life$",
            "attack_speed_full_life"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+(?:to\s+)?Increased\s+Total\s+Attack\s+Rating$",
            "attack_rating_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Damage\s+Return$",
            "damage_return_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Damage\s+Returned\s+against\s+Bosses$",
            "damage_returned_against_bosses"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+returned\s+damage\s+against\s+a\s+boss\s+to\s+echo\s+an\s+additional\s+time$",
            "returned_damage_echo_chance_boss"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+returned\s+damage\s+to\s+echo\s+an\s+additional\s+time(?:\s+till\s+failior)?$",
            "returned_damage_echo_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+return\s+the\s+damage\s+as\s+area\s+of\s+effect\s+damage$",
            "returned_damage_aoe_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+returned\s+damage\s+to\s+critically\s+hit$",
            "returned_damage_crit_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Damage\s+Returned\s+is\s+converted\s+into\s+Burning$",
            "returned_damage_to_burning"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Returned\s+Damage\s+is\s+dealt\s+as\s+Lightning\s+Damage$",
            "returned_damage_to_lightning"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+monsters\s+to\s+rest\s+in\s+peace$",
            "rest_in_peace_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Maximum\s+Magic\s+Damage\s+Reduction$",
            "max_magic_damage_reduction"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Maximum\s+Physical\s+Damage\s+Reduction$",
            "max_physical_damage_reduction"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+per\s+stack\s+of\s+rage$",
            "ranged_damage_per_rage_stack"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Damage\s+Increased\s+per\s+Stack\s+of\s+Rage$",
            "damage_per_rage_stack"
        ),
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(?:Ranged\s+)?Physical\s+Damage\s+per\s+500\s+mana$",
            )
            .unwrap(),
            build: |m| {
                let is_ranged = m[0].to_ascii_lowercase().contains("ranged");
                Some(ParsedMod {
                    key: if is_ranged {
                        "ranged_physical_per_500_mana".to_string()
                    } else {
                        "additive_physical_per_500_mana".to_string()
                    },
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Magic\s+Skills?\s+Damage\s+per\s+750\s+points\s+in\s+Mana$",
            "magic_skill_damage_per_750_mana"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Charging\s+Damage\s+per\s+point\s+in\s+Strength$",
            "charging_damage_per_strength"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Life\s+per\s+5\s+points\s+in\s+Strength$",
            "life_replenish_flat_strength"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Damage\s+Replenished\s+as\s+Life\s+when\s+struck$",
            "damage_to_life_replenish_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+replenished\s+when\s+evading$",
            "life_replenish_when_evading"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+replenished\s+when\s+struck$",
            "damage_to_life_replenish_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+replenished\s+when\s+suppressing$",
            "life_replenish_when_suppressing"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+replenish\s+\d+\s+life\s+and\s+\d+\s+mana\s+on\s+hit\s+with\s+ranged\s+attacks$",
            "life_replenish_mana_on_ranged_hit"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Life\s+Replenish(?:\s+when\s+current\s+life\s+is\s+below\s+40%\s+of\s+Maximum\s+Life)?$",
            "life_replenish_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Life\s+Regeneration\s+from\s+Flasks$",
            "life_regen_flask"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Reduced\s+damage\s+taken\s+while\s+flask\s+regeneration$",
            "flask_damage_reduction"
        ),
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(?:Total\s+)?Life\s+Steal$",
            )
            .unwrap(),
            build: |m| {
                let is_total = m[0].to_ascii_lowercase().contains("total");
                Some(ParsedMod {
                    key: if is_total {
                        "life_steal_more".to_string()
                    } else {
                        "life_steal".to_string()
                    },
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Life\s+Stolen\s+Suppressed$",
            "life_steal_suppressed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Mana\s+regenerated\s+per\s+second$",
            "mana_regen_per_second"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Damage\s+Mitigation\s+for\s+a\s+short\s+duration\s+when\s+struck$",
            "damage_mitigation_when_struck"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+damage\s+dealt\s+by\s+Area\s+of\s+Effect\s+skills$",
            "area_of_effect"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+(?:Increased\s+Crushing\s+Blow\s+Chance|Chance\s+for\s+a\s+Crushing\s+Blow)$",
            "crushing_blow_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Crushing\s+Blow\s+Effectiveness$",
            "crushing_blow_effectiveness"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+effectiveness\s+of\s+Deadly\s+Blow$",
            "deadly_blow_effectiveness"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+deal\s+area\s+of\s+effect\s+damage\s+with\s+deadly\s+blow$",
            "deadly_blow_aoe_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Deadly\s+Blow\s+Area\s+of\s+Effect\s+Damage$",
            "deadly_blow_aoe_size"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+Projectiles\s+to\s+Fork(?:\s+on\s+Hit)?$",
            "fork_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Extra\s+Forking\s+Projectile$",
            "additional_forking_projectiles"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Extra\s+Projectile(?:\s+Fired\s+with\s+Ranged\s+Attacks)?$",
            "extra_ranged_projectiles"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Additional\s+projectile\s+fired\s+when\s+performing\s+a\s+ranged\s+attack$",
            "additional_projectile_fixed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+fire\s+an\s+additional\s+projectile\s+when\s+performing\s+a\s+ranged\s+attack$",
            "extra_ranged_projectile_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+on\s+ranged\s+attack\s+to\s+perform\s+it\s+an\s+additional\s+time$",
            "ranged_extra_attack_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+unleash\s+an\s+additional\s+attack\s+or\s+projectile\s+on\s+attack$",
            "extra_attack_or_projectile_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Colossus\s+damage$",
            "colossus_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Life\s+Replenish\s+for\s+each\s+Colossus\s+stack$",
            "life_replenish_per_colossus_stack"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Death\s+Explosion\s+damage$",
            "death_explosion_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+monsters\s+slain\s+by\s+damage\s+return\s+to\s+explode\s+dealing\s+area\s+of\s+effect\s+damage$",
            "soul_ignition_explode_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Overflow\s+Effectiveness$",
            "overflow_effectiveness"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Agitation\s+Movement\s+Speed$",
            "agitation_movement_speed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+gain\s+\d+%\s+Evasion\s+for\s+a\s+short\s+duration\s+when\s+struck$",
            "evasion_when_struck_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+gain\s+\d+%\s+Evasion\s+for\s+a\s+short\s+duration\s+after\s+being\s+hit$",
            "evasion_when_struck_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)(?:\s*s)?\s+Evasion\s+Duration$",
            "evasion_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+leave\s+a\s+cloud\s+of\s+poisonous\s+gas\s+when\s+struck$",
            "gas_cloud_when_struck_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Gas\s+Cloud\s+Poisoned\s+Damage$",
            "gas_cloud_poisoned_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+as\s+Explosion\s+Damage$",
            "summon_max_life_as_explosion"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Weapon\s+Throw\s+Damage$",
            "weapon_throw_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Temporal\s+Echo\s+Damage$",
            "temporal_echo_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Branch\s+Damage$",
            "branch_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Stampede\s+Damage\s+based\s+on\s+your\s+physical\s+damage$",
            "stampede_damage_per_physical"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+splash\s+damage\s+to\s+monsters\s+around\s+the\s+target$",
            "splash_damage_around_target"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+unleash\s+piercing\s+spikes\s+outwards\s+when\s+struck$",
            "damage_return_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Total\s+Damage\s+Dealt(?:\s+and\s+Damage\s+Taken)?$",
            "total_damage_dealt_and_taken"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+and\s+Increased\s+Damage\s+Taken$",
            "damage_dealt_and_taken_amp"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Decreased\s+Monster\s+Crowd\s+Control\s+Immunity$",
            "cc_immunity_decrease_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+dealt\s+to\s+monsters\s+with\s+Crowd\s+Control\s+Immunity$",
            "damage_to_cc_immune"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Monster\s+Damage\s+Over\s+Time\s+Immunity\s+Shattered$",
            "monster_dot_immunity_shattered"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Elemental\s+Weakness$",
            "elemental_weakness"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Target\s+Defense\s+Ignored$",
            "defense_ignored"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)(?:\s*s)?\s+Stun\s+Duration$",
            "stun_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Mana\s+Costs\s+are\s+taken\s+from\s+life\s+instead$",
            "mana_cost_paid_in_life"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Damage\s+Taken\s+is\s+drained\s+from\s+mana\s+instead$",
            "damage_drained_from_mana"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+deal\s+\d+%\s+area\s+of\s+effect\s+damage\s+on\s+bleed\s+tick$",
            "bleed_aoe_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Bleed\s+AoE\s+Damage$",
            "bleed_aoe_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s*Seconds?\s+to\s+Spell\s+Duration$",
            "spell_duration_seconds"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Spell\s+Area\s+of\s+Effect\s+Radius$",
            "spell_aoe_radius"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Spell\s+Projectile\s+Size$",
            "spell_projectile_size"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+Spell\s+Critical\s+hits\s+to\s+grant\s+Time\s+Surge\s+for\s+\d+\s+seconds$",
            "time_surge_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+on\s+cast\s+to\s+summon\s+a\s+Mirror\s+of\s+Odin\s+reaking\s+havoc\s+at\s+nearby\s+monsters$",
            "mirror_of_odin_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Lightning\s+Damage\s+scaling\s+from\s+all\s+elemental\s+sources$",
            "lightning_per_element"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+after\s+kill\s+to\s+recover\s+all\s+skill\s+cooldowns$",
            "cooldown_recovery_after_kill_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)(?:\s*s)?\s+Cooldown\s+Recovered$",
            "cooldown_recovered_flat"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Extra\s+Damage\s+based\s+on\s+summons\s+maximum\s+life$",
            "extra_damage_from_summon_life"
        ),
        null_rule!(r"(?i)^([+\-\d.]+)%\s+of\s+Resistances\s+converted\s+to\s+Life$"),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Faster\s+Cast\s+Rate\s+per\s+Stack\s+of\s+Wizardry$",
            "wizardry_cast_rate"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+gain\s+a\s+stack\s+of\s+Mage\s+Guard\s+when\s+you\s+cast\s+a\s+spell\s+up\s+to\s+\d+\s+stacks$",
            "mage_guard_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Damage\s+Reduction\s+per\s+stack\s+of\s+Mage\s+Guard$",
            "mage_guard_damage_reduction"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Desert\s+Ripple\s+Damage$",
            "sand_ripple_damage"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+on\s+hit\s+for\s+Area\s+of\s+Effect\s+skills\s+to\s+pull\s+monsters\s+in$",
            "aoe_pull_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+cause\s+monsters\s+damaged\s+by\s+an\s+explosion\s+to\s+unleash\s+an\s+additional\s+explosion$",
            "additional_explosion_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+on\s+explosion\s+or\s+area\s+of\s+effect\s+skill\s+to\s+gain\s+a\s+stack\s+of\s+Ramping\s+Pulse\s+increasing\s+the\s+radius\s+of\s+area\s+of\s+effect\s+and\s+explosion\s+skills$",
            "ramping_pulse_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Maximum\s+Stacks$",
            "ramping_pulse_max_stacks"
        ),
        fixed_rule!(
            r"(?i)^(?:[+\-\d.]+%\s+)?You\s+can\s+now\s+dual\s+wield\s+Two\s+Handed\s+(?:Melee\s+Weapons|Swords,\s+Maces\s+and\s+Axes)$",
            "dual_wield_2h_melee",
            1.0
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Manacost$",
            "increased_manacost"
        ),
        cond_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+when\s+Critical\s+Strike\s+Chance\s+is\s+below\s+40%$",
            "ranged_projectile_damage",
            SelfConditionKey::CritChanceBelow40
        ),
        cond_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Physical\s+Damage\s+while\s+below\s+40%\s+Maximum\s+Life$",
            "enhanced_damage",
            SelfConditionKey::LifeBelow40
        ),
        fixed_rule!(
            r"(?i)^You\s+can\s+no\s+longer\s+dodge\s+monster\s+attacks\s+but\s+also\s+cannot\s+be\s+stunned\s+or\s+frozen$",
            "force_field_protection",
            1.0
        ),
        mod_rule!(
            r"(?i)^Your\s+Maximum\s+All\s+Resistances\s+are\s+capped\s+to\s+([+\-\d.]+)%$",
            "max_all_resistances_cap"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+your\s+Light\s+Radius\s+added\s+as\s+Increased\s+All\s+Attributes$",
            "light_radius_to_attributes"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Melee\s+Enhanced\s+Damage$",
            "enhanced_damage_melee_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Ranged\s+Enhanced\s+Damage$",
            "enhanced_damage_ranged_more"
        ),
        fixed_rule!(
            r"(?i)^(?:[+\-\d.]+%\s+)?Your\s+skill\s+weapon\s+type\s+restrictions\s+are\s+removed$",
            "skill_restrictions_removed",
            1.0
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+gain\s+an\s+orbiting\s+Bone\s+Fragment\s+when\s+struck$",
            "bone_fragment_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Return\s+Damage\s+inherited\s+by\s+Bone\s+Fragment$",
            "bone_fragment_inherit"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Life\s+Steal\s+is\s+now\s+instant\s+but\s+you\s+cannot\s+replenish\s+life\s+from\s+any\s+other\s+sources$",
            "vampirism_instant_life_steal"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+of\s+Incoming\s+Damage\s+is\s+ignored$",
            "damage_ignored_flat"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Damage\s+Taken\s+Reduced$",
            "damage_taken_reduced"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Extra\s+Jump\s+that\s+can\s+be\s+performed\s+mid\s+air$",
            "extra_jump_count"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Jump\s+Power$",
            "jumping_power"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Damage\s+Mitigation\s+when\s+phasing\s+through\s+monsters$",
            "damage_mitigation"
        ),
        null_rule!(r"(?i)^(?:\+0\s+)?Path\s+to\s+any\s+Black\s+Hole$"),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Combat\s+Mitigation\s+Stacks$",
            "max_combat_mitigation_stacks"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Damage\s+Taken\s+Recovered\s+as\s+Mana$",
            "damage_recouped_as_mana"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+Evade\s+projectiles\s+when\s+dual\s+wielding$",
            "evade_projectile_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Attack\s+Range$",
            "melee_range"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Attack\s+Range\s+when\s+using\s+a\s+Shield$",
            "melee_range"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Your\s+Mana\s+Costs\s+are\s+taken\s+from\s+life\s+instead$",
            "mana_cost_paid_in_life"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Ailment\s+Damage\s+but\s+you\s+can\s+only\s+deal\s+damage\s+with\s+ailments$",
            "ailment_damage_all"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Skills\s+Damage\s+added\s+to\s+the\s+Ailments\s+damage$",
            "skill_damage_to_ailments"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+of\s+Reduced\s+Damage\s+Healed\s+Over\s+Time$",
            "damage_to_life_replenish_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+on\s+hit\s+with\s+Area\s+of\s+Effect\s+Spells\s+to\s+call\s+down\s+a\s+chaos\s+meteor\s+dealing\s+\d+%\s+of\s+the\s+damage\.?\s+The\s+proc\s+chance\s+is\s+lowered\s+to\s+\d+%\s+with\s+non\s+single\s+hitting\s+skills$",
            "additional_explosion_chance"
        ),
        fixed_rule!(
            r"(?i)^Summons\s+explode\s+at\s+\d+%\s+life\s+dealing\s+area\s+of\s+effect\s+fire\s+damage\s+based\s+on\s+their\s+life$",
            "summon_explode_on_low_life",
            1.0
        ),
        fixed_rule!(
            r"(?i)^Summons\s+now\s+explode\s+instantly\s+after\s+coming\s+in\s+contact\s+with\s+a\s+monster$",
            "summon_instant_explode",
            1.0
        ),
        fixed_rule!(
            r"(?i)^Summon\s+projectile\s+chain\s+hits\s+unleash\s+a\s+Void\s+Blast\s+dealing\s+damage\s+around\s+the\s+target$",
            "summon_chain_void_blast",
            1.0
        ),
        fixed_rule!(
            r"(?i)^(?:[+\-\d.]+s\s+)?Life\s+replenish\s+now\s+happens\s+every\s+\d+\s+seconds\s+with\s+increased\s+power$",
            "life_replenish_more",
            0.0
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Life\s+Replenish$",
            "life_replenish_more"
        ),
        fixed_rule!(
            r"(?i)^Chance\s+on\s+hit\s+to\s+unleash\s+a\s+Sand\s+Ripple\s+dealing\s+damage\s+on\s+a\s+radius\s+around\s+the\s+target$",
            "sand_ripple_chance",
            0.0
        ),
        fixed_rule!(
            r"(?i)^After\s+\d+\s+attacks\s+guardians\s+unleash\s+a\s+Sand\s+Beam$",
            "sand_beam_damage",
            0.0
        ),
        fixed_rule!(
            r"(?i)^Gain\s+\d+%\s+increased\s+total\s+cast\r?\s+rate\s+but\s+over\s+heat\s+after\s+\d+\s+casts\s+causing\s+decreased\s+cast\s+rate\s+but\s+increased\s+total\s+damage$",
            "wizardry_cast_rate",
            0.0
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Damage\s+and\s+Decreased\s+Cast\s+Rate\s+from\s+over\s+heat$",
            "enhanced_damage_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Attack\s+Speed\s+&\s+Damage\s+and\s+(?:In|De)creased\s+Damage\s+Reduction\s+&\s+All\s+Resistances$",
            "increased_attack_speed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+When\s+at\s+full\s+Life\s+gain\s+increased\s+total\s+Attack\s+Speed\s+and\s+Damage\s+Dealt\s+but\s+also\s+decreased\s+Damage\s+Reduction\s+and\s+All\s+Resistances$",
            "attack_speed_full_life"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Total\s+Summon\s+Projectile\s+Damage$",
            "summon_projectile_damage_more"
        ),
    ]
}
