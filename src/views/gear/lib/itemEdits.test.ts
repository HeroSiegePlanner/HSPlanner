import { describe, expect, it } from 'vitest'
import { augments, items, runewords } from '../../../data'
import { AUGMENT_MAX_LEVEL } from '../../../types'
import type { EquippedItem } from '../../../types'
import {
  makeEquippedItem,
  withAffixAdded,
  withAffixRemoved,
  withAugment,
  withAugmentLevel,
  withForgedModAdded,
  withForgedModRemoved,
  withRuneword,
  withSocketCount,
  withSocketType,
  withSocketed,
  withStars,
} from './itemEdits'

function synthItem(baseId = '__synthetic__'): EquippedItem {
  return {
    baseId,
    affixes: [],
    socketCount: 3,
    socketed: [null, null, null],
    socketTypes: ['normal', 'normal', 'normal'],
    stars: 0,
    forgedMods: [],
  }
}

describe('itemEdits — immutability', () => {
  it('withStars returns a new object and leaves the source untouched', () => {
    const item = synthItem()
    const next = withStars(item, 3)
    expect(next).not.toBe(item)
    expect(next.stars).toBe(3)
    expect(item.stars).toBe(0)
  })
})

describe('itemEdits — sockets', () => {
  it('withSocketed sets the socketable at the index', () => {
    const next = withSocketed(synthItem(), 1, 'gem_x')
    expect(next.socketed).toEqual([null, 'gem_x', null])
  })

  it('withSocketed ignores out-of-range indices', () => {
    const item = synthItem()
    expect(withSocketed(item, 9, 'gem_x')).toBe(item)
  })

  it('withSocketType sets the type at the index', () => {
    const next = withSocketType(synthItem(), 0, 'rainbow')
    expect(next.socketTypes[0]).toBe('rainbow')
  })
})

describe('itemEdits — stars clamp', () => {
  it('clamps above MAX_STARS to 5 and floors fractionals', () => {
    expect(withStars(synthItem(), 99).stars).toBe(5)
    expect(withStars(synthItem(), 2.9).stars).toBe(2)
  })

  it('clamps below 0 to 0', () => {
    expect(withStars(synthItem(), -3).stars).toBe(0)
  })
})

describe('itemEdits — affixes', () => {
  it('withAffixAdded appends an affix at roll 1', () => {
    const next = withAffixAdded(synthItem(), 'affix_a', 2)
    expect(next.affixes).toEqual([{ affixId: 'affix_a', tier: 2, roll: 1 }])
  })

  it('withAffixRemoved drops the affix at index', () => {
    const item = {
      ...synthItem(),
      affixes: [
        { affixId: 'a', tier: 1, roll: 1 },
        { affixId: 'b', tier: 1, roll: 1 },
      ],
    }
    expect(withAffixRemoved(item, 0).affixes).toEqual([
      { affixId: 'b', tier: 1, roll: 1 },
    ])
  })
})

describe('itemEdits — forged mods resync sockets', () => {
  it('withForgedModRemoved keeps socketCount within the socket arrays', () => {
    const item = {
      ...synthItem(),
      forgedMods: [{ affixId: 'crystal_add_socket', tier: 1, roll: 1 }],
    }
    const next = withForgedModRemoved(item, 0)
    expect(next.forgedMods).toEqual([])
    expect(next.socketCount).toBe(next.socketed.length)
  })
})

describe('itemEdits — data-driven happy paths', () => {
  it('makeEquippedItem builds a default item for a real base', () => {
    const base = items[0]
    const made = makeEquippedItem(base.id)
    expect(made).not.toBeNull()
    expect(made!.baseId).toBe(base.id)
    expect(made!.affixes).toEqual([])
    expect(made!.socketed.length).toBe(made!.socketCount)
  })

  it('makeEquippedItem returns null for an unknown base', () => {
    expect(makeEquippedItem('__nope__')).toBeNull()
  })

  it('withSocketCount clamps to the base max', () => {
    const base =
      items.find((i) => (i.maxSockets ?? i.sockets ?? 0) >= 1) ?? items[0]
    const item = makeEquippedItem(base.id)!
    const huge = withSocketCount(item, 99)
    expect(huge.socketCount).toBe(huge.socketed.length)
    expect(huge.socketCount).toBeLessThanOrEqual(6)
  })

  it('withRuneword fills sockets with the rune sequence for a compatible base', () => {
    const rw = runewords[0]
    const base = items.find(
      (i) => i.rarity === 'common' && rw.allowedBaseTypes.includes(i.baseType),
    )
    if (!base) return
    const item = makeEquippedItem(base.id)!
    const next = withRuneword(item, rw.id)
    if (rw.runes.length <= 6) {
      expect(next.socketed).toEqual([...rw.runes])
      expect(next.socketCount).toBe(rw.runes.length)
    }
  })

  it('withRuneword returns the item unchanged for an unknown base', () => {
    const item = synthItem('__nope__')
    expect(withRuneword(item, runewords[0].id)).toBe(item)
  })
})

describe('itemEdits — withForgedModAdded', () => {
  it('returns a new object', () => {
    const item = synthItem()
    const next = withForgedModAdded(item, 'crystal_add_socket', 1)
    expect(next).not.toBe(item)
  })

  it('replaces forgedMods with exactly one entry at roll 1', () => {
    const item: EquippedItem = {
      ...synthItem(),
      forgedMods: [{ affixId: 'old_mod', tier: 2, roll: 1 }],
    }
    const next = withForgedModAdded(item, 'new_mod', 3)
    expect(next.forgedMods).toEqual([{ affixId: 'new_mod', tier: 3, roll: 1 }])
  })

  it('keeps socketCount === socketed.length === socketTypes.length after resync', () => {
    const item = synthItem()
    const next = withForgedModAdded(item, 'crystal_add_socket', 1)
    expect(next.socketCount).toBe(next.socketed.length)
    expect(next.socketCount).toBe(next.socketTypes.length)
  })
})

describe('itemEdits — withAugment', () => {
  it('adds augment with level 1 when id is valid', () => {
    if (!augments.length) return
    const id = augments[0].id
    const item = synthItem()
    const next = withAugment(item, id)
    expect(next.augment).toEqual({ id, level: 1 })
  })

  it('removes augment key entirely when null is passed', () => {
    if (!augments.length) return
    const id = augments[0].id
    const item: EquippedItem = { ...synthItem(), augment: { id, level: 2 } }
    const next = withAugment(item, null)
    expect('augment' in next).toBe(false)
  })

  it('returns the item unchanged for an unknown augment id', () => {
    const item = synthItem()
    expect(withAugment(item, '__nope__')).toBe(item)
  })

  it('preserves existing level when re-adding the same augment id', () => {
    if (!augments.length) return
    const id = augments[0].id
    const item: EquippedItem = { ...synthItem(), augment: { id, level: 3 } }
    const next = withAugment(item, id)
    expect(next.augment?.level).toBe(3)
  })
})

describe('itemEdits — withAugmentLevel', () => {
  it('clamps level below 1 up to 1', () => {
    if (!augments.length) return
    const item: EquippedItem = { ...synthItem(), augment: { id: augments[0].id, level: 1 } }
    expect(withAugmentLevel(item, 0).augment?.level).toBe(1)
  })

  it('clamps level above AUGMENT_MAX_LEVEL down to AUGMENT_MAX_LEVEL', () => {
    if (!augments.length) return
    const item: EquippedItem = { ...synthItem(), augment: { id: augments[0].id, level: 1 } }
    expect(withAugmentLevel(item, 999).augment?.level).toBe(AUGMENT_MAX_LEVEL)
  })

  it('rounds fractional levels', () => {
    if (!augments.length) return
    const item: EquippedItem = { ...synthItem(), augment: { id: augments[0].id, level: 1 } }
    expect(withAugmentLevel(item, 2.9).augment?.level).toBe(3)
    expect(withAugmentLevel(item, 2.1).augment?.level).toBe(2)
  })

  it('returns item unchanged when no augment is present', () => {
    const item = synthItem()
    expect(withAugmentLevel(item, 3)).toBe(item)
  })
})
