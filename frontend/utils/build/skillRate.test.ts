import { describe, expect, it } from 'vitest'
import { skillRate } from './skillRate'
import type { Skill } from '../../types'

const orbOfFrost: Skill = {
  id: 'orb_of_frost',
  classId: 'jotunn',
  name: 'Orb of Frost',
  kind: 'active',
  maxRank: 20,
  ranks: [],
  usesSkillHaste: true,
  baseCooldown: 1.75,
} as Skill

describe('skillRate', () => {
  it('derives the rate of a cooldown-gated skill from skill haste, ignoring faster cast rate', () => {
    const rate = skillRate(orbOfFrost, (key) =>
      key === 'skill_haste' ? 75 : 500,
    )

    expect(rate?.base).toBeCloseTo(1 / 1.75)
    expect(rate?.max).toBeCloseTo(1)
  })

  it('keeps faster cast rate for plain cast skills', () => {
    const bolt = { ...orbOfFrost, usesSkillHaste: false, baseCastRate: 2 }
    const rate = skillRate(bolt, (key) =>
      key === 'faster_cast_rate' ? 50 : 999,
    )

    expect(rate?.base).toBe(2)
    expect(rate?.max).toBeCloseTo(3)
  })
})
