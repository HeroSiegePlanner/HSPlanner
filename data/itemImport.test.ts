import { describe, expect, it } from 'vitest'
import { getItem, items, seasonDataErrors } from './index'
import type { ItemBase } from '../frontend/types'
import { BONUS_SOCKET_MOD_ID, maxSocketsFor } from '../frontend/store/itemRules'
import { makeEquippedItem, withForgedModAdded, withSocketCount } from '../frontend/views/gear/lib/itemEdits'

const modules = import.meta.glob<{ default: ItemBase[] }>('./items/*.json', { eager: true })
const baseItems = Object.values(modules).flatMap((m) => m.default)

describe('Hero Siege Helper equipment import', () => {
  it('loads equipment directly from the item files and keeps IDs unique', () => {
    const common = baseItems.filter((item) => item.rarity === 'common')
    expect(common).toHaveLength(156)
    for (const item of baseItems) {
      expect(getItem(item.id), item.id).toEqual(item)
    }
    expect(items).toHaveLength(baseItems.length)
    expect(seasonDataErrors).toEqual([])
    expect(new Set(items.map((item) => item.id)).size).toBe(items.length)
  })

  it('keeps fixed, ranged and zero-start socket values distinct', () => {
    for (const [id, min, max] of [
      ['helmet_satanic_harlequinn_s_crest', 0, 2],
      ['s10_leviathans_ribcage', 2, 4],
      ['s10_captains_attire', 1, 6],
      ['charm_heroic_tablet_of_awakening', 1, 4],
      ['shields_gabriels_unholy_oath', 4, 6],
    ] as const) {
      const item = getItem(id)
      expect(item, id).toBeDefined()
      expect([item?.sockets, item?.maxSockets], id).toEqual([min, max])
    }
    for (const item of items.filter((item) => item.rarity !== 'common')) {
      expect(item.sockets, item.id).toBeGreaterThanOrEqual(0)
      expect(item.maxSockets, item.id).toBeGreaterThanOrEqual(item.sockets!)
      expect(item.maxSockets, item.id).toBeLessThanOrEqual(6)
    }
  })

  it('cannot add sockets to an item without source socket information, even with a crystal', () => {
    const base = items.find((item) => item.name === 'The Colossal Avenger')!
    const equipped = makeEquippedItem(base.id)!
    expect([base.sockets, base.maxSockets]).toEqual([0, 0])
    expect(maxSocketsFor(base.id, [{ affixId: BONUS_SOCKET_MOD_ID }])).toBe(0)
    const forged = withForgedModAdded(equipped, BONUS_SOCKET_MOD_ID, 1)
    expect(withSocketCount(forged, 6)).toMatchObject({ socketCount: 0, socketed: [], socketTypes: [] })
  })

  it('retains the Common socket and bonus-socket rules', () => {
    const cap = getItem('helmet_normal_cap')!
    expect([cap.sockets, cap.maxSockets]).toEqual([0, 4])
    expect(maxSocketsFor(cap.id, [{ affixId: BONUS_SOCKET_MOD_ID }])).toBe(5)
  })

  it('updates elemental variants without adding both additive and total cast rate', () => {
    const cloak = getItem('body_armor_heroic_wraith_s_cloak_cold')!
    expect(cloak.implicit?.faster_cast_rate_more).toBe(20)
    expect(cloak.implicit?.faster_cast_rate).toBeUndefined()
    expect(cloak.implicit?.cold_skills).toEqual([5, 8])
    expect(cloak.implicit?.fire_skills).toBeUndefined()
    expect(getItem('s10_captains_anchor')?.implicit?.random_skill_element).toEqual([3, 6])
  })
})
