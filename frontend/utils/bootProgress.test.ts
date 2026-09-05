import { describe, expect, it } from 'vitest'
import { SPRITES_WEIGHT, WARMUP_WEIGHT, bootProgress } from './bootProgress'

const done = { done: 1, total: 1 }
const idle = { done: 0, total: 0 }

describe('boot progress weights', () => {
  it('weights warmup and sprites to a full bar', () => {
    expect(WARMUP_WEIGHT + SPRITES_WEIGHT).toBeCloseTo(1)
  })

  it('gives sprites the dominant share of the bar', () => {
    expect(SPRITES_WEIGHT).toBeGreaterThan(WARMUP_WEIGHT)
  })
})

describe('bootProgress', () => {
  it('starts at zero before either phase reports', () => {
    expect(bootProgress(idle, idle).pct).toBeCloseTo(0)
  })

  it('treats a phase with no total as not started, not complete', () => {
    expect(bootProgress(idle, done).pct).toBeCloseTo(SPRITES_WEIGHT * 100)
  })

  it('sums both phases by weight', () => {
    const { pct } = bootProgress({ done: 50, total: 100 }, { done: 335, total: 670 })

    expect(pct).toBeCloseTo((WARMUP_WEIGHT * 0.5 + SPRITES_WEIGHT * 0.5) * 100)
  })

  it('reaches 100 only when both phases are complete', () => {
    expect(bootProgress(done, { done: 670, total: 670 }).pct).toBeCloseTo(100)
    expect(bootProgress({ done: 99, total: 100 }, done).pct).toBeLessThan(100)
  })

  it('never overshoots when done exceeds total', () => {
    expect(bootProgress({ done: 5, total: 3 }, done).pct).toBeCloseTo(100)
  })

  it('labels the sprite phase with its count while sprites load', () => {
    expect(bootProgress(idle, { done: 12, total: 670 }).status).toBe('Loading sprites · 12/670')
  })

  it('labels the sprite phase without a count before the first sprite reports', () => {
    expect(bootProgress(idle, idle).status).toBe('Loading sprites')
  })

  it('falls back to the game-data label once sprites are done', () => {
    expect(bootProgress({ done: 30, total: 340 }, done).status).toBe('Loading game data · 30/340')
  })

  it('labels game data without a count before the engine reports', () => {
    expect(bootProgress(idle, done).status).toBe('Loading game data')
  })

  it('reports Ready when both phases are complete', () => {
    expect(bootProgress(done, done).status).toBe('Ready')
  })
})
