import { describe, expect, it } from 'vitest'
import { etherTree } from '@data'
import {
  ETHER_ADJ,
  ETHER_NODE_BY_ID,
  ETHER_START_IDS,
} from './etherGraph'
import { findPath, reachableFromAny } from './treeGraph'

describe('etherGraph', () => {
  it('indexes every node and keeps edges symmetric', () => {
    expect(ETHER_NODE_BY_ID.size).toBe(etherTree.nodes.length)
    for (const [a, b] of etherTree.edges) {
      expect(ETHER_ADJ.get(a)?.has(b)).toBe(true)
      expect(ETHER_ADJ.get(b)?.has(a)).toBe(true)
    }
  })

  it('has root start nodes', () => {
    expect(ETHER_START_IDS.length).toBeGreaterThan(0)
    for (const id of ETHER_START_IDS) {
      expect(ETHER_NODE_BY_ID.get(id)?.t).toBe('root')
    }
  })

  it('reaches every node from the roots', () => {
    const all = new Set(
      etherTree.nodes
        .filter((n) => !n.key.endsWith('Indicator'))
        .map((n) => n.id),
    )
    const reachable = reachableFromAny(ETHER_START_IDS, all, ETHER_ADJ)
    expect(reachable.size).toBe(all.size)
  })

  it('finds an allocation path from roots to any node', () => {
    const far = etherTree.nodes.find((n) => n.t === 'big')
    expect(far).toBeDefined()
    const path = findPath(new Set(ETHER_START_IDS), far!.id, ETHER_ADJ)
    expect(path).not.toBeNull()
    expect(path![path!.length - 1]).toBe(far!.id)
    expect(ETHER_START_IDS).toContain(path![0])
    for (let i = 0; i < path!.length - 1; i++) {
      expect(ETHER_ADJ.get(path![i]!)?.has(path![i + 1]!)).toBe(true)
    }
  })

  it('prunes nodes disconnected from the roots', () => {
    const [rootId] = ETHER_START_IDS
    const neighbor = [...(ETHER_ADJ.get(rootId!) ?? [])][0]
    expect(neighbor).toBeDefined()
    const rootAdj = ETHER_ADJ.get(rootId!) ?? new Set()
    const neighborAdj = ETHER_ADJ.get(neighbor!) ?? new Set()
    const orphan = etherTree.nodes.find(
      (n) =>
        n.id !== rootId &&
        n.id !== neighbor &&
        !rootAdj.has(n.id) &&
        !neighborAdj.has(n.id),
    )
    expect(orphan).toBeDefined()
    const allowed = new Set([rootId!, neighbor!, orphan!.id])
    const reachable = reachableFromAny([rootId!], allowed, ETHER_ADJ)
    expect(reachable.has(rootId!)).toBe(true)
    expect(reachable.has(neighbor!)).toBe(true)
    expect(reachable.has(orphan!.id)).toBe(false)
  })
})
