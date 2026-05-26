import { useEffect, useState } from 'react'
import {
  computeWeaponDamageNative,
  type NativeWeaponDamageInput,
} from '../utils/nativeDamage'
import type { WeaponDamageBreakdown } from '../utils/stats'

// Hook expects `input` to be memoized by the caller (e.g. useMemo) so the
// effect does not re-fire on every render.
export function useWeaponDamage(
  input: NativeWeaponDamageInput,
): WeaponDamageBreakdown | null {
  const [result, setResult] = useState<WeaponDamageBreakdown | null>(null)

  useEffect(() => {
    let cancelled = false
    computeWeaponDamageNative(input).then((value) => {
      if (!cancelled) setResult(value)
    })
    return () => {
      cancelled = true
    }
  }, [input])

  return result
}
