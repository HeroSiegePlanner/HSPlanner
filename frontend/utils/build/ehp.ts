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

// `ehp: null` = immune (a mitigation layer reached 100%).
export interface EhpEntry {
  type: DamageType
  ehp: number | null
  multiplier: number
  layers: EhpLayer[]
}

export interface EhpResult {
  entries: EhpEntry[]
  worst: DamageType | null
}

export interface DefenseInsight {
  text: string
  gainPct: number | null
}

export interface EhpRow {
  key: string
  label: string
  ehp: number | null
}

export const EMPTY_EHP: EhpResult = { entries: [], worst: null }

const ELEMENTS: Exclude<DamageType, 'physical'>[] = [
  'fire',
  'cold',
  'lightning',
  'poison',
  'arcane',
]

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1)
}

export function formatEhp(n: number | null): string {
  return n === null ? '∞' : Math.round(n).toLocaleString('en-US')
}

function sameEhp(a: number | null, b: number | null): boolean {
  if (a === null || b === null) return a === b
  return Math.round(a) === Math.round(b)
}

export function groupEhpRows({ entries }: EhpResult): EhpRow[] {
  if (entries.length === 0) return []

  const ehpOf = new Map(entries.map((e) => [e.type, e.ehp]))
  const physical = ehpOf.get('physical') ?? 0
  const elemEhps = ELEMENTS.map((t) => ehpOf.get(t) ?? 0)
  const elemEhp = elemEhps[0] ?? 0
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
