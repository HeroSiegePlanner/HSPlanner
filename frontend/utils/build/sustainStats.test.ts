import { describe, expect, it, vi } from 'vitest'
import type * as BridgeModule from '../calc/bridge'

vi.mock('../calc/bridge', async (importOriginal) => {
  const mod = await importOriginal<typeof BridgeModule>()
  return {
    ...mod,
    manaCostAtRankNative: vi.fn(async (_skill: unknown, rank: number) => 10 + rank),
  }
})

import { computeSustainStats } from './sustainStats'
import type { Skill } from '../../types'

const skill: Skill = {
  id: 'thunder_fury',
  classId: 'amazon',
  name: 'Thunder Fury',
  kind: 'active',
  maxRank: 12,
  ranks: [],
  baseCastRate: 2,
} as Skill

describe('computeSustainStats', () => {
  it('computes effective rank, cast rate, mana/sec and net mana with no bonuses or reductions', async () => {
    const result = await computeSustainStats({
      skill,
      activeRank: 5,
      rankBonusMin: 0,
      rankBonusMax: 0,
      stats: {},
      statsCombined: {},
    })
    expect(result.effRankMin).toBe(5)
    expect(result.effRankMax).toBe(5)
    expect(result.effManaMin).toBe(15)
    expect(result.effCastMin).toBe(2)
    expect(result.manaPerSecMin).toBe(30)
    expect(result.manaRegenMin).toBe(0)
    expect(result.sustainable).toBe(false)
    expect(result.unsustainable).toBe(true)
  })

  it('rates attack-speed skills off attacks/s and IAS, ignoring cast rate', async () => {
    const kunai = {
      ...skill,
      baseCastRate: undefined,
      usesAttackSpeed: true,
    } as Skill
    const result = await computeSustainStats({
      skill: kunai,
      activeRank: 5,
      rankBonusMin: 0,
      rankBonusMax: 0,
      stats: {
        attacks_per_second: 1.25,
        increased_attack_speed: 60,
        faster_cast_rate: 999,
      },
      statsCombined: {},
    })
    expect(result.effCastMin).toBeCloseTo(2)
    expect(result.effCastMax).toBeCloseTo(2)
  })

  it('rates attack-speed skills off attacks/s and IAS, ignoring cast rate', async () => {
    const kunai = {
      ...skill,
      baseCastRate: undefined,
      usesAttackSpeed: true,
    } as Skill
    const result = await computeSustainStats({
      skill: kunai,
      activeRank: 5,
      rankBonusMin: 0,
      rankBonusMax: 0,
      stats: {
        attacks_per_second: 1.25,
        increased_attack_speed: 60,
        faster_cast_rate: 999,
      },
      statsCombined: {},
    })
    expect(result.effCastMin).toBeCloseTo(2)
    expect(result.effCastMax).toBeCloseTo(2)
  })

  it('pays part of the cast with life when mana costs are taken from life', async () => {
    const result = await computeSustainStats({
      skill,
      activeRank: 5,
      rankBonusMin: 0,
      rankBonusMax: 0,
      stats: { mana_cost_paid_in_life: 25 },
      statsCombined: {},
    })
    expect(result.effManaMin).toBe(11.25)
    expect(result.lifePerCastMin).toBe(3.75)
    expect(result.manaPerSecMin).toBe(22.5)
  })

  it('reports sustainable when mana regen covers the worst-case mana/sec', async () => {
    const result = await computeSustainStats({
      skill,
      activeRank: 5,
      rankBonusMin: 0,
      rankBonusMax: 0,
      stats: { mana_replenish: 999 },
      statsCombined: {},
    })
    expect(result.sustainable).toBe(true)
    expect(result.unsustainable).toBe(false)
  })
})
