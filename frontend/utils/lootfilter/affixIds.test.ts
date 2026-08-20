import { describe, expect, test } from 'vitest'
import type { LootFilter } from '../../types'
import { createDefaultLootFilter, decodeLootFilter, encodeLootFilter } from './codec'
import { FILTER_STAT_BY_ID, FILTER_STATS } from './constants'
import gameAffixCodes from './gameAffixCodes.fixture.json'

interface GameAffixCode {
  name: string
  id: number
  code: string
}

const CODES = gameAffixCodes as GameAffixCode[]

function hideAffix(statId: number): LootFilter {
  const base = createDefaultLootFilter()
  const type = base.types[0]!
  return {
    ...base,
    types: {
      ...base.types,
      0: {
        ...type,
        tiers: type.tiers.map((t, i) => (i === 0 ? { ...t, hidden: [statId] } : t)),
      },
    },
  }
}

describe('kalibracja ID afiksów na eksportach z gry', () => {
  test('fixture pokrywa 66 afiksów o unikalnych ID', () => {
    expect(CODES).toHaveLength(66)
    expect(new Set(CODES.map((c) => c.id)).size).toBe(66)
  })

  test('każdy kod z gry dekoduje się do afiksu o tej samej nazwie', () => {
    for (const { id, name, code } of CODES) {
      const decoded = decodeLootFilter(code)
      expect(decoded, name).not.toBeNull()
      expect(decoded!.types[0]!.tiers[0]!.hidden, name).toEqual([id])
      expect(FILTER_STAT_BY_ID.get(id)?.name, `id ${id}`).toBe(name)
    }
  })

  test('kodowanie odtwarza string gry znak w znak', () => {
    for (const { id, name, code } of CODES) {
      expect(encodeLootFilter(hideAffix(id)), name).toBe(code)
    }
  })

  test('lista afiksów ma 165 unikalnych ID w 4 kolumnach', () => {
    expect(FILTER_STATS).toHaveLength(165)
    expect(new Set(FILTER_STATS.map((s) => s.id)).size).toBe(165)
    expect(new Set(FILTER_STATS.map((s) => s.col))).toEqual(new Set([1, 2, 3, 4]))
  })

  test('ID rosną w obrębie każdej kolumny — kolejność zgodna z układem gry', () => {
    for (const col of [1, 2, 3, 4]) {
      const ids = FILTER_STATS.filter((s) => s.col === col).map((s) => s.id)
      expect(ids, `col ${col}`).toEqual([...ids].sort((a, b) => a - b))
    }
  })
})
