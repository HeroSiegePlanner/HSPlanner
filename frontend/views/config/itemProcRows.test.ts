import { describe, expect, it } from 'vitest'
import { ITEM_PROC_ICD_SECS, itemProcRows } from './itemProcRows'
import type { EquippedItem } from '../../types'

describe('itemProcRows', () => {
  it('returns a toggle row for an equipped item granting a proc skill', () => {
    // relics.json: The Eye grants "The Eye" [1,85]; its granted-skill def
    // carries procDamage (arcane 19.5/rank).
    const inventory: Record<string, EquippedItem | undefined> = {
      relic_1: { baseId: 'relic_relic_the_eye', affixes: [] },
    }
    const rows = itemProcRows(inventory)
    expect(rows).toHaveLength(1)
    const row = rows[0]!
    expect(row.id).toBe('the_eye')
    expect(row.name).toBe('The Eye')
    expect(row.toggleKey).toBe('granted:the_eye')
    expect(row.rankMin).toBe(1)
    expect(row.rankMax).toBe(85)
    expect(row.types).toEqual(['arcane'])
    expect(row.intervalSecs).toBe(ITEM_PROC_ICD_SECS)
  })

  it('returns nothing for an empty inventory', () => {
    expect(itemProcRows({})).toEqual([])
  })

  it('ignores granted skills without procDamage', () => {
    // Doom Flute grants Holy Aura — a passive aura without procDamage.
    const inventory: Record<string, EquippedItem | undefined> = {
      relic_1: { baseId: 'relic_relic_doom_flute', affixes: [] },
    }
    expect(itemProcRows(inventory)).toEqual([])
  })
})
