import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  StorageWriteError,
  deleteBuild,
  getSavedBuild,
  listSavedBuilds,
} from './savedBuilds'

afterEach(() => {
  vi.restoreAllMocks()
})

function failingSetItem() {
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new DOMException('quota exceeded', 'QuotaExceededError')
  })
}

describe('savedBuilds — persistence failures surface instead of being swallowed', () => {
  it('deleteBuild throws StorageWriteError when localStorage rejects the write', () => {
    failingSetItem()
    expect(() => deleteBuild('any-build-id')).toThrow(StorageWriteError)
  })

  it('listSavedBuilds still returns migrated builds when the migration write fails', () => {
    const now = new Date().toISOString()
    localStorage.setItem(
      'hsplanner.savedBuilds.v1',
      JSON.stringify([
        {
          id: 'b1',
          name: 'Legacy build',
          classId: null,
          level: 1,
          createdAt: now,
          updatedAt: now,
          code: 'abc',
        },
      ]),
    )
    failingSetItem()

    const builds = listSavedBuilds()

    expect(builds).toHaveLength(1)
    expect(builds[0]?.name).toBe('Legacy build')
  })
})

describe('savedBuilds — stash sanitization', () => {
  it('keeps valid stash entries and drops malformed ones on read', () => {
    localStorage.setItem(
      'hsplanner.savedBuilds.v3',
      JSON.stringify({
        version: 3,
        builds: [
          {
            id: 'b1',
            name: 'Stash build',
            profiles: [{ id: 'p1', name: 'Default', code: 'abc' }],
            activeProfileId: 'p1',
            stash: [
              { id: 'e1', savedAt: 1, item: { baseId: 'item_x' } },
              { id: 'e2', savedAt: 'not-a-number', item: { baseId: 'item_y' } },
              'garbage',
              { id: 'e3' },
            ],
          },
        ],
        folders: [],
      }),
    )

    const build = getSavedBuild('b1')

    expect(build).not.toBeNull()
    expect(build!.stash.map((e) => e.id)).toEqual(['e1'])
  })

  it('defaults stash to an empty array for builds saved before the field existed', () => {
    localStorage.setItem(
      'hsplanner.savedBuilds.v3',
      JSON.stringify({
        version: 3,
        builds: [
          {
            id: 'b1',
            name: 'Old build',
            profiles: [{ id: 'p1', name: 'Default', code: 'abc' }],
            activeProfileId: 'p1',
          },
        ],
        folders: [],
      }),
    )

    expect(getSavedBuild('b1')?.stash).toEqual([])
  })
})
