import { gameConfig, getItem, isGearSlot } from '../../data'
import type { ForgeKind } from '../../data'
import {
  isStatStarImmune,
  itemGrantedSkillRankFlatBonus,
  statStarFlatBonus,
  statStarPercentMultiplier,
} from './starScaling'
import type {
  AttributeKey,
  Inventory,
  RangedStatMap,
  RangedValue,
  Skill,
  StatDef,
} from '../../types'

export type SourceType =
  | 'class'
  | 'allocated'
  | 'level'
  | 'attribute'
  | 'item'
  | 'socket'
  | 'skill'
  | 'subskill'
  | 'custom'
  | 'tree'

export function normalizeSkillName(name: string): string {
  return name.trim().toLowerCase()
}

const STAT_DEFS_MAP = new Map<string, StatDef>(
  gameConfig.stats.map((s) => [s.key, s]),
)

export interface SourceContribution {
  label: string
  sourceType: SourceType
  value: RangedValue
  forge?: {
    itemName: string
    modName: string
    kind: ForgeKind
  }
}

export interface ComputedStats {
  attributes: Record<AttributeKey, RangedValue>
  stats: RangedStatMap
  attributeSources: Record<AttributeKey, SourceContribution[]>
  statSources: Record<string, SourceContribution[]>
}

export function combineAdditiveAndMore(
  additive: RangedValue | undefined,
  more: RangedValue | undefined,
): RangedValue {
  const [aMin, aMax] =
    additive === undefined
      ? [0, 0]
      : typeof additive === 'number'
        ? [additive, additive]
        : additive
  const [mMin, mMax] =
    more === undefined
      ? [0, 0]
      : typeof more === 'number'
        ? [more, more]
        : more
  const round = (n: number) => Math.round(n * 1e6) / 1e6
  const min = round(((1 + aMin / 100) * (1 + mMin / 100) - 1) * 100)
  const max = round(((1 + aMax / 100) * (1 + mMax / 100) - 1) * 100)
  return min === max ? min : [min, max]
}

export function statName(key: string): string {
  const direct = STAT_DEFS_MAP.get(key)?.name
  if (direct) return direct
  if (key.endsWith('_more')) {
    const base = key.slice(0, -'_more'.length)
    const baseName = STAT_DEFS_MAP.get(base)?.name
    if (baseName) return `Total ${baseName}`
  }
  return key
}

export function statDef(key: string): StatDef | undefined {
  const direct = STAT_DEFS_MAP.get(key)
  if (direct) return direct
  if (key.endsWith('_more')) {
    const base = STAT_DEFS_MAP.get(key.slice(0, -'_more'.length))
    if (base) {
      return {
        ...base,
        key,
        name: `Total ${base.name}`,
        format: 'percent',
      }
    }
  }
  return undefined
}

export function effectiveCap(
  statKey: string,
  stats: RangedStatMap,
): number | undefined {
  const baseCap = statDef(statKey)?.cap
  if (baseCap === undefined) return undefined
  const mod = stats[`max_${statKey}`]
  if (mod === undefined) return baseCap
  return baseCap + rangedMax(mod)
}

export function isZero(v: RangedValue): boolean {
  if (typeof v === 'number') return v === 0
  return v[0] === 0 && v[1] === 0
}

export function rangedMin(v: RangedValue): number {
  return typeof v === 'number' ? v : v[0]
}

export function rangedMax(v: RangedValue): number {
  return typeof v === 'number' ? v : v[1]
}

export function formatValue(value: RangedValue, key: string): string {
  const def = statDef(key)
  const suffix = def?.format === 'percent' ? '%' : ''
  if (typeof value === 'number') {
    const sign = value >= 0 ? '+' : ''
    const num = Number.isInteger(value)
      ? value
      : Math.round(value * 100) / 100
    return `${sign}${num}${suffix}`
  }
  const [min, max] = value
  if (min === max) return formatValue(min, key)
  const sign = min >= 0 ? '+' : ''
  const minF = Number.isInteger(min) ? min : Math.round(min * 100) / 100
  const maxF = Number.isInteger(max) ? max : Math.round(max * 100) / 100
  return `${sign}[${minF}-${maxF}]${suffix}`
}

export function fmtStats(stats: RangedStatMap): string {
  return Object.entries(stats)
    .map(([k, v]) => `${formatValue(v, k)} ${statName(k)}`)
    .join(', ')
}

export interface SkillDamageBreakdown {
  effectiveRankMin: number
  effectiveRankMax: number
  baseMin: number
  baseMax: number
  flatMin: number
  flatMax: number
  synergyMinPct: number
  synergyMaxPct: number
  skillDamageMinPct: number
  skillDamageMaxPct: number
  extraDamagePct: number
  extraDamageSources: Array<{ label: string; pct: number }>
  critChance: number
  critDamagePct: number
  critMultiplierAvg: number
  multicastChancePct: number
  multicastMultiplier: number
  projectileCount: number
  elementalBreakPct: number
  elementalBreakMultiplier: number
  enemyResistancePct: number
  resistanceIgnoredPct: number
  effectiveResistancePct: number
  resistanceMultiplier: number
  hitMin: number
  hitMax: number
  critMin: number
  critMax: number
  finalMin: number
  finalMax: number
  avgMin: number
  avgMax: number
}

export interface AttackSkillDamageBreakdown {
  effectiveRankMin: number
  effectiveRankMax: number
  weaponDamagePctMin: number
  weaponDamagePctMax: number
  skillFlatPhysMin: number
  skillFlatPhysMax: number
  attackRatingPctMin: number
  attackRatingPctMax: number
  physicalHitMin: number
  physicalHitMax: number
  physicalAvgMin: number
  physicalAvgMax: number
  poisonHitMin: number
  poisonHitMax: number
  poisonAvgMin: number
  poisonAvgMax: number
  combinedHitMin: number
  combinedHitMax: number
  combinedAvgMin: number
  combinedAvgMax: number
  attacksPerSecondMin: number
  attacksPerSecondMax: number
  dpsMin: number
  dpsMax: number
}

export interface WeaponDamageBreakdown {
  hasWeapon: boolean
  weaponName?: string
  weaponDamageMin: number
  weaponDamageMax: number
  enhancedDamageMinPct: number
  enhancedDamageMaxPct: number
  additivePhysicalMin: number
  additivePhysicalMax: number
  additiveElementalMin: number
  additiveElementalMax: number
  additiveElementalBreakdown: Array<{ label: string; pct: number }>
  attackDamageMinPct: number
  attackDamageMaxPct: number
  extraDamagePct: number
  extraDamageSources: Array<{ label: string; pct: number }>
  crushingBlowModifier: number
  armorBreakPct: number
  deadlyBlowChance: number
  hitChance: number
  critChance: number
  critDamagePct: number
  critMultiplierAvg: number
  attacksPerSecondMin: number
  attacksPerSecondMax: number
  projectileCount: number
  enemyPhysResPct: number
  physResistanceIgnoredPct: number
  hitMin: number
  hitMax: number
  critMin: number
  critMax: number
  avgMin: number
  avgMax: number
  openWoundsMin: number
  openWoundsMax: number
  dpsMin: number
  dpsMax: number
}

export function rolledAffixValue(
  affix: {
    sign: '+' | '-'
    format: 'flat' | 'percent'
    valueMin: number | null
    valueMax: number | null
  },
  roll: number,
): number {
  if (affix.valueMin === null || affix.valueMax === null) return 0
  const raw =
    affix.valueMin === affix.valueMax
      ? affix.valueMax
      : affix.valueMin + (affix.valueMax - affix.valueMin) * roll
  const rounded = affix.format === 'flat' ? Math.round(raw) : raw
  return affix.sign === '-' ? -rounded : rounded
}

function affixStarMultiplier(
  statKey: string | null,
  stars: number | undefined,
): number {
  return statStarPercentMultiplier(statKey, stars)
}

export function rolledAffixRange(affix: {
  sign: '+' | '-'
  format: 'flat' | 'percent'
  valueMin: number | null
  valueMax: number | null
}): RangedValue {
  if (affix.valueMin === null || affix.valueMax === null) return 0
  const round = (n: number) => (affix.format === 'flat' ? Math.round(n) : n)
  const min = round(affix.valueMin)
  const max = round(affix.valueMax)
  if (affix.sign === '-') {
    return min === max ? -max : [-max, -min]
  }
  return min === max ? min : [min, max]
}

export function isAffixStarImmune(statKey: string | null): boolean {
  return isStatStarImmune(statKey)
}

export function rolledAffixValueWithStars(
  affix: {
    sign: '+' | '-'
    format: 'flat' | 'percent'
    valueMin: number | null
    valueMax: number | null
    statKey: string | null
  },
  roll: number,
  stars: number | undefined,
): number {
  const base = rolledAffixValue(affix, roll)
  const mult = affixStarMultiplier(affix.statKey, stars)
  const flat = statStarFlatBonus(affix.statKey, stars)
  if (base === 0 && flat === 0) return 0
  const direction = affix.sign === '-' ? -1 : 1
  const scaled = base * mult + flat * direction
  const starsActive = (stars ?? 0) > 0 && (mult !== 1 || flat !== 0)
  if (starsActive) return Math.floor(scaled)
  return affix.format === 'flat' ? Math.round(scaled) : scaled
}

export function formatAffixRange(
  affix: {
    sign: '+' | '-'
    format: 'flat' | 'percent'
    valueMin: number | null
    valueMax: number | null
    statKey: string | null
  },
  stars?: number,
): string {
  if (affix.valueMin === null || affix.valueMax === null) return affix.sign
  const minSigned = rolledAffixValueWithStars(affix, 0, stars)
  const maxSigned = rolledAffixValueWithStars(affix, 1, stars)
  const fmtAbs = (v: number) => {
    const abs = Math.abs(v)
    return Number.isInteger(abs) ? abs : Math.round(abs * 100) / 100
  }
  const lo = fmtAbs(minSigned)
  const hi = fmtAbs(maxSigned)
  const sign =
    affix.sign === '-' || minSigned < 0 || maxSigned < 0 ? '-' : '+'
  const suffix = affix.format === 'percent' ? '%' : ''
  if (lo === hi) return `${sign}${hi}${suffix}`
  return `${sign}[${lo}-${hi}]${suffix}`
}

export function applyStarsToRangedValue(
  value: RangedValue,
  statKey: string,
  stars: number | undefined,
): RangedValue {
  if (!stars || stars <= 0) return value
  const flat =
    statKey === 'item_granted_skill_rank'
      ? itemGrantedSkillRankFlatBonus(stars)
      : statStarFlatBonus(statKey, stars)
  const mult = statStarPercentMultiplier(statKey, stars)
  if (mult === 1 && flat === 0) return value
  if (typeof value === 'number') {
    return Math.floor(value * mult + flat)
  }
  const [min, max] = value
  return [Math.floor(min * mult + flat), Math.floor(max * mult + flat)]
}

export function shouldScaleImplicit(isRuneword: boolean): boolean {
  return !isRuneword
}

export function aggregateItemSkillBonuses(
  inventory: Inventory,
): Record<string, [number, number]> {
  const out: Record<string, [number, number]> = {}
  for (const [slotKey, item] of Object.entries(inventory)) {
    if (!item) continue
    const base = getItem(item.baseId)
    if (!base?.skillBonuses) continue
    const stars = isGearSlot(slotKey) ? item.stars : undefined
    for (const [skillName, val] of Object.entries(base.skillBonuses)) {
      const scaled = applyStarsToRangedValue(val, 'item_granted_skill_rank', stars)
      const min = Math.round(rangedMin(scaled))
      const max = Math.round(rangedMax(scaled))
      const key = normalizeSkillName(skillName)
      const cur = out[key] ?? [0, 0]
      out[key] = [cur[0] + min, cur[1] + max]
    }
  }
  return out
}

export function effectiveRankRangeFor(
  skill: Skill,
  baseRank: number,
  stats: RangedStatMap,
  itemSkillBonuses: Record<string, [number, number]>,
): [number, number] {
  if (baseRank <= 0) return [0, 0]
  const all = stats.all_skills ?? 0
  const elem = skill.damageType
    ? (stats[`${skill.damageType}_skills`] ?? 0)
    : 0
  const item = itemSkillBonuses[normalizeSkillName(skill.name)] ?? [0, 0]
  return [
    baseRank + rangedMin(all) + rangedMin(elem) + item[0],
    baseRank + rangedMax(all) + rangedMax(elem) + item[1],
  ]
}
