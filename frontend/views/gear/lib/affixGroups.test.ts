import { describe, expect, test } from 'vitest'
import {
  affixStatLabel,
  affixTierLabel,
  affixTiers,
  buildAffixGroups,
  describeAffixValue,
  groupBounds,
  tierIndexForValue,
} from './affixGroups'
import type { Affix } from '../../../types'

const tier = (
  n: number,
  name: string,
  valueMin: number | null,
  valueMax: number | null,
  description: string,
): Affix => ({
  id: `grp_t${n}_${name}`,
  groupId: 'grp',
  tier: n,
  name,
  description,
  statKey: 'life',
  sign: '+',
  format: 'flat',
  valueMin,
  valueMax,
})

const EXP_TIERS = [
  tier(1, 'Battlescarred', 1, 1, '+1% Increased Experience Gain'),
  tier(2, 'Fieldtested', 1, 2, '+[1-2]% Increased Experience Gain'),
  tier(3, 'Wartorned', 1, 3, '+[1-3]% Increased Experience Gain'),
  tier(4, 'Posttraumatic', 1, 4, '+[1-4]% Increased Experience Gain'),
  tier(5, 'Warlegend', 1, 5, '+[1-5]% Increased Experience Gain'),
]

const asRanges = (tiers: Affix[]) =>
  tiers.map((t) =>
    t.valueMin === null || t.valueMax === null
      ? null
      : { rangeMin: t.valueMin, rangeMax: t.valueMax },
  )

describe('affixTierLabel', () => {
  test('grades the five tiers from D up to S', () => {
    expect([1, 2, 3, 4, 5].map(affixTierLabel)).toEqual(['D', 'C', 'B', 'A', 'S'])
  })

  test('falls back to the raw tier past the grade scale', () => {
    expect(affixTierLabel(7)).toBe('T7')
  })
})

describe('affixStatLabel', () => {
  test('drops a leading value', () => {
    expect(affixStatLabel(tier(1, 'A', 5, 15, '-[5-15]% to Enemy Poison Resistance'))).toBe(
      'to Enemy Poison Resistance',
    )
    expect(affixStatLabel(tier(1, 'A', 1, 1, '+1% Increased Experience Gain'))).toBe(
      'Increased Experience Gain',
    )
  })

  test('drops a range printed mid-sentence, percent sign included', () => {
    expect(affixStatLabel(tier(1, 'A', 6, 20, 'Slows target by [6-20]%'))).toBe(
      'Slows target by',
    )
    expect(affixStatLabel(tier(1, 'A', 1, 4, 'Life Increased by [1-4]%'))).toBe(
      'Life Increased by',
    )
  })

  test('leaves a description that carries no value alone', () => {
    expect(affixStatLabel(tier(1, 'A', null, null, 'Cannot be Frozen'))).toBe(
      'Cannot be Frozen',
    )
  })
})

describe('describeAffixValue', () => {
  test('swaps a bracketed range for the rolled value', () => {
    const t = tier(1, 'A', 15, 30, '+[15-30] to Life')
    expect(describeAffixValue(t, 23)).toBe('+23 to Life')
  })

  test('swaps a single-value description, which carries no brackets', () => {
    const t = tier(1, 'A', 1, 1, '+1% Increased Experience Gain')
    expect(describeAffixValue(t, 4)).toBe('+4% Increased Experience Gain')
  })

  test('swaps a range printed mid-sentence', () => {
    const t = tier(1, 'A', 6, 20, 'Slows target by [6-20]%')
    expect(describeAffixValue(t, 13)).toBe('Slows target by 13%')
  })

  test('keeps the sign that lives outside the range token', () => {
    const t = tier(1, 'A', 2, 6, '-[2-6]% to Enemy Arcane Resistance')
    expect(describeAffixValue(t, -4)).toBe('-4% to Enemy Arcane Resistance')
  })

  test('returns null when the description has no range to swap', () => {
    expect(describeAffixValue(tier(1, 'A', null, null, 'Cannot be Frozen'), 1)).toBeNull()
    expect(describeAffixValue(tier(1, 'A', 3, 9, 'Double Jump'), 5)).toBeNull()
  })
})

describe('buildAffixGroups', () => {
  test('collapses a group into one entry spanning every tier', () => {
    const groups = buildAffixGroups(EXP_TIERS)
    expect(groups).toHaveLength(1)
    expect(groups[0]!.tiers).toHaveLength(5)
    expect(groups[0]!.label).toBe('+[1-5]% Increased Experience Gain')
    expect(groups[0]!.topTier.name).toBe('Warlegend')
  })

  test('widens a range printed mid-sentence, not just at the front', () => {
    const groups = buildAffixGroups([
      tier(1, 'A', 1, 4, 'Life Increased by [1-4]%'),
      tier(2, 'B', 5, 20, 'Life Increased by [5-20]%'),
    ])
    expect(groups[0]!.label).toBe('Life Increased by [1-20]%')
  })

  test('picks the strongest tier as the top tier even when it is not the last', () => {
    const groups = buildAffixGroups([
      tier(1, 'A', 1, 5, '+[1-5] x'),
      tier(2, 'B', 2, 99, '+[2-99] x'),
      tier(3, 'C', 3, 8, '+[3-8] x'),
    ])
    expect(groups[0]!.topTier.name).toBe('B')
  })

  test('keeps unrelated groups apart', () => {
    const other = { ...tier(1, 'Z', 1, 2, '+[1-2] z'), groupId: 'other' }
    expect(buildAffixGroups([...EXP_TIERS, other])).toHaveLength(2)
  })
})

describe('groupBounds', () => {
  test('spans the weakest and strongest reachable value', () => {
    expect(groupBounds(EXP_TIERS, asRanges(EXP_TIERS))).toEqual({ lo: 1, hi: 5 })
  })

  test('ignores tiers with no roll range', () => {
    const tiers = [tier(1, 'A', null, null, 'x'), tier(2, 'B', 2, 6, '+[2-6] x')]
    expect(groupBounds(tiers, asRanges(tiers))).toEqual({ lo: 2, hi: 6 })
  })

  test('returns null when nothing in the group rolls', () => {
    const tiers = [tier(1, 'A', null, null, 'x')]
    expect(groupBounds(tiers, asRanges(tiers))).toBeNull()
  })

  test('reports magnitudes so a negative affix grows toward its strongest tier', () => {
    const tiers = [tier(1, 'A', 2, 6, '-[2-6]% x'), tier(2, 'B', 5, 15, '-[5-15]% x')]
    expect(
      groupBounds(tiers, [
        { rangeMin: -2, rangeMax: -6 },
        { rangeMin: -5, rangeMax: -15 },
      ]),
    ).toEqual({ lo: 2, hi: 15 })
  })
})

describe('tierIndexForValue', () => {
  const ranges = asRanges(EXP_TIERS)

  test('picks the lowest tier that can reach the value', () => {
    expect(tierIndexForValue(EXP_TIERS, ranges, 1)).toBe(0)
    expect(tierIndexForValue(EXP_TIERS, ranges, 3)).toBe(2)
    expect(tierIndexForValue(EXP_TIERS, ranges, 5)).toBe(4)
  })

  test('handles overlapping tiers by preferring the cheaper one', () => {
    const tiers = [
      tier(1, 'A', 7, 15, '+[7-15] x'),
      tier(2, 'B', 16, 25, '+[16-25] x'),
      tier(3, 'C', 10, 35, '+[10-35] x'),
    ]
    expect(tierIndexForValue(tiers, asRanges(tiers), 20)).toBe(1)
    expect(tierIndexForValue(tiers, asRanges(tiers), 30)).toBe(2)
  })

  test('snaps an unreachable value to the nearest tier', () => {
    const tiers = [tier(1, 'A', 1, 5, '+[1-5] x'), tier(2, 'B', 20, 40, '+[20-40] x')]
    expect(tierIndexForValue(tiers, asRanges(tiers), 8)).toBe(0)
    expect(tierIndexForValue(tiers, asRanges(tiers), 17)).toBe(1)
  })

  test('works on negative affixes, which store magnitudes', () => {
    const tiers = [tier(1, 'A', 2, 6, '-[2-6]% x'), tier(2, 'B', 5, 15, '-[5-15]% x')]
    expect(
      tierIndexForValue(
        tiers,
        [
          { rangeMin: -2, rangeMax: -6 },
          { rangeMin: -5, rangeMax: -15 },
        ],
        -12,
      ),
    ).toBe(1)
  })

  test('returns -1 when no tier rolls', () => {
    const tiers = [tier(1, 'A', null, null, 'x')]
    expect(tierIndexForValue(tiers, asRanges(tiers), 3)).toBe(-1)
  })
})

describe('affixTiers', () => {
  test('returns every tier of a real affix group, tier-ordered', () => {
    const tiers = affixTiers('1_increased_experience_gain_t3_wartorned')
    expect(tiers.map((t) => t.tier)).toEqual([1, 2, 3, 4, 5])
    expect(tiers[0]!.name).toBe('Battlescarred')
  })

  test('returns nothing for an unknown id', () => {
    expect(affixTiers('nope')).toEqual([])
  })
})
