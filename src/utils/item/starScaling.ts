import starScalingData from '../../data/star-scaling.json'

// Per-star = % multiplier on base, or flat staircase for skill-rank affixes.
// Single source of truth shared with Rust calc/star_scaling.rs via
// src/data/star-scaling.json. Source: listamodow-2.txt.

export type StarScaleConfig =
  | { kind: 'percent'; perStar: number }
  | { kind: 'flat-skill-staircase' }
  | { kind: 'item-specific-staircase' }
  | { kind: 'none' }
  | { kind: 'unknown' }
  | { kind: 'glitch' }

interface StarScalingData {
  flatSkillStaircase: number[]
  itemSpecificStaircase: number[]
  map: Record<string, StarScaleConfig>
}

// Controlled in-repo data file; the discriminated-union kinds are validated by tests.
const DATA = starScalingData as unknown as StarScalingData

const DEFAULT_PERCENT_PER_STAR = 0

// listamodow-2.txt: "ITEM SPECIFIC 2* = +1 | 4* = +2 | 5* = +3".
export const ITEM_SPECIFIC_STAIRCASE: Record<number, number> =
  DATA.itemSpecificStaircase

// listamodow-2.txt for fire_skills/cold_skills/etc: "na trzech * +1 na pieciu * +2".
export const FLAT_SKILL_STAIRCASE: Record<number, number> =
  DATA.flatSkillStaircase

// Unlisted keys default to "none" — new affixes are explicit, not silently inheriting the old global 8% rule.
const STAR_SCALE_MAP: Readonly<Record<string, StarScaleConfig>> = DATA.map

export function getStarScaleConfig(statKey: string | null): StarScaleConfig {
  if (!statKey) return { kind: 'none' }
  return STAR_SCALE_MAP[statKey] ?? { kind: 'none' }
}

export function isStatStarImmune(statKey: string | null): boolean {
  const cfg = getStarScaleConfig(statKey)
  return cfg.kind === 'none' || cfg.kind === 'unknown' || cfg.kind === 'glitch'
}

export function statStarPercentMultiplier(
  statKey: string | null,
  stars: number | undefined,
): number {
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

// Synthetic key used by skillBonuses; follows item-specific staircase (2*=+1, 4*=+2, 5*=+3).
export function itemGrantedSkillRankFlatBonus(stars: number | undefined): number {
  if (!stars || stars <= 0) return 0
  return ITEM_SPECIFIC_STAIRCASE[stars] ?? 0
}
