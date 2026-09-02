import type { LoadoutData } from '../../utils/build/loadouts'
import { skillPointsFor, subskillPointsFor } from './helpers'
import {
  DEPENDENT_SKILL_IDS,
  prereqDepth,
  SKILL_BY_ID,
  SUBSKILL_BY_KEY,
} from './skillGraph'

// An id the game no longer ships keeps its rank here and is charged against the
// budget like any other: dropping it is pruneUnknownIds' job, not the budget's.
const NO_CAP = Number.POSITIVE_INFINITY

/** Spends the budget prerequisite-first, so running out cuts leaf skills. */
function clampSkillRanks(
  ranks: Record<string, number>,
  budget: number,
): Record<string, number> {
  const ordered = Object.keys(ranks).sort(
    (a, b) => prereqDepth(a) - prereqDepth(b) || a.localeCompare(b),
  )
  const out: Record<string, number> = {}
  let left = budget
  for (const id of ordered) {
    const maxRank = SKILL_BY_ID.get(id)?.maxRank ?? NO_CAP
    const rank = Math.min(Math.floor(ranks[id] ?? 0), maxRank, left)
    if (rank <= 0) continue
    out[id] = rank
    left -= rank
  }
  // Same cascade as setSkillRank: a prerequisite at 0 takes its dependents down.
  const dropped = ordered.filter((id) => out[id] === undefined)
  while (dropped.length > 0) {
    const id = dropped.pop()!
    for (const dependent of DEPENDENT_SKILL_IDS.get(id) ?? []) {
      if (out[dependent] === undefined) continue
      delete out[dependent]
      dropped.push(dependent)
    }
  }
  return out
}

/** Subskill budgets are per parent skill, keyed `skillId:subskillId`. */
function clampSubskillRanks(
  ranks: Record<string, number>,
  budget: number,
): Record<string, number> {
  const out: Record<string, number> = {}
  const left = new Map<string, number>()
  for (const key of Object.keys(ranks).sort()) {
    const separator = key.indexOf(':')
    // Without a parent skill there is no budget to charge the rank against.
    if (separator < 0) continue
    const skillId = key.slice(0, separator)
    const maxRank = SUBSKILL_BY_KEY.get(key)?.maxRank ?? NO_CAP
    const remaining = left.get(skillId) ?? budget
    const rank = Math.min(Math.floor(ranks[key] ?? 0), maxRank, remaining)
    if (rank <= 0) continue
    out[key] = rank
    left.set(skillId, remaining - rank)
  }
  return out
}

/**
 * Clamps a stored payload to what `level` can pay for. A loadout does not carry
 * level, so a slot filled at 100 can land on a level 1 character; applied raw it
 * would show negative points available. Tree and ether nodes are exempt — hero
 * level derives from the node count instead of paying for it.
 */
export function clampLoadoutToLevel(data: LoadoutData, level: number): LoadoutData {
  if (!data.skillRanks && !data.subskillRanks) return data
  return {
    ...data,
    ...(data.skillRanks
      ? { skillRanks: clampSkillRanks(data.skillRanks, skillPointsFor(level)) }
      : {}),
    ...(data.subskillRanks
      ? { subskillRanks: clampSubskillRanks(data.subskillRanks, subskillPointsFor(level)) }
      : {}),
  }
}
