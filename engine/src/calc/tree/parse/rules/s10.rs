use super::super::*;

pub(super) fn rules() -> Vec<ParseRule> {
    vec![
        // ---- S10 incarnation tree phrasing variants ----
        // Mechanics the engine already tracks, worded differently by the
        // S10 incarnation tree data.
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Attack\s+Speed\s+when\s+at\s+Full\s+Life$",
            "attack_speed_full_life"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+Returned$",
            "damage_return_more"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Damage\s+Return\s+against\s+Bosses$",
            "damage_returned_against_bosses"
        ),
        // Value carries its own minus sign in the data (e.g. "-6% Reduced ...").
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Reduced\s+Movement\s+Speed$",
            "movement_speed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Magic\s+Damage\s+taken\s+Reduced$",
            "magic_damage_reduction"
        ),
        // Bare "Damage Reduction" is the physical one; S10 uses it for
        // trade-off nodes with a negative value.
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Damage\s+Reduction$",
            "physical_damage_reduction"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Buffing\s+Aura\s+Effectiveness$",
            "buffing_aura_effectiveness"
        ),
        // Per-element break family; lightning_break is consumed by the damage
        // formula, the other elements aggregate under the same naming scheme.
        ParseRule {
            test: Regex::new(
                r"(?i)^([+\-\d.]+)%\s+Increased\s+(Arcane|Cold|Fire|Lightning|Poison)\s+Break$",
            )
            .unwrap(),
            build: |m| {
                Some(ParsedMod {
                    key: format!("{}_break", m[2].to_ascii_lowercase()),
                    value: num(&m[1]),
                    self_condition: None,
                })
            },
        },
        mod_rule!(r"(?i)^([+\-\d.]+)%\s+Branch\s+Damage$", "branch_damage"),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Light\s+Radius$",
            "light_radius_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Cap\s+on\s+All\s+Resistances$",
            "max_all_resistances"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)(?:\s*s)?\s+to\s+Spell\s+Duration$",
            "spell_duration_seconds"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Total\s+Summon\s+Projectile\s+Size$",
            "summon_projectile_size"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+on\s+hit\s+to\s+unleash\s+a\s+Sand\s+Ripple\s+dealing\s+damage\s+on\s+a\s+radius\s+around\s+the\s+target$",
            "sand_ripple_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Charge\s+Skill\s+damage\s+per\s+point\s+in\s+Vitality$",
            "charging_damage_per_vitality"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Magic\s+Skill\s+Damage\s+per\s+points?\s+in\s+Light\s+Radius$",
            "magic_skill_damage_per_light_radius"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Magic\s+Skill\s+Damage\s+per\s+points?\s+in\s+Light\s+Radius$",
            "flat_magic_skill_damage_per_light_radius"
        ),
        // S10 cosmetic stats: aggregated and displayed, no damage-formula
        // consumer yet.
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Melee\s+Projectile\s+Size$",
            "melee_projectile_size"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%?\s+Increased\s+Melee\s+Projectile\s+Speed$",
            "melee_projectile_speed"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%?\s+Increased\s+Ranged\s+Projectile\s+Speed$",
            "ranged_projectile_speed"
        ),
        // Ailments tick once per second, so duration == tick count. "+#s" lines add
        // to the flat key, "+#%" lines aggregate separately in the _pct key.
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s*s\s+Increased\s+Burning\s+Duration$",
            "burning_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Burning\s+Duration$",
            "burning_duration_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s*s\s+Increased\s+Frostbite\s+Duration$",
            "frostbite_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Frostbite\s+Duration$",
            "frostbite_duration_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s*s\s+Increased\s+Poisoned\s+Duration$",
            "poisoned_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Poisoned\s+Duration$",
            "poisoned_duration_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s*s\s+Increased\s+Shadowburn\s+Duration$",
            "shadowburn_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Shadowburn\s+Duration$",
            "shadowburn_duration_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s*s\s+Increased\s+Stasis\s+Duration$",
            "stasis_duration"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Increased\s+Stasis\s+Duration$",
            "stasis_duration_pct"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+to\s+unleash\s+multiple\s+projectiles\s+on\s+attack$",
            "multishot_chance"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Multishot\s+Projectiles\s+unleashed$",
            "max_multishot_projectiles"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Maximum\s+Skill\s+Stacks$",
            "max_skill_stacks"
        ),
        // Flat variant; the "% Magic Damage taken Reduced" rule above wins
        // first for percent lines.
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+Magic\s+Damage\s+taken\s+Reduced$",
            "magic_damage_taken_reduced"
        ),
        // "Excecution Treshold" is the in-game typo; accept the fixed
        // spelling too in case a patch corrects it.
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+(?:Excecution|Execution)\s+(?:Treshold|Threshold)$",
            "execution_threshold"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Stun\s+&\s+Freeze\s+Immunity$",
            "stun_freeze_immunity"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+Chance\s+for\s+returned\s+damage\s+to\s+echo\s+an\s+additional\s+time\s+till\s+failure$",
            "damage_return_echo_chance"
        ),
        // Dagger conditionals: folded into physical/enhanced damage by the
        // weapon-type pass in stats.rs when a Dagger is equipped.
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Physical\s+Damage\s+while\s+wielding\s+a\s+Dagger$",
            "physical_damage_with_dagger"
        ),
        mod_rule!(
            r"(?i)^([+\-\d.]+)%\s+to\s+Enhanced\s+Damage\s+while\s+wielding\s+a\s+Dagger$",
            "enhanced_damage_with_dagger"
        ),
        // Skill-level bonus for skills tagged "Projectile"; consumed by
        // apply_skill_ranks.
        mod_rule!(
            r"(?i)^([+\-\d.]+)\s+to\s+Projectile\s+Skills$",
            "projectile_skills"
        ),
    ]
}
