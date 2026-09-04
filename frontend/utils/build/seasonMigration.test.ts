import { describe, expect, it } from 'vitest'
import { gems, items, skills } from '@data'
import type { EquippedItem } from '../../types'
import { makeSnapshot } from './buildSnapshot.fixture'
import {
  emptyLoadoutSlots,
  initialLoadoutIndexes,
  renameSlot,
  writeSlot,
} from './loadouts'
import {
  clearSeasonBoundAllocations,
  pruneUnknownIds,
} from './seasonMigration'

const realItemId = items[0].id
const realSkillId = skills[0].id
const realGemId = gems[0].id
const skillWithSubtree = skills.find((s) => (s.subskills ?? []).length > 0)!
const realSubskillKey = `${skillWithSubtree.id}:${skillWithSubtree.subskills![0].id}`

function equipped(baseId: string, socketed: (string | null)[] = []): EquippedItem {
  return {
    baseId,
    affixes: [],
    socketCount: socketed.length,
    socketed,
    socketTypes: [],
  }
}

describe('clearSeasonBoundAllocations', () => {
  it('resets tree, ether and tree sockets but keeps gear and skills', () => {
    const snap = makeSnapshot({
      allocatedTreeNodes: new Set([1, 2]),
      allocatedEtherNodes: new Set([4]),
      treeSocketed: { 7: { kind: 'item', id: 'gem_x' } as never },
      inventory: { weapon: equipped(realItemId) },
      skillRanks: { [realSkillId]: 5 },
    })

    const out = clearSeasonBoundAllocations(snap)

    expect(out.allocatedTreeNodes.size).toBe(0)
    expect(out.allocatedEtherNodes.size).toBe(0)
    expect(out.treeSocketed).toEqual({})
    expect(out.inventory.weapon?.baseId).toBe(realItemId)
    expect(out.skillRanks[realSkillId]).toBe(5)
  })

  it('does not mutate the original snapshot', () => {
    const snap = makeSnapshot({ allocatedTreeNodes: new Set([1]) })

    clearSeasonBoundAllocations(snap)

    expect(snap.allocatedTreeNodes.has(1)).toBe(true)
  })
})

describe('pruneUnknownIds', () => {
  it('drops gear whose base does not exist in the active season', () => {
    const snap = makeSnapshot({
      inventory: {
        weapon: equipped(realItemId),
        armor: equipped('item_from_another_season'),
      },
    })

    const out = pruneUnknownIds(snap)

    expect(out.inventory.weapon?.baseId).toBe(realItemId)
    expect(out.inventory.armor).toBeUndefined()
  })

  it('empties unknown socketables but keeps known ones', () => {
    const snap = makeSnapshot({
      inventory: { weapon: equipped(realItemId, [realGemId, 'gem_ghost', null]) },
    })

    const out = pruneUnknownIds(snap)

    expect(out.inventory.weapon?.socketed).toEqual([realGemId, null, null])
  })

  it('drops unknown skill ranks, subskills, active skills and aura', () => {
    const snap = makeSnapshot({
      skillRanks: { [realSkillId]: 3, ghost_skill: 9 },
      subskillRanks: {
        [realSubskillKey]: 1,
        [`${skillWithSubtree.id}:ghost_node`]: 2,
        'ghost_skill:sub_b': 2,
      },
      activeSkillIds: [realSkillId, 'ghost_skill'],
      activeAuraId: 'ghost_aura',
    })

    const out = pruneUnknownIds(snap)

    expect(out.skillRanks).toEqual({ [realSkillId]: 3 })
    expect(out.subskillRanks).toEqual({ [realSubskillKey]: 1 })
    expect(out.activeSkillIds).toEqual([realSkillId])
    expect(out.activeAuraId).toBeNull()
  })

  it('does not mutate the original snapshot', () => {
    const snap = makeSnapshot({
      inventory: { weapon: equipped('item_from_another_season') },
      skillRanks: { ghost_skill: 9 },
    })

    pruneUnknownIds(snap)

    expect(snap.inventory.weapon?.baseId).toBe('item_from_another_season')
    expect(snap.skillRanks.ghost_skill).toBe(9)
  })
})

describe('migrations reach into parked loadouts', () => {
  it('clears season-bound node ids in every slot, not just the live tree', () => {
    const snap = makeSnapshot({
      allocatedTreeNodes: new Set([1, 2]),
      loadoutSlots: {
        ...emptyLoadoutSlots(),
        tree: writeSlot(emptyLoadoutSlots().tree, 3, {
          allocatedTreeNodes: new Set([90, 91]),
          treeSocketed: { 5: { kind: 'item', id: realGemId } },
        }),
        ether: writeSlot(emptyLoadoutSlots().ether, 2, {
          allocatedEtherNodes: new Set([7]),
        }),
      },
      activeLoadouts: initialLoadoutIndexes(),
    })

    const out = clearSeasonBoundAllocations(snap)

    expect(out.allocatedTreeNodes.size).toBe(0)
    expect(out.loadoutSlots?.tree[3]?.data?.allocatedTreeNodes?.size).toBe(0)
    expect(out.loadoutSlots?.tree[3]?.data?.treeSocketed).toEqual({})
    expect(out.loadoutSlots?.ether[2]?.data?.allocatedEtherNodes?.size).toBe(0)
  })

  it('keeps slot labels while clearing their season-bound content', () => {
    const slots = emptyLoadoutSlots()
    const snap = makeSnapshot({
      loadoutSlots: {
        ...slots,
        tree: renameSlot(writeSlot(slots.tree, 1, { allocatedTreeNodes: new Set([4]) }), 1, 'Boss'),
      },
      activeLoadouts: initialLoadoutIndexes(),
    })

    const out = clearSeasonBoundAllocations(snap)

    expect(out.loadoutSlots?.tree[1]?.name).toBe('Boss')
  })

  it('prunes ids the game dropped out of parked gear and skill slots', () => {
    const slots = emptyLoadoutSlots()
    const snap = makeSnapshot({
      loadoutSlots: {
        ...slots,
        gear: writeSlot(slots.gear, 2, {
          inventory: {
            weapon: equipped(realItemId),
            offhand: equipped('item_from_another_season'),
          },
        }),
        skills: writeSlot(slots.skills, 1, {
          skillRanks: { [realSkillId]: 5, ghost_skill: 9 },
          subskillRanks: { [realSubskillKey]: 2, 'ghost:sub': 3 },
        }),
      },
      activeLoadouts: initialLoadoutIndexes(),
    })

    const out = pruneUnknownIds(snap)
    const gear = out.loadoutSlots?.gear[2]?.data?.inventory
    const parkedSkills = out.loadoutSlots?.skills[1]?.data

    expect(gear?.weapon?.baseId).toBe(realItemId)
    expect(gear?.offhand).toBeUndefined()
    expect(parkedSkills?.skillRanks?.[realSkillId]).toBe(5)
    expect(parkedSkills?.skillRanks?.ghost_skill).toBeUndefined()
    expect(parkedSkills?.subskillRanks?.[realSubskillKey]).toBe(2)
    expect(parkedSkills?.subskillRanks?.['ghost:sub']).toBeUndefined()
  })

  it('leaves snapshots without loadouts alone', () => {
    const snap = makeSnapshot({ allocatedTreeNodes: new Set([1]) })
    expect(clearSeasonBoundAllocations(snap).loadoutSlots).toBeUndefined()
    expect(pruneUnknownIds(snap).loadoutSlots).toBeUndefined()
  })
})
