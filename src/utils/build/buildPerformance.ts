import { gameConfig } from '../../data'
import { rangedMax, rangedMin, statDef, statName } from '../item/stats'
import type { AttackSkillDamageBreakdown, SkillDamageBreakdown } from '../item/stats'
import type {
  AttributeKey,
  CustomStat,
  Inventory,
  RangedValue,
  TreeSocketContent,
} from '../../types'

export interface PerSkillDps {
  id: string
  name: string | null
  hitDpsMin: number | undefined
  hitDpsMax: number | undefined
}

export interface BuildPerformance {
  attributes: Record<AttributeKey, RangedValue>
  stats: Record<string, RangedValue>
  damage: SkillDamageBreakdown | null
  attackDamage: AttackSkillDamageBreakdown | null
  hitDpsMin: number | undefined
  hitDpsMax: number | undefined
  avgHitDpsMin: number | undefined
  avgHitDpsMax: number | undefined
  procDpsMin: number
  procDpsMax: number
  combinedDpsMin: number | undefined
  combinedDpsMax: number | undefined
  activeSkillName: string | null
  statsCombined: Record<string, RangedValue>
  itemSkillBonuses: Record<string, [number, number]>
  rankBonuses: Record<string, [number, number]>
  perSkill?: PerSkillDps[]
}

export interface BuildPerformanceDeps {
  classId: string | null
  level: number
  allocatedAttrs: Record<AttributeKey, number>
  inventory: Inventory
  skillRanks: Record<string, number>
  subskillRanks: Record<string, number>
  activeAuraId: string | null
  activeBuffs: Record<string, boolean>
  customStats: CustomStat[]
  allocatedTreeNodes: Set<number>
  treeSocketed: Record<number, TreeSocketContent | null>
  activeSkillIds: string[]
  enemyConditions: Record<string, boolean>
  playerConditions: Record<string, boolean>
  skillProjectiles: Record<string, number>
  enemyResistances: Record<string, number>
  procToggles: Record<string, boolean>
  killsPerSec: number
  season?: string
  grantedSkillRanks?: Record<string, [number, number]>
}

export interface BuildStatDiff {
  key: string
  label: string
  beforeMin: number
  beforeMax: number
  afterMin: number
  afterMax: number
  delta: number
  isPercent: boolean
  kind: 'up' | 'down'
}

export function applyDisabledPotions(
  inventory: Inventory,
  disabledPotions: Record<string, boolean>,
): Inventory {
  const off = Object.keys(disabledPotions).filter((k) => disabledPotions[k])
  if (off.length === 0) return inventory
  const next = { ...inventory }
  for (const slot of off) delete next[slot as keyof Inventory]
  return next
}

export function rangedBounds(v: RangedValue | undefined): {
  min: number
  max: number
} {
  if (v === undefined) return { min: 0, max: 0 }
  return { min: rangedMin(v), max: rangedMax(v) }
}

function diffRangePair(
  key: string,
  label: string,
  beforeMin: number,
  beforeMax: number,
  afterMin: number,
  afterMax: number,
  isPercent: boolean,
): BuildStatDiff | null {
  const beforeAvg = (beforeMin + beforeMax) / 2
  const afterAvg = (afterMin + afterMax) / 2
  const delta = afterAvg - beforeAvg
  if (Math.abs(delta) < 0.001) return null
  return {
    key,
    label,
    beforeMin,
    beforeMax,
    afterMin,
    afterMax,
    delta,
    isPercent,
    kind: delta > 0 ? 'up' : 'down',
  }
}

export function diffPerformanceStats(
  before: BuildPerformance,
  after: BuildPerformance,
): BuildStatDiff[] {
  const rows: BuildStatDiff[] = []
  const attrLabel = (key: string): string =>
    gameConfig.attributes.find((a) => a.key === key)?.name ?? statName(key)

  const allAttrKeys = new Set<string>([
    ...Object.keys(before.attributes),
    ...Object.keys(after.attributes),
  ])
  for (const key of allAttrKeys) {
    const b = rangedBounds(before.attributes[key])
    const a = rangedBounds(after.attributes[key])
    const diff = diffRangePair(
      key,
      attrLabel(key),
      b.min,
      b.max,
      a.min,
      a.max,
      false,
    )
    if (diff) rows.push(diff)
  }

  const allStatKeys = new Set<string>([
    ...Object.keys(before.stats),
    ...Object.keys(after.stats),
  ])
  for (const key of allStatKeys) {
    const b = rangedBounds(before.stats[key])
    const a = rangedBounds(after.stats[key])
    const diff = diffRangePair(
      key,
      statName(key),
      b.min,
      b.max,
      a.min,
      a.max,
      statDef(key)?.format === 'percent',
    )
    if (diff) rows.push(diff)
  }

  return rows
}

export function diffPerformanceDps(
  before: BuildPerformance,
  after: BuildPerformance,
): BuildStatDiff[] {
  const rows: BuildStatDiff[] = []
  const hit = diffRangePair(
    'hit_dps',
    'Hit DPS',
    before.hitDpsMin ?? 0,
    before.hitDpsMax ?? 0,
    after.hitDpsMin ?? 0,
    after.hitDpsMax ?? 0,
    false,
  )
  if (hit) rows.push(hit)

  const combined = diffRangePair(
    'combined_dps',
    'Combined DPS',
    before.combinedDpsMin ?? 0,
    before.combinedDpsMax ?? 0,
    after.combinedDpsMin ?? 0,
    after.combinedDpsMax ?? 0,
    false,
  )
  if (combined) rows.push(combined)

  const avgHit = diffRangePair(
    'avg_hit',
    'Average Hit',
    before.damage !== null ? before.damage.avgMin : 0,
    before.damage !== null ? before.damage.avgMax : 0,
    after.damage !== null ? after.damage.avgMin : 0,
    after.damage !== null ? after.damage.avgMax : 0,
    false,
  )
  if (avgHit) rows.push(avgHit)

  return rows
}
