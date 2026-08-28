import type {
  EtherTreePatch,
  GameConfigPatch,
  IncarnationTreePatch,
  ListPatch,
  MercDataPatch,
  RecordPatch,
  SpatialTreePatch,
} from './patchTypes'
import type {
  EtherNode,
  EtherTree,
  IncarnationNode,
  IncarnationTree,
  MercData,
} from '../../frontend/types'

export interface PatchResult<T> {
  data: T
  errors: string[]
}

export function applyListPatch<T extends object>(
  base: T[],
  patch: ListPatch<Record<string, unknown>> | undefined,
  label: string,
  key = 'id',
): PatchResult<T[]> {
  if (!patch) return { data: base, errors: [] }
  const errors: string[] = []
  const byKey = new Map<string, T>(
    base.map((e) => [String((e as Record<string, unknown>)[key]), e]),
  )
  for (const id of patch.remove ?? []) {
    if (!byKey.delete(id)) errors.push(`${label}: remove unknown id "${id}"`)
  }
  for (const [id, fields] of Object.entries(patch.change ?? {})) {
    const cur = byKey.get(id)
    if (!cur) {
      errors.push(`${label}: change unknown id "${id}"`)
      continue
    }
    byKey.set(id, { ...cur, ...fields } as T)
  }
  for (const entry of patch.add ?? []) {
    const id = String(entry[key])
    if (byKey.has(id)) {
      errors.push(`${label}: add duplicates id "${id}"`)
      continue
    }
    byKey.set(id, entry as unknown as T)
  }
  return { data: [...byKey.values()], errors }
}

// Values are whole lists, so change replaces instead of merging fields.
export function applyStringListRecordPatch(
  base: Record<string, string[]>,
  patch: RecordPatch<string[]> | undefined,
  label: string,
): PatchResult<Record<string, string[]>> {
  if (!patch) return { data: base, errors: [] }
  const errors: string[] = []
  const out: Record<string, string[]> = { ...base }
  for (const id of patch.remove ?? []) {
    if (!(id in out)) {
      errors.push(`${label}: remove unknown id "${id}"`)
      continue
    }
    delete out[id]
  }
  for (const [id, value] of Object.entries(patch.change ?? {})) {
    if (!(id in out)) {
      errors.push(`${label}: change unknown id "${id}"`)
      continue
    }
    out[id] = value as string[]
  }
  for (const [id, value] of Object.entries(patch.add ?? {})) {
    if (id in out) {
      errors.push(`${label}: add duplicates id "${id}"`)
      continue
    }
    out[id] = value as string[]
  }
  return { data: out, errors }
}

export function applyRecordMergePatch<T extends object>(
  base: Record<string, T>,
  patch: RecordPatch<Record<string, unknown>> | undefined,
  label: string,
): PatchResult<Record<string, T>> {
  if (!patch) return { data: base, errors: [] }
  const errors: string[] = []
  const out: Record<string, T> = { ...base }
  for (const id of patch.remove ?? []) {
    if (!(id in out)) {
      errors.push(`${label}: remove unknown id "${id}"`)
      continue
    }
    delete out[id]
  }
  for (const [id, fields] of Object.entries(patch.change ?? {})) {
    if (!(id in out)) {
      errors.push(`${label}: change unknown id "${id}"`)
      continue
    }
    out[id] = { ...out[id], ...(fields as Partial<T>) } as T
  }
  for (const [id, value] of Object.entries(patch.add ?? {})) {
    if (id in out) {
      errors.push(`${label}: add duplicates id "${id}"`)
      continue
    }
    out[id] = value as T
  }
  return { data: out, errors }
}

export function applyGameConfigPatch<T extends object>(
  base: T,
  patch: GameConfigPatch | undefined,
  label: string,
): PatchResult<T> {
  if (!patch) return { data: base, errors: [] }
  const errors: string[] = []
  let out: T = { ...base, ...(patch.change ?? {}) }
  if (patch.stats) {
    const baseStats = (base as Record<string, unknown>).stats
    const stats = Array.isArray(baseStats)
      ? (baseStats as Record<string, unknown>[])
      : []
    const r = applyListPatch(stats, patch.stats, `${label}.stats`, 'key')
    errors.push(...r.errors)
    out = { ...out, stats: r.data }
  }
  return { data: out, errors }
}

function edgeKey(a: number, b: number): string {
  return a < b ? `${a}_${b}` : `${b}_${a}`
}

interface SpatialTree<N> {
  viewBox: string
  nodes: N[]
  edges: [number, number][]
}

function applySpatialTreePatch<N extends { id: number }>(
  base: SpatialTree<N>,
  patch: SpatialTreePatch<N>,
  label: string,
  errors: string[],
): SpatialTree<N> {
  const nodes = new Map<number, N>(base.nodes.map((n) => [n.id, n]))
  for (const id of patch.removeNodes ?? []) {
    if (!nodes.delete(id)) errors.push(`${label}: removeNodes unknown id ${id}`)
  }
  for (const [idStr, fields] of Object.entries(patch.changeNodes ?? {})) {
    const id = Number(idStr)
    const cur = nodes.get(id)
    if (!cur) {
      errors.push(`${label}: changeNodes unknown id ${idStr}`)
      continue
    }
    nodes.set(id, { ...cur, ...fields, id })
  }
  for (const n of patch.addNodes ?? []) {
    if (nodes.has(n.id)) {
      errors.push(`${label}: addNodes duplicates id ${n.id}`)
      continue
    }
    nodes.set(n.id, n)
  }

  const edges = new Map<string, [number, number]>()
  for (const [a, b] of base.edges) {
    if (a === b) continue
    edges.set(edgeKey(a, b), a < b ? [a, b] : [b, a])
  }
  for (const [a, b] of patch.removeEdges ?? []) {
    if (!edges.delete(edgeKey(a, b))) {
      errors.push(`${label}: removeEdges unknown edge (${a}, ${b})`)
    }
  }
  for (const [a, b] of patch.addEdges ?? []) {
    if (!nodes.has(a) || !nodes.has(b)) {
      errors.push(`${label}: addEdges endpoint unknown (${a}, ${b})`)
      continue
    }
    if (edges.has(edgeKey(a, b))) {
      errors.push(`${label}: addEdges duplicates edge (${a}, ${b})`)
      continue
    }
    edges.set(edgeKey(a, b), a < b ? [a, b] : [b, a])
  }

  const outEdges: [number, number][] = []
  for (const [a, b] of edges.values()) {
    if (!nodes.has(a) || !nodes.has(b)) continue
    outEdges.push([a, b])
  }

  return {
    viewBox: patch.viewBox ?? base.viewBox,
    nodes: [...nodes.values()],
    edges: outEdges,
  }
}

export function applyIncarnationTreePatch(
  base: IncarnationTree,
  patch: IncarnationTreePatch | undefined,
  label: string,
): PatchResult<IncarnationTree> {
  if (!patch) return { data: base, errors: [] }
  const errors: string[] = []
  const data = applySpatialTreePatch<IncarnationNode>(base, patch, label, errors)
  return { data, errors }
}

export function applyEtherTreePatch(
  base: EtherTree,
  patch: EtherTreePatch | undefined,
  label: string,
): PatchResult<EtherTree> {
  if (!patch) return { data: base, errors: [] }
  const errors: string[] = []
  const tree = applySpatialTreePatch<EtherNode>(base, patch, label, errors)

  const statsResult = applyRecordMergePatch(
    base.stats,
    patch.stats,
    `${label}.stats`,
  )
  errors.push(...statsResult.errors)
  const stats = statsResult.data

  for (const n of tree.nodes) {
    if (!stats[n.key]) {
      errors.push(`${label}: node ${n.id} references missing stat key "${n.key}"`)
    }
  }

  return { data: { ...tree, stats }, errors }
}

export function applyMercDataPatch(
  base: MercData,
  patch: MercDataPatch | undefined,
  label: string,
): PatchResult<MercData> {
  if (!patch) return { data: base, errors: [] }
  const errors: string[] = []
  let out: MercData = { ...base, ...(patch.change ?? {}) } as MercData
  if (patch.classes) {
    const r = applyListPatch(base.classes, patch.classes, `${label}.classes`)
    errors.push(...r.errors)
    out = { ...out, classes: r.data }
  }
  return { data: out, errors }
}
