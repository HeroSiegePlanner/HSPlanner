import { describe, expect, it } from 'vitest'
import type { Skill } from '../../types'
import { manaCostAtRank, splitManaCost } from './manaCost'

function skillOf(partial: Partial<Skill>): Skill {
  return {
    id: 'x',
    classId: 'c',
    name: 'X',
    kind: 'active',
    maxRank: 20,
    ranks: [],
    ...partial,
  } as Skill
}

describe('manaCostAtRank', () => {
  it('uses the formula when present, floored like the engine', () => {
    const s = skillOf({
      manaCostFormula: { base: 12, perLevel: 2 },
      ranks: [{ rank: 1, manaCost: 12 }],
    })
    expect(manaCostAtRank(s, 1)).toBe(12)
    expect(manaCostAtRank(s, 20)).toBe(50)
  })

  it('floors fractional formula results', () => {
    const s = skillOf({ manaCostFormula: { base: 5, perLevel: 0.5 } })
    expect(manaCostAtRank(s, 2)).toBe(5)
    expect(manaCostAtRank(s, 3)).toBe(6)
  })

  it('falls back to the exact rank row, then the first rank', () => {
    const s = skillOf({
      ranks: [
        { rank: 1, manaCost: 10 },
        { rank: 2, manaCost: 14 },
      ],
    })
    expect(manaCostAtRank(s, 2)).toBe(14)
    expect(manaCostAtRank(s, 7)).toBe(10)
  })

  it('clamps rank to at least 1', () => {
    const s = skillOf({ manaCostFormula: { base: 12, perLevel: 2 } })
    expect(manaCostAtRank(s, 0)).toBe(12)
  })

  it('returns undefined without any mana data', () => {
    expect(manaCostAtRank(skillOf({}), 5)).toBeUndefined()
  })
})

describe('splitManaCost', () => {
  it('leaves the whole cost as mana when nothing is paid in life', () => {
    expect(splitManaCost(40, 40, 0)).toEqual({
      manaMin: 40,
      manaMax: 40,
      lifeMin: 0,
      lifeMax: 0,
    })
  })

  it('moves the given share of the cost to life', () => {
    const out = splitManaCost(40, 40, 25)
    expect(out.manaMin).toBe(30)
    expect(out.manaMax).toBe(30)
    expect(out.lifeMin).toBe(10)
    expect(out.lifeMax).toBe(10)
  })

  it('clamps the share to 0-100', () => {
    expect(splitManaCost(40, 40, 150).manaMin).toBe(0)
    expect(splitManaCost(40, 40, -20).lifeMax).toBe(0)
  })

  it('keeps undefined costs undefined', () => {
    expect(splitManaCost(undefined, undefined, 25)).toEqual({
      manaMin: undefined,
      manaMax: undefined,
      lifeMin: undefined,
      lifeMax: undefined,
    })
  })
})
