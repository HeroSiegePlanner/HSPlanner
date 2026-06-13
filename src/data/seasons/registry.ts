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

export const SEASON_STORAGE_KEY = 'hsplanner.season.v1'

export function isKnownSeasonId(id: string): boolean {
  return SEASONS.some((s) => s.id === id)
}

export function getSeason(id: string): Season | undefined {
  return SEASONS.find((s) => s.id === id)
}

export function resolveActiveSeasonId(): string {
  try {
    const stored = window.localStorage.getItem(SEASON_STORAGE_KEY)
    if (stored && isKnownSeasonId(stored)) return stored
  } catch {
    return DEFAULT_SEASON_ID
  }
  return DEFAULT_SEASON_ID
}

export function setStoredSeasonId(id: string): boolean {
  if (!isKnownSeasonId(id)) return false
  try {
    window.localStorage.setItem(SEASON_STORAGE_KEY, id)
    return true
  } catch {
    return false
  }
}
