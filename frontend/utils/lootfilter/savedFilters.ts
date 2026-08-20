import type { SavedLootFilter } from '../../types'
import {
  newId,
  readStorage,
  StorageCapacityError,
  StorageWriteError,
  writeStorage,
} from '../storage'
import { nextDuplicateName } from '../build/savedBuilds'
import { createDefaultLootFilter, decodeLootFilter, encodeLootFilter } from './codec'

const STORAGE_KEY = 'hsplanner.lootFilters.v1'

const MAX_FILTERS = 200
const MAX_NAME_LENGTH = 200
const MAX_CODE_LENGTH = 200_000

interface FilterLibrary {
  version: 1
  filters: SavedLootFilter[]
}

function emptyLibrary(): FilterLibrary {
  return { version: 1, filters: [] }
}

function cleanFilter(entry: unknown, now: string): SavedLootFilter | null {
  if (!entry || typeof entry !== 'object') return null
  const f = entry as Record<string, unknown>
  if (
    typeof f.id !== 'string' ||
    f.id.length > MAX_NAME_LENGTH ||
    typeof f.buildId !== 'string' ||
    f.buildId.length > MAX_NAME_LENGTH ||
    typeof f.name !== 'string' ||
    typeof f.code !== 'string' ||
    f.code.length > MAX_CODE_LENGTH
  ) {
    return null
  }
  return {
    id: f.id,
    buildId: f.buildId,
    name: f.name.slice(0, MAX_NAME_LENGTH),
    code: f.code,
    favorite: f.favorite === true,
    createdAt: typeof f.createdAt === 'string' ? f.createdAt : now,
    updatedAt: typeof f.updatedAt === 'string' ? f.updatedAt : now,
  }
}

function readFilterLibrary(): FilterLibrary {
  const raw = readStorage(STORAGE_KEY)
  if (!raw) return emptyLibrary()
  try {
    const parsed: unknown = JSON.parse(raw)
    if (
      !parsed ||
      typeof parsed !== 'object' ||
      !Array.isArray((parsed as { filters?: unknown }).filters)
    ) {
      return emptyLibrary()
    }
    const now = new Date().toISOString()
    const filters: SavedLootFilter[] = []
    for (const entry of (parsed as { filters: unknown[] }).filters.slice(0, MAX_FILTERS)) {
      const filter = cleanFilter(entry, now)
      if (filter) filters.push(filter)
    }
    return { version: 1, filters }
  } catch {
    return emptyLibrary()
  }
}

function writeFilterLibrary(library: FilterLibrary): void {
  if (!writeStorage(STORAGE_KEY, JSON.stringify(library))) {
    throw new StorageWriteError()
  }
}

export function listSavedFilters(buildId: string): SavedLootFilter[] {
  return readFilterLibrary()
    .filters.filter((f) => f.buildId === buildId)
    .sort((a, b) => {
      if (a.favorite !== b.favorite) return a.favorite ? -1 : 1
      return b.updatedAt.localeCompare(a.updatedAt)
    })
}

export function getSavedFilter(id: string): SavedLootFilter | null {
  return readFilterLibrary().filters.find((f) => f.id === id) ?? null
}

export function createFilter(
  buildId: string,
  name: string,
  code: string = encodeLootFilter(createDefaultLootFilter()),
): SavedLootFilter {
  return appendFilter(readFilterLibrary(), buildId, name, code)
}

function appendFilter(
  library: FilterLibrary,
  buildId: string,
  name: string,
  code: string,
): SavedLootFilter {
  if (library.filters.length >= MAX_FILTERS) {
    throw new StorageCapacityError(
      `Filter limit reached (${MAX_FILTERS}). Delete an existing filter first.`,
    )
  }
  const now = new Date().toISOString()
  const record: SavedLootFilter = {
    id: newId('lf'),
    buildId,
    name: name.slice(0, MAX_NAME_LENGTH),
    code,
    favorite: false,
    createdAt: now,
    updatedAt: now,
  }
  writeFilterLibrary({ ...library, filters: [...library.filters, record] })
  return record
}

export function importFilter(
  buildId: string,
  name: string,
  code: string,
): SavedLootFilter | null {
  const trimmed = code.trim()
  if (!decodeLootFilter(trimmed)) return null
  return createFilter(buildId, name, trimmed)
}

function updateFilter(
  id: string,
  update: (filter: SavedLootFilter, now: string) => SavedLootFilter,
): SavedLootFilter | null {
  const library = readFilterLibrary()
  const existing = library.filters.find((f) => f.id === id)
  if (!existing) return null
  const now = new Date().toISOString()
  const updated = update(existing, now)
  writeFilterLibrary({
    ...library,
    filters: library.filters.map((f) => (f.id === id ? updated : f)),
  })
  return updated
}

export function updateFilterCode(id: string, code: string): SavedLootFilter | null {
  return updateFilter(id, (f, now) => ({ ...f, code, updatedAt: now }))
}

export function setFilterFavorite(
  id: string,
  favorite: boolean,
): SavedLootFilter | null {
  return updateFilter(id, (f) => ({ ...f, favorite }))
}

export function renameFilter(id: string, name: string): SavedLootFilter | null {
  return updateFilter(id, (f, now) => ({
    ...f,
    name: name.slice(0, MAX_NAME_LENGTH),
    updatedAt: now,
  }))
}

export function duplicateFilter(id: string): SavedLootFilter | null {
  const library = readFilterLibrary()
  const src = library.filters.find((f) => f.id === id)
  if (!src) return null
  const siblings = library.filters
    .filter((f) => f.buildId === src.buildId)
    .map((f) => f.name)
  return appendFilter(library, src.buildId, nextDuplicateName(src.name, siblings), src.code)
}

export function deleteFilter(id: string): void {
  const library = readFilterLibrary()
  writeFilterLibrary({
    ...library,
    filters: library.filters.filter((f) => f.id !== id),
  })
}
