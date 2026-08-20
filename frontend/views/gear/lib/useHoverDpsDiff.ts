import { useMemo } from 'react'
import { useBuildPerformanceDeps } from '../../../hooks/useBuildPerformanceDeps'
import { useCalcResult } from '../../../hooks/useCalcResult'
import { computeBuildPerformanceAsync } from '../../../utils/calc/bridge'
import {
  diffPerformanceDps,
  type BuildPerformance,
  type BuildStatDiff,
} from '../../../utils/build/buildPerformance'
import type { EquippedItem } from '../../../types'

export interface HoverDpsDiff {
  diffs: BuildStatDiff[]
  activeSkillName: string | null
}

export function useHoverDpsDiff(
  slot: string,
  currentItem: EquippedItem,
  variantItem: EquippedItem | null,
  enabled = true,
): HoverDpsDiff | null {
  const deps = useBuildPerformanceDeps()
  const beforeDeps = useMemo(
    () =>
      enabled
        ? { ...deps, inventory: { ...deps.inventory, [slot]: currentItem } }
        : null,
    [deps, slot, currentItem, enabled],
  )
  const afterDeps = useMemo(
    () =>
      enabled && variantItem
        ? { ...deps, inventory: { ...deps.inventory, [slot]: variantItem } }
        : null,
    [deps, slot, variantItem, enabled],
  )
  const before = useCalcResult<BuildPerformance | null>(
    () => (beforeDeps ? computeBuildPerformanceAsync(beforeDeps) : null),
    [beforeDeps],
    null,
  )
  const after = useCalcResult<BuildPerformance | null>(
    () => (afterDeps ? computeBuildPerformanceAsync(afterDeps) : null),
    [afterDeps],
    null,
  )
  return useMemo(() => {
    if (!before || !after) return null
    return {
      diffs: diffPerformanceDps(before, after),
      activeSkillName: before.activeSkillName,
    }
  }, [before, after])
}
