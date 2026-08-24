import type { RangedValue, Skill } from '../../types'
import { rangedMax, rangedMin } from '../item/stats'

export interface SkillRate {
  base: number
  speedMax: number
  min: number
  max: number
}

export function skillSpeedKey(skill: Skill): string {
  if (skill.usesAttackSpeed === true) return 'increased_attack_speed'
  return skill.usesSkillHaste === true ? 'skill_haste' : 'faster_cast_rate'
}

// Cooldown-gated skills fire once per cooldown, and skill haste is what shortens it.
export function skillBaseRate(
  skill: Skill,
  attacksPerSecond = 0,
): number | undefined {
  if (skill.usesAttackSpeed === true) return attacksPerSecond
  if (skill.usesSkillHaste === true && skill.baseCooldown)
    return 1 / skill.baseCooldown
  return skill.baseCastRate
}

// Mirrors the rate branch in engine/src/calc/build.rs.
export function skillRate(
  skill: Skill,
  getStat: (key: string) => RangedValue,
): SkillRate | undefined {
  const base = skillBaseRate(skill, rangedMax(getStat('attacks_per_second')))
  if (!base) return undefined

  const speed = getStat(skillSpeedKey(skill))
  return {
    base,
    speedMax: rangedMax(speed),
    min: base * (1 + rangedMin(speed) / 100),
    max: base * (1 + rangedMax(speed) / 100),
  }
}
