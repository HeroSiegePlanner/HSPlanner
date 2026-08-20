import { describe, expect, it } from 'vitest'
import { applyDisabledPotions, diffPerformanceDps } from './buildPerformance'
import type { BuildPerformance } from './buildPerformance'
import type { EquippedItem, Inventory } from '../../types'

function perf(hitDps: number, combinedDps: number, avgHit: number): BuildPerformance {
  return {
    hitDpsMin: hitDps,
    hitDpsMax: hitDps,
    combinedDpsMin: combinedDps,
    combinedDpsMax: combinedDps,
    damage: { avgMin: avgHit, avgMax: avgHit },
  } as BuildPerformance
}

function potion(baseId: string): EquippedItem {
  return { baseId, affixes: [], socketCount: 0, socketed: [], socketTypes: [] }
}

describe('applyDisabledPotions', () => {
  it('returns the same inventory reference when nothing is disabled', () => {
    const inv: Inventory = { potion_1: potion('p1'), weapon: potion('w') }
    expect(applyDisabledPotions(inv, {})).toBe(inv)
  })

  it('removes only the disabled slot, leaving others intact', () => {
    const inv: Inventory = { potion_1: potion('p1'), potion_2: potion('p2') }
    const out = applyDisabledPotions(inv, { potion_1: true })
    expect(out.potion_1).toBeUndefined()
    expect(out.potion_2).toBeDefined()
  })

  it('does not mutate the input inventory', () => {
    const inv: Inventory = { potion_1: potion('p1') }
    applyDisabledPotions(inv, { potion_1: true })
    expect(inv.potion_1).toBeDefined()
  })

  it('ignores slots whose flag is false', () => {
    const inv: Inventory = { potion_1: potion('p1') }
    const out = applyDisabledPotions(inv, { potion_1: false })
    expect(out.potion_1).toBeDefined()
  })
})

describe('diffPerformanceDps', () => {
  it('reports every damage row as a percentage change', () => {
    const rows = diffPerformanceDps(perf(100, 200, 50), perf(125, 150, 55))
    expect(rows.map((r) => [r.key, r.deltaPct])).toEqual([
      ['hit_dps', 25],
      ['combined_dps', -25],
      ['avg_hit', 10],
    ])
  })

  it('leaves the delta absolute when there is nothing to compare against', () => {
    const rows = diffPerformanceDps(perf(0, 0, 0), perf(500, 0, 0))
    expect(rows).toHaveLength(1)
    expect(rows[0].deltaPct).toBeUndefined()
    expect(rows[0].delta).toBe(500)
  })
})
