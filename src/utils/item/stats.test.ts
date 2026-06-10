import { describe, expect, it } from 'vitest'
import type { Inventory, RangedStatMap } from '../types'
import {
  aggregateItemSkillBonuses,
  applyStarsToRangedValue,
  effectiveCap,
  fmtStats,
  formatAffixRange,
  formatValue,
  isAffixStarImmune,
  isZero,
  normalizeSkillName,
  rangedMax,
  rangedMin,
  rolledAffixRange,
  rolledAffixValue,
  rolledAffixValueWithStars,
  shouldScaleImplicit,
  statName,
} from './stats'

// -----------------------------------------------------------------------
// normalizeSkillName
// -----------------------------------------------------------------------

describe('normalizeSkillName', () => {
  it('trims and lowercases', () => {
    expect(normalizeSkillName('  Charged Bolts  ')).toBe('charged bolts')
  })

  it('is idempotent on already-normalised names', () => {
    expect(normalizeSkillName('charged bolts')).toBe('charged bolts')
  })
})

// -----------------------------------------------------------------------
// rangedMin / rangedMax / isZero
// -----------------------------------------------------------------------

describe('rangedMin / rangedMax', () => {
  it('returns the number itself for scalar RangedValue', () => {
    expect(rangedMin(7)).toBe(7)
    expect(rangedMax(7)).toBe(7)
  })

  it('returns the bounds for tuple RangedValue', () => {
    expect(rangedMin([3, 9])).toBe(3)
    expect(rangedMax([3, 9])).toBe(9)
  })
})

describe('isZero', () => {
  it('is true for scalar 0', () => {
    expect(isZero(0)).toBe(true)
  })

  it('is true for [0, 0]', () => {
    expect(isZero([0, 0])).toBe(true)
  })

  it('is false for scalars or tuples with any nonzero', () => {
    expect(isZero(1)).toBe(false)
    expect(isZero([0, 1])).toBe(false)
    expect(isZero([-1, 0])).toBe(false)
  })
})

// -----------------------------------------------------------------------
// statName / effectiveCap / formatValue / fmtStats
// -----------------------------------------------------------------------

describe('statName', () => {
  it('falls back to the raw key when no definition exists', () => {
    expect(statName('totally_made_up_stat')).toBe('totally_made_up_stat')
  })

  it('prefixes "Total" for _more synthetic keys', () => {
    // increased_attack_speed has a stat def. The _more variant gets a
    // "Total" prefix to distinguish the multiplicative aggregate.
    const name = statName('increased_attack_speed_more')
    expect(name.startsWith('Total ')).toBe(true)
  })
})

describe('effectiveCap', () => {
  it('returns undefined when the stat has no documented cap', () => {
    expect(effectiveCap('totally_made_up_stat', {})).toBeUndefined()
  })

  it('adds the max_<key> mod to the base cap', () => {
    // fire_resistance has cap=75. max_fire_resistance bumps it.
    const stats: RangedStatMap = { max_fire_resistance: 5 }
    const cap = effectiveCap('fire_resistance', stats)
    expect(cap).toBe(80)
  })
})

describe('formatValue / fmtStats', () => {
  it('formats positive integers with a + sign', () => {
    expect(formatValue(42, 'totally_made_up_stat')).toBe('+42')
  })

  it('formats negative integers without an extra +', () => {
    expect(formatValue(-3, 'totally_made_up_stat')).toBe('-3')
  })

  it('formats percentages with % suffix when the stat def says so', () => {
    expect(formatValue(25, 'crit_chance')).toBe('+25%')
  })

  it('collapses tuples whose min equals max to a single value', () => {
    expect(formatValue([5, 5], 'totally_made_up_stat')).toBe('+5')
  })

  it('formats genuine tuple ranges as +[lo-hi]', () => {
    expect(formatValue([3, 9], 'totally_made_up_stat')).toBe('+[3-9]')
  })

  it('joins multiple entries in fmtStats', () => {
    const joined = fmtStats({ totally_made_up_stat: 5 })
    expect(joined).toBe('+5 totally_made_up_stat')
  })
})

// -----------------------------------------------------------------------
// Affix rolling
// -----------------------------------------------------------------------

describe('rolledAffixValue', () => {
  const baseAffix = {
    sign: '+' as const,
    format: 'flat' as const,
    valueMin: 10,
    valueMax: 20,
  }

  it('returns 0 when the affix has no documented range', () => {
    expect(rolledAffixValue({ ...baseAffix, valueMin: null, valueMax: null }, 0.5)).toBe(0)
  })

  it('returns the exact value when min equals max (no lerp)', () => {
    expect(rolledAffixValue({ ...baseAffix, valueMin: 7, valueMax: 7 }, 0.5)).toBe(7)
  })

  it('lerps roll=0 to min and roll=1 to max', () => {
    expect(rolledAffixValue(baseAffix, 0)).toBe(10)
    expect(rolledAffixValue(baseAffix, 1)).toBe(20)
  })

  it('rounds flat-format values to integers', () => {
    // roll=0.5 → 15 exactly, which is already integer
    expect(rolledAffixValue(baseAffix, 0.5)).toBe(15)
    // odd roll exercises rounding
    expect(rolledAffixValue(baseAffix, 0.33)).toBe(13) // 10 + 10*0.33 = 13.3 → 13
  })

  it('preserves fractional values for percent format', () => {
    const percentAffix = { ...baseAffix, format: 'percent' as const, valueMin: 1, valueMax: 2 }
    expect(rolledAffixValue(percentAffix, 0.5)).toBe(1.5)
  })

  it('negates the result when the sign is "-"', () => {
    expect(rolledAffixValue({ ...baseAffix, sign: '-' }, 1)).toBe(-20)
  })
})

describe('rolledAffixRange', () => {
  it('returns 0 when the affix has no documented range', () => {
    expect(
      rolledAffixRange({
        sign: '+',
        format: 'flat',
        valueMin: null,
        valueMax: null,
      }),
    ).toBe(0)
  })

  it('collapses to a scalar when min equals max', () => {
    expect(
      rolledAffixRange({ sign: '+', format: 'flat', valueMin: 5, valueMax: 5 }),
    ).toBe(5)
  })

  it('returns [min, max] for non-degenerate ranges', () => {
    expect(
      rolledAffixRange({ sign: '+', format: 'flat', valueMin: 3, valueMax: 9 }),
    ).toEqual([3, 9])
  })

  it('flips the sign and order for "-" sign (so the range stays ascending)', () => {
    expect(
      rolledAffixRange({ sign: '-', format: 'flat', valueMin: 3, valueMax: 9 }),
    ).toEqual([-9, -3])
  })
})

describe('isAffixStarImmune', () => {
  it('delegates to starScaling.isStatStarImmune', () => {
    expect(isAffixStarImmune('to_strength')).toBe(false)
    expect(isAffixStarImmune('totally_made_up_stat')).toBe(true)
  })
})

describe('rolledAffixValueWithStars', () => {
  const percentAffix = {
    sign: '+' as const,
    format: 'percent' as const,
    valueMin: 10,
    valueMax: 20,
    statKey: 'to_strength',
  }

  it('returns base value when stars is 0 / undefined', () => {
    expect(rolledAffixValueWithStars(percentAffix, 1, 0)).toBe(20)
    expect(rolledAffixValueWithStars(percentAffix, 1, undefined)).toBe(20)
  })

  it('applies the percent multiplier and floors when stars are active', () => {
    // roll=1 → base 20; 5 stars * 5% = 25% bonus → 20 * 1.25 = 25
    expect(rolledAffixValueWithStars(percentAffix, 1, 5)).toBe(25)
  })

  it('returns 0 when both base and flat bonus are 0', () => {
    expect(
      rolledAffixValueWithStars(
        { ...percentAffix, valueMin: 0, valueMax: 0 },
        1,
        3,
      ),
    ).toBe(0)
  })

  it('adds the flat-skill-staircase bonus for elemental_skills affixes', () => {
    // fire_skills is flat-skill-staircase. 3 stars adds +1 flat, no percent.
    const flatAffix = {
      sign: '+' as const,
      format: 'flat' as const,
      valueMin: 1,
      valueMax: 1,
      statKey: 'fire_skills',
    }
    expect(rolledAffixValueWithStars(flatAffix, 1, 3)).toBe(2) // 1 + 1
    expect(rolledAffixValueWithStars(flatAffix, 1, 5)).toBe(3) // 1 + 2
  })
})

describe('formatAffixRange', () => {
  it('returns just the sign when min/max are null', () => {
    expect(
      formatAffixRange({
        sign: '+',
        format: 'flat',
        valueMin: null,
        valueMax: null,
        statKey: null,
      }),
    ).toBe('+')
  })

  it('formats degenerate ranges as a single signed value', () => {
    expect(
      formatAffixRange({
        sign: '+',
        format: 'flat',
        valueMin: 5,
        valueMax: 5,
        statKey: null,
      }),
    ).toBe('+5')
  })

  it('formats ranges as +[lo-hi] with the right suffix', () => {
    expect(
      formatAffixRange({
        sign: '+',
        format: 'percent',
        valueMin: 10,
        valueMax: 20,
        statKey: null,
      }),
    ).toBe('+[10-20]%')
  })

  it('flips to "-" when both bounds end up negative after star scaling', () => {
    expect(
      formatAffixRange({
        sign: '-',
        format: 'flat',
        valueMin: 3,
        valueMax: 9,
        statKey: null,
      }),
    ).toBe('-[3-9]')
  })
})

describe('applyStarsToRangedValue', () => {
  it('returns the value unchanged when stars is 0 / undefined', () => {
    expect(applyStarsToRangedValue(10, 'to_strength', 0)).toBe(10)
    expect(applyStarsToRangedValue([5, 7], 'to_strength', undefined)).toEqual([5, 7])
  })

  it('applies the percent multiplier and floors scalar values', () => {
    // 100 * (1 + 5 * 5/100) = 125
    expect(applyStarsToRangedValue(100, 'to_strength', 5)).toBe(125)
  })

  it('applies the multiplier element-wise to tuples', () => {
    // [100, 200] * 1.25 = [125, 250]
    expect(applyStarsToRangedValue([100, 200], 'to_strength', 5)).toEqual([125, 250])
  })

  it('routes item_granted_skill_rank through the item-specific staircase', () => {
    // The synthetic key uses ITEM_SPECIFIC_STAIRCASE: 5 stars adds +3.
    expect(applyStarsToRangedValue(1, 'item_granted_skill_rank', 5)).toBe(4)
  })

  it('returns the value unchanged when neither multiplier nor flat bonus applies', () => {
    // all_skills has kind: 'none' so stars must not change it.
    expect(applyStarsToRangedValue(5, 'all_skills', 5)).toBe(5)
  })
})

describe('shouldScaleImplicit', () => {
  it('scales for non-runeword items', () => {
    expect(shouldScaleImplicit(false)).toBe(true)
  })

  it('does not scale for runeword items', () => {
    expect(shouldScaleImplicit(true)).toBe(false)
  })
})

// -----------------------------------------------------------------------
// aggregateItemSkillBonuses
// -----------------------------------------------------------------------

describe('aggregateItemSkillBonuses', () => {
  it('returns an empty map for an empty inventory', () => {
    expect(aggregateItemSkillBonuses({} as Inventory)).toEqual({})
  })
})
