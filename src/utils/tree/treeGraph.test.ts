import { describe, expect, it } from 'vitest'
import { ADJ, START_IDS, START_SET } from './treeGraph'

describe('treeGraph season-aware build', () => {
  it('derives START_IDS from root nodes (s9 baseline)', () => {
    expect([...START_IDS]).toEqual([0, 8, 16, 28, 42, 44, 46, 48])
    expect(START_SET.has(0)).toBe(true)
  })

  it('builds adjacency for every node', () => {
    expect(ADJ.size).toBeGreaterThan(1000)
    expect(ADJ.get(0)?.size).toBeGreaterThan(0)
  })
})
