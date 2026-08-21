use super::*;

pub(crate) fn default_projectile_count() -> f64 {
    1.0
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponDamageInput {
    #[serde(default)]
    pub weapon: Option<WeaponDto>,
    #[serde(default)]
    pub stats: HashMap<String, NumberOrRange>,
    #[serde(default)]
    pub enemy_conditions: HashMap<String, bool>,
    #[serde(default)]
    pub enemy_resistances: HashMap<String, f64>,
    #[serde(default)]
    pub projectile_count: Option<f64>,
}

#[derive(Serialize)]
pub struct ExtraSourceOut {
    pub label: String,
    pub pct: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDamageOutput {
    pub effective_rank_min: f64,
    pub effective_rank_max: f64,
    pub base_min: f64,
    pub base_max: f64,
    pub flat_min: f64,
    pub flat_max: f64,
    pub synergy_min_pct: f64,
    pub synergy_max_pct: f64,
    pub skill_damage_min_pct: f64,
    pub skill_damage_max_pct: f64,
    pub extra_damage_pct: f64,
    pub extra_damage_sources: Vec<ExtraSourceOut>,
    pub crit_chance: f64,
    pub crit_damage_pct: f64,
    pub crit_multiplier_avg: f64,
    pub multicast_chance_pct: f64,
    pub multicast_multiplier: f64,
    pub projectile_count: u32,
    pub elemental_break_pct: f64,
    pub elemental_break_multiplier: f64,
    pub enemy_resistance_pct: f64,
    pub resistance_ignored_pct: f64,
    pub effective_resistance_pct: f64,
    pub resistance_multiplier: f64,
    pub hit_min: i64,
    pub hit_max: i64,
    pub crit_min: i64,
    pub crit_max: i64,
    pub final_min: i64,
    pub final_max: i64,
    pub avg_min: i64,
    pub avg_max: i64,
}
impl From<calc::SkillDamageBreakdown> for SkillDamageOutput {
    fn from(v: calc::SkillDamageBreakdown) -> Self {
        Self {
            effective_rank_min: v.effective_rank_min,
            effective_rank_max: v.effective_rank_max,
            base_min: v.base_min,
            base_max: v.base_max,
            flat_min: v.flat_min,
            flat_max: v.flat_max,
            synergy_min_pct: v.synergy_min_pct,
            synergy_max_pct: v.synergy_max_pct,
            skill_damage_min_pct: v.skill_damage_min_pct,
            skill_damage_max_pct: v.skill_damage_max_pct,
            extra_damage_pct: v.extra_damage_pct,
            extra_damage_sources: v
                .extra_damage_sources
                .into_iter()
                .map(|e| ExtraSourceOut {
                    label: e.label.to_string(),
                    pct: e.pct,
                })
                .collect(),
            crit_chance: v.crit_chance,
            crit_damage_pct: v.crit_damage_pct,
            crit_multiplier_avg: v.crit_multiplier_avg,
            multicast_chance_pct: v.multicast_chance_pct,
            multicast_multiplier: v.multicast_multiplier,
            projectile_count: v.projectile_count,
            elemental_break_pct: v.elemental_break_pct,
            elemental_break_multiplier: v.elemental_break_multiplier,
            enemy_resistance_pct: v.enemy_resistance_pct,
            resistance_ignored_pct: v.resistance_ignored_pct,
            effective_resistance_pct: v.effective_resistance_pct,
            resistance_multiplier: v.resistance_multiplier,
            hit_min: v.hit_min,
            hit_max: v.hit_max,
            crit_min: v.crit_min,
            crit_max: v.crit_max,
            final_min: v.final_min,
            final_max: v.final_max,
            avg_min: v.avg_min,
            avg_max: v.avg_max,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponDamageOutput {
    pub has_weapon: bool,
    pub weapon_name: Option<String>,
    pub weapon_damage_min: f64,
    pub weapon_damage_max: f64,
    pub enhanced_damage_min_pct: f64,
    pub enhanced_damage_max_pct: f64,
    pub additive_physical_min: f64,
    pub additive_physical_max: f64,
    pub additive_elemental_min: f64,
    pub additive_elemental_max: f64,
    pub additive_elemental_breakdown: Vec<ExtraSourceOut>,
    pub attack_damage_min_pct: f64,
    pub attack_damage_max_pct: f64,
    pub extra_damage_pct: f64,
    pub extra_damage_sources: Vec<ExtraSourceOut>,
    pub crushing_blow_modifier: f64,
    pub armor_break_pct: f64,
    pub deadly_blow_chance: f64,
    pub hit_chance: f64,
    pub crit_chance: f64,
    pub crit_damage_pct: f64,
    pub crit_multiplier_avg: f64,
    pub attacks_per_second_min: f64,
    pub attacks_per_second_max: f64,
    pub projectile_count: u32,
    pub enemy_phys_res_pct: f64,
    pub phys_resistance_ignored_pct: f64,
    pub hit_min: i64,
    pub hit_max: i64,
    pub crit_min: i64,
    pub crit_max: i64,
    pub avg_min: i64,
    pub avg_max: i64,
    pub open_wounds_min: i64,
    pub open_wounds_max: i64,
    pub dps_min: i64,
    pub dps_max: i64,
}
impl From<calc::WeaponDamageBreakdown> for WeaponDamageOutput {
    fn from(v: calc::WeaponDamageBreakdown) -> Self {
        let map_sources = |xs: Vec<calc::ExtraSource>| {
            xs.into_iter()
                .map(|e| ExtraSourceOut {
                    label: e.label.to_string(),
                    pct: e.pct,
                })
                .collect()
        };
        Self {
            has_weapon: v.has_weapon,
            weapon_name: v.weapon_name,
            weapon_damage_min: v.weapon_damage_min,
            weapon_damage_max: v.weapon_damage_max,
            enhanced_damage_min_pct: v.enhanced_damage_min_pct,
            enhanced_damage_max_pct: v.enhanced_damage_max_pct,
            additive_physical_min: v.additive_physical_min,
            additive_physical_max: v.additive_physical_max,
            additive_elemental_min: v.additive_elemental_min,
            additive_elemental_max: v.additive_elemental_max,
            additive_elemental_breakdown: map_sources(v.additive_elemental_breakdown),
            attack_damage_min_pct: v.attack_damage_min_pct,
            attack_damage_max_pct: v.attack_damage_max_pct,
            extra_damage_pct: v.extra_damage_pct,
            extra_damage_sources: map_sources(v.extra_damage_sources),
            crushing_blow_modifier: v.crushing_blow_modifier,
            armor_break_pct: v.armor_break_pct,
            deadly_blow_chance: v.deadly_blow_chance,
            hit_chance: v.hit_chance,
            crit_chance: v.crit_chance,
            crit_damage_pct: v.crit_damage_pct,
            crit_multiplier_avg: v.crit_multiplier_avg,
            attacks_per_second_min: v.attacks_per_second_min,
            attacks_per_second_max: v.attacks_per_second_max,
            projectile_count: v.projectile_count,
            enemy_phys_res_pct: v.enemy_phys_res_pct,
            phys_resistance_ignored_pct: v.phys_resistance_ignored_pct,
            hit_min: v.hit_min,
            hit_max: v.hit_max,
            crit_min: v.crit_min,
            crit_max: v.crit_max,
            avg_min: v.avg_min,
            avg_max: v.avg_max,
            open_wounds_min: v.open_wounds_min,
            open_wounds_max: v.open_wounds_max,
            dps_min: v.dps_min,
            dps_max: v.dps_max,
        }
    }
}

#[tauri::command]
pub fn compute_skill_damage(input: SkillDamageInput) -> Option<SkillDamageOutput> {
    let skill: calc::Skill = input.skill.into();
    let attributes = ranged_map(normalized_keys(input.attributes));
    let stats = ranged_map(normalized_keys(input.stats));
    let skill_ranks = normalized_keys(input.skill_ranks_by_name);
    let item_bonuses = normalized_keys(input.item_skill_bonuses);
    let skills_by_name: HashMap<String, calc::Skill> = input
        .skills_by_name
        .into_iter()
        .map(|(k, v)| (norm(&k), v.into()))
        .collect();

    // Standalone preview: the caller passes the already-resolved
    // `of_total_damage`, so there is no scoped subtree map to hand over.
    let no_scoped = calc::StatMap::new();
    let inp = calc::SkillInput {
        skill: &skill,
        allocated_rank: input.allocated_rank,
        attributes: &attributes,
        stats: &stats,
        skill_ranks_by_name: &skill_ranks,
        item_skill_bonuses: &item_bonuses,
        enemy_conditions: &input.enemy_conditions,
        enemy_resistances: &input.enemy_resistances,
        skills_by_name: &skills_by_name,
        projectile_count: input.projectile_count.max(0.0) as u32,
        of_total_damage: input.of_total_damage,
        scoped: &no_scoped,
        conversion_flat: 0.0,
        conversion_skill_damage_pct: 0.0,
    };
    calc::compute_skill_damage(&inp).map(Into::into)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackSkillDamageInput {
    #[serde(flatten)]
    pub base: SkillDamageInput,
    #[serde(default)]
    pub weapon: Option<WeaponDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackSkillDamageOutput {
    pub effective_rank_min: f64,
    pub effective_rank_max: f64,
    pub weapon_damage_pct_min: f64,
    pub weapon_damage_pct_max: f64,
    pub skill_flat_phys_min: f64,
    pub skill_flat_phys_max: f64,
    pub attack_rating_pct_min: f64,
    pub attack_rating_pct_max: f64,
    pub synergy_min_pct: f64,
    pub synergy_max_pct: f64,
    pub projectile_count: u32,
    pub weapon_damage_min: f64,
    pub weapon_damage_max: f64,
    pub enhanced_damage_min_pct: f64,
    pub enhanced_damage_max_pct: f64,
    pub additive_physical_min: f64,
    pub additive_physical_max: f64,
    pub attack_damage_min_pct: f64,
    pub attack_damage_max_pct: f64,
    pub crushing_blow_modifier: f64,
    pub armor_break_pct: f64,
    pub deadly_blow_chance: f64,
    pub crit_chance: f64,
    pub crit_damage_pct: f64,
    pub crit_multiplier_avg: f64,
    pub extra_damage_sources: Vec<ExtraSourceOut>,
    pub physical_hit_min: i64,
    pub physical_hit_max: i64,
    pub physical_avg_min: i64,
    pub physical_avg_max: i64,
    pub poison_hit_min: i64,
    pub poison_hit_max: i64,
    pub poison_avg_min: i64,
    pub poison_avg_max: i64,
    pub combined_hit_min: i64,
    pub combined_hit_max: i64,
    pub combined_avg_min: i64,
    pub combined_avg_max: i64,
    pub attacks_per_second_min: f64,
    pub attacks_per_second_max: f64,
    pub dps_min: f64,
    pub dps_max: f64,
}
impl From<calc::AttackSkillDamageBreakdown> for AttackSkillDamageOutput {
    fn from(v: calc::AttackSkillDamageBreakdown) -> Self {
        Self {
            effective_rank_min: v.effective_rank_min,
            effective_rank_max: v.effective_rank_max,
            weapon_damage_pct_min: v.weapon_damage_pct_min,
            weapon_damage_pct_max: v.weapon_damage_pct_max,
            skill_flat_phys_min: v.skill_flat_phys_min,
            skill_flat_phys_max: v.skill_flat_phys_max,
            attack_rating_pct_min: v.attack_rating_pct_min,
            attack_rating_pct_max: v.attack_rating_pct_max,
            synergy_min_pct: v.synergy_min_pct,
            synergy_max_pct: v.synergy_max_pct,
            projectile_count: v.projectile_count,
            weapon_damage_min: v.weapon_damage_min,
            weapon_damage_max: v.weapon_damage_max,
            enhanced_damage_min_pct: v.enhanced_damage_min_pct,
            enhanced_damage_max_pct: v.enhanced_damage_max_pct,
            additive_physical_min: v.additive_physical_min,
            additive_physical_max: v.additive_physical_max,
            attack_damage_min_pct: v.attack_damage_min_pct,
            attack_damage_max_pct: v.attack_damage_max_pct,
            crushing_blow_modifier: v.crushing_blow_modifier,
            armor_break_pct: v.armor_break_pct,
            deadly_blow_chance: v.deadly_blow_chance,
            crit_chance: v.crit_chance,
            crit_damage_pct: v.crit_damage_pct,
            crit_multiplier_avg: v.crit_multiplier_avg,
            extra_damage_sources: v
                .extra_damage_sources
                .into_iter()
                .map(|e| ExtraSourceOut {
                    label: e.label.to_string(),
                    pct: e.pct,
                })
                .collect(),
            physical_hit_min: v.physical_hit_min,
            physical_hit_max: v.physical_hit_max,
            physical_avg_min: v.physical_avg_min,
            physical_avg_max: v.physical_avg_max,
            poison_hit_min: v.poison_hit_min,
            poison_hit_max: v.poison_hit_max,
            poison_avg_min: v.poison_avg_min,
            poison_avg_max: v.poison_avg_max,
            combined_hit_min: v.combined_hit_min,
            combined_hit_max: v.combined_hit_max,
            combined_avg_min: v.combined_avg_min,
            combined_avg_max: v.combined_avg_max,
            attacks_per_second_min: v.attacks_per_second_min,
            attacks_per_second_max: v.attacks_per_second_max,
            dps_min: v.dps_min,
            dps_max: v.dps_max,
        }
    }
}

// Standalone per-card preview of an attack skill: weapon physical plus the
// skill's elemental part, mirroring the attack branch in build.rs.
#[tauri::command]
pub fn compute_attack_skill_damage(
    input: AttackSkillDamageInput,
) -> Option<AttackSkillDamageOutput> {
    let weapon: Option<calc::Weapon> = input.weapon.map(Into::into);
    let b = input.base;
    let skill: calc::Skill = b.skill.into();
    let attributes = ranged_map(normalized_keys(b.attributes));
    let stats = ranged_map(normalized_keys(b.stats));
    let skill_ranks = normalized_keys(b.skill_ranks_by_name);
    let item_bonuses = normalized_keys(b.item_skill_bonuses);
    let skills_by_name: HashMap<String, calc::Skill> = b
        .skills_by_name
        .into_iter()
        .map(|(k, v)| (norm(&k), v.into()))
        .collect();

    let no_scoped = calc::StatMap::new();
    let spell_input = calc::SkillInput {
        skill: &skill,
        allocated_rank: b.allocated_rank,
        attributes: &attributes,
        stats: &stats,
        skill_ranks_by_name: &skill_ranks,
        item_skill_bonuses: &item_bonuses,
        enemy_conditions: &b.enemy_conditions,
        enemy_resistances: &b.enemy_resistances,
        skills_by_name: &skills_by_name,
        projectile_count: b.projectile_count.max(0.0) as u32,
        of_total_damage: b.of_total_damage,
        scoped: &no_scoped,
        conversion_flat: 0.0,
        conversion_skill_damage_pct: 0.0,
    };
    let elemental = calc::compute_skill_damage(&spell_input);

    let attack_input = calc::AttackSkillInput {
        skill: &skill,
        allocated_rank: b.allocated_rank,
        attributes: &attributes,
        stats: &stats,
        skill_ranks_by_name: &skill_ranks,
        skills_by_name: &skills_by_name,
        item_skill_bonuses: &item_bonuses,
        enemy_conditions: &b.enemy_conditions,
        weapon: weapon.as_ref(),
        poison_breakdown: elemental.as_ref(),
        scoped: &no_scoped,
        projectile_count: b.projectile_count.max(0.0) as u32,
        conversion_flat: 0.0,
    };
    calc::compute_attack_skill_damage(&attack_input).map(Into::into)
}

#[cfg(test)]
mod attack_preview_tests {
    use super::*;

    #[test]
    fn attack_preview_scales_weapon_damage_by_skill_pct_and_aps() {
        let input: AttackSkillDamageInput = serde_json::from_value(serde_json::json!({
            "skill": {
                "name": "Heavy Ball",
                "damageType": "physical",
                "attackKind": "attack",
                "attackScaling": {
                    "weaponDamagePct": { "base": 100.0, "perLevel": 0.0 }
                }
            },
            "allocatedRank": 1.0,
            "stats": { "attacks_per_second": 2.0 },
            "weapon": { "name": "Test Maul", "damageMin": 100.0, "damageMax": 100.0 }
        }))
        .unwrap();

        let out = compute_attack_skill_damage(input).expect("attack breakdown");
        // 100 weapon dmg * 100% skill pct * 1.5 default crushing blow -> 150
        // per hit, 2 APS -> 300 dps.
        assert_eq!(out.combined_hit_min, 150);
        assert_eq!(out.combined_hit_max, 150);
        assert!((out.attacks_per_second_min - 2.0).abs() < 1e-9);
        assert!((out.dps_min - 300.0).abs() < 1e-6);
    }

    #[test]
    fn attack_preview_without_scaling_returns_none() {
        let input: AttackSkillDamageInput = serde_json::from_value(serde_json::json!({
            "skill": { "name": "Plain Spell" },
            "allocatedRank": 1.0
        }))
        .unwrap();
        assert!(compute_attack_skill_damage(input).is_none());
    }
}

#[tauri::command]
pub fn compute_weapon_damage(input: WeaponDamageInput) -> WeaponDamageOutput {
    let weapon: Option<calc::Weapon> = input.weapon.map(Into::into);
    let stats = ranged_map(normalized_keys(input.stats));
    calc::compute_weapon_damage(
        weapon.as_ref(),
        &stats,
        &input.enemy_conditions,
        &input.enemy_resistances,
        input.projectile_count.map(|p| p.max(0.0) as u32),
    )
    .into()
}

