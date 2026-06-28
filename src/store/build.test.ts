import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useBuild } from './build'
import { items } from '../data'
import type { EquippedItem } from '../types'

function makeItem(baseId: string): EquippedItem {
  return {
    baseId,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
    stars: 0,
    forgedMods: [],
  }
}

function failingSetItem() {
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new DOMException('quota exceeded', 'QuotaExceededError')
  })
}

describe('build store — storage errors are surfaced, not swallowed', () => {
  beforeEach(() => {
    useBuild.setState({
      storageError: null,
      activeBuildId: null,
      activeProfileId: null,
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
    useBuild.setState({
      storageError: null,
      activeBuildId: null,
      activeProfileId: null,
    })
  })

  it('records a storageError when a persisting action fails to write', () => {
    failingSetItem()
    expect(useBuild.getState().storageError).toBeNull()

    useBuild.getState().deleteSavedBuild('missing-build-id')

    expect(useBuild.getState().storageError).not.toBeNull()
  })

  it('saveCurrentAsNewBuild reports the failure and stays unbound when the write fails', () => {
    failingSetItem()

    const result = useBuild.getState().saveCurrentAsNewBuild('Quota Test')

    expect(result).toBeNull()
    expect(useBuild.getState().storageError).not.toBeNull()
    expect(useBuild.getState().activeBuildId).toBeNull()
  })

  it('saveCurrentAsNewBuild persists and binds the build when the write succeeds', () => {
    const result = useBuild.getState().saveCurrentAsNewBuild('Happy Path')

    expect(result).not.toBeNull()
    expect(useBuild.getState().storageError).toBeNull()
    expect(useBuild.getState().activeBuildId).not.toBeNull()
  })

  it('dismissStorageError clears a recorded error', () => {
    useBuild.setState({ storageError: 'something went wrong' })

    useBuild.getState().dismissStorageError()

    expect(useBuild.getState().storageError).toBeNull()
  })
})

describe('build store — commitEquippedItem', () => {
  beforeEach(() => {
    useBuild.setState({ inventory: {} })
  })

  it('sets the slot to the given item', () => {
    const base = items[0]
    useBuild.getState().commitEquippedItem('helm', makeItem(base.id))
    expect(useBuild.getState().inventory.helm?.baseId).toBe(base.id)
  })

  it('null unequips the slot', () => {
    const base = items[0]
    useBuild.setState({ inventory: { helm: makeItem(base.id) } })
    useBuild.getState().commitEquippedItem('helm', null)
    expect(useBuild.getState().inventory.helm).toBeUndefined()
  })

  it('committing a two-handed weapon clears the offhand', () => {
    const twoH = items.find((i) => i.twoHanded)
    const off = items.find((i) => i.slot === 'offhand' || i.baseType === 'shield')
    if (!twoH || !off) return
    useBuild.setState({ inventory: { offhand: makeItem(off.id) } })
    useBuild.getState().commitEquippedItem('weapon', makeItem(twoH.id))
    expect(useBuild.getState().inventory.offhand).toBeUndefined()
    expect(useBuild.getState().inventory.weapon?.baseId).toBe(twoH.id)
  })

  it('ignores an item with an unknown base', () => {
    useBuild.getState().commitEquippedItem('helm', makeItem('__nope__'))
    expect(useBuild.getState().inventory.helm).toBeUndefined()
  })
})
