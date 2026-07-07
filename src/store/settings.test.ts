import { beforeEach, describe, expect, it, vi } from 'vitest'

const SETTINGS_KEY = 'hsplanner.settings.v1'

async function freshStore() {
  vi.resetModules()
  const mod = await import('./settings')
  return mod.useSettings
}

describe('settings store', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('starts with auto-save on and billions scale when nothing is stored', async () => {
    const useSettings = await freshStore()
    expect(useSettings.getState().autoSave).toBe(true)
    expect(useSettings.getState().numberScale).toBe('billions')
  })

  it('persists changes to localStorage', async () => {
    const useSettings = await freshStore()
    useSettings.getState().setAutoSave(false)
    useSettings.getState().setNumberScale('millions')
    const stored = JSON.parse(window.localStorage.getItem(SETTINGS_KEY) ?? '{}')
    expect(stored).toEqual({ autoSave: false, numberScale: 'millions' })
  })

  it('restores persisted settings on load', async () => {
    window.localStorage.setItem(
      SETTINGS_KEY,
      JSON.stringify({ autoSave: false, numberScale: 'billions' }),
    )
    const useSettings = await freshStore()
    expect(useSettings.getState().autoSave).toBe(false)
    expect(useSettings.getState().numberScale).toBe('billions')
  })

  it('falls back to defaults on corrupted or invalid stored values', async () => {
    window.localStorage.setItem(SETTINGS_KEY, '{not json')
    let useSettings = await freshStore()
    expect(useSettings.getState().autoSave).toBe(true)

    window.localStorage.setItem(
      SETTINGS_KEY,
      JSON.stringify({ autoSave: 'yes', numberScale: 'trillions' }),
    )
    useSettings = await freshStore()
    expect(useSettings.getState().autoSave).toBe(true)
    expect(useSettings.getState().numberScale).toBe('billions')
  })
})
