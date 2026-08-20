import { getItem, getItemGrantedSkillByName } from '@data'
import type { EquippedItem } from '../../types'
import { rangedMax, rangedMin } from '../../utils/item/stats'

// Mirrors ITEM_PROC_ICD_SECS in engine/src/calc/build.rs — item procs fire
// on an internal cooldown even when their listed cooldown is shorter.
export const ITEM_PROC_ICD_SECS = 1.5

export interface ItemProcRow {
  id: string
  name: string
  toggleKey: string
  rankMin: number
  rankMax: number
  types: string[]
  intervalSecs: number
}

export function itemProcRows(
  inventory: Record<string, EquippedItem | undefined>,
): ItemProcRow[] {
  const best = new Map<string, ItemProcRow>()
  for (const equipped of Object.values(inventory)) {
    if (!equipped) continue
    const base = getItem(equipped.baseId)
    for (const [skillName, range] of Object.entries(base?.skillBonuses ?? {})) {
      const def = getItemGrantedSkillByName(skillName)
      if (!def?.procDamage?.length) continue
      const rankMin = rangedMin(range)
      const rankMax = rangedMax(range)
      const cur = best.get(def.id)
      if (cur && cur.rankMax >= rankMax) continue
      best.set(def.id, {
        id: def.id,
        name: def.name,
        toggleKey: `granted:${def.id}`,
        rankMin,
        rankMax,
        types: [...new Set(def.procDamage.map((p) => p.type))],
        intervalSecs: Math.max(def.procCooldown ?? ITEM_PROC_ICD_SECS, ITEM_PROC_ICD_SECS),
      })
    }
  }
  return [...best.values()].sort((a, b) => a.name.localeCompare(b.name))
}
