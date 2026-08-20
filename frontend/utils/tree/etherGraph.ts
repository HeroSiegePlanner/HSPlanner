import { etherTree } from '@data'
import type { EtherNode } from '../../types'
import type { NodeAdjacency } from './treeGraph'

export const ETHER_NODE_BY_ID = new Map<number, EtherNode>(
  etherTree.nodes.map((n) => [n.id, n]),
)

const adj = new Map<number, Set<number>>()
for (const n of etherTree.nodes) adj.set(n.id, new Set())
for (const [a, b] of etherTree.edges) {
  if (a === b) continue
  const setA = adj.get(a)
  const setB = adj.get(b)
  if (!setA || !setB) continue
  setA.add(b)
  setB.add(a)
}

export const ETHER_ADJ: NodeAdjacency = adj

export const ETHER_START_IDS: ReadonlyArray<number> = etherTree.nodes
  .filter((n) => n.t === 'root')
  .map((n) => n.id)
  .sort((a, b) => a - b)

export const ETHER_START_SET: ReadonlySet<number> = new Set(ETHER_START_IDS)
