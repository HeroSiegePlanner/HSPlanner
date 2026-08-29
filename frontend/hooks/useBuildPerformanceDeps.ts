import { useMemo } from 'react'
import { useBuild } from '../store/build'
import {
  applyDisabledPotions,
  type BuildPerformanceDeps,
} from '../utils/build/buildPerformance'
import { mercGrantedSkillRanks } from '../utils/build/mercStats'
import { boostedSubskillRanks } from '../utils/build/subskillBoost'

export function useBuildPerformanceDeps(): BuildPerformanceDeps {
  const classId = useBuild((s) => s.classId)
  const level = useBuild((s) => s.level)
  const allocatedAttrs = useBuild((s) => s.allocated)
  const inventory = useBuild((s) => s.inventory)
  const disabledPotions = useBuild((s) => s.disabledPotions)
  const skillRanks = useBuild((s) => s.skillRanks)
  const subskillRanks = useBuild((s) => s.subskillRanks)
  const activeAuraId = useBuild((s) => s.activeAuraId)
  const activeBuffs = useBuild((s) => s.activeBuffs)
  const customStats = useBuild((s) => s.customStats)
  const allocatedTreeNodes = useBuild((s) => s.allocatedTreeNodes)
  const treeSocketed = useBuild((s) => s.treeSocketed)
  const activeSkillIds = useBuild((s) => s.activeSkillIds)
  const enemyConditions = useBuild((s) => s.enemyConditions)
  const playerConditions = useBuild((s) => s.playerConditions)
  const skillProjectiles = useBuild((s) => s.skillProjectiles)
  const enemyResistances = useBuild((s) => s.enemyResistances)
  const procToggles = useBuild((s) => s.procToggles)
  const killsPerSec = useBuild((s) => s.killsPerSec)
  const entityRates = useBuild((s) => s.entityRates)
  const stackCounts = useBuild((s) => s.stackCounts)
  const mercInventory = useBuild((s) => s.mercInventory)
  const mercDisabledAuras = useBuild((s) => s.mercDisabledAuras)

  const inventoryForCalc = useMemo(
    () => applyDisabledPotions(inventory, disabledPotions),
    [inventory, disabledPotions],
  )

  const effectiveSubskillRanks = useMemo(
    () => boostedSubskillRanks(inventoryForCalc, subskillRanks),
    [inventoryForCalc, subskillRanks],
  )

  const grantedSkillRanks = useMemo(
    () => mercGrantedSkillRanks(mercInventory, mercDisabledAuras),
    [mercInventory, mercDisabledAuras],
  )

  return useMemo<BuildPerformanceDeps>(
    () => ({
      classId,
      level,
      allocatedAttrs,
      inventory: inventoryForCalc,
      skillRanks,
      subskillRanks: effectiveSubskillRanks,
      activeAuraId,
      activeBuffs,
      customStats,
      allocatedTreeNodes,
      treeSocketed,
      activeSkillIds,
      enemyConditions,
      playerConditions,
      skillProjectiles,
      enemyResistances,
      procToggles,
      killsPerSec,
      entityRates,
      stackCounts,
      grantedSkillRanks,
    }),
    [
      classId,
      level,
      allocatedAttrs,
      inventoryForCalc,
      skillRanks,
      effectiveSubskillRanks,
      activeAuraId,
      activeBuffs,
      customStats,
      allocatedTreeNodes,
      treeSocketed,
      activeSkillIds,
      enemyConditions,
      playerConditions,
      skillProjectiles,
      enemyResistances,
      procToggles,
      killsPerSec,
      entityRates,
      stackCounts,
      grantedSkillRanks,
    ],
  )
}
