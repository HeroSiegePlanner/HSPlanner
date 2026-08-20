import { describe, expect, it } from 'vitest'
import { collectSocketGroups, collectSocketStats } from './socketStats'
import { getGem, getItem, getRune } from '@data'
import { RAINBOW_MULTIPLIER } from '../../store/itemRules'
import type { EquippedItem } from '../../types'

describe('collectSocketStats', () => {
  it('aggregates gem stats with the rainbow multiplier applied to a rainbow socket', () => {
    const gem = getGem('gem_chipped_amethyst')
    if (!gem) throw new Error('fixture gem missing from game data — pick a real id from src/data/gems/gems.json')
    const equipped: EquippedItem = {
      baseId: 'boots_satanic_boots_of_wild',
      affixes: [],
      socketCount: 1,
      socketed: [gem.id],
      socketTypes: ['rainbow'],
    }
    const out = collectSocketStats(equipped)
    expect(out.length).toBeGreaterThan(0)
    const outMap = Object.fromEntries(out)
    for (const [key, value] of Object.entries(gem.stats)) {
      expect(outMap[key]).toBe(value * RAINBOW_MULTIPLIER)
    }
  })

  it('applies the rainbow multiplier to built-in rainbow sockets from the base', () => {
    const gem = getGem('gem_chipped_amethyst')
    if (!gem) throw new Error('fixture gem missing from game data')
    const equipped: EquippedItem = {
      baseId: 'charm_heroic_tablet_of_awakening',
      affixes: [],
      socketCount: 2,
      socketed: [gem.id, gem.id],
      socketTypes: ['normal', 'normal'],
    }
    const base = {
      id: 'charm_heroic_tablet_of_awakening',
      name: 'Tablet of Awakening',
      baseType: 'Charm',
      slot: 'charm_1',
      rarity: 'heroic',
      rainbowSockets: [2, 3, 4],
    }
    const out = Object.fromEntries(
      collectSocketStats(equipped, base as never),
    )
    // socket 1 normal + socket 2 built-in rainbow: x1 + x1.5
    for (const [key, value] of Object.entries(gem.stats)) {
      expect(out[key]).toBeCloseTo(value * (1 + RAINBOW_MULTIPLIER))
    }
  })

  it('sums stats across multiple normal sockets (mixing a gem and a rune) without a multiplier', () => {
    const gem = getGem('gem_chipped_amethyst')
    const rune = getRune('rune_old')
    if (!gem || !rune) throw new Error('fixture gem/rune missing from game data')
    const equipped: EquippedItem = {
      baseId: 'boots_satanic_boots_of_wild',
      affixes: [],
      socketCount: 2,
      socketed: [gem.id, rune.id],
      socketTypes: ['normal', 'normal'],
    }
    const out = collectSocketStats(equipped)
    const outMap = Object.fromEntries(out)
    for (const [key, value] of Object.entries(gem.stats)) {
      expect(outMap[key]).toBe(value)
    }
    for (const [key, value] of Object.entries(rune.stats)) {
      expect(outMap[key]).toBe(value)
    }
  })

  it("uses the base item's socketTransform stats instead of the gem's own stats when a transform is defined", () => {
    const base = getItem('ring_heroic_azazel_s_despair')
    if (!base) throw new Error('fixture item missing from game data — pick a real id from src/data/items')
    const transformGemId = 'gem_damien_s_soulgem'
    const transformStats = base.socketTransforms?.[transformGemId]
    if (!transformStats) {
      throw new Error('fixture socketTransforms entry missing — check src/data/items/rings.json')
    }
    const equipped: EquippedItem = {
      baseId: base.id,
      affixes: [],
      socketCount: 1,
      socketed: [transformGemId],
      socketTypes: ['normal'],
    }
    const out = collectSocketStats(equipped, base)
    const outMap = Object.fromEntries(out)
    for (const [key, value] of Object.entries(transformStats)) {
      if (value === 0) continue
      expect(outMap[key]).toBe(value)
    }
    const gem = getGem(transformGemId)
    expect(gem).toBeDefined()
    if (gem) {
      for (const key of Object.keys(gem.stats)) {
        if (!(key in transformStats)) expect(outMap[key]).toBeUndefined()
      }
    }
  })

  it('skips empty socket slots and unresolvable ids without throwing', () => {
    const equipped: EquippedItem = {
      baseId: 'boots_satanic_boots_of_wild',
      affixes: [],
      socketCount: 2,
      socketed: [null, 'not_a_real_gem_or_rune'],
      socketTypes: ['normal', 'normal'],
    }
    expect(() => collectSocketStats(equipped)).not.toThrow()
    expect(collectSocketStats(equipped)).toEqual([])
  })
})

describe('collectSocketGroups', () => {
  it('merges sockets holding the same gem into one group and counts them', () => {
    const gem = getGem('gem_chipped_amethyst')
    const rune = getRune('rune_old')
    if (!gem || !rune) throw new Error('fixture gem/rune missing from game data')
    const equipped: EquippedItem = {
      baseId: 'boots_satanic_boots_of_wild',
      affixes: [],
      socketCount: 3,
      socketed: [gem.id, rune.id, gem.id],
      socketTypes: ['normal', 'normal', 'normal'],
    }
    const groups = collectSocketGroups(equipped)
    expect(groups.map((g) => [g.id, g.count])).toEqual([
      [gem.id, 2],
      [rune.id, 1],
    ])
    expect(groups[0]!.name).toBe(gem.name)
    const gemStats = Object.fromEntries(groups[0]!.stats)
    for (const [key, value] of Object.entries(gem.stats)) {
      expect(gemStats[key]).toBe(value * 2)
    }
  })
})
