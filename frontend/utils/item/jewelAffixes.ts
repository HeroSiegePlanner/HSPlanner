import { affixPools, affixes } from '@data'
import type { Affix } from '../../types'

// Uncut jewels roll the game's "Socketable" pool: the same affix families as
// gear, but with their own far lower ranges (data/affix-pools.json).
export function isJewelAffix(a: Affix): boolean {
  return affixPools[a.groupId]?.includes('Socketable') ?? false
}

export const JEWEL_AFFIX_POOL: Affix[] = affixes.filter(
  (a) => isJewelAffix(a) && !!a.statKey,
)

export const JEWEL_AFFIX_POOL_BY_GROUP: Map<string, Affix[]> = (() => {
  const m = new Map<string, Affix[]>()
  for (const a of JEWEL_AFFIX_POOL) {
    if (!m.has(a.groupId)) m.set(a.groupId, [])
    m.get(a.groupId)!.push(a)
  }
  for (const list of m.values()) list.sort((x, y) => x.tier - y.tier)
  return m
})()
