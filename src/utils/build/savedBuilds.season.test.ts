import { beforeEach, describe, expect, it } from 'vitest'
import { makeSnapshot } from './buildSnapshot.fixture'
import {
  createBuild,
  getSavedBuild,
  readLibrary,
  writeLibrary,
} from './savedBuilds'

beforeEach(() => {
  localStorage.clear()
})

describe('saved build season field', () => {
  it('new builds are stamped with the active season', () => {
    const build = createBuild('Test', makeSnapshot())
    expect(getSavedBuild(build.id)?.season).toBe('s9')
  })

  it('legacy builds without season are stamped s9 on read', () => {
    const build = createBuild('Legacy', makeSnapshot())
    const lib = readLibrary()
    const raw = lib.builds.map((b) => {
      if (b.id !== build.id) return b
      const { season: _drop, ...rest } = b as typeof b & { season?: string }
      return rest as typeof b
    })
    writeLibrary({ ...lib, builds: raw as typeof lib.builds })
    expect(getSavedBuild(build.id)?.season).toBe('s9')
  })
})
