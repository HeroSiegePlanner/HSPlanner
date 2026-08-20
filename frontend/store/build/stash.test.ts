import { beforeEach, describe, expect, it } from 'vitest'
import { items } from '@data'
import type { EquippedItem } from '../../types'
import { getSavedBuild } from '../../utils/build/savedBuilds'
import { makeSnapshot } from '../../utils/build/buildSnapshot.fixture'
import { useBuild } from './index'

function makeItem(overrides: Partial<EquippedItem> = {}): EquippedItem {
  return {
    baseId: items[0]!.id,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
    ...overrides,
  }
}

describe('build store — stash', () => {
  beforeEach(() => {
    localStorage.clear()
    useBuild.setState({
      stash: [],
      activeBuildId: null,
      activeProfileId: null,
      storageError: null,
    })
  })

  it('adds an entry with a deep copy of the item', () => {
    const item = makeItem({ stars: 3 })

    useBuild.getState().addStashItem(item)
    item.stars = 5

    const { stash } = useBuild.getState()
    expect(stash).toHaveLength(1)
    expect(stash[0]!.item.stars).toBe(3)
  })

  it('dedupes identical item snapshots', () => {
    useBuild.getState().addStashItem(makeItem({ stars: 2 }))
    useBuild.getState().addStashItem(makeItem({ stars: 2 }))

    expect(useBuild.getState().stash).toHaveLength(1)
  })

  it('keeps distinct configurations, newest first', () => {
    useBuild.getState().addStashItem(makeItem({ stars: 1 }))
    useBuild.getState().addStashItem(makeItem({ stars: 2 }))

    const { stash } = useBuild.getState()
    expect(stash).toHaveLength(2)
    expect(stash[0]!.item.stars).toBe(2)
  })

  it('removes an entry by id', () => {
    useBuild.getState().addStashItem(makeItem())
    const id = useBuild.getState().stash[0]!.id

    useBuild.getState().removeStashItem(id)

    expect(useBuild.getState().stash).toHaveLength(0)
  })

  it('caps entries at 200, dropping the oldest', () => {
    for (let i = 0; i <= 200; i++) {
      useBuild.getState().addStashItem(makeItem({ stars: i }))
    }

    const { stash } = useBuild.getState()
    expect(stash).toHaveLength(200)
    expect(stash[0]!.item.stars).toBe(200)
    expect(stash.at(-1)!.item.stars).toBe(1)
  })

  it('saveCurrentAsNewBuild persists the stash with the build', () => {
    useBuild.getState().addStashItem(makeItem({ stars: 3 }))

    const record = useBuild.getState().saveCurrentAsNewBuild('With stash')

    expect(record).not.toBeNull()
    expect(getSavedBuild(record!.id)?.stash).toHaveLength(1)
  })

  it('loadSavedBuild restores the stash saved with that build', () => {
    useBuild.getState().addStashItem(makeItem({ stars: 1 }))
    const a = useBuild.getState().saveCurrentAsNewBuild('A')!
    useBuild.getState().resetBuild()
    useBuild.getState().addStashItem(makeItem({ stars: 2 }))
    useBuild.getState().addStashItem(makeItem({ stars: 3 }))
    useBuild.getState().saveCurrentAsNewBuild('B')

    useBuild.getState().loadSavedBuild(a.id)

    expect(useBuild.getState().stash.map((e) => e.item.stars)).toEqual([1])
  })

  it('resetBuild clears the stash', () => {
    useBuild.getState().addStashItem(makeItem())

    useBuild.getState().resetBuild()

    expect(useBuild.getState().stash).toHaveLength(0)
  })

  it('importBuildSnapshot clears the stash', () => {
    useBuild.getState().addStashItem(makeItem())

    useBuild.getState().importBuildSnapshot(makeSnapshot())

    expect(useBuild.getState().stash).toHaveLength(0)
  })

  it('saveBuildNow commits stash changes to the active build', () => {
    const record = useBuild.getState().saveCurrentAsNewBuild('Live')!
    useBuild.getState().addStashItem(makeItem({ stars: 4 }))
    expect(getSavedBuild(record.id)?.stash).toHaveLength(0)

    useBuild.getState().saveBuildNow()

    expect(getSavedBuild(record.id)?.stash).toHaveLength(1)
  })
})
