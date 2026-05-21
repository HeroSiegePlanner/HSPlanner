import { useEffect, useState } from 'react'
import { computeBuildPerformanceAsync } from '../../lib/calc/bridge'
import type { BuildPerformance, BuildPerformanceDeps } from '../../utils/buildPerformance'
import { getActiveProfile, type SavedBuild } from '../../utils/savedBuilds'
import { type BuildSnapshot, decodeShareToBuild } from '../../utils/shareBuild'

/** Debounce before the (potentially expensive) Rust calc fires, so arrow-key
 * navigation through the build table does not spawn a calc per row. */
const CALC_DEBOUNCE_MS = 130

export interface PreviewStats {
  /** Computed performance for the selected build, or null while loading / unavailable. */
  performance: BuildPerformance | null
  /** Decoded snapshot of the build's active profile, or null when the code is corrupt. */
  snapshot: BuildSnapshot | null
  /** True while the async calc is in flight. */
  loading: boolean
  /** False when the build code could not be decoded OR the Rust calc is unavailable
   *  (e.g. running in a plain browser without Tauri). The preview then falls back to
   *  snapshot-only metadata. */
  available: boolean
}

const EMPTY: PreviewStats = {
  performance: null,
  snapshot: null,
  loading: false,
  available: false,
}

function snapshotToDeps(snapshot: BuildSnapshot): BuildPerformanceDeps {
  // Maps a decoded BuildSnapshot onto the BuildPerformanceDeps shape the calc
  // bridge expects. Fields align 1:1 except `allocated` → `allocatedAttrs`.
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
    mainSkillId: snapshot.mainSkillId,
    enemyConditions: snapshot.enemyConditions,
    playerConditions: snapshot.playerConditions,
    skillProjectiles: snapshot.skillProjectiles,
    enemyResistances: snapshot.enemyResistances,
    procToggles: snapshot.procToggles,
    killsPerSec: snapshot.killsPerSec,
  }
}

export function usePreviewStats(build: SavedBuild | null): PreviewStats {
  // Decodes the active profile of `build` and runs the Rust calc to produce a
  // live stats preview. Debounced and cancellation-guarded so rapid selection
  // changes never leave a stale result on screen. Degrades gracefully to
  // snapshot-only metadata when decoding fails or the calc is unavailable.
  const [state, setState] = useState<PreviewStats>(EMPTY)

  // Re-run only when the selected build (or its active profile's code) changes,
  // not on every unrelated library mutation.
  const profile = build ? getActiveProfile(build) : null
  const key = build ? `${build.id}:${profile?.code ?? ''}` : ''

  useEffect(() => {
    // One-shot sync to the selected build — not a render loop. The disabled
    // rule fires because the resets run synchronously on key change.
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
      computeBuildPerformanceAsync(snapshotToDeps(snapshot))
        .then((performance) => {
          if (cancelled) return
          setState({ performance, snapshot, loading: false, available: true })
        })
        .catch(() => {
          if (cancelled) return
          // No Tauri / calc failed — keep the snapshot so the preview still
          // shows class/level/tags/notes, just without computed numbers.
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
