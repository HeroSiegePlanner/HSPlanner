import { getItem } from '@data'
import type { EquippedItem, Inventory } from '../../types'

export const SUBSKILL_BOOST_KEY = 'grant_subskills'

function boostAmount(item: EquippedItem): number {
  const override = item.implicitOverrides?.[SUBSKILL_BOOST_KEY]
  if (override !== undefined) return Math.floor(override)
  const raw = getItem(item.baseId)?.implicit?.[SUBSKILL_BOOST_KEY]
  if (raw === undefined) return 0
  return Math.floor(Array.isArray(raw) ? raw[1] : raw)
}

/** Skill id -> total "+X to Random Skill Sub Skills" from equipped items. */
export function subskillBoostBySkill(inventory: Inventory): Map<string, number> {
  const out = new Map<string, number>()
  for (const item of Object.values(inventory)) {
    const skillId = item?.subskillBoostSkillId
    if (!item || !skillId) continue
    const amount = boostAmount(item)
    if (amount <= 0) continue
    out.set(skillId, (out.get(skillId) ?? 0) + amount)
  }
  return out
}

/** The implicit raises ranks already spent — an untouched node stays at zero. */
export function boostedSubskillRanks(
  inventory: Inventory,
  ranks: Record<string, number>,
): Record<string, number> {
  const boosts = subskillBoostBySkill(inventory)
  if (boosts.size === 0) return ranks
  const out = { ...ranks }
  for (const [skillId, boost] of boosts) {
    for (const [key, rank] of Object.entries(ranks)) {
      if (rank > 0 && key.startsWith(`${skillId}:`)) out[key] = rank + boost
    }
  }
  return out
}
