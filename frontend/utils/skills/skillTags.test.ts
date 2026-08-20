import { describe, expect, it } from 'vitest'
import {
  effectiveSkillTags,
  tagSkillBonuses,
  visibleEffectiveSkillTags,
  visibleSkillTags,
} from './skillTags'

// death_from_above w data/subskill-tags.json: ancient_device add Sentry, remove Area of Effect
const DFA = {
  id: 'death_from_above',
  damageType: 'poison',
  tags: ['Cast', 'Active', 'Spell', 'Area of Effect', 'Poison'],
}

describe('effectiveSkillTags', () => {
  it('returns base tags when no tag-changing subskill is taken', () => {
    expect(effectiveSkillTags(DFA, {})).toEqual(DFA.tags)
  })

  it('applies add and remove from a taken subskill', () => {
    const tags = effectiveSkillTags(DFA, { 'death_from_above:ancient_device': 1 })
    expect(tags).toContain('Sentry')
    expect(tags).not.toContain('Area of Effect')
  })
})

describe('visibleEffectiveSkillTags', () => {
  it('marks added and removed tags and keeps the damage-type filter', () => {
    const view = visibleEffectiveSkillTags(DFA, {
      'death_from_above:ancient_device': 1,
    })
    expect(view.tags).toContain('Sentry')
    expect(view.added.has('Sentry')).toBe(true)
    expect(view.removed).toEqual(['Area of Effect'])
    expect(view.tags).not.toContain('Poison')
  })

  it('reports nothing added or removed without ranks', () => {
    const view = visibleEffectiveSkillTags(DFA, {})
    expect(view.added.size).toBe(0)
    expect(view.removed).toEqual([])
    expect(view.tags).toEqual(['Cast', 'Active', 'Spell', 'Area of Effect'])
  })
})

describe('visibleSkillTags', () => {
  it('drops the tag that duplicates the damage type pill', () => {
    expect(
      visibleSkillTags({
        damageType: 'poison',
        tags: ['Attack', 'Active', 'Poison', 'Physical'],
      }),
    ).toEqual(['Attack', 'Active', 'Physical'])
  })

  it('keeps all tags when there is no damage type', () => {
    expect(visibleSkillTags({ tags: ['Passive'] })).toEqual(['Passive'])
  })

  it('returns empty array when tags are missing', () => {
    expect(visibleSkillTags({ damageType: 'fire' })).toEqual([])
  })
})

describe('tagSkillBonuses', () => {
  const stats = { sentry_skills: [6, 8] as [number, number], summon_skills: 3 }

  it('lists the tag rank affixes the skill tags unlock', () => {
    expect(tagSkillBonuses(['Spell', 'Sentry'], stats)).toEqual([
      { key: 'sentry_skills', label: 'Sentry', value: [6, 8] },
    ])
  })

  it('skips affixes whose tag the skill does not carry', () => {
    expect(tagSkillBonuses(['Spell', 'Projectile'], stats)).toEqual([])
  })

  it('drops rows the build has no points in', () => {
    expect(tagSkillBonuses(['Summon'], { sentry_skills: [6, 8] })).toEqual([])
  })
})
