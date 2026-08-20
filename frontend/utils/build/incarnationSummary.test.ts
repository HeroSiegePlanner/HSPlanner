import { describe, expect, it, vi } from 'vitest'
import type * as DataModule from '@data'
import type { EtherNodeType, IncarnationTree } from '../../types/ether'
import type { TreeNodeInfo } from '@data/seasons/patchTypes'
import type { TreeSocketContent } from '../../types'

const fx = vi.hoisted(() => {
  const n = (id: number, r: number, t: EtherNodeType) => ({ id, x: 0, y: 0, r, t, icon: '' })
  const nodes = [
    n(1, 12, 'big'),
    n(2, 10, 'big'),
    n(3, 10, 'big'),
    n(10, 4, 'small'),
    n(11, 4, 'small'),
    n(12, 4, 'small'),
    n(13, 4, 'small'),
    n(20, 4, 'root'),
    n(21, 4, 'small'),
    n(30, 4, 'small'),
    n(31, 4, 'small'),
    n(32, 4, 'small'),
    n(40, 4, 'small'),
  ]
  const info: Record<string, { t: string; n: string; l: string[] }> = {
    '1': { t: 'Big Keystone', n: 'big', l: ['+100 Life', '+50 Mana'] },
    '2': { t: 'First Notable', n: 'big', l: ['+10% Fire Damage', '+5% Cold Damage'] },
    '3': { t: 'Second Notable', n: 'big', l: ['+20% Lightning Damage'] },
    '10': { t: 'Minor Phys', n: 'normal', l: ['+8% Increased Physical Damage'] },
    '11': { t: 'Minor Phys', n: 'normal', l: ['+8% Increased Physical Damage'] },
    '12': { t: 'Minor Phys', n: 'normal', l: ['+8% Increased Physical Damage'] },
    '13': { t: 'Minor Move', n: 'normal', l: ['+3% Increased Movement Speed'] },
    '20': { t: 'Root', n: 'root', l: [] },
    '21': { t: 'Warp', n: 'warp', l: [] },
    '30': { t: 'Socket', n: 'jewelry', l: [] },
    '31': { t: 'Socket', n: 'jewelry', l: [] },
    '32': { t: 'Socket', n: 'jewelry', l: [] },
    '40': { t: 'Extra Minor', n: 'normal', l: ['+1% Increased Attack Speed'] },
  }
  return { nodes, info }
})

vi.mock('@data', async (importOriginal) => {
  const actual = await importOriginal<typeof DataModule>()
  return {
    ...actual,
    incarnationTree: {
      viewBox: '0 0 100 100',
      nodes: fx.nodes,
      edges: [],
    } as IncarnationTree,
    incarnationNodeInfo: fx.info as Record<string, TreeNodeInfo>,
  }
})

import { buildIncarnation } from './incarnationSummary'
import { affixes, getGem } from '@data'
import { formatValue, statName } from '../item/stats'
import { socketIconPath } from './assetPaths'

const TOTAL = fx.nodes.length
const NO_SOCKETS: Record<number, TreeSocketContent | null> = {}

describe('buildIncarnation', () => {
  it('buckets allocated big nodes into keystones (classifyTier) and notables', () => {
    const r = buildIncarnation(new Set([1, 2, 3]), NO_SOCKETS)

    expect(r.keystones).toEqual([{ name: 'Big Keystone', lines: ['+100 Life', '+50 Mana'] }])
    expect(r.notables).toEqual([
      { name: 'First Notable', line: '+10% Fire Damage · +5% Cold Damage' },
      { name: 'Second Notable', line: '+20% Lightning Damage' },
    ])
  })

  it('keystone carries info.l lines; notable joins them with " · "', () => {
    const r = buildIncarnation(new Set([1, 2]), NO_SOCKETS)

    expect(r.keystones[0]?.lines).toEqual(['+100 Life', '+50 Mana'])
    expect(r.notables[0]?.line).toBe('+10% Fire Damage · +5% Cold Damage')
  })

  it('aggregates identical minor stat lines into {text, count} sorted by count desc', () => {
    const r = buildIncarnation(new Set([10, 11, 12, 13]), NO_SOCKETS)

    expect(r.minors).toEqual([
      { text: '+8% Increased Physical Damage', count: 3 },
      { text: '+3% Increased Movement Speed', count: 1 },
    ])
  })

  it('excludes root and warp nodes from buckets but counts them in countLabel', () => {
    const r = buildIncarnation(new Set([1, 20, 21]), NO_SOCKETS)

    expect(r.keystones).toEqual([{ name: 'Big Keystone', lines: ['+100 Life', '+50 Mana'] }])
    expect(r.notables).toEqual([])
    expect(r.minors).toEqual([])
    expect(r.jewelry).toEqual([])
    expect(r.countLabel).toBe(`3 / ${TOTAL} nodes`)
    expect(r.summaryLabel).toBe('3 nodes · 1 keystones · 0 notables · 0 minors · 0 jewelry')
  })

  it('maps jewelry sockets: gem/rune name + stats + socketIconPath; uncut → "Uncut Jewel"; empty → "Empty socket"/"—"', () => {
    const gemId = 'gem_chipped_amethyst'
    const gem = getGem(gemId)!
    const uncutAffix = affixes.find(
      (a) => a.statKey && a.valueMin != null && a.valueMax != null,
    )!
    const treeSocketed: Record<number, TreeSocketContent | null> = {
      30: { kind: 'item', id: gemId },
      31: null,
      32: {
        kind: 'uncut',
        affixes: [{ affixId: uncutAffix.id, tier: uncutAffix.tier, roll: 0 }],
      },
    }

    const r = buildIncarnation(new Set([30, 31, 32]), treeSocketed)
    expect(r.jewelry).toHaveLength(3)

    const expectedItemLine = Object.entries(gem.stats)
      .filter(([, v]) => v !== 0)
      .map(([k, v]) => `${formatValue(v, k)} ${statName(k)}`)
      .join(' · ')
    expect(r.jewelry[0]).toEqual({
      name: gem.name,
      icon: socketIconPath(gem.name),
      line: expectedItemLine,
    })
    expect(r.jewelry[0]?.line).toContain('Additive Arcane Damage')

    expect(r.jewelry[1]).toEqual({ name: 'Empty socket', line: '—' })

    const expectedUncutLine = `${formatValue(
      [uncutAffix.valueMin!, uncutAffix.valueMax!],
      uncutAffix.statKey!,
    )} ${statName(uncutAffix.statKey!)}`
    expect(r.jewelry[2]).toEqual({ name: 'Uncut Jewel', line: expectedUncutLine })
  })

  it('builds countLabel "N / TOTAL nodes", tabLabel "N nodes" and summaryLabel with bucket counts', () => {
    const treeSocketed: Record<number, TreeSocketContent | null> = {
      30: { kind: 'item', id: 'gem_chipped_amethyst' },
    }
    const r = buildIncarnation(new Set([1, 2, 3, 10, 11, 13, 30, 20]), treeSocketed)

    expect(r.countLabel).toBe(`8 / ${TOTAL} nodes`)
    expect(r.tabLabel).toBe('8 nodes')
    expect(r.summaryLabel).toBe(
      '8 nodes · 1 keystones · 2 notables · 3 minors · 1 jewelry',
    )
  })

  it('does not emit branches', () => {
    const r = buildIncarnation(new Set([1]), NO_SOCKETS)
    expect(r.branches).toBeUndefined()
  })

  it('empty allocation → zeroed labels and empty buckets', () => {
    const r = buildIncarnation(new Set(), NO_SOCKETS)

    expect(r.countLabel).toBe(`0 / ${TOTAL} nodes`)
    expect(r.tabLabel).toBe('0 nodes')
    expect(r.keystones).toEqual([])
    expect(r.notables).toEqual([])
    expect(r.minors).toEqual([])
    expect(r.jewelry).toEqual([])
    expect(r.summaryLabel).toBe('0 nodes · 0 keystones · 0 notables · 0 minors · 0 jewelry')
    expect(r.branches).toBeUndefined()
  })
})
