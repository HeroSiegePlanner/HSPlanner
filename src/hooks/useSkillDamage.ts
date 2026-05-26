import { useEffect, useState } from 'react'
import {
  computeSkillDamageNative,
  type NativeSkillDamageInput,
} from '../utils/nativeDamage'
import type { SkillDamageBreakdown } from '../utils/stats'

// Hook expects `input` to be memoized by the caller (e.g. useMemo) so the
// effect does not re-fire on every render.
export function useSkillDamage(
  input: NativeSkillDamageInput | null,
): SkillDamageBreakdown | null {
  const [result, setResult] = useState<SkillDamageBreakdown | null>(null)

  useEffect(() => {
    if (!input) return
    let cancelled = false
    computeSkillDamageNative(input).then((value) => {
      if (!cancelled) setResult(value)
    })
    return () => {
      cancelled = true
    }
  }, [input])

  return input ? result : null
}
