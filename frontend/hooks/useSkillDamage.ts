import { useEffect, useState } from 'react'
import {
  computeAttackSkillDamageNative,
  computeSkillDamageNative,
  type NativeAttackSkillDamageInput,
  type NativeSkillDamageInput,
} from '../utils/nativeDamage'
import type {
  AttackSkillDamageBreakdown,
  SkillDamageBreakdown,
} from '../utils/item/stats'

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

export function useAttackSkillDamage(
  input: NativeAttackSkillDamageInput | null,
): AttackSkillDamageBreakdown | null {
  const [result, setResult] = useState<AttackSkillDamageBreakdown | null>(null)

  useEffect(() => {
    if (!input) return
    let cancelled = false
    computeAttackSkillDamageNative(input)
      .then((value) => {
        if (!cancelled) setResult(value)
      })
      .catch((err) => {
        if (!cancelled) {
          console.error('computeAttackSkillDamageNative failed', err)
          setResult(null)
        }
      })
    return () => {
      cancelled = true
    }
  }, [input])

  return input ? result : null
}
