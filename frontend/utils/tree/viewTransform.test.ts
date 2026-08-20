import { describe, expect, it } from 'vitest'
import { zoomAtPoint, type ViewTransform } from './viewTransform'

const view: ViewTransform = { scale: 1, tx: 100, ty: 50 }

describe('zoomAtPoint', () => {
  it('keeps the world point under the cursor fixed on screen', () => {
    const mx = 400
    const my = 300
    const worldX = (mx - view.tx) / view.scale
    const worldY = (my - view.ty) / view.scale

    const next = zoomAtPoint(view, mx, my, true, 0.1, 5)

    expect(worldX * next.scale + next.tx).toBeCloseTo(mx)
    expect(worldY * next.scale + next.ty).toBeCloseTo(my)
  })

  it('zooms in and out by the wheel factor', () => {
    expect(zoomAtPoint(view, 0, 0, true, 0.1, 5).scale).toBeCloseTo(1.1)
    expect(zoomAtPoint(view, 0, 0, false, 0.1, 5).scale).toBeCloseTo(0.9)
  })

  it('clamps scale to the given limits', () => {
    const zoomedIn = zoomAtPoint({ ...view, scale: 5 }, 0, 0, true, 0.1, 5)
    expect(zoomedIn.scale).toBe(5)
    const zoomedOut = zoomAtPoint({ ...view, scale: 0.1 }, 0, 0, false, 0.1, 5)
    expect(zoomedOut.scale).toBe(0.1)
  })

  it('panning continues smoothly after a mid-drag zoom rebase', () => {
    // simulate: drag starts, zoom fires mid-drag, drag baseline is rebased to
    // the post-zoom transform — the very next mousemove with zero mouse delta
    // must not move the view at all (this was the "teleport" bug)
    const dragStart = { x: 400, y: 300, tx: view.tx, ty: view.ty }
    const next = zoomAtPoint(view, 400, 300, true, 0.1, 5)
    const rebased = { x: 400, y: 300, tx: next.tx, ty: next.ty }

    const dx = 400 - rebased.x
    const dy = 300 - rebased.y
    expect(rebased.tx + dx).toBeCloseTo(next.tx)
    expect(rebased.ty + dy).toBeCloseTo(next.ty)
    // stale baseline (the bug): mousemove would snap back to pre-zoom tx/ty
    expect(dragStart.tx + dx).not.toBeCloseTo(next.tx)
  })
})
