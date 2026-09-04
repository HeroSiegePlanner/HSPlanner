import { describe, expect, test } from 'vitest'
import { buildSuggestPayload } from './nativeSuggest'
import type { BuildPerformanceDeps } from '../build/buildPerformance'

const baseDeps = {
  classId: 'amazon',
  level: 60,
  allocatedAttrs: {},
  inventory: {},
  skillRanks: { noxious_strike: 20 },
  subskillRanks: {},
  activeAuraId: null,
  activeBuffs: {},
  customStats: [],
  allocatedTreeNodes: new Set([1, 2]),
  treeSocketed: {},
  activeSkillIds: ['noxious_strike', 'second_skill'],
  enemyConditions: {},
  playerConditions: {},
  skillProjectiles: {},
  enemyResistances: {},
  procToggles: {},
  killsPerSec: 0,
  entityRates: {},
  stackCounts: {},
  difficulty: 'hell',
} as unknown as BuildPerformanceDeps

describe('buildSuggestPayload', () => {
  test('sends full perf input with the current allocation override', () => {
    const payload = buildSuggestPayload(baseDeps, new Set([5, 6, 7]), 12)
    expect(payload.budget).toBe(12)
    expect(payload.activeSkillIds).toEqual(['noxious_strike', 'second_skill'])
    expect([...payload.perf.allocatedTreeNodes!].sort()).toEqual([5, 6, 7])
    expect(payload.perf.difficulty).toBe('hell')
    expect(payload.perf.classId).toBe('amazon')
    expect(payload.graph.adjacency).toBeDefined()
    expect(payload.graph.startIds.length).toBeGreaterThan(0)
    expect(payload.graph.valuableIds.length).toBeGreaterThan(0)
  })

  test('does not mutate the deps allocation', () => {
    const before = new Set(baseDeps.allocatedTreeNodes)
    buildSuggestPayload(baseDeps, new Set([9]), 1)
    expect(baseDeps.allocatedTreeNodes).toEqual(before)
  })
})
