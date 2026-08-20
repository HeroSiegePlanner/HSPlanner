import { describe, expect, it } from 'vitest'
import { heroLevelFor } from './heroLevel'

describe('heroLevelFor', () => {
  it('counts allocated incarnation-tree nodes', () => {
    expect(heroLevelFor({ allocatedTreeNodes: new Set([1, 2, 3]) })).toBe(3)
  })

  it('returns zero for an empty allocation', () => {
    expect(heroLevelFor({ allocatedTreeNodes: new Set() })).toBe(0)
  })
})
