import { renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { NativeSkillDamageInput } from '../utils/nativeDamage'
import { computeSkillDamageNative } from '../utils/nativeDamage'
import type { SkillDamageBreakdown } from '../utils/stats'
import { useSkillDamage } from './useSkillDamage'

vi.mock('../utils/nativeDamage', () => ({
  computeSkillDamageNative: vi.fn(),
}))

const mockedCompute = vi.mocked(computeSkillDamageNative)

const fakeBreakdown = { hitMin: 10, hitMax: 20 } as unknown as SkillDamageBreakdown
const fakeInput = {
  skill: { name: 'X' },
  allocatedRank: 5,
} as unknown as NativeSkillDamageInput

beforeEach(() => {
  mockedCompute.mockReset()
})

describe('useSkillDamage', () => {
  it('returns null and does not invoke native when input is null', () => {
    const { result } = renderHook(() => useSkillDamage(null))
    expect(result.current).toBeNull()
    expect(mockedCompute).not.toHaveBeenCalled()
  })

  it('populates result once the native call resolves', async () => {
    mockedCompute.mockResolvedValue(fakeBreakdown)
    const { result } = renderHook(() => useSkillDamage(fakeInput))
    await waitFor(() => expect(result.current).toEqual(fakeBreakdown))
    expect(mockedCompute).toHaveBeenCalledWith(fakeInput)
  })

  it('logs and keeps result null when the native call rejects', async () => {
    mockedCompute.mockRejectedValue(new Error('IPC fail'))
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const { result } = renderHook(() => useSkillDamage(fakeInput))
    await waitFor(() => expect(spy).toHaveBeenCalled())
    expect(result.current).toBeNull()
    spy.mockRestore()
  })

  it('clears the result when input transitions back to null', async () => {
    mockedCompute.mockResolvedValue(fakeBreakdown)
    const { result, rerender } = renderHook(
      ({ input }: { input: NativeSkillDamageInput | null }) =>
        useSkillDamage(input),
      { initialProps: { input: fakeInput } },
    )
    await waitFor(() => expect(result.current).toEqual(fakeBreakdown))
    rerender({ input: null })
    expect(result.current).toBeNull()
  })
})
