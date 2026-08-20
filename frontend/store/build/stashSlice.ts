import type { StateCreator } from 'zustand'
import type { EquippedItem, StashEntry } from '../../types'
import { readStorage } from '../../utils/storage'
import { sanitizeStash } from '../../utils/build/savedBuilds'
import type { BuildStore } from './types'

// oldest entries are dropped past this cap to keep localStorage bounded
const MAX_ENTRIES = 200

// stash used to be global (one shared key); seed from it once so old items survive
const LEGACY_STASH_KEY = 'hsplanner.stash.v1'

function legacySeed(): StashEntry[] {
  const raw = readStorage(LEGACY_STASH_KEY)
  if (!raw) return []
  try {
    return sanitizeStash(JSON.parse(raw))
  } catch {
    return []
  }
}

type StashSlice = Pick<BuildStore, 'stash' | 'addStashItem' | 'removeStashItem'>

export const createStashSlice: StateCreator<BuildStore, [], [], StashSlice> = (
  set,
  get,
) => ({
  stash: legacySeed(),

  addStashItem: (item: EquippedItem) => {
    const snapshot = JSON.stringify(item)
    const { stash } = get()
    if (stash.some((e) => JSON.stringify(e.item) === snapshot)) return
    const entry: StashEntry = {
      id: crypto.randomUUID(),
      item: JSON.parse(snapshot) as EquippedItem,
      savedAt: Date.now(),
    }
    set({ stash: [entry, ...stash].slice(0, MAX_ENTRIES) })
  },

  removeStashItem: (id: string) =>
    set((s) => ({ stash: s.stash.filter((e) => e.id !== id) })),
})
