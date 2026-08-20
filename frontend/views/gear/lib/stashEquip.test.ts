import { describe, expect, it } from 'vitest'
import { items } from '@data'
import type { EquippedItem, SlotKey } from '../../../types'
import { equipTargets, targetSlotFor } from './stashEquip'

function equippedOf(baseId: string): EquippedItem {
  return { baseId, affixes: [], socketCount: 0, socketed: [], socketTypes: [] }
}

const helmetBase = items.find((i) => i.slot === 'helmet')!
const ringBase = items.find((i) => i.slot.startsWith('ring'))!
const charmBase = items.find((i) => i.slot.startsWith('charm'))!
const potionBase = items.find((i) => i.slot.startsWith('potion'))!
const shieldBase = items.find((i) => i.slot === 'offhand')!
const oneHandBase = items.find(
  (i) => i.slot === 'weapon' && !i.twoHanded && i.baseType !== 'Wand',
)!
const twoHandBase = items.find((i) => i.slot === 'weapon' && i.twoHanded)!

const NO_NODES = new Set<number>()

describe('targetSlotFor', () => {
  it('targets the single slot for unique-slot items', () => {
    expect(targetSlotFor(equippedOf(helmetBase.id), {})).toBe('helmet')
  })

  it('picks the first free slot in a numbered group', () => {
    const inventory: Partial<Record<SlotKey, EquippedItem>> = {
      ring_1: equippedOf(ringBase.id),
    }
    expect(targetSlotFor(equippedOf(ringBase.id), inventory)).toBe('ring_2')
  })

  it('falls back to the first slot when the whole group is occupied', () => {
    const inventory: Partial<Record<SlotKey, EquippedItem>> = {
      ring_1: equippedOf(ringBase.id),
      ring_2: equippedOf(ringBase.id),
    }
    expect(targetSlotFor(equippedOf(ringBase.id), inventory)).toBe('ring_1')
  })

  it('walks numbered charm slots for the first free one', () => {
    const inventory: Partial<Record<SlotKey, EquippedItem>> = {
      charm_1: equippedOf(charmBase.id),
      charm_2: equippedOf(charmBase.id),
    }
    expect(targetSlotFor(equippedOf(charmBase.id), inventory)).toBe('charm_3')
  })

  it('returns null for an unknown base', () => {
    expect(targetSlotFor(equippedOf('no-such-item'), {})).toBeNull()
  })
})

describe('equipTargets', () => {
  it('offers weapon and offhand for a one-handed weapon', () => {
    expect(equipTargets(equippedOf(oneHandBase.id), {}, NO_NODES)).toEqual([
      'weapon',
      'offhand',
    ])
  })

  it('keeps a two-handed weapon out of the offhand without Hercules Grip', () => {
    expect(equipTargets(equippedOf(twoHandBase.id), {}, NO_NODES)).toEqual([
      'weapon',
    ])
  })

  it('offers only the offhand for a shield', () => {
    expect(equipTargets(equippedOf(shieldBase.id), {}, NO_NODES)).toEqual([
      'offhand',
    ])
  })

  it('offers nothing for a shield while a two-handed weapon is equipped', () => {
    const inventory: Partial<Record<SlotKey, EquippedItem>> = {
      weapon: equippedOf(twoHandBase.id),
    }
    expect(equipTargets(equippedOf(shieldBase.id), inventory, NO_NODES)).toEqual([])
  })

  it('offers both ring slots even when occupied', () => {
    const inventory: Partial<Record<SlotKey, EquippedItem>> = {
      ring_1: equippedOf(ringBase.id),
      ring_2: equippedOf(ringBase.id),
    }
    expect(equipTargets(equippedOf(ringBase.id), inventory, NO_NODES)).toEqual([
      'ring_1',
      'ring_2',
    ])
  })

  it('offers all four potion slots', () => {
    expect(equipTargets(equippedOf(potionBase.id), {}, NO_NODES)).toEqual([
      'potion_1',
      'potion_2',
      'potion_3',
      'potion_4',
    ])
  })

  it('keeps the automatic pick for charms', () => {
    const inventory: Partial<Record<SlotKey, EquippedItem>> = {
      charm_1: equippedOf(charmBase.id),
    }
    expect(equipTargets(equippedOf(charmBase.id), inventory, NO_NODES)).toEqual([
      'charm_2',
    ])
  })

  it('returns an empty list for an unknown base', () => {
    expect(equipTargets(equippedOf('no-such-item'), {}, NO_NODES)).toEqual([])
  })
})
