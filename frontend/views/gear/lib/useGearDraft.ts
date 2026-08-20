import { useCallback, useMemo, useState } from 'react'
import type { EquippedItem, SocketType } from '../../../types'
import * as edits from './itemEdits'

export function isSameItem(
  a: EquippedItem | null,
  b: EquippedItem | null,
): boolean {
  return JSON.stringify(a) === JSON.stringify(b)
}

export function useGearDraft(equipped: EquippedItem | undefined) {
  const [baselineEquipped] = useState<EquippedItem | null>(() =>
    equipped ? structuredClone(equipped) : null,
  )
  const [draft, setDraft] = useState<EquippedItem | null>(() =>
    equipped ? structuredClone(equipped) : null,
  )

  const dirty = useMemo(
    () => !isSameItem(draft, baselineEquipped),
    [draft, baselineEquipped],
  )

  const edit = useCallback(
    (fn: (item: EquippedItem) => EquippedItem) =>
      setDraft((cur) => (cur ? fn(cur) : cur)),
    [],
  )

  const pickBase = useCallback(
    (baseId: string) => setDraft(edits.makeEquippedItem(baseId)),
    [],
  )
  const clearDraft = useCallback(() => setDraft(null), [])
  const replaceDraft = useCallback((item: EquippedItem) => setDraft(item), [])

  const setSocketCount = useCallback(
    (n: number) => edit((cur) => edits.withSocketCount(cur, n)),
    [edit],
  )
  const setSocketed = useCallback(
    (idx: number, id: string | null) => edit((cur) => edits.withSocketed(cur, idx, id)),
    [edit],
  )
  const setSocketType = useCallback(
    (idx: number, t: SocketType) => edit((cur) => edits.withSocketType(cur, idx, t)),
    [edit],
  )
  const setStars = useCallback(
    (n: number) => edit((cur) => edits.withStars(cur, n)),
    [edit],
  )
  const addAffix = useCallback(
    (affixId: string, tier: number) => edit((cur) => edits.withAffixAdded(cur, affixId, tier)),
    [edit],
  )
  const removeAffix = useCallback(
    (idx: number) => edit((cur) => edits.withAffixRemoved(cur, idx)),
    [edit],
  )
  const addForgedMod = useCallback(
    (modId: string, tier: number) => edit((cur) => edits.withForgedModAdded(cur, modId, tier)),
    [edit],
  )
  const removeForgedMod = useCallback(
    (idx: number) => edit((cur) => edits.withForgedModRemoved(cur, idx)),
    [edit],
  )
  const applyRuneword = useCallback(
    (rwId: string) => edit((cur) => edits.withRuneword(cur, rwId)),
    [edit],
  )
  const setAugment = useCallback(
    (id: string | null) => edit((cur) => edits.withAugment(cur, id)),
    [edit],
  )
  const setAugmentLevel = useCallback(
    (lvl: number) => edit((cur) => edits.withAugmentLevel(cur, lvl)),
    [edit],
  )

  return {
    draft,
    baselineEquipped,
    dirty,
    pickBase,
    clearDraft,
    replaceDraft,
    setSocketCount,
    setSocketed,
    setSocketType,
    setStars,
    addAffix,
    removeAffix,
    addForgedMod,
    removeForgedMod,
    applyRuneword,
    setAugment,
    setAugmentLevel,
  }
}
