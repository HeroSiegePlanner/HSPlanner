import { act, renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { items } from '@data'
import type { EquippedItem } from '../../../types'
import { isSameItem, useGearDraft } from './useGearDraft'

function equippedFixture(baseId: string): EquippedItem {
  return {
    baseId,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
    stars: 0,
    forgedMods: [],
  }
}

describe('isSameItem', () => {
  it('treats deep-equal items as the same', () => {
    expect(isSameItem(equippedFixture('x'), equippedFixture('x'))).toBe(true)
  })

  it('treats null vs item as different and null vs null as same', () => {
    expect(isSameItem(null, equippedFixture('x'))).toBe(false)
    expect(isSameItem(null, null)).toBe(true)
  })

  it('detects a changed field', () => {
    const a = equippedFixture('x')
    expect(isSameItem(a, { ...a, stars: 2 })).toBe(false)
  })
})

describe('useGearDraft', () => {
  it('initialises the draft as a clone (not the same reference)', () => {
    const eq = equippedFixture(items[0].id)
    const { result } = renderHook(() => useGearDraft(eq))
    expect(result.current.draft).toEqual(eq)
    expect(result.current.draft).not.toBe(eq)
    expect(result.current.dirty).toBe(false)
  })

  it('setStars makes the draft dirty and updates it', () => {
    const { result } = renderHook(() => useGearDraft(equippedFixture(items[0].id)))
    act(() => result.current.setStars(3))
    expect(result.current.draft?.stars).toBe(3)
    expect(result.current.dirty).toBe(true)
  })

  it('reverting an edit clears dirty', () => {
    const { result } = renderHook(() => useGearDraft(equippedFixture(items[0].id)))
    act(() => result.current.setStars(3))
    act(() => result.current.setStars(0))
    expect(result.current.dirty).toBe(false)
  })

  it('clearDraft makes draft null and dirty when something was equipped', () => {
    const { result } = renderHook(() => useGearDraft(equippedFixture(items[0].id)))
    act(() => result.current.clearDraft())
    expect(result.current.draft).toBeNull()
    expect(result.current.dirty).toBe(true)
  })

  it('pickBase swaps the draft to a fresh item', () => {
    const { result } = renderHook(() => useGearDraft(undefined))
    act(() => result.current.pickBase(items[0].id))
    expect(result.current.draft?.baseId).toBe(items[0].id)
    expect(result.current.dirty).toBe(true)
  })
})
