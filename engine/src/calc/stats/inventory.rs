use super::*;
use std::borrow::Cow;

const RANDOM_ELEMENT_KEY: &str = "random_skill_element";
const ALL_SKILLS_CLASS_KEY: &str = "all_skills_class";

// Rolled keys stay inert until the user says what the item landed on.
// The resolved all_skills_{class} only pays out for a build of that class.
fn resolved_stat_key<'a>(
    stat_key: &'a str,
    element: Option<&str>,
    all_skills_class: Option<&str>,
) -> Option<Cow<'a, str>> {
    match stat_key {
        RANDOM_ELEMENT_KEY => element.map(|e| Cow::Owned(format!("{e}_skills"))),
        ALL_SKILLS_CLASS_KEY => {
            all_skills_class.map(|c| Cow::Owned(format!("all_skills_{c}")))
        }
        _ => Some(Cow::Borrowed(stat_key)),
    }
}

// ---------- inventory loop ----------

// Pushes implicit/affix/forge/runeword/socket/augment contributions; set bonuses
// come later. Returns `weapon_has_attack_speed` so baseline can skip default APS.
pub fn apply_inventory(
    inventory: &Inventory,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) -> bool {
    let mut weapon_has_attack_speed = false;

    for (slot_key, item, is_mirror) in data::inventory_entries(inventory) {
        let Some(base) = data::get_item(&item.base_id) else {
            continue;
        };
        let item_name = if is_mirror {
            format!("{} (mirrored)", base.name)
        } else {
            base.name.clone()
        };

        // Runeword suppresses per-socket gem contributions and implicit scaling.
        let socketed_refs: Vec<Option<&str>> =
            item.socketed.iter().map(|s| s.as_deref()).collect();
        let runeword = data::detect_runeword(base, &socketed_refs);
        let scale_implicit = runeword.is_none();
        let can_sf = data::can_star_forge(slot_key, &base.rarity);
        let effective_stars: Option<u32> = if can_sf { item.stars } else { None };

        let aps_in_implicit = base
            .implicit
            .as_ref()
            .map(|m| m.contains_key("attacks_per_second"))
            .unwrap_or(false);
        if slot_key == "weapon" && (aps_in_implicit || base.attack_speed.is_some()) {
            weapon_has_attack_speed = true;
        }

        let ed_override = item.implicit_overrides.get("enhanced_defense").copied();
        let ed_raw = base
            .implicit
            .as_ref()
            .and_then(|m| m.get("enhanced_defense"))
            .copied();
        let ed_scaled: Option<Ranged> = match (ed_override, ed_raw) {
            (Some(ov), _) => Some((ov, ov)),
            (None, Some(raw)) if scale_implicit => Some(apply_stars_to_ranged_value(
                raw.as_ranged(),
                "enhanced_defense",
                effective_stars,
            )),
            (None, Some(raw)) => Some(raw.as_ranged()),
            (None, None) => None,
        };
        if let Some(eff) =
            compute_item_effective_defense(base.defense_min, base.defense_max, ed_scaled)
        {
            if !is_zero(eff) {
                push_source(
                    stat_sources,
                    "defense",
                    SourceContribution {
                        label: item_name.clone(),
                        source_type: SourceType::Item,
                        value: eff,
                        forge: None,
                    },
                );
            }
        }

        if let Some(implicit) = base.implicit.as_ref() {
            for (stat_key, value) in implicit.iter() {
                // Frontend keys overrides by the raw base key, so check it first —
                // random_skill_element resolves to {element}_skills below.
                let raw_key = stat_key.as_str();
                let Some(stat_key) =
                    resolved_stat_key(
                    stat_key,
                    item.random_skill_element.as_deref(),
                    item.all_skills_class_id.as_deref(),
                )
                else {
                    continue;
                };
                let stat_key = stat_key.as_ref();
                let override_val = item
                    .implicit_overrides
                    .get(raw_key)
                    .or_else(|| item.implicit_overrides.get(stat_key))
                    .copied();
                let scaled: Ranged = match override_val {
                    Some(ov) => (ov, ov),
                    None if scale_implicit => apply_stars_to_ranged_value(
                        value.as_ranged(),
                        stat_key,
                        effective_stars,
                    ),
                    None => value.as_ranged(),
                };
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    stat_key,
                    scaled,
                    item_name.clone(),
                    SourceType::Item,
                    None,
                );
            }
        }

        // User-added overrides for keys not in base.implicit; ED handled above.
        for (stat_key, &value) in item.implicit_overrides.iter() {
            if let Some(implicit) = base.implicit.as_ref() {
                if implicit.contains_key(stat_key) {
                    continue;
                }
            }
            if stat_key == "enhanced_defense" {
                continue;
            }
            apply_contribution(
                attr_sources,
                stat_sources,
                stat_key,
                (value, value),
                item_name.clone(),
                SourceType::Item,
                None,
            );
        }

        if slot_key == "weapon" && !aps_in_implicit {
            if let Some(aps) = base.attack_speed {
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    "attacks_per_second",
                    aps.as_ranged(),
                    item_name.clone(),
                    SourceType::Item,
                    None,
                );
            }
        }

        for eq in item.affixes.iter() {
            let Some(affix) = data::get_affix(&eq.affix_id) else {
                continue;
            };
            let Some(stat_key) = affix.stat_key.as_deref() else {
                continue;
            };
            let Some(stat_key) = resolved_stat_key(
                stat_key,
                item.random_skill_element.as_deref(),
                item.all_skills_class_id.as_deref(),
            )
            else {
                continue;
            };
            let signed: f64 = if let Some(cv) = eq.custom_value {
                cv
            } else {
                rolled_affix_value_with_stars(affix, eq.roll, effective_stars)
            };
            if signed == 0.0 {
                continue;
            }
            let label = format!("{} ({})", affix.name, item_name);
            apply_contribution(
                attr_sources,
                stat_sources,
                stat_key.as_ref(),
                (signed, signed),
                label,
                SourceType::Item,
                None,
            );
        }

        if can_sf {
            if let Some(forge_kind) = data::forge_kind_for(&base.rarity) {
                for eq in item.forged_mods.iter() {
                    let Some(mod_def) = data::get_crystal_mod(&eq.affix_id) else {
                        continue;
                    };
                    let Some(stat_key) = mod_def.stat_key.as_deref() else {
                        continue;
                    };
                    let ranged: Ranged = if let Some(cv) = eq.custom_value {
                        (cv, cv)
                    } else {
                        rolled_affix_range(mod_def)
                    };
                    if is_zero(ranged) {
                        continue;
                    }
                    apply_contribution(
                        attr_sources,
                        stat_sources,
                        stat_key,
                        ranged,
                        mod_def.name.clone(),
                        SourceType::Item,
                        Some(Forge {
                            item_name: item_name.clone(),
                            mod_name: mod_def.name.clone(),
                            kind: forge_kind,
                        }),
                    );
                }
            }
        }

        // Runeword XOR per-socket contributions.
        if let Some(rw) = runeword {
            let label = format!("{} ({})", rw.name, item_name);
            for (stat_key, &value) in rw.stats.iter() {
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    stat_key,
                    (value, value),
                    label.clone(),
                    SourceType::Item,
                    None,
                );
            }
        } else {
            for (i, slot_id_opt) in item.socketed.iter().enumerate() {
                let Some(id) = slot_id_opt.as_deref() else {
                    continue;
                };
                let socketable = data::get_socketable_by_id(id);
                let (source_name, source_stats): (String, &std::collections::HashMap<String, f64>) =
                    match socketable {
                        Some(data::Socketable::Gem(g)) => (g.name.clone(), &g.stats),
                        Some(data::Socketable::Rune(r)) => (r.name.clone(), &r.stats),
                        None => continue,
                    };
                let is_rainbow = item
                    .socket_types
                    .get(i)
                    .copied()
                    .unwrap_or(SocketType::Normal)
                    == SocketType::Rainbow
                    || base
                        .rainbow_sockets
                        .as_ref()
                        .is_some_and(|r| r.contains(&(i as u32 + 1)));
                let mult = if is_rainbow { RAINBOW_MULTIPLIER } else { 1.0 };
                let transform = base.socket_transforms.as_ref().and_then(|m| m.get(id));
                let effective_stats: &std::collections::HashMap<String, f64> =
                    transform.unwrap_or(source_stats);
                let socket_label = {
                    let mut s = format!("{} in {} #{}", source_name, item_name, i + 1);
                    if is_rainbow {
                        s.push_str(" (Rainbow)");
                    }
                    if transform.is_some() {
                        s.push_str(" (Transform)");
                    }
                    s
                };
                for (stat_key, &raw_value) in effective_stats.iter() {
                    let v = raw_value * mult;
                    apply_contribution(
                        attr_sources,
                        stat_sources,
                        stat_key,
                        (v, v),
                        socket_label.clone(),
                        SourceType::Socket,
                        None,
                    );
                }
            }
        }

        if let Some(aug_ref) = item.augment.as_ref() {
            if let Some(aug) = data::get_augment(&aug_ref.id) {
                if !aug.levels.is_empty() {
                    let lvl = aug_ref
                        .level
                        .clamp(1, aug.levels.len() as u32);
                    if let Some(tier) = aug.levels.get((lvl - 1) as usize) {
                        let label =
                            format!("Augment: {} Lv {} ({})", aug.name, lvl, item_name);
                        for (stat_key, &value) in tier.stats.iter() {
                            apply_contribution(
                                attr_sources,
                                stat_sources,
                                stat_key,
                                (value, value),
                                label.clone(),
                                SourceType::Item,
                                None,
                            );
                        }
                    }
                }
            }
        }
    }

    weapon_has_attack_speed
}

