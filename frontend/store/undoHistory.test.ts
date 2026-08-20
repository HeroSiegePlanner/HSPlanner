import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { useBuild } from './build'
import { initUndoHistory, redoLastChange, undoLastChange } from './undoHistory'

describe('undo history', () => {
  let dispose: () => void

  beforeEach(() => {
    useBuild.setState({ level: 1, activeBuildId: null, activeProfileId: null })
    dispose = initUndoHistory()
  })

  afterEach(() => {
    dispose()
  })

  it('returns false with no recorded history', () => {
    expect(undoLastChange()).toBe(false)
  })

  it('restores the previous build state one step back', () => {
    useBuild.getState().setLevel(10)
    useBuild.getState().setLevel(20)

    expect(undoLastChange()).toBe(true)
    expect(useBuild.getState().level).toBe(10)

    expect(undoLastChange()).toBe(true)
    expect(useBuild.getState().level).toBe(1)
  })

  it('does not record the undo itself as a new change', () => {
    useBuild.getState().setLevel(10)

    expect(undoLastChange()).toBe(true)
    expect(useBuild.getState().level).toBe(1)
    expect(undoLastChange()).toBe(false)
  })

  it('ignores changes outside the snapshot keys', () => {
    useBuild.setState({ storageError: 'boom' })
    expect(undoLastChange()).toBe(false)
  })

  it('clears history when the active build changes', () => {
    useBuild.getState().setLevel(10)
    useBuild.setState({ activeBuildId: 'other-build' })

    expect(undoLastChange()).toBe(false)
  })

  it('redoes an undone change', () => {
    useBuild.getState().setLevel(10)
    useBuild.getState().setLevel(20)

    undoLastChange()
    expect(useBuild.getState().level).toBe(10)

    expect(redoLastChange()).toBe(true)
    expect(useBuild.getState().level).toBe(20)

    // and the redo itself can be undone again
    expect(undoLastChange()).toBe(true)
    expect(useBuild.getState().level).toBe(10)
  })

  it('returns false when there is nothing to redo', () => {
    useBuild.getState().setLevel(10)
    expect(redoLastChange()).toBe(false)
  })

  it('clears redo when a new change lands after an undo', () => {
    useBuild.getState().setLevel(10)
    undoLastChange()
    useBuild.getState().setLevel(30)

    expect(redoLastChange()).toBe(false)
    expect(useBuild.getState().level).toBe(30)
  })
})
