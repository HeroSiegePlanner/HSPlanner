import type { StateCreator } from 'zustand'
import {
  clearSlot,
  clearSlotData,
  emptyLoadout,
  emptyLoadoutSlots,
  extractLoadout,
  initialLoadoutIndexes,
  isSlotOccupied,
  isValidSlotIndex,
  renameSlot,
  switchSlot,
  writeSlot,
  type LoadoutData,
  type LoadoutSlots,
  type LoadoutSlotsMap,
  type LoadoutTab,
} from '../../utils/build/loadouts'
import type { BuildState, BuildStore } from './types'

type LoadoutsSlice = Pick<
  BuildStore,
  | 'loadoutSlots'
  | 'activeLoadouts'
  | 'switchLoadout'
  | 'clearLoadout'
  | 'renameLoadout'
  | 'duplicateLoadout'
>

/**
 * Doubles as the compile-time guard that every `LoadoutData` field still lines
 * up with its `BuildState` counterpart — if one drifts, this stops compiling.
 */
function toStatePatch(data: LoadoutData): Partial<BuildState> {
  return data
}

function setTabSlots(
  map: LoadoutSlotsMap,
  tab: LoadoutTab,
  slots: LoadoutSlots<LoadoutData>,
): LoadoutSlotsMap {
  return { ...map, [tab]: slots }
}

export const createLoadoutsSlice: StateCreator<
  BuildStore,
  [],
  [],
  LoadoutsSlice
> = (set, get) => ({
  loadoutSlots: emptyLoadoutSlots(),
  activeLoadouts: initialLoadoutIndexes(),

  switchLoadout: (tab, to) => {
    const s = get()
    const from = s.activeLoadouts[tab]
    const result = switchSlot(s.loadoutSlots[tab], from, to, extractLoadout(tab, s))
    if (!result) return false
    // An empty target slot means "this tab starts blank here", not "keep what
    // the previous slot had".
    set({
      ...toStatePatch(result.data ?? emptyLoadout(tab)),
      loadoutSlots: setTabSlots(s.loadoutSlots, tab, result.slots),
      activeLoadouts: { ...s.activeLoadouts, [tab]: to },
    })
    return true
  },

  clearLoadout: (tab, index) => {
    const s = get()
    if (!isValidSlotIndex(index)) return false
    // Clearing the active slot resets this tab's live state; the slot itself
    // already stores null, so only its label needs dropping.
    if (index === s.activeLoadouts[tab]) {
      set({
        ...toStatePatch(emptyLoadout(tab)),
        loadoutSlots: setTabSlots(
          s.loadoutSlots,
          tab,
          renameSlot(clearSlotData(s.loadoutSlots[tab], index), index, null),
        ),
      })
      return true
    }
    const slot = s.loadoutSlots[tab][index]
    // A named-but-empty slot is a real entry (a gear stage), so a bare
    // `data == null` check would refuse to delete it.
    if (slot?.data == null && slot?.name == null) return false
    set({
      loadoutSlots: setTabSlots(
        s.loadoutSlots,
        tab,
        clearSlot(s.loadoutSlots[tab], index),
      ),
    })
    return true
  },

  renameLoadout: (tab, index, name) => {
    const s = get()
    if (!isValidSlotIndex(index)) return false
    set({
      loadoutSlots: setTabSlots(
        s.loadoutSlots,
        tab,
        renameSlot(s.loadoutSlots[tab], index, name),
      ),
    })
    return true
  },

  duplicateLoadout: (tab, from, to) => {
    const s = get()
    const active = s.activeLoadouts[tab]
    if (!isValidSlotIndex(from) || !isValidSlotIndex(to) || from === to) return false
    if (!isSlotOccupied(s.loadoutSlots[tab], from, active)) return false
    // The active slot stores null, so its payload has to come from live state.
    const payload =
      from === active ? extractLoadout(tab, s) : s.loadoutSlots[tab][from]?.data
    if (!payload) return false
    // Copying onto the target overwrites it, occupied or not. Landing on the
    // active slot means overwriting live state instead of the slot, which is
    // what keeps the `data === null` invariant true.
    if (to === active) {
      set({ ...toStatePatch(payload) })
      return true
    }
    set({
      loadoutSlots: setTabSlots(
        s.loadoutSlots,
        tab,
        writeSlot(s.loadoutSlots[tab], to, payload),
      ),
    })
    return true
  },
})
