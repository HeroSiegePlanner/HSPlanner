import { heroSiegeTree, treeNodeInfo } from '../../data'
import { posKey } from '../../data/seasons/resolve'

type RawNode = [id: number, x: number, y: number, r: number]
type RawEdge = [x1: number, y1: number, x2: number, y2: number]

const NODES = heroSiegeTree.nodes as RawNode[]
const EDGES = heroSiegeTree.edges as RawEdge[]

const POS_TO_ID = new Map<string, number>()
for (const [id, x, y] of NODES) {
  POS_TO_ID.set(posKey(x, y), id)
}

export const ADJ = new Map<number, Set<number>>()
for (const [id] of NODES) ADJ.set(id, new Set())

const treeGraphWarnings: string[] = []
for (const [x1, y1, x2, y2] of EDGES) {
  const a = POS_TO_ID.get(posKey(x1, y1))
  const b = POS_TO_ID.get(posKey(x2, y2))
  if (a == null || b == null) {
    treeGraphWarnings.push(
      `tree edge does not resolve to nodes: (${x1}, ${y1})-(${x2}, ${y2})`,
    )
    continue
  }
  if (a === b) continue
  ADJ.get(a)!.add(b)
  ADJ.get(b)!.add(a)
}
if (import.meta.env.DEV && treeGraphWarnings.length > 0) {
  console.warn('[treeGraph]', treeGraphWarnings)
}

export const START_IDS: ReadonlyArray<number> = Object.entries(treeNodeInfo)
  .filter(([, info]) => info.n === 'root')
  .map(([id]) => Number(id))
  .sort((a, b) => a - b)
export const START_SET: ReadonlySet<number> = new Set(START_IDS)

export function findPath(
  sources: Iterable<number>,
  target: number,
): number[] | null {
  const srcSet = new Set(sources)
  if (srcSet.has(target)) return [target]
  if (srcSet.size === 0) return null

  const parent = new Map<number, number>()
  const queue: number[] = []
  let head = 0
  for (const s of srcSet) {
    parent.set(s, -1)
    queue.push(s)
  }

  while (head < queue.length) {
    const cur = queue[head++]!
    const nbrs = ADJ.get(cur)
    if (!nbrs) continue
    for (const nb of nbrs) {
      if (parent.has(nb)) continue
      parent.set(nb, cur)
      if (nb === target) {
        const path: number[] = []
        let n: number = nb
        while (n !== -1) {
          path.push(n)
          const p = parent.get(n)
          if (p === undefined) break
          n = p
        }
        return path.reverse()
      }
      queue.push(nb)
    }
  }
  return null
}

export function reachableFromAny(
  starts: Iterable<number>,
  allowed: Set<number>,
): Set<number> {
  const seen = new Set<number>()
  const queue: number[] = []
  let head = 0
  for (const s of starts) {
    if (allowed.has(s) && !seen.has(s)) {
      seen.add(s)
      queue.push(s)
    }
  }
  while (head < queue.length) {
    const cur = queue[head++]!
    const nbrs = ADJ.get(cur)
    if (!nbrs) continue
    for (const nb of nbrs) {
      if (seen.has(nb) || !allowed.has(nb)) continue
      seen.add(nb)
      queue.push(nb)
    }
  }
  return seen
}
