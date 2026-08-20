import { describe, expect, it } from 'vitest'
import fixture from './parity-fixture.json'
import {
  applyEtherTreePatch,
  applyGameConfigPatch,
  applyIncarnationTreePatch,
  applyListPatch,
  applyMercDataPatch,
  applyRecordMergePatch,
} from './resolve'
import type { IncarnationTree } from '../../frontend/types'

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

describe('applyIncarnationTreePatch', () => {
  const base: IncarnationTree = {
    viewBox: '0 0 100 100',
    nodes: [
      { id: 0, x: 10, y: 10, r: 8, t: 'root', icon: 'A_spr_0' },
      { id: 2, x: 20, y: 20, r: 7, t: 'small', icon: 'B_spr_0' },
      { id: 4, x: 30, y: 30, r: 10, t: 'big', icon: 'C_spr_0' },
    ],
    edges: [
      [0, 2],
      [2, 4],
    ],
  }

  it('returns base when patch is undefined', () => {
    expect(applyIncarnationTreePatch(base, undefined, 'tree').data).toBe(base)
  })

  it('removes a node together with its edges', () => {
    const r = applyIncarnationTreePatch(base, { removeNodes: [4] }, 'tree')
    expect(r.errors).toEqual([])
    expect(r.data.nodes.map((n) => n.id)).toEqual([0, 2])
    expect(r.data.edges).toEqual([[0, 2]])
  })

  it('merges changed node fields and keeps the id', () => {
    const r = applyIncarnationTreePatch(
      base,
      { changeNodes: { '2': { x: 25, y: 25, t: 'big' } } },
      'tree',
    )
    expect(r.errors).toEqual([])
    expect(r.data.nodes.find((n) => n.id === 2)).toEqual({
      id: 2,
      x: 25,
      y: 25,
      r: 7,
      t: 'big',
      icon: 'B_spr_0',
    })
  })

  it('adds node and edge by id pair', () => {
    const r = applyIncarnationTreePatch(
      base,
      {
        addNodes: [{ id: 6, x: 40, y: 40, r: 7, t: 'small', icon: 'D_spr_0' }],
        addEdges: [[4, 6]],
      },
      'tree',
    )
    expect(r.errors).toEqual([])
    expect(r.data.nodes.map((n) => n.id)).toEqual([0, 2, 4, 6])
    expect(r.data.edges).toContainEqual([4, 6])
  })

  it('removes an edge by id pair regardless of direction', () => {
    const r = applyIncarnationTreePatch(base, { removeEdges: [[2, 0]] }, 'tree')
    expect(r.errors).toEqual([])
    expect(r.data.edges).toEqual([[2, 4]])
  })

  it('replaces the whole tree (remove-all + add-all), swapping viewBox', () => {
    const r = applyIncarnationTreePatch(
      base,
      {
        viewBox: '0 0 500 500',
        removeNodes: [0, 2, 4],
        addNodes: [
          { id: 0, x: 100, y: 100, r: 8, t: 'root', icon: 'N_spr_0' },
          { id: 1, x: 120, y: 100, r: 7, t: 'small', icon: 'M_spr_0' },
        ],
        removeEdges: [
          [0, 2],
          [2, 4],
        ],
        addEdges: [[0, 1]],
      },
      'tree',
    )
    expect(r.errors).toEqual([])
    expect(r.data.viewBox).toBe('0 0 500 500')
    expect(r.data.nodes.map((n) => n.id)).toEqual([0, 1])
    expect(r.data.edges).toEqual([[0, 1]])
  })

  it('reports invalid operations', () => {
    const r = applyIncarnationTreePatch(
      base,
      {
        removeNodes: [999],
        changeNodes: { '888': { x: 1 } },
        addNodes: [{ id: 0, x: 1, y: 1, r: 1, t: 'small', icon: 'X' }],
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
})

describe('applyEtherTreePatch', () => {
  const etherBase = {
    viewBox: '0 0 100 100',
    nodes: [
      { id: 1, x: 10, y: 10, r: 8, t: 'root' as const, icon: 'A_spr_0', key: 'etherA' },
      { id: 2, x: 20, y: 10, r: 8, t: 'small' as const, icon: 'B_spr_0', key: 'etherB' },
      { id: 3, x: 30, y: 10, r: 10, t: 'big' as const, icon: 'C_spr_0', key: 'etherC' },
    ],
    edges: [[1, 2], [2, 3]] as [number, number][],
    stats: {
      etherA: { label: 'A', value: '1%', desc: 'a' },
      etherB: { label: 'B', value: '1%', desc: 'b' },
      etherC: { label: 'C', value: '10%', desc: 'c' },
    },
  }

  it('returns base unchanged when patch is undefined', () => {
    const r = applyEtherTreePatch(etherBase, undefined, 'ether')
    expect(r.data).toBe(etherBase)
    expect(r.errors).toEqual([])
  })

  it('changes stat values via stats patch', () => {
    const r = applyEtherTreePatch(
      etherBase,
      { stats: { change: { etherB: { value: '2%' } } } },
      'ether',
    )
    expect(r.errors).toEqual([])
    expect(r.data.stats.etherB).toEqual({ label: 'B', value: '2%', desc: 'b' })
  })

  it('adds, changes and removes nodes and edges', () => {
    const r = applyEtherTreePatch(
      etherBase,
      {
        addNodes: [
          { id: 4, x: 40, y: 10, r: 8, t: 'small', icon: 'B_spr_0', key: 'etherB' },
        ],
        changeNodes: { '2': { key: 'etherC', r: 10 } },
        removeNodes: [3],
        addEdges: [[2, 4]],
        removeEdges: [[2, 3]],
      },
      'ether',
    )
    expect(r.errors).toEqual([])
    expect(r.data.nodes.map((n) => n.id).sort()).toEqual([1, 2, 4])
    expect(r.data.nodes.find((n) => n.id === 2)).toMatchObject({
      key: 'etherC',
      r: 10,
      icon: 'B_spr_0',
    })
    expect(r.data.edges).toEqual([[1, 2], [2, 4]])
  })

  it('drops edges whose endpoint was removed', () => {
    const r = applyEtherTreePatch(etherBase, { removeNodes: [3] }, 'ether')
    expect(r.errors).toEqual([])
    expect(r.data.edges).toEqual([[1, 2]])
  })

  it('reports unknown ids, duplicate adds and missing stat keys', () => {
    const r = applyEtherTreePatch(
      etherBase,
      {
        removeNodes: [99],
        changeNodes: { '98': { r: 9 }, '2': { key: 'etherMissing' } },
        addNodes: [
          { id: 1, x: 5, y: 5, r: 8, t: 'small', icon: 'A_spr_0', key: 'etherA' },
        ],
        addEdges: [[1, 77]],
        removeEdges: [[1, 3]],
      },
      'ether',
    )
    expect(r.errors).toContain('ether: removeNodes unknown id 99')
    expect(r.errors).toContain('ether: changeNodes unknown id 98')
    expect(r.errors).toContain('ether: addNodes duplicates id 1')
    expect(r.errors).toContain('ether: addEdges endpoint unknown (1, 77)')
    expect(r.errors).toContain('ether: removeEdges unknown edge (1, 3)')
    expect(r.errors).toContain(
      'ether: node 2 references missing stat key "etherMissing"',
    )
  })
})

describe('applyMercDataPatch', () => {
  const mercBase = {
    maxSkillRank: 20,
    slots: ['helmet', 'weapon'],
    classes: [
      {
        id: 'merc_knight',
        name: 'Knight',
        role: 'Melee',
        location: 'Act 1',
        skills: [
          {
            id: 'taunt',
            name: 'Taunt',
            kind: 'active' as const,
            damageType: null,
            shared: false,
            description: 'old',
          },
        ],
      },
    ],
  }

  it('returns base unchanged when patch is undefined', () => {
    const r = applyMercDataPatch(mercBase, undefined, 'mercenaries')
    expect(r.data).toBe(mercBase)
    expect(r.errors).toEqual([])
  })

  it('changes top-level fields and merges class fields', () => {
    const r = applyMercDataPatch(
      mercBase,
      {
        change: { maxSkillRank: 25 },
        classes: { change: { merc_knight: { location: 'Act 2' } } },
      },
      'mercenaries',
    )
    expect(r.errors).toEqual([])
    expect(r.data.maxSkillRank).toBe(25)
    expect(r.data.classes[0]).toMatchObject({ location: 'Act 2', name: 'Knight' })
  })

  it('reports unknown class ids', () => {
    const r = applyMercDataPatch(
      mercBase,
      { classes: { change: { merc_bard: { name: 'Bard' } } } },
      'mercenaries',
    )
    expect(r.errors).toEqual(['mercenaries.classes: change unknown id "merc_bard"'])
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
