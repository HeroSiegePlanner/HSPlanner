import { rangedMax, rangedMin } from '../../utils/item/stats'
import { manaCostAtRank, splitManaCost } from '../../utils/build/manaCost'
import type { RangedValue, Skill } from '../../types'

export interface EffectiveSkillCost {
  baseManaMin: number | undefined
  baseManaMax: number | undefined
  mcrMax: number
  fcrMax: number
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
  fcrRange: RangedValue,
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
  const fcrMin = rangedMin(fcrRange)
  const fcrMax = rangedMax(fcrRange)
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
    fcrMax,
    effectiveManaMin: split.manaMin,
    effectiveManaMax: split.manaMax,
    lifeCostMin: split.lifeMin,
    lifeCostMax: split.lifeMax,
    effectiveCastRateMin:
      skill.baseCastRate !== undefined
        ? skill.baseCastRate * (1 + fcrMin / 100)
        : undefined,
    effectiveCastRateMax:
      skill.baseCastRate !== undefined
        ? skill.baseCastRate * (1 + fcrMax / 100)
        : undefined,
  }
}
