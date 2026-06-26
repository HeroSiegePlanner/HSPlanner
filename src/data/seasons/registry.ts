import { readStorage, writeStorage } from '../../utils/storage'

export interface Season {
  id: string
  name: string
}

export const SEASONS: ReadonlyArray<Season> = [
  { id: 's9', name: 'Season 9' },
  { id: 's10', name: 'Season 10' },
]

// Flip to 's10' (here and in src-tauri/src/calc/season.rs) at S10 launch.
export const DEFAULT_SEASON_ID = 's9' as const

// Stamp for pre-season data (v1 share codes, unstamped builds); never flips.
export const LEGACY_SEASON_ID = 's9'

// The one season where charms cannot gain stars/forge; mirrored in src-tauri/src/calc/data.rs.
export const SEASON_BEFORE_CHARM_STARS = 's9'

export const SEASON_STORAGE_KEY = 'hsplanner.season.v1'

export function isKnownSeasonId(id: string): boolean {
  return SEASONS.some((s) => s.id === id)
}

export function getSeason(id: string): Season | undefined {
  return SEASONS.find((s) => s.id === id)
}

export function resolveActiveSeasonId(): string {
  const stored = readStorage(SEASON_STORAGE_KEY)
  return stored && isKnownSeasonId(stored) ? stored : DEFAULT_SEASON_ID
}

export function setStoredSeasonId(id: string): boolean {
  return isKnownSeasonId(id) && writeStorage(SEASON_STORAGE_KEY, id)
}

export const PENDING_BUILD_KEY = 'hsplanner.pendingBuild.v1'
export const PENDING_IMPORT_KEY = 'hsplanner.pendingImport.v1'

// Persists the season + a pending boot action then reloads; returns true when the caller must stop its flow.
export function reloadIntoSeason(
  season: string,
  pendingKey: string,
  pendingValue: string,
  activeSeason: string,
  reload: () => void = () => window.location.reload(),
): boolean {
  if (!isKnownSeasonId(season) || season === activeSeason) return false
  // Only reload once BOTH writes land — a failed localStorage write (private mode,
  // quota) would otherwise reload into the default season and lose the pending build.
  if (!writeStorage(pendingKey, pendingValue) || !setStoredSeasonId(season)) return false
  reload()
  return true
}
