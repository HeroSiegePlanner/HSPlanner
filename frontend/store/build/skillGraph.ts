import { skills as ALL_SKILLS } from '@data'
import type { Skill, SubskillNode } from '../../types'
import { subskillKey } from './helpers'

export const SKILL_BY_ID = new Map<string, Skill>(ALL_SKILLS.map((s) => [s.id, s]))

/** Prerequisite -> skills that require it, for the cascade when it drops to 0. */
export const DEPENDENT_SKILL_IDS = new Map<string, string[]>()
for (const s of ALL_SKILLS) {
  if (!s.requiresSkill) continue
  const list = DEPENDENT_SKILL_IDS.get(s.requiresSkill) ?? []
  list.push(s.id)
  DEPENDENT_SKILL_IDS.set(s.requiresSkill, list)
}

export const SUBSKILL_BY_KEY = new Map<string, SubskillNode>(
  ALL_SKILLS.flatMap((s) =>
    (s.subskills ?? []).map((sub) => [subskillKey(s.id, sub.id), sub] as const),
  ),
)

const DEPTH = new Map<string, number>()

/**
 * Steps up the `requiresSkill` chain. Sorting by it puts prerequisites before
 * their dependents, so a budget that runs out cuts leaves first.
 */
export function prereqDepth(skillId: string): number {
  const cached = DEPTH.get(skillId)
  if (cached !== undefined) return cached
  let depth = 0
  const seen = new Set<string>([skillId])
  let cur = SKILL_BY_ID.get(skillId)?.requiresSkill
  // Guarded against a cycle in the data, which would otherwise hang the app.
  while (cur && !seen.has(cur)) {
    seen.add(cur)
    depth++
    cur = SKILL_BY_ID.get(cur)?.requiresSkill
  }
  DEPTH.set(skillId, depth)
  return depth
}
