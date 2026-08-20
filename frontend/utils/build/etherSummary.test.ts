import { describe, expect, it } from 'vitest'
import { etherTree } from '@data'
import {
  ETHER_MAGIC_FIND_KEY,
  ETHER_REGION_FALLBACK_COLOR,
  etherMagicFindTotal,
  etherRegionColor,
  etherRegionLabel,
  formatEtherTotal,
  groupEtherSummary,
  parseEtherValue,
  summarizeEtherNodes,
} from './etherSummary'

const nodesWithKey = (key: string) =>
  etherTree.nodes.filter((n) => n.key === key).map((n) => n.id)

describe('parseEtherValue', () => {
  it('parses percent values', () => {
    expect(parseEtherValue('1%')).toEqual({ num: 1, isPercent: true })
    expect(parseEtherValue('35%')).toEqual({ num: 35, isPercent: true })
  })

  it('parses flat values', () => {
    expect(parseEtherValue('2')).toEqual({ num: 2, isPercent: false })
  })

  it('returns zero for malformed values', () => {
    expect(parseEtherValue('abc').num).toBe(0)
  })
})

describe('summarizeEtherNodes', () => {
  it('returns empty for no allocation', () => {
    expect(summarizeEtherNodes([])).toEqual([])
  })

  it('counts multiple copies of the same notable and sums totals', () => {
    const mfNodes = nodesWithKey(ETHER_MAGIC_FIND_KEY)
    expect(mfNodes.length).toBeGreaterThanOrEqual(2)
    const summary = summarizeEtherNodes(mfNodes.slice(0, 2))
    expect(summary).toHaveLength(1)
    const entry = summary[0]!
    expect(entry.key).toBe(ETHER_MAGIC_FIND_KEY)
    expect(entry.count).toBe(2)
    expect(entry.total).toBe(entry.valuePer * 2)
    expect(entry.isPercent).toBe(true)
  })

  it('ignores unknown node ids', () => {
    expect(summarizeEtherNodes([999999])).toEqual([])
  })

  it('sorts by count descending', () => {
    const mfNodes = nodesWithKey(ETHER_MAGIC_FIND_KEY).slice(0, 2)
    const other = etherTree.nodes.find(
      (n) => n.key !== ETHER_MAGIC_FIND_KEY && n.t === 'small',
    )
    const summary = summarizeEtherNodes([...mfNodes, other!.id])
    expect(summary[0]!.key).toBe(ETHER_MAGIC_FIND_KEY)
  })
})

describe('etherMagicFindTotal', () => {
  it('sums only unconditional magic find nodes', () => {
    const mfNodes = nodesWithKey(ETHER_MAGIC_FIND_KEY)
    const other = etherTree.nodes.find((n) => n.key !== ETHER_MAGIC_FIND_KEY)
    const total = etherMagicFindTotal([...mfNodes.slice(0, 3), other!.id])
    const per = parseEtherValue(
      etherTree.stats[ETHER_MAGIC_FIND_KEY]!.value,
    ).num
    expect(total).toBe(per * 3)
  })

  it('is zero without allocation', () => {
    expect(etherMagicFindTotal([])).toBe(0)
  })
})

describe('formatEtherTotal', () => {
  it('formats percent and flat totals', () => {
    expect(formatEtherTotal({ total: 5, isPercent: true })).toBe('+5%')
    expect(formatEtherTotal({ total: 2, isPercent: false })).toBe('+2')
    expect(formatEtherTotal({ total: 2.5, isPercent: true })).toBe('+2.5%')
  })
})

describe('etherRegionLabel', () => {
  it('maps key prefixes to mechanic labels', () => {
    expect(etherRegionLabel('etherUnSmall01')).toBe('Universal')
    expect(etherRegionLabel('etherCtBig03')).toBe('Chaos Tower')
    expect(etherRegionLabel('etherUSSmall02')).toBe('Unholy Siege')
    expect(etherRegionLabel('etherSRBig05')).toBe('Shadow Realm')
    expect(etherRegionLabel('etherDngSmall04')).toBe('Dungeons')
  })

  it('falls back to Other for unknown keys', () => {
    expect(etherRegionLabel('somethingElse')).toBe('Other')
  })

  it('maps every key used by tree nodes to a known region', () => {
    for (const n of etherTree.nodes) {
      if (n.key.endsWith('Indicator')) continue
      expect(etherRegionLabel(n.key), n.key).not.toBe('Other')
    }
  })
})

describe('groupEtherSummary', () => {
  it('groups entries by region with Universal first', () => {
    const ids = [
      ...nodesWithKey(ETHER_MAGIC_FIND_KEY).slice(0, 1),
      ...nodesWithKey('etherCtSmall03').slice(0, 1),
      ...nodesWithKey('etherSRSmall01').slice(0, 1),
    ]
    const groups = groupEtherSummary(summarizeEtherNodes(ids))
    expect(groups[0]!.region).toBe('Universal')
    const regions = groups.map((g) => g.region)
    expect(regions).toContain('Chaos Tower')
    expect(regions).toContain('Shadow Realm')
    expect(regions.slice(1)).toEqual([...regions.slice(1)].sort())
  })

  it('carries the mechanic color on each group', () => {
    const ids = [
      ...nodesWithKey(ETHER_MAGIC_FIND_KEY).slice(0, 1),
      ...nodesWithKey('etherCtSmall03').slice(0, 1),
    ]
    const groups = groupEtherSummary(summarizeEtherNodes(ids))
    for (const group of groups) {
      expect(group.color).toBe(etherRegionColor(group.entries[0]!.key))
    }
  })
})

describe('etherRegionColor', () => {
  it('gives every mechanic used by tree nodes a distinct color', () => {
    const colors = new Map<string, string>()
    for (const n of etherTree.nodes) {
      if (n.key.endsWith('Indicator')) continue
      colors.set(etherRegionLabel(n.key), etherRegionColor(n.key))
    }
    expect(colors.size).toBeGreaterThanOrEqual(2)
    expect(new Set(colors.values()).size).toBe(colors.size)
    for (const color of colors.values()) {
      expect(color).not.toBe(ETHER_REGION_FALLBACK_COLOR)
    }
  })

  it('falls back to the neutral color for unknown keys', () => {
    expect(etherRegionColor('somethingElse')).toBe(ETHER_REGION_FALLBACK_COLOR)
  })
})
