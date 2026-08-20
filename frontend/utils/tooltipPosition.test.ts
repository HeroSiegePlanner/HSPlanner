import { describe, expect, it } from 'vitest'
import { clampTooltipToViewport } from './tooltipPosition'

const vp = { width: 1000, height: 800 }

describe('clampTooltipToViewport', () => {
  it('places tooltip below-right of the anchor when it fits', () => {
    const p = clampTooltipToViewport({ x: 100, y: 100 }, { width: 200, height: 150 }, vp)
    expect(p).toEqual({ left: 118, top: 118 })
  })

  it('flips upward when it would overflow the bottom edge (the reported bug)', () => {
    const p = clampTooltipToViewport({ x: 100, y: 700 }, { width: 200, height: 150 }, vp)
    expect(p.top).toBe(532)
  })

  it('flips left when it would overflow the right edge', () => {
    const p = clampTooltipToViewport({ x: 950, y: 100 }, { width: 200, height: 150 }, vp)
    expect(p.left).toBe(732)
  })

  it('keeps a tooltip taller than the viewport pinned to the top margin', () => {
    const p = clampTooltipToViewport({ x: 100, y: 400 }, { width: 200, height: 900 }, vp)
    expect(p.top).toBe(12)
  })

  it('never positions above or left of the margin', () => {
    const p = clampTooltipToViewport({ x: 2, y: 2 }, { width: 200, height: 150 }, vp)
    expect(p.left).toBeGreaterThanOrEqual(12)
    expect(p.top).toBeGreaterThanOrEqual(12)
  })
})
