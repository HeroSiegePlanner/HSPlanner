import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  StorageCapacityError,
  StorageWriteError,
  addProfile,
  createBuild,
  commitProfileSnapshot,
  deleteBuild,
  getSavedBuild,
  listSavedBuilds,
} from './savedBuilds'
import { makeSnapshot } from './buildSnapshot.fixture'
import {
  emptyLoadoutSlots,
  initialLoadoutIndexes,
  writeSlot,
  LOADOUT_SLOT_COUNT,
} from './loadouts'
import type { EquippedItem } from '../../types'

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

describe('savedBuilds — an oversized build is refused, never silently trimmed', () => {
  function bigItem(seed: number): EquippedItem {
    return {
      baseId: `base_${seed}`,
      affixes: Array.from({ length: 6 }, (_, k) => ({
        affixId: `affix_${seed}_${k}_${'x'.repeat(150)}`,
        tier: 5,
        roll: 0.5,
      })),
      socketCount: 6,
      socketed: Array.from({ length: 6 }, (_, k) => `socket_${seed}_${k}`),
      socketTypes: Array.from({ length: 6 }, () => 'normal' as const),
      stars: 5,
      forgedMods: [],
    }
  }

  const fatInventory = () =>
    Object.fromEntries(Array.from({ length: 30 }, (_, i) => [`slot_${i}`, bigItem(i)]))

  function oversizedSnapshot() {
    let slots = emptyLoadoutSlots()
    for (let i = 1; i < LOADOUT_SLOT_COUNT; i++) {
      slots = { ...slots, gear: writeSlot(slots.gear, i, { inventory: fatInventory() }) }
    }
    // serialize needs both halves; slots alone are skipped entirely.
    return makeSnapshot({
      inventory: fatInventory(),
      loadoutSlots: slots,
      activeLoadouts: initialLoadoutIndexes(),
    })
  }

  afterEach(() => {
    localStorage.clear()
  })

  it('createBuild refuses rather than storing a code that lost its slots', () => {
    // encodeBuildToShare would drop the parked loadouts to stay decodable, and
    // the user would only find out when the slots came back empty on reload.
    expect(() => createBuild('Too big', oversizedSnapshot())).toThrow(
      StorageCapacityError,
    )
    expect(listSavedBuilds()).toHaveLength(0)
  })

  it('commitProfileSnapshot and addProfile refuse on the same grounds', () => {
    const build = createBuild('Fine', makeSnapshot({}))
    const profileId = build.activeProfileId!

    expect(() =>
      commitProfileSnapshot(build.id, profileId, oversizedSnapshot()),
    ).toThrow(StorageCapacityError)
    expect(() => addProfile(build.id, 'Variant', oversizedSnapshot())).toThrow(
      StorageCapacityError,
    )

    // The stored build is untouched.
    expect(getSavedBuild(build.id)?.profiles).toHaveLength(1)
  })

  it('a build that fits still saves', () => {
    const build = createBuild('Normal', makeSnapshot({}))
    expect(getSavedBuild(build.id)?.profiles[0]?.code).toBeTruthy()
  })
})
