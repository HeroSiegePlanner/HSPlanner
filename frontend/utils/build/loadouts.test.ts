import { describe, expect, it } from 'vitest'
import {
  clearSlot,
  clearSlotData,
  dematerializeSlots,
  emptyLoadout,
  emptyLoadoutSlots,
  emptySlots,
  extractLoadout,
  fromSparse,
  LOADOUT_FIELDS,
  initialLoadoutIndexes,
  isSlotOccupied,
  isValidSlotIndex,
  LOADOUT_SLOT_COUNT,
  LOADOUT_TABS,
  MAX_LOADOUT_NAME_LENGTH,
  occupiedCount,
  renameSlot,
  slotLabel,
  switchSlot,
  toSparse,
  writeSlot,
} from './loadouts'

interface Payload {
  tag: string
}

const p = (tag: string): Payload => ({ tag })

describe('loadouts — shape', () => {
  it('starts with 8 empty slots per tab and index 0 active', () => {
    const slots = emptyLoadoutSlots()
    for (const tab of LOADOUT_TABS) {
      expect(slots[tab]).toHaveLength(LOADOUT_SLOT_COUNT)
      expect(slots[tab].every((s) => s.data === null && s.name === null)).toBe(true)
    }
    expect(initialLoadoutIndexes()).toEqual({ tree: 0, ether: 0, skills: 0, gear: 0 })
  })

  it('rejects out-of-range and non-integer slot indexes', () => {
    expect(isValidSlotIndex(0)).toBe(true)
    expect(isValidSlotIndex(LOADOUT_SLOT_COUNT - 1)).toBe(true)
    expect(isValidSlotIndex(LOADOUT_SLOT_COUNT)).toBe(false)
    expect(isValidSlotIndex(-1)).toBe(false)
    expect(isValidSlotIndex(1.5)).toBe(false)
    expect(isValidSlotIndex(NaN)).toBe(false)
  })
})

describe('loadouts — per-tab field slice', () => {
  const liveState = {
    allocatedTreeNodes: new Set([1, 2, 3]),
    treeSocketed: { 7: null },
    allocatedEtherNodes: new Set([10]),
    skillRanks: { fireball: 20 },
    subskillRanks: { 'fireball:sub_1': 5 },
    inventory: { weapon: { baseId: 'sword' } },
  } as const

  it('gives every tab a disjoint, non-empty set of fields', () => {
    const seen = new Set<string>()
    for (const tab of LOADOUT_TABS) {
      expect(LOADOUT_FIELDS[tab].length).toBeGreaterThan(0)
      for (const field of LOADOUT_FIELDS[tab]) {
        expect(seen.has(field)).toBe(false)
        seen.add(field)
      }
    }
  })

  it('extracts only the fields the tab owns', () => {
    expect(Object.keys(extractLoadout('tree', liveState)).sort()).toEqual([
      'allocatedTreeNodes',
      'treeSocketed',
    ])
    expect(extractLoadout('ether', liveState)).toEqual({
      allocatedEtherNodes: new Set([10]),
    })
    expect(extractLoadout('gear', liveState)).toEqual({
      inventory: { weapon: { baseId: 'sword' } },
    })
    expect(Object.keys(extractLoadout('skills', liveState)).sort()).toEqual([
      'skillRanks',
      'subskillRanks',
    ])
  })

  it('does not leak another tab’s state', () => {
    const gear = extractLoadout('gear', liveState)
    expect('allocatedTreeNodes' in gear).toBe(false)
    expect('skillRanks' in gear).toBe(false)
  })

  it('shares references rather than deep-copying (store state is immutable)', () => {
    expect(extractLoadout('ether', liveState).allocatedEtherNodes).toBe(
      liveState.allocatedEtherNodes,
    )
  })

  it('builds a cleared payload covering exactly the tab’s fields', () => {
    expect(emptyLoadout('tree')).toEqual({
      allocatedTreeNodes: new Set<number>(),
      treeSocketed: {},
    })
    expect(emptyLoadout('skills')).toEqual({ skillRanks: {}, subskillRanks: {} })
    for (const tab of LOADOUT_TABS) {
      expect(Object.keys(emptyLoadout(tab)).sort()).toEqual(
        [...LOADOUT_FIELDS[tab]].sort(),
      )
    }
  })

  it('returns a fresh cleared payload each call', () => {
    const a = emptyLoadout('tree')
    const b = emptyLoadout('tree')
    expect(a.allocatedTreeNodes).not.toBe(b.allocatedTreeNodes)
    a.allocatedTreeNodes!.add(99)
    expect(b.allocatedTreeNodes!.size).toBe(0)
  })
})

describe('loadouts — occupancy', () => {
  it('counts the active slot as occupied even though its data is null', () => {
    const slots = emptySlots<Payload>()
    expect(slots[3]?.data).toBeNull()
    expect(isSlotOccupied(slots, 3, 3)).toBe(true)
    expect(isSlotOccupied(slots, 4, 3)).toBe(false)
    expect(occupiedCount(slots, 3)).toBe(1)
  })

  it('counts stored slots plus the active one', () => {
    let slots = emptySlots<Payload>()
    slots = writeSlot(slots, 4, p('boss'))
    slots = writeSlot(slots, 7, p('farm'))
    expect(occupiedCount(slots, 0)).toBe(3)
    // Active index already holding data must not be double counted.
    expect(occupiedCount(slots, 4)).toBe(2)
  })

  it('treats an out-of-range index as unoccupied', () => {
    expect(isSlotOccupied(emptySlots<Payload>(), 99, 0)).toBe(false)
  })
})

describe('loadouts — labels', () => {
  it('falls back to the 1-based slot number', () => {
    const slots = emptySlots<Payload>()
    expect(slotLabel(slots, 0)).toBe('1')
    expect(slotLabel(slots, 7)).toBe('8')
  })

  it('trims, caps and nulls out blank names', () => {
    let slots = renameSlot(emptySlots<Payload>(), 0, '  Boss  ')
    expect(slots[0]?.name).toBe('Boss')
    expect(slotLabel(slots, 0)).toBe('Boss')

    slots = renameSlot(slots, 0, '   ')
    expect(slots[0]?.name).toBeNull()
    expect(slotLabel(slots, 0)).toBe('1')

    slots = renameSlot(slots, 1, 'x'.repeat(MAX_LOADOUT_NAME_LENGTH + 20))
    expect(slots[1]?.name).toHaveLength(MAX_LOADOUT_NAME_LENGTH)
  })

  it('keeps the payload when renaming and the name when writing', () => {
    let slots = writeSlot(emptySlots<Payload>(), 2, p('keep'))
    slots = renameSlot(slots, 2, 'Named')
    expect(slots[2]).toEqual({ name: 'Named', data: p('keep') })
    slots = writeSlot(slots, 2, p('new'))
    expect(slots[2]).toEqual({ name: 'Named', data: p('new') })
  })
})

describe('loadouts — immutability', () => {
  it('never mutates the input array', () => {
    const slots = emptySlots<Payload>()
    const frozen = [...slots]
    writeSlot(slots, 1, p('a'))
    renameSlot(slots, 1, 'a')
    clearSlot(slots, 1)
    expect(slots).toEqual(frozen)
  })

  it('returns the same array for an invalid index', () => {
    const slots = emptySlots<Payload>()
    expect(writeSlot(slots, 99, p('a'))).toBe(slots)
  })
})

describe('loadouts — clearing', () => {
  it('clearSlotData drops the payload but keeps the label', () => {
    let slots = writeSlot(emptySlots<Payload>(), 1, p('a'))
    slots = renameSlot(slots, 1, 'Named')
    expect(clearSlotData(slots, 1)).toMatchObject({ 1: { name: 'Named', data: null } })
  })

  it('clearSlot frees label and payload', () => {
    let slots = writeSlot(emptySlots<Payload>(), 1, p('a'))
    slots = renameSlot(slots, 1, 'Named')
    expect(clearSlot(slots, 1)[1]).toEqual({ name: null, data: null })
  })
})

describe('loadouts — switchSlot', () => {
  it('parks the live payload and hands back the target payload', () => {
    const slots = writeSlot(emptySlots<Payload>(), 4, p('boss'))
    const res = switchSlot(slots, 0, 4, p('live'))
    expect(res).not.toBeNull()
    expect(res!.data).toEqual(p('boss'))
    // Old slot now holds what was live; new active slot is nulled out.
    expect(res!.slots[0]?.data).toEqual(p('live'))
    expect(res!.slots[4]?.data).toBeNull()
  })

  it('returns null data when switching into an empty slot', () => {
    const res = switchSlot(emptySlots<Payload>(), 0, 5, p('live'))
    expect(res!.data).toBeNull()
    expect(res!.slots[0]?.data).toEqual(p('live'))
    expect(res!.slots[5]?.data).toBeNull()
  })

  it('preserves the null invariant for the new active slot', () => {
    const slots = writeSlot(emptySlots<Payload>(), 2, p('parked'))
    const res = switchSlot(slots, 0, 2, p('live'))
    expect(res!.slots[2]?.data).toBeNull()
  })

  it('rejects a no-op or invalid switch', () => {
    const slots = emptySlots<Payload>()
    expect(switchSlot(slots, 1, 1, p('live'))).toBeNull()
    expect(switchSlot(slots, -1, 2, p('live'))).toBeNull()
    expect(switchSlot(slots, 0, 99, p('live'))).toBeNull()
  })

  it('round-trips back to the original payloads', () => {
    const start = writeSlot(emptySlots<Payload>(), 1, p('b'))
    const there = switchSlot(start, 0, 1, p('a'))!
    const back = switchSlot(there.slots, 1, 0, there.data!)!
    expect(back.data).toEqual(p('a'))
    expect(back.slots[1]?.data).toEqual(p('b'))
  })
})

describe('loadouts — dematerialize', () => {
  it('restores the invariant and returns the payload the active slot carried', () => {
    const slots = writeSlot(emptySlots<Payload>(), 2, p('smuggled'))
    const { slots: fixed, data } = dematerializeSlots(slots, 2)
    expect(data).toEqual(p('smuggled'))
    expect(fixed[2]?.data).toBeNull()
  })

  it('keeps the label while dropping the payload', () => {
    let slots = writeSlot(emptySlots<Payload>(), 1, p('x'))
    slots = renameSlot(slots, 1, 'Boss')
    expect(dematerializeSlots(slots, 1).slots[1]).toEqual({ name: 'Boss', data: null })
  })

  it('is a no-op on slots that already hold the invariant', () => {
    const slots = writeSlot(emptySlots<Payload>(), 4, p('parked'))
    expect(dematerializeSlots(slots, 0).slots).toEqual(slots)
  })

  it('leaves slots untouched for an invalid active index', () => {
    const slots = emptySlots<Payload>()
    expect(dematerializeSlots(slots, 99)).toEqual({ slots, data: null })
  })
})

describe('loadouts — sparse wire form', () => {
  it('emits only non-empty slots', () => {
    let slots = writeSlot(emptySlots<Payload>(), 0, p('a'))
    slots = writeSlot(slots, 4, p('e'))
    expect(Object.keys(toSparse(slots)).sort()).toEqual(['0', '4'])
  })

  it('preserves the slot index instead of compacting', () => {
    // Creator used slots 1 and 5 (indexes 0 and 4).
    let slots = writeSlot(emptySlots<Payload>(), 0, p('main'))
    slots = writeSlot(slots, 4, p('boss'))
    const restored = fromSparse(toSparse(slots))
    expect(restored[4]?.data).toEqual(p('boss'))
    expect(restored[1]?.data).toBeNull()
    expect(restored[2]?.data).toBeNull()
    expect(restored[3]?.data).toBeNull()
  })

  it('always rebuilds a full 8-slot grid', () => {
    expect(fromSparse<Payload>({ '2': { d: p('x') } })).toHaveLength(LOADOUT_SLOT_COUNT)
    expect(fromSparse<Payload>(undefined)).toHaveLength(LOADOUT_SLOT_COUNT)
  })

  it('carries names and omits them when absent', () => {
    let slots = writeSlot(emptySlots<Payload>(), 1, p('a'))
    slots = renameSlot(slots, 1, 'Boss')
    slots = writeSlot(slots, 2, p('b'))
    const sparse = toSparse(slots)
    expect(sparse['1']).toEqual({ n: 'Boss', d: p('a') })
    expect(sparse['2']).toEqual({ d: p('b') })
    expect(fromSparse(sparse)[1]?.name).toBe('Boss')
    expect(fromSparse(sparse)[2]?.name).toBeNull()
  })

  it('drops out-of-range keys from untrusted input', () => {
    const restored = fromSparse<Payload>({
      '0': { d: p('ok') },
      '8': { d: p('overflow') },
      '-1': { d: p('negative') },
      'x': { d: p('garbage') },
      '1.5': { d: p('fractional') },
    })
    expect(restored).toHaveLength(LOADOUT_SLOT_COUNT)
    expect(restored[0]?.data).toEqual(p('ok'))
    expect(restored.filter((s) => s.data !== null)).toHaveLength(1)
  })

  it('caps names coming off the wire', () => {
    const restored = fromSparse<Payload>({
      '0': { n: 'y'.repeat(MAX_LOADOUT_NAME_LENGTH + 50), d: p('a') },
    })
    expect(restored[0]?.name).toHaveLength(MAX_LOADOUT_NAME_LENGTH)
  })

  it('round-trips a full grid', () => {
    let slots = emptySlots<Payload>()
    for (let i = 0; i < LOADOUT_SLOT_COUNT; i++) slots = writeSlot(slots, i, p(`s${i}`))
    expect(fromSparse(toSparse(slots))).toEqual(slots)
  })
})

describe('loadouts — named-but-empty slots (gear progression stages)', () => {
  it('counts a labelled slot as occupied even with no payload', () => {
    const slots = renameSlot(emptySlots<Payload>(), 3, 'Early')
    expect(slots[3]?.data).toBeNull()
    expect(isSlotOccupied(slots, 3, 0)).toBe(true)
    expect(isSlotOccupied(slots, 4, 0)).toBe(false)
    expect(occupiedCount(slots, 0)).toBe(2) // slot 3 plus the active slot 0
  })

  it('stops counting once the label is dropped', () => {
    let slots = renameSlot(emptySlots<Payload>(), 3, 'Early')
    slots = renameSlot(slots, 3, null)
    expect(isSlotOccupied(slots, 3, 0)).toBe(false)
  })

  it('sends a label with no payload over the wire', () => {
    const slots = renameSlot(emptySlots<Payload>(), 2, 'Aspirational')
    const sparse = toSparse(slots)
    expect(sparse['2']).toEqual({ n: 'Aspirational' })
    expect('d' in sparse['2']!).toBe(false)
  })

  it('restores a label-only slot with null data', () => {
    const restored = fromSparse<Payload>({ '2': { n: 'Late' } })
    expect(restored[2]).toEqual({ name: 'Late', data: null })
  })

  it('round-trips a mix of named, filled and empty slots', () => {
    let slots = writeSlot(emptySlots<Payload>(), 0, p('early'))
    slots = renameSlot(slots, 0, 'Early')
    slots = renameSlot(slots, 1, 'Mid Game') // named, not filled yet
    slots = writeSlot(slots, 4, p('late'))   // filled, unnamed
    expect(fromSparse(toSparse(slots))).toEqual(slots)
  })

  it('still skips slots that are neither named nor filled', () => {
    expect(Object.keys(toSparse(emptySlots<Payload>()))).toEqual([])
  })
})
