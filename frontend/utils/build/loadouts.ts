import type { Inventory, TreeSocketContent } from '../../types'

/**
 * Per-tab loadout slots — the in-game profile row (1..8) scoped to one tab.
 * `SavedProfile` in ./savedBuilds is the whole-build equivalent.
 *
 * Invariant: `slots[activeIndex].data` is always `null`, on the wire and in
 * storage too. The live store holds the active loadout, so only one copy of it
 * exists anywhere and live/stored divergence is impossible by construction.
 */

export const LOADOUT_SLOT_COUNT = 8

export const LOADOUT_TABS = ['tree', 'ether', 'skills', 'gear'] as const
export type LoadoutTab = (typeof LOADOUT_TABS)[number]

export const MAX_LOADOUT_NAME_LENGTH = 40

/**
 * Every field a loadout can carry, as one optional-field type so slots, slice
 * and wire format stay generic over a single payload. `LOADOUT_FIELDS` maps
 * fields to tabs; `loadoutsSlice.toStatePatch` asserts at compile time that
 * these stay assignable to `BuildState`.
 */
export interface LoadoutData {
  allocatedTreeNodes?: Set<number>
  treeSocketed?: Record<number, TreeSocketContent | null>
  allocatedEtherNodes?: Set<number>
  skillRanks?: Record<string, number>
  subskillRanks?: Record<string, number>
  inventory?: Inventory
}

export type LoadoutField = keyof LoadoutData

/** Single source of truth for the per-tab slice of build state. */
export const LOADOUT_FIELDS = {
  tree: ['allocatedTreeNodes', 'treeSocketed'],
  ether: ['allocatedEtherNodes'],
  skills: ['skillRanks', 'subskillRanks'],
  gear: ['inventory'],
} as const satisfies Record<LoadoutTab, readonly LoadoutField[]>

/** Cleared value for every field, used when switching into an empty slot. */
const EMPTY_FIELD_VALUE: { [K in LoadoutField]-?: () => LoadoutData[K] } = {
  allocatedTreeNodes: () => new Set<number>(),
  treeSocketed: () => ({}),
  allocatedEtherNodes: () => new Set<number>(),
  skillRanks: () => ({}),
  subskillRanks: () => ({}),
  inventory: () => ({}),
}

/** Reads the fields owned by `tab` out of any object carrying build state. */
export function extractLoadout(tab: LoadoutTab, source: LoadoutData): LoadoutData {
  const out: LoadoutData = {}
  for (const field of LOADOUT_FIELDS[tab]) {
    assignField(out, field, source[field])
  }
  return out
}

/** The cleared payload for `tab` — what an empty slot applies to live state. */
export function emptyLoadout(tab: LoadoutTab): LoadoutData {
  const out: LoadoutData = {}
  for (const field of LOADOUT_FIELDS[tab]) {
    assignField(out, field, EMPTY_FIELD_VALUE[field]())
  }
  return out
}

function assignField<K extends LoadoutField>(
  target: LoadoutData,
  field: K,
  value: LoadoutData[K],
): void {
  target[field] = value
}

export interface LoadoutSlot<T> {
  /** User label; `null` falls back to the slot number in the UI. */
  name: string | null
  /** `null` until the slot is used, and on the active slot — its payload is live state. */
  data: T | null
}

export type LoadoutSlots<T> = readonly LoadoutSlot<T>[]

export type LoadoutSlotsMap = Record<LoadoutTab, LoadoutSlots<LoadoutData>>

export type LoadoutIndexMap = Record<LoadoutTab, number>

const EMPTY_SLOT: LoadoutSlot<never> = { name: null, data: null }

export function emptySlots<T>(): LoadoutSlots<T> {
  return Array.from({ length: LOADOUT_SLOT_COUNT }, () => EMPTY_SLOT as LoadoutSlot<T>)
}

export function emptyLoadoutSlots(): LoadoutSlotsMap {
  return {
    tree: emptySlots<LoadoutData>(),
    ether: emptySlots<LoadoutData>(),
    skills: emptySlots<LoadoutData>(),
    gear: emptySlots<LoadoutData>(),
  }
}

export function initialLoadoutIndexes(): LoadoutIndexMap {
  return { tree: 0, ether: 0, skills: 0, gear: 0 }
}

export function isValidSlotIndex(index: number): boolean {
  return Number.isInteger(index) && index >= 0 && index < LOADOUT_SLOT_COUNT
}

/**
 * Holds data, carries a label, or is the active slot. A label alone counts, so
 * the gear tab can create a named-but-empty stage.
 */
export function isSlotOccupied<T>(
  slots: LoadoutSlots<T>,
  index: number,
  activeIndex: number,
): boolean {
  if (!isValidSlotIndex(index)) return false
  if (index === activeIndex) return true
  const slot = slots[index]
  return slot?.data != null || slot?.name != null
}

export function occupiedCount<T>(
  slots: LoadoutSlots<T>,
  activeIndex: number,
): number {
  let n = 0
  for (let i = 0; i < LOADOUT_SLOT_COUNT; i++) {
    if (isSlotOccupied(slots, i, activeIndex)) n++
  }
  return n
}

export function slotLabel<T>(slots: LoadoutSlots<T>, index: number): string {
  return slots[index]?.name?.trim() || String(index + 1)
}

function replaceSlot<T>(
  slots: LoadoutSlots<T>,
  index: number,
  next: LoadoutSlot<T>,
): LoadoutSlots<T> {
  if (!isValidSlotIndex(index)) return slots
  return slots.map((slot, i) => (i === index ? next : slot))
}

export function writeSlot<T>(
  slots: LoadoutSlots<T>,
  index: number,
  data: T,
): LoadoutSlots<T> {
  return replaceSlot(slots, index, { name: slots[index]?.name ?? null, data })
}

export function clearSlotData<T>(
  slots: LoadoutSlots<T>,
  index: number,
): LoadoutSlots<T> {
  return replaceSlot(slots, index, { name: slots[index]?.name ?? null, data: null })
}

/** Frees a slot entirely — content and label. */
export function clearSlot<T>(
  slots: LoadoutSlots<T>,
  index: number,
): LoadoutSlots<T> {
  return replaceSlot(slots, index, EMPTY_SLOT as LoadoutSlot<T>)
}

export function renameSlot<T>(
  slots: LoadoutSlots<T>,
  index: number,
  name: string | null,
): LoadoutSlots<T> {
  const trimmed = name?.trim().slice(0, MAX_LOADOUT_NAME_LENGTH) ?? ''
  return replaceSlot(slots, index, {
    name: trimmed === '' ? null : trimmed,
    data: slots[index]?.data ?? null,
  })
}

/** `data` is `null` for a never-used target slot: apply the tab's cleared state. */
export interface SwitchResult<T> {
  slots: LoadoutSlots<T>
  data: T | null
}

// Parking a blank tab stores its cleared payload, not `null`: the slot the user
// was sitting in stays a real entry (a gear stage they had not filled yet).
export function switchSlot<T>(
  slots: LoadoutSlots<T>,
  from: number,
  to: number,
  livePayload: T,
): SwitchResult<T> | null {
  if (!isValidSlotIndex(from) || !isValidSlotIndex(to) || from === to) return null
  const parked = writeSlot(slots, from, livePayload)
  return { slots: clearSlotData(parked, to), data: parked[to]?.data ?? null }
}

/** Restores the `null` invariant on decoded slots; a hand-edited code may break it. */
export function dematerializeSlots<T>(
  slots: LoadoutSlots<T>,
  activeIndex: number,
): SwitchResult<T> {
  if (!isValidSlotIndex(activeIndex)) return { slots, data: null }
  return {
    slots: clearSlotData(slots, activeIndex),
    data: slots[activeIndex]?.data ?? null,
  }
}

/**
 * Maps stored payloads, leaving labels and the active slot's `null` alone.
 * Migrations must run through this or stale content survives in parked slots.
 */
export function mapSlotData<T>(
  slots: LoadoutSlots<T>,
  fn: (data: T) => T,
): LoadoutSlots<T> {
  return slots.map((slot) =>
    slot.data == null ? slot : { name: slot.name, data: fn(slot.data) },
  )
}

export function mapAllSlotData(
  map: LoadoutSlotsMap,
  fn: (data: LoadoutData, tab: LoadoutTab) => LoadoutData,
): LoadoutSlotsMap {
  return {
    tree: mapSlotData(map.tree, (d) => fn(d, 'tree')),
    ether: mapSlotData(map.ether, (d) => fn(d, 'ether')),
    skills: mapSlotData(map.skills, (d) => fn(d, 'skills')),
    gear: mapSlotData(map.gear, (d) => fn(d, 'gear')),
  }
}

/** Sparse wire form keyed by slot index, so a loadout parked in slot 5 returns in slot 5. */
export type SparseSlots<T> = Record<string, { n?: string; d?: T }>

export function toSparse<T>(slots: LoadoutSlots<T>): SparseSlots<T> {
  const out: SparseSlots<T> = {}
  for (let i = 0; i < LOADOUT_SLOT_COUNT; i++) {
    const slot = slots[i]
    if (!slot) continue
    // A label with no payload still travels: it is a named stage the user
    // created but has not filled yet.
    if (slot.data == null && slot.name == null) continue
    out[String(i)] = {
      ...(slot.name ? { n: slot.name } : {}),
      ...(slot.data != null ? { d: slot.data } : {}),
    }
  }
  return out
}

export function fromSparse<T>(sparse: SparseSlots<T> | undefined): LoadoutSlots<T> {
  const out: LoadoutSlot<T>[] = Array.from(
    { length: LOADOUT_SLOT_COUNT },
    () => EMPTY_SLOT as LoadoutSlot<T>,
  )
  if (!sparse) return out
  for (const [key, entry] of Object.entries(sparse)) {
    const index = Number(key)
    if (!isValidSlotIndex(index) || !entry) continue
    out[index] = {
      name: entry.n?.trim().slice(0, MAX_LOADOUT_NAME_LENGTH) || null,
      data: entry.d ?? null,
    }
  }
  return out
}
