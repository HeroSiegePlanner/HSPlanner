import { entityRatesFrom } from '../../utils/build/entityRates'
import { gameConfig, getClass } from '@data'
import { defaultEnemyResistances } from '../../utils/build/shareBuild'
import type { BuildSnapshot } from '../../utils/build/shareBuild'
import { pruneUnknownIds } from '../../utils/build/seasonMigration'
import type { AttrMap, BuildState } from './types'

export function emptyAllocation(): AttrMap {
  return gameConfig.attributes.reduce<AttrMap>((acc, a) => {
    acc[a.key] = 0
    return acc
  }, {})
}

export function bumpSavedBuilds(
  set: (fn: (s: BuildState) => Partial<BuildState>) => void,
) {
  set((s) => ({ savedBuildsVersion: s.savedBuildsVersion + 1 }))
}

export function snapshotPatch(rawSnap: BuildSnapshot) {
  const snap = pruneUnknownIds(rawSnap)
  return {
    classId: snap.classId,
    level: snap.level,
    allocated: snap.allocated,
    inventory: snap.inventory,
    skillRanks: snap.skillRanks,
    subskillRanks: snap.subskillRanks,
    allocatedTreeNodes: new Set(snap.allocatedTreeNodes),
    treeSocketed: snap.treeSocketed ?? {},
    activeSkillIds: snap.activeSkillIds,
    activeAuraId: snap.activeAuraId,
    activeBuffs: snap.activeBuffs,
    enemyConditions: snap.enemyConditions,
    playerConditions: snap.playerConditions ?? {},
    skillProjectiles: snap.skillProjectiles ?? {},
    enemyResistances: snap.enemyResistances ?? defaultEnemyResistances(),
    procToggles: snap.procToggles,
    disabledPotions: snap.disabledPotions ?? {},
    killsPerSec: snap.killsPerSec,
    entityRates: entityRatesFrom(snap.entityRates, snap.entityAttacksPerSecond),
    customStats: snap.customStats ?? [],
    allocatedEtherNodes: snap.allocatedEtherNodes ?? new Set<number>(),
    mercClassId: snap.mercClassId ?? null,
    mercSkillRanks: snap.mercSkillRanks ?? {},
    mercInventory: snap.mercInventory ?? {},
    mercDisabledAuras: snap.mercDisabledAuras ?? {},
  }
}

export function skillPointsFor(level: number): number {
  return level * gameConfig.skillPointsPerLevel
}

export function subskillPointsFor(level: number): number {
  return Math.floor(level / gameConfig.levelsPerSubskillPoint)
}

export function subskillKey(skillId: string, subskillId: string): string {
  return `${skillId}:${subskillId}`
}

export function subskillSpentFor(
  subskillRanks: Record<string, number>,
  skillId: string,
): number {
  const prefix = `${skillId}:`
  return Object.entries(subskillRanks).reduce(
    (sum, [key, rank]) => (key.startsWith(prefix) ? sum + rank : sum),
    0,
  )
}

export function attrPointsFor(level: number): number {
  return level * gameConfig.attributePointsPerLevel
}

export function finalAttributes(
  classId: string | null,
  allocated: AttrMap,
): AttrMap {
  const cls = classId ? getClass(classId) : undefined
  const out = emptyAllocation()
  for (const attr of gameConfig.attributes) {
    const defaultBase = gameConfig.defaultBaseAttributes?.[attr.key] ?? 0
    const classBase = cls?.baseAttributes[attr.key] ?? 0
    const spent = allocated[attr.key] ?? 0
    out[attr.key] = defaultBase + classBase + spent
  }
  return out
}
