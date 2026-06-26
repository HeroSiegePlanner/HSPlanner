import { heroSiegeTree } from '../../data'
import { TREE_NODE_INFO, TREE_WARP_IDS } from './treeStats'

type RawNode = [id: number, x: number, y: number, r: number]

const NODE_RADIUS = new Map<number, number>(
  (heroSiegeTree.nodes as RawNode[]).map(([id, , , r]) => [id, r]),
)

function isValuableNode(id: number): boolean {
  if (TREE_WARP_IDS.has(id)) return false
  const info = TREE_NODE_INFO[String(id)]
  if (!info) return false
  if (info.n === 'root') return false
  if (info.n === 'jewelry') return true
  if (info.n === 'big') return true
  return (NODE_RADIUS.get(id) ?? 0) >= 10
}

export const VALUABLE_NODE_IDS: Set<number> = new Set(
  (heroSiegeTree.nodes as RawNode[]).map(([id]) => id).filter(isValuableNode),
)
