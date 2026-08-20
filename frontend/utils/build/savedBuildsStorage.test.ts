import { beforeEach, describe, expect, it } from 'vitest'
import {
  duplicateBuild,
  getSavedBuild,
  listSavedBuilds,
  moveBuildToFolder,
  readLibrary,
  setBuildFavorite,
  setBuildTags,
  type Folder,
  type SavedBuild,
} from './savedBuilds'
import { createFolder, deleteFolder, listFolders } from './savedFolders'

const ISO = '2026-01-01T00:00:00.000Z'

function makeBuild(id: string, over: Partial<SavedBuild> = {}): SavedBuild {
  return {
    id,
    name: `Build ${id}`,
    classId: null,
    notes: '',
    createdAt: ISO,
    updatedAt: ISO,
    profiles: [{ id: `p-${id}`, name: 'Default', code: 'CODE', updatedAt: ISO }],
    activeProfileId: `p-${id}`,
    folderId: null,
    favorite: false,
    tags: [],
    ...over,
  }
}

function makeFolder(id: string, parentId: string | null): Folder {
  return { id, name: `Folder ${id}`, parentId, createdAt: ISO }
}

function seed(builds: SavedBuild[], folders: Folder[] = []) {
  localStorage.setItem(
    'hsplanner.savedBuilds.v3',
    JSON.stringify({ version: 3, builds, folders }),
  )
}

beforeEach(() => {
  localStorage.clear()
})

describe('savedBuilds v3 — migration', () => {
  it('migrates a v2 builds array to a v3 library with default fields', () => {
    localStorage.setItem(
      'hsplanner.savedBuilds.v2',
      JSON.stringify([
        {
          id: 'b1',
          name: 'Old build',
          classId: 'mage',
          notes: 'hi',
          createdAt: ISO,
          updatedAt: ISO,
          profiles: [{ id: 'p1', name: 'Default', code: 'X', updatedAt: ISO }],
          activeProfileId: 'p1',
        },
      ]),
    )

    const builds = listSavedBuilds()
    expect(builds).toHaveLength(1)
    expect(builds[0]?.folderId).toBeNull()
    expect(builds[0]?.favorite).toBe(false)
    expect(builds[0]?.tags).toEqual([])

    const lib = readLibrary()
    expect(lib.version).toBe(3)
    expect(lib.folders).toEqual([])
    expect(lib.builds).toHaveLength(1)
  })

  it('coerces an orphan folderId to null on read', () => {
    seed([makeBuild('b1', { folderId: 'ghost-folder' })], [])
    expect(getSavedBuild('b1')?.folderId).toBeNull()
  })
})

describe('savedBuilds v3 — corrupt data', () => {
  it('backs up a corrupt v3 blob instead of silently discarding it', () => {
    const corrupt = '{"version":3,"builds":[oops not valid json'
    localStorage.setItem('hsplanner.savedBuilds.v3', corrupt)

    const lib = readLibrary()
    expect(lib.builds).toEqual([])
    expect(localStorage.getItem('hsplanner.savedBuilds.v3.corrupt')).toBe(corrupt)
  })
})

describe('savedBuilds v3 — build mutations', () => {
  it('duplicateBuild deep-clones with a fresh id, "(copy)" name and favorite reset', () => {
    seed([makeBuild('b1', { name: 'My Build', favorite: true, tags: ['hc'] })])

    const dup = duplicateBuild('b1')
    expect(dup).not.toBeNull()
    expect(dup?.id).not.toBe('b1')
    expect(dup?.name).toBe('My Build (copy)')
    expect(dup?.favorite).toBe(false)
    expect(dup?.tags).toEqual(['hc'])
    expect(dup?.profiles[0]?.id).not.toBe('p-b1')
    expect(listSavedBuilds()).toHaveLength(2)
  })

  it('setBuildFavorite toggles the flag', () => {
    seed([makeBuild('b1')])
    setBuildFavorite('b1', true)
    expect(getSavedBuild('b1')?.favorite).toBe(true)
    setBuildFavorite('b1', false)
    expect(getSavedBuild('b1')?.favorite).toBe(false)
  })

  it('setBuildTags trims, de-duplicates and drops empty tags', () => {
    seed([makeBuild('b1')])
    setBuildTags('b1', ['  HC  ', 'hc', 'ssf', '   '])
    expect(getSavedBuild('b1')?.tags).toEqual(['HC', 'ssf'])
  })

  it('moveBuildToFolder validates the target folder', () => {
    seed([makeBuild('b1')], [makeFolder('f1', null)])
    expect(moveBuildToFolder('b1', 'f1')).not.toBeNull()
    expect(getSavedBuild('b1')?.folderId).toBe('f1')
    expect(moveBuildToFolder('b1', 'nope')).toBeNull()
    expect(getSavedBuild('b1')?.folderId).toBe('f1')
    expect(moveBuildToFolder('b1', null)).not.toBeNull()
    expect(getSavedBuild('b1')?.folderId).toBeNull()
  })
})

describe('savedFolders — folder CRUD', () => {
  it('createFolder nests under an existing parent', () => {
    const parent = createFolder('Season 7', null)
    const child = createFolder('Starters', parent.id)
    expect(child.parentId).toBe(parent.id)
    expect(listFolders()).toHaveLength(2)
  })

  it('createFolder treats an unknown parent as top-level', () => {
    const folder = createFolder('Loose', 'ghost')
    expect(folder.parentId).toBeNull()
  })

  it('deleteFolder (cascade: false) reparents children and unfiles builds', () => {
    seed(
      [
        makeBuild('b1', { folderId: 'fp' }),
        makeBuild('b2', { folderId: 'fc' }),
      ],
      [makeFolder('fp', null), makeFolder('fc', 'fp')],
    )

    deleteFolder('fp', { cascade: false })

    const folders = listFolders()
    expect(folders.map((f) => f.id)).toEqual(['fc'])
    expect(folders[0]?.parentId).toBeNull()
    expect(getSavedBuild('b1')?.folderId).toBeNull()
    expect(getSavedBuild('b2')?.folderId).toBe('fc')
  })

  it('deleteFolder (cascade: true) removes the subtree and its builds', () => {
    seed(
      [
        makeBuild('b1', { folderId: 'fp' }),
        makeBuild('b2', { folderId: 'fc' }),
        makeBuild('b3', { folderId: null }),
      ],
      [makeFolder('fp', null), makeFolder('fc', 'fp')],
    )

    deleteFolder('fp', { cascade: true })

    expect(listFolders()).toEqual([])
    expect(getSavedBuild('b1')).toBeNull()
    expect(getSavedBuild('b2')).toBeNull()
    expect(getSavedBuild('b3')).not.toBeNull()
  })
})
