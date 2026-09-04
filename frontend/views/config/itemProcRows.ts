import { getItem, getItemGrantedSkillByName, skills } from '@data'
import type { EquippedItem } from '../../types'
import { normalizeSkillName, rangedMax, rangedMin } from '../../utils/item/stats'

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
  /** Set on procs that cast a class skill; they list a chance, not an interval. */
  chance?: number
  trigger?: string
  sourceName?: string
}

// Mirrors item_cast_toggle_key in engine/src/calc/build.rs.
export function itemCastToggleKey(baseId: string, targetName: string): string {
  return `cast:${baseId}:${normalizeSkillName(targetName)}`
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
  const cast: ItemProcRow[] = []
  for (const equipped of Object.values(inventory)) {
    if (!equipped) continue
    const base = getItem(equipped.baseId)
    for (const proc of base?.procs ?? []) {
      if (!proc.target || proc.castLevel == null) continue
      const skill = skills.find(
        (s) => normalizeSkillName(s.name) === normalizeSkillName(proc.target!),
      )
      if (!skill) continue
      cast.push({
        id: `${equipped.baseId}:${skill.id}`,
        name: skill.name,
        toggleKey: itemCastToggleKey(equipped.baseId, proc.target),
        rankMin: proc.castLevel,
        rankMax: proc.castLevel,
        types: skill.damageType ? [skill.damageType] : [],
        intervalSecs: ITEM_PROC_ICD_SECS,
        chance: proc.chance ?? 0,
        trigger: proc.trigger,
        sourceName: base?.name ?? equipped.baseId,
      })
    }
  }
  return [...best.values(), ...cast].sort((a, b) => a.name.localeCompare(b.name))
}
