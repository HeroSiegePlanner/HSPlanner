import { beforeEach, describe, expect, it } from 'vitest'
import { makeSnapshot } from './buildSnapshot.fixture'
import {
  convertSavedBuildToSeason,
  createBuild,
  getActiveProfile,
  getSavedBuild,
  readLibrary,
  writeLibrary,
  type SavedBuild,
} from './savedBuilds'
import { decodeShareToBuild } from './shareBuild'

const GHOST_NODE_ID = 999_999

function stampSeason(build: SavedBuild, season: string): void {
  const lib = readLibrary()
  writeLibrary({
    ...lib,
    builds: lib.builds.map((b) => (b.id === build.id ? { ...b, season } : b)),
  })
}

beforeEach(() => {
  localStorage.clear()
})

describe('convertSavedBuildToSeason', () => {
  it('converts every profile, restamps season and returns the active profile report', () => {
    const build = createBuild(
      'Conv',
      makeSnapshot({ allocatedTreeNodes: new Set([GHOST_NODE_ID]) }),
    )
    stampSeason(build, 's10')

    const result = convertSavedBuildToSeason(build.id)

    expect(result).not.toBeNull()
    expect(result!.report!.removedTreeNodes).toEqual([GHOST_NODE_ID])
    const stored = getSavedBuild(build.id)
    expect(stored?.season).toBe('s9')
    // The persisted profile code must decode without the ghost node.
    const profile = stored ? getActiveProfile(stored) : null
    const decoded = profile ? decodeShareToBuild(profile.code) : null
    expect(decoded).not.toBeNull()
    expect(decoded!.snapshot.allocatedTreeNodes.has(GHOST_NODE_ID)).toBe(false)
  })

  it('returns null for already-converted or missing builds', () => {
    const build = createBuild(
      'Same',
      makeSnapshot({ allocatedTreeNodes: new Set([GHOST_NODE_ID]) }),
    )
    expect(convertSavedBuildToSeason(build.id)).toBeNull()
    expect(convertSavedBuildToSeason('missing')).toBeNull()
  })
})
