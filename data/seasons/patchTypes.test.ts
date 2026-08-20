import { describe, expect, it } from 'vitest'
import {
  etherTreePatchSchema,
  gameConfigPatchSchema,
  incarnationTreePatchSchema,
  listPatchSchema,
  recordPatchSchema,
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
      incarnationTreePatchSchema,
      etherTreePatchSchema,
      gameConfigPatchSchema,
    ]) {
      expect(s.safeParse({}).success).toBe(true)
    }
  })

  it('validates incarnation tree patch nodes and edges', () => {
    expect(
      incarnationTreePatchSchema.safeParse({
        viewBox: '0 0 100 100',
        addNodes: [{ id: 9000, x: 100, y: 200, r: 7, t: 'small', icon: 'X_spr_0' }],
        changeNodes: { '4': { x: 10, y: 20 } },
        removeNodes: [2],
        addEdges: [[9000, 0]],
        removeEdges: [[0, 4]],
      }).success,
    ).toBe(true)
    expect(
      incarnationTreePatchSchema.safeParse({ addNodes: [{ id: 1, x: 2 }] }).success,
    ).toBe(false)
    expect(
      incarnationTreePatchSchema.safeParse({
        addNodes: [{ id: 1, x: 0, y: 0, r: 7, t: 'small', icon: 'X', key: 'k' }],
      }).success,
    ).toBe(false)
  })

  it('ether tree patch requires the stat key on added nodes', () => {
    expect(
      etherTreePatchSchema.safeParse({
        addNodes: [{ id: 1, x: 0, y: 0, r: 7, t: 'small', icon: 'X' }],
      }).success,
    ).toBe(false)
    expect(
      etherTreePatchSchema.safeParse({
        addNodes: [{ id: 1, x: 0, y: 0, r: 7, t: 'small', icon: 'X', key: 'k' }],
      }).success,
    ).toBe(true)
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
})
