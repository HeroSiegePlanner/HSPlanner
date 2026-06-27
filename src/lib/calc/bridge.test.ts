import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import type { Skill } from '../../types'
import {
  __depsToInputForTest,
  computeBuildPerformanceAsync,
  manaCostAtRankNative,
  passiveStatsAtRankNative,
  setBridgeErrorListener,
  type BuildPerformanceOutput,
} from './bridge'
import { activeSeasonId } from '../../data'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockedInvoke = vi.mocked(invoke)
const fakeSkill = { id: 'x', name: 'X' } as unknown as Skill

describe('bridge error notification', () => {
  const listener = vi.fn()

  beforeEach(() => {
    mockedInvoke.mockReset()
    listener.mockReset()
    setBridgeErrorListener(listener)
  })

  afterEach(() => {
    setBridgeErrorListener(null)
  })

  it('passiveStatsAtRankNative resolves with the invoke result', async () => {
    mockedInvoke.mockResolvedValue({ life: 10 })
    await expect(passiveStatsAtRankNative(fakeSkill, 3)).resolves.toEqual({
      life: 10,
    })
    expect(listener).not.toHaveBeenCalled()
  })

  it('passiveStatsAtRankNative notifies the listener and rejects on failure', async () => {
    mockedInvoke.mockRejectedValue('rust panic')
    await expect(passiveStatsAtRankNative(fakeSkill, 3)).rejects.toBeInstanceOf(
      Error,
    )
    expect(listener).toHaveBeenCalledTimes(1)
    expect(listener.mock.calls[0]![0]).toBeInstanceOf(Error)
  })

  it('manaCostAtRankNative resolves with the invoke result', async () => {
    mockedInvoke.mockResolvedValue(42)
    await expect(manaCostAtRankNative(fakeSkill, 3)).resolves.toBe(42)
    expect(listener).not.toHaveBeenCalled()
  })

  it('manaCostAtRankNative notifies the listener and rejects on failure', async () => {
    mockedInvoke.mockRejectedValue(new Error('IPC fail'))
    await expect(manaCostAtRankNative(fakeSkill, 3)).rejects.toThrow('IPC fail')
    expect(listener).toHaveBeenCalledTimes(1)
  })
})

function baseDeps(season?: string) {
  return {
    classId: 'amazon',
    level: 1,
    allocatedAttrs: { strength: 0, dexterity: 0, intelligence: 0, energy: 0, vitality: 0, armor: 0 },
    inventory: {},
    skillRanks: {},
    subskillRanks: {},
    activeAuraId: null,
    activeBuffs: {},
    customStats: [],
    allocatedTreeNodes: new Set<number>(),
    treeSocketed: {},
    activeSkillIds: [],
    enemyConditions: {},
    playerConditions: {},
    skillProjectiles: {},
    enemyResistances: {},
    procToggles: {},
    killsPerSec: 1,
    ...(season ? { season } : {}),
  }
}

describe('depsToInput season', () => {
  it('uses the deps season when provided', () => {
    expect(__depsToInputForTest(baseDeps('s10')).season).toBe('s10')
  })
  it('falls back to the active season when deps season is absent', () => {
    expect(__depsToInputForTest(baseDeps()).season).toBe(activeSeasonId)
  })
})

function fakeOutput(o: Partial<BuildPerformanceOutput>): BuildPerformanceOutput {
  return {
    attributes: {},
    stats: {},
    damage: null,
    attackDamage: null,
    hitDpsMin: null,
    hitDpsMax: null,
    avgHitDpsMin: null,
    avgHitDpsMax: null,
    procDpsMin: 0,
    procDpsMax: 0,
    combinedDpsMin: null,
    combinedDpsMax: null,
    activeSkillName: null,
    statsCombined: {},
    itemSkillBonuses: {},
    rankBonuses: {},
    ...o,
  }
}

describe('computeBuildPerformanceAsync — combined (combo) DPS', () => {
  beforeEach(() => {
    mockedInvoke.mockReset()
    setBridgeErrorListener(null)
  })

  it('combines per-skill DPS into combinedDps (proc counted once); hit/avg stay primary', async () => {
    mockedInvoke.mockImplementation(async (_cmd, args) => {
      const id = (args as { input: { mainSkillId: string | null } }).input
        .mainSkillId
      if (id === 'a')
        return fakeOutput({
          hitDpsMin: 120,
          hitDpsMax: 120,
          avgHitDpsMin: 100,
          avgHitDpsMax: 100,
          procDpsMin: 5,
          procDpsMax: 5,
          combinedDpsMin: 105,
          combinedDpsMax: 105,
          activeSkillName: 'A',
        })
      return fakeOutput({
        hitDpsMin: 90,
        hitDpsMax: 90,
        avgHitDpsMin: 80,
        avgHitDpsMax: 80,
        procDpsMin: 5,
        procDpsMax: 5,
        combinedDpsMin: 85,
        combinedDpsMax: 85,
        activeSkillName: 'B',
      })
    })

    const perf = await computeBuildPerformanceAsync({
      ...baseDeps(),
      activeSkillIds: ['a', 'b'],
    })

    expect(mockedInvoke).toHaveBeenCalledTimes(2)
    expect(perf.combinedDpsMin).toBe(185)
    expect(perf.procDpsMin).toBe(5)
    expect(perf.hitDpsMin).toBe(120)
    expect(perf.avgHitDpsMin).toBe(100)
    expect(perf.activeSkillName).toBe('A + B')
    expect(perf.perSkill).toEqual([
      { id: 'a', name: 'A', hitDpsMin: 120, hitDpsMax: 120 },
      { id: 'b', name: 'B', hitDpsMin: 90, hitDpsMax: 90 },
    ])
  })

  it('a single active skill matches the legacy single-skill result', async () => {
    mockedInvoke.mockResolvedValue(
      fakeOutput({
        avgHitDpsMin: 100,
        avgHitDpsMax: 100,
        procDpsMin: 5,
        procDpsMax: 5,
        combinedDpsMin: 105,
        combinedDpsMax: 105,
        activeSkillName: 'A',
      }),
    )
    const perf = await computeBuildPerformanceAsync({
      ...baseDeps(),
      activeSkillIds: ['a'],
    })
    expect(mockedInvoke).toHaveBeenCalledTimes(1)
    expect(perf.combinedDpsMin).toBe(105)
  })

  it('no active skill reports proc-only combined DPS via a single call', async () => {
    mockedInvoke.mockResolvedValue(
      fakeOutput({
        procDpsMin: 7,
        procDpsMax: 7,
        combinedDpsMin: 7,
        combinedDpsMax: 7,
      }),
    )
    const perf = await computeBuildPerformanceAsync({
      ...baseDeps(),
      activeSkillIds: [],
    })
    expect(mockedInvoke).toHaveBeenCalledTimes(1)
    expect(perf.combinedDpsMin).toBe(7)
  })
})
