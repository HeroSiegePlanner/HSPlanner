import { describe, expect, it } from 'vitest'
import {
  SPRITES_WEIGHT,
  WARMUP_WEIGHT,
  spriteBootProgress,
  warmupBootProgress,
} from './bootProgress'

describe('boot progress weights', () => {
  it('weights warmup and sprites to a full bar', () => {
    expect(WARMUP_WEIGHT + SPRITES_WEIGHT).toBeCloseTo(1)
  })

  it('gives sprites the dominant share of the bar', () => {
    expect(SPRITES_WEIGHT).toBeGreaterThan(WARMUP_WEIGHT)
  })
})

describe('warmupBootProgress', () => {
  it('starts the warmup phase at zero', () => {
    const { pct, status } = warmupBootProgress(0, 100)

    expect(pct).toBeCloseTo(0)
    expect(status).toBe('Loading game data')
  })

  it('scales the bar across the warmup weight at the halfway mark', () => {
    const { pct } = warmupBootProgress(50, 100)

    expect(pct).toBeCloseTo(WARMUP_WEIGHT * 0.5 * 100)
  })

  it('reaches the warmup ceiling when complete', () => {
    const { pct } = warmupBootProgress(100, 100)

    expect(pct).toBeCloseTo(WARMUP_WEIGHT * 100)
  })

  it('treats an empty node set as complete', () => {
    const { pct } = warmupBootProgress(0, 0)

    expect(pct).toBeCloseTo(WARMUP_WEIGHT * 100)
  })

  it('hands off continuously to the sprite phase', () => {
    expect(warmupBootProgress(100, 100).pct).toBeCloseTo(
      spriteBootProgress(0, 670).pct,
    )
  })
})

describe('spriteBootProgress', () => {
  it('starts the sprite phase at the warmup ceiling', () => {
    const { pct, status } = spriteBootProgress(0, 670)

    expect(pct).toBeCloseTo(WARMUP_WEIGHT * 100)
    expect(status).toBe('Loading sprites · 0/670')
  })

  it('reaches 100% with a full count when all sprites load', () => {
    const { pct, status } = spriteBootProgress(670, 670)

    expect(pct).toBeCloseTo(100)
    expect(status).toBe('Loading sprites · 670/670')
  })

  it('scales the bar across the sprite weight at the halfway mark', () => {
    const { pct } = spriteBootProgress(335, 670)

    expect(pct).toBeCloseTo((WARMUP_WEIGHT + SPRITES_WEIGHT * 0.5) * 100)
  })

  it('treats an empty sprite set as complete without a count', () => {
    const { pct, status } = spriteBootProgress(0, 0)

    expect(pct).toBeCloseTo(100)
    expect(status).toBe('Loading sprites')
  })
})
