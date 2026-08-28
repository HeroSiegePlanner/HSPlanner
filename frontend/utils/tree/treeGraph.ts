import { incarnationTree } from '@data'

const adj = new Map<number, Set<number>>()
for (const n of incarnationTree.nodes) adj.set(n.id, new Set())
for (const [a, b] of incarnationTree.edges) {
  if (a === b) continue
  const setA = adj.get(a)
  const setB = adj.get(b)
  if (!setA || !setB) continue
  setA.add(b)
  setB.add(a)
}

export const ADJ = adj

export const START_IDS: ReadonlyArray<number> = incarnationTree.nodes
  .filter((n) => n.t === 'root')
  .map((n) => n.id)
  .sort((a, b) => a - b)
export const START_SET: ReadonlySet<number> = new Set(START_IDS)

export type NodeAdjacency = ReadonlyMap<number, ReadonlySet<number>>

export function findPath(
  sources: Iterable<number>,
  target: number,
  adj: NodeAdjacency = ADJ,
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
    const nbrs = adj.get(cur)
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

// The node's own contribution: no path in, no orphan cleanup out.
export function withToggled(allocated: ReadonlySet<number>, id: number): Set<number> {
  const next = new Set(allocated)
  if (!next.delete(id)) next.add(id)
  return next
}

export function reachableFromAny(
  starts: Iterable<number>,
  allowed: Set<number>,
  adj: NodeAdjacency = ADJ,
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
    const nbrs = adj.get(cur)
    if (!nbrs) continue
    for (const nb of nbrs) {
      if (seen.has(nb) || !allowed.has(nb)) continue
      seen.add(nb)
      queue.push(nb)
    }
  }
  return seen
}
