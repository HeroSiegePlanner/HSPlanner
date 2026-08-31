import { useBuild } from './build'
import { useSettings } from './settings'
import type { BuildStore } from './build/types'

export const AUTO_SAVE_DEBOUNCE_MS = 800

export const SNAPSHOT_KEYS = [
  'classId',
  'level',
  'allocated',
  'inventory',
  'skillRanks',
  'subskillRanks',
  'allocatedTreeNodes',
  'treeSocketed',
  'activeSkillIds',
  'activeAuraId',
  'activeBuffs',
  'enemyConditions',
  'playerConditions',
  'skillProjectiles',
  'enemyResistances',
  'procToggles',
  'disabledPotions',
  'killsPerSec',
  'entityRates',
  'customStats',
  'allocatedEtherNodes',
  'mercClassId',
  'mercSkillRanks',
  'mercInventory',
  'mercDisabledAuras',
  'notes',
  'stash',
  // Loadout slots ride along so a slot switch is both auto-saved and undoable:
  // undo restores the previous slot index together with the live state it held.
  'loadoutSlots',
  'activeLoadouts',
] as const satisfies readonly (keyof BuildStore)[]

let timer: ReturnType<typeof setTimeout> | null = null

export function initAutoSave(): () => void {
  const unsubscribe = useBuild.subscribe((state, prev) => {
    if (!useSettings.getState().autoSave) return
    if (!state.activeBuildId || !state.activeProfileId) return
    if (!SNAPSHOT_KEYS.some((k) => state[k] !== prev[k])) return
    if (timer !== null) clearTimeout(timer)
    timer = setTimeout(() => {
      timer = null
      useBuild.getState().saveBuildNow()
    }, AUTO_SAVE_DEBOUNCE_MS)
  })
  return () => {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
    unsubscribe()
  }
}
