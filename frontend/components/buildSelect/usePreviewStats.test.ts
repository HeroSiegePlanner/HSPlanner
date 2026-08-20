import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'

const computeMock = vi.fn()
vi.mock('../../utils/calc/bridge', () => ({
  computeBuildPerformanceAsync: (deps: unknown) => computeMock(deps),
}))

const decodeMock = vi.fn()
vi.mock('../../utils/build/shareBuild', () => ({
  decodeShareToBuild: (code: string) => decodeMock(code),
}))

vi.mock('../../utils/build/savedBuilds', () => ({
  getActiveProfile: () => ({ id: 'p1', name: 'P1', code: 'CODE' }),
}))

vi.mock('../../utils/build/buildPerformance', () => ({
  applyDisabledPotions: (inventory: unknown, disabledPotions: unknown) => ({
    __filtered: true,
    inventory,
    disabledPotions,
  }),
}))

vi.mock('../../utils/build/mercStats', () => ({
  mercGrantedSkillRanks: vi.fn(() => ({ merc_aura: [3, 3] })),
}))

import { usePreviewStats } from './usePreviewStats'
import { mercGrantedSkillRanks } from '../../utils/build/mercStats'
import type { SavedBuild } from '../../utils/build/savedBuilds'

afterEach(() => {
  vi.clearAllMocks()
  vi.useRealTimers()
})

describe('usePreviewStats season source', () => {
  it('computes with the build metadata season, not the stale blob season', async () => {
    vi.useFakeTimers()
    computeMock.mockResolvedValue({})
    decodeMock.mockReturnValue({ snapshot: {}, season: 's10' })
    const build = { id: 'b1', season: 's9' } as unknown as SavedBuild

    renderHook(() => usePreviewStats(build))
    await act(async () => {
      vi.advanceTimersByTime(200)
    })

    expect(computeMock).toHaveBeenCalledTimes(1)
    expect(computeMock.mock.calls[0]![0]).toMatchObject({ season: 's9' })
  })
})

describe('usePreviewStats calc parity', () => {
  it('filters disabled potions and passes merc-granted skill ranks', async () => {
    vi.useFakeTimers()
    computeMock.mockResolvedValue({})
    const snapshot = {
      inventory: { helmet: null },
      disabledPotions: { potion_hp: true },
      mercInventory: { weapon: { id: 'merc_axe' } },
      mercDisabledAuras: { merc_aura: true },
    }
    decodeMock.mockReturnValue({ snapshot, season: 's10' })
    const build = { id: 'b1', season: 's10' } as unknown as SavedBuild

    renderHook(() => usePreviewStats(build))
    await act(async () => {
      vi.advanceTimersByTime(200)
    })

    expect(computeMock.mock.calls[0]![0]).toMatchObject({
      inventory: { __filtered: true, disabledPotions: { potion_hp: true } },
      grantedSkillRanks: { merc_aura: [3, 3] },
    })
    expect(mercGrantedSkillRanks).toHaveBeenCalledWith(
      snapshot.mercInventory,
      snapshot.mercDisabledAuras,
    )
  })
})
