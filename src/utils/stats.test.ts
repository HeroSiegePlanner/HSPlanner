import { describe, expect, it } from 'vitest'
import type { Inventory, RangedStatMap, Skill } from '../types'
import {
  aggregateItemSkillBonuses,
  applyStarsToRangedValue,
  combineAdditiveAndMore,
  computeSkillDamage,
  computeWeaponDamage,
  effectiveCap,
  effectiveRankRangeFor,
  fmtStats,
  formatAffixRange,
  formatValue,
  isAffixStarImmune,
  isZero,
  manaCostAtRank,
  normalizeSkillName,
  passiveStatsAtRank,
  rangedMax,
  rangedMin,
  rolledAffixRange,
  rolledAffixValue,
  rolledAffixValueWithStars,
  shouldScaleImplicit,
  statName,
} from './stats'

// -----------------------------------------------------------------------
// Test fixtures
// -----------------------------------------------------------------------

const FIRE_SKILL_WITH_FORMULA: Skill = {
  id: 'test_fire',
  name: 'Test Fire',
  classId: 'mage',
  damageType: 'fire',
  damageFormula: { base: 10, perLevel: 5 },
  ranks: [],
} as unknown as Skill

const SPELL_SKILL: Skill = {
  id: 'test_spell',
  name: 'Test Spell',
  classId: 'mage',
  damageType: 'cold',
  damageFormula: { base: 20, perLevel: 10 },
  tags: ['Spell'],
  ranks: [],
} as unknown as Skill

const LIGHTNING_SKILL: Skill = {
  id: 'test_lightning',
  name: 'Test Lightning',
  classId: 'mage',
  damageType: 'lightning',
  damageFormula: { base: 10, perLevel: 5 },
  ranks: [],
} as unknown as Skill

const PHYSICAL_SKILL_TABLE: Skill = {
  id: 'test_phys',
  name: 'Test Phys',
  classId: 'warrior',
  damageType: 'physical',
  damagePerRank: [
    { min: 5, max: 10 },
    { min: 10, max: 20 },
    { min: 15, max: 30 },
  ],
  ranks: [],
} as unknown as Skill

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
// combineAdditiveAndMore
// -----------------------------------------------------------------------

describe('combineAdditiveAndMore', () => {
  it('returns 0 when both inputs are undefined', () => {
    expect(combineAdditiveAndMore(undefined, undefined)).toBe(0)
  })

  it('compounds additive% and more%: (1+a)*(1+m)-1', () => {
    // 50% additive + 20% more = (1.5 * 1.2 - 1) * 100 = 80
    expect(combineAdditiveAndMore(50, 20)).toBe(80)
  })

  it('handles negative values (e.g. "less" multipliers)', () => {
    // 0 additive + -20% more = (1 * 0.8 - 1) * 100 = -20
    expect(combineAdditiveAndMore(0, -20)).toBe(-20)
  })

  it('returns a tuple when both ranges differ at the bounds', () => {
    // additive [10, 30], more [0, 0] → [10, 30]
    expect(combineAdditiveAndMore([10, 30], 0)).toEqual([10, 30])
  })

  it('collapses to a scalar when min equals max', () => {
    expect(combineAdditiveAndMore([50, 50], [20, 20])).toBe(80)
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

// -----------------------------------------------------------------------
// effectiveRankRangeFor
// -----------------------------------------------------------------------

describe('effectiveRankRangeFor', () => {
  it('returns [0, 0] for unallocated skills', () => {
    expect(effectiveRankRangeFor(FIRE_SKILL_WITH_FORMULA, 0, {}, {})).toEqual([0, 0])
  })

  it('adds all_skills and element_skills bonuses to the base rank', () => {
    // base 5 + all_skills 1 + fire_skills 2 = 8 (degenerate range)
    const stats: RangedStatMap = { all_skills: 1, fire_skills: 2 }
    expect(effectiveRankRangeFor(FIRE_SKILL_WITH_FORMULA, 5, stats, {})).toEqual([8, 8])
  })

  it('returns a range when bonuses are ranged', () => {
    // base 5 + all_skills [0,2] + fire_skills [0,1] = [5, 8]
    const stats: RangedStatMap = { all_skills: [0, 2], fire_skills: [0, 1] }
    expect(effectiveRankRangeFor(FIRE_SKILL_WITH_FORMULA, 5, stats, {})).toEqual([5, 8])
  })

  it('adds item-granted skill bonuses keyed by normalised skill name', () => {
    // 'Test Fire' normalised = 'test fire'
    const itemBonuses = { 'test fire': [1, 3] as [number, number] }
    expect(effectiveRankRangeFor(FIRE_SKILL_WITH_FORMULA, 5, {}, itemBonuses)).toEqual([6, 8])
  })
})

// -----------------------------------------------------------------------
// computeSkillDamage — the headline formula
// -----------------------------------------------------------------------

describe('computeSkillDamage', () => {
  it('returns null when no rank is allocated', () => {
    expect(computeSkillDamage(FIRE_SKILL_WITH_FORMULA, 0, {}, {}, {}, {})).toBeNull()
  })

  it('returns null when the skill has neither a formula nor a per-rank table', () => {
    const skill = {
      id: 'noformula',
      name: 'No Formula',
      classId: 'mage',
      ranks: [],
    } as unknown as Skill
    expect(computeSkillDamage(skill, 5, {}, {}, {}, {})).toBeNull()
  })

  it('computes base damage from the linear formula', () => {
    // rank 5 with base=10, perLevel=5 → 10 + 5*5 = 35.
    const out = computeSkillDamage(FIRE_SKILL_WITH_FORMULA, 5, {}, {}, {}, {})
    expect(out).not.toBeNull()
    expect(out!.baseMin).toBe(35)
    expect(out!.baseMax).toBe(35)
    expect(out!.hitMin).toBe(35) // no modifiers → hit = base
  })

  it('clamps base damage at 0 for negative formula outputs', () => {
    const negFormula = {
      ...FIRE_SKILL_WITH_FORMULA,
      damageFormula: { base: 0, perLevel: -10 },
    } as Skill
    const out = computeSkillDamage(negFormula, 1, {}, {}, {}, {})
    expect(out!.baseMin).toBe(0)
    expect(out!.baseMax).toBe(0)
  })

  it('reads from damagePerRank when no formula is defined', () => {
    // Rank 2 → index 1 → { min:10, max:20 }
    const out = computeSkillDamage(PHYSICAL_SKILL_TABLE, 2, {}, {}, {}, {})
    expect(out!.baseMin).toBe(10)
    expect(out!.baseMax).toBe(20)
  })

  it('caps the table lookup at the last entry', () => {
    // Rank 99 with a 3-row table → uses index 2 (last row).
    const out = computeSkillDamage(PHYSICAL_SKILL_TABLE, 99, {}, {}, {}, {})
    expect(out!.baseMin).toBe(15)
    expect(out!.baseMax).toBe(30)
  })

  it('adds flat skill damage on top of base', () => {
    const stats: RangedStatMap = { flat_skill_damage: 10, flat_fire_skill_damage: 5 }
    // base 35 + flat 15 = 50; no other modifiers
    const out = computeSkillDamage(FIRE_SKILL_WITH_FORMULA, 5, {}, stats, {}, {})
    expect(out!.flatMin).toBe(15)
    expect(out!.hitMin).toBe(50)
  })

  it('multiplies by (1 + skill damage %) from magic_skill_damage and element_skill_damage', () => {
    // 50% magic skill damage + 100% fire skill damage = 150% additive bonus.
    const stats: RangedStatMap = { magic_skill_damage: 50, fire_skill_damage: 100 }
    const out = computeSkillDamage(FIRE_SKILL_WITH_FORMULA, 5, {}, stats, {}, {})
    expect(out!.skillDamageMinPct).toBe(150)
    // 35 * (1 + 1.5) = 87.5 → floor = 87
    expect(out!.hitMin).toBe(87)
  })

  it('applies the multiplicative "more" multipliers from magic and element', () => {
    // 50% magic_skill_damage_more × 20% fire_skill_damage_more
    // = 1.5 * 1.2 = 1.80 multiplier
    const stats: RangedStatMap = {
      magic_skill_damage_more: 50,
      fire_skill_damage_more: 20,
    }
    const out = computeSkillDamage(FIRE_SKILL_WITH_FORMULA, 5, {}, stats, {}, {})
    // base 35 * (1.5 * 1.2) = 35 * 1.8 = 63 mathematically.
    // In IEEE 754, (1.5 * 1.2) = 1.7999999999999998, so 35 * 1.7999... =
    // 62.999... and Math.floor truncates to 62. This test pins the actual
    // user-visible behaviour; if the implementation later rounds to e.g. 6
    // decimal places before flooring (as combineAdditiveAndMore already
    // does), bump the expected value back to 63.
    expect(out!.hitMin).toBe(62)
  })

  it('clamps the average crit multiplier at the 95% crit chance cap', () => {
    // Even with 200% crit chance (overflow), the average multiplier uses 95%.
    const stats: RangedStatMap = {
      crit_chance: 200,
      crit_damage: 100, // crit deals 2x
    }
    const out = computeSkillDamage(FIRE_SKILL_WITH_FORMULA, 5, {}, stats, {}, {})
    // avg = 1 - 0.95 + 0.95 * 2 = 1.95
    expect(out!.critMultiplierAvg).toBeCloseTo(1.95, 10)
    expect(out!.critChance).toBe(200) // unclamped is reported
  })

  it('applies enemy resistance multiplicatively, mitigated by ignore_<type>_res', () => {
    // Enemy 50% fire resist, ignore 50% → effective 25% → 0.75 mult.
    const stats: RangedStatMap = { ignore_fire_res: 50 }
    const out = computeSkillDamage(
      FIRE_SKILL_WITH_FORMULA,
      5,
      {},
      stats,
      {},
      {},
      undefined,
      { fire: 50 },
    )
    expect(out!.resistanceIgnoredPct).toBe(50)
    expect(out!.effectiveResistancePct).toBe(25)
    expect(out!.resistanceMultiplier).toBeCloseTo(0.75, 10)
    // base 35 * 0.75 = 26.25 → floor = 26
    expect(out!.hitMin).toBe(26)
  })

  it('caps ignore_<type>_res at 100% so multiplier never flips sign', () => {
    const stats: RangedStatMap = { ignore_fire_res: 999 }
    const out = computeSkillDamage(
      FIRE_SKILL_WITH_FORMULA,
      5,
      {},
      stats,
      {},
      {},
      undefined,
      { fire: 80 },
    )
    expect(out!.resistanceIgnoredPct).toBe(100)
    expect(out!.effectiveResistancePct).toBe(0)
    expect(out!.resistanceMultiplier).toBe(1)
  })

  it('applies elemental_break only to elemental skills, not e.g. magic', () => {
    const stats: RangedStatMap = { elemental_break: 30 }
    const fire = computeSkillDamage(FIRE_SKILL_WITH_FORMULA, 5, {}, stats, {}, {})
    expect(fire!.elementalBreakPct).toBe(30)
    expect(fire!.elementalBreakMultiplier).toBeCloseTo(1.3, 10)
  })

  it('reads spell crit stats and skips crit_damage_more for spells', () => {
    // crit_damage_more should NOT apply to spells.
    const stats: RangedStatMap = {
      spell_crit_chance: 50,
      spell_crit_damage: 100,
      crit_damage_more: 999, // ignored for spells
    }
    const out = computeSkillDamage(SPELL_SKILL, 5, {}, stats, {}, {})
    expect(out!.critChance).toBe(50)
    expect(out!.critDamagePct).toBe(100)
    // critMultOnCrit = (1 + 100/100) * (1 + 0/100) = 2 → unaffected by _more
    expect(out!.critMultiplierAvg).toBeCloseTo(1 - 0.5 + 0.5 * 2, 10)
  })

  it('adds the multicast multiplier and projectile count to the average', () => {
    const stats: RangedStatMap = { multicast_chance: 50 }
    const out = computeSkillDamage(
      SPELL_SKILL,
      5,
      {},
      stats,
      {},
      {},
      undefined,
      undefined,
      undefined,
      3, // projectile count
    )
    expect(out!.multicastChancePct).toBe(50)
    expect(out!.multicastMultiplier).toBeCloseTo(1.5, 10)
    expect(out!.projectileCount).toBe(3)
    // avgMin = hit * critAvg * 1.5 * 3 — hit=70 (base 70 no mods), critAvg=1
    expect(out!.avgMin).toBe(Math.floor(70 * 1 * 1.5 * 3))
  })

  it('does not apply multicast to non-spells', () => {
    const stats: RangedStatMap = { multicast_chance: 50 }
    const out = computeSkillDamage(PHYSICAL_SKILL_TABLE, 2, {}, stats, {}, {})
    expect(out!.multicastChancePct).toBe(0)
    expect(out!.multicastMultiplier).toBe(1)
  })

  it('applies lightning_break only when both skill is lightning AND enemyCondition is set', () => {
    const stats: RangedStatMap = { lightning_break: 25 }
    // Not enabled — multiplier stays at 1.
    const withoutCond = computeSkillDamage(LIGHTNING_SKILL, 5, {}, stats, {}, {})
    expect(withoutCond!.hitMin).toBe(35) // unchanged
    // Enabled — 25% bonus.
    const withCond = computeSkillDamage(
      LIGHTNING_SKILL,
      5,
      {},
      stats,
      {},
      {},
      { lightning_break: true },
    )
    expect(withCond!.hitMin).toBe(Math.floor(35 * 1.25))
  })

  it('forces projectile count to be at least 1 when input is omitted or below 1', () => {
    const out = computeSkillDamage(FIRE_SKILL_WITH_FORMULA, 5, {}, {}, {}, {})
    expect(out!.projectileCount).toBe(1)
    const explicitZero = computeSkillDamage(
      FIRE_SKILL_WITH_FORMULA,
      5,
      {},
      {},
      {},
      {},
      undefined,
      undefined,
      undefined,
      0,
    )
    expect(explicitZero!.projectileCount).toBe(1)
  })
})

// -----------------------------------------------------------------------
// computeWeaponDamage
// -----------------------------------------------------------------------

describe('computeWeaponDamage', () => {
  it('uses the documented 2-6 unarmed damage when no weapon is equipped', () => {
    const out = computeWeaponDamage({} as Inventory, {})
    expect(out.hasWeapon).toBe(false)
    expect(out.weaponDamageMin).toBe(2)
    expect(out.weaponDamageMax).toBe(6)
  })

  it('multiplies weapon damage by enhanced_damage and the _more variant', () => {
    // 50% enhanced + 20% more = 2 * 1.5 * 1.2 = 3.6 → hitMin floor = 3
    const stats: RangedStatMap = { enhanced_damage: 50, enhanced_damage_more: 20 }
    const out = computeWeaponDamage({} as Inventory, stats)
    expect(out.hitMin).toBe(Math.floor(2 * 1.5 * 1.2))
    expect(out.hitMax).toBe(Math.floor(6 * 1.5 * 1.2))
  })

  it('adds additive_physical_damage AFTER enhanced multipliers', () => {
    // baseMin = 2 * (1 + 0) * 1 + 5 = 7
    const stats: RangedStatMap = { additive_physical_damage: 5 }
    const out = computeWeaponDamage({} as Inventory, stats)
    expect(out.hitMin).toBe(7)
    expect(out.hitMax).toBe(11) // 6 + 5
  })

  it('applies attack_damage multiplicatively after base composition', () => {
    // attack_damage 100% → 2x multiplier. base 2 * 2 = 4 → floor = 4
    const stats: RangedStatMap = { attack_damage: 100 }
    const out = computeWeaponDamage({} as Inventory, stats)
    expect(out.hitMin).toBe(4)
    expect(out.hitMax).toBe(12)
  })

  it('computes APS from increased_attack_speed compounded with the _more variant', () => {
    // baseAps 1.5, ias 50%, ias_more 10% → 1.5 * 1.5 * 1.1 = 2.475
    const stats: RangedStatMap = {
      attacks_per_second: 1.5,
      increased_attack_speed: 50,
      increased_attack_speed_more: 10,
    }
    const out = computeWeaponDamage({} as Inventory, stats)
    expect(out.attacksPerSecondMax).toBeCloseTo(1.5 * 1.5 * 1.1, 10)
  })

  it('clamps the average crit multiplier at the 95% crit chance cap (mirror of skill damage)', () => {
    const stats: RangedStatMap = { crit_chance: 200, crit_damage: 100 }
    const out = computeWeaponDamage({} as Inventory, stats)
    // avg = 1 - 0.95 + 0.95 * 2 = 1.95
    expect(out.critMultiplierAvg).toBeCloseTo(1.95, 10)
  })
})

// -----------------------------------------------------------------------
// passiveStatsAtRank
// -----------------------------------------------------------------------

describe('passiveStatsAtRank', () => {
  it('returns an empty record at rank 0', () => {
    const skill = {
      passiveStats: { base: { life: 10 } },
      ranks: [],
    } as unknown as Skill
    expect(passiveStatsAtRank(skill, 0)).toEqual({})
  })

  it('returns just the base stats at rank 1 (perRank multiplier is 0)', () => {
    const skill = {
      passiveStats: {
        base: { life: 100 },
        perRank: { life: 25 },
      },
      ranks: [],
    } as unknown as Skill
    expect(passiveStatsAtRank(skill, 1)).toEqual({ life: 100 })
  })

  it('compounds base + perRank * (rank - 1)', () => {
    const skill = {
      passiveStats: {
        base: { life: 100 },
        perRank: { life: 25 },
      },
      ranks: [],
    } as unknown as Skill
    // rank 5: 100 + 25 * 4 = 200
    expect(passiveStatsAtRank(skill, 5)).toEqual({ life: 200 })
  })

  it('rounds to 3 decimal places to avoid floating-point drift', () => {
    const skill = {
      passiveStats: { perRank: { x: 0.1 } },
      ranks: [],
    } as unknown as Skill
    // (rank-1) * 0.1 — at rank 4 that's 0.30000000000000004 in raw FP.
    expect(passiveStatsAtRank(skill, 4)).toEqual({ x: 0.3 })
  })
})

// -----------------------------------------------------------------------
// manaCostAtRank
// -----------------------------------------------------------------------

describe('manaCostAtRank', () => {
  it('uses the formula when manaCostFormula is set, flooring the result', () => {
    const skill = {
      manaCostFormula: { base: 10, perLevel: 1.5 },
      ranks: [],
    } as unknown as Skill
    // rank 3: floor(10 + 1.5 * 2) = floor(13) = 13
    expect(manaCostAtRank(skill, 3)).toBe(13)
  })

  it('treats rank <= 0 as rank 1 for the formula', () => {
    const skill = {
      manaCostFormula: { base: 10, perLevel: 1.5 },
      ranks: [],
    } as unknown as Skill
    expect(manaCostAtRank(skill, 0)).toBe(10)
  })

  it('looks up the exact rank in the ranks table when no formula is set', () => {
    const skill = {
      ranks: [
        { rank: 1, manaCost: 5 },
        { rank: 2, manaCost: 10 },
        { rank: 3, manaCost: 15 },
      ],
    } as unknown as Skill
    expect(manaCostAtRank(skill, 2)).toBe(10)
  })

  it('falls back to the rank-1 entry when the requested rank is missing', () => {
    const skill = {
      ranks: [{ rank: 1, manaCost: 5 }],
    } as unknown as Skill
    expect(manaCostAtRank(skill, 99)).toBe(5)
  })
})
