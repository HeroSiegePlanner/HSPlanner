import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import type { Skill } from '../../types'
import {
  __depsToInputForTest,
  manaCostAtRankNative,
  passiveStatsAtRankNative,
  setBridgeErrorListener,
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
    mainSkillId: null,
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
