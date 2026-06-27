import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'

const computeMock = vi.fn()
vi.mock('../../lib/calc/bridge', () => ({
  computeBuildPerformanceAsync: (deps: unknown) => computeMock(deps),
}))

const decodeMock = vi.fn()
vi.mock('../../utils/build/shareBuild', () => ({
  decodeShareToBuild: (code: string) => decodeMock(code),
}))

vi.mock('../../utils/build/savedBuilds', () => ({
  getActiveProfile: () => ({ id: 'p1', name: 'P1', code: 'CODE' }),
}))

import { usePreviewStats } from './usePreviewStats'
import type { SavedBuild } from '../../utils/build/savedBuilds'

afterEach(() => {
  vi.clearAllMocks()
  vi.useRealTimers()
})

// Regression: preview must use the build metadata season, not the stale blob season.
describe('usePreviewStats season source', () => {
  it('computes with the build metadata season, not the stale blob season', async () => {
    // Arrange
    vi.useFakeTimers()
    computeMock.mockResolvedValue({})
    decodeMock.mockReturnValue({ snapshot: {}, season: 's10' })
    const build = { id: 'b1', season: 's9' } as unknown as SavedBuild

    // Act
    renderHook(() => usePreviewStats(build))
    await act(async () => {
      vi.advanceTimersByTime(200)
    })

    // Assert
    expect(computeMock).toHaveBeenCalledTimes(1)
    expect(computeMock.mock.calls[0]![0]).toMatchObject({ season: 's9' })
  })
})
