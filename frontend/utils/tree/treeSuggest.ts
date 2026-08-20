import { incarnationTree } from '@data'
import { TREE_NODE_INFO, TREE_WARP_IDS } from './treeStats'

const NODE_RADIUS = new Map<number, number>(
  incarnationTree.nodes.map((n) => [n.id, n.r]),
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
  incarnationTree.nodes.map((n) => n.id).filter(isValuableNode),
)
