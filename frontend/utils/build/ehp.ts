import { effectiveCap, rangedMax, type RangedStatMap } from '../item/stats'

export type DamageType =
  | 'physical'
  | 'fire'
  | 'cold'
  | 'lightning'
  | 'poison'
  | 'arcane'

export interface EhpLayer {
  label: string
  pct: number
}

export interface EhpEntry {
  type: DamageType
  ehp: number
  multiplier: number
  layers: EhpLayer[]
}

export interface EhpResult {
  entries: EhpEntry[]
  worst: EhpEntry | null
}

export interface EhpRow {
  key: string
  label: string
  ehp: number
}

export interface DefenseInsight {
  text: string
  gainPct: number
}

const DAMAGE_TYPES: DamageType[] = [
  'physical',
  'fire',
  'cold',
  'lightning',
  'poison',
  'arcane',
]
const ELEMENTS: Exclude<DamageType, 'physical'>[] = [
  'fire',
  'cold',
  'lightning',
  'poison',
  'arcane',
]

export const DEFAULT_RES_CAP = 75
const INSIGHT_MIN_GAIN_PCT = 2
const INSIGHT_MAX_COUNT = 3

function statPct(stats: RangedStatMap, key: string): number {
  const v = stats[key]
  return v === undefined ? 0 : rangedMax(v)
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1)
}

function multiplierFor(
  type: DamageType,
  stats: RangedStatMap,
  resOverride?: number,
): { multiplier: number; layers: EhpLayer[] } {
  const layers: EhpLayer[] = []
  let multiplier = 1
  const apply = (label: string, pct: number, alwaysShow = false) => {
    if (pct !== 0 || alwaysShow) layers.push({ label, pct })
    multiplier *= 1 - pct / 100
  }

  if (type === 'physical') {
    apply(
      'Physical damage reduction',
      statPct(stats, 'physical_damage_reduction'),
    )
  } else {
    const key = `${type}_resistance`
    const cap = effectiveCap(key, stats) ?? DEFAULT_RES_CAP
    const raw = resOverride ?? statPct(stats, key)
    apply(`${capitalize(type)} resistance`, Math.min(raw, cap), true)
    apply('Magic damage reduction', statPct(stats, 'magic_damage_reduction'))
    apply(
      'Magic damage taken reduced',
      statPct(stats, 'magic_damage_taken_reduced'),
    )
  }
  apply('Damage taken reduced', statPct(stats, 'damage_taken_reduced'))
  apply(
    'All damage taken reduced',
    statPct(stats, 'all_damage_taken_reduced_pct'),
  )
  return { multiplier, layers }
}

export function computeEhp(stats: RangedStatMap): EhpResult {
  const life = statPct(stats, 'life')
  if (life <= 0) return { entries: [], worst: null }

  const entries: EhpEntry[] = DAMAGE_TYPES.map((type) => {
    const { multiplier, layers } = multiplierFor(type, stats)
    const ehp = multiplier <= 0 ? Infinity : life / multiplier
    return { type, ehp, multiplier, layers }
  })

  let worst: EhpEntry | null = null
  for (const entry of entries) {
    if (worst === null || entry.ehp < worst.ehp) worst = entry
  }
  return { entries, worst }
}

export function deriveDefenseInsights(stats: RangedStatMap): DefenseInsight[] {
  const life = statPct(stats, 'life')
  if (life <= 0) return []

  const insights: DefenseInsight[] = []
  for (const element of ELEMENTS) {
    const key = `${element}_resistance`
    const cap = effectiveCap(key, stats) ?? DEFAULT_RES_CAP
    const raw = statPct(stats, key)
    if (raw >= cap) continue
    const now = multiplierFor(element, stats)
    if (now.multiplier <= 0) continue
    const capped = multiplierFor(element, stats, cap)
    const gainPct =
      capped.multiplier <= 0
        ? Infinity
        : (now.multiplier / capped.multiplier - 1) * 100
    if (gainPct <= INSIGHT_MIN_GAIN_PCT) continue
    const gainLabel = Number.isFinite(gainPct)
      ? `+${Math.round(gainPct)}% EHP`
      : 'immunity'
    insights.push({
      text: `Cap ${element} res (${Math.round(raw)}→${cap}): ${gainLabel} vs ${element}`,
      gainPct,
    })
  }
  return insights
    .toSorted((a, b) => (a.gainPct === b.gainPct ? 0 : b.gainPct - a.gainPct))
    .slice(0, INSIGHT_MAX_COUNT)
}

export function formatEhp(n: number): string {
  return n === Infinity ? '∞' : Math.round(n).toLocaleString('en-US')
}

export function groupEhpRows(stats: RangedStatMap): EhpRow[] {
  const { entries } = computeEhp(stats)
  if (entries.length === 0) return []

  const ehpOf = new Map(entries.map((e) => [e.type, e.ehp]))
  const physical = ehpOf.get('physical') ?? 0
  const elemEhps = ELEMENTS.map((t) => ehpOf.get(t) ?? 0)
  const elemEhp = elemEhps[0] ?? 0
  const sameEhp = (a: number, b: number) => Math.round(a) === Math.round(b)
  const elementsEqual = elemEhps.every((v) => sameEhp(v, elemEhp))

  if (elementsEqual && sameEhp(physical, elemEhp)) {
    return [{ key: 'effective', label: 'eHP', ehp: physical }]
  }
  if (elementsEqual) {
    return [
      { key: 'physical', label: 'Physical eHP', ehp: physical },
      { key: 'elemental', label: 'Elemental eHP', ehp: elemEhp },
    ]
  }
  return entries.map((e) => ({
    key: e.type,
    label: `${capitalize(e.type)} eHP`,
    ehp: e.ehp,
  }))
}
