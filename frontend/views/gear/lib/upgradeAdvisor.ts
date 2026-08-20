import { gameConfig, getItem } from '@data'
import { rankSlotItemsNative } from '../../../utils/calc/bridge'
import { pickerItemsForSlot } from '../pickerItems'
import type { BuildPerformanceDeps } from '../../../utils/build/buildPerformance'
import type { PickerRow } from '../PickerModal'
import type { SlotDef, SlotKey } from '../../../types'

export interface UpgradeSuggestion {
  slot: SlotKey
  slotName: string
  currentBaseName: string
  bestBaseId: string
  bestBaseName: string
  gainPct: number
}

export interface UpgradeScanResult {
  emptySlots: { slot: SlotKey; slotName: string }[]
  upgrades: UpgradeSuggestion[]
}

export const UPGRADE_MIN_GAIN_PCT = 2
export const UPGRADE_MAX_COUNT = 5

function evaluateSlot(
  slot: SlotDef,
  scores: Record<string, number>,
  rows: PickerRow[],
  currentBaseId: string,
): UpgradeSuggestion | null {
  let bestId: string | null = null
  let bestScore = 0
  for (const row of rows) {
    const score = scores[row.id] ?? 0
    if (score > bestScore) {
      bestScore = score
      bestId = row.id
    }
  }
  if (bestId === null) return null
  const bestName = rows.find((r) => r.id === bestId)?.name ?? bestId

  const currentScore = scores[currentBaseId] ?? 0
  if (currentScore <= 0 || bestId === currentBaseId) return null
  const gainPct = (bestScore / currentScore - 1) * 100
  if (gainPct <= UPGRADE_MIN_GAIN_PCT) return null
  const currentName =
    rows.find((r) => r.id === currentBaseId)?.name ??
    getItem(currentBaseId)?.name ??
    currentBaseId
  return {
    slot: slot.key,
    slotName: slot.name,
    currentBaseName: currentName,
    bestBaseId: bestId,
    bestBaseName: bestName,
    gainPct,
  }
}

export async function scanForUpgrades(
  deps: BuildPerformanceDeps,
  onProgress?: (done: number, total: number) => void,
): Promise<UpgradeScanResult> {
  if (deps.activeSkillIds.length === 0) return { emptySlots: [], upgrades: [] }

  const isTwoHanded = !!getItem(deps.inventory.weapon?.baseId ?? '')?.twoHanded
  const slots = gameConfig.slots.filter(
    (s) => !s.key.startsWith('charm_') && (s.key !== 'offhand' || !isTwoHanded),
  )
  const emptySlots: UpgradeScanResult['emptySlots'] = []
  const upgrades: UpgradeSuggestion[] = []

  for (const [index, slot] of slots.entries()) {
    const rows = pickerItemsForSlot(slot.key)
    const currentBaseId = deps.inventory[slot.key]?.baseId
    const ids = [
      ...new Set([
        ...rows.map((r) => r.id),
        ...(currentBaseId ? [currentBaseId] : []),
      ]),
    ]
    if (ids.length === 0) {
      onProgress?.(index + 1, slots.length)
      continue
    }

    const scores = await rankSlotItemsNative(deps, slot.key, ids)
    onProgress?.(index + 1, slots.length)

    if (currentBaseId === undefined) {
      emptySlots.push({ slot: slot.key, slotName: slot.name })
      continue
    }

    const suggestion = evaluateSlot(slot, scores, rows, currentBaseId)
    if (suggestion) upgrades.push(suggestion)
  }

  return {
    emptySlots,
    upgrades: upgrades
      .toSorted((a, b) => b.gainPct - a.gainPct)
      .slice(0, UPGRADE_MAX_COUNT),
  }
}
