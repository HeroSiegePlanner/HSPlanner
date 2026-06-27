import { gameConfig, getClass } from '../../data'
import { defaultEnemyResistances } from '../../utils/build/shareBuild'
import type { BuildSnapshot } from '../../utils/build/shareBuild'
import type { AttrMap, BuildState } from './types'

export const RAINBOW_MULTIPLIER = 1.5

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

export function snapshotPatch(snap: BuildSnapshot) {
  return {
    classId: snap.classId,
    level: snap.level,
    allocated: snap.allocated,
    inventory: snap.inventory,
    skillRanks: snap.skillRanks,
    subskillRanks: snap.subskillRanks,
    allocatedTreeNodes: snap.allocatedTreeNodes,
    treeSocketed: snap.treeSocketed ?? {},
    activeSkillIds: snap.activeSkillIds,
    activeAuraId: snap.activeAuraId,
    activeBuffs: snap.activeBuffs,
    enemyConditions: snap.enemyConditions,
    playerConditions: snap.playerConditions ?? {},
    skillProjectiles: snap.skillProjectiles ?? {},
    enemyResistances: snap.enemyResistances ?? defaultEnemyResistances(),
    procToggles: snap.procToggles,
    killsPerSec: snap.killsPerSec,
    customStats: snap.customStats ?? [],
  }
}

export function skillPointsFor(level: number): number {
  return level * gameConfig.skillPointsPerLevel
}

export function subskillPointsFor(level: number): number {
  return Math.floor(level / 5)
}

export function subskillKey(skillId: string, subskillId: string): string {
  return `${skillId}:${subskillId}`
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
