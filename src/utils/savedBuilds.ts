import {
  type BuildSnapshot,
  decodeShareToBuild,
  encodeBuildToShare,
} from './shareBuild'
import { readStorageWithLegacy, writeStorage } from './storage'

const STORAGE_KEY_V1 = 'hsplanner.savedBuilds.v1'
const STORAGE_KEY = 'hsplanner.savedBuilds.v2'

const LEGACY_STORAGE_KEY_V1 = 'heroplanner.savedBuilds.v1'
const LEGACY_STORAGE_KEY = 'heroplanner.savedBuilds.v2'

export const DEFAULT_PROFILE_NAME = 'Default'

// Surfaced to the UI when the ~5 MB localStorage quota is exhausted instead of silently dropping the save.
export class StorageWriteError extends Error {
  constructor(message = 'Could not save to local storage — it may be full.') {
    super(message)
    this.name = 'StorageWriteError'
  }
}

const MAX_BUILDS = 1_000
const MAX_PROFILES_PER_BUILD = 100
const MAX_NAME_LENGTH = 500
const MAX_NOTES_LENGTH = 200_000
const MAX_CODE_LENGTH = 200_000

export interface SavedProfile {
  id: string
  name: string
  code: string
  updatedAt: string
}

export interface SavedBuild {
  id: string
  name: string
  classId: string | null
  notes: string
  createdAt: string
  updatedAt: string
  profiles: SavedProfile[]
  activeProfileId: string
}

interface SavedBuildV1 {
  id: string
  name: string
  classId: string | null
  level: number
  createdAt: string
  updatedAt: string
  code: string
}

function newId(prefix: string): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID()
  }
  return `${prefix}_${Date.now().toString(36)}${Math.random()
    .toString(36)
    .slice(2, 8)}`
}

function readV1(): SavedBuildV1[] {
  const raw = readStorageWithLegacy(STORAGE_KEY_V1, LEGACY_STORAGE_KEY_V1)
  if (!raw) return []
  try {
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed.filter(
      (b) =>
        b &&
        typeof b.id === 'string' &&
        typeof b.name === 'string' &&
        typeof b.code === 'string',
    )
  } catch {
    return []
  }
}

function migrateV1(list: SavedBuildV1[]): SavedBuild[] {
  return list.map((b) => {
    const profileId = newId('p')
    const profile: SavedProfile = {
      id: profileId,
      name: DEFAULT_PROFILE_NAME,
      code: b.code,
      updatedAt: b.updatedAt,
    }
    return {
      id: b.id,
      name: b.name,
      classId: b.classId,
      notes: '',
      createdAt: b.createdAt,
      updatedAt: b.updatedAt,
      profiles: [profile],
      activeProfileId: profileId,
    }
  })
}

function isSavedProfile(p: unknown): p is SavedProfile {
  if (!p || typeof p !== 'object') return false
  const candidate = p as Partial<SavedProfile>
  return (
    typeof candidate.id === 'string' &&
    candidate.id.length <= MAX_NAME_LENGTH &&
    typeof candidate.name === 'string' &&
    candidate.name.length <= MAX_NAME_LENGTH &&
    typeof candidate.code === 'string' &&
    candidate.code.length <= MAX_CODE_LENGTH
  )
}

function clampString(s: unknown, max: number, fallback = ''): string {
  return typeof s === 'string' ? s.slice(0, max) : fallback
}

function read(): SavedBuild[] {
  const raw = readStorageWithLegacy(STORAGE_KEY, LEGACY_STORAGE_KEY)
  try {
    if (raw) {
      const parsed: unknown = JSON.parse(raw)
      if (!Array.isArray(parsed)) return []
      const cleaned: SavedBuild[] = []
      const now = new Date().toISOString()
      for (const entry of parsed.slice(0, MAX_BUILDS)) {
        if (!entry || typeof entry !== 'object') continue
        const b = entry as Record<string, unknown>
        if (
          typeof b.id !== 'string' ||
          b.id.length > MAX_NAME_LENGTH ||
          typeof b.name !== 'string' ||
          b.name.length > MAX_NAME_LENGTH
        ) {
          continue
        }
        const profiles: SavedProfile[] = Array.isArray(b.profiles)
          ? (b.profiles as unknown[])
              .filter(isSavedProfile)
              .slice(0, MAX_PROFILES_PER_BUILD)
              .map((p) => ({
                id: p.id,
                name: p.name.slice(0, MAX_NAME_LENGTH),
                code: p.code,
                updatedAt: clampString(p.updatedAt, MAX_NAME_LENGTH, now),
              }))
          : []
        if (profiles.length === 0) continue
        const activeProfileId =
          typeof b.activeProfileId === 'string' &&
          profiles.some((p) => p.id === b.activeProfileId)
            ? b.activeProfileId
            : profiles[0]!.id
        cleaned.push({
          id: b.id,
          name: b.name,
          classId:
            typeof b.classId === 'string'
              ? b.classId.slice(0, MAX_NAME_LENGTH)
              : null,
          notes: clampString(b.notes, MAX_NOTES_LENGTH),
          createdAt: clampString(b.createdAt, MAX_NAME_LENGTH, now),
          updatedAt: clampString(b.updatedAt, MAX_NAME_LENGTH, now),
          profiles,
          activeProfileId,
        })
      }
      return cleaned
    }
    const v1 = readV1()
    if (v1.length === 0) return []
    const migrated = migrateV1(v1)
    try {
      write(migrated)
    } catch {
      // Best-effort: throwing here would make every read look like the entire library was wiped.
    }
    return migrated
  } catch {
    return []
  }
}

function write(list: SavedBuild[]): void {
  if (!writeStorage(STORAGE_KEY, JSON.stringify(list))) {
    throw new StorageWriteError()
  }
}

export function listSavedBuilds(): SavedBuild[] {
  return read().sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
}

export function getSavedBuild(id: string): SavedBuild | null {
  return read().find((b) => b.id === id) ?? null
}

export function getActiveProfile(b: SavedBuild): SavedProfile | null {
  return (
    b.profiles.find((p) => p.id === b.activeProfileId) ?? b.profiles[0] ?? null
  )
}

export function createBuild(
  name: string,
  snapshot: BuildSnapshot,
  profileName: string = DEFAULT_PROFILE_NAME,
  notes: string = '',
): SavedBuild {
  const list = read()
  const now = new Date().toISOString()
  const profileId = newId('p')
  const code = encodeBuildToShare(snapshot)
  const record: SavedBuild = {
    id: newId('b'),
    name,
    classId: snapshot.classId,
    notes,
    createdAt: now,
    updatedAt: now,
    profiles: [
      {
        id: profileId,
        name: profileName,
        code,
        updatedAt: now,
      },
    ],
    activeProfileId: profileId,
  }
  list.push(record)
  write(list)
  return record
}

export function setBuildNotes(
  buildId: string,
  notes: string,
): SavedBuild | null {
  const list = read()
  const build = list.find((b) => b.id === buildId)
  if (!build) return null
  build.notes = notes
  build.updatedAt = new Date().toISOString()
  write(list)
  return build
}

export function commitProfileSnapshot(
  buildId: string,
  profileId: string,
  snapshot: BuildSnapshot,
): SavedBuild | null {
  const list = read()
  const build = list.find((b) => b.id === buildId)
  if (!build) return null
  const profile = build.profiles.find((p) => p.id === profileId)
  if (!profile) return null
  const now = new Date().toISOString()
  profile.code = encodeBuildToShare(snapshot)
  profile.updatedAt = now
  build.classId = snapshot.classId
  build.updatedAt = now
  write(list)
  return build
}

export function renameBuild(
  buildId: string,
  name: string,
): SavedBuild | null {
  const list = read()
  const build = list.find((b) => b.id === buildId)
  if (!build) return null
  build.name = name
  build.updatedAt = new Date().toISOString()
  write(list)
  return build
}

export function deleteBuild(buildId: string): void {
  const list = read().filter((b) => b.id !== buildId)
  write(list)
}

export function setActiveProfile(
  buildId: string,
  profileId: string,
): SavedBuild | null {
  const list = read()
  const build = list.find((b) => b.id === buildId)
  if (!build) return null
  if (!build.profiles.some((p) => p.id === profileId)) return null
  build.activeProfileId = profileId
  build.updatedAt = new Date().toISOString()
  write(list)
  return build
}

export function addProfile(
  buildId: string,
  name: string,
  snapshot: BuildSnapshot,
  options: { activate?: boolean } = { activate: true },
): { build: SavedBuild; profile: SavedProfile } | null {
  const list = read()
  const build = list.find((b) => b.id === buildId)
  if (!build) return null
  const now = new Date().toISOString()
  const profile: SavedProfile = {
    id: newId('p'),
    name,
    code: encodeBuildToShare(snapshot),
    updatedAt: now,
  }
  build.profiles.push(profile)
  if (options.activate ?? true) build.activeProfileId = profile.id
  build.updatedAt = now
  write(list)
  return { build, profile }
}

export function duplicateProfile(
  buildId: string,
  profileId: string,
): { build: SavedBuild; profile: SavedProfile } | null {
  const list = read()
  const build = list.find((b) => b.id === buildId)
  if (!build) return null
  const src = build.profiles.find((p) => p.id === profileId)
  if (!src) return null
  const now = new Date().toISOString()
  const profile: SavedProfile = {
    id: newId('p'),
    name: nextDuplicateName(
      src.name,
      build.profiles.map((p) => p.name),
    ),
    code: src.code,
    updatedAt: now,
  }
  build.profiles.push(profile)
  build.activeProfileId = profile.id
  build.updatedAt = now
  write(list)
  return { build, profile }
}

export function renameProfile(
  buildId: string,
  profileId: string,
  name: string,
): SavedBuild | null {
  const list = read()
  const build = list.find((b) => b.id === buildId)
  if (!build) return null
  const profile = build.profiles.find((p) => p.id === profileId)
  if (!profile) return null
  profile.name = name
  profile.updatedAt = new Date().toISOString()
  build.updatedAt = profile.updatedAt
  write(list)
  return build
}

// Refuses to remove the last surviving profile; reassigns activeProfileId to a neighbour when the deleted one was active.
export function removeProfile(
  buildId: string,
  profileId: string,
): SavedBuild | null {
  const list = read()
  const build = list.find((b) => b.id === buildId)
  if (!build) return null
  if (build.profiles.length <= 1) return null
  const idx = build.profiles.findIndex((p) => p.id === profileId)
  if (idx === -1) return null
  build.profiles.splice(idx, 1)
  if (build.activeProfileId === profileId) {
    const fallback = build.profiles[Math.max(0, idx - 1)]
    if (!fallback) return null
    build.activeProfileId = fallback.id
  }
  build.updatedAt = new Date().toISOString()
  write(list)
  return build
}

export function loadProfileSnapshot(
  buildId: string,
  profileId: string,
): BuildSnapshot | null {
  const build = getSavedBuild(buildId)
  if (!build) return null
  const profile = build.profiles.find((p) => p.id === profileId)
  if (!profile) return null
  return decodeShareToBuild(profile.code)?.snapshot ?? null
}

// Falls back to a timestamped name after fifty "(copy N)" collisions.
function nextDuplicateName(base: string, taken: string[]): string {
  const cleanBase = base.replace(/\s+\(copy(?:\s+\d+)?\)$/i, '')
  const candidates = [
    `${cleanBase} (copy)`,
    ...Array.from({ length: 50 }, (_, i) => `${cleanBase} (copy ${i + 2})`),
  ]
  for (const c of candidates) {
    if (!taken.includes(c)) return c
  }
  return `${cleanBase} (copy ${Date.now()})`
}
