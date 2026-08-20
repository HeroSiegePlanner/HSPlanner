import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useBuild } from './index'
import { classes, incarnationNodeInfo, items } from '@data'
import { listSavedBuilds } from '../../utils/build/savedBuilds'
import { encodeBuildToShare } from '../../utils/build/shareBuild'
import type { EquippedItem } from '../../types'

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

describe('build store — dual wielding', () => {
  const wand = items.find((i) => i.baseType === 'Wand')!
  const sword = items.find((i) => i.baseType === 'Sword' && !i.twoHanded)!
  const twoHandedSword = items.find(
    (i) => i.baseType === 'Sword' && i.twoHanded,
  )!
  const masterOfWands = Number(
    Object.entries(incarnationNodeInfo).find(
      ([, info]) => info.t === 'Master of Wands',
    )![0],
  )

  beforeEach(() => {
    useBuild.setState({ inventory: {}, allocatedTreeNodes: new Set() })
  })

  it('keeps a second one-handed sword without any tree node', () => {
    useBuild.getState().commitEquippedItem('weapon', makeItem(sword.id))
    useBuild.getState().commitEquippedItem('offhand', makeItem(sword.id))
    expect(useBuild.getState().inventory.offhand?.baseId).toBe(sword.id)
  })

  it('drops the offhand weapon when a two-handed weapon goes in', () => {
    useBuild.getState().commitEquippedItem('weapon', makeItem(sword.id))
    useBuild.getState().commitEquippedItem('offhand', makeItem(sword.id))
    useBuild.getState().commitEquippedItem('weapon', makeItem(twoHandedSword.id))
    expect(useBuild.getState().inventory.offhand).toBeUndefined()
  })

  it('keeps a second wand while Master of Wands is allocated', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([masterOfWands]) })
    useBuild.getState().commitEquippedItem('weapon', makeItem(wand.id))
    useBuild.getState().commitEquippedItem('offhand', makeItem(wand.id))
    expect(useBuild.getState().inventory.offhand?.baseId).toBe(wand.id)
  })

  it('drops the offhand wand when the node is deallocated', () => {
    useBuild.setState({ allocatedTreeNodes: new Set([masterOfWands]) })
    useBuild.getState().commitEquippedItem('weapon', makeItem(wand.id))
    useBuild.getState().commitEquippedItem('offhand', makeItem(wand.id))
    useBuild.getState().resetTreeNodes()
    expect(useBuild.getState().inventory.offhand).toBeUndefined()
  })
})

describe('build store — importCodeToLibrary', () => {
  beforeEach(() => {
    useBuild.setState({
      storageError: null,
      activeBuildId: null,
      activeProfileId: null,
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
    useBuild.setState({ storageError: null })
  })

  it('saves a pasted code as a new Unfiled build without binding the planner', () => {
    const code = encodeBuildToShare(useBuild.getState().exportBuildSnapshot())
    const before = listSavedBuilds().length

    const rec = useBuild.getState().importCodeToLibrary(code)

    expect(rec).not.toBeNull()
    expect(rec!.folderId).toBeNull()
    expect(rec!.name).toMatch(/^Imported /)
    expect(listSavedBuilds().length).toBe(before + 1)
    expect(useBuild.getState().activeBuildId).toBeNull()
    expect(useBuild.getState().storageError).toBeNull()
  })

  it('returns null for an unreadable code and adds nothing', () => {
    const before = listSavedBuilds().length

    const rec = useBuild.getState().importCodeToLibrary('not-a-build-code')

    expect(rec).toBeNull()
    expect(listSavedBuilds().length).toBe(before)
  })

  it('surfaces a storage error when the write fails', () => {
    const code = encodeBuildToShare(useBuild.getState().exportBuildSnapshot())
    failingSetItem()

    const rec = useBuild.getState().importCodeToLibrary(code)

    expect(rec).toBeNull()
    expect(useBuild.getState().storageError).not.toBeNull()
  })
})

describe('build store — setClass keeps the saved-build binding', () => {
  beforeEach(() => {
    localStorage.clear()
    useBuild.getState().resetBuild()
  })

  it('changing class keeps the active build attached with notes and custom stats', () => {
    const record = useBuild.getState().saveCurrentAsNewBuild('Moja nazwa')!
    useBuild.getState().setNotes('hello')
    useBuild.getState().setCustomStats([{ statKey: 'life', value: '10' }])
    useBuild.getState().setSkillRank('some_skill', 3, 20)
    const other = classes.find((c) => c.id !== useBuild.getState().classId)!

    useBuild.getState().setClass(other.id)

    const s = useBuild.getState()
    expect(s.activeBuildId).toBe(record.id)
    expect(s.activeProfileId).toBe(record.activeProfileId)
    expect(s.notes).toBe('hello')
    expect(s.customStats).toHaveLength(1)
    expect(s.skillRanks).toEqual({})
  })
})
