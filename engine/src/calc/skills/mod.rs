use std::collections::HashMap;

use super::conditions::is_condition_active;

pub mod ailment;
pub mod attack;
pub mod conversion;
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

pub(crate) const CRUSHING_BLOW_DEFAULT: f64 = 1.5;
// S8: a deadly blow hits for 1.35x (was 1.5x), scaled by deadly blow effectiveness.
pub(crate) const DEADLY_BLOW_ON_PROC_MULT: f64 = 1.35;

// avg = (1 - chance) + chance * 1.35 * (1 + effect); chance past 100% is meaningless.
pub(crate) fn deadly_blow_mult(chance_pct: f64, effect_pct: f64) -> f64 {
    let chance = chance_pct.clamp(0.0, 100.0) / 100.0;
    1.0 + chance * (DEADLY_BLOW_ON_PROC_MULT * (1.0 + effect_pct / 100.0) - 1.0)
}

// (stat key, label, counts as an ailment for the generic ailment bonuses). The
// gating condition is derived from the stat key by `conditions::condition_key_for`.
pub const EXTRA_DAMAGE_CONDITIONS: &[(&str, &str, bool)] = &[
    ("extra_damage_stunned",        "Stunned",        true),
    ("extra_damage_bleeding",       "Bleeding",       true),
    ("extra_damage_frozen",         "Frozen",         true),
    ("extra_damage_poisoned",       "Poisoned",       true),
    ("extra_damage_burning",        "Burning",        true),
    ("extra_damage_stasis",         "Stasis",         true),
    ("extra_damage_shadow_burning", "Shadow Burning", true),
    ("extra_damage_frost_bitten",   "Frost Bitten",   true),
    ("extra_dmg_to_deep_frozen",    "Deep Frozen",    true),
    ("extra_damage_low_life",       "Low Life",       false),
    ("extra_damage_serrated_chains","Serrated Chains",false),
    ("extra_damage_bosses",         "Bosses",         false),
];

// Same gating, but the bonus only lands when the hit's damage type matches.
const ELEMENT_EXTRA_DAMAGE: &[(&str, &str, &str)] = &[
    ("extra_fire_dmg_to_burning",      "fire",      "Fire vs Burning"),
    ("extra_poison_dmg_to_poisoned",   "poison",    "Poison vs Poisoned"),
    ("extra_physical_dmg_to_bleeding", "physical",  "Physical vs Bleeding"),
    ("extra_lightning_dmg_stasis",     "lightning", "Lightning vs Stasis"),
    ("extra_lightning_dmg_to_stasis",  "lightning", "Lightning vs Stasis"),
    ("extra_lightning_dmg_slow",       "lightning", "Lightning vs Slowed"),
];

// Generic "target is afflicted" bonuses; the data uses both spellings.
const AILMENT_EXTRA_DAMAGE: &[&str] = &[
    "extra_damage_ailments",
    "extra_dmg_to_monsters_afflicted_with_ailments",
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
    AttributePoint { source: String, stat: String, value: f64 },
    SkillLevel { source: String, stat: String, value: f64 },
}

impl BonusSource {
    pub fn stat(&self) -> &str {
        match self {
            BonusSource::AttributePoint { stat, .. } => stat,
            BonusSource::SkillLevel { stat, .. } => stat,
        }
    }
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
    pub extra_damage_sources: Vec<ExtraSource>,
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

// multiplier = Π(1 + pct_i / 100) over active ailment-gated extra-damage sources;
// stacks multiplicatively to match the HeroSiCalc reference formula.
pub(crate) fn collect_extra_damage(
    stats: &StatMap,
    enemy_conditions: &ConditionMap,
    damage_type: Option<&str>,
) -> (f64, Vec<ExtraSource>) {
    let mut sources: Vec<ExtraSource> = Vec::new();
    let mut sum_pct = 0.0;
    let mut any_ailment = false;

    for (stat_key, label, is_ailment) in EXTRA_DAMAGE_CONDITIONS {
        if !is_condition_active(stat_key, Some(enemy_conditions)) {
            continue;
        }
        any_ailment |= *is_ailment;
        push_source(stats, stat_key, label, &mut sum_pct, &mut sources);
    }

    for (stat_key, stat_damage_type, label) in ELEMENT_EXTRA_DAMAGE {
        if damage_type != Some(*stat_damage_type)
            || !is_condition_active(stat_key, Some(enemy_conditions))
        {
            continue;
        }
        any_ailment = true;
        push_source(stats, stat_key, label, &mut sum_pct, &mut sources);
    }

    if any_ailment {
        for stat_key in AILMENT_EXTRA_DAMAGE {
            push_source(
                stats,
                stat_key,
                "Afflicted with Ailments",
                &mut sum_pct,
                &mut sources,
            );
        }
    }

    // S8: the extra-damage modifiers are additive with each other, one shared multiplier.
    (1.0 + sum_pct / 100.0, sources)
}

fn push_source(
    stats: &StatMap,
    stat_key: &str,
    label: &'static str,
    sum_pct: &mut f64,
    sources: &mut Vec<ExtraSource>,
) {
    let v = rg(stats, stat_key);
    let avg = (r_min(v) + r_max(v)) * 0.5;
    if avg == 0.0 {
        return;
    }
    sources.push(ExtraSource { label, pct: avg });
    *sum_pct += avg;
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
        let (mult, sources) = collect_extra_damage(&stats, &conds, None);
        assert_eq!(mult, 1.0);
        assert!(sources.is_empty());
    }

    #[test]
    fn stacks_two_ailments_additively() {
        // S8: Bleeding +20% and Burning +20% are additive with each other,
        // 1 + (0.20 + 0.20) = 1.40 — NOT 1.2 * 1.2 = 1.44.
        let stats = merge(&[
            stat("extra_damage_bleeding", 20.0),
            stat("extra_damage_burning", 20.0),
        ]);
        let conds = cond(&["bleeding", "burning"]);
        let (mult, sources) = collect_extra_damage(&stats, &conds, None);
        assert!((mult - 1.40).abs() < 1e-9, "expected 1.40, got {mult}");
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn afflicted_with_ailments_adds_to_the_shared_sum() {
        // Bleeding +50% and a generic +30% "Afflicted with Ailments" bonus:
        // 1 + 0.5 + 0.3 = 1.8
        let stats = merge(&[
            stat("extra_damage_bleeding", 50.0),
            stat("extra_damage_ailments", 30.0),
        ]);
        let conds = cond(&["bleeding"]);
        let (mult, _) = collect_extra_damage(&stats, &conds, None);
        assert!((mult - 1.8).abs() < 1e-9, "expected 1.8, got {mult}");
    }

    #[test]
    fn ailment_with_zero_bonus_does_not_contribute() {
        let stats = stat("extra_damage_bleeding", 0.0);
        let conds = cond(&["bleeding"]);
        let (mult, sources) = collect_extra_damage(&stats, &conds, None);
        assert_eq!(mult, 1.0);
        assert!(sources.is_empty());
    }

    #[test]
    fn ui_condition_keys_reach_their_stats() {
        // The UI stores `shadow_burn` / `frozenbite` / `shocked` / `deep_frozen`,
        // which used to miss the differently-named stat keys entirely.
        for (ui_key, stat_key) in [
            ("shadow_burn", "extra_damage_shadow_burning"),
            ("frozenbite", "extra_damage_frost_bitten"),
            ("shocked", "extra_damage_stasis"),
            ("deep_frozen", "extra_dmg_to_deep_frozen"),
        ] {
            let (mult, _) = collect_extra_damage(&stat(stat_key, 50.0), &cond(&[ui_key]), None);
            assert!(
                (mult - 1.5).abs() < 1e-9,
                "{ui_key} did not reach {stat_key}"
            );
        }
    }

    #[test]
    fn burning_does_not_trigger_shadow_burning_bonus() {
        let stats = stat("extra_damage_shadow_burning", 50.0);
        let (mult, _) = collect_extra_damage(&stats, &cond(&["burning"]), None);
        assert_eq!(mult, 1.0);
    }

    #[test]
    fn element_gated_bonus_needs_a_matching_damage_type() {
        let stats = stat("extra_fire_dmg_to_burning", 40.0);
        let conds = cond(&["burning"]);
        let (fire, _) = collect_extra_damage(&stats, &conds, Some("fire"));
        assert!((fire - 1.4).abs() < 1e-9, "expected 1.4, got {fire}");
        let (cold, _) = collect_extra_damage(&stats, &conds, Some("cold"));
        assert_eq!(cold, 1.0);
    }

    #[test]
    fn boss_and_serrated_chains_bonuses_reach_their_toggles() {
        for (ui_key, stat_key) in [
            ("is_boss", "extra_damage_bosses"),
            ("serrated_chains", "extra_damage_serrated_chains"),
        ] {
            let (mult, sources) = collect_extra_damage(&stat(stat_key, 40.0), &cond(&[ui_key]), None);
            assert!((mult - 1.4).abs() < 1e-9, "{ui_key} did not reach {stat_key}");
            assert_eq!(sources.len(), 1);
            let (off, _) = collect_extra_damage(&stat(stat_key, 40.0), &cond(&[]), None);
            assert_eq!(off, 1.0, "{stat_key} must stay gated by {ui_key}");
        }
    }

    #[test]
    fn boss_and_serrated_chains_are_not_ailments() {
        let stats = merge(&[
            stat("extra_damage_bosses", 50.0),
            stat("extra_damage_serrated_chains", 50.0),
            stat("extra_damage_ailments", 30.0),
        ]);
        let (mult, sources) =
            collect_extra_damage(&stats, &cond(&["is_boss", "serrated_chains"]), None);
        assert!((mult - 2.0).abs() < 1e-9, "expected additive 1 + 0.5 + 0.5, got {mult}");
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn deadly_blow_mult_matches_s8_formula() {
        // no chance -> neutral
        assert!((deadly_blow_mult(0.0, 0.0) - 1.0).abs() < 1e-9);
        // 100% chance, no effect -> flat 1.35x
        assert!((deadly_blow_mult(100.0, 0.0) - 1.35).abs() < 1e-9);
        // 35% chance: 1 + 0.35 * 0.35
        assert!((deadly_blow_mult(35.0, 0.0) - 1.1225).abs() < 1e-9);
        // effect scales the on-proc multiplier: 1.35 * 2 = 2.7 on proc
        assert!((deadly_blow_mult(100.0, 100.0) - 2.7).abs() < 1e-9);
        // chance clamps at 100%
        assert!((deadly_blow_mult(160.0, 0.0) - 1.35).abs() < 1e-9);
    }

    #[test]
    fn low_life_does_not_count_as_an_ailment() {
        let stats = merge(&[
            stat("extra_damage_low_life", 50.0),
            stat("extra_damage_ailments", 30.0),
        ]);
        let (mult, sources) = collect_extra_damage(&stats, &cond(&["low_life"]), None);
        assert!((mult - 1.5).abs() < 1e-9, "expected 1.5, got {mult}");
        assert_eq!(sources.len(), 1);
    }
}
