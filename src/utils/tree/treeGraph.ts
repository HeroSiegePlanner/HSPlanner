import treeData from '../../data/hero-siege-tree.json'

type RawNode = [id: number, x: number, y: number, r: number]
type RawEdge = [x1: number, y1: number, x2: number, y2: number]

const NODES = treeData.nodes as RawNode[]
const EDGES = treeData.edges as RawEdge[]

function posKey(x: number, y: number): string {
  // Builds a stable string key from a node's (x, y) coordinates rounded to one decimal so that floating-point edge endpoints reliably resolve back to a node id. Used during module load to construct the position-to-id index.
  return `${Math.round(x * 10)}_${Math.round(y * 10)}`
}

const POS_TO_ID = new Map<string, number>()
for (const [id, x, y] of NODES) {
  POS_TO_ID.set(posKey(x, y), id)
}

export const ADJ = new Map<number, Set<number>>()
for (const [id] of NODES) ADJ.set(id, new Set())

for (const [x1, y1, x2, y2] of EDGES) {
  const a = POS_TO_ID.get(posKey(x1, y1))
  const b = POS_TO_ID.get(posKey(x2, y2))
  if (a == null || b == null || a === b) continue
  ADJ.get(a)!.add(b)
  ADJ.get(b)!.add(a)
}

export const START_IDS: ReadonlyArray<number> = [0, 8, 16, 28, 42, 44, 46, 48]
export const START_SET: ReadonlySet<number> = new Set(START_IDS)

export function findPath(
  sources: Iterable<number>,
  target: number,
): number[] | null {
  // Performs a multi-source BFS over the talent-tree adjacency graph and returns the shortest sequence of node ids reaching `target`, or null when no path exists. Used by the TreeView to find the cheapest cluster of nodes to allocate when the user clicks a distant node.
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
  // BFS-style search that returns the set of nodes reachable from any starting node while only stepping through nodes that appear in `allowed`. Used by the TreeView to detect "orphaned" allocated nodes when the player removes a connecting node.
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
