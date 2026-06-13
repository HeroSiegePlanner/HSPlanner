import { describe, expect, it } from 'vitest'
import { affixes, gems, items, skills, treeNodeInfo } from '../../data'
import { ADJ, START_IDS } from '../tree/treeGraph'
import { makeSnapshot } from './buildSnapshot.fixture'
import { convertSnapshotToActiveSeason } from './seasonConvert'

describe('convertSnapshotToActiveSeason', () => {
  it('valid snapshot passes through with no changes', () => {
    const start = START_IDS[0]!
    const snap = makeSnapshot({
      allocatedTreeNodes: new Set([start]),
      skillRanks: { [skills[0]!.id]: 3 },
    })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')
    expect(report.hasChanges).toBe(false)
    expect(snapshot.allocatedTreeNodes.has(start)).toBe(true)
    expect(report.fromSeason).toBe('s10')
    expect(report.toSeason).toBe('s9')
  })

  it('drops unknown tree nodes and prunes orphans', () => {
    const start = START_IDS[0]!
    const snap = makeSnapshot({
      allocatedTreeNodes: new Set([start, 999_999]),
    })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')
    expect(report.hasChanges).toBe(true)
    expect(report.removedTreeNodes).toEqual([999_999])
    expect(snapshot.allocatedTreeNodes.has(999_999)).toBe(false)
    expect(report.freedTreePoints).toBe(1)
  })

  it('drops unknown items, affixes, skills and clears dangling references', () => {
    const snap = makeSnapshot({
      inventory: {
        weapon: {
          baseId: 'no_such_item_id',
          affixes: [],
          socketCount: 0,
          socketed: [],
          socketTypes: [],
        },
        helmet: {
          baseId: items[0]!.id,
          affixes: [
            { affixId: affixes[0]!.id, tier: 1, roll: 0.5 },
            { affixId: 'no_such_affix', tier: 1, roll: 0.5 },
          ],
          socketCount: 0,
          socketed: [],
          socketTypes: [],
        },
      },
      skillRanks: { no_such_skill: 5 },
      mainSkillId: 'no_such_skill',
    })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')
    expect(snapshot.inventory.weapon).toBeUndefined()
    expect(snapshot.inventory.helmet?.affixes).toHaveLength(1)
    expect(snapshot.skillRanks).toEqual({})
    expect(snapshot.mainSkillId).toBeNull()
    expect(report.removedItems).toEqual([{ slot: 'weapon', baseId: 'no_such_item_id' }])
    expect(report.removedAffixes).toEqual([{ slot: 'helmet', affixId: 'no_such_affix' }])
    expect(report.removedSkills).toEqual(['no_such_skill'])
    expect(report.hasChanges).toBe(true)
  })

  it('does not mutate the input snapshot', () => {
    const snap = makeSnapshot({ allocatedTreeNodes: new Set([999_999]) })
    convertSnapshotToActiveSeason(snap, 's10')
    expect(snap.allocatedTreeNodes.has(999_999)).toBe(true)
  })

  // B1: orphan pruning — a real node not connected to the allocated start becomes orphaned
  it('orphan pruning: real node unreachable from allocated start is orphaned', () => {
    const startId = START_IDS[0]!
    const adjToStart = ADJ.get(startId) ?? new Set<number>()
    const startSet = new Set(START_IDS)
    // Find a real node id that is not a start and not adjacent to startId
    const orphanId = Object.keys(treeNodeInfo)
      .map(Number)
      .find((id) => !startSet.has(id) && !adjToStart.has(id))!

    const snap = makeSnapshot({ allocatedTreeNodes: new Set([startId, orphanId]) })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')

    expect(report.orphanedTreeNodes).toContain(orphanId)
    expect(snapshot.allocatedTreeNodes.has(orphanId)).toBe(false)
    expect(report.freedTreePoints).toBe(1)
    expect(report.hasChanges).toBe(true)
  })

  // B2a: treeSocketed — socket on unreachable/unknown node id is dropped into removedTreeSockets
  it('treeSocketed: socket on unknown node id is removed', () => {
    const startId = START_IDS[0]!
    const snap = makeSnapshot({
      allocatedTreeNodes: new Set([startId]),
      treeSocketed: {
        999999: { kind: 'item', id: gems[0]!.id },
      },
    })
    const { report } = convertSnapshotToActiveSeason(snap, 's10')

    expect(report.removedTreeSockets).toContain(999999)
    expect(report.hasChanges).toBe(true)
  })

  // B2b: treeSocketed — kind 'item' with unknown id on a reachable node is removed
  it('treeSocketed: kind item with unknown id on reachable node is removed', () => {
    const startId = START_IDS[0]!
    const snap = makeSnapshot({
      allocatedTreeNodes: new Set([startId]),
      treeSocketed: {
        [startId]: { kind: 'item', id: 'no_such_gem_or_item' },
      },
    })
    const { report } = convertSnapshotToActiveSeason(snap, 's10')

    expect(report.removedTreeSockets).toContain(startId)
    expect(report.hasChanges).toBe(true)
  })

  // B2c: treeSocketed — kind 'uncut' with one known + one unknown affix
  it('treeSocketed: kind uncut keeps known affixes and records dropped affixId in removedUncutAffixes', () => {
    const startId = START_IDS[0]!
    const knownAffixId = affixes[0]!.id
    const snap = makeSnapshot({
      allocatedTreeNodes: new Set([startId]),
      treeSocketed: {
        [startId]: {
          kind: 'uncut',
          affixes: [
            { affixId: knownAffixId, tier: 1, roll: 0.5 },
            { affixId: 'no_such_affix', tier: 1, roll: 0.5 },
          ],
        },
      },
    })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')

    const content = snapshot.treeSocketed[startId]
    expect(content?.kind).toBe('uncut')
    expect((content as { kind: 'uncut'; affixes: { affixId: string }[] }).affixes).toHaveLength(1)
    expect(report.removedUncutAffixes).toEqual([{ nodeId: startId, affixId: 'no_such_affix' }])
    expect(report.hasChanges).toBe(true)
  })

  // B3a: convertItem — forgedMods with unknown affix id is dropped
  it('convertItem: unknown forgedMod is recorded in removedForgedMods', () => {
    const snap = makeSnapshot({
      inventory: {
        weapon: {
          baseId: items[0]!.id,
          affixes: [],
          socketCount: 0,
          socketed: [],
          socketTypes: [],
          forgedMods: [
            { affixId: affixes[0]!.id, tier: 1, roll: 0.5 },
            { affixId: 'no_such_forged_mod', tier: 1, roll: 0.5 },
          ],
        },
      },
    })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')

    expect(snapshot.inventory.weapon?.forgedMods).toHaveLength(1)
    expect(report.removedForgedMods).toEqual([{ slot: 'weapon', affixId: 'no_such_forged_mod' }])
    expect(report.hasChanges).toBe(true)
  })

  // B3b: convertItem — socketed with unknown gem, null, and real gem
  it('convertItem: unknown socketable becomes null and is recorded in removedSocketables', () => {
    const snap = makeSnapshot({
      inventory: {
        weapon: {
          baseId: items[0]!.id,
          affixes: [],
          socketCount: 3,
          socketed: ['no_such_gem', null, gems[0]!.id],
          socketTypes: [],
        },
      },
    })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')

    expect(snapshot.inventory.weapon?.socketed).toEqual([null, null, gems[0]!.id])
    expect(report.removedSocketables).toEqual([{ slot: 'weapon', id: 'no_such_gem' }])
    expect(report.hasChanges).toBe(true)
  })

  // B3c: convertItem — unknown runewordId is cleared
  it('convertItem: unknown runewordId is cleared and recorded in removedRunewords', () => {
    const snap = makeSnapshot({
      inventory: {
        weapon: {
          baseId: items[0]!.id,
          affixes: [],
          socketCount: 0,
          socketed: [],
          socketTypes: [],
          runewordId: 'no_such_rw',
        },
      },
    })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')

    expect(snapshot.inventory.weapon?.runewordId).toBeUndefined()
    expect(report.removedRunewords).toEqual([{ slot: 'weapon', runewordId: 'no_such_rw' }])
    expect(report.hasChanges).toBe(true)
  })

  // B3d: convertItem — unknown augment id is cleared
  it('convertItem: unknown augment is cleared and recorded in removedAugments', () => {
    const snap = makeSnapshot({
      inventory: {
        weapon: {
          baseId: items[0]!.id,
          affixes: [],
          socketCount: 0,
          socketed: [],
          socketTypes: [],
          augment: { id: 'no_such_aug', level: 1 },
        },
      },
    })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')

    expect(snapshot.inventory.weapon?.augment).toBeUndefined()
    expect(report.removedAugments).toEqual([{ slot: 'weapon', id: 'no_such_aug' }])
    expect(report.hasChanges).toBe(true)
  })

  // B4a: activeAuraId unknown → cleared to null
  it('unknown activeAuraId is cleared to null', () => {
    const snap = makeSnapshot({ activeAuraId: 'no_such_aura' })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')

    expect(snapshot.activeAuraId).toBeNull()
    expect(report.hasChanges).toBe(true)
  })

  // B4b: subskillRanks with unknown id → filtered out + removedSubskills
  it('unknown subskill is filtered and recorded in removedSubskills', () => {
    const allSubskills = skills.flatMap((s) => (s.subskills ?? []).map((n) => n.id))
    const knownSubskillId = allSubskills[0]!
    const snap = makeSnapshot({
      subskillRanks: { [knownSubskillId]: 2, no_such_subskill: 1 },
    })
    const { snapshot, report } = convertSnapshotToActiveSeason(snap, 's10')

    expect(snapshot.subskillRanks[knownSubskillId]).toBe(2)
    expect(snapshot.subskillRanks['no_such_subskill']).toBeUndefined()
    expect(report.removedSubskills).toContain('no_such_subskill')
  })
})
