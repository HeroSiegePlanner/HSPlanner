import { describe, expect, it } from 'vitest'
import { bonusSourceSynergy, synergyEffectText } from './synergyText'
import type { BonusSource } from '../../types'

describe('synergyText', () => {
  it('formats a skill_level bonus source as "<value> <stat> per rank"', () => {
    const bs: BonusSource = {
      source: 'Envenom',
      stat: 'poison_skill_damage',
      value: 22.5,
      per: 'skill_level',
    }
    expect(bonusSourceSynergy(bs)).toEqual({
      name: 'Envenom',
      text: '+22.5% Poison Skill Damage per rank',
    })
  })

  it('labels attribute_point sources "per point"', () => {
    const bs: BonusSource = {
      source: 'Intelligence',
      stat: 'poison_skill_damage',
      value: 7.25,
      per: 'attribute_point',
    }
    expect(bonusSourceSynergy(bs)).toEqual({
      name: 'Intelligence',
      text: '+7.25% Poison Skill Damage per point',
    })
  })

  it('exposes the same effect string SkillEffectsBlock renders and passes the source name through', () => {
    const bs: BonusSource = {
      source: 'Master Poisoner',
      stat: 'attack_damage',
      value: 10,
      per: 'skill_level',
    }
    expect(synergyEffectText(bs)).toBe(bonusSourceSynergy(bs).text)
    expect(bonusSourceSynergy(bs).name).toBe('Master Poisoner')
  })
})
