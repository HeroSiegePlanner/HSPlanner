import { describe, expect, test } from 'vitest'
import { createDefaultLootFilter, DEFAULT_RS } from '../../utils/lootfilter/codec'
import { FILTER_STATS, WEAPON_TYPES } from '../../utils/lootfilter/constants'
import {
  affixCellState,
  copyTypeConfig,
  filterSummary,
  rarityCellState,
  socketCellState,
  toggleAffixHighlight,
  toggleAffixRow,
  toggleAffixVisible,
  toggleRarityHighlight,
  toggleRarityRow,
  toggleRarityTier,
  toggleRarityVisible,
  toggleSocketHighlight,
  toggleSocketVisible,
  toggleTierColumn,
  toggleWeaponType,
  typeSummary,
} from './filterModel'

describe('filterModel', () => {
  test('rarity: ukrycie gasi highlight, highlight tylko widocznych', () => {
    let f = createDefaultLootFilter()
    expect(rarityCellState(DEFAULT_RS, 0)).toBe('visible')
    f = toggleRarityHighlight(f, 0, 4, 0)
    expect(rarityCellState(f.types[0]!.tiers[4]!.rs, 0)).toBe('highlighted')
    expect(f.types[0]!.tiers[4]!.rs).toBe(2017)
    f = toggleRarityVisible(f, 0, 4, 0)
    expect(rarityCellState(f.types[0]!.tiers[4]!.rs, 0)).toBe('hidden')
    const same = toggleRarityHighlight(f, 0, 4, 0)
    expect(same.types[0]!.tiers[4]!.rs).toBe(f.types[0]!.tiers[4]!.rs)
    expect(rarityCellState(DEFAULT_RS, 0)).toBe('visible')
  })

  test('afiks: toggle widoczności i highlightu', () => {
    let f = createDefaultLootFilter()
    const id = FILTER_STATS[0]!.id
    f = toggleAffixVisible(f, 3, 2, id)
    expect(affixCellState(f.types[3]!, 2, id)).toBe('hidden')
    f = toggleAffixVisible(f, 3, 2, id)
    expect(affixCellState(f.types[3]!, 2, id)).toBe('visible')
    f = toggleAffixHighlight(f, 3, 2, id)
    expect(affixCellState(f.types[3]!, 2, id)).toBe('highlighted')
    f = toggleAffixVisible(f, 3, 2, id)
    expect(f.types[3]!.tiers[2]!.highlighted).not.toContain(id)
  })

  test('klik w nazwę afiksu przełącza cały wiersz', () => {
    let f = createDefaultLootFilter()
    const id = FILTER_STATS[5]!.id
    f = toggleAffixRow(f, 0, id)
    for (let tier = 0; tier < 5; tier++) {
      expect(affixCellState(f.types[0]!, tier, id)).toBe('hidden')
    }
    f = toggleAffixRow(f, 0, id)
    for (let tier = 0; tier < 5; tier++) {
      expect(affixCellState(f.types[0]!, tier, id)).toBe('visible')
    }
  })

  test('klik w literę tieru przełącza kolumnę wszystkich afiksów', () => {
    let f = createDefaultLootFilter()
    f = toggleTierColumn(f, 0, 0)
    expect(f.types[0]!.tiers[0]!.hidden).toHaveLength(FILTER_STATS.length)
    expect(f.types[0]!.tiers[1]!.hidden).toHaveLength(0)
    f = toggleTierColumn(f, 0, 0)
    expect(f.types[0]!.tiers[0]!.hidden).toHaveLength(0)
  })

  test('sockety: widoczność i highlight na bitach', () => {
    let f = createDefaultLootFilter()
    expect(socketCellState(f.types[0]!, 0)).toBe('visible')
    f = toggleSocketVisible(f, 0, 0)
    expect(socketCellState(f.types[0]!, 0)).toBe('hidden')
    expect(f.types[0]!.soc).toBe(0b111110)
    f = toggleSocketVisible(f, 0, 0)
    f = toggleSocketHighlight(f, 0, 0)
    expect(socketCellState(f.types[0]!, 0)).toBe('highlighted')
    f = toggleSocketVisible(f, 0, 0)
    expect(f.types[0]!.soch & 1).toBe(0)
  })

  test('typy broni: 16 kategorii gry w kolejności siatki 4×4', () => {
    expect(WEAPON_TYPES.filter(Boolean)).toEqual([
      'Sword',
      'Dagger',
      'Mace',
      'Axe',
      'Claw',
      'Polearm',
      'Chainsaw',
      'Staff',
      'Cane',
      'Wand',
      'Book',
      'Spellblade',
      'Bow',
      'Gun',
      'Flask',
      'Throwing',
    ])
    expect(WEAPON_TYPES[1]).toBe('Sword')
    expect(WEAPON_TYPES[16]).toBe('Throwing')
  })

  test('typy broni: xor bitu', () => {
    let f = createDefaultLootFilter()
    f = toggleWeaponType(f, 7)
    expect((f.wtc >> 7) & 1).toBe(0)
    f = toggleWeaponType(f, 7)
    expect((f.wtc >> 7) & 1).toBe(1)
  })

  test('klik w nazwę rarity przełącza ją na wszystkich tierach', () => {
    let f = createDefaultLootFilter()
    f = toggleRarityRow(f, 0, 2)
    for (let tier = 0; tier < 5; tier++) {
      expect(rarityCellState(f.types[0]!.tiers[tier]!.rs, 2)).toBe('hidden')
    }
    f = toggleRarityRow(f, 0, 2)
    for (let tier = 0; tier < 5; tier++) {
      expect(rarityCellState(f.types[0]!.tiers[tier]!.rs, 2)).toBe('visible')
    }
  })

  test('klik w literę tieru w macierzy rarity przełącza wszystkie rarity tego tieru', () => {
    let f = createDefaultLootFilter()
    f = toggleRarityTier(f, 0, 3)
    for (let rarity = 0; rarity < 5; rarity++) {
      expect(rarityCellState(f.types[0]!.tiers[3]!.rs, rarity)).toBe('hidden')
      expect(rarityCellState(f.types[0]!.tiers[2]!.rs, rarity)).toBe('visible')
    }
    f = toggleRarityTier(f, 0, 3)
    expect(rarityCellState(f.types[0]!.tiers[3]!.rs, 0)).toBe('visible')
  })

  test('typeSummary liczy unikalne afiksy i ukryte komórki rarity', () => {
    let f = createDefaultLootFilter()
    expect(typeSummary(f.types[0]!)).toEqual({
      edited: false,
      hidden: new Set(),
      highlighted: new Set(),
      raritiesHidden: 0,
    })
    const a = FILTER_STATS[0]!.id
    const b = FILTER_STATS[1]!.id
    f = toggleAffixVisible(f, 0, 0, a)
    f = toggleAffixVisible(f, 0, 1, a)
    f = toggleAffixHighlight(f, 0, 2, b)
    f = toggleRarityVisible(f, 0, 4, 4)
    expect(typeSummary(f.types[0]!)).toEqual({
      edited: true,
      hidden: new Set([a]),
      highlighted: new Set([b]),
      raritiesHidden: 1,
    })
  })

  test('typeSummary uznaje highlight rarity za edycję', () => {
    let f = createDefaultLootFilter()
    f = toggleRarityHighlight(f, 0, 0, 0)
    const summary = typeSummary(f.types[0]!)
    expect(summary.raritiesHidden).toBe(0)
    expect(summary.edited).toBe(true)
  })

  test('filterSummary agreguje typy i dolicza wtc', () => {
    let f = createDefaultLootFilter()
    expect(filterSummary(f)).toEqual({
      editedTypes: 0,
      hiddenStats: 0,
      highlightedStats: 0,
    })
    const a = FILTER_STATS[0]!.id
    f = toggleAffixVisible(f, 0, 0, a)
    f = toggleAffixHighlight(f, 3, 1, a)
    f = toggleWeaponType(f, 1)
    expect(filterSummary(f)).toEqual({
      editedTypes: 3,
      hiddenStats: 1,
      highlightedStats: 1,
    })
  })

  test('copyTypeConfig kopiuje tiery i sockety do innych typów', () => {
    let f = createDefaultLootFilter()
    const id = FILTER_STATS[0]!.id
    f = toggleAffixVisible(f, 0, 1, id)
    f = toggleSocketVisible(f, 0, 2)
    f = copyTypeConfig(f, 0, [3, 7])
    expect(affixCellState(f.types[3]!, 1, id)).toBe('hidden')
    expect(f.types[7]!.soc).toBe(f.types[0]!.soc)
    const after = toggleAffixVisible(f, 3, 1, id)
    expect(affixCellState(after.types[0]!, 1, id)).toBe('hidden')
  })
})
