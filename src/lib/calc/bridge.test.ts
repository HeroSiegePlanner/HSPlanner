import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import type { Skill } from '../../types'
import {
  manaCostAtRankNative,
  passiveStatsAtRankNative,
  setBridgeErrorListener,
} from './bridge'

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
