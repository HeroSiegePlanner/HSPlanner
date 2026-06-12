import type {
  GameConfigPatch,
  ListPatch,
  RecordPatch,
  ScalarRecordPatch,
} from './patchTypes'

export interface PatchResult<T> {
  data: T
  errors: string[]
}

// Shared add/change/remove semantics for id-keyed collections; the Rust twin
// lives in src-tauri/src/calc/season.rs and is held to parity-fixture.json.
export function applyListPatch<T extends Record<string, unknown>>(
  base: T[],
  patch: ListPatch<T> | undefined,
  label: string,
  key = 'id',
): PatchResult<T[]> {
  if (!patch) return { data: base, errors: [] }
  const errors: string[] = []
  const byKey = new Map<string, T>(base.map((e) => [String(e[key]), e]))
  for (const id of patch.remove ?? []) {
    if (!byKey.delete(id)) errors.push(`${label}: remove unknown id "${id}"`)
  }
  for (const [id, fields] of Object.entries(patch.change ?? {})) {
    const cur = byKey.get(id)
    if (!cur) {
      errors.push(`${label}: change unknown id "${id}"`)
      continue
    }
    byKey.set(id, { ...cur, ...(fields as Partial<T>) })
  }
  for (const entry of patch.add ?? []) {
    const id = String(entry[key])
    if (byKey.has(id)) {
      errors.push(`${label}: add duplicates id "${id}"`)
      continue
    }
    byKey.set(id, entry)
  }
  return { data: [...byKey.values()], errors }
}

export function applyRecordMergePatch<T extends Record<string, unknown>>(
  base: Record<string, T>,
  patch: RecordPatch<T> | undefined,
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

export function applyRecordReplacePatch<T>(
  base: Record<string, T>,
  patch: ScalarRecordPatch<T> | undefined,
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
  for (const [id, value] of Object.entries(patch.change ?? {})) {
    if (!(id in out)) {
      errors.push(`${label}: change unknown id "${id}"`)
      continue
    }
    out[id] = value as T
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

export function applyGameConfigPatch<T extends Record<string, unknown>>(
  base: T,
  patch: GameConfigPatch | undefined,
  label: string,
): PatchResult<T> {
  if (!patch) return { data: base, errors: [] }
  const errors: string[] = []
  let out: T = { ...base, ...(patch.change ?? {}) }
  if (patch.stats) {
    const stats = Array.isArray(base.stats)
      ? (base.stats as Record<string, unknown>[])
      : []
    const r = applyListPatch(stats, patch.stats, `${label}.stats`, 'key')
    errors.push(...r.errors)
    out = { ...out, stats: r.data }
  }
  return { data: out, errors }
}
