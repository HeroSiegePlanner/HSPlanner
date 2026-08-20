use super::*;

pub(crate) static CONVERSION_RULES: Lazy<Vec<ConversionRule>> = Lazy::new(|| {
    vec![
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Increased\s+Attack\s+Speed\s+is\s+added\s+as\s+Magic\s+Skill\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "increased_attack_speed".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "magic_skill_damage".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?(arcane|cold|fire|lightning|poison)\s+Resistance\s+is\s+converted\s+to\s+Increased\s+(?:arcane|cold|fire|lightning|poison)\s+Skill\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                let element = m[2].to_ascii_lowercase();
                Some(ParsedConversion {
                    from_key: format!("{}_resistance", element),
                    from_kind: ConvertKind::Stat,
                    to_key: format!("{}_skill_damage", element),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+(strength|dexterity|intelligence|energy|vitality|armor)\s+(?:converted\s+to|(?:is\s+)?added\s+as)\s+(.+)$",
            )
            .unwrap(),
            build: |m| {
                let target = m[3].trim().to_ascii_lowercase();
                let target_key = CONVERSION_TARGET_STATS.get(target.as_str()).copied()?;
                Some(ParsedConversion {
                    from_key: m[2].to_ascii_lowercase(),
                    from_kind: ConvertKind::Attribute,
                    to_key: target_key.to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Defense\s+is\s+converted\s+to\s+life$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "defense".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "life".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Resistances\s+converted\s+to\s+Life$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "all_resistances".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "life".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+All\s+Resistances\s+over\s+the\s+cap\s+converted\s+to\s+life$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "all_resistances".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "life".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Attack\s+Damage\s+converted\s+to\s+Increased\s+Life$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "attack_damage".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "increased_life".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Negative\s+All\s+Resistances\s+are\s+added\s+as\s+Increased\s+Maximum\s+Life$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "all_resistances".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "increased_life".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Negative\s+All\s+Resistances\s+added\s+as\s+increased\s+damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "all_resistances".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "enhanced_damage".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+added\s+as\s+Maximum\s+Mana$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "life".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "mana".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Maximum\s+Mana\s+added\s+as\s+Maximum\s+life$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "mana".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "life".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Increased\s+Maximum\s+Mana\s+added\s+as\s+Magic\s+Skill\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "increased_mana".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "magic_skill_damage".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+(?:your\s+)?Increased\s+Movement\s+Speed\s+converted\s+to\s+Attack\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "movement_speed".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "attack_damage".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Energy\s+is\s+added\s+as\s+Ranged\s+Physical\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "energy".to_string(),
                    from_kind: ConvertKind::Attribute,
                    to_key: "ranged_physical_per_500_mana".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Area\s+of\s+Effect\s+Radius\s+converted\s+to\s+Area\s+of\s+Effect\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "area_of_effect".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "spell_aoe_damage".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Explosion\s+Area\s+of\s+Effect\s+radius\s+converted\s+to\s+Explosion\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "explosion_aoe".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "explosion_damage".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Dexterity\s+Converted\s+to\s+Ranged\s+Projectile\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "dexterity".to_string(),
                    from_kind: ConvertKind::Attribute,
                    to_key: "ranged_projectile_damage".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Strength\s+(?:converted\s+to|Added\s+as)\s+weapon\s+damage\s+when\s+Unarmed,\s+Strength\s+no\s+longer\s+provides\s+attack\s+damage\.?$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "strength".to_string(),
                    from_kind: ConvertKind::Attribute,
                    to_key: "str_to_unarmed_damage".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+physical\s+damage\s+as\s+Arrow\s+Damage$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedConversion {
                    from_key: "additive_physical_damage".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: "physical_to_arrow_damage".to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+to\s+Enhanced\s+Damage\s+when\s+using\s+(Bow|Throwing\s+Weapon|Gun|Axe)$",
            )
            .unwrap(),
            build: |m| {
                let weapon = m[2].to_ascii_lowercase();
                let target_key = if weapon == "bow" {
                    "damage_with_bow"
                } else if weapon == "gun" {
                    "damage_with_gun"
                } else if weapon == "axe" {
                    "damage_with_axe"
                } else {
                    "damage_with_throwing"
                };
                Some(ParsedConversion {
                    from_key: "enhanced_damage".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: target_key.to_string(),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
        ConversionRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+of\s+Physical\s+Damage\s+converted\s+to\s+(arcane|cold|fire|lightning|poison)$",
            )
            .unwrap(),
            build: |m| {
                let element = m[2].to_ascii_lowercase();
                Some(ParsedConversion {
                    from_key: "additive_physical_damage".to_string(),
                    from_kind: ConvertKind::Stat,
                    to_key: format!("physical_to_{}", element),
                    to_kind: ConvertKind::Stat,
                    pct: num(&m[1]),
                })
            },
        },
    ]
});

pub(crate) static DISABLE_RULES: Lazy<Vec<DisableRule>> = Lazy::new(|| {
    vec![DisableRule {
        test: Regex::new(
            r"(?i)^You\s+cannot\s+regenerate\s+life\s+from\s+life\s+replenish\s+anymore$",
        )
        .unwrap(),
        target: DisableTarget::LifeReplenish,
    }]
});

