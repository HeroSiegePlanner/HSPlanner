import { describe, expect, it } from 'vitest'
import {
  FLAT_SKILL_STAIRCASE,
  ITEM_SPECIFIC_STAIRCASE,
  getStarScaleConfig,
  isStatStarImmune,
  itemGrantedSkillRankFlatBonus,
  statStarFlatBonus,
  statStarPercentMultiplier,
} from './starScaling'

describe('FLAT_SKILL_STAIRCASE', () => {
  it('matches the documented "3*=+1, 5*=+2" progression', () => {
    expect(FLAT_SKILL_STAIRCASE[0]).toBe(0)
    expect(FLAT_SKILL_STAIRCASE[1]).toBe(0)
    expect(FLAT_SKILL_STAIRCASE[2]).toBe(0)
    expect(FLAT_SKILL_STAIRCASE[3]).toBe(1)
    expect(FLAT_SKILL_STAIRCASE[4]).toBe(1)
    expect(FLAT_SKILL_STAIRCASE[5]).toBe(2)
  })
})

describe('ITEM_SPECIFIC_STAIRCASE', () => {
  it('matches the documented "2*=+1, 4*=+2, 5*=+3" progression', () => {
    expect(ITEM_SPECIFIC_STAIRCASE[0]).toBe(0)
    expect(ITEM_SPECIFIC_STAIRCASE[1]).toBe(0)
    expect(ITEM_SPECIFIC_STAIRCASE[2]).toBe(1)
    expect(ITEM_SPECIFIC_STAIRCASE[3]).toBe(1)
    expect(ITEM_SPECIFIC_STAIRCASE[4]).toBe(2)
    expect(ITEM_SPECIFIC_STAIRCASE[5]).toBe(3)
  })
})

describe('getStarScaleConfig', () => {
  it('returns "none" when the stat key is null', () => {
    expect(getStarScaleConfig(null)).toEqual({ kind: 'none' })
  })

  it('returns "none" when the stat key is an empty string', () => {
    expect(getStarScaleConfig('')).toEqual({ kind: 'none' })
  })

  it('returns "none" for stat keys that are not in the map', () => {
    expect(getStarScaleConfig('totally_made_up_stat')).toEqual({ kind: 'none' })
  })

  it('returns the percent config for a percent-scaled stat', () => {
    expect(getStarScaleConfig('to_strength')).toEqual({
      kind: 'percent',
      perStar: 5,
    })
    expect(getStarScaleConfig('fire_skill_damage')).toEqual({
      kind: 'percent',
      perStar: 4,
    })
  })

  it('returns the flat-skill-staircase kind for elemental_skills affixes', () => {
    expect(getStarScaleConfig('fire_skills')).toEqual({
      kind: 'flat-skill-staircase',
    })
    expect(getStarScaleConfig('cold_skills')).toEqual({
      kind: 'flat-skill-staircase',
    })
  })

  it('returns "none" for explicitly disabled scaling (e.g. all_skills, defense)', () => {
    expect(getStarScaleConfig('all_skills')).toEqual({ kind: 'none' })
    expect(getStarScaleConfig('defense')).toEqual({ kind: 'none' })
  })

  it('returns "unknown" for documented-but-untested entries', () => {
    expect(getStarScaleConfig('max_fire_resistance')).toEqual({ kind: 'unknown' })
  })

  it('returns "glitch" for stats flagged as broken in-game', () => {
    expect(getStarScaleConfig('magic_damage_reduction')).toEqual({ kind: 'glitch' })
    expect(getStarScaleConfig('physical_damage_reduction')).toEqual({ kind: 'glitch' })
  })
})

describe('isStatStarImmune', () => {
  it('treats null as immune (defaults to "none")', () => {
    expect(isStatStarImmune(null)).toBe(true)
  })

  it('returns false for percent-scaled stats', () => {
    expect(isStatStarImmune('to_strength')).toBe(false)
    expect(isStatStarImmune('crit_damage')).toBe(false)
  })

  it('returns false for flat-skill-staircase stats', () => {
    expect(isStatStarImmune('fire_skills')).toBe(false)
  })

  it('returns true for stats explicitly marked as "none"', () => {
    expect(isStatStarImmune('all_skills')).toBe(true)
    expect(isStatStarImmune('mana_cost_reduction')).toBe(true)
  })

  it('returns true for "unknown" and "glitch" stats', () => {
    expect(isStatStarImmune('max_fire_resistance')).toBe(true)
    expect(isStatStarImmune('magic_damage_reduction')).toBe(true)
  })
})

describe('statStarPercentMultiplier', () => {
  it('returns 1 when stars is 0 or undefined', () => {
    expect(statStarPercentMultiplier('to_strength', 0)).toBe(1)
    expect(statStarPercentMultiplier('to_strength', undefined)).toBe(1)
  })

  it('returns 1 for negative star counts (treated as no stars)', () => {
    expect(statStarPercentMultiplier('to_strength', -3)).toBe(1)
  })

  it('returns 1 for stats that do not scale percentually', () => {
    // flat-skill-staircase stats get their bonus through statStarFlatBonus,
    // not through the percent multiplier.
    expect(statStarPercentMultiplier('fire_skills', 5)).toBe(1)
    expect(statStarPercentMultiplier('all_skills', 5)).toBe(1) // none
    expect(statStarPercentMultiplier('max_fire_resistance', 5)).toBe(1) // unknown
    expect(statStarPercentMultiplier('magic_damage_reduction', 5)).toBe(1) // glitch
  })

  it('returns 1 for unknown stat keys (treated as "none")', () => {
    expect(statStarPercentMultiplier('totally_made_up_stat', 5)).toBe(1)
  })

  it('returns 1 + stars*perStar/100 for percent stats', () => {
    // to_strength has perStar=5: 1 star = 1.05, 5 stars = 1.25
    expect(statStarPercentMultiplier('to_strength', 1)).toBeCloseTo(1.05, 10)
    expect(statStarPercentMultiplier('to_strength', 5)).toBeCloseTo(1.25, 10)
  })

  it('uses the per-stat perStar value rather than a global default', () => {
    // fire_skill_damage perStar=4: 3 stars = 1.12
    expect(statStarPercentMultiplier('fire_skill_damage', 3)).toBeCloseTo(1.12, 10)
    // crit_damage perStar=4: 5 stars = 1.20
    expect(statStarPercentMultiplier('crit_damage', 5)).toBeCloseTo(1.2, 10)
    // life_steal perStar=2: 5 stars = 1.10
    expect(statStarPercentMultiplier('life_steal', 5)).toBeCloseTo(1.1, 10)
  })
})

describe('statStarFlatBonus', () => {
  it('returns 0 when stars is 0 or undefined', () => {
    expect(statStarFlatBonus('fire_skills', 0)).toBe(0)
    expect(statStarFlatBonus('fire_skills', undefined)).toBe(0)
  })

  it('returns 0 for negative star counts', () => {
    expect(statStarFlatBonus('fire_skills', -1)).toBe(0)
  })

  it('returns 0 for percent-scaled stats (those use the multiplier instead)', () => {
    expect(statStarFlatBonus('to_strength', 5)).toBe(0)
    expect(statStarFlatBonus('fire_skill_damage', 5)).toBe(0)
  })

  it('returns 0 for explicitly disabled stats', () => {
    expect(statStarFlatBonus('all_skills', 5)).toBe(0)
    expect(statStarFlatBonus('max_fire_resistance', 5)).toBe(0) // unknown
  })

  it('returns the FLAT_SKILL_STAIRCASE bonus for elemental_skills affixes', () => {
    expect(statStarFlatBonus('fire_skills', 2)).toBe(0)
    expect(statStarFlatBonus('fire_skills', 3)).toBe(1)
    expect(statStarFlatBonus('fire_skills', 4)).toBe(1)
    expect(statStarFlatBonus('fire_skills', 5)).toBe(2)
    expect(statStarFlatBonus('arcane_skills', 5)).toBe(2)
  })

  it('returns 0 for star counts outside the documented staircase (>5)', () => {
    // The staircase records only cover 0..5; anything higher is treated as 0
    // rather than extrapolated.
    expect(statStarFlatBonus('fire_skills', 7)).toBe(0)
  })
})

describe('itemGrantedSkillRankFlatBonus', () => {
  it('returns 0 when stars is 0 or undefined', () => {
    expect(itemGrantedSkillRankFlatBonus(0)).toBe(0)
    expect(itemGrantedSkillRankFlatBonus(undefined)).toBe(0)
  })

  it('returns 0 for negative star counts', () => {
    expect(itemGrantedSkillRankFlatBonus(-2)).toBe(0)
  })

  it('follows the documented (2*=+1, 4*=+2, 5*=+3) progression', () => {
    expect(itemGrantedSkillRankFlatBonus(1)).toBe(0)
    expect(itemGrantedSkillRankFlatBonus(2)).toBe(1)
    expect(itemGrantedSkillRankFlatBonus(3)).toBe(1)
    expect(itemGrantedSkillRankFlatBonus(4)).toBe(2)
    expect(itemGrantedSkillRankFlatBonus(5)).toBe(3)
  })

  it('returns 0 for star counts above the documented range', () => {
    expect(itemGrantedSkillRankFlatBonus(6)).toBe(0)
    expect(itemGrantedSkillRankFlatBonus(99)).toBe(0)
  })
})
