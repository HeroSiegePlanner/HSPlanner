import { useEffect, useState } from 'react'
import type { Skill } from '../types'
import {
  manaCostAtRankNative,
  passiveStatsAtRankNative,
} from '../lib/calc/bridge'

export interface SkillRankInfo {
  passive: Record<string, number>
  mana: number | undefined
}

const EMPTY: Map<number, SkillRankInfo> = new Map()

// Fetches passive stats + mana cost for a set of ranks of one skill from the
// Rust calc engine (calc/passive.rs). Returns a rank -> info map; the map is
// keyed to (skill, ranks) so a stale skill's values never bleed into another's
// while the async IPC is in flight.
export function useSkillRankInfo(
  skill: Skill | null | undefined,
  ranks: number[],
): Map<number, SkillRankInfo> {
  const [state, setState] = useState<{
    key: string
    map: Map<number, SkillRankInfo>
  }>({ key: '', map: EMPTY })

  const distinct = [...new Set(ranks.filter((r) => Number.isFinite(r)))].sort(
    (a, b) => a - b,
  )
  const key = `${skill?.id ?? ''}|${distinct.join(',')}`

  useEffect(() => {
    if (!skill || distinct.length === 0) return
    let cancelled = false
    Promise.all(
      distinct.map(async (rank): Promise<[number, SkillRankInfo]> => {
        const [passive, mana] = await Promise.all([
          passiveStatsAtRankNative(skill, rank),
          manaCostAtRankNative(skill, rank),
        ])
        return [rank, { passive, mana: mana ?? undefined }]
      }),
    )
      .then((entries) => {
        if (!cancelled) setState({ key, map: new Map(entries) })
      })
      .catch(() => {
        if (!cancelled) setState({ key, map: EMPTY })
      })
    return () => {
      cancelled = true
    }
    // `key` encodes skill id + the distinct ranks it depends on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key])

  return state.key === key ? state.map : EMPTY
}
