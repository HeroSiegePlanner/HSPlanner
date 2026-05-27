import { useEffect, useState } from 'react'
import {
  computeSkillDamageNative,
  type NativeSkillDamageInput,
} from '../utils/nativeDamage'
import type { SkillDamageBreakdown } from '../utils/item/stats'

export function useSkillDamage(
  input: NativeSkillDamageInput | null,
): SkillDamageBreakdown | null {
  const [result, setResult] = useState<SkillDamageBreakdown | null>(null)

  useEffect(() => {
    if (!input) return
    let cancelled = false
    computeSkillDamageNative(input)
      .then((value) => {
        if (!cancelled) setResult(value)
      })
      .catch((err) => {
        if (!cancelled) {
          console.error('computeSkillDamageNative failed', err)
          setResult(null)
        }
      })
    return () => {
      cancelled = true
    }
  }, [input])

  return input ? result : null
}
