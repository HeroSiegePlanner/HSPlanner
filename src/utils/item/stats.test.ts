import { describe, expect, it } from 'vitest'
import type { RangedStatMap } from '../types'
import {
  effectiveCap,
  fmtStats,
  formatAffixRangeFromValues,
  formatValue,
  isZero,
  normalizeSkillName,
  rangedMax,
  rangedMin,
  shouldScaleImplicit,
  statName,
} from './stats'

// Affix/star math (rolled values, star scaling, item skill bonuses) moved to
// Rust — covered by src-tauri/src/calc/{affix,star_scaling,rank}.rs tests.

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
// formatAffixRangeFromValues — string formatting over Rust-computed ranges
// -----------------------------------------------------------------------

describe('formatAffixRangeFromValues', () => {
  it('returns just the sign when min/max are null or no range is available', () => {
    expect(
      formatAffixRangeFromValues(
        { sign: '+', format: 'flat', valueMin: null, valueMax: null },
        { rangeMin: 0, rangeMax: 0 },
      ),
    ).toBe('+')
    expect(
      formatAffixRangeFromValues(
        { sign: '+', format: 'flat', valueMin: 1, valueMax: 2 },
        null,
      ),
    ).toBe('+')
  })

  it('formats degenerate ranges as a single signed value', () => {
    expect(
      formatAffixRangeFromValues(
        { sign: '+', format: 'flat', valueMin: 5, valueMax: 5 },
        { rangeMin: 5, rangeMax: 5 },
      ),
    ).toBe('+5')
  })

  it('formats ranges as +[lo-hi] with the right suffix', () => {
    expect(
      formatAffixRangeFromValues(
        { sign: '+', format: 'percent', valueMin: 10, valueMax: 20 },
        { rangeMin: 10, rangeMax: 20 },
      ),
    ).toBe('+[10-20]%')
  })

  it('flips to "-" for negative ranges (rangeMin/Max are roll-0/roll-1 values)', () => {
    expect(
      formatAffixRangeFromValues(
        { sign: '-', format: 'flat', valueMin: 3, valueMax: 9 },
        { rangeMin: -3, rangeMax: -9 },
      ),
    ).toBe('-[3-9]')
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
