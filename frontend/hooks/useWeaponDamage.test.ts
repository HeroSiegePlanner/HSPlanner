import { renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { NativeWeaponDamageInput } from '../utils/nativeDamage'
import { computeWeaponDamageNative } from '../utils/nativeDamage'
import type { WeaponDamageBreakdown } from '../utils/item/stats'
import { useWeaponDamage } from './useWeaponDamage'

vi.mock('../utils/nativeDamage', () => ({
  computeWeaponDamageNative: vi.fn(),
}))

const mockedCompute = vi.mocked(computeWeaponDamageNative)

const fakeBreakdown = {
  hasWeapon: true,
  hitMin: 5,
  hitMax: 12,
} as unknown as WeaponDamageBreakdown

const fakeInput = {
  weapon: { name: 'Sword', damageMin: 1, damageMax: 4 },
  stats: {},
  enemyConditions: {},
} as unknown as NativeWeaponDamageInput

beforeEach(() => {
  mockedCompute.mockReset()
})

describe('useWeaponDamage', () => {
  it('starts with null result before the native call resolves', () => {
    mockedCompute.mockReturnValue(new Promise(() => {}))
    const { result } = renderHook(() => useWeaponDamage(fakeInput))
    expect(result.current).toBeNull()
  })

  it('populates result once the native call resolves', async () => {
    mockedCompute.mockResolvedValue(fakeBreakdown)
    const { result } = renderHook(() => useWeaponDamage(fakeInput))
    await waitFor(() => expect(result.current).toEqual(fakeBreakdown))
    expect(mockedCompute).toHaveBeenCalledWith(fakeInput)
  })

  it('logs and resets to null when the native call rejects', async () => {
    mockedCompute.mockRejectedValue(new Error('IPC fail'))
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const { result } = renderHook(() => useWeaponDamage(fakeInput))
    await waitFor(() => expect(spy).toHaveBeenCalled())
    expect(result.current).toBeNull()
    spy.mockRestore()
  })
})
