import type { Skill, RangedValue } from '../../types'
import { manaCostAtRankNative } from '../calc/bridge'
import { rangedMax, rangedMin } from '../item/stats'
import { skillRate } from './skillRate'
import { splitManaCost } from './manaCost'
import { effectiveStatValue } from './statSectionDefs'

export interface SustainStatsInput {
  skill: Skill
  activeRank: number
  rankBonusMin: number
  rankBonusMax: number
  stats: Record<string, RangedValue>
  statsCombined: Record<string, RangedValue>
}

export interface SustainStats {
  effRankMin: number
  effRankMax: number
  effManaMin: number | undefined
  effManaMax: number | undefined
  lifePerCastMin: number | undefined
  lifePerCastMax: number | undefined
  effCastMin: number | undefined
  effCastMax: number | undefined
  manaPerSecMin: number | undefined
  manaPerSecMax: number | undefined
  manaRegenMin: number
  manaRegenMax: number
  sustainable: boolean
  unsustainable: boolean
  netMin: number | undefined
  netMax: number | undefined
  uptimeMin: number | undefined
  uptimeMax: number | undefined
}

export async function computeSustainStats(input: SustainStatsInput): Promise<SustainStats> {
  const { skill, activeRank, rankBonusMin, rankBonusMax, stats, statsCombined } = input
  const effRankMin = activeRank + rankBonusMin
  const effRankMax = activeRank + rankBonusMax

  const [baseManaMin, baseManaMax] = await Promise.all([
    manaCostAtRankNative(skill, Math.max(effRankMin, 1)),
    manaCostAtRankNative(skill, Math.max(effRankMax, 1)),
  ])

  const mcrMin = rangedMin(stats.mana_cost_reduction ?? 0)
  const mcrMax = rangedMax(stats.mana_cost_reduction ?? 0)

  const rate = skillRate(skill, (key) =>
    effectiveStatValue(stats, statsCombined, key),
  )
  const effCastMin = rate?.min
  const effCastMax = rate?.max
  const {
    manaMin: effManaMin,
    manaMax: effManaMax,
    lifeMin: lifePerCastMin,
    lifeMax: lifePerCastMax,
  } = splitManaCost(
    baseManaMin != null ? baseManaMin * (1 - mcrMax / 100) : undefined,
    baseManaMax != null ? baseManaMax * (1 - mcrMin / 100) : undefined,
    stats.mana_cost_paid_in_life ?? 0,
  )

  const manaPerSecMin =
    effManaMin !== undefined && effCastMin !== undefined ? effManaMin * effCastMin : undefined
  const manaPerSecMax =
    effManaMax !== undefined && effCastMax !== undefined ? effManaMax * effCastMax : undefined

  const manaRegenCombined = effectiveStatValue(stats, statsCombined, 'mana_replenish')
  const manaRegenMin = rangedMin(manaRegenCombined)
  const manaRegenMax = rangedMax(manaRegenCombined)

  const sustainable = manaPerSecMax !== undefined && manaPerSecMax <= manaRegenMin
  const unsustainable = manaPerSecMin !== undefined && manaPerSecMin > manaRegenMax
  const netMin = manaPerSecMax !== undefined ? manaRegenMin - manaPerSecMax : undefined
  const netMax = manaPerSecMin !== undefined ? manaRegenMax - manaPerSecMin : undefined
  const uptimeMin =
    manaPerSecMax !== undefined
      ? manaPerSecMax <= 0
        ? 100
        : Math.min(100, (manaRegenMin / manaPerSecMax) * 100)
      : undefined
  const uptimeMax =
    manaPerSecMin !== undefined
      ? manaPerSecMin <= 0
        ? 100
        : Math.min(100, (manaRegenMax / manaPerSecMin) * 100)
      : undefined

  return {
    effRankMin,
    effRankMax,
    effManaMin,
    effManaMax,
    lifePerCastMin,
    lifePerCastMax,
    effCastMin,
    effCastMax,
    manaPerSecMin,
    manaPerSecMax,
    manaRegenMin,
    manaRegenMax,
    sustainable,
    unsustainable,
    netMin,
    netMax,
    uptimeMin,
    uptimeMax,
  }
}
