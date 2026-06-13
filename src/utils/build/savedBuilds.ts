import {
  type BuildSnapshot,
  decodeShareToBuild,
  encodeBuildToShare,
} from './shareBuild'
import {
  convertSnapshotToActiveSeason,
  type SeasonConversionReport,
} from './seasonConvert'
import { readStorage, readStorageWithLegacy, writeStorage } from '../storage'
import { activeSeasonId } from '../../data'

const STORAGE_KEY_V1 = 'hsplanner.savedBuilds.v1'
const STORAGE_KEY_V2 = 'hsplanner.savedBuilds.v2'
const STORAGE_KEY = 'hsplanner.savedBuilds.v3'

const LEGACY_STORAGE_KEY_V1 = 'heroplanner.savedBuilds.v1'
const LEGACY_STORAGE_KEY_V2 = 'heroplanner.savedBuilds.v2'
const LEGACY_STORAGE_KEY = 'heroplanner.savedBuilds.v3'

const DEFAULT_PROFILE_NAME = 'Default'

export class StorageWriteError extends Error {
  // Thrown by `writeLibrary` when persisting the saved-builds library to
  // localStorage fails — most commonly because the ~5 MB origin quota is
  // exhausted. It lets callers (and ultimately the build store) surface the
  // failure to the user instead of silently dropping the build they believed
  // they had saved.
  constructor(message = 'Could not save to local storage — it may be full.') {
    super(message)
    this.name = 'StorageWriteError'
  }
}

// Subtypes StorageWriteError so existing instanceof catches still surface it.
export class StorageCapacityError extends StorageWriteError {
  constructor(message: string) {
    super(message)
    this.name = 'StorageCapacityError'
  }
}

const LEGACY_BUILD_SEASON = 's9'

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
  /** Owning folder; `null` means the build is unfiled. */
  folderId: string | null
  favorite: boolean
  tags: string[]
  season: string
}

/** v3 storage shape: builds and folders co-located in one object. */
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
  // Returns a fresh unique identifier, preferring `crypto.randomUUID` and falling back to a `prefix_<timestamp><random>` form on platforms without it. Used to generate ids for newly created builds, profiles and folders.
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID()
  }
  return `${prefix}_${Date.now().toString(36)}${Math.random()
    .toString(36)
    .slice(2, 8)}`
}

function clampString(s: unknown, max: number, fallback = ''): string {
  // Returns the first `max` characters of `s` when it is a string, otherwise returns `fallback`. Used to bound user-controllable string fields to defensive maximum lengths before they enter the app state.
  return typeof s === 'string' ? s.slice(0, max) : fallback
}

function sanitizeTags(raw: unknown): string[] {
  // Coerces a persisted `tags` value into a clean, de-duplicated, capped string array. Non-array input yields `[]`; entries are trimmed, length-clamped, and empties dropped. Used by `cleanBuild` and `setBuildTags`.
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
  // Type guard validating that an unknown blob has the SavedProfile shape with all string fields under their length caps. Used to defensively filter persisted profile entries before exposing them to the rest of the app.
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
  // Filters an unknown `profiles` value down to valid, length-bounded SavedProfile records (capped at MAX_PROFILES_PER_BUILD). Used by `cleanBuild`.
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
  // Validates a single persisted build entry, trimming oversized fields, applying defaults for the v3 fields (folderId/favorite/tags), and coercing an orphan `folderId` to null. Returns null for malformed entries (missing id/name, no profiles). Used by `cleanBuilds`.
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
        : LEGACY_BUILD_SEASON,
  }
}

function cleanBuilds(
  raw: unknown[],
  validFolderIds: Set<string>,
): SavedBuild[] {
  // Maps an array of unknown persisted entries to valid SavedBuild records, dropping malformed ones and capping the count at MAX_BUILDS.
  const now = new Date().toISOString()
  const out: SavedBuild[] = []
  for (const entry of raw.slice(0, MAX_BUILDS)) {
    const build = cleanBuild(entry, validFolderIds, now)
    if (build) out.push(build)
  }
  return out
}

function sanitizeFolders(raw: unknown): Folder[] {
  // Validates an unknown `folders` value: drops malformed/duplicate entries, coerces a parentId that points at a missing folder to null, and breaks any parent-chain cycles. Used by `readLibrary`.
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
  // Drop parent references that point at a non-existent folder.
  const ids = new Set(out.map((f) => f.id))
  for (const f of out) {
    if (f.parentId !== null && !ids.has(f.parentId)) f.parentId = null
  }
  // Break parent-chain cycles defensively: walk each folder up to the root and
  // null the link that closes a loop.
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
  // Reads the previous-generation v1 saved-builds blob (with legacy "heroplanner" key fallback) and returns only entries whose required string fields are present. Used as the migration source when no v2/v3 data exists yet.
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
  // Wraps every legacy v1 build into a v3 build that contains a single "Default" profile carrying the original code, with the v3 fields defaulted (unfiled, not favorite, no tags).
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
      season: LEGACY_BUILD_SEASON,
    }
  })
}

function readLegacyBuilds(): SavedBuild[] {
  // Reads builds from the pre-v3 storage: the v2 array (with legacy key fallback) or, failing that, a v1→v2 migration. Returns builds with the v3 fields defaulted. Used by `readLibrary` to seed a v3 library on first run after the upgrade.
  const raw = readStorageWithLegacy(STORAGE_KEY_V2, LEGACY_STORAGE_KEY_V2)
  if (raw) {
    try {
      const parsed: unknown = JSON.parse(raw)
      if (Array.isArray(parsed)) return cleanBuilds(parsed, new Set())
    } catch {
      // Corrupt/non-array v2: fall through to the v1 migration below rather
      // than returning [], so a damaged v2 key can't mask recoverable v1 data.
    }
  }
  const v1 = readV1()
  return v1.length > 0 ? migrateV1(v1) : []
}

export function readLibrary(): SavedLibrary {
  // Loads the persisted v3 saved-builds library (with legacy key fallback), defensively trimming oversized fields and dropping malformed entries. When no v3 data exists, migrates from v2/v1 and best-effort persists the result. Used internally by every public read/mutate function and by `savedFolders.ts`.
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
    // The v3 blob exists but is unparseable or malformed. Preserve it under a
    // backup key BEFORE returning an empty library, otherwise the next save
    // would overwrite the v3 key and permanently destroy recoverable builds.
    backupCorruptLibrary(raw)
    return emptyLibrary()
  }
  // No v3 data yet — migrate from v2/v1 and best-effort persist the result.
  const builds = readLegacyBuilds()
  const library: SavedLibrary = { version: 3, builds, folders: [] }
  if (builds.length > 0) {
    try {
      writeLibrary(library)
    } catch {
      // Best-effort migration: if the rewrite under the v3 key fails (e.g.
      // storage is full) we still return the migrated library in memory and
      // retry persisting it on the next load — throwing here would make
      // every read appear to wipe the user's entire library.
    }
  }
  return library
}

// Saves the first seen corrupt v3 blob under a one-off backup key so a later
// write can't silently overwrite (and destroy) data that might be recoverable
// by hand. Best-effort: never throws from the read path.
function backupCorruptLibrary(raw: string): void {
  try {
    const key = `${STORAGE_KEY}.corrupt`
    if (!readStorage(key)) writeStorage(key, raw)
  } catch {
    // ignore — the backup is purely best-effort
  }
}

export function writeLibrary(library: SavedLibrary): void {
  // Persists the supplied SavedLibrary to localStorage as JSON under the v3 key, throwing StorageWriteError when the write is rejected (e.g. the storage quota is exceeded) so the failure is never silently swallowed. Used by every mutating function in this module and in `savedFolders.ts`.
  if (!writeStorage(STORAGE_KEY, JSON.stringify(library))) {
    throw new StorageWriteError()
  }
}

export function listSavedBuilds(): SavedBuild[] {
  // Returns every persisted build sorted by `updatedAt` descending so the most recently used build appears first. Used by the build library UI.
  return readLibrary().builds.sort((a, b) =>
    b.updatedAt.localeCompare(a.updatedAt),
  )
}

export function getSavedBuild(id: string): SavedBuild | null {
  // Looks up a single SavedBuild by id, returning null when nothing matches. Used to hydrate the active build before reading or mutating its profiles.
  return readLibrary().builds.find((b) => b.id === id) ?? null
}

export function getActiveProfile(b: SavedBuild): SavedProfile | null {
  // Returns the profile referenced by `activeProfileId`, falling back to the first profile or null when the build has none. Used by the UI to know which profile's snapshot to load when the user opens a build.
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
  // Creates a brand-new SavedBuild containing exactly one (active) profile encoded from `snapshot`, persists it, and returns the freshly built record. Optionally files it under `folderId`. Used when the user explicitly saves the current state as a new build.
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
  // Deep-clones a build: fresh build + profile ids, a non-colliding "(copy)" name, copied notes/folder/tags, and `favorite` reset to false. Returns the new record, or null when the source build does not exist. Used by the library "Copy" action.
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
  // Sets the `favorite` flag on a build and refreshes `updatedAt`. Used by the library's star toggle.
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
  // Replaces a build's tag list (sanitised: trimmed, de-duplicated, capped) and refreshes `updatedAt`. Used by the library's tag editor.
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  build.tags = sanitizeTags(tags)
  build.updatedAt = new Date().toISOString()
  writeLibrary(library)
  return build
}

export interface SavedBuildConversion {
  build: SavedBuild
  /** Null when the active profile's code could not be decoded. */
  report: SeasonConversionReport | null
}

// Converts ALL profiles of the build in one pass (a build has one season, so a
// half-converted profile set would corrupt it), then restamps the season.
// Undecodable profile codes pass through untouched.
export function convertSavedBuildToSeason(
  buildId: string,
): SavedBuildConversion | null {
  const lib = readLibrary()
  const idx = lib.builds.findIndex((b) => b.id === buildId)
  if (idx === -1) return null
  const build = lib.builds[idx]!
  if (build.season === activeSeasonId) return null

  let activeReport: SeasonConversionReport | null = null
  const profiles: SavedProfile[] = []
  for (const p of build.profiles) {
    const decoded = decodeShareToBuild(p.code)
    if (!decoded) {
      profiles.push(p)
      continue
    }
    const { snapshot, report } = convertSnapshotToActiveSeason(
      decoded.snapshot,
      build.season,
    )
    if (p.id === build.activeProfileId) activeReport = report
    profiles.push({ ...p, code: encodeBuildToShare(snapshot, decoded.notes) })
  }

  const updated: SavedBuild = {
    ...build,
    profiles,
    season: activeSeasonId,
    updatedAt: new Date().toISOString(),
  }
  const builds = lib.builds.map((b, i) => (i === idx ? updated : b))
  writeLibrary({ ...lib, builds })
  return { build: updated, report: activeReport }
}

export function moveBuildToFolder(
  buildId: string,
  folderId: string | null,
): SavedBuild | null {
  // Moves a build into the given folder (or unfiles it when `folderId` is null), validating that the target folder exists. Used by the library's "Move to folder" action.
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
  // Replaces the build-level notes (shared by every profile) with the supplied HTML string and bumps `updatedAt`. Used by NotesView whenever the user edits a build's notes.
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
  // Encodes `snapshot` and writes it onto the named profile of the named build, refreshing both the profile and build timestamps as well as the build's classId. Used both for explicit saves and for committing the current state before switching profiles.
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
  // Renames a SavedBuild by id and refreshes its `updatedAt`. Used by the library when the user edits a build's title.
  const library = readLibrary()
  const build = library.builds.find((b) => b.id === buildId)
  if (!build) return null
  build.name = name
  build.updatedAt = new Date().toISOString()
  writeLibrary(library)
  return build
}

export function deleteBuild(buildId: string): void {
  // Removes the SavedBuild with the supplied id from storage. Used by the library delete action.
  const library = readLibrary()
  library.builds = library.builds.filter((b) => b.id !== buildId)
  writeLibrary(library)
}

export function setActiveProfile(
  buildId: string,
  profileId: string,
): SavedBuild | null {
  // Marks `profileId` as the active profile of `buildId`, validating that the profile exists, and updates the build timestamp. Used by ProfileSwitcher when the user switches profiles within a build.
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
  // Appends a new profile to the named build using the supplied snapshot, optionally promoting it to the active profile. Used by ProfileSwitcher when the user adds a new variant inside a build.
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
  // Clones an existing profile, picking a non-colliding "(copy)" name. When `options.activate` is set (the default) the duplicate becomes the active profile — ProfileSwitcher relies on this so the live editor follows the fork; the Build Select library passes `activate: false` so duplicating never changes which profile a non-loaded build opens with.
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
  // Renames a single profile within a build and refreshes both timestamps. Used by ProfileSwitcher's inline rename action.
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
  // Deletes a profile from a build (refusing to remove the last surviving profile) and reassigns the active profile to a sensible neighbour when the deleted profile was active. Used by ProfileSwitcher's delete action.
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
  // Decodes the lz-string-compressed share code stored on a profile back into a BuildSnapshot, returning null on any lookup or decode failure. Used by the build store when hydrating a profile into the live editor state.
  const build = getSavedBuild(buildId)
  if (!build) return null
  const profile = build.profiles.find((p) => p.id === profileId)
  if (!profile) return null
  return decodeShareToBuild(profile.code)?.snapshot ?? null
}

function nextDuplicateName(base: string, taken: string[]): string {
  // Generates the next available "(copy)" / "(copy 2)" / "(copy 3)" name from a base, skipping names already taken in `taken` and falling back to a timestamped name after fifty collisions. Used when duplicating builds and profiles.
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
