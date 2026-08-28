import { describe, expect, it } from 'vitest'
import {
  formatAffixValue,
  rollFromValue,
  sliderPct,
  sliderStep,
  valueFromRoll,
} from './rollMath'

describe('sliderStep', () => {
  it('uses whole steps for integer endpoints', () => {
    expect(sliderStep(10, 20)).toBe(1)
  })

  it('uses fine steps when an endpoint is fractional', () => {
    expect(sliderStep(1.2, 1.5)).toBe(0.1)
    expect(sliderStep(10, 12.5)).toBe(0.1)
  })
})

describe('rollFromValue', () => {
  it('maps the range linearly for positive affixes', () => {
    expect(rollFromValue(10, 10, 20)).toBe(0)
    expect(rollFromValue(20, 10, 20)).toBe(1)
    expect(rollFromValue(15, 10, 20)).toBe(0.5)
  })

  it('maps through magnitude for negative affixes (engine order roll0=-min, roll1=-max)', () => {
    expect(rollFromValue(-12, -12, -20)).toBe(0)
    expect(rollFromValue(-20, -12, -20)).toBe(1)
    expect(rollFromValue(-16, -12, -20)).toBe(0.5)
  })

  it('clamps outside the range and collapses degenerate ranges to 1', () => {
    expect(rollFromValue(5, 10, 20)).toBe(0)
    expect(rollFromValue(25, 10, 20)).toBe(1)
    expect(rollFromValue(12, 12, 12)).toBe(1)
  })
})

describe('valueFromRoll', () => {
  it('round-trips with rollFromValue on flat positive ranges', () => {
    expect(valueFromRoll(0, 10, 20, 'flat')).toBe(10)
    expect(valueFromRoll(1, 10, 20, 'flat')).toBe(20)
    expect(valueFromRoll(0.5, 10, 20, 'flat')).toBe(15)
  })

  it('rounds flat values like the engine and keeps percent fractional', () => {
    expect(valueFromRoll(0.33, 10, 20, 'flat')).toBe(13)
    expect(valueFromRoll(0.33, 10, 20, 'percent')).toBeCloseTo(13.3)
  })

  it('keeps the negative sign from the endpoints', () => {
    expect(valueFromRoll(0, -12, -20, 'flat')).toBe(-12)
    expect(valueFromRoll(1, -12, -20, 'flat')).toBe(-20)
  })

  it('clamps roll outside 0..1', () => {
    expect(valueFromRoll(2, 10, 20, 'flat')).toBe(20)
    expect(valueFromRoll(-1, 10, 20, 'flat')).toBe(10)
  })
})

describe('formatAffixValue', () => {
  it('formats flat and percent values with sign', () => {
    expect(formatAffixValue({ sign: '+', format: 'flat' }, 17)).toBe('+17')
    expect(formatAffixValue({ sign: '+', format: 'percent' }, 13.3)).toBe('+13.3%')
  })

  it('uses minus for negative values and minus-signed affixes', () => {
    expect(formatAffixValue({ sign: '-', format: 'flat' }, -15)).toBe('-15')
    expect(formatAffixValue({ sign: '-', format: 'percent' }, 15)).toBe('-15%')
  })

  it('rounds display noise to two decimals', () => {
    expect(formatAffixValue({ sign: '+', format: 'percent' }, 13.333333)).toBe('+13.33%')
  })
})

describe('sliderPct', () => {
  it('returns the position as a css percentage', () => {
    expect(sliderPct(15, 10, 20)).toBe('50%')
    expect(sliderPct(-16, -20, -12)).toBe('50%')
  })

  it('clamps and survives degenerate ranges', () => {
    expect(sliderPct(25, 10, 20)).toBe('100%')
    expect(sliderPct(5, 10, 20)).toBe('0%')
    expect(sliderPct(10, 10, 10)).toBe('100%')
  })
})
