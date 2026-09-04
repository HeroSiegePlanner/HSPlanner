import { beforeEach, describe, expect, it, vi } from 'vitest'
import { scanForUpgrades } from './upgradeAdvisor'
import { rankSlotItemsNative } from '../../../utils/calc/bridge'
import { pickerItemsForSlot } from '../pickerItems'
import { getItem } from '@data'
import type { BuildPerformanceDeps } from '../../../utils/build/buildPerformance'

vi.mock('../../../utils/calc/bridge', () => ({
  rankSlotItemsNative: vi.fn(),
}))
vi.mock('../pickerItems', () => ({
  pickerItemsForSlot: vi.fn(),
}))
vi.mock('@data', () => ({
  gameConfig: {
    slots: [
      { key: 'helm', name: 'Helm', group: 'armor' },
      { key: 'weapon', name: 'Weapon', group: 'weapons' },
      { key: 'offhand', name: 'Offhand', group: 'weapons' },
      { key: 'boots', name: 'Boots', group: 'armor' },
      { key: 'gloves', name: 'Gloves', group: 'armor' },
      { key: 'belt', name: 'Belt', group: 'armor' },
      { key: 'ring', name: 'Ring', group: 'jewelry' },
      { key: 'amulet', name: 'Amulet', group: 'jewelry' },
      { key: 'charm_1', name: 'Charm 1', group: 'charms' },
      { key: 'relic_1', name: 'Relic 1', group: 'special' },
      { key: 'relic_2', name: 'Relic 2', group: 'special' },
    ],
  },
  getItem: vi.fn(),
}))

const mockRank = vi.mocked(rankSlotItemsNative)
const mockPicker = vi.mocked(pickerItemsForSlot)
const mockGetItem = vi.mocked(getItem)

function makeDeps(overrides: Partial<BuildPerformanceDeps> = {}): BuildPerformanceDeps {
  return {
    classId: 'amazon',
    level: 50,
    allocatedAttrs: {},
    inventory: {},
    skillRanks: {},
    subskillRanks: {},
    activeAuraId: null,
    activeBuffs: {},
    customStats: [],
    allocatedTreeNodes: new Set(),
    allocatedIncarnationNodes: new Set(),
    treeSocketed: {},
    activeSkillIds: ['skill-1'],
    enemyConditions: {},
    playerConditions: {},
    skillProjectiles: {},
    enemyResistances: {},
    procToggles: {},
    killsPerSec: 0,
    ...overrides,
  }
}

beforeEach(() => {
  mockRank.mockReset()
  mockPicker.mockReset()
  mockPicker.mockReturnValue([
    { id: 'base_a', name: 'Base A' },
    { id: 'base_b', name: 'Base B' },
  ])
  mockGetItem.mockReset()
  mockGetItem.mockReturnValue(undefined)
})

describe('scanForUpgrades', () => {
  it('returns empty without a main skill and never calls the engine', async () => {
    const out = await scanForUpgrades(makeDeps({ activeSkillIds: [] }))
    expect(out.emptySlots).toHaveLength(0)
    expect(out.upgrades).toHaveLength(0)
    expect(mockRank).not.toHaveBeenCalled()
  })

  it('skips charm slots entirely', async () => {
    mockRank.mockResolvedValue({ base_a: 100, base_b: 120 })
    await scanForUpgrades(makeDeps())
    const scannedSlots = mockRank.mock.calls.map((c) => c[1])
    expect(scannedSlots).toEqual([
      'helm',
      'weapon',
      'offhand',
      'boots',
      'gloves',
      'belt',
      'ring',
      'amulet',
      'relic_1',
      'relic_2',
    ])
  })

  it('puts the empty slot in emptySlots and caps occupied upgrades at 5', async () => {
    mockRank.mockResolvedValue({ base_a: 100, base_b: 150 })
    const deps = makeDeps({
      inventory: {
        weapon: { baseId: 'base_a' },
        offhand: { baseId: 'base_a' },
        boots: { baseId: 'base_a' },
        gloves: { baseId: 'base_a' },
        belt: { baseId: 'base_a' },
        ring: { baseId: 'base_a' },
        amulet: { baseId: 'base_a' },
        relic_1: { baseId: 'base_a' },
        relic_2: { baseId: 'base_a' },
      } as BuildPerformanceDeps['inventory'],
    })
    const out = await scanForUpgrades(deps)
    expect(out.emptySlots).toEqual([{ slot: 'helm', slotName: 'Helm' }])
    expect(out.upgrades).toHaveLength(5)
    expect(out.upgrades[0]?.gainPct).toBeCloseTo(50)
  })

  it('omits occupied slots whose current base is already best', async () => {
    mockRank.mockResolvedValue({ base_a: 100, base_b: 101 })
    const deps = makeDeps({
      inventory: {
        helm: { baseId: 'base_b' },
        weapon: { baseId: 'base_b' },
      } as BuildPerformanceDeps['inventory'],
    })
    const out = await scanForUpgrades(deps)
    expect(
      out.upgrades.find((s) => s.slot === 'helm' || s.slot === 'weapon'),
    ).toBeUndefined()
  })

  it('omits gains at or below the 2% threshold when a better base exists', async () => {
    mockRank.mockResolvedValue({ base_a: 100, base_b: 101.99 })
    const deps = makeDeps({
      inventory: {
        helm: { baseId: 'base_a' },
        weapon: { baseId: 'base_a' },
        offhand: { baseId: 'base_a' },
        boots: { baseId: 'base_a' },
        gloves: { baseId: 'base_a' },
        belt: { baseId: 'base_a' },
        ring: { baseId: 'base_a' },
        amulet: { baseId: 'base_a' },
        relic_1: { baseId: 'base_a' },
        relic_2: { baseId: 'base_a' },
      } as BuildPerformanceDeps['inventory'],
    })
    const out = await scanForUpgrades(deps)
    expect(out.upgrades).toHaveLength(0)
    expect(out.emptySlots).toHaveLength(0)
  })

  it('names both sides of the swap on each suggestion', async () => {
    mockRank.mockResolvedValue({ base_a: 100, base_b: 150 })
    const deps = makeDeps({
      inventory: { helm: { baseId: 'base_a' } } as BuildPerformanceDeps['inventory'],
    })
    const out = await scanForUpgrades(deps)
    expect(out.upgrades[0]).toMatchObject({
      currentBaseName: 'Base A',
      bestBaseName: 'Base B',
    })
  })

  it('falls back to the item db name when the current base is not a picker row', async () => {
    mockRank.mockResolvedValue({ base_x: 100, base_a: 90, base_b: 150 })
    mockGetItem.mockReturnValue({ name: 'Custom X' } as ReturnType<typeof getItem>)
    const deps = makeDeps({
      inventory: { helm: { baseId: 'base_x' } } as BuildPerformanceDeps['inventory'],
    })
    const out = await scanForUpgrades(deps)
    expect(out.upgrades[0]?.currentBaseName).toBe('Custom X')
  })

  it('skips occupied slots whose current score is missing or non-positive', async () => {
    mockRank.mockResolvedValue({ base_a: 0, base_b: 120 })
    const deps = makeDeps({
      inventory: { helm: { baseId: 'base_a' } } as BuildPerformanceDeps['inventory'],
    })
    const out = await scanForUpgrades(deps)
    expect(out.upgrades.find((s) => s.slot === 'helm')).toBeUndefined()
  })

  it('surfaces every empty slot uncapped, with no numeric upgrades', async () => {
    mockRank.mockResolvedValue({ base_a: 100, base_b: 120 })
    const out = await scanForUpgrades(makeDeps())
    expect(out.emptySlots).toHaveLength(10)
    expect(out.upgrades).toHaveLength(0)
  })

  it('reports progress after each slot', async () => {
    mockRank.mockResolvedValue({ base_a: 100, base_b: 120 })
    const seen: Array<[number, number]> = []
    await scanForUpgrades(makeDeps(), (done, total) => seen.push([done, total]))
    expect(seen).toEqual(
      Array.from({ length: 10 }, (_, i) => [i + 1, 10]),
    )
  })

  it('includes the current base in the ranked id list exactly once', async () => {
    mockRank.mockResolvedValue({ base_a: 100, base_b: 150 })
    const deps = makeDeps({
      inventory: { helm: { baseId: 'base_a' } } as BuildPerformanceDeps['inventory'],
    })
    await scanForUpgrades(deps)
    const helmCall = mockRank.mock.calls.find((c) => c[1] === 'helm')
    expect(helmCall?.[2].filter((id) => id === 'base_a')).toHaveLength(1)
  })

  it('surfaces empty slots even when every candidate scores zero', async () => {
    mockRank.mockResolvedValue({ base_a: 0, base_b: 0 })
    const out = await scanForUpgrades(makeDeps())
    expect(out.emptySlots).toContainEqual({ slot: 'helm', slotName: 'Helm' })
    expect(out.upgrades).toHaveLength(0)
  })

  it('sorts occupied upgrades by gain descending, separately from empty slots', async () => {
    mockRank.mockImplementation(async (_deps, slot) =>
      slot === 'helm' ? { base_a: 100, base_b: 150 } : { base_a: 100, base_b: 110 },
    )
    const deps = makeDeps({
      inventory: {
        helm: { baseId: 'base_a' },
        weapon: { baseId: 'base_a' },
        offhand: { baseId: 'base_b' },
        gloves: { baseId: 'base_b' },
        belt: { baseId: 'base_b' },
        ring: { baseId: 'base_b' },
        amulet: { baseId: 'base_b' },
        relic_1: { baseId: 'base_b' },
        relic_2: { baseId: 'base_b' },
      } as BuildPerformanceDeps['inventory'],
    })
    const out = await scanForUpgrades(deps)
    expect(out.emptySlots).toEqual([{ slot: 'boots', slotName: 'Boots' }])
    expect(out.upgrades.map((s) => s.slot)).toEqual(['helm', 'weapon'])
    expect(out.upgrades[0]?.gainPct ?? 0).toBeGreaterThan(
      out.upgrades[1]?.gainPct ?? 0,
    )
  })

  it('skips the offhand slot when the equipped weapon is two-handed', async () => {
    mockGetItem.mockReturnValue({ twoHanded: true } as unknown as ReturnType<
      typeof getItem
    >)
    mockRank.mockResolvedValue({ base_a: 100, base_b: 120 })
    const deps = makeDeps({
      inventory: { weapon: { baseId: 'base_a' } } as BuildPerformanceDeps['inventory'],
    })
    await scanForUpgrades(deps)
    const scannedSlots = mockRank.mock.calls.map((c) => c[1])
    expect(scannedSlots).not.toContain('offhand')
  })

  it('never suggests a relic already equipped in a sibling relic slot', async () => {
    mockRank.mockResolvedValue({ base_a: 100, base_b: 150 })
    const deps = makeDeps({
      inventory: {
        relic_1: { baseId: 'base_a' },
        relic_2: { baseId: 'base_b' },
      } as BuildPerformanceDeps['inventory'],
    })
    const out = await scanForUpgrades(deps)
    expect(out.upgrades.find((s) => s.slot === 'relic_1')).toBeUndefined()
    const relic1Call = mockRank.mock.calls.find((c) => c[1] === 'relic_1')
    expect(relic1Call?.[2]).not.toContain('base_b')
  })

  it('suggests each relic base at most once across the relic slots', async () => {
    mockPicker.mockReturnValue([
      { id: 'base_a', name: 'Base A' },
      { id: 'base_b', name: 'Base B' },
      { id: 'base_c', name: 'Base C' },
    ])
    mockRank.mockResolvedValue({ base_a: 100, base_b: 150, base_c: 120 })
    const deps = makeDeps({
      inventory: {
        relic_1: { baseId: 'base_a' },
        relic_2: { baseId: 'base_a' },
      } as BuildPerformanceDeps['inventory'],
    })
    const out = await scanForUpgrades(deps)
    expect(out.upgrades.map((s) => [s.slot, s.bestBaseId])).toEqual([
      ['relic_1', 'base_b'],
      ['relic_2', 'base_c'],
    ])
  })

  it('still lets the other ring slot hold the same base', async () => {
    mockRank.mockResolvedValue({ base_a: 100, base_b: 150 })
    const deps = makeDeps({
      inventory: {
        ring: { baseId: 'base_a' },
        amulet: { baseId: 'base_b' },
      } as BuildPerformanceDeps['inventory'],
    })
    const out = await scanForUpgrades(deps)
    expect(out.upgrades.find((s) => s.slot === 'ring')?.bestBaseId).toBe('base_b')
  })

  it('scans the offhand slot when the equipped weapon is one-handed', async () => {
    mockGetItem.mockReturnValue({ twoHanded: false } as unknown as ReturnType<
      typeof getItem
    >)
    mockRank.mockResolvedValue({ base_a: 100, base_b: 120 })
    const deps = makeDeps({
      inventory: { weapon: { baseId: 'base_a' } } as BuildPerformanceDeps['inventory'],
    })
    await scanForUpgrades(deps)
    const scannedSlots = mockRank.mock.calls.map((c) => c[1])
    expect(scannedSlots).toContain('offhand')
  })
})
