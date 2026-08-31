import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { useBuild } from './index'
import {
  initUndoHistory,
  redoLastChange,
  undoLastChange,
} from '../undoHistory'
import {
  emptyLoadoutSlots,
  initialLoadoutIndexes,
  LOADOUT_SLOT_COUNT,
  LOADOUT_TABS,
} from '../../utils/build/loadouts'
import { items } from '@data'
import {
  decodeShareToBuild,
  encodeBuildToShare,
} from '../../utils/build/shareBuild'
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

let stopUndo: (() => void) | null = null

// snapshotPatch runs pruneUnknownIds, which drops ids the game does not ship —
// the import round trip needs real ones.
const REAL_ITEM_A = items[0]!.id
const REAL_ITEM_B = items[1]!.id

function reset() {
  useBuild.setState({
    loadoutSlots: emptyLoadoutSlots(),
    activeLoadouts: initialLoadoutIndexes(),
    allocatedTreeNodes: new Set<number>(),
    treeSocketed: {},
    allocatedEtherNodes: new Set<number>(),
    skillRanks: {},
    subskillRanks: {},
    inventory: {},
  })
}

describe('loadouts slice — switching', () => {
  beforeEach(reset)

  it('parks live state in the old slot and loads the target', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([1, 2, 3]) })

    expect(useBuild.getState().switchLoadout('tree', 1)).toBe(true)
    // Slot 2 is empty, so the tree starts blank there.
    expect(useBuild.getState().allocatedTreeNodes.size).toBe(0)
    expect(useBuild.getState().activeLoadouts.tree).toBe(1)

    useBuild.setState({ allocatedTreeNodes: new Set([9]) })
    expect(useBuild.getState().switchLoadout('tree', 0)).toBe(true)
    expect([...useBuild.getState().allocatedTreeNodes]).toEqual([1, 2, 3])

    expect(useBuild.getState().switchLoadout('tree', 1)).toBe(true)
    expect([...useBuild.getState().allocatedTreeNodes]).toEqual([9])
  })

  it('keeps the active slot payload null — one copy of every loadout', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([1]) })
    useBuild.getState().switchLoadout('tree', 3)
    const { loadoutSlots, activeLoadouts } = useBuild.getState()
    expect(activeLoadouts.tree).toBe(3)
    expect(loadoutSlots.tree[3]?.data).toBeNull()
    expect(loadoutSlots.tree[0]?.data?.allocatedTreeNodes).toEqual(new Set([1]))
  })

  it('carries every field the tab owns, not just the first', () => {
    useBuild.setState({ skillRanks: { fireball: 20 }, subskillRanks: { 'fireball:s1': 5 } })
    useBuild.getState().switchLoadout('skills', 1)
    expect(useBuild.getState().skillRanks).toEqual({})
    expect(useBuild.getState().subskillRanks).toEqual({})
    useBuild.getState().switchLoadout('skills', 0)
    expect(useBuild.getState().skillRanks).toEqual({ fireball: 20 })
    expect(useBuild.getState().subskillRanks).toEqual({ 'fireball:s1': 5 })
  })

  it('does not touch state owned by other tabs', () => {
    useBuild.setState({
      allocatedTreeNodes: new Set([1]),
      inventory: { weapon: item(REAL_ITEM_A) },
      skillRanks: { fireball: 10 },
      allocatedEtherNodes: new Set([5]),
    })

    useBuild.getState().switchLoadout('tree', 2)

    const s = useBuild.getState()
    expect(s.inventory).toEqual({ weapon: item(REAL_ITEM_A) })
    expect(s.skillRanks).toEqual({ fireball: 10 })
    expect([...s.allocatedEtherNodes]).toEqual([5])
    expect(s.activeLoadouts).toEqual({ tree: 2, ether: 0, skills: 0, gear: 0 })
  })

  it('keeps each tab on its own independent slot index', () => {
    useBuild.getState().switchLoadout('tree', 1)
    useBuild.getState().switchLoadout('gear', 4)
    expect(useBuild.getState().activeLoadouts).toEqual({
      tree: 1,
      ether: 0,
      skills: 0,
      gear: 4,
    })
  })

  it('rejects a no-op or out-of-range switch', () => {
    const s = useBuild.getState()
    expect(s.switchLoadout('tree', 0)).toBe(false)
    expect(s.switchLoadout('tree', LOADOUT_SLOT_COUNT)).toBe(false)
    expect(s.switchLoadout('tree', -1)).toBe(false)
    expect(useBuild.getState().activeLoadouts.tree).toBe(0)
  })

  it('works for every tab', () => {
    for (const tab of LOADOUT_TABS) {
      expect(useBuild.getState().switchLoadout(tab, 5)).toBe(true)
      expect(useBuild.getState().activeLoadouts[tab]).toBe(5)
    }
  })
})

describe('loadouts slice — clearing', () => {
  beforeEach(reset)

  it('clearing the active slot resets that tab live state', () => {
    useBuild.setState({ inventory: { weapon: item(REAL_ITEM_A) } })
    expect(useBuild.getState().clearLoadout('gear', 0)).toBe(true)
    expect(useBuild.getState().inventory).toEqual({})
  })

  it('clearing the active slot leaves other tabs alone', () => {
    useBuild.setState({
      inventory: { weapon: item(REAL_ITEM_A) },
      allocatedTreeNodes: new Set([1]),
    })
    useBuild.getState().clearLoadout('gear', 0)
    expect([...useBuild.getState().allocatedTreeNodes]).toEqual([1])
  })

  it('frees a stored slot without touching live state', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([1]) })
    useBuild.getState().switchLoadout('tree', 1)
    useBuild.setState({ allocatedTreeNodes: new Set([7]) })

    expect(useBuild.getState().clearLoadout('tree', 0)).toBe(true)
    expect(useBuild.getState().loadoutSlots.tree[0]?.data).toBeNull()
    expect([...useBuild.getState().allocatedTreeNodes]).toEqual([7])
  })

  it('drops the label along with the payload', () => {
    useBuild.getState().renameLoadout('tree', 2, 'Boss')
    useBuild.getState().switchLoadout('tree', 2)
    useBuild.setState({ allocatedTreeNodes: new Set([1]) })
    useBuild.getState().switchLoadout('tree', 0)

    useBuild.getState().clearLoadout('tree', 2)
    expect(useBuild.getState().loadoutSlots.tree[2]?.name).toBeNull()
  })

  it('reports false for an empty slot and an invalid index', () => {
    const s = useBuild.getState()
    expect(s.clearLoadout('tree', 4)).toBe(false)
    expect(s.clearLoadout('tree', 99)).toBe(false)
  })
})

describe('loadouts slice — renaming', () => {
  beforeEach(reset)

  it('names an empty slot and can clear the name again', () => {
    expect(useBuild.getState().renameLoadout('gear', 3, '  Boss  ')).toBe(true)
    expect(useBuild.getState().loadoutSlots.gear[3]?.name).toBe('Boss')
    useBuild.getState().renameLoadout('gear', 3, null)
    expect(useBuild.getState().loadoutSlots.gear[3]?.name).toBeNull()
  })

  it('survives a switch through the slot', () => {
    useBuild.getState().renameLoadout('tree', 1, 'Farm')
    useBuild.getState().switchLoadout('tree', 1)
    expect(useBuild.getState().loadoutSlots.tree[1]?.name).toBe('Farm')
  })

  it('rejects an invalid index', () => {
    expect(useBuild.getState().renameLoadout('tree', 99, 'x')).toBe(false)
  })
})

describe('loadouts slice — duplicating', () => {
  beforeEach(reset)

  it('copies the active slot live state into an empty slot', () => {
    useBuild.setState({ inventory: { weapon: item(REAL_ITEM_A) } })
    expect(useBuild.getState().duplicateLoadout('gear', 0, 2)).toBe(true)
    expect(useBuild.getState().loadoutSlots.gear[2]?.data?.inventory).toEqual({
      weapon: item(REAL_ITEM_A),
    })
    // Source stays put; live state untouched.
    expect(useBuild.getState().inventory).toEqual({ weapon: item(REAL_ITEM_A) })
  })

  it('copies a stored slot into another stored slot', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([1, 2]) })
    useBuild.getState().switchLoadout('tree', 1)
    expect(useBuild.getState().duplicateLoadout('tree', 0, 5)).toBe(true)
    expect(useBuild.getState().loadoutSlots.tree[5]?.data?.allocatedTreeNodes).toEqual(
      new Set([1, 2]),
    )
  })

  it('overwrites a target that already holds a loadout', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([1]) })
    useBuild.getState().duplicateLoadout('tree', 0, 3)
    useBuild.setState({ allocatedTreeNodes: new Set([9]) })

    expect(useBuild.getState().duplicateLoadout('tree', 0, 3)).toBe(true)
    expect(useBuild.getState().loadoutSlots.tree[3]?.data?.allocatedTreeNodes).toEqual(
      new Set([9]),
    )
  })

  it('copying onto the active slot overwrites live state, keeping data null', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([1]) })
    useBuild.getState().switchLoadout('tree', 1)
    useBuild.setState({ allocatedTreeNodes: new Set([9]) })

    // Slot 1 holds [1]; copy it over the active slot 2.
    expect(useBuild.getState().duplicateLoadout('tree', 0, 1)).toBe(true)
    expect([...useBuild.getState().allocatedTreeNodes]).toEqual([1])
    expect(useBuild.getState().loadoutSlots.tree[1]?.data).toBeNull()
    // The source is untouched.
    expect(useBuild.getState().loadoutSlots.tree[0]?.data?.allocatedTreeNodes).toEqual(
      new Set([1]),
    )
  })

  it('still refuses an empty source, a no-op and an invalid index', () => {
    expect(useBuild.getState().duplicateLoadout('tree', 4, 6)).toBe(false)
    expect(useBuild.getState().duplicateLoadout('tree', 0, 0)).toBe(false)
    expect(useBuild.getState().duplicateLoadout('tree', 0, 99)).toBe(false)
    expect(useBuild.getState().duplicateLoadout('tree', -1, 2)).toBe(false)
  })

  it('makes an independent copy — editing live state does not leak into it', () => {
    useBuild.setState({ skillRanks: { fireball: 5 } })
    useBuild.getState().duplicateLoadout('skills', 0, 3)
    useBuild.setState({ skillRanks: { fireball: 20 } })
    expect(useBuild.getState().loadoutSlots.skills[3]?.data?.skillRanks).toEqual({
      fireball: 5,
    })
  })
})

describe('loadouts slice — undo/redo', () => {
  beforeEach(() => {
    reset()
    stopUndo?.()
    stopUndo = initUndoHistory()
  })

  afterEach(() => {
    stopUndo?.()
    stopUndo = null
  })

  it('undoes a slot switch back to the previous slot and its state', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([1, 2]) })
    useBuild.getState().switchLoadout('tree', 1)
    useBuild.setState({ allocatedTreeNodes: new Set([9]) })

    expect(undoLastChange()).toBe(true) // undo the edit made in slot 2
    expect(undoLastChange()).toBe(true) // undo the switch itself

    expect(useBuild.getState().activeLoadouts.tree).toBe(0)
    expect([...useBuild.getState().allocatedTreeNodes]).toEqual([1, 2])
  })

  it('redoes the switch', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([1]) })
    useBuild.getState().switchLoadout('tree', 2)
    undoLastChange()
    expect(useBuild.getState().activeLoadouts.tree).toBe(0)

    expect(redoLastChange()).toBe(true)
    expect(useBuild.getState().activeLoadouts.tree).toBe(2)
    expect(useBuild.getState().allocatedTreeNodes.size).toBe(0)
  })

  it('undoes a clear', () => {
    useBuild.setState({ inventory: { weapon: item(REAL_ITEM_A) } })
    useBuild.getState().clearLoadout('gear', 0)
    expect(useBuild.getState().inventory).toEqual({})
    expect(undoLastChange()).toBe(true)
    expect(useBuild.getState().inventory).toEqual({ weapon: item(REAL_ITEM_A) })
  })
})

describe('loadouts slice — full round trip through the store', () => {
  beforeEach(reset)

  it('survives export → encode → decode → import', () => {
    // Build up: gear stage named + filled in slot 3, a tree loadout in slot 2,
    // and each tab sitting on a different active slot.
    useBuild.setState({ inventory: { weapon: item(REAL_ITEM_A) } })
    useBuild.getState().renameLoadout('gear', 2, 'Early')
    useBuild.getState().switchLoadout('gear', 2)
    useBuild.setState({ inventory: { helmet: item(REAL_ITEM_B) } })

    useBuild.setState({ allocatedTreeNodes: new Set([11, 12]) })
    useBuild.getState().switchLoadout('tree', 1)
    useBuild.setState({ allocatedTreeNodes: new Set([21]) })

    const code = encodeBuildToShare(useBuild.getState().exportBuildSnapshot())
    const decoded = decodeShareToBuild(code)
    expect(decoded).not.toBeNull()

    reset()
    useBuild.getState().importBuildSnapshot(decoded!.snapshot)

    const s = useBuild.getState()
    // Active slots and their live content.
    expect(s.activeLoadouts).toEqual({ tree: 1, ether: 0, skills: 0, gear: 2 })
    expect([...s.allocatedTreeNodes]).toEqual([21])
    expect(s.inventory.helmet?.baseId).toBe(REAL_ITEM_B)
    // Parked slots, with the label intact.
    expect(s.loadoutSlots.gear[2]?.name).toBe('Early')
    expect(s.loadoutSlots.gear[0]?.data?.inventory?.weapon?.baseId).toBe(REAL_ITEM_A)
    expect(s.loadoutSlots.tree[0]?.data?.allocatedTreeNodes).toEqual(new Set([11, 12]))
  })

  it('switching still works after an import', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([5]) })
    useBuild.getState().switchLoadout('tree', 1)
    useBuild.setState({ allocatedTreeNodes: new Set([6]) })

    const decoded = decodeShareToBuild(
      encodeBuildToShare(useBuild.getState().exportBuildSnapshot()),
    )
    reset()
    useBuild.getState().importBuildSnapshot(decoded!.snapshot)

    expect(useBuild.getState().switchLoadout('tree', 0)).toBe(true)
    expect([...useBuild.getState().allocatedTreeNodes]).toEqual([5])
    expect([...useBuild.getState().loadoutSlots.tree[1]!.data!.allocatedTreeNodes!]).toEqual([6])
  })
})
