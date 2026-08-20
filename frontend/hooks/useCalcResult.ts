import { useEffect, useState } from 'react'
import type { DependencyList } from 'react'

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
