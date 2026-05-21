import { useMemo } from 'react'
import { useBuild } from '../../store/build'
import { getClass } from '../../data'
import {
  getActiveProfile,
  listSavedBuilds,
  type Folder,
  type SavedBuild,
} from '../../utils/savedBuilds'
import { listFolders } from '../../utils/savedFolders'
import { decodeShareToBuild } from '../../utils/shareBuild'

export interface BuildMeta {
  /** Character level from the active profile snapshot. */
  level: number
  /** Allocated talent-tree node count from the active profile snapshot. */
  nodes: number
  /** Class display name, or "Unknown" when the class id is missing. */
  className: string
  /** False when the active profile's build code could not be decoded. */
  decoded: boolean
}

export interface BuildLibrary {
  builds: SavedBuild[]
  folders: Folder[]
  /** Per-build derived metadata, keyed by build id. */
  meta: Record<string, BuildMeta>
  /** Folder id → direct child folders. Top-level folders are under the empty-string key. */
  childFolders: Record<string, Folder[]>
}

export function useBuildLibrary(): BuildLibrary {
  // Reads the persisted build library (builds + folders) and derives per-build
  // metadata plus a parent→children folder index. Re-reads from storage on
  // every `savedBuildsVersion` bump, exactly like the old BuildsMenu.
  const version = useBuild((s) => s.savedBuildsVersion)

  return useMemo<BuildLibrary>(() => {
    void version
    const builds = listSavedBuilds()
    const folders = listFolders()

    const meta: Record<string, BuildMeta> = {}
    for (const b of builds) {
      const profile = getActiveProfile(b)
      const cls = b.classId ? getClass(b.classId) : undefined
      let level = 1
      let nodes = 0
      let decoded = false
      if (profile) {
        const share = decodeShareToBuild(profile.code)
        if (share) {
          level = share.snapshot.level
          nodes = share.snapshot.allocatedTreeNodes.size
          decoded = true
        }
      }
      meta[b.id] = { level, nodes, className: cls?.name ?? 'Unknown', decoded }
    }

    const childFolders: Record<string, Folder[]> = {}
    for (const f of folders) {
      const key = f.parentId ?? ''
      ;(childFolders[key] ??= []).push(f)
    }
    for (const key of Object.keys(childFolders)) {
      childFolders[key]!.sort((a, b) => a.name.localeCompare(b.name))
    }

    return { builds, folders, meta, childFolders }
  }, [version])
}
