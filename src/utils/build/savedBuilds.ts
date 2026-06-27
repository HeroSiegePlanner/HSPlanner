import {
  type BuildSnapshot,
  decodeShareToBuild,
  encodeBuildToShare,
} from './shareBuild'
import { readStorage, readStorageWithLegacy, writeStorage } from '../storage'
import { activeSeasonId } from '../../data'
import { LEGACY_SEASON_ID } from '../../data/seasons/registry'

const STORAGE_KEY_V1 = 'hsplanner.savedBuilds.v1'
const STORAGE_KEY_V2 = 'hsplanner.savedBuilds.v2'
const STORAGE_KEY = 'hsplanner.savedBuilds.v3'

const LEGACY_STORAGE_KEY_V1 = 'heroplanner.savedBuilds.v1'
const LEGACY_STORAGE_KEY_V2 = 'heroplanner.savedBuilds.v2'
const LEGACY_STORAGE_KEY = 'heroplanner.savedBuilds.v3'

const DEFAULT_PROFILE_NAME = 'Default'

export class StorageWriteError extends Error {
  constructor(message = 'Could not save to local storage — it may be full.') {
    super(message)
    this.name = 'StorageWriteError'
  }
}

export class StorageCapacityError extends StorageWriteError {
  constructor(message: string) {
    super(message)
    this.name = 'StorageCapacityError'
  }
}

const MAX_BUILDS = 1_000
const MAX_PROFILES_PER_BUILD = 100
const MAX_FOLDERS = 500
const MAX_NAME_LENGTH = 500
const MAX_NOTES_LENGTH = 200_000
const MAX_CODE_LENGTH = 200_000
const MAX_TAGS = 24
const MAX_TAG_LENGTH = 40

export interface SavedProfile {
  id: string
  name: string
  code: string
  updatedAt: string
}

export interface Folder {
  id: string
  name: string
  parentId: string | null
  createdAt: string
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
  folderId: string | null
  favorite: boolean
  tags: string[]
  season: string
}

export interface SavedLibrary {
  version: 3
  builds: SavedBuild[]
  folders: Folder[]
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

export function newId(prefix: string): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID()
  }
  return `${prefix}_${Date.now().toString(36)}${Math.random()
    .toString(36)
    .slice(2, 8)}`
}

function clampString(s: unknown, max: number, fallback = ''): string {
  return typeof s === 'string' ? s.slice(0, max) : fallback
}

function sanitizeTags(raw: unknown): string[] {
  if (!Array.isArray(raw)) return []
  const out: string[] = []
  const seen = new Set<string>()
  for (const entry of raw) {
    if (typeof entry !== 'string') continue
    const tag = entry.trim().slice(0, MAX_TAG_LENGTH)
    if (!tag) continue
    const key = tag.toLowerCase()
    if (seen.has(key)) continue
    seen.add(key)
    out.push(tag)
    if (out.length >= MAX_TAGS) break
  }
  return out
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

function sanitizeProfiles(raw: unknown, now: string): SavedProfile[] {
  if (!Array.isArray(raw)) return []
  return raw
    .filter(isSavedProfile)
    .slice(0, MAX_PROFILES_PER_BUILD)
    .map((p) => ({
      id: p.id,
      name: p.name.slice(0, MAX_NAME_LENGTH),
      code: p.code,
      updatedAt: clampString(p.updatedAt, MAX_NAME_LENGTH, now),
    }))
}

function cleanBuild(
  entry: unknown,
  validFolderIds: Set<string>,
  now: string,
): SavedBuild | null {
  if (!entry || typeof entry !== 'object') return null
  const b = entry as Record<string, unknown>
  if (
    typeof b.id !== 'string' ||
    b.id.length > MAX_NAME_LENGTH ||
    typeof b.name !== 'string' ||
    b.name.length > MAX_NAME_LENGTH
  ) {
    return null
  }
  const profiles = sanitizeProfiles(b.profiles, now)
  if (profiles.length === 0) return null
  const activeProfileId =
    typeof b.activeProfileId === 'string' &&
    profiles.some((p) => p.id === b.activeProfileId)
      ? b.activeProfileId
      : profiles[0]!.id
  const folderId =
    typeof b.folderId === 'string' && validFolderIds.has(b.folderId)
      ? b.folderId
      : null
  return {
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
    folderId,
    favorite: b.favorite === true,
    tags: sanitizeTags(b.tags),
    season:
      typeof (b as { season?: unknown }).season === 'string'
        ? (b as { season: string }).season
        : LEGACY_SEASON_ID,
  }
}

function cleanBuilds(
  raw: unknown[],
  validFolderIds: Set<string>,
): SavedBuild[] {
  const now = new Date().toISOString()
  const out: SavedBuild[] = []
  for (const entry of raw.slice(0, MAX_BUILDS)) {
    const build = cleanBuild(entry, validFolderIds, now)
    if (build) out.push(build)
  }
  return out
}

function sanitizeFolders(raw: unknown): Folder[] {
  if (!Array.isArray(raw)) return []
  const now = new Date().toISOString()
  const out: Folder[] = []
  const seen = new Set<string>()
  for (const entry of raw.slice(0, MAX_FOLDERS)) {
    if (!entry || typeof entry !== 'object') continue
    const f = entry as Record<string, unknown>
    if (
      typeof f.id !== 'string' ||
      f.id.length > MAX_NAME_LENGTH ||
      seen.has(f.id) ||
      typeof f.name !== 'string' ||
      f.name.length > MAX_NAME_LENGTH
    ) {
      continue
    }
    seen.add(f.id)
    out.push({
      id: f.id,
      name: f.name,
      parentId: typeof f.parentId === 'string' ? f.parentId : null,
      createdAt: clampString(f.createdAt, MAX_NAME_LENGTH, now),
    })
  }
  const ids = new Set(out.map((f) => f.id))
  for (const f of out) {
    if (f.parentId !== null && !ids.has(f.parentId)) f.parentId = null
  }
  const byId = new Map(out.map((f) => [f.id, f]))
  for (const f of out) {
    const visited = new Set<string>([f.id])
    let cur: Folder = f
    while (cur.parentId !== null) {
      if (visited.has(cur.parentId)) {
        cur.parentId = null
        break
      }
      visited.add(cur.parentId)
      const parent = byId.get(cur.parentId)
      if (!parent) {
        cur.parentId = null
        break
      }
      cur = parent
    }
  }
  return out
}

function emptyLibrary(): SavedLibrary {
  return { version: 3, builds: [], folders: [] }
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
      folderId: null,
      favorite: false,
      tags: [],
      season: LEGACY_SEASON_ID,
    }
  })
}

function readLegacyBuilds(): SavedBuild[] {
  const raw = readStorageWithLegacy(STORAGE_KEY_V2, LEGACY_STORAGE_KEY_V2)
  if (raw) {
    try {
      const parsed: unknown = JSON.parse(raw)
      if (Array.isArray(parsed)) return cleanBuilds(parsed, new Set())
    } catch {
      // Corrupt/non-array v2: fall through to the v1 migration below so a
      // damaged v2 key can't mask recoverable v1 data.
    }
  }
  const v1 = readV1()
  return v1.length > 0 ? migrateV1(v1) : []
}

export function readLibrary(): SavedLibrary {
  const raw = readStorageWithLegacy(STORAGE_KEY, LEGACY_STORAGE_KEY)
  if (raw) {
    try {
      const parsed: unknown = JSON.parse(raw)
      if (
        parsed &&
        typeof parsed === 'object' &&
        Array.isArray((parsed as { builds?: unknown }).builds)
      ) {
        const p = parsed as { builds: unknown[]; folders?: unknown }
        const folders = sanitizeFolders(p.folders)
        const validIds = new Set(folders.map((f) => f.id))
        return { version: 3, builds: cleanBuilds(p.builds, validIds), folders }
      }
    } catch {
      // fall through to the corrupt-data path below
    }
    // Back up the corrupt v3 blob before returning empty, otherwise the next
    // save overwrites the v3 key and destroys recoverable builds.
    backupCorruptLibrary(raw)
    return emptyLibrary()
  }
  const builds = readLegacyBuilds()
  const library: SavedLibrary = { version: 3, builds, folders: [] }
  if (builds.length > 0) {
    try {
      writeLibrary(library)
    } catch {
      // Best-effort migration: keep the in-memory library and retry persisting
      // on the next load instead of throwing (which would appear to wipe data).
    }
  }
  return library
}

function backupCorruptLibrary(raw: string): void {
  try {
    const key = `${STORAGE_KEY}.corrupt`
    if (!readStorage(key)) writeStorage(key, raw)
  } catch {
    // ignore — the backup is purely best-effort
  }
}

export function writeLibrary(library: SavedLibrary): void {
  if (!writeStorage(STORAGE_KEY, JSON.stringify(library))) {
    throw new StorageWriteError()
  }
}

export function listSavedBuilds(): SavedBuild[] {
  return readLibrary().builds.sort((a, b) =>
    b.updatedAt.localeCompare(a.updatedAt),
  )
}

export function getSavedBuild(id: string): SavedBuild | null {
  return readLibrary().builds.find((b) => b.id === id) ?? null
}

export function setBuildSeason(buildId: string, season: string): boolean {
  const lib = readLibrary()
  const idx = lib.builds.findIndex((b) => b.id === buildId)
  if (idx === -1) return false
  const builds = lib.builds.map((b, i) =>
    i === idx ? { ...b, season, updatedAt: new Date().toISOString() } : b,
  )
  writeLibrary({ ...lib, builds })
  return true
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
  folderId: string | null = null,
): SavedBuild {
  const library = readLibrary()
  if (library.builds.length >= MAX_BUILDS) {
    throw new StorageCapacityError(
      `Saved-builds limit reached (${MAX_BUILDS}). Delete an existing build before saving a new one.`,
    )
  }
  const now = new Date().toISOString()
  const profileId = newId('p')
  const code = encodeBuildToShare(snapshot)
  const validFolderId =
    folderId !== null && library.folders.some((f) => f.id === folderId)
      ? folderId
      : null
  const record: SavedBuild = {
    id: newId('b'),
    name,
    classId: snapshot.classId,
    notes,
    createdAt: now,
    updatedAt: now,
    profiles: [{ id: profileId, name: profileName, code, updatedAt: now }],
    activeProfileId: profileId,
    folderId: validFolderId,
    favorite: false,
    tags: [],
    season: activeSeasonId,
  }
  library.builds.push(record)
  writeLibrary(library)
  return record
}

export function duplicateBuild(buildId: string): SavedBuild | null {
  const library = readLibrary()
  const src = library.builds.find((b) => b.id === buildId)
  if (!src) return null
  const now = new Date().toISOString()
  const activeIdx = src.profiles.findIndex(
    (p) => p.id === src.activeProfileId,
  )
  const profiles: SavedProfile[] = src.profiles.map((p) => ({
    id: newId('p'),
    name: p.name,
    code: p.code,
    updatedAt: now,
  }))
  const record: SavedBuild = {
    id: newId('b'),
    name: nextDuplicateName(
      src.name,
      library.builds.map((b) => b.name),
    ),
    classId: src.classId,
    notes: src.notes,
    createdAt: now,
    updatedAt: now,
    profiles,
    activeProfileId: profiles[activeIdx >= 0 ? activeIdx : 0]!.id,
    folderId: src.folderId,
    favorite: false,
    tags: [...src.tags],
    season: src.season,
  }
  library.builds.push(record)
  writeLibrary(library)
  return record
}

export function setBuildFavorite(
  buildId: string,
  favorite: boolean,
): SavedBuild | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  build.favorite = favorite
  build.updatedAt = new Date().toISOString()
  writeLibrary(library)
  return build
}

export function setBuildTags(
  buildId: string,
  tags: string[],
): SavedBuild | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  build.tags = sanitizeTags(tags)
  build.updatedAt = new Date().toISOString()
  writeLibrary(library)
  return build
}

export function moveBuildToFolder(
  buildId: string,
  folderId: string | null,
): SavedBuild | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  if (folderId !== null && !library.folders.some((f) => f.id === folderId)) {
    return null
  }
  build.folderId = folderId
  build.updatedAt = new Date().toISOString()
  writeLibrary(library)
  return build
}

export function setBuildNotes(
  buildId: string,
  notes: string,
): SavedBuild | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  build.notes = notes
  build.updatedAt = new Date().toISOString()
  writeLibrary(library)
  return build
}

export function commitProfileSnapshot(
  buildId: string,
  profileId: string,
  snapshot: BuildSnapshot,
): SavedBuild | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  const profile = build.profiles.find((p) => p.id === profileId)
  if (!profile) return null
  const now = new Date().toISOString()
  profile.code = encodeBuildToShare(snapshot)
  profile.updatedAt = now
  build.classId = snapshot.classId
  build.updatedAt = now
  writeLibrary(library)
  return build
}

export function renameBuild(
  buildId: string,
  name: string,
): SavedBuild | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  build.name = name
  build.updatedAt = new Date().toISOString()
  writeLibrary(library)
  return build
}

export function deleteBuild(buildId: string): void {
  const library = readLibrary()
  library.builds = library.builds.filter((b) => b.id !== buildId)
  writeLibrary(library)
}

export function setActiveProfile(
  buildId: string,
  profileId: string,
): SavedBuild | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  if (!build.profiles.some((p) => p.id === profileId)) return null
  build.activeProfileId = profileId
  build.updatedAt = new Date().toISOString()
  writeLibrary(library)
  return build
}

export function addProfile(
  buildId: string,
  name: string,
  snapshot: BuildSnapshot,
  options: { activate?: boolean } = { activate: true },
): { build: SavedBuild; profile: SavedProfile } | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  if (build.profiles.length >= MAX_PROFILES_PER_BUILD) {
    throw new StorageCapacityError(
      `Profile limit reached (${MAX_PROFILES_PER_BUILD}). Delete an existing profile first.`,
    )
  }
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
  writeLibrary(library)
  return { build, profile }
}

export function duplicateProfile(
  buildId: string,
  profileId: string,
  options: { activate?: boolean } = { activate: true },
): { build: SavedBuild; profile: SavedProfile } | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  const src = build.profiles.find((p) => p.id === profileId)
  if (!src) return null
  if (build.profiles.length >= MAX_PROFILES_PER_BUILD) {
    throw new StorageCapacityError(
      `Profile limit reached (${MAX_PROFILES_PER_BUILD}). Delete an existing profile before duplicating.`,
    )
  }
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
  if (options.activate ?? true) build.activeProfileId = profile.id
  build.updatedAt = now
  writeLibrary(library)
  return { build, profile }
}

export function renameProfile(
  buildId: string,
  profileId: string,
  name: string,
): SavedBuild | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  const profile = build.profiles.find((p) => p.id === profileId)
  if (!profile) return null
  profile.name = name
  profile.updatedAt = new Date().toISOString()
  build.updatedAt = profile.updatedAt
  writeLibrary(library)
  return build
}

export function removeProfile(
  buildId: string,
  profileId: string,
): SavedBuild | null {
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
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
  writeLibrary(library)
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

function nextDuplicateName(base: string, taken: string[]): string {
  const cleanBase = base.replace(/\s+\(copy(?:\s+\d+)?\)$/i, '')
  const seen = new Set(taken)
  if (!seen.has(`${cleanBase} (copy)`)) return `${cleanBase} (copy)`
  for (let i = 2; ; i++) {
    const candidate = `${cleanBase} (copy ${i})`
    if (!seen.has(candidate)) return candidate
  }
}
