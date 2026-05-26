import { useEffect, useState } from 'react'
import {
  computeWeaponDamageNative,
  type NativeWeaponDamageInput,
} from '../utils/nativeDamage'
import type { WeaponDamageBreakdown } from '../utils/stats'

export function useWeaponDamage(
  input: NativeWeaponDamageInput,
): WeaponDamageBreakdown | null {
  const [result, setResult] = useState<WeaponDamageBreakdown | null>(null)

  useEffect(() => {
    let cancelled = false
    computeWeaponDamageNative(input)
      .then((value) => {
        if (!cancelled) setResult(value)
      })
      .catch((err) => {
        if (!cancelled) {
          console.error('computeWeaponDamageNative failed', err)
          setResult(null)
        }
      })
    return () => {
      cancelled = true
    }
  }, [input])

  return result
}
