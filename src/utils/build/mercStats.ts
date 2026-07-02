import { gameConfig, getItem, getItemGrantedSkillByName } from '../../data'
import type { Inventory } from '../../types'
import { rangedMax, rangedMin } from '../item/stats'
import type { BuildPerformanceDeps } from './buildPerformance'
import { defaultEnemyResistances } from './shareBuild'

const GRANT_AURA_RE = /^Grant Aura:\s*(.+?)\s+Level\b/i

export function mercOnlyDeps(mercInventory: Inventory): BuildPerformanceDeps {
  const allocatedAttrs = Object.fromEntries(
    gameConfig.attributes.map((a) => [a.key, 0]),
  )
  return {
    classId: null,
    level: 1,
    allocatedAttrs,
    inventory: mercInventory,
    skillRanks: {},
    subskillRanks: {},
    activeAuraId: null,
    activeBuffs: {},
    customStats: [],
    allocatedTreeNodes: new Set<number>(),
    treeSocketed: {},
    activeSkillIds: [],
    enemyConditions: {},
    playerConditions: {},
    skillProjectiles: {},
    enemyResistances: defaultEnemyResistances(),
    procToggles: {},
    killsPerSec: 1,
  }
}

export interface MercSharedEffect {
  itemName: string
  effect: string
}

export function mercSharedEffects(mercInventory: Inventory): MercSharedEffect[] {
  const out: MercSharedEffect[] = []
  for (const equipped of Object.values(mercInventory)) {
    if (!equipped) continue
    const base = getItem(equipped.baseId)
    if (!base) continue
    for (const effect of base.uniqueEffects ?? []) {
      out.push({ itemName: base.name, effect })
    }
  }
  return out
}

export function hasMercGear(mercInventory: Inventory): boolean {
  return Object.values(mercInventory).some((item) => item != null)
}

export interface MercGrantedAura {
  name: string
  itemName: string
  baseId: string
  levelMin: number
  levelMax: number
}

export function mercAuraKey(name: string): string {
  return name.trim().toLowerCase()
}

export function mercGrantedAuras(mercInventory: Inventory): MercGrantedAura[] {
  const out: MercGrantedAura[] = []
  for (const equipped of Object.values(mercInventory)) {
    if (!equipped) continue
    const base = getItem(equipped.baseId)
    if (!base) continue
    for (const effect of base.uniqueEffects ?? []) {
      const name = GRANT_AURA_RE.exec(effect)?.[1]?.trim()
      if (!name || !getItemGrantedSkillByName(name)) continue
      const level = base.skillBonuses?.[name]
      if (level == null) continue
      out.push({
        name,
        itemName: base.name,
        baseId: base.id,
        levelMin: rangedMin(level),
        levelMax: rangedMax(level),
      })
    }
  }
  return out.sort((a, b) => a.name.localeCompare(b.name))
}

// counted ranks for the calc input; same aura from two items -> higher wins
export function mercGrantedSkillRanks(
  mercInventory: Inventory,
  disabledAuras: Record<string, boolean>,
): Record<string, [number, number]> | undefined {
  const out: Record<string, [number, number]> = {}
  for (const aura of mercGrantedAuras(mercInventory)) {
    if (disabledAuras[mercAuraKey(aura.name)]) continue
    const cur = out[aura.name]
    out[aura.name] = cur
      ? [Math.max(cur[0], aura.levelMin), Math.max(cur[1], aura.levelMax)]
      : [aura.levelMin, aura.levelMax]
  }
  return Object.keys(out).length > 0 ? out : undefined
}
