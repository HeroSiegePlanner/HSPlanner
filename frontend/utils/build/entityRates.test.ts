import { describe, expect, it } from 'vitest'
import {
  defaultEntityRates,
  entityAttackRate,
  entityAttackRateFixedKey,
  entityAttackSpeedKey,
  entityKindOfTag,
  entityRatesFrom,
} from './entityRates'

describe('entityRatesFrom', () => {
  it('keeps stored per-kind rates and fills the gaps', () => {
    expect(entityRatesFrom({ sentry: 3 }, undefined)).toEqual({
      sentry: 3,
      summon: 1,
      guardian: 1,
    })
  })

  it('spreads a pre-split build\'s single rate over all three kinds', () => {
    expect(entityRatesFrom(undefined, 2.5)).toEqual({
      sentry: 2.5,
      summon: 2.5,
      guardian: 2.5,
    })
  })

  it('falls back to the defaults when a build carries neither', () => {
    expect(entityRatesFrom(undefined, undefined)).toEqual(defaultEntityRates())
  })
})

describe('entityKindOfTag', () => {
  it('maps entity tags to their knob', () => {
    expect(entityKindOfTag('Sentry')).toBe('sentry')
    expect(entityKindOfTag('Guardian')).toBe('guardian')
    expect(entityKindOfTag('Projectile')).toBeUndefined()
  })
})

describe('entityAttackRate', () => {
  const rates = { sentry: 2, summon: 1, guardian: 1 }

  it('multiplies the flat Config rate by increased entity attack speed', () => {
    expect(entityAttackRate('sentry', rates, [25, 50])).toEqual({
      base: 2,
      min: 2.5,
      max: 3,
    })
  })

  it('returns the flat rate when nothing increases it', () => {
    expect(entityAttackRate('summon', rates, [0, 0])).toEqual({
      base: 1,
      min: 1,
      max: 1,
    })
  })

  it('pins the rate flat, ignoring the knob and entity attack speed', () => {
    expect(entityAttackRate('sentry', rates, [100, 100], 4)).toEqual({
      base: 4,
      min: 4,
      max: 4,
    })
  })

  it('names the stat keys per kind', () => {
    expect(entityAttackSpeedKey('guardian')).toBe('guardian_attack_speed')
    expect(entityAttackRateFixedKey('sentry')).toBe('sentry_attack_rate_fixed')
  })
})
