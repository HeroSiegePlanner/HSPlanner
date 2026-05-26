use super::{
    ConditionMap, StatMap, Weapon, WeaponDamageBreakdown, collect_extra_damage, crit_factors,
    r_max, r_min, rg,
};

pub fn compute_weapon_damage(
    weapon: Option<&Weapon>,
    stats: &StatMap,
    enemy_conditions: &ConditionMap,
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

    let (extra_mult, extra_sources) = collect_extra_damage(stats, enemy_conditions);
    let extra_pct = (extra_mult - 1.0) * 100.0;

    let crit = crit_factors(stats, false);

    let base_min = w_min * (1.0 + r_min(ed) / 100.0) * (1.0 + r_min(ed_more) / 100.0) + r_min(add_phys);
    let base_max = w_max * (1.0 + r_max(ed) / 100.0) * (1.0 + r_max(ed_more) / 100.0) + r_max(add_phys);
    let hit_min = base_min * (1.0 + r_min(atk) / 100.0) * extra_mult;
    let hit_max = base_max * (1.0 + r_max(atk) / 100.0) * extra_mult;
    let crit_min = hit_min * crit.on_crit_mult;
    let crit_max = hit_max * crit.on_crit_mult;
    let avg_min = hit_min * crit.avg_mult;
    let avg_max = hit_max * crit.avg_mult;

    let ias = rg(stats, "increased_attack_speed");
    let ias_more = rg(stats, "increased_attack_speed_more");
    let base_aps = r_max(rg(stats, "attacks_per_second"));
    let aps_min = base_aps * (1.0 + r_min(ias) / 100.0) * (1.0 + r_min(ias_more) / 100.0);
    let aps_max = base_aps * (1.0 + r_max(ias) / 100.0) * (1.0 + r_max(ias_more) / 100.0);

    WeaponDamageBreakdown {
        has_weapon,
        weapon_name: weapon.map(|w| w.name.clone()),
        weapon_damage_min: w_min,
        weapon_damage_max: w_max,
        enhanced_damage_min_pct: r_min(ed),
        enhanced_damage_max_pct: r_max(ed),
        additive_physical_min: r_min(add_phys),
        additive_physical_max: r_max(add_phys),
        attack_damage_min_pct: r_min(atk),
        attack_damage_max_pct: r_max(atk),
        extra_damage_pct: extra_pct,
        extra_damage_sources: extra_sources,
        crit_chance: crit.chance,
        crit_damage_pct: crit.damage_pct,
        crit_multiplier_avg: crit.avg_mult,
        attacks_per_second_min: aps_min,
        attacks_per_second_max: aps_max,
        hit_min: hit_min.floor() as i64,
        hit_max: hit_max.floor() as i64,
        crit_min: crit_min.floor() as i64,
        crit_max: crit_max.floor() as i64,
        avg_min: avg_min.floor() as i64,
        avg_max: avg_max.floor() as i64,
        dps_min: (avg_min * aps_min).floor() as i64,
        dps_max: (avg_max * aps_max).floor() as i64,
    }
}
