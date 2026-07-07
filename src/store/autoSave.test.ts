import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { AUTO_SAVE_DEBOUNCE_MS, initAutoSave } from './autoSave'
import { useBuild } from './build'
import { useSettings } from './settings'
import { loadProfileSnapshot } from '../utils/build/savedBuilds'

function saveFixtureBuild(name: string) {
  const record = useBuild.getState().saveCurrentAsNewBuild(name)
  if (!record) throw new Error('fixture build not saved')
  return { buildId: record.id, profileId: record.activeProfileId }
}

describe('auto-save engine', () => {
  let unsubscribe: () => void

  beforeEach(() => {
    window.localStorage.clear()
    vi.useFakeTimers()
    useSettings.setState({ autoSave: true })
    useBuild.getState().resetBuild()
    unsubscribe = initAutoSave()
  })

  afterEach(() => {
    unsubscribe()
    vi.useRealTimers()
    useBuild.getState().resetBuild()
    useSettings.setState({ autoSave: true })
  })

  it('commits build changes after the debounce when auto-save is on', () => {
    const { buildId, profileId } = saveFixtureBuild('Auto On')
    useBuild.getState().setLevel(50)

    vi.advanceTimersByTime(AUTO_SAVE_DEBOUNCE_MS + 1)

    expect(loadProfileSnapshot(buildId, profileId)?.level).toBe(50)
  })

  it('does not persist changes when auto-save is off', () => {
    const { buildId, profileId } = saveFixtureBuild('Auto Off')
    useSettings.setState({ autoSave: false })
    useBuild.getState().setLevel(60)

    vi.advanceTimersByTime(AUTO_SAVE_DEBOUNCE_MS * 5)

    expect(loadProfileSnapshot(buildId, profileId)?.level).toBe(1)
  })

  it('saveBuildNow persists manually even when auto-save is off', () => {
    const { buildId, profileId } = saveFixtureBuild('Manual')
    useSettings.setState({ autoSave: false })
    useBuild.getState().setLevel(60)

    expect(useBuild.getState().saveBuildNow()).toBe(true)

    expect(loadProfileSnapshot(buildId, profileId)?.level).toBe(60)
  })

  it('discards unsaved edits on build reload when auto-save is off', () => {
    const { buildId, profileId } = saveFixtureBuild('Discard')
    useSettings.setState({ autoSave: false })
    useBuild.getState().setLevel(70)

    useBuild.getState().loadSavedBuild(buildId)

    expect(loadProfileSnapshot(buildId, profileId)?.level).toBe(1)
    expect(useBuild.getState().level).toBe(1)
  })
})
