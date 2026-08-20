import { rangedMax, rangedMin } from '../item/stats'
import type { RangedValue, Skill } from '../../types'

// Mirrors the engine's mana_cost_at_rank: formula first, then the exact rank
// row, then the first rank's cost.
export function manaCostAtRank(skill: Skill, rank: number): number | undefined {
  const r = Math.max(1, rank)
  const f = skill.manaCostFormula
  if (f) return Math.floor(f.base + f.perLevel * (r - 1))
  const exact = skill.ranks.find((sr) => sr.rank === r)?.manaCost
  return exact ?? skill.ranks[0]?.manaCost
}

export interface ManaCostSplit {
  manaMin: number | undefined
  manaMax: number | undefined
  lifeMin: number | undefined
  lifeMax: number | undefined
}

// "+X% of Your Mana Costs are taken from life instead" — part of every cast is
// paid with life, so the mana number alone is not the real cost.
export function splitManaCost(
  costMin: number | undefined,
  costMax: number | undefined,
  paidInLife: RangedValue,
): ManaCostSplit {
  const pctMin = Math.min(100, Math.max(0, rangedMin(paidInLife)))
  const pctMax = Math.min(100, Math.max(0, rangedMax(paidInLife)))
  return {
    manaMin: costMin === undefined ? undefined : costMin * (1 - pctMax / 100),
    manaMax: costMax === undefined ? undefined : costMax * (1 - pctMin / 100),
    lifeMin: costMin === undefined ? undefined : costMin * (pctMin / 100),
    lifeMax: costMax === undefined ? undefined : costMax * (pctMax / 100),
  }
}
