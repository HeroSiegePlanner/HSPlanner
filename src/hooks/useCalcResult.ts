import { useEffect, useState } from 'react'
import type { DependencyList } from 'react'

// Shared lifecycle for Rust-IPC results rendered from effects: keeps the last
// value while a recompute is in flight, drops resolutions/rejections from
// cancelled runs, and resets to `fallback` when the call fails so the UI never
// shows stale numbers after an error. `compute` may return a plain value to
// skip the async path (e.g. "nothing to compute" resets).
export function useCalcResult<T>(
  compute: () => Promise<T> | T,
  deps: DependencyList,
  fallback: T,
): T {
  const [result, setResult] = useState<T>(fallback)
  useEffect(() => {
    let r: Promise<T> | T
    try {
      r = compute()
    } catch {
      r = fallback
    }
    if (!(r instanceof Promise)) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setResult(r)
      return
    }
    let cancelled = false
    r.then((v) => {
      if (!cancelled) setResult(v)
    }).catch(() => {
      if (!cancelled) setResult(fallback)
    })
    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps)
  return result
}
