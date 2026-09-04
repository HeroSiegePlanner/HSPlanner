import { describe, expect, it } from 'vitest'
import {
  compressToEncodedURIComponent,
  decompressFromEncodedURIComponent,
} from 'lz-string'
import {
  decodeShareToBuild,
  encodeBuildToShare,
  type BuildSnapshot,
} from './shareBuild'
import { makeSnapshot } from './buildSnapshot.fixture'
import {
  emptyLoadoutSlots,
  initialLoadoutIndexes,
  LOADOUT_SLOT_COUNT,
  renameSlot,
  writeSlot,
  type LoadoutSlotsMap,
} from './loadouts'
import type { EquippedItem } from '../../types'

function item(baseId: string): EquippedItem {
  return {
    baseId,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
    stars: 0,
    forgedMods: [],
  }
}

function withLoadouts(
  build: (slots: LoadoutSlotsMap) => LoadoutSlotsMap,
  active = initialLoadoutIndexes(),
): BuildSnapshot {
  return makeSnapshot({
    loadoutSlots: build(emptyLoadoutSlots()),
    activeLoadouts: active,
  })
}

const roundTrip = (snap: BuildSnapshot) => {
  const decoded = decodeShareToBuild(encodeBuildToShare(snap))
  expect(decoded).not.toBeNull()
  return decoded!.snapshot
}

describe('share v3 — loadouts round-trip', () => {
  it('carries a parked tree loadout', () => {
    const snap = withLoadouts((s) => ({
      ...s,
      tree: writeSlot(s.tree, 1, { allocatedTreeNodes: new Set([4, 2, 9]), treeSocketed: {} }),
    }))
    const out = roundTrip(snap)
    expect(out.loadoutSlots?.tree[1]?.data?.allocatedTreeNodes).toEqual(new Set([2, 4, 9]))
  })

  it('carries parked ether, skills and gear loadouts', () => {
    const snap = withLoadouts((s) => ({
      ...s,
      ether: writeSlot(s.ether, 2, { allocatedEtherNodes: new Set([7]) }),
      skills: writeSlot(s.skills, 3, {
        skillRanks: { fireball: 20 },
        subskillRanks: { 'fireball:s1': 5 },
      }),
      gear: writeSlot(s.gear, 4, { inventory: { weapon: item('sword') } }),
    }))
    const out = roundTrip(snap)
    expect(out.loadoutSlots?.ether[2]?.data?.allocatedEtherNodes).toEqual(new Set([7]))
    expect(out.loadoutSlots?.skills[3]?.data?.skillRanks).toEqual({ fireball: 20 })
    expect(out.loadoutSlots?.skills[3]?.data?.subskillRanks).toEqual({ 'fireball:s1': 5 })
    expect(out.loadoutSlots?.gear[4]?.data?.inventory.weapon?.baseId).toBe('sword')
  })

  it('preserves the slot index instead of compacting it', () => {
    // Creator filled slots 1 and 5 (indexes 0 and 4).
    const snap = withLoadouts((s) => ({
      ...s,
      gear: writeSlot(writeSlot(s.gear, 0, { inventory: {} }), 4, {
        inventory: { weapon: item('sword') },
      }),
    }))
    const out = roundTrip(snap)
    expect(out.loadoutSlots?.gear[4]?.data?.inventory.weapon?.baseId).toBe('sword')
    expect(out.loadoutSlots?.gear[1]?.data).toBeNull()
    expect(out.loadoutSlots?.gear[2]?.data).toBeNull()
    expect(out.loadoutSlots?.gear[3]?.data).toBeNull()
  })

  it('carries stage names, including for slots with no payload yet', () => {
    const snap = withLoadouts((s) => ({
      ...s,
      gear: renameSlot(renameSlot(s.gear, 1, 'Early'), 5, 'Aspirational'),
    }))
    const out = roundTrip(snap)
    expect(out.loadoutSlots?.gear[1]).toEqual({ name: 'Early', data: null })
    expect(out.loadoutSlots?.gear[5]).toEqual({ name: 'Aspirational', data: null })
  })

  it('carries the active slot index per tab', () => {
    const snap = withLoadouts((s) => s, { tree: 3, ether: 0, skills: 7, gear: 1 })
    const out = roundTrip(snap)
    expect(out.activeLoadouts).toEqual({ tree: 3, ether: 0, skills: 7, gear: 1 })
  })

  it('keeps the active slot empty — its content rides in the snapshot fields', () => {
    const snap = makeSnapshot({
      allocatedTreeNodes: new Set([1, 2, 3]),
      loadoutSlots: emptyLoadoutSlots(),
      activeLoadouts: { tree: 2, ether: 0, skills: 0, gear: 0 },
    })
    const out = roundTrip(snap)
    expect(out.loadoutSlots?.tree[2]?.data).toBeNull()
    expect([...out.allocatedTreeNodes]).toEqual([1, 2, 3])
  })

  it('adds nothing to the payload when no loadout is in use', () => {
    const bare = makeSnapshot()
    const withEmpty = makeSnapshot({
      loadoutSlots: emptyLoadoutSlots(),
      activeLoadouts: initialLoadoutIndexes(),
    })
    expect(encodeBuildToShare(withEmpty)).toBe(encodeBuildToShare(bare))
  })
})

describe('share v3 — backwards compatibility', () => {
  it('still decodes a v2 code, with empty loadouts', () => {
    // A v2 payload: same shape, older version marker, no `ld`.
    const v2 = compressToEncodedURIComponent(
      JSON.stringify({
        v: 2,
        c: 'amazon',
        l: 42,
        a: {},
        i: {},
        s: {},
        ss: {},
        t: [1, 2],
        m: [],
        u: null,
        buf: {},
        ec: {},
        pt: {},
        kps: 1,
        se: 's10',
      }),
    )
    const decoded = decodeShareToBuild(v2)
    expect(decoded).not.toBeNull()
    expect(decoded!.snapshot.level).toBe(42)
    expect(decoded!.snapshot.activeLoadouts).toEqual(initialLoadoutIndexes())
    expect(decoded!.snapshot.loadoutSlots?.tree.every((s) => s.data === null)).toBe(true)
  })

  it('rejects a version above the current schema', () => {
    const future = compressToEncodedURIComponent(
      JSON.stringify({
        v: 99,
        c: null,
        l: 1,
        a: {},
        i: {},
        s: {},
        ss: {},
        t: [],
        m: [],
        u: null,
        buf: {},
        ec: {},
        pt: {},
        kps: 1,
      }),
    )
    expect(decodeShareToBuild(future)).toBeNull()
  })
})

describe('share v3 — hostile input', () => {
  const base = {
    v: 3,
    c: null,
    l: 1,
    a: {},
    i: {},
    s: {},
    ss: {},
    t: [],
    m: [],
    u: null,
    buf: {},
    ec: {},
    pt: {},
    kps: 1,
    se: 's10',
  }
  const encode = (ld: unknown) =>
    compressToEncodedURIComponent(JSON.stringify({ ...base, ld }))

  it('drops slot indexes outside the grid', () => {
    const decoded = decodeShareToBuild(
      encode({ gear: { '0': { n: 'ok' }, '99': { n: 'overflow' }, '-1': { n: 'neg' } } }),
    )
    expect(decoded).not.toBeNull()
    const gear = decoded!.snapshot.loadoutSlots?.gear
    expect(gear).toHaveLength(LOADOUT_SLOT_COUNT)
    // Slot 0 is the active one, so its name survives but its payload is nulled.
    expect(gear?.filter((s) => s.name !== null)).toHaveLength(1)
  })

  it('refuses a code with more slots than the grid holds', () => {
    const tooMany = Object.fromEntries(
      Array.from({ length: LOADOUT_SLOT_COUNT + 4 }, (_, i) => [String(i), { n: `s${i}` }]),
    )
    expect(decodeShareToBuild(encode({ gear: tooMany }))).toBeNull()
  })

  it('refuses an out-of-range active index', () => {
    expect(decodeShareToBuild(encode({ a: { tree: 99 } }))).toBeNull()
    expect(decodeShareToBuild(encode({ a: { tree: -1 } }))).toBeNull()
  })

  it('refuses an unknown tab name', () => {
    expect(decodeShareToBuild(encode({ a: { bogus: 1 } }))).toBeNull()
  })

  it('cannot smuggle gear into a tree slot', () => {
    const decoded = decodeShareToBuild(
      encode({ tree: { '1': { d: { i: { weapon: { baseId: 'sword' } }, t: [5] } } } }),
    )
    expect(decoded).not.toBeNull()
    const parked = decoded!.snapshot.loadoutSlots?.tree[1]?.data
    expect(parked?.allocatedTreeNodes).toEqual(new Set([5]))
    expect('inventory' in (parked ?? {})).toBe(false)
  })

  it('survives a malformed payload without throwing', () => {
    expect(decodeShareToBuild(encode({ gear: 'not an object' }))).toBeNull()
    expect(decodeShareToBuild(encode({ gear: { '0': { d: { i: 'nope' } } } }))).toBeNull()
  })
})

describe('share v3 — size guard', () => {
  function bigItem(seed: number): EquippedItem {
    return {
      baseId: `base_${seed}`,
      affixes: Array.from({ length: 6 }, (_, k) => ({
        affixId: `affix_${seed}_${k}_${'x'.repeat(150)}`,
        tier: 5,
        roll: 0.5,
      })),
      socketCount: 6,
      socketed: Array.from({ length: 6 }, (_, k) => `socket_${seed}_${k}`),
      socketTypes: Array.from({ length: 6 }, () => 'normal' as const),
      stars: 5,
      forgedMods: [],
    }
  }

  const fatInventory = () =>
    Object.fromEntries(
      Array.from({ length: 30 }, (_, i) => [`slot_${i}`, bigItem(i)]),
    )

  it('drops parked loadouts rather than emitting an undecodable code', () => {
    let slots = emptyLoadoutSlots()
    for (let i = 1; i < LOADOUT_SLOT_COUNT; i++) {
      slots = { ...slots, gear: writeSlot(slots.gear, i, { inventory: fatInventory() }) }
    }
    const snap = makeSnapshot({
      inventory: fatInventory(),
      loadoutSlots: slots,
      activeLoadouts: { tree: 0, ether: 0, skills: 0, gear: 2 },
    })

    let dropped = false
    const code = encodeBuildToShare(snap, undefined, 's10', {
      onDegraded: (what) => {
        dropped = what === 'loadouts-dropped'
      },
    })
    expect(dropped).toBe(true)

    // The point of dropping: the code still decodes, and the active build is intact.
    const decoded = decodeShareToBuild(code)
    expect(decoded).not.toBeNull()
    expect(Object.keys(decoded!.snapshot.inventory)).toHaveLength(30)
    // Active indexes are kept so the receiver lands on the same slot.
    expect(decoded!.snapshot.activeLoadouts?.gear).toBe(2)
    expect(decoded!.snapshot.loadoutSlots?.gear.every((s) => s.data === null)).toBe(true)
  })

  it('does not fire for a payload that fits', () => {
    let dropped = false
    encodeBuildToShare(
      withLoadouts((s) => ({
        ...s,
        gear: writeSlot(s.gear, 1, { inventory: { weapon: item('sword') } }),
      })),
      undefined,
      's10',
      {
        onDegraded: () => {
          dropped = true
        },
      },
    )
    expect(dropped).toBe(false)
  })
})

describe('share v3 — empty loadout fields', () => {
  it('does not put empty records or sets on the wire', () => {
    // {} and Set(0) are truthy; before the guard every one of them shipped.
    const code = encodeBuildToShare(
      withLoadouts((s) => ({
        ...s,
        skills: writeSlot(s.skills, 1, { skillRanks: {}, subskillRanks: {} }),
        gear: writeSlot(s.gear, 1, { inventory: {} }),
        ether: writeSlot(s.ether, 1, { allocatedEtherNodes: new Set<number>() }),
      })),
    )
    const wire = JSON.parse(decompressFromEncodedURIComponent(code)!) as {
      ld: Record<string, Record<string, { d?: Record<string, unknown> }>>
    }
    expect(wire.ld.skills?.['1']?.d).toEqual({})
    expect(wire.ld.gear?.['1']?.d).toEqual({})
    expect(wire.ld.ether?.['1']?.d).toEqual({})

    // The slot still reads as used; wireToPayload rebuilds the cleared fields.
    const decoded = decodeShareToBuild(code)
    expect(decoded!.snapshot.loadoutSlots?.gear[1]?.data?.inventory).toEqual({})
  })
})

describe('share v3 — degradation is re-checked', () => {
  const hugeNotes = 'x'.repeat(210_000)

  it('truncates notes when dropping the loadouts is not enough', () => {
    const degraded: string[] = []
    const code = encodeBuildToShare(
      withLoadouts((s) => ({
        ...s,
        gear: writeSlot(s.gear, 1, { inventory: { weapon: item('sword') } }),
      })),
      hugeNotes,
      's10',
      { onDegraded: (what) => degraded.push(what) },
    )

    expect(degraded).toContain('loadouts-dropped')
    expect(degraded).toContain('notes-truncated')
    expect(degraded).not.toContain('oversize')
    // The point of re-checking: the code still decodes.
    const decoded = decodeShareToBuild(code)
    expect(decoded).not.toBeNull()
    expect(decoded!.notes.length).toBeLessThan(hugeNotes.length)
  })

  it('truncates notes even with no loadouts to drop', () => {
    const degraded: string[] = []
    const code = encodeBuildToShare(makeSnapshot({}), hugeNotes, 's10', {
      onDegraded: (what) => degraded.push(what),
    })

    expect(degraded).toEqual(['notes-truncated'])
    expect(decodeShareToBuild(code)).not.toBeNull()
  })
})
