import type { Inventory, TreeSocketContent } from '../../types'

/**
 * Per-tab loadout slots, mirroring the in-game profile row (slots 1..8 at the
 * top-right of each panel). Distinct from `SavedProfile` in ./savedBuilds,
 * which is a variant of the *whole* build; a loadout only covers the slice of
 * state its own tab owns.
 *
 * Core invariant: the live store is the single source of truth for the active
 * slot, so `slots[activeIndex].data` is always `null`. There is exactly one
 * copy of every loadout, which makes divergence between the live state and a
 * stored copy impossible. The invariant holds on the wire and in storage too —
 * the active slot's content is already carried by the snapshot's own fields, so
 * duplicating it into the slot would only cost bytes. `dematerializeSlots`
 * restores the invariant on anything decoded from outside.
 */

export const LOADOUT_SLOT_COUNT = 8

export const LOADOUT_TABS = ['tree', 'ether', 'skills', 'gear'] as const
export type LoadoutTab = (typeof LOADOUT_TABS)[number]

export const MAX_LOADOUT_NAME_LENGTH = 40

/**
 * The union of every build-state field a loadout can carry. Kept as one
 * optional-field type rather than four per-tab types so the slots, the store
 * slice and the wire format are all generic over a single payload — no casts
 * anywhere. Which fields actually belong to which tab is declared once, in
 * `LOADOUT_FIELDS`.
 *
 * These field names and types must stay assignable to the matching `BuildState`
 * fields; `loadoutsSlice` asserts that at compile time.
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
  /** `null` means empty — or the active slot, which lives in the store. */
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
 * A slot is in use when it holds data, carries a deliberate label, or is the
 * active one — the active slot reads as occupied even though its `data` is
 * `null`, because its content is the live store state. Counting a label alone
 * is what lets the gear tab create a named-but-still-empty stage ("Early") and
 * have it show up as a real entry.
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

/**
 * Switches the active slot: the live payload is parked in `from` and the
 * payload stored at `to` is handed back for the store to apply. `data` is
 * `null` when the target slot is empty, meaning the caller should apply that
 * tab's cleared state.
 */
export interface SwitchResult<T> {
  slots: LoadoutSlots<T>
  data: T | null
}

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

/**
 * Restores the `null` invariant on decoded slots, handing back whatever payload
 * the active slot carried so the caller can decide what to do with it. Codes
 * written by this app leave the active slot empty — its content travels in the
 * snapshot's own fields — but a hand-edited one may not.
 */
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
 * Transforms every stored payload, leaving labels and the active slot's `null`
 * alone. Migrations (season change, pruning ids the game dropped) must run
 * through this, or stale content survives in the inactive slots.
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

/**
 * Sparse wire form: only non-empty slots travel, keyed by slot index so a
 * loadout parked in slot 5 comes back in slot 5 rather than being compacted
 * into slot 2.
 */
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
