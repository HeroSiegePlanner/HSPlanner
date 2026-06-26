import { describe, expect, it } from 'vitest'
import { buildSeasonPatchSet } from './load'

describe('buildSeasonPatchSet', () => {
  it('returns empty set for empty modules map', () => {
    const r = buildSeasonPatchSet('s10', {})
    expect(r.patches).toEqual({})
    expect(r.errors).toEqual([])
  })

  it('maps filenames to collection keys, scoped to season', () => {
    const r = buildSeasonPatchSet('s10', {
      './s10/affixes.patch.json': { default: { change: { x: { valueMax: 5 } } } },
      './s10/tree-nodes.patch.json': { default: { remove: ['2'] } },
      './s9/affixes.patch.json': { default: { remove: ['ignored-other-season'] } },
    })
    expect(r.errors).toEqual([])
    expect(r.patches.affixes).toEqual({ change: { x: { valueMax: 5 } } })
    expect(r.patches.treeNodes).toEqual({ remove: ['2'] })
  })

  it('reports invalid patch shape and unknown filename', () => {
    const r = buildSeasonPatchSet('s10', {
      './s10/affixes.patch.json': { default: { bogus: true } },
      './s10/wat.patch.json': { default: {} },
    })
    expect(r.patches.affixes).toBeUndefined()
    expect(r.errors.some((e) => e.includes('affixes.patch.json'))).toBe(true)
    expect(r.errors.some((e) => e.includes('wat.patch.json'))).toBe(true)
  })
})
