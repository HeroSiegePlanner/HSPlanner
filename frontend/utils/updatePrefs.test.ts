import { beforeEach, describe, expect, it } from 'vitest'
import {
  AUTO_CHECK_COOLDOWN_MS,
  isAutoCheckEnabled,
  markChecked,
  readLastCheck,
  setAutoCheckEnabled,
  shouldAutoCheck,
} from './updatePrefs'

const AUTO_CHECK_KEY = 'hsplanner.update.auto_check'
const LAST_CHECK_KEY = 'hsplanner.update.last_check'

const NOW = 1_800_000_000_000

beforeEach(() => {
  window.localStorage.clear()
})

describe('auto-check flag', () => {
  it('defaults to on when nothing was ever stored', () => {
    expect(isAutoCheckEnabled()).toBe(true)
  })

  it('round-trips through the documented storage key', () => {
    setAutoCheckEnabled(false)
    expect(window.localStorage.getItem(AUTO_CHECK_KEY)).toBe('0')
    expect(isAutoCheckEnabled()).toBe(false)

    setAutoCheckEnabled(true)
    expect(window.localStorage.getItem(AUTO_CHECK_KEY)).toBe('1')
    expect(isAutoCheckEnabled()).toBe(true)
  })
})

describe('last check timestamp', () => {
  it('returns null for missing and unparseable values', () => {
    expect(readLastCheck()).toBeNull()
    window.localStorage.setItem(LAST_CHECK_KEY, 'not-a-number')
    expect(readLastCheck()).toBeNull()
    window.localStorage.setItem(LAST_CHECK_KEY, '-5')
    expect(readLastCheck()).toBeNull()
  })

  it('stores the instant it was told about', () => {
    markChecked(NOW)
    expect(readLastCheck()).toBe(NOW)
  })
})

describe('shouldAutoCheck', () => {
  it('checks on a first run', () => {
    expect(shouldAutoCheck(NOW)).toBe(true)
  })

  it('holds off inside the cooldown', () => {
    markChecked(NOW)
    expect(shouldAutoCheck(NOW + AUTO_CHECK_COOLDOWN_MS - 1)).toBe(false)
  })

  it('checks again once the cooldown elapses', () => {
    markChecked(NOW)
    expect(shouldAutoCheck(NOW + AUTO_CHECK_COOLDOWN_MS)).toBe(true)
  })

  it('never checks while the pref is off', () => {
    setAutoCheckEnabled(false)
    expect(shouldAutoCheck(NOW)).toBe(false)
  })

  it('recovers from a clock that jumped backwards', () => {
    markChecked(NOW)
    expect(shouldAutoCheck(NOW - 60_000)).toBe(true)
  })
})
