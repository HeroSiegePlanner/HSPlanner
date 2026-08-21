import { describe, expect, it } from 'vitest'
import { ADJ, START_IDS, START_SET } from './treeGraph'

describe('treeGraph season-aware build', () => {
  it('derives START_IDS from the active season\'s root nodes (s10)', () => {
    expect([...START_IDS]).toEqual([0, 1, 22, 39, 44, 61, 66, 83])
    expect(START_SET.has(0)).toBe(true)
  })

  it('builds adjacency for every node', () => {
    expect(ADJ.size).toBeGreaterThan(1000)
    expect(ADJ.get(0)?.size).toBeGreaterThan(0)
  })
})
