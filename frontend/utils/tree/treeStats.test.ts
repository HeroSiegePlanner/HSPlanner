import { describe, expect, it } from 'vitest'
import {
  SELF_CONDITION_KEYS,
  SELF_CONDITION_LABELS,
  TREE_JEWELRY_IDS,
  TREE_NODE_INFO,
  TREE_WARP_IDS,
} from './treeStats'

describe('SELF_CONDITION_KEYS', () => {
  it('exposes the two supported self-condition keys', () => {
    expect(SELF_CONDITION_KEYS).toEqual(['crit_chance_below_40', 'life_below_40'])
  })

  it('has a label for every key', () => {
    for (const key of SELF_CONDITION_KEYS) {
      expect(SELF_CONDITION_LABELS[key]).toBeTruthy()
    }
  })
})

describe('TREE_NODE_INFO', () => {
  it('contains at least one node entry', () => {
    expect(Object.keys(TREE_NODE_INFO).length).toBeGreaterThan(0)
  })

  it('partitions warp and jewelry ids into disjoint sets', () => {
    for (const id of TREE_WARP_IDS) {
      expect(TREE_JEWELRY_IDS.has(id)).toBe(false)
    }
  })
})
