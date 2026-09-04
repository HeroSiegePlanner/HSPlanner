import { describe, expect, it } from 'vitest'
import { formatEhp, groupEhpRows, type DamageType, type EhpResult } from './ehp'

const TYPES: DamageType[] = ['physical', 'fire', 'cold', 'lightning', 'poison', 'arcane']

function ehpOf(values: Partial<Record<DamageType, number | null>>, fallback = 1000): EhpResult {
  return {
    entries: TYPES.map((type) => ({
      type,
      ehp: type in values ? (values[type] ?? null) : fallback,
      multiplier: 1,
      layers: [],
    })),
    worst: 'physical',
  }
}

describe('groupEhpRows', () => {
  it('returns no rows without entries', () => {
    expect(groupEhpRows({ entries: [], worst: null })).toHaveLength(0)
  })

  it('collapses to a single eHP row when every type is equal', () => {
    expect(groupEhpRows(ehpOf({}))).toEqual([{ key: 'effective', label: 'eHP', ehp: 1000 }])
  })

  it('splits into Physical + Elemental when only physical differs', () => {
    expect(groupEhpRows(ehpOf({ physical: 2000 }))).toEqual([
      { key: 'physical', label: 'Physical eHP', ehp: 2000 },
      { key: 'elemental', label: 'Elemental eHP', ehp: 1000 },
    ])
  })

  it('lists all six types when the elements are not equal', () => {
    const rows = groupEhpRows(ehpOf({ fire: 2000 }))
    expect(rows.map((r) => r.label)).toEqual([
      'Physical eHP',
      'Fire eHP',
      'Cold eHP',
      'Lightning eHP',
      'Poison eHP',
      'Arcane eHP',
    ])
  })

  it('treats immunity as its own value', () => {
    expect(groupEhpRows(ehpOf({ physical: null }))).toHaveLength(2)
    expect(formatEhp(null)).toBe('∞')
    expect(formatEhp(1234.6)).toBe('1,235')
  })
})
