import type {
  GameConfigPatch,
  HeroSiegeTree,
  ListPatch,
  RawTreeNode,
  RecordPatch,
  ScalarRecordPatch,
  TreePatch,
} from './patchTypes'

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

function applyRecordPatchCore<T>(
  base: Record<string, T>,
  patch: {
    add?: Record<string, unknown>
    change?: Record<string, unknown>
    remove?: string[]
  },
  label: string,
  applyChange: (cur: T | undefined, fields: unknown) => T,
): PatchResult<Record<string, T>> {
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
    out[id] = applyChange(out[id], fields)
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

export function applyRecordMergePatch<T extends object>(
  base: Record<string, T>,
  patch: RecordPatch<Record<string, unknown>> | undefined,
  label: string,
): PatchResult<Record<string, T>> {
  if (!patch) return { data: base, errors: [] }
  return applyRecordPatchCore(
    base,
    patch,
    label,
    (cur, fields) => ({ ...cur, ...(fields as Partial<T>) }) as T,
  )
}

export function applyRecordReplacePatch<T>(
  base: Record<string, T>,
  patch: ScalarRecordPatch<T> | undefined,
  label: string,
): PatchResult<Record<string, T>> {
  if (!patch) return { data: base, errors: [] }
  return applyRecordPatchCore(base, patch, label, (_cur, value) => value as T)
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

export function posKey(x: number, y: number): string {
  return `${Math.round(x * 10)}_${Math.round(y * 10)}`
}

function edgeKey(a: number, b: number): string {
  return a < b ? `${a}_${b}` : `${b}_${a}`
}

export function applyTreePatch(
  base: HeroSiegeTree,
  patch: TreePatch | undefined,
  label: string,
): PatchResult<HeroSiegeTree> {
  if (!patch) return { data: base, errors: [] }
  const errors: string[] = []

  const idByPos = new Map<string, number>()
  for (const [id, x, y] of base.nodes) idByPos.set(posKey(x, y), id)

  const edges = new Map<string, [number, number]>()
  for (const [x1, y1, x2, y2] of base.edges) {
    const a = idByPos.get(posKey(x1, y1))
    const b = idByPos.get(posKey(x2, y2))
    if (a == null || b == null) {
      errors.push(`${label}: base edge does not resolve (${x1}, ${y1})-(${x2}, ${y2})`)
      continue
    }
    if (a === b) continue
    edges.set(edgeKey(a, b), [a, b])
  }

  const nodes = new Map<number, RawTreeNode>(base.nodes.map((n) => [n[0], n]))
  for (const id of patch.removeNodes ?? []) {
    if (!nodes.delete(id)) errors.push(`${label}: removeNodes unknown id ${id}`)
  }
  for (const [idStr, [x, y, r]] of Object.entries(patch.changeNodes ?? {})) {
    const id = Number(idStr)
    if (!nodes.has(id)) {
      errors.push(`${label}: changeNodes unknown id ${idStr}`)
      continue
    }
    nodes.set(id, [id, x, y, r])
  }
  for (const n of patch.addNodes ?? []) {
    if (nodes.has(n[0])) {
      errors.push(`${label}: addNodes duplicates id ${n[0]}`)
      continue
    }
    nodes.set(n[0], n)
  }

  const seenPos = new Map<string, number>()
  for (const [id, x, y] of nodes.values()) {
    const k = posKey(x, y)
    const other = seenPos.get(k)
    if (other !== undefined) {
      errors.push(`${label}: nodes ${other} and ${id} collide at position ${k}`)
      continue
    }
    seenPos.set(k, id)
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
    edges.set(edgeKey(a, b), [a, b])
  }

  const outEdges: HeroSiegeTree['edges'] = []
  for (const [a, b] of edges.values()) {
    const na = nodes.get(a)
    const nb = nodes.get(b)
    if (!na || !nb) continue
    outEdges.push([na[1], na[2], nb[1], nb[2]])
  }

  return {
    data: { viewBox: base.viewBox, nodes: [...nodes.values()], edges: outEdges },
    errors,
  }
}
