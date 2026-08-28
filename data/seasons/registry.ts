import { readStorage, writeStorage } from '../../frontend/utils/storage'

export interface Season {
  id: string
  name: string
}

export const SEASONS: ReadonlyArray<Season> = [
  { id: 's10', name: 'Season 10' },
]

export const DEFAULT_SEASON_ID = 's10' as const

// Bumped to v2 at the S10 default flip: dropping the stored pick is the point,
// so everyone lands on DEFAULT_SEASON_ID instead of their old season.
export const SEASON_STORAGE_KEY = 'hsplanner.season.v2'

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

export function reloadIntoSeason(
  season: string,
  pendingKey: string,
  pendingValue: string,
  activeSeason: string,
  reload: () => void = () => window.location.reload(),
): boolean {
  if (!isKnownSeasonId(season) || season === activeSeason) return false
  if (!writeStorage(pendingKey, pendingValue) || !setStoredSeasonId(season)) return false
  reload()
  return true
}
