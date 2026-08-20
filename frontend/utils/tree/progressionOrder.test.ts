import { describe, expect, it } from 'vitest'
import { preserveOrder, progressionOrder } from './progressionOrder'

const ADJ = new Map<number, Set<number>>([
  [1, new Set([2, 5])],
  [2, new Set([1, 3])],
  [3, new Set([2, 4])],
  [4, new Set([3])],
  [5, new Set([1, 6])],
  [6, new Set([5])],
  [10, new Set([11])],
  [11, new Set([10])],
])
const STARTS = new Set([1, 10])

function order(ids: number[]): number[] {
  return progressionOrder(ids, STARTS, ADJ)
}

describe('progressionOrder', () => {
  it('returns empty array for empty allocation', () => {
    expect(order([])).toEqual([])
  })

  it('keeps an already-connected order untouched', () => {
    expect(order([1, 2, 3, 4])).toEqual([1, 2, 3, 4])
    expect(order([1, 5, 6, 2, 3])).toEqual([1, 5, 6, 2, 3])
  })

  it('repairs a sorted-by-id order into connected prefixes', () => {
    expect(order([2, 3, 4, 1])).toEqual([1, 2, 3, 4])
  })

  it('every prefix of the result is reachable from the starts', () => {
    const result = order([4, 6, 3, 5, 2, 1])
    expect(new Set(result)).toEqual(new Set([1, 2, 3, 4, 5, 6]))
    const taken = new Set<number>()
    for (const id of result) {
      const connected =
        STARTS.has(id) || [...(ADJ.get(id) ?? [])].some((nb) => taken.has(nb))
      expect(connected).toBe(true)
      taken.add(id)
    }
  })

  it('supports multiple start nodes', () => {
    expect(order([11, 2, 1, 10])).toEqual([1, 2, 10, 11])
  })

  it('appends unreachable nodes at the end in input order', () => {
    expect(order([1, 99, 2, 98])).toEqual([1, 2, 99, 98])
  })
})

describe('preserveOrder', () => {
  it('keeps reference order for surviving members', () => {
    const result = preserveOrder([1, 2, 3, 4], new Set([4, 1, 3]))
    expect([...result]).toEqual([1, 3, 4])
  })

  it('appends members missing from the reference in member order', () => {
    const result = preserveOrder([1, 2], new Set([8, 2, 1, 7]))
    expect([...result]).toEqual([1, 2, 8, 7])
  })

  it('returns an equal set (same elements)', () => {
    const members = new Set([5, 6, 1])
    expect(preserveOrder([6, 5], members)).toEqual(members)
  })
})
