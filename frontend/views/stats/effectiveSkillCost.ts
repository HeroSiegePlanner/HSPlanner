import { rangedMax, rangedMin } from '../../utils/item/stats'
import { manaCostAtRank, splitManaCost } from '../../utils/build/manaCost'
import { skillBaseRate } from '../../utils/build/skillRate'
import type { RangedValue, Skill } from '../../types'

export interface EffectiveSkillCost {
  baseManaMin: number | undefined
  baseManaMax: number | undefined
  mcrMax: number
  baseRate: number | undefined
  speedMax: number
  effectiveManaMin: number | undefined
  effectiveManaMax: number | undefined
  lifeCostMin: number | undefined
  lifeCostMax: number | undefined
  effectiveCastRateMin: number | undefined
  effectiveCastRateMax: number | undefined
}

export function effectiveSkillCost(
  skill: Skill,
  mcrRange: RangedValue,
  speedRange: RangedValue,
  paidInLifeRange: RangedValue = 0,
  rankMin = 1,
  rankMax = rankMin,
): EffectiveSkillCost {
  const costAtMin = manaCostAtRank(skill, rankMin)
  const costAtMax = manaCostAtRank(skill, rankMax)
  const baseManaMin =
    costAtMin === undefined || costAtMax === undefined
      ? (costAtMin ?? costAtMax)
      : Math.min(costAtMin, costAtMax)
  const baseManaMax =
    costAtMin === undefined || costAtMax === undefined
      ? (costAtMin ?? costAtMax)
      : Math.max(costAtMin, costAtMax)
  const mcrMin = rangedMin(mcrRange)
  const mcrMax = rangedMax(mcrRange)
  const speedMin = rangedMin(speedRange)
  const speedMax = rangedMax(speedRange)
  // No attacks_per_second here — attack-speed skills fall back to their
  // baseCastRate (or hide the row) instead of skillBaseRate's default 0.
  const baseRate =
    skill.usesAttackSpeed === true ? skill.baseCastRate : skillBaseRate(skill)
  const costMin =
    baseManaMin !== undefined
      ? Math.max(0, baseManaMin * (1 - mcrMax / 100))
      : undefined
  const costMax =
    baseManaMax !== undefined
      ? Math.max(0, baseManaMax * (1 - mcrMin / 100))
      : undefined
  const split = splitManaCost(costMin, costMax, paidInLifeRange)
  return {
    baseManaMin,
    baseManaMax,
    mcrMax,
    baseRate,
    speedMax,
    effectiveManaMin: split.manaMin,
    effectiveManaMax: split.manaMax,
    lifeCostMin: split.lifeMin,
    lifeCostMax: split.lifeMax,
    effectiveCastRateMin:
      baseRate !== undefined ? baseRate * (1 + speedMin / 100) : undefined,
    effectiveCastRateMax:
      baseRate !== undefined ? baseRate * (1 + speedMax / 100) : undefined,
  }
}
