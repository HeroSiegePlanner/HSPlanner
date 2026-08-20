import { useMemo, useState } from 'react'
import { rankPointOrder } from '../utils/skills/rankProgression'

export interface RankProgressionPreview {
  progressStep: number | null
  setProgressStep: (step: number | null) => void
  visibleRanks: Record<string, number>
  markerId: string | null
  isPreview: boolean
  total: number
}

export function useRankProgressionPreview(
  ranks: Record<string, number>,
  requires?: (id: string) => string | undefined,
): RankProgressionPreview {
  const [progressStep, setProgressStep] = useState<number | null>(null)

  const pointOrder = useMemo(
    () => rankPointOrder(ranks, requires),
    [ranks, requires],
  )

  const total = useMemo(
    () => Object.values(ranks).reduce((sum, rank) => sum + rank, 0),
    [ranks],
  )

  const visibleRanks = useMemo(() => {
    if (progressStep == null) return ranks
    const prefix = pointOrder.slice(0, progressStep)
    const counted: Record<string, number> = {}
    for (const id of prefix) {
      counted[id] = (counted[id] ?? 0) + 1
    }
    return counted
  }, [progressStep, ranks, pointOrder])

  const [prevRanks, setPrevRanks] = useState(ranks)
  if (prevRanks !== ranks) {
    setPrevRanks(ranks)
    if (progressStep != null) setProgressStep(null)
  }

  const markerId =
    progressStep != null ? pointOrder[progressStep - 1] ?? null : null

  return {
    progressStep,
    setProgressStep,
    visibleRanks,
    markerId,
    isPreview: progressStep != null,
    total,
  }
}
