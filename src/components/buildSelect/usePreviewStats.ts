import { useEffect, useState } from 'react'
import { computeBuildPerformanceAsync } from '../../lib/calc/bridge'
import type { BuildPerformance, BuildPerformanceDeps } from '../../utils/build/buildPerformance'
import { getActiveProfile, type SavedBuild } from '../../utils/build/savedBuilds'
import { type BuildSnapshot, decodeShareToBuild } from '../../utils/build/shareBuild'

const CALC_DEBOUNCE_MS = 130

export interface PreviewStats {
  performance: BuildPerformance | null
  snapshot: BuildSnapshot | null
  loading: boolean
  available: boolean
}

const EMPTY: PreviewStats = {
  performance: null,
  snapshot: null,
  loading: false,
  available: false,
}

function snapshotToDeps(snapshot: BuildSnapshot): BuildPerformanceDeps {
  return {
    classId: snapshot.classId,
    level: snapshot.level,
    allocatedAttrs: snapshot.allocated,
    inventory: snapshot.inventory,
    skillRanks: snapshot.skillRanks,
    subskillRanks: snapshot.subskillRanks,
    activeAuraId: snapshot.activeAuraId,
    activeBuffs: snapshot.activeBuffs,
    customStats: snapshot.customStats,
    allocatedTreeNodes: snapshot.allocatedTreeNodes,
    treeSocketed: snapshot.treeSocketed,
    activeSkillIds: snapshot.activeSkillIds,
    enemyConditions: snapshot.enemyConditions,
    playerConditions: snapshot.playerConditions,
    skillProjectiles: snapshot.skillProjectiles,
    enemyResistances: snapshot.enemyResistances,
    procToggles: snapshot.procToggles,
    killsPerSec: snapshot.killsPerSec,
  }
}

export function usePreviewStats(build: SavedBuild | null): PreviewStats {
  const [state, setState] = useState<PreviewStats>(EMPTY)

  const profile = build ? getActiveProfile(build) : null
  const key = build ? `${build.id}:${profile?.code ?? ''}` : ''

  useEffect(() => {
    /* eslint-disable react-hooks/set-state-in-effect */
    if (!build || !profile) {
      setState(EMPTY)
      return
    }
    const decoded = decodeShareToBuild(profile.code)
    if (!decoded) {
      setState({ ...EMPTY })
      return
    }
    const snapshot = decoded.snapshot
    setState({ performance: null, snapshot, loading: true, available: true })
    /* eslint-enable react-hooks/set-state-in-effect */

    let cancelled = false
    const timer = window.setTimeout(() => {
      computeBuildPerformanceAsync({ ...snapshotToDeps(snapshot), season: build.season })
        .then((performance) => {
          if (cancelled) return
          setState({ performance, snapshot, loading: false, available: true })
        })
        .catch(() => {
          if (cancelled) return
          setState({ performance: null, snapshot, loading: false, available: false })
        })
    }, CALC_DEBOUNCE_MS)

    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key])

  return state
}
