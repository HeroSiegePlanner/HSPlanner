use std::collections::HashMap;

pub mod attack;
pub mod damage;
pub mod weapon;

pub use attack::{AttackSkillInput, compute_attack_skill_damage};
pub use damage::{SkillInput, compute_skill_damage};
pub use weapon::compute_weapon_damage;

pub type Ranged = (f64, f64);
pub type StatMap = HashMap<String, Ranged>;
pub type AttrMap = HashMap<String, Ranged>;
pub type ConditionMap = HashMap<String, bool>;
pub type ResistMap = HashMap<String, f64>;
pub type ItemSkillBonuses = HashMap<String, (f64, f64)>;
pub type SkillRanks = HashMap<String, f64>;

pub const ELEMENTS: [&str; 5] = ["fire", "cold", "lightning", "poison", "arcane"];

pub const EXTRA_DAMAGE_CONDITIONS: &[(&str, &str, &str)] = &[
    ("extra_damage_stunned",        "stunned",        "Stunned"),
    ("extra_damage_bleeding",       "bleeding",       "Bleeding"),
    ("extra_damage_frozen",         "frozen",         "Frozen"),
    ("extra_damage_poisoned",       "poisoned",       "Poisoned"),
    ("extra_damage_burning",        "burning",        "Burning"),
    ("extra_damage_stasis",         "stasis",         "Stasis"),
    ("extra_damage_shadow_burning", "shadow_burning", "Shadow Burning"),
    ("extra_damage_frost_bitten",   "frost_bitten",   "Frost Bitten"),
];

#[derive(Debug, Clone)]
pub struct DamageFormula {
    pub base: f64,
    pub per_level: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct DamageRow {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone)]
pub enum BonusSource {
    AttributePoint { source: String, value: f64 },
    SkillLevel { source: String, value: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackKind {
    Attack,
    Spell,
}

#[derive(Debug, Clone, Default)]
pub struct AttackSkillScaling {
    pub weapon_damage_pct: Option<DamageFormula>,
    pub flat_physical_min: Option<DamageFormula>,
    pub flat_physical_max: Option<DamageFormula>,
    pub attack_rating_pct: Option<DamageFormula>,
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub tags: Vec<String>,
    pub damage_type: Option<String>,
    pub damage_formula: Option<DamageFormula>,
    pub damage_per_rank: Option<Vec<DamageRow>>,
    pub bonus_sources: Vec<BonusSource>,
    pub attack_kind: Option<AttackKind>,
    pub attack_scaling: Option<AttackSkillScaling>,
}

#[derive(Debug, Clone, Default)]
pub struct Weapon {
    pub name: String,
    pub damage_min: f64,
    pub damage_max: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExtraSource {
    pub label: &'static str,
    pub pct: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDamageBreakdown {
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
    pub extra_damage_sources: Vec<ExtraSource>,
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

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackSkillDamageBreakdown {
    pub effective_rank_min: f64,
    pub effective_rank_max: f64,
    pub weapon_damage_pct_min: f64,
    pub weapon_damage_pct_max: f64,
    pub skill_flat_phys_min: f64,
    pub skill_flat_phys_max: f64,
    pub attack_rating_pct_min: f64,
    pub attack_rating_pct_max: f64,
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

#[derive(Debug, Clone, Default)]
pub struct WeaponDamageBreakdown {
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
    pub additive_elemental_breakdown: Vec<ExtraSource>,
    pub attack_damage_min_pct: f64,
    pub attack_damage_max_pct: f64,
    pub extra_damage_pct: f64,
    pub extra_damage_sources: Vec<ExtraSource>,
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

#[inline]
pub fn rg(map: &HashMap<String, Ranged>, k: &str) -> Ranged {
    *map.get(k).unwrap_or(&(0.0, 0.0))
}

#[inline]
pub fn r_min(v: Ranged) -> f64 {
    v.0
}

#[inline]
pub fn r_max(v: Ranged) -> f64 {
    v.1
}

// Returns (multiplier, sources) where multiplier = Π(1 + pct_i / 100) over
// all active ailment-gated extra-damage sources. Stacks multiplicatively to
// match the HeroSiCalc reference formula.
pub(crate) fn collect_extra_damage(
    stats: &StatMap,
    enemy_conditions: &ConditionMap,
) -> (f64, Vec<ExtraSource>) {
    let mut sources: Vec<ExtraSource> = Vec::new();
    let mut mult = 1.0;
    let mut any_ailment = false;
    for (stat_key, cond, label) in EXTRA_DAMAGE_CONDITIONS {
        if !*enemy_conditions.get(*cond).unwrap_or(&false) {
            continue;
        }
        any_ailment = true;
        let v = rg(stats, stat_key);
        let avg = (r_min(v) + r_max(v)) * 0.5;
        if avg == 0.0 {
            continue;
        }
        sources.push(ExtraSource { label, pct: avg });
        mult *= 1.0 + avg / 100.0;
    }
    if any_ailment {
        let v = rg(stats, "extra_damage_ailments");
        let avg = (r_min(v) + r_max(v)) * 0.5;
        if avg != 0.0 {
            sources.push(ExtraSource {
                label: "Afflicted with Ailments",
                pct: avg,
            });
            mult *= 1.0 + avg / 100.0;
        }
    }
    (mult, sources)
}

pub(crate) struct CritFactors {
    pub chance: f64,
    pub damage_pct: f64,
    pub on_crit_mult: f64,
    pub avg_mult: f64,
}

pub(crate) fn crit_factors(stats: &StatMap, is_spell: bool) -> CritFactors {
    let chance = r_max(rg(
        stats,
        if is_spell { "spell_crit_chance" } else { "crit_chance" },
    ));
    let damage_pct = r_max(rg(
        stats,
        if is_spell { "spell_crit_damage" } else { "crit_damage" },
    ));
    let damage_more = if is_spell {
        0.0
    } else {
        r_max(rg(stats, "crit_damage_more"))
    };
    let on_crit_mult = (1.0 + damage_pct / 100.0) * (1.0 + damage_more / 100.0);
    let clamped = chance.clamp(0.0, 95.0) / 100.0;
    let avg_mult = 1.0 - clamped + clamped * on_crit_mult;
    CritFactors {
        chance,
        damage_pct,
        on_crit_mult,
        avg_mult,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cond(active: &[&str]) -> ConditionMap {
        let mut m = ConditionMap::new();
        for c in active {
            m.insert((*c).to_string(), true);
        }
        m
    }

    fn stat(key: &str, pct: f64) -> StatMap {
        let mut m = StatMap::new();
        m.insert(key.to_string(), (pct, pct));
        m
    }

    fn merge(maps: &[StatMap]) -> StatMap {
        let mut out = StatMap::new();
        for m in maps {
            for (k, v) in m {
                out.insert(k.clone(), *v);
            }
        }
        out
    }

    #[test]
    fn returns_unit_multiplier_when_no_ailment_active() {
        let stats = stat("extra_damage_bleeding", 50.0);
        let conds = cond(&[]);
        let (mult, sources) = collect_extra_damage(&stats, &conds);
        assert_eq!(mult, 1.0);
        assert!(sources.is_empty());
    }

    #[test]
    fn stacks_two_ailments_multiplicatively() {
        // Bleeding +20% and Burning +20% must yield 1.2 * 1.2 = 1.44,
        // NOT 1 + (0.20 + 0.20) = 1.40 (the old additive behavior).
        let stats = merge(&[
            stat("extra_damage_bleeding", 20.0),
            stat("extra_damage_burning", 20.0),
        ]);
        let conds = cond(&["bleeding", "burning"]);
        let (mult, sources) = collect_extra_damage(&stats, &conds);
        assert!((mult - 1.44).abs() < 1e-9, "expected 1.44, got {mult}");
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn afflicted_with_ailments_stacks_multiplicatively_over_others() {
        // Bleeding +50% and a generic +30% "Afflicted with Ailments" bonus:
        // 1.5 * 1.3 = 1.95
        let stats = merge(&[
            stat("extra_damage_bleeding", 50.0),
            stat("extra_damage_ailments", 30.0),
        ]);
        let conds = cond(&["bleeding"]);
        let (mult, _) = collect_extra_damage(&stats, &conds);
        assert!((mult - 1.95).abs() < 1e-9, "expected 1.95, got {mult}");
    }

    #[test]
    fn ailment_with_zero_bonus_does_not_contribute() {
        let stats = stat("extra_damage_bleeding", 0.0);
        let conds = cond(&["bleeding"]);
        let (mult, sources) = collect_extra_damage(&stats, &conds);
        assert_eq!(mult, 1.0);
        assert!(sources.is_empty());
    }
}
