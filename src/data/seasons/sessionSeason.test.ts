import { describe, expect, it, vi } from 'vitest'
import {
  PENDING_BUILD_KEY,
  reloadIntoSeason,
  SEASON_STORAGE_KEY,
} from './registry'

describe('reloadIntoSeason', () => {
  it('does nothing and returns false when the season already matches', () => {
    const reload = vi.fn()
    expect(reloadIntoSeason('s9', PENDING_BUILD_KEY, 'b1', 's9', reload)).toBe(false)
    expect(reload).not.toHaveBeenCalled()
    expect(window.localStorage.getItem(PENDING_BUILD_KEY)).toBeNull()
  })

  it('persists season + pending value and reloads when the season differs', () => {
    const reload = vi.fn()
    expect(reloadIntoSeason('s10', PENDING_BUILD_KEY, 'b1', 's9', reload)).toBe(true)
    expect(window.localStorage.getItem(SEASON_STORAGE_KEY)).toBe('s10')
    expect(window.localStorage.getItem(PENDING_BUILD_KEY)).toBe('b1')
    expect(reload).toHaveBeenCalledTimes(1)
  })

  it('rejects an unknown target season (no write, no reload)', () => {
    const reload = vi.fn()
    expect(reloadIntoSeason('s99', PENDING_BUILD_KEY, 'b1', 's9', reload)).toBe(false)
    expect(reload).not.toHaveBeenCalled()
  })
})
