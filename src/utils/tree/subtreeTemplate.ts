import type { SubskillRole } from '../../types'

export interface TemplateNode {
  index: number
  x: number
  y: number
  role: SubskillRole
  defaultMaxRank: number
  edges: number[]
}

interface NodeDef {
  x: number
  y: number
  role: SubskillRole
  maxRank: number
  edges: number[]
}

const defs: NodeDef[] = [
  { x: 0.500, y: 0.870, role: 'keystone', maxRank: 1, edges: [1, 2] },
  { x: 0.453, y: 0.724, role: 'minor', maxRank: 5, edges: [0, 4]},
  { x: 0.547, y: 0.724, role: 'minor', maxRank: 5, edges: [0, 6]},
  { x: 0.253, y: 0.579, role: 'minor', maxRank: 5, edges: [4, 11] },
  { x: 0.406, y: 0.579, role: 'minor', maxRank: 5, edges: [1, 3, 5, 8] },
  { x: 0.500, y: 0.579, role: 'minor', maxRank: 5, edges: [4, 6] },
  { x: 0.594, y: 0.579, role: 'minor', maxRank: 5, edges: [2, 5, 7, 9] },
  { x: 0.747, y: 0.579, role: 'minor', maxRank: 5, edges: [6, 14] },
  { x: 0.347, y: 0.399, role: 'minor', maxRank: 5, edges: [4, 10, 11, 12] },
  { x: 0.653, y: 0.399, role: 'minor', maxRank: 5, edges: [6, 10, 13, 14] },
  { x: 0.500, y: 0.289, role: 'minor', maxRank: 5, edges: [8, 9, 12, 13] },
  { x: 0.100, y: 0.579, role: 'notable', maxRank: 1, edges: [3, 8] },
  { x: 0.253, y: 0.109, role: 'notable', maxRank: 1, edges: [8, 10] },
  { x: 0.747, y: 0.109, role: 'notable', maxRank: 1, edges: [9, 10] },
  { x: 0.900, y: 0.579, role: 'notable', maxRank: 1, edges: [7, 9] },
]

export const SUBTREE_TEMPLATE: TemplateNode[] = defs.map((n, i) => ({
  index: i,
  x: n.x,
  y: n.y,
  role: n.role,
  defaultMaxRank: n.maxRank,
  edges: n.edges,
}))

export function getTemplateEdges(): Array<[number, number]> {
  const seen = new Set<string>()
  const out: Array<[number, number]> = []
  for (const n of SUBTREE_TEMPLATE) {
    for (const t of n.edges) {
      const key = n.index < t ? `${n.index}-${t}` : `${t}-${n.index}`
      if (seen.has(key)) continue
      seen.add(key)
      out.push([n.index, t])
    }
  }
  return out
}
