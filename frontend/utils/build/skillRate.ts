import type { RangedValue, Skill } from '../../types'
import { rangedMax, rangedMin } from '../item/stats'

export interface SkillRate {
  base: number
  speedMax: number
  min: number
  max: number
}

// Mirrors the rate branch in engine/src/calc/build.rs.
export function skillRate(
  skill: Skill,
  getStat: (key: string) => RangedValue,
): SkillRate | undefined {
  const byAttackSpeed = skill.usesAttackSpeed === true
  const base = byAttackSpeed
    ? rangedMax(getStat('attacks_per_second'))
    : skill.baseCastRate
  if (!base) return undefined

  const speed = getStat(
    byAttackSpeed ? 'increased_attack_speed' : 'faster_cast_rate',
  )
  return {
    base,
    speedMax: rangedMax(speed),
    min: base * (1 + rangedMin(speed) / 100),
    max: base * (1 + rangedMax(speed) / 100),
  }
}
