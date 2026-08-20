import { describe, expect, test } from 'vitest'
import type { LootFilter } from '../../types'
import {
  DEFAULT_RS,
  DEFAULT_SOC,
  DEFAULT_WTC,
  FILTER_TYPE_IDS,
  createDefaultLootFilter,
  decodeLootFilter,
  encodeLootFilter,
} from './codec'
import { HELL_FILTER_S9 } from './hellFilter.fixture'

function normalized(filter: LootFilter): LootFilter {
  return {
    ...filter,
    types: Object.fromEntries(
      Object.entries(filter.types).map(([id, t]) => [
        id,
        {
          ...t,
          tiers: t.tiers.map((tier) => ({
            rs: tier.rs,
            hidden: [...tier.hidden].sort((a, b) => a - b),
            highlighted: [...tier.highlighted].sort((a, b) => a - b),
          })),
        },
      ]),
    ),
  }
}

describe('decodeLootFilter', () => {
  test('dekoduje prawdziwy eksport z gry (Hell Filter s9)', () => {
    const filter = decodeLootFilter(HELL_FILTER_S9)
    expect(filter).not.toBeNull()
    const f = filter!
    expect(f.version).toBe(2)
    expect(f.wtc).toBe(212031)
    const helmet = f.types[0]!
    expect(helmet.tiers).toHaveLength(5)
    expect(helmet.tiers[3]!.rs).toBe(2016)
    expect(helmet.tiers[3]!.hidden).toHaveLength(155)
    expect(helmet.tiers[3]!.highlighted).toContain(201)
    expect(helmet.tiers[4]!.rs).toBe(2017)
    expect(helmet.soc).toBe(62)
    expect(helmet.soch).toBe(62)
    expect(f.types[4]!.soc).toBe(DEFAULT_SOC)
    for (const id of FILTER_TYPE_IDS) expect(f.types[id]).toBeDefined()
  })

  test('toleruje floaty GameMakera i deltę (brakujące klucze = default)', () => {
    const json = '{"version":2,"t0":{"tr4":{"hls":[75.0]},"soc":59,"tr1":{"hs":[75.0]}},"wtc":260959}'
    const filter = decodeLootFilter(btoa(json))
    expect(filter).not.toBeNull()
    const f = filter!
    expect(f.types[0]!.tiers[4]!.highlighted).toEqual([75])
    expect(f.types[0]!.tiers[1]!.hidden).toEqual([75])
    expect(f.types[0]!.tiers[0]!.rs).toBe(DEFAULT_RS)
    expect(f.types[0]!.tiers[0]!.hidden).toEqual([])
    expect(f.types[0]!.soc).toBe(59)
    expect(f.wtc).toBe(260959)
    expect(f.types[3]!.tiers[2]!.rs).toBe(DEFAULT_RS)
  })

  test('odrzuca śmieci', () => {
    expect(decodeLootFilter('')).toBeNull()
    expect(decodeLootFilter('nie-base64 !!!')).toBeNull()
    expect(decodeLootFilter(btoa('[]'))).toBeNull()
    expect(decodeLootFilter(btoa('"tekst"'))).toBeNull()
    expect(decodeLootFilter(btoa('{zepsuty json'))).toBeNull()
  })
})

describe('encodeLootFilter', () => {
  test('round-trip na prawdziwym filtrze zachowuje semantykę', () => {
    const original = decodeLootFilter(HELL_FILTER_S9)!
    const reencoded = encodeLootFilter(original)
    const roundTripped = decodeLootFilter(reencoded)!
    expect(normalized(roundTripped)).toEqual(normalized(original))
  })

  test('domyślny filtr koduje się do samego {"version":2}', () => {
    const code = encodeLootFilter(createDefaultLootFilter())
    expect(JSON.parse(atob(code))).toEqual({ version: 2 })
  })

  test('emituje quirk GameMakera: ostatni element tablicy jako float', () => {
    const filter = createDefaultLootFilter()
    filter.types[0] = {
      ...filter.types[0]!,
      tiers: filter.types[0]!.tiers.map((t, i) =>
        i === 1 ? { ...t, hidden: [26, 75] } : t,
      ),
    }
    const json = atob(encodeLootFilter(filter))
    expect(json).toContain('"hs":[26,75.0]')
  })

  test('pomija wartości domyślne (delta jak w grze)', () => {
    const filter = createDefaultLootFilter()
    filter.wtc = 260959
    filter.types[3] = {
      ...filter.types[3]!,
      soc: 59,
      tiers: filter.types[3]!.tiers.map((t, i) =>
        i === 4 ? { ...t, rs: 2017 } : t,
      ),
    }
    const parsed = JSON.parse(atob(encodeLootFilter(filter))) as Record<string, unknown>
    expect(Object.keys(parsed).sort()).toEqual(['t3', 'version', 'wtc'])
    expect(parsed.t3).toEqual({ tr4: { rs: 2017 }, soc: 59 })
    expect(parsed.wtc).toBe(260959)
  })

  test('default wtc nie jest emitowany, nie-default tak', () => {
    const def = createDefaultLootFilter()
    expect(def.wtc).toBe(DEFAULT_WTC)
    expect(JSON.parse(atob(encodeLootFilter(def)))).not.toHaveProperty('wtc')
  })
})
