import { describe, expect, it, vi } from 'vitest'
import {
  DEFAULT_SEASON_ID,
  PENDING_BUILD_KEY,
  SEASONS,
  SEASON_STORAGE_KEY,
  getSeason,
  isKnownSeasonId,
  reloadIntoSeason,
  resolveActiveSeasonId,
  setStoredSeasonId,
} from './registry'

describe('season registry', () => {
  it('contains s9 and s10 with s9 as default', () => {
    expect(SEASONS.map((s) => s.id)).toEqual(['s9', 's10'])
    expect(DEFAULT_SEASON_ID).toBe('s9')
  })

  it('falls back to default when storage is empty', () => {
    expect(resolveActiveSeasonId()).toBe(DEFAULT_SEASON_ID)
  })

  it('returns stored season when valid', () => {
    window.localStorage.setItem(SEASON_STORAGE_KEY, 's10')
    expect(resolveActiveSeasonId()).toBe('s10')
  })

  it('ignores unknown stored season', () => {
    window.localStorage.setItem(SEASON_STORAGE_KEY, 's99')
    expect(resolveActiveSeasonId()).toBe(DEFAULT_SEASON_ID)
  })

  it('setStoredSeasonId rejects unknown ids and persists known ones', () => {
    expect(setStoredSeasonId('s99')).toBe(false)
    expect(setStoredSeasonId('s10')).toBe(true)
    expect(window.localStorage.getItem(SEASON_STORAGE_KEY)).toBe('s10')
  })

  it('returns true for known season ids and false otherwise', () => {
    expect(isKnownSeasonId('s9')).toBe(true)
    expect(isKnownSeasonId('')).toBe(false)
  })

  it('getSeason returns the season entry or undefined', () => {
    expect(getSeason('s10')?.name).toBe('Season 10')
    expect(getSeason('nope')).toBeUndefined()
  })
})

describe('reloadIntoSeason storage-failure guard', () => {
  it('reloads and returns true once both writes land', () => {
    let reloaded = false
    const result = reloadIntoSeason('s10', PENDING_BUILD_KEY, 'build-1', 's9', () => {
      reloaded = true
    })
    expect(result).toBe(true)
    expect(reloaded).toBe(true)
    expect(window.localStorage.getItem(SEASON_STORAGE_KEY)).toBe('s10')
    expect(window.localStorage.getItem(PENDING_BUILD_KEY)).toBe('build-1')
  })

  it('does not reload (returns false) when a write is rejected, so the pending build is not lost', () => {
    const spy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new Error('QuotaExceeded')
      })
    let reloaded = false
    try {
      const result = reloadIntoSeason('s10', PENDING_BUILD_KEY, 'build-2', 's9', () => {
        reloaded = true
      })
      expect(result).toBe(false)
      expect(reloaded).toBe(false)
    } finally {
      spy.mockRestore()
    }
  })

  it('returns false without reloading when the target season is already active', () => {
    let reloaded = false
    const result = reloadIntoSeason('s9', PENDING_BUILD_KEY, 'x', 's9', () => {
      reloaded = true
    })
    expect(result).toBe(false)
    expect(reloaded).toBe(false)
  })
})
