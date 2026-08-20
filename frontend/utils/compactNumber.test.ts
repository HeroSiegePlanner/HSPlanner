import { describe, expect, it } from 'vitest'
import { compact, compactRange } from './compactNumber'

const plain = (n: number) => Math.round(n).toLocaleString()
const inUnit = (n: number, unit: number) =>
  (n / unit).toLocaleString(undefined, { maximumFractionDigits: 1 })

describe('compact — default scale (billions) cascades k/M/B', () => {
  it('leaves numbers below 10k unabbreviated', () => {
    expect(compact(9999)).toBe(plain(9999))
    expect(compact(0)).toBe('0')
  })

  it('picks the largest fitting unit', () => {
    expect(compact(12345)).toBe('12.3k')
    expect(compact(1_500_000)).toBe('1.5M')
    expect(compact(1_000_000)).toBe('1M')
    expect(compact(2_000_000_000)).toBe('2B')
  })

  it('handles negatives by magnitude', () => {
    expect(compact(-12345)).toBe('-12.3k')
  })
})

describe('compact — scale caps the largest unit used', () => {
  it("scale 'none' never divides", () => {
    expect(compact(12345, 'none')).toBe(plain(12345))
    expect(compact(2_000_000_000, 'none')).toBe(plain(2_000_000_000))
  })

  it("scale 'thousands' expresses everything large in k", () => {
    expect(compact(45_678, 'thousands')).toBe('45.7k')
    expect(compact(12_345_678, 'thousands')).toBe(
      `${inUnit(12_345_678, 1e3)}k`,
    )
    expect(compact(2_500_000_000, 'thousands')).toBe(
      `${inUnit(2_500_000_000, 1e3)}k`,
    )
  })

  it("scale 'millions' caps at M and keeps k below", () => {
    expect(compact(45_678, 'millions')).toBe('45.7k')
    expect(compact(500_500_000, 'millions')).toBe('500.5M')
    expect(compact(2_500_000_000, 'millions')).toBe(
      `${inUnit(2_500_000_000, 1e6)}M`,
    )
  })

  it("scale 'billions' allows the full cascade", () => {
    expect(compact(45_678, 'billions')).toBe('45.7k')
    expect(compact(12_345_678, 'billions')).toBe('12.3M')
    expect(compact(2_500_000_000, 'billions')).toBe('2.5B')
  })
})

describe('compactRange', () => {
  it('collapses near-equal bounds and forwards the scale', () => {
    expect(compactRange(1_500_000, 1_500_000, 'millions')).toBe('1.5M')
    expect(compactRange(12345, 67890, 'none')).toBe(
      `${plain(12345)}–${plain(67890)}`,
    )
    expect(compactRange(12345, 67890)).toBe('12.3k–67.9k')
  })
})
