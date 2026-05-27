use std::collections::HashMap;

use super::{
    ConditionMap, ELEMENTS, ExtraSource, StatMap, Weapon, WeaponDamageBreakdown,
    collect_extra_damage, r_max, r_min, rg,
};

const CRUSHING_BLOW_DEFAULT: f64 = 1.5;
const DEADLY_BLOW_MORE_PER_PROC: f64 = 0.5;
const OPEN_WOUNDS_FRACTION: f64 = 0.2;
const OPEN_WOUNDS_PHYS_MULT: f64 = 1.5;
const DEFAULT_HIT_CHANCE: f64 = 100.0;

fn additive_elemental_breakdown(stats: &StatMap) -> (f64, f64, Vec<ExtraSource>) {
    let mut min_sum = 0.0;
    let mut max_sum = 0.0;
    let mut sources: Vec<ExtraSource> = Vec::new();
    for elem in ELEMENTS {
        let v = rg(stats, &format!("additive_{elem}_damage"));
        let lo = r_min(v);
        let hi = r_max(v);
        if lo == 0.0 && hi == 0.0 {
            continue;
        }
        min_sum += lo;
        max_sum += hi;
        sources.push(ExtraSource {
            label: elem,
            pct: (lo + hi) * 0.5,
        });
    }
    (min_sum, max_sum, sources)
}

fn open_wounds_damage(
    base_phys_with_skill: f64,
    crush_armor_mult: f64,
    status_mult: f64,
    projectiles: f64,
) -> f64 {
    base_phys_with_skill
        * OPEN_WOUNDS_PHYS_MULT
        * crush_armor_mult
        * status_mult
        * projectiles
        * OPEN_WOUNDS_FRACTION
}

pub fn compute_weapon_damage(
    weapon: Option<&Weapon>,
    stats: &StatMap,
    enemy_conditions: &ConditionMap,
    enemy_resistances: &HashMap<String, f64>,
    projectile_count: Option<u32>,
) -> WeaponDamageBreakdown {
    let has_weapon = weapon.is_some();
    // Unarmed: every character has a 2-6 base physical damage when no weapon is equipped.
    let (w_min, w_max) = weapon
        .map(|w| (w.damage_min, w.damage_max))
        .unwrap_or((2.0, 6.0));

    let ed = rg(stats, "enhanced_damage");
    let ed_more = rg(stats, "enhanced_damage_more");
    let add_phys = rg(stats, "additive_physical_damage");
    let atk = rg(stats, "attack_damage");

    let weapon_min = w_min * (1.0 + r_min(ed) / 100.0) * (1.0 + r_min(ed_more) / 100.0);
    let weapon_max = w_max * (1.0 + r_max(ed) / 100.0) * (1.0 + r_max(ed_more) / 100.0);
    let add_phys_min = r_min(add_phys);
    let add_phys_max = r_max(add_phys);

    let (add_elem_min, add_elem_max, add_elem_breakdown) = additive_elemental_breakdown(stats);

    let skill_min = r_min(atk);
    let skill_max = r_max(atk);

    // PRD §4.1: base_phys = weapon * (1 + Skill%) + AddPhys.
    // AddElem joins later (line "(phys_eff + AddElem)") and AddPhys is added
    // back outside the Skill% multiplier.
    let base_phys_min = weapon_min * (1.0 + skill_min / 100.0) + add_phys_min;
    let base_phys_max = weapon_max * (1.0 + skill_max / 100.0) + add_phys_max;

    // Crit: averaged via (1 + crit% * chance% / 10000)
    let crit_chance = r_max(rg(stats, "crit_chance"));
    let crit_damage_pct = r_max(rg(stats, "crit_damage"));
    let crit_mult = 1.0 + crit_damage_pct * crit_chance / 10_000.0;
    let crit_mult_on_proc = 1.0 + crit_damage_pct / 100.0;

    let phys_crit_min = base_phys_min * crit_mult;
    let phys_crit_max = base_phys_max * crit_mult;

    let crushing_raw = r_max(rg(stats, "crushing_blow_modifier"));
    let crushing_blow_modifier = if crushing_raw == 0.0 {
        CRUSHING_BLOW_DEFAULT
    } else {
        crushing_raw
    };
    let armor_break_pct = r_max(rg(stats, "armor_break"));
    let crush_armor_mult = crushing_blow_modifier + armor_break_pct / 100.0;

    let deadly_blow_chance = r_max(rg(stats, "deadly_blow_chance"));
    let deadly_mult = 1.0 + DEADLY_BLOW_MORE_PER_PROC * deadly_blow_chance / 100.0;

    let phys_eff_min = phys_crit_min * crush_armor_mult * deadly_mult;
    let phys_eff_max = phys_crit_max * crush_armor_mult * deadly_mult;

    let (status_mult, extra_sources) = collect_extra_damage(stats, enemy_conditions);
    let extra_pct = (status_mult - 1.0) * 100.0;

    let hit_chance_raw = r_max(rg(stats, "hit_chance"));
    let hit_chance = if hit_chance_raw == 0.0 {
        DEFAULT_HIT_CHANCE
    } else {
        hit_chance_raw
    };
    let projectiles = projectile_count.unwrap_or(1).max(1) as f64;

    let hit_min_raw =
        (phys_eff_min + add_elem_min) * status_mult * (hit_chance / 100.0) * projectiles;
    let hit_max_raw =
        (phys_eff_max + add_elem_max) * status_mult * (hit_chance / 100.0) * projectiles;

    // Open wounds: not affected by crit or deadly.
    let ow_min = open_wounds_damage(base_phys_min, crush_armor_mult, status_mult, projectiles);
    let ow_max = open_wounds_damage(base_phys_max, crush_armor_mult, status_mult, projectiles);

    let ias = rg(stats, "increased_attack_speed");
    let ias_more = rg(stats, "increased_attack_speed_more");
    let base_aps = r_max(rg(stats, "attacks_per_second"));
    let aps_min = base_aps * (1.0 + r_min(ias) / 100.0) * (1.0 + r_min(ias_more) / 100.0);
    let aps_max = base_aps * (1.0 + r_max(ias) / 100.0) * (1.0 + r_max(ias_more) / 100.0);

    let enemy_phys_res_pct = enemy_resistances.get("physical").copied().unwrap_or(0.0);
    let phys_resistance_ignored_pct = r_max(rg(stats, "ignore_physical_res")).clamp(0.0, 100.0);
    let effective_res_pct = enemy_phys_res_pct * (1.0 - phys_resistance_ignored_pct / 100.0);
    let res_mult = 1.0 - effective_res_pct / 100.0;

    // Crit-on-proc values (max-roll variant, used for "max hit" tooltip).
    // Uses base_phys (not phys_crit) to avoid stacking averaged + proc multipliers.
    let crit_min_raw =
        base_phys_min * crit_mult_on_proc * crush_armor_mult * deadly_mult + add_elem_min;
    let crit_max_raw =
        base_phys_max * crit_mult_on_proc * crush_armor_mult * deadly_mult + add_elem_max;

    let dps_min_raw = (hit_min_raw * aps_min + ow_min) * res_mult;
    let dps_max_raw = (hit_max_raw * aps_max + ow_max) * res_mult;

    let avg_min = hit_min_raw * res_mult;
    let avg_max = hit_max_raw * res_mult;

    WeaponDamageBreakdown {
        has_weapon,
        weapon_name: weapon.map(|w| w.name.clone()),
        weapon_damage_min: w_min,
        weapon_damage_max: w_max,
        enhanced_damage_min_pct: r_min(ed),
        enhanced_damage_max_pct: r_max(ed),
        additive_physical_min: add_phys_min,
        additive_physical_max: add_phys_max,
        additive_elemental_min: add_elem_min,
        additive_elemental_max: add_elem_max,
        additive_elemental_breakdown: add_elem_breakdown,
        attack_damage_min_pct: skill_min,
        attack_damage_max_pct: skill_max,
        extra_damage_pct: extra_pct,
        extra_damage_sources: extra_sources,
        crushing_blow_modifier,
        armor_break_pct,
        deadly_blow_chance,
        hit_chance,
        crit_chance,
        crit_damage_pct,
        crit_multiplier_avg: crit_mult,
        attacks_per_second_min: aps_min,
        attacks_per_second_max: aps_max,
        projectile_count: projectiles as u32,
        enemy_phys_res_pct,
        phys_resistance_ignored_pct,
        hit_min: hit_min_raw.floor() as i64,
        hit_max: hit_max_raw.floor() as i64,
        crit_min: crit_min_raw.floor() as i64,
        crit_max: crit_max_raw.floor() as i64,
        avg_min: avg_min.floor() as i64,
        avg_max: avg_max.floor() as i64,
        open_wounds_min: ow_min.floor() as i64,
        open_wounds_max: ow_max.floor() as i64,
        dps_min: dps_min_raw.floor() as i64,
        dps_max: dps_max_raw.floor() as i64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weapon(min: f64, max: f64) -> Weapon {
        Weapon {
            name: "test".into(),
            damage_min: min,
            damage_max: max,
        }
    }

    fn stat(key: &str, v: f64) -> (String, (f64, f64)) {
        (key.to_string(), (v, v))
    }

    fn stats(items: &[(&str, f64)]) -> StatMap {
        items.iter().map(|(k, v)| stat(k, *v)).collect()
    }

    fn conds(active: &[&str]) -> ConditionMap {
        active.iter().map(|c| (c.to_string(), true)).collect()
    }

    fn no_res() -> HashMap<String, f64> {
        HashMap::new()
    }

    #[test]
    fn unarmed_no_buffs_yields_base_2_to_6_with_crushing_default() {
        let s = stats(&[]);
        let r = compute_weapon_damage(None, &s, &ConditionMap::new(), &no_res(), None);
        // base_phys = (2..6 - 0 - 0) * 1 + 0 = 2..6; phys_eff = 2..6 * 1.5 * 1 = 3..9
        assert_eq!(r.hit_min, 3);
        assert_eq!(r.hit_max, 9);
        // ow_min = 2 * 1.5 * 1.5 * 1 * 1 * 0.2 = 0.9 → 0
        assert_eq!(r.open_wounds_min, 0);
        // ow_max = 6 * 1.5 * 1.5 * 1 * 1 * 0.2 = 2.7 → 2
        assert_eq!(r.open_wounds_max, 2);
        assert!(!r.has_weapon);
    }

    #[test]
    fn weapon_with_crit_chance_uses_averaged_multiplier() {
        // crit% = 100, chance = 50 → crit_mult = 1 + 100*50/10000 = 1.5
        // base = 100, phys_eff = 100 * 1.5 * 1.5 = 225
        let w = weapon(100.0, 100.0);
        let s = stats(&[("crit_damage", 100.0), ("crit_chance", 50.0)]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), None);
        assert_eq!(r.hit_min, 225);
        assert_eq!(r.hit_max, 225);
    }

    #[test]
    fn additive_elemental_added_outside_skill_and_crit() {
        // weapon=100, AddElem fire=50, Skill%=100
        //   base_phys = 100 * 2 + 0 = 200; phys_eff = 200 * 1.5 = 300
        //   hit = 300 + 50 = 350 (AddElem joins outside Skill% and crushing)
        let w = weapon(100.0, 100.0);
        let s = stats(&[("attack_damage", 100.0), ("additive_fire_damage", 50.0)]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), None);
        assert_eq!(r.hit_min, 350);
        assert_eq!(r.hit_max, 350);
        assert_eq!(r.additive_elemental_min, 50.0);
        assert_eq!(r.additive_elemental_breakdown.len(), 1);
    }

    #[test]
    fn deadly_blow_chance_averages_a_fifty_percent_proc() {
        // weapon 100, Deadly% = 100 → deadly_mult = 1 + 0.5 = 1.5
        // phys_eff = 100 * 1.5 * 1.5 = 225
        let w = weapon(100.0, 100.0);
        let s = stats(&[("deadly_blow_chance", 100.0)]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), None);
        assert_eq!(r.hit_min, 225);
    }

    #[test]
    fn ailments_stack_multiplicatively_in_hit() {
        // bleeding +20% and burning +30% → status_mult = 1.2 * 1.3 = 1.56
        // base = 100, phys_eff = 150, hit = 150 * 1.56 = 234
        let w = weapon(100.0, 100.0);
        let s = stats(&[
            ("extra_damage_bleeding", 20.0),
            ("extra_damage_burning", 30.0),
        ]);
        let c = conds(&["bleeding", "burning"]);
        let r = compute_weapon_damage(Some(&w), &s, &c, &no_res(), None);
        assert_eq!(r.hit_min, 234);
    }

    #[test]
    fn enemy_phys_res_reduces_dps_with_ignore_factor() {
        // res = 50, ignore = 50 → eff_res = 25 → res_mult = 0.75
        // base = 100, hit = 150, APS = 1, ow = 45 → dps = (150 + 45) * 0.75 = 146.25
        let w = weapon(100.0, 100.0);
        let s = stats(&[
            ("attacks_per_second", 1.0),
            ("ignore_physical_res", 50.0),
        ]);
        let mut res = HashMap::new();
        res.insert("physical".into(), 50.0);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &res, None);
        assert_eq!(r.dps_min, 146);
        assert_eq!(r.enemy_phys_res_pct, 50.0);
        assert_eq!(r.phys_resistance_ignored_pct, 50.0);
    }

    #[test]
    fn projectile_count_scales_hit_and_open_wounds() {
        // Proj = 3 → hit = 450, ow = 135
        let w = weapon(100.0, 100.0);
        let s = stats(&[]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), Some(3));
        assert_eq!(r.hit_min, 450);
        assert_eq!(r.open_wounds_min, 135);
        assert_eq!(r.projectile_count, 3);
    }

    #[test]
    fn open_wounds_unaffected_by_crit_and_deadly() {
        // Even with full crit/deadly, ow stays at 100 * 1.5 * 1.5 * 1 * 1 * 0.2 = 45
        let w = weapon(100.0, 100.0);
        let s = stats(&[
            ("crit_damage", 100.0),
            ("crit_chance", 100.0),
            ("deadly_blow_chance", 100.0),
        ]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), None);
        assert_eq!(r.open_wounds_min, 45);
    }

    #[test]
    fn weapon_explicit_zero_yields_zero_hit_and_open_wounds() {
        // PRD §7 sanity: weapon=0..0, no flats, no skill% → hit=0, ow=0.
        let w = weapon(0.0, 0.0);
        let s = stats(&[]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), None);
        assert_eq!(r.hit_min, 0);
        assert_eq!(r.hit_max, 0);
        assert_eq!(r.open_wounds_min, 0);
        assert_eq!(r.open_wounds_max, 0);
    }

    #[test]
    fn additive_elemental_aggregates_multiple_elements() {
        // fire=30 + cold=20 → sum=50, breakdown has both rows.
        let w = weapon(100.0, 100.0);
        let s = stats(&[
            ("additive_fire_damage", 30.0),
            ("additive_cold_damage", 20.0),
        ]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), None);
        assert_eq!(r.additive_elemental_min, 50.0);
        assert_eq!(r.additive_elemental_max, 50.0);
        assert_eq!(r.additive_elemental_breakdown.len(), 2);
        // base_phys=100, phys_eff=150, hit=150+50=200
        assert_eq!(r.hit_min, 200);
    }

    #[test]
    fn crit_min_max_reflect_full_proc_multiplier_not_averaged() {
        // crit% = 100, chance = 50 → averaged crit_mult = 1.5,
        // on-proc crit_mult = 2.0 → crit_min = base*2.0*crush*deadly = 100*2*1.5*1 = 300
        let w = weapon(100.0, 100.0);
        let s = stats(&[("crit_damage", 100.0), ("crit_chance", 50.0)]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), None);
        assert_eq!(r.hit_min, 225); // averaged
        assert_eq!(r.crit_min, 300); // full proc
    }

    #[test]
    fn hit_chance_below_default_scales_hit_raw_proportionally() {
        // hit_chance = 50 → hit = 150 * 0.5 = 75 (per PRD §4.1; arkusz I6 inputs)
        let w = weapon(100.0, 100.0);
        let s = stats(&[("hit_chance", 50.0)]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), None);
        assert_eq!(r.hit_min, 75);
        assert_eq!(r.hit_chance, 50.0);
    }

    #[test]
    fn enhanced_damage_scales_weapon_but_not_additive_phys() {
        // weapon=100, enhanced=100% → weapon_eff=200; additive_phys=50 added after.
        // base_phys=(250-50-0)*1+50=250, phys_eff=250*1.5=375.
        let w = weapon(100.0, 100.0);
        let s = stats(&[
            ("enhanced_damage", 100.0),
            ("additive_physical_damage", 50.0),
        ]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), None);
        assert_eq!(r.hit_min, 375);
    }

    #[test]
    fn skill_pct_scales_base_only_not_additive_phys_or_elem() {
        // weapon=100, additive_phys=20, additive_elem fire=30, skill%=100.
        // base_no_flats = 150-20-30 = 100; base_phys = 100*2 + 20 = 220
        // phys_eff = 220*1.5 = 330; hit = 330 + 30 = 360
        let w = weapon(100.0, 100.0);
        let s = stats(&[
            ("attack_damage", 100.0),
            ("additive_physical_damage", 20.0),
            ("additive_fire_damage", 30.0),
        ]);
        let r = compute_weapon_damage(Some(&w), &s, &ConditionMap::new(), &no_res(), None);
        assert_eq!(r.hit_min, 360);
    }

    #[test]
    fn full_stack_compound_parity() {
        // Compound smoke test of every multiplicative stage interacting.
        //   weapon = 100, attack_damage = 50%, additive_phys = 10, fire = 20
        //   crit% = 100, crit_chance = 100 → crit_mult = 2.0 (averaged = proc)
        //   deadly = 100 → deadly_mult = 1.5
        //   crushing default 1.5, armor_break = 0
        //   bleeding = 30% (active), burning = 20% (active) → status = 1.3 * 1.2 = 1.56
        //   proj = 2, hit_chance = 100 (default)
        //   enemy_phys_res = 40%, ignore = 50% → eff_res = 20%, res_mult = 0.8
        //   APS = 1.0
        let w = weapon(100.0, 100.0);
        let s = stats(&[
            ("attack_damage", 50.0),
            ("additive_physical_damage", 10.0),
            ("additive_fire_damage", 20.0),
            ("crit_damage", 100.0),
            ("crit_chance", 100.0),
            ("deadly_blow_chance", 100.0),
            ("extra_damage_bleeding", 30.0),
            ("extra_damage_burning", 20.0),
            ("attacks_per_second", 1.0),
            ("ignore_physical_res", 50.0),
        ]);
        let c = conds(&["bleeding", "burning"]);
        let mut res = HashMap::new();
        res.insert("physical".into(), 40.0);
        // base_phys = 100 * 1.5 + 10 = 160
        // phys_crit = 160 * 2 = 320
        // phys_eff  = 320 * 1.5 * 1.5 = 720
        // hit_raw   = (720 + 20) * 1.56 * 1 * 2 = 2308.8 → 2308
        // ow        = 160 * 1.5 * 1.5 * 1.56 * 2 * 0.2 = 224.64 → 224
        // dps       = (2308.8 + 224.64) * 0.8 = 2026.752 → 2026
        let r = compute_weapon_damage(Some(&w), &s, &c, &res, Some(2));
        assert_eq!(r.hit_min, 2308);
        assert_eq!(r.open_wounds_min, 224);
        assert_eq!(r.dps_min, 2026);
    }
}
