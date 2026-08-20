import { useMemo, useState } from 'react'
import { progressionOrder } from '../utils/tree/progressionOrder'
import type { NodeAdjacency } from '../utils/tree/treeGraph'

export interface ProgressionPreview {
  progressStep: number | null
  setProgressStep: (step: number | null) => void
  visibleAllocated: Set<number>
  markerId: number | null
  isPreview: boolean
}

export function useProgressionPreview(
  allocated: Set<number>,
  startSet: ReadonlySet<number>,
  adj: NodeAdjacency,
): ProgressionPreview {
  const [progressStep, setProgressStep] = useState<number | null>(null)

  const progressionIds = useMemo(
    () => progressionOrder(allocated, startSet, adj),
    [allocated, startSet, adj],
  )

  const visibleAllocated = useMemo(
    () =>
      progressStep == null
        ? allocated
        : new Set(progressionIds.slice(0, progressStep)),
    [progressStep, allocated, progressionIds],
  )

  const [prevAllocated, setPrevAllocated] = useState(allocated)
  if (prevAllocated !== allocated) {
    setPrevAllocated(allocated)
    if (progressStep != null) setProgressStep(null)
  }

  const markerId =
    progressStep != null ? progressionIds[progressStep - 1] ?? null : null

  return {
    progressStep,
    setProgressStep,
    visibleAllocated,
    markerId,
    isPreview: progressStep != null,
  }
}
