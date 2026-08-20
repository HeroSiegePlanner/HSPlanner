import { describe, expect, it } from 'vitest'
import type { SlotKey } from '../../../types'
import {
  CHARM_EXTRA_SLOT_CELL,
  CHARM_GRID_COLS,
  CHARM_GRID_ROWS,
  charmBlockedCells,
  packCharms,
} from './charmPacking'

const unitCharms = (count: number) =>
  Array.from({ length: count }, (_, i) => ({
    slotKey: `charm_${i + 1}` as SlotKey,
    w: 1,
    h: 1,
  }))

describe('charmBlockedCells', () => {
  it('blocks 3 cells with the extra slot unlocked, 4 without', () => {
    expect(charmBlockedCells(true)).toHaveLength(3)
    expect(charmBlockedCells(false)).toHaveLength(4)
    expect(charmBlockedCells(false)).toContainEqual(CHARM_EXTRA_SLOT_CELL)
    expect(charmBlockedCells(true)).not.toContainEqual(CHARM_EXTRA_SLOT_CELL)
  })
})

describe('packCharms with the extra slot locked', () => {
  it('fits 30 unit charms only when the extra slot is unlocked', () => {
    const charms = unitCharms(30)
    expect(packCharms(charms, charmBlockedCells(true)).overflow).toHaveLength(0)
    expect(packCharms(charms, charmBlockedCells(false)).overflow).toHaveLength(1)
  })

  it('never places a charm on the locked extra cell', () => {
    const result = packCharms(unitCharms(29), charmBlockedCells(false))
    const [er, ec] = CHARM_EXTRA_SLOT_CELL
    expect(result.overflow).toHaveLength(0)
    expect(
      result.placed.some((p) => p.row === er && p.col === ec),
    ).toBe(false)
  })

  it('defaults to the unlocked grid', () => {
    expect(packCharms(unitCharms(30)).overflow).toHaveLength(0)
  })

  it('keeps blocked cells occupied in the reported occupancy', () => {
    const { occupancy } = packCharms([], charmBlockedCells(false))
    for (const [r, c] of charmBlockedCells(false)) {
      expect(occupancy[r * CHARM_GRID_COLS + c]).toBe(true)
    }
    const freeCells = occupancy.filter((cell) => !cell).length
    expect(freeCells).toBe(CHARM_GRID_ROWS * CHARM_GRID_COLS - 4)
  })
})
