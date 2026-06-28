import { describe, expect, test } from 'vitest'
import type { RangedStatMap, Skill } from '../../types'
import { normalizeSkillName } from '../../utils/item/stats'
import { skillLevelBonus } from './skillBonus'

const lightningSkill = {
  id: 'thunder',
  classId: 'x',
  name: 'Thunder Fury',
  kind: 'active',
  maxRank: 20,
  ranks: [],
  damageType: 'lightning',
} as Skill

const auraSkill = {
  id: 'aura',
  classId: 'x',
  name: 'Some Aura',
  kind: 'aura',
  maxRank: 20,
  ranks: [],
} as Skill

const stats: RangedStatMap = { all_skills: 3, lightning_skills: 2 }

describe('skillLevelBonus', () => {
  test('returns [0,0] for an unallocated skill (rank 0)', () => {
    expect(skillLevelBonus(lightningSkill, 0, stats, {})).toEqual([0, 0])
  })

  test('sums +all skills, +element skills, and +named-skill item bonus', () => {
    const itemBonuses = {
      [normalizeSkillName('Thunder Fury')]: [1, 1] as [number, number],
    }
    expect(skillLevelBonus(lightningSkill, 5, stats, itemBonuses)).toEqual([6, 6])
  })

  test('omits element bonus for a skill with no damage type', () => {
    expect(skillLevelBonus(auraSkill, 1, stats, {})).toEqual([3, 3])
  })

  test('preserves ranged (min/max) bonuses', () => {
    const ranged: RangedStatMap = { all_skills: [1, 2] }
    expect(skillLevelBonus(lightningSkill, 1, ranged, {})).toEqual([1, 2])
  })
})
