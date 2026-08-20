import { act, renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { useProgressionPreview } from './useProgressionPreview'

const ADJ = new Map<number, Set<number>>([
  [1, new Set([2])],
  [2, new Set([1, 3])],
  [3, new Set([2])],
])
const STARTS = new Set([1])

function setup(allocated: Set<number>) {
  return renderHook(
    ({ alloc }: { alloc: Set<number> }) =>
      useProgressionPreview(alloc, STARTS, ADJ),
    { initialProps: { alloc: allocated } },
  )
}

describe('useProgressionPreview', () => {
  it('starts in live mode exposing the full allocation', () => {
    const allocated = new Set([1, 2, 3])
    const { result } = setup(allocated)

    expect(result.current.progressStep).toBeNull()
    expect(result.current.isPreview).toBe(false)
    expect(result.current.visibleAllocated).toBe(allocated)
    expect(result.current.markerId).toBeNull()
  })

  it('exposes the first-K prefix and marker in preview mode', () => {
    const { result } = setup(new Set([1, 2, 3]))

    act(() => result.current.setProgressStep(2))

    expect(result.current.isPreview).toBe(true)
    expect([...result.current.visibleAllocated]).toEqual([1, 2])
    expect(result.current.markerId).toBe(2)
  })

  it('snaps back to live mode when the allocation changes', () => {
    const { result, rerender } = setup(new Set([1, 2, 3]))
    act(() => result.current.setProgressStep(2))
    expect(result.current.isPreview).toBe(true)

    rerender({ alloc: new Set([1, 2]) })

    expect(result.current.progressStep).toBeNull()
    expect(result.current.visibleAllocated.size).toBe(2)
  })
})
