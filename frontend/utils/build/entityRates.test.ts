import { describe, expect, it } from 'vitest'
import { defaultEntityRates, entityKindOfTag, entityRatesFrom } from './entityRates'

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
