import { describe, expect, it } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useBuildPerformanceDeps } from './useBuildPerformanceDeps'
import { snapshotToDeps } from '../components/buildSelect/usePreviewStats'
import { makeSnapshot } from '../utils/build/buildSnapshot.fixture'

describe('useBuildPerformanceDeps', () => {
  it('builds the same dependency keys as the preview snapshotToDeps', () => {
    const previewKeys = Object.keys(snapshotToDeps(makeSnapshot())).toSorted()

    const { result } = renderHook(() => useBuildPerformanceDeps())
    const mainKeys = Object.keys(result.current).toSorted()

    expect(previewKeys).toEqual(mainKeys)
  })
})
