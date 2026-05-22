import { subskillKey } from '../../store/build'
import type { Skill, SubskillEffect } from '../../types'

type StatMap = Record<string, number>

function applyEffect(
  out: StatMap,
  effect: SubskillEffect | undefined,
  rank: number,
  multiplier = 1,
): void {
  // Adds a SubskillEffect's `base` and per-rank contributions into the supplied accumulator map, scaling the per-rank portion by `rank` and the whole result by `multiplier`. Used internally by aggregateSubskillStats to fold both deterministic and proc-weighted effects into the same kind of stat map.
  if (!effect || rank <= 0) return
  if (effect.base) {
    for (const [k, v] of Object.entries(effect.base)) {
      out[k] = (out[k] ?? 0) + v * multiplier
    }
  }
  if (effect.perRank) {
    for (const [k, v] of Object.entries(effect.perRank)) {
      out[k] = (out[k] ?? 0) + v * rank * multiplier
    }
  }
}

export interface AppliedStateInfo {
  state: string
  trigger: string
  chance: number
  amount?: number
}

export interface SubtreeAggregation {
  stats: StatMap
  procStats: StatMap
  appliedStates: AppliedStateInfo[]
}

export function aggregateSubskillStats(
  skill: Skill,
  subskillRanks: Record<string, number>,
  enemyConditions?: Record<string, boolean>,
): SubtreeAggregation {
  // Walks every subskill node on a skill, gating conditional bonuses by the supplied enemy conditions, and folds deterministic effects, proc-weighted effects, and applied-state metadata into a single SubtreeAggregation. Used by the stat pipeline and skill tooltip to compute the contribution of a skill's subtree at the player's current allocation.
  const stats: StatMap = {}
  const procStats: StatMap = {}
  const appliedStates: AppliedStateInfo[] = []

  const subskills = skill.subskills ?? []
  for (const sub of subskills) {
    const rank = subskillRanks[subskillKey(skill.id, sub.id)] ?? 0
    if (rank <= 0) continue

    if (sub.effects) {
      const gated = Object.fromEntries(
        Object.entries({
          ...(sub.effects.base ?? {}),
          ...(sub.effects.perRank ?? {}),
        }).filter(([k]) => isConditionalKey(k)),
      )

      if (Object.keys(gated).length === 0) {
        applyEffect(stats, sub.effects, rank)
      } else {
        const baseUnconditional: StatMap = {}
        const perRankUnconditional: StatMap = {}
        const baseConditional: StatMap = {}
        const perRankConditional: StatMap = {}
        for (const [k, v] of Object.entries(sub.effects.base ?? {})) {
          if (isConditionalKey(k)) baseConditional[k] = v
          else baseUnconditional[k] = v
        }
        for (const [k, v] of Object.entries(sub.effects.perRank ?? {})) {
          if (isConditionalKey(k)) perRankConditional[k] = v
          else perRankUnconditional[k] = v
        }
        applyEffect(
          stats,
          { base: baseUnconditional, perRank: perRankUnconditional },
          rank,
        )
        for (const [k, v] of Object.entries(baseConditional)) {
          if (isConditionActive(k, enemyConditions)) {
            stats[k] = (stats[k] ?? 0) + v
          }
        }
        for (const [k, v] of Object.entries(perRankConditional)) {
          if (isConditionActive(k, enemyConditions)) {
            stats[k] = (stats[k] ?? 0) + v * rank
          }
        }
      }
    }

    if (sub.proc) {
      const chance =
        (sub.proc.chance.base ?? 0) + (sub.proc.chance.perRank ?? 0) * rank
      const factor = chance / 100

      if (sub.proc.effects && factor > 0) {
        applyEffect(procStats, sub.proc.effects, rank, factor)
      }

      for (const s of sub.proc.appliesStates ?? []) {
        if (typeof s === 'string') {
          appliedStates.push({
            state: s,
            trigger: sub.proc.trigger,
            chance,
          })
        } else {
          const amount =
            (s.amount?.base ?? 0) + (s.amount?.perRank ?? 0) * rank
          appliedStates.push({
            state: s.state,
            trigger: sub.proc.trigger,
            chance,
            amount: amount || undefined,
          })
        }
      }
    }
  }

  const combined: StatMap = { ...stats }
  for (const [k, v] of Object.entries(procStats)) {
    combined[k] = (combined[k] ?? 0) + v
  }

  return { stats: combined, procStats, appliedStates }
}

function isConditionalKey(key: string): boolean {
  // Returns true when a stat key ends with one of the recognised enemy-condition suffixes (`_stasis`, `_burning`, `_low_life`, etc.). Used by aggregateSubskillStats to decide whether a stat entry must be gated behind an active enemy condition.
  return /_(stasis|slow|lightning_break|burning|poisoned|frozen|shocked|bleeding|stunned|low_life)$/.test(
    key,
  )
}

function isConditionActive(
  key: string,
  enemyConditions?: Record<string, boolean>,
): boolean {
  // Extracts the enemy-condition suffix from a stat key and looks it up in the supplied conditions map, returning true only when that condition is currently flagged active. Used by aggregateSubskillStats to apply gated stat values.
  if (!enemyConditions) return false
  const m = key.match(
    /_(stasis|slow|lightning_break|burning|poisoned|frozen|shocked|bleeding|stunned|low_life)$/,
  )
  if (!m) return false
  return !!enemyConditions[m[1]!]
}

