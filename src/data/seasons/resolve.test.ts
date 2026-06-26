import { describe, expect, it } from 'vitest'
import fixture from './parity-fixture.json'
import {
  applyGameConfigPatch,
  applyListPatch,
  applyRecordMergePatch,
  applyRecordReplacePatch,
  applyTreePatch,
} from './resolve'
import type { HeroSiegeTree } from './patchTypes'

describe('applyListPatch', () => {
  const base = [
    { id: 'a', v: 1, nested: { x: 1 } },
    { id: 'b', v: 2 },
  ]

  it('returns base unchanged when patch is undefined', () => {
    const r = applyListPatch(base, undefined, 'test')
    expect(r.data).toBe(base)
    expect(r.errors).toEqual([])
  })

  it('applies remove, change, add preserving order', () => {
    const r = applyListPatch(
      base,
      {
        remove: ['b'],
        change: { a: { v: 10 } },
        add: [{ id: 'c', v: 3 }],
      },
      'test',
    )
    expect(r.errors).toEqual([])
    expect(r.data).toEqual([{ id: 'a', v: 10, nested: { x: 1 } }, { id: 'c', v: 3 }])
  })

  it('shallow-overrides nested fields as a whole', () => {
    const r = applyListPatch(base, { change: { a: { nested: { x: 9 } } } }, 'test')
    expect(r.data[0]).toEqual({ id: 'a', v: 1, nested: { x: 9 } })
  })

  it('does not mutate base', () => {
    applyListPatch(base, { change: { a: { v: 99 } } }, 'test')
    expect(base[0].v).toBe(1)
  })

  it('reports unknown remove/change and duplicate add', () => {
    const r = applyListPatch(
      base,
      { remove: ['zz'], change: { yy: { v: 0 } }, add: [{ id: 'a', v: 0 }] },
      'aff',
    )
    expect(r.errors).toEqual([
      'aff: remove unknown id "zz"',
      'aff: change unknown id "yy"',
      'aff: add duplicates id "a"',
    ])
  })

  it('supports a custom key', () => {
    const byName = [{ name: 'Orb', power: 1 }]
    const r = applyListPatch(byName, { change: { Orb: { power: 2 } } }, 'igs', 'name')
    expect(r.data).toEqual([{ name: 'Orb', power: 2 }])
  })
})

describe('applyRecordMergePatch', () => {
  const base = { '1': { t: 'A', n: 'normal', l: ['+1% X'] } }

  it('merges change shallowly and replaces arrays whole', () => {
    const r = applyRecordMergePatch(base, { change: { '1': { l: ['+2% X'] } } }, 'tn')
    expect(r.data['1']).toEqual({ t: 'A', n: 'normal', l: ['+2% X'] })
  })

  it('add/remove with validation', () => {
    const r = applyRecordMergePatch(
      base,
      { add: { '2': { t: 'B', n: 'big', l: [] } }, remove: ['1'] },
      'tn',
    )
    expect(r.errors).toEqual([])
    expect(Object.keys(r.data)).toEqual(['2'])
    const bad = applyRecordMergePatch(base, { add: { '1': { t: 'B', n: 'big', l: [] } } }, 'tn')
    expect(bad.errors).toEqual(['tn: add duplicates id "1"'])
  })
})

describe('applyRecordReplacePatch', () => {
  it('replaces scalar values', () => {
    const r = applyRecordReplacePatch({ '0': 'icon_a' }, { change: { '0': 'icon_b' } }, 'ni')
    expect(r.data).toEqual({ '0': 'icon_b' })
  })

  it('reports unknown remove/change and duplicate add', () => {
    const r = applyRecordReplacePatch(
      { '0': 'icon_a' },
      { remove: ['9'], change: { '8': 'x' }, add: { '0': 'y' } },
      'ni',
    )
    expect(r.errors).toEqual([
      'ni: remove unknown id "9"',
      'ni: change unknown id "8"',
      'ni: add duplicates id "0"',
    ])
  })
})

describe('applyGameConfigPatch', () => {
  const base = {
    maxCharacterLevel: 100,
    stats: [{ key: 'all_skills', name: 'to All Skills', category: 'base', format: 'flat' }],
  }

  it('overrides top-level scalars and patches stats by key', () => {
    const r = applyGameConfigPatch(
      base,
      {
        change: { maxCharacterLevel: 110 },
        stats: { add: [{ key: 'corruption', name: 'Corruption', category: 'base', format: 'flat' }] },
      },
      'gc',
    )
    expect(r.errors).toEqual([])
    expect(r.data.maxCharacterLevel).toBe(110)
    expect((r.data.stats as unknown[]).length).toBe(2)
  })

  it('stats list patch wins over a stats key in change', () => {
    const r = applyGameConfigPatch(
      base,
      {
        change: { stats: [] },
        stats: { change: { all_skills: { name: 'to ALL Skills' } } },
      },
      'gc',
    )
    expect(r.errors).toEqual([])
    expect((r.data.stats as { name: string }[])[0]?.name).toBe('to ALL Skills')
  })

  it('falls back to empty stats list when base has none', () => {
    const r = applyGameConfigPatch(
      { maxCharacterLevel: 100 },
      { stats: { add: [{ key: 'x', name: 'X' }] } },
      'gc',
    )
    expect(r.errors).toEqual([])
    expect((r.data as { stats: unknown[] }).stats).toHaveLength(1)
  })
})

describe('applyTreePatch', () => {
  const base: HeroSiegeTree = {
    viewBox: '0 0 100 100',
    nodes: [
      [0, 10, 10, 7],
      [2, 20, 20, 6],
      [4, 30, 30, 6],
    ],
    edges: [
      [10, 10, 20, 20],
      [10, 10, 20, 20],
      [20, 20, 30, 30],
    ],
  }

  it('returns base when patch is undefined', () => {
    expect(applyTreePatch(base, undefined, 'tree').data).toBe(base)
  })

  it('removes a node together with its edges', () => {
    const r = applyTreePatch(base, { removeNodes: [4] }, 'tree')
    expect(r.errors).toEqual([])
    expect(r.data.nodes.map((n) => n[0])).toEqual([0, 2])
    expect(r.data.edges).toEqual([[10, 10, 20, 20]])
  })

  it('moving a node keeps its edges attached at new coords', () => {
    const r = applyTreePatch(base, { changeNodes: { '2': [25, 25, 6] } }, 'tree')
    expect(r.errors).toEqual([])
    expect(r.data.edges).toEqual([
      [10, 10, 25, 25],
      [25, 25, 30, 30],
    ])
  })

  it('adds node and edge by id pair', () => {
    const r = applyTreePatch(
      base,
      { addNodes: [[6, 40, 40, 6]], addEdges: [[4, 6]] },
      'tree',
    )
    expect(r.errors).toEqual([])
    expect(r.data.nodes.map((n) => n[0])).toEqual([0, 2, 4, 6])
    expect(r.data.edges).toContainEqual([30, 30, 40, 40])
  })

  it('removes an edge by id pair regardless of direction', () => {
    const r = applyTreePatch(base, { removeEdges: [[2, 0]] }, 'tree')
    expect(r.errors).toEqual([])
    expect(r.data.edges).toEqual([[20, 20, 30, 30]])
  })

  it('reports invalid operations', () => {
    const r = applyTreePatch(
      base,
      {
        removeNodes: [999],
        changeNodes: { '888': [1, 1, 1] },
        addNodes: [[0, 1, 1, 1]],
        addEdges: [[0, 777]],
        removeEdges: [[0, 4]],
      },
      'tree',
    )
    expect(r.errors).toEqual([
      'tree: removeNodes unknown id 999',
      'tree: changeNodes unknown id 888',
      'tree: addNodes duplicates id 0',
      'tree: removeEdges unknown edge (0, 4)',
      'tree: addEdges endpoint unknown (0, 777)',
    ])
  })

  it('reports base edges that do not resolve to nodes', () => {
    const broken: HeroSiegeTree = { ...base, edges: [[1, 1, 2, 2]] }
    const r = applyTreePatch(broken, { removeNodes: [4] }, 'tree')
    expect(r.errors).toEqual(['tree: base edge does not resolve (1, 1)-(2, 2)'])
  })

  it('reports node position collisions introduced by a patch', () => {
    const r = applyTreePatch(base, { changeNodes: { '2': [10.01, 10.01, 6] } }, 'tree')
    expect(r.errors).toEqual(['tree: nodes 0 and 2 collide at position 100_100'])
  })
})

describe('parity fixture (contract shared with Rust)', () => {
  it('list case matches expected', () => {
    const { base, patch, expected } = fixture.list
    const r = applyListPatch(base, patch, 'list')
    expect(r.errors).toEqual([])
    expect(r.data).toEqual(expected)
  })

  it('record case matches expected', () => {
    const { base, patch, expected } = fixture.record
    const r = applyRecordMergePatch(base, patch, 'record')
    expect(r.errors).toEqual([])
    expect(r.data).toEqual(expected)
  })

  it('game config case matches expected', () => {
    const { base, patch, expected } = fixture.gameConfig
    const r = applyGameConfigPatch(base, patch, 'gameConfig')
    expect(r.errors).toEqual([])
    expect(r.data).toEqual(expected)
  })
})
