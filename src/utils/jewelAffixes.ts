import { affixes } from '../data'
import type { Affix } from '../types'

const SKILL_MULTIPLIER_KEYS = new Set([
  'all_skills',
  'arcane_skills',
  'cold_skills',
  'explosion_skills',
  'fire_skills',
  'lightning_skills',
  'physical_skills',
  'poison_skills',
  'summon_skills',
])

function isJewelEligibleAffix(a: Affix): boolean {
  if (!a.statKey) return false
  if (SKILL_MULTIPLIER_KEYS.has(a.statKey)) return false
  return true
}

export const JEWEL_AFFIX_POOL: Affix[] = affixes.filter(isJewelEligibleAffix)

export const JEWEL_AFFIX_POOL_BY_GROUP: Map<string, Affix[]> = (() => {
  const m = new Map<string, Affix[]>()
  for (const a of JEWEL_AFFIX_POOL) {
    if (!m.has(a.groupId)) m.set(a.groupId, [])
    m.get(a.groupId)!.push(a)
  }
  for (const list of m.values()) list.sort((x, y) => x.tier - y.tier)
  return m
})()
