import { act, renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { useCalcResult } from './useCalcResult'

// Manual promise so tests control resolve/reject timing.
function deferred<T>() {
  let resolve!: (v: T) => void
  let reject!: (e: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

describe('useCalcResult', () => {
  it('returns fallback initially and the resolved value once compute resolves', async () => {
    const d = deferred<string>()
    const { result } = renderHook(() => useCalcResult(() => d.promise, [], 'fallback'))
    expect(result.current).toBe('fallback')
    await act(async () => d.resolve('value'))
    await waitFor(() => expect(result.current).toBe('value'))
  })

  it('resets to fallback instead of keeping a stale value when compute rejects', async () => {
    const first = deferred<string>()
    const second = deferred<string>()
    const queue = [first.promise, second.promise]
    let calls = 0
    const { result, rerender } = renderHook(
      ({ dep }: { dep: number }) => useCalcResult(() => queue[calls++]!, [dep], 'fallback'),
      { initialProps: { dep: 0 } },
    )
    await act(async () => first.resolve('first'))
    await waitFor(() => expect(result.current).toBe('first'))
    rerender({ dep: 1 })
    await act(async () => second.reject(new Error('IPC fail')))
    await waitFor(() => expect(result.current).toBe('fallback'))
  })

  it('applies a non-promise compute result synchronously', () => {
    const { result } = renderHook(() => useCalcResult(() => 'sync', [], 'fallback'))
    expect(result.current).toBe('sync')
  })

  it('falls back when compute throws synchronously', () => {
    const { result } = renderHook(() =>
      useCalcResult<string>(
        () => {
          throw new Error('sync boom')
        },
        [],
        'fallback',
      ),
    )
    expect(result.current).toBe('fallback')
  })

  it('ignores a stale resolution arriving after deps changed', async () => {
    const slow = deferred<string>()
    const fast = deferred<string>()
    const queue = [slow.promise, fast.promise]
    let calls = 0
    const { result, rerender } = renderHook(
      ({ dep }: { dep: number }) => useCalcResult(() => queue[calls++]!, [dep], 'fallback'),
      { initialProps: { dep: 0 } },
    )
    rerender({ dep: 1 })
    await act(async () => fast.resolve('fast'))
    await waitFor(() => expect(result.current).toBe('fast'))
    await act(async () => slow.resolve('slow'))
    expect(result.current).toBe('fast')
  })

  it('ignores a stale rejection arriving after deps changed', async () => {
    const slow = deferred<string>()
    const fast = deferred<string>()
    const queue = [slow.promise, fast.promise]
    let calls = 0
    const { result, rerender } = renderHook(
      ({ dep }: { dep: number }) => useCalcResult(() => queue[calls++]!, [dep], 'fallback'),
      { initialProps: { dep: 0 } },
    )
    rerender({ dep: 1 })
    await act(async () => fast.resolve('fast'))
    await waitFor(() => expect(result.current).toBe('fast'))
    await act(async () => slow.reject(new Error('late fail')))
    expect(result.current).toBe('fast')
  })

  it('does not crash when a pending compute rejects after unmount', async () => {
    const d = deferred<string>()
    const { result, unmount } = renderHook(() =>
      useCalcResult(() => d.promise, [], 'fallback'),
    )
    expect(result.current).toBe('fallback')
    unmount()
    await act(async () => d.reject(new Error('after unmount')))
  })
})
