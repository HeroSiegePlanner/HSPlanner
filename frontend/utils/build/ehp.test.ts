import { describe, expect, it } from 'vitest'
import { computeEhp, deriveDefenseInsights, groupEhpRows } from './ehp'

describe('computeEhp', () => {
  it('returns empty result when life is missing or zero', () => {
    expect(computeEhp({}).worst).toBeNull()
    expect(computeEhp({ life: 0 }).entries).toHaveLength(0)
  })

  it('computes per-type EHP and picks the worst type', () => {
    const result = computeEhp({ life: 1000, cold_resistance: 50 })
    const cold = result.entries.find((e) => e.type === 'cold')
    const physical = result.entries.find((e) => e.type === 'physical')
    expect(result.entries).toHaveLength(6)
    expect(cold?.ehp).toBeCloseTo(2000)
    expect(physical?.ehp).toBeCloseTo(1000)
    expect(result.worst?.ehp).toBeCloseTo(1000)
    expect(result.worst?.type).not.toBe('cold')
  })

  it('multiplies stacked layers', () => {
    const result = computeEhp({
      life: 1000,
      physical_damage_reduction: 30,
      damage_taken_reduced: 10,
      all_damage_taken_reduced_pct: 20,
    })
    const physical = result.entries.find((e) => e.type === 'physical')
    expect(physical?.ehp).toBeCloseTo(1000 / 0.504)
    expect(physical?.layers).toHaveLength(3)
  })

  it('applies resistance caps before computing', () => {
    const result = computeEhp({ life: 1000, fire_resistance: 90 })
    const fire = result.entries.find((e) => e.type === 'fire')
    expect(fire?.ehp).toBeCloseTo(4000)
    expect(fire?.layers.find((l) => l.label.includes('resistance'))?.pct).toBe(75)
  })

  it('negative resistance lowers EHP below life', () => {
    const result = computeEhp({ life: 1000, poison_resistance: -25 })
    const poison = result.entries.find((e) => e.type === 'poison')
    expect(poison?.ehp).toBeCloseTo(800)
  })

  it('returns Infinity when a layer reaches full immunity', () => {
    const result = computeEhp({ life: 1000, all_damage_taken_reduced_pct: 100 })
    expect(result.entries.every((e) => e.ehp === Infinity)).toBe(true)
  })

  it('elemental types include magic reduction layers, physical does not', () => {
    const result = computeEhp({ life: 1000, magic_damage_reduction: 20 })
    const fire = result.entries.find((e) => e.type === 'fire')
    const physical = result.entries.find((e) => e.type === 'physical')
    expect(fire?.ehp).toBeCloseTo(1250)
    expect(physical?.ehp).toBeCloseTo(1000)
  })
})

describe('deriveDefenseInsights', () => {
  it('returns empty when life is missing', () => {
    expect(deriveDefenseInsights({ cold_resistance: 10 })).toHaveLength(0)
  })

  it('suggests capping an uncapped resistance with computed gain', () => {
    const insights = deriveDefenseInsights({
      life: 1000,
      fire_resistance: 75,
      lightning_resistance: 75,
      poison_resistance: 75,
      arcane_resistance: 75,
      cold_resistance: 12,
    })
    expect(insights).toHaveLength(1)
    const cold = insights[0]
    expect(cold?.gainPct).toBeCloseTo(252)
    expect(cold?.text).toContain('12→75')
    expect(cold?.text).toContain('+252% EHP')
  })

  it('skips capped resistances and gains at or below the threshold', () => {
    const insights = deriveDefenseInsights({
      life: 1000,
      fire_resistance: 75,
      lightning_resistance: 75,
      poison_resistance: 75,
      arcane_resistance: 75,
      cold_resistance: 74.6,
    })
    expect(insights).toHaveLength(0)
  })

  it('treats absent resistance keys as 0% holes', () => {
    const insights = deriveDefenseInsights({ life: 1000 })
    expect(insights).toHaveLength(3)
    expect(insights[0]?.gainPct).toBeCloseTo(300)
  })

  it('caps at three insights sorted by gain descending', () => {
    const insights = deriveDefenseInsights({
      life: 1000,
      fire_resistance: 0,
      cold_resistance: 30,
      lightning_resistance: 50,
      poison_resistance: 60,
      arcane_resistance: 70,
    })
    expect(insights).toHaveLength(3)
    expect(insights[0]?.text).toContain('fire')
    expect(insights[0]?.gainPct ?? 0).toBeGreaterThan(insights[2]?.gainPct ?? 0)
  })

  it('sorts without NaN when two elements both reach Infinity gain', () => {
    const insights = deriveDefenseInsights({
      life: 1000,
      fire_resistance: 0,
      max_fire_resistance: 25,
      cold_resistance: 0,
      max_cold_resistance: 25,
      lightning_resistance: 75,
      poison_resistance: 75,
      arcane_resistance: 75,
    })
    expect(insights).toHaveLength(2)
    expect(insights.every((i) => i.gainPct === Infinity)).toBe(true)
    expect(insights.every((i) => Number.isNaN(i.gainPct))).toBe(false)
    expect(insights.every((i) => i.text.includes('immunity'))).toBe(true)
    expect(insights[0]?.text).toContain('fire')
    expect(insights[1]?.text).toContain('cold')
  })
})

describe('groupEhpRows', () => {
  it('returns no rows when life is missing', () => {
    expect(groupEhpRows({})).toHaveLength(0)
  })

  it('collapses to a single eHP row when every type is equal', () => {
    const rows = groupEhpRows({ life: 1000 })
    expect(rows).toEqual([{ key: 'effective', label: 'eHP', ehp: 1000 }])
  })

  it('splits into Physical + Elemental when only physical differs', () => {
    const rows = groupEhpRows({ life: 1000, physical_damage_reduction: 50 })
    expect(rows).toEqual([
      { key: 'physical', label: 'Physical eHP', ehp: 2000 },
      { key: 'elemental', label: 'Elemental eHP', ehp: 1000 },
    ])
  })

  it('lists all six types when the elements are not equal', () => {
    const rows = groupEhpRows({ life: 1000, fire_resistance: 50 })
    expect(rows.map((r) => r.label)).toEqual([
      'Physical eHP',
      'Fire eHP',
      'Cold eHP',
      'Lightning eHP',
      'Poison eHP',
      'Arcane eHP',
    ])
    expect(rows.find((r) => r.key === 'fire')?.ehp).toBeCloseTo(2000)
  })
})
