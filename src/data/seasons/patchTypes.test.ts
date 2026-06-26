import { describe, expect, it } from 'vitest'
import {
  gameConfigPatchSchema,
  listPatchSchema,
  recordPatchSchema,
  scalarRecordPatchSchema,
  treePatchSchema,
} from './patchTypes'

describe('patch schemas', () => {
  it('accepts a minimal list patch', () => {
    const r = listPatchSchema.safeParse({
      add: [{ id: 'x', name: 'X' }],
      change: { y: { name: 'Y2' } },
      remove: ['z'],
    })
    expect(r.success).toBe(true)
  })

  it('rejects unknown top-level keys (strict)', () => {
    expect(listPatchSchema.safeParse({ patch: [] }).success).toBe(false)
  })

  it('accepts empty object for every schema', () => {
    for (const s of [
      listPatchSchema,
      recordPatchSchema,
      scalarRecordPatchSchema,
      treePatchSchema,
      gameConfigPatchSchema,
    ]) {
      expect(s.safeParse({}).success).toBe(true)
    }
  })

  it('validates tree patch tuples', () => {
    expect(
      treePatchSchema.safeParse({
        addNodes: [[9000, 100, 200, 7]],
        changeNodes: { '4': [2078, 1940.4, 10] },
        removeNodes: [2],
        addEdges: [[9000, 0]],
        removeEdges: [[0, 4]],
      }).success,
    ).toBe(true)
    expect(treePatchSchema.safeParse({ addNodes: [[1, 2]] }).success).toBe(false)
  })

  it('validates game-config patch with stats list patch', () => {
    expect(
      gameConfigPatchSchema.safeParse({
        change: { maxCharacterLevel: 110 },
        stats: { add: [{ key: 'new_stat', name: 'New', category: 'base', format: 'flat' }] },
      }).success,
    ).toBe(true)
  })

  it('record patch accepts object values under add/change', () => {
    expect(
      recordPatchSchema.safeParse({
        add: { '14': { t: 'X', n: 'normal', l: [] } },
        change: { '10': { l: ['+8 to Strength'] } },
      }).success,
    ).toBe(true)
  })

  it('scalar record patch rejects non-string values', () => {
    expect(scalarRecordPatchSchema.safeParse({ add: { icon: 42 } }).success).toBe(false)
    expect(scalarRecordPatchSchema.safeParse({ change: { '0': 'icon_b' } }).success).toBe(true)
  })
})
