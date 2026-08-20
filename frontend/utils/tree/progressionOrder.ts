import type { NodeAdjacency } from './treeGraph'

export function progressionOrder(
  allocated: Iterable<number>,
  startSet: ReadonlySet<number>,
  adj: NodeAdjacency,
): number[] {
  const remaining = [...allocated]
  const result: number[] = []
  const taken = new Set<number>()

  const isEligible = (id: number): boolean => {
    if (startSet.has(id)) return true
    const nbrs = adj.get(id)
    if (!nbrs) return false
    for (const nb of nbrs) if (taken.has(nb)) return true
    return false
  }

  while (remaining.length > 0) {
    const idx = remaining.findIndex(isEligible)
    if (idx < 0) break
    const [id] = remaining.splice(idx, 1)
    taken.add(id!)
    result.push(id!)
  }
  return [...result, ...remaining]
}

export function preserveOrder(
  reference: Iterable<number>,
  members: ReadonlySet<number>,
): Set<number> {
  const ordered = new Set<number>()
  for (const id of reference) {
    if (members.has(id)) ordered.add(id)
  }
  for (const id of members) ordered.add(id)
  return ordered
}
