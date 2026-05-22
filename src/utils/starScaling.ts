// Per-statKey star scaling configuration.
// Source: research dump in `listamodow-2.txt`. Each star adds the listed % bonus
// (multiplicative on top of base rolled value), or a flat staircase bonus for
// skill-rank affixes that follow the documented (3*=+1, 5*=+2) pattern.

export type StarScaleConfig =
  | { kind: 'percent'; perStar: number }
  | { kind: 'flat-skill-staircase' }
  | { kind: 'item-specific-staircase' }
  | { kind: 'none' }
  | { kind: 'unknown' }
  | { kind: 'glitch' }

const DEFAULT_PERCENT_PER_STAR = 0

// Documented in listamodow-2.txt header:
//   "ITEM SPECIFIC 2* = +1 | 4* = +2 | 5* = +3"
// Used by item-granted skill ranks (`item_granted_skill_rank` / skillBonuses).
const ITEM_SPECIFIC_STAIRCASE: Record<number, number> = {
  0: 0,
  1: 0,
  2: 1,
  3: 1,
  4: 2,
  5: 3,
}

// Documented in listamodow-2.txt for affixes such as fire_skills / cold_skills:
//   "na trzech * +1 na pieciu * +2"
const FLAT_SKILL_STAIRCASE: Record<number, number> = {
  0: 0,
  1: 0,
  2: 0,
  3: 1,
  4: 1,
  5: 2,
}

// Map of statKey -> star scaling rule. Anything not listed defaults to "none"
// (no scaling) so additions to the affix system are explicit, not silently
// inheriting the old global 8% per-star bonus.
const STAR_SCALE_MAP: Readonly<Record<string, StarScaleConfig>> = {
  // ----- attributes (flat) -----
  to_strength: { kind: 'percent', perStar: 5 },
  to_dexterity: { kind: 'percent', perStar: 5 },
  to_intelligence: { kind: 'percent', perStar: 5 },
  to_energy: { kind: 'percent', perStar: 5 },
  to_vitality: { kind: 'percent', perStar: 5 },
  to_armor: { kind: 'percent', perStar: 5 },
  all_attributes: { kind: 'percent', perStar: 4 },

  // ----- attributes (% increased) -----
  increased_all_attributes: { kind: 'percent', perStar: 10 },

  // ----- resistances -----
  fire_resistance: { kind: 'percent', perStar: 5 },
  cold_resistance: { kind: 'percent', perStar: 5 },
  lightning_resistance: { kind: 'percent', perStar: 5 },
  poison_resistance: { kind: 'percent', perStar: 5 },
  arcane_resistance: { kind: 'percent', perStar: 5 },
  all_resistances: { kind: 'percent', perStar: 3 },

  // ----- maximum resistances -----
  max_fire_resistance: { kind: 'unknown' },
  max_cold_resistance: { kind: 'unknown' },
  max_lightning_resistance: { kind: 'unknown' },
  max_poison_resistance: { kind: 'unknown' },
  max_arcane_resistance: { kind: 'unknown' },
  max_all_resistances: { kind: 'unknown' },

  // ----- absorption (no scaling) -----
  fire_absorption: { kind: 'none' },
  cold_absorption: { kind: 'none' },
  lightning_absorption: { kind: 'none' },
  poison_absorption: { kind: 'none' },
  arcane_absorption: { kind: 'none' },
  magic_absorption: { kind: 'none' },

  // ----- skill ranks (flat staircase) -----
  all_skills: { kind: 'none' },
  fire_skills: { kind: 'flat-skill-staircase' },
  cold_skills: { kind: 'flat-skill-staircase' },
  lightning_skills: { kind: 'flat-skill-staircase' },
  poison_skills: { kind: 'flat-skill-staircase' },
  arcane_skills: { kind: 'flat-skill-staircase' },
  physical_skills: { kind: 'percent', perStar: 8 },
  summon_skills: { kind: 'unknown' },
  explosion_skills: { kind: 'none' },

  // ----- skill damage (% increased) -----
  magic_skill_damage: { kind: 'percent', perStar: 3 },
  fire_skill_damage: { kind: 'percent', perStar: 4 },
  cold_skill_damage: { kind: 'percent', perStar: 4 },
  lightning_skill_damage: { kind: 'percent', perStar: 4 },
  poison_skill_damage: { kind: 'percent', perStar: 4 },
  arcane_skill_damage: { kind: 'percent', perStar: 4 },

  // ----- additive elemental damage (flat) -----
  additive_physical_damage: { kind: 'percent', perStar: 5 },
  additive_fire_damage: { kind: 'percent', perStar: 5 },
  additive_cold_damage: { kind: 'percent', perStar: 5 },
  additive_lightning_damage: { kind: 'percent', perStar: 5 },
  additive_poison_damage: { kind: 'percent', perStar: 5 },
  additive_arcane_damage: { kind: 'percent', perStar: 5 },

  // ----- flat skill damage (raw skill-damage adds) -----
  flat_fire_skill_damage: { kind: 'percent', perStar: 5 },
  flat_cold_skill_damage: { kind: 'percent', perStar: 5 },
  flat_lightning_skill_damage: { kind: 'percent', perStar: 5 },
  flat_poison_skill_damage: { kind: 'percent', perStar: 5 },
  flat_arcane_skill_damage: { kind: 'percent', perStar: 5 },
  flat_skill_damage: { kind: 'percent', perStar: 5 },
  flat_elemental_skill_damage: { kind: 'percent', perStar: 5 },

  // ----- offense modifiers -----
  attack_damage: { kind: 'percent', perStar: 3 },
  enhanced_damage: { kind: 'percent', perStar: 3 },
  enhanced_defense: { kind: 'none' },
  damage_return: { kind: 'percent', perStar: 5 },

  // ----- defenses (% / flat) -----
  defense: { kind: 'none' },
  defense_vs_missiles: { kind: 'percent', perStar: 4 },
  damage_taken_reduced: { kind: 'percent', perStar: 4 },
  all_damage_taken_reduced_pct: { kind: 'percent', perStar: 4 },
  damage_recouped_as_life: { kind: 'none' },
  damage_recouped_as_mana: { kind: 'none' },
  magic_damage_reduction: { kind: 'glitch' },
  physical_damage_reduction: { kind: 'glitch' },

  // ----- enemy resistance penetration -----
  ignore_fire_res: { kind: 'percent', perStar: 3 },
  ignore_cold_res: { kind: 'percent', perStar: 3 },
  ignore_lightning_res: { kind: 'percent', perStar: 3 },
  ignore_poison_res: { kind: 'percent', perStar: 3 },
  ignore_arcane_res: { kind: 'percent', perStar: 3 },

  // ----- life / mana pools -----
  life: { kind: 'percent', perStar: 4 },
  mana: { kind: 'percent', perStar: 5 },
  increased_life: { kind: 'percent', perStar: 4 },
  increased_mana: { kind: 'percent', perStar: 4 },
  life_replenish: { kind: 'percent', perStar: 4 },
  mana_replenish: { kind: 'percent', perStar: 4 },
  life_replenish_pct: { kind: 'percent', perStar: 4 },
  mana_replenish_pct: { kind: 'percent', perStar: 4 },
  life_per_kill: { kind: 'percent', perStar: 4 },
  mana_per_kill: { kind: 'percent', perStar: 4 },
  life_steal: { kind: 'percent', perStar: 2 },
  mana_steal: { kind: 'percent', perStar: 2 },
  mana_cost_reduction: { kind: 'none' },

  // ----- speed / utility -----
  faster_cast_rate: { kind: 'percent', perStar: 3 },
  faster_hit_recovery: { kind: 'percent', perStar: 4 },
  movement_speed: { kind: 'percent', perStar: 4 },
  jumping_power: { kind: 'percent', perStar: 4 },
  light_radius: { kind: 'none' },
  experience_gain: { kind: 'none' },
  magic_find: { kind: 'percent', perStar: 3 },
  gold_find: { kind: 'percent', perStar: 3 },
  merchant_prices: { kind: 'none' },

  // ----- attack stats -----
  increased_attack_speed: { kind: 'percent', perStar: 3 },
  attacks_per_second: { kind: 'none' },
  attack_rating: { kind: 'percent', perStar: 5 },
  attack_rating_pct: { kind: 'percent', perStar: 4 },
  attack_speed_below_40_life: { kind: 'percent', perStar: 3 },

  // ----- crit / special hits -----
  deadly_blow: { kind: 'none' },
  crit_chance: { kind: 'none' },
  crit_damage: { kind: 'percent', perStar: 4 },
  crushing_blow_chance: { kind: 'none' },
  open_wounds: { kind: 'none' },

  // ----- projectiles -----
  projectile_size: { kind: 'percent', perStar: 2 },
  projectile_speed: { kind: 'none' },

  // ----- skill / aoe modifiers -----
  skill_haste: { kind: 'percent', perStar: 3 },
  area_of_effect: { kind: 'none' },
  ranged_range: { kind: 'none' },

  // ----- ailment damage (extra damage to ...) -----
  extra_damage_bleeding: { kind: 'percent', perStar: 4 },
  extra_damage_burning: { kind: 'unknown' },
  extra_damage_stunned: { kind: 'percent', perStar: 3 },
  extra_damage_poisoned: { kind: 'percent', perStar: 3 },
  extra_damage_stasis: { kind: 'percent', perStar: 3 },
  extra_damage_frozen: { kind: 'percent', perStar: 3 },
  extra_damage_frost_bitten: { kind: 'unknown' },
  extra_damage_shadow_burning: { kind: 'percent', perStar: 4 },
  extra_damage_ailments: { kind: 'percent', perStar: 3 },

  // ----- explosion -----
  explosion_damage: { kind: 'percent', perStar: 3 },
  explosion_aoe: { kind: 'none' },

  // ----- misc / status -----
  poison_length_reduced: { kind: 'none' },
}

function getStarScaleConfig(statKey: string | null): StarScaleConfig {
  if (!statKey) return { kind: 'none' }
  return STAR_SCALE_MAP[statKey] ?? { kind: 'none' }
}

export function statStarPercentMultiplier(
  statKey: string | null,
  stars: number | undefined,
): number {
  // Returns 1 + stars*perStar/100 for percent-scaled stats, or 1 for stats that
  // do not scale percentually (flat-skill staircase / none / unknown / glitch).
  if (!stars || stars <= 0) return 1
  const cfg = getStarScaleConfig(statKey)
  if (cfg.kind !== 'percent') return 1
  const perStar = cfg.perStar ?? DEFAULT_PERCENT_PER_STAR
  return 1 + (stars * perStar) / 100
}

export function statStarFlatBonus(
  statKey: string | null,
  stars: number | undefined,
): number {
  // Returns the flat staircase bonus (in raw stat units) added by stars for
  // skill-rank affixes / item-granted skill ranks, or 0 otherwise.
  if (!stars || stars <= 0) return 0
  const cfg = getStarScaleConfig(statKey)
  if (cfg.kind === 'flat-skill-staircase') {
    return FLAT_SKILL_STAIRCASE[stars] ?? 0
  }
  if (cfg.kind === 'item-specific-staircase') {
    return ITEM_SPECIFIC_STAIRCASE[stars] ?? 0
  }
  return 0
}

// `item_granted_skill_rank` is the synthetic key used by skillBonuses. Treat it
// as item-specific staircase so the rank map gets the documented (2*=+1,
// 4*=+2, 5*=+3) progression.
export function itemGrantedSkillRankFlatBonus(stars: number | undefined): number {
  if (!stars || stars <= 0) return 0
  return ITEM_SPECIFIC_STAIRCASE[stars] ?? 0
}
