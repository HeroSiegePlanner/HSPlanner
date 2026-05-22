import { afterEach, describe, expect, it, vi } from 'vitest'
import { StorageWriteError, deleteBuild, listSavedBuilds } from './savedBuilds'

afterEach(() => {
  vi.restoreAllMocks()
})

function failingSetItem() {
  // Replaces localStorage.setItem with a stub that throws a quota-exceeded
  // DOMException, the way a real browser does once the ~5 MB origin quota is
  // hit. Used to prove that a failed persist surfaces instead of being lost.
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
