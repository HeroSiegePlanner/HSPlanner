import { gameConfig } from '../../data'
import type { ForgeKind } from '../../data'
import type {
  AttributeKey,
  RangedStatMap,
  RangedValue,
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
  statsCombined: Record<string, RangedValue>
  itemSkillBonuses: Record<string, [number, number]>
  rankBonuses: Record<string, [number, number]>
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

export function formatAffixRangeFromValues(
  affix: {
    sign: '+' | '-'
    format: 'flat' | 'percent'
    valueMin: number | null
    valueMax: number | null
  },
  range: { rangeMin: number; rangeMax: number } | null,
): string {
  if (!range || affix.valueMin === null || affix.valueMax === null) {
    return affix.sign
  }
  const fmtAbs = (v: number) => {
    const abs = Math.abs(v)
    return Number.isInteger(abs) ? abs : Math.round(abs * 100) / 100
  }
  const lo = fmtAbs(range.rangeMin)
  const hi = fmtAbs(range.rangeMax)
  const sign =
    affix.sign === '-' || range.rangeMin < 0 || range.rangeMax < 0 ? '-' : '+'
  const suffix = affix.format === 'percent' ? '%' : ''
  if (lo === hi) return `${sign}${hi}${suffix}`
  return `${sign}[${lo}-${hi}]${suffix}`
}

export function shouldScaleImplicit(isRuneword: boolean): boolean {
  return !isRuneword
}
