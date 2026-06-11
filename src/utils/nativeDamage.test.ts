import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { setBridgeErrorListener } from '../lib/calc/bridge'
import type {
  NativeSkillDamageInput,
  NativeWeaponDamageInput,
} from './nativeDamage'
import { computeSkillDamageNative, computeWeaponDamageNative } from './nativeDamage'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockedInvoke = vi.mocked(invoke)
const skillInput = { skill: { name: 'X' }, allocatedRank: 1 } as NativeSkillDamageInput
const weaponInput = {} as NativeWeaponDamageInput

describe('nativeDamage error notification', () => {
  const listener = vi.fn()

  beforeEach(() => {
    mockedInvoke.mockReset()
    listener.mockReset()
    setBridgeErrorListener(listener)
  })

  afterEach(() => {
    setBridgeErrorListener(null)
  })

  it('computeSkillDamageNative resolves with the invoke result', async () => {
    mockedInvoke.mockResolvedValue(null)
    await expect(computeSkillDamageNative(skillInput)).resolves.toBeNull()
    expect(listener).not.toHaveBeenCalled()
  })

  it('computeSkillDamageNative notifies the listener and rejects on failure', async () => {
    mockedInvoke.mockRejectedValue(new Error('IPC fail'))
    await expect(computeSkillDamageNative(skillInput)).rejects.toThrow('IPC fail')
    expect(listener).toHaveBeenCalledTimes(1)
    expect(listener.mock.calls[0]![0]).toBeInstanceOf(Error)
  })

  it('computeWeaponDamageNative notifies the listener and rejects on failure', async () => {
    mockedInvoke.mockRejectedValue('rust panic')
    await expect(computeWeaponDamageNative(weaponInput)).rejects.toBeInstanceOf(
      Error,
    )
    expect(listener).toHaveBeenCalledTimes(1)
  })
})
