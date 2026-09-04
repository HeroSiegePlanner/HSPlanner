import { describe, expect, test } from 'vitest'
import { affixPools, getAffix } from '@data'
import { affixFitsPool } from './affixPools'
import {
  JEWEL_AFFIX_POOL,
  JEWEL_AFFIX_POOL_BY_GROUP,
  isJewelAffix,
} from './jewelAffixes'

describe('uncut jewel affix pool', () => {
  test('rolls only Socketable families, every one with a stat the engine knows', () => {
    expect(JEWEL_AFFIX_POOL.length).toBeGreaterThan(0)
    for (const a of JEWEL_AFFIX_POOL) {
      expect(affixPools[a.groupId], a.id).toEqual(['Socketable'])
      expect(a.statKey, a.id).toBeTruthy()
    }
  })

  test('uses the jewel ranges from the game, not the gear ranges', () => {
    const tiers = JEWEL_AFFIX_POOL_BY_GROUP.get('jewel_enhanced_damage') ?? []
    expect(tiers.map((t) => [t.tier, t.valueMin, t.valueMax])).toEqual([
      [1, 4, 15],
      [2, 6, 20],
      [3, 8, 25],
      [4, 10, 30],
      [5, 12, 35],
    ])
    expect(getAffix('25_50_enhanced_damage_t1_soldier_s')?.valueMax).toBe(50)
  })

  test('jewel families stay out of every gear pool', () => {
    const jewel = getAffix('jewel_enhanced_damage_t1_soldier_s')!
    expect(isJewelAffix(jewel)).toBe(true)
    expect(affixFitsPool(jewel, 'Weapon:Melee')).toBe(false)
    const gear = getAffix('25_50_enhanced_damage_t1_soldier_s')!
    expect(isJewelAffix(gear)).toBe(false)
  })
})
