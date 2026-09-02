import type { EntityRates } from './entityRates'
import {
  compressToEncodedURIComponent,
  decompressFromEncodedURIComponent,
} from 'lz-string'
import { z } from 'zod'
import type {
  AttributeKey,
  CustomStat,
  Inventory,
  SlotKey,
  SocketType,
  TreeSocketContent,
} from '../../types'
import { AUGMENT_MAX_LEVEL, SKILL_ELEMENTS } from '../../types'
import { activeSeasonId } from '@data'
import { DEFAULT_SEASON_ID, isKnownSeasonId } from '@data/seasons/registry'
import {
  dematerializeSlots,
  emptyLoadoutSlots,
  fromSparse,
  initialLoadoutIndexes,
  isValidSlotIndex,
  LOADOUT_SLOT_COUNT,
  LOADOUT_TABS,
  toSparse,
  type LoadoutData,
  type LoadoutIndexMap,
  type LoadoutSlotsMap,
  type LoadoutTab,
  type SparseSlots,
} from './loadouts'
import { clearSeasonBoundAllocations } from './seasonMigration'
import { sanitizeHtml } from '../sanitizeHtml'

const SCHEMA_VERSION = 3

const DEFAULT_ENEMY_RESISTANCE_PCT = 85

export function defaultEnemyResistances(): Record<string, number> {
  return {
    fire: DEFAULT_ENEMY_RESISTANCE_PCT,
    cold: DEFAULT_ENEMY_RESISTANCE_PCT,
    lightning: DEFAULT_ENEMY_RESISTANCE_PCT,
    poison: DEFAULT_ENEMY_RESISTANCE_PCT,
    arcane: DEFAULT_ENEMY_RESISTANCE_PCT,
  }
}
const URL_PARAM = 'b'

const BUILD_CODE_RE_INPUT = new RegExp(`[#&?]${URL_PARAM}=([^&\\s]+)`)

const MAX_LEVEL = 10_000
const MAX_KEY_LENGTH = 200
const MAX_RECORD_ENTRIES = 5_000
const MAX_TREE_NODES = 10_000
const MAX_AFFIXES_PER_ITEM = 64
const MAX_SOCKETS = 32
const MAX_NOTES_LENGTH = 200_000
const MAX_CUSTOM_STATS = 200
const MAX_SHARE_INPUT_LENGTH = 200_000

const FINITE_NUMBER = z.number().finite()
const NON_NEGATIVE_NUMBER = z.number().finite().min(0)
const SAFE_STRING = z.string().max(MAX_KEY_LENGTH)

const recordOfNumbers = z
  .record(SAFE_STRING, FINITE_NUMBER)
  .refine((r) => Object.keys(r).length <= MAX_RECORD_ENTRIES, {
    message: 'too many entries',
  })

const recordOfNonNegativeNumbers = z
  .record(SAFE_STRING, NON_NEGATIVE_NUMBER)
  .refine((r) => Object.keys(r).length <= MAX_RECORD_ENTRIES, {
    message: 'too many entries',
  })

const recordOfBooleans = z
  .record(SAFE_STRING, z.boolean())
  .refine((r) => Object.keys(r).length <= MAX_RECORD_ENTRIES, {
    message: 'too many entries',
  })

const equippedAffixSchema = z.object({
  affixId: SAFE_STRING,
  tier: FINITE_NUMBER,
  roll: FINITE_NUMBER,
  customValue: FINITE_NUMBER.optional(),
})

const treeSocketContentSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('item'), id: SAFE_STRING }),
  z.object({
    kind: z.literal('uncut'),
    affixes: z.array(equippedAffixSchema).max(MAX_AFFIXES_PER_ITEM),
  }),
])

const treeSocketedSchema = z
  .record(SAFE_STRING, treeSocketContentSchema.nullable())
  .refine((r) => Object.keys(r).length <= MAX_RECORD_ENTRIES, {
    message: 'too many tree sockets',
  })

const equippedItemSchema = z
  .object({
    baseId: SAFE_STRING,
    affixes: z.array(equippedAffixSchema).max(MAX_AFFIXES_PER_ITEM).optional(),
    socketCount: FINITE_NUMBER.optional(),
    socketed: z.array(z.string().max(MAX_KEY_LENGTH).nullable()).max(MAX_SOCKETS).optional(),
    socketTypes: z.array(SAFE_STRING).max(MAX_SOCKETS).optional(),
    runewordId: SAFE_STRING.optional(),
    stars: FINITE_NUMBER.optional(),
    forgedMods: z.array(equippedAffixSchema).max(MAX_AFFIXES_PER_ITEM).optional(),
    augment: z
      .object({ id: SAFE_STRING, level: FINITE_NUMBER })
      .optional(),
    implicitOverrides: z
      .record(SAFE_STRING, FINITE_NUMBER)
      .refine((r) => Object.keys(r).length <= MAX_RECORD_ENTRIES, {
        message: 'too many implicit overrides',
      })
      .optional(),
    skillBonusOverrides: z
      .record(SAFE_STRING, FINITE_NUMBER)
      .refine((r) => Object.keys(r).length <= MAX_RECORD_ENTRIES, {
        message: 'too many skill bonus overrides',
      })
      .optional(),
    randomSkillId: SAFE_STRING.optional(),
    randomSkillElement: z.enum(SKILL_ELEMENTS).optional(),
  })
  .passthrough()

const inventorySchema = z
  .record(SAFE_STRING, equippedItemSchema)
  .refine((r) => Object.keys(r).length <= MAX_RECORD_ENTRIES, {
    message: 'too many slots',
  })

/**
 * Loadout wire form. Payload fields reuse the top-level short keys (`t`, `ts`,
 * `et`, `s`, `ss`, `i`) so a parked loadout costs the same bytes as the live one,
 * and slots stay keyed by index — a loadout parked in slot 5 comes back in slot
 * 5 instead of being compacted into slot 2.
 */
const loadoutPayloadSchema = z.object({
  t: z.array(FINITE_NUMBER).max(MAX_TREE_NODES).optional(),
  ts: treeSocketedSchema.optional(),
  et: z.array(FINITE_NUMBER).max(MAX_TREE_NODES).optional(),
  s: recordOfNonNegativeNumbers.optional(),
  ss: recordOfNonNegativeNumbers.optional(),
  i: inventorySchema.optional(),
})

const loadoutSlotSchema = z.object({
  n: z.string().max(MAX_KEY_LENGTH).optional(),
  d: loadoutPayloadSchema.optional(),
})

const sparseSlotsSchema = z
  .record(z.string().max(4), loadoutSlotSchema)
  .refine((r) => Object.keys(r).length <= LOADOUT_SLOT_COUNT, {
    message: 'too many loadout slots',
  })

// An explicit object, not z.record(z.enum(...)): a record keyed by an enum is
// exhaustive in zod 4, and `serialize` omits tabs still sitting on slot 1.
// `.strict()` makes an unknown tab name a hard reject rather than a silent strip.
const slotIndexSchema = z.number().int().min(0).max(LOADOUT_SLOT_COUNT - 1)

const activeLoadoutsSchema = z
  .object({
    tree: slotIndexSchema.optional(),
    ether: slotIndexSchema.optional(),
    skills: slotIndexSchema.optional(),
    gear: slotIndexSchema.optional(),
  })
  .strict()

const loadoutsSchema = z.object({
  tree: sparseSlotsSchema.optional(),
  ether: sparseSlotsSchema.optional(),
  skills: sparseSlotsSchema.optional(),
  gear: sparseSlotsSchema.optional(),
  a: activeLoadoutsSchema.optional(),
})

export interface LoadoutWirePayload {
  t?: number[]
  ts?: Record<string, TreeSocketContent | null>
  et?: number[]
  s?: Record<string, number>
  ss?: Record<string, number>
  i?: Inventory
}
export type LoadoutsWire = {
  tree?: SparseSlots<LoadoutWirePayload>
  ether?: SparseSlots<LoadoutWirePayload>
  skills?: SparseSlots<LoadoutWirePayload>
  gear?: SparseSlots<LoadoutWirePayload>
  a?: Partial<Record<LoadoutTab, number>>
}

// Empty sets and records are truthy, so every field tests its content instead —
// an empty one only costs bytes. wireToPayload rebuilds the tab's full field set
// on the way back, so a slot parked while blank still decodes as a used slot.
function payloadToWire(data: LoadoutData): LoadoutWirePayload {
  const out: LoadoutWirePayload = {}
  if (data.allocatedTreeNodes?.size) {
    out.t = [...data.allocatedTreeNodes].sort((x, y) => x - y)
  }
  if (data.treeSocketed) {
    const ts: Record<string, TreeSocketContent | null> = {}
    for (const [id, content] of Object.entries(data.treeSocketed)) {
      if (content != null) ts[id] = content
    }
    if (Object.keys(ts).length > 0) out.ts = ts
  }
  if (data.allocatedEtherNodes?.size) {
    out.et = [...data.allocatedEtherNodes].sort((x, y) => x - y)
  }
  if (data.skillRanks && Object.keys(data.skillRanks).length > 0) {
    out.s = data.skillRanks
  }
  if (data.subskillRanks && Object.keys(data.subskillRanks).length > 0) {
    out.ss = data.subskillRanks
  }
  if (data.inventory && Object.keys(data.inventory).length > 0) {
    out.i = data.inventory
  }
  return out
}

/**
 * Rebuilds a payload carrying exactly the fields its tab owns, so a hostile or
 * malformed code cannot smuggle gear into a tree slot.
 */
function wireToPayload(tab: LoadoutTab, wire: LoadoutWirePayload): LoadoutData {
  switch (tab) {
    case 'tree':
      return {
        allocatedTreeNodes: new Set(wire.t ?? []),
        treeSocketed: treeSocketedFromWire(wire.ts),
      }
    case 'ether':
      return { allocatedEtherNodes: new Set(wire.et ?? []) }
    case 'skills':
      return { skillRanks: wire.s ?? {}, subskillRanks: wire.ss ?? {} }
    case 'gear':
      return { inventory: normalizeInventory(wire.i) }
  }
}

function treeSocketedFromWire(
  ts: Record<string, TreeSocketContent | null> | undefined,
): Record<number, TreeSocketContent | null> {
  if (!ts) return {}
  return Object.fromEntries(
    Object.entries(ts)
      .filter(([, v]) => v != null)
      .map(([id, content]) => [Number(id), content as TreeSocketContent] as const)
      .filter(([n]) => Number.isInteger(n) && n >= 0),
  )
}

function loadoutsToWire(
  slots: LoadoutSlotsMap,
  active: LoadoutIndexMap,
): LoadoutsWire | undefined {
  const out: LoadoutsWire = {}
  let any = false
  for (const tab of LOADOUT_TABS) {
    const sparse = toSparse(slots[tab])
    const wired: SparseSlots<LoadoutWirePayload> = {}
    for (const [index, entry] of Object.entries(sparse)) {
      wired[index] = {
        ...(entry.n ? { n: entry.n } : {}),
        ...(entry.d ? { d: payloadToWire(entry.d) } : {}),
      }
    }
    if (Object.keys(wired).length > 0) {
      out[tab] = wired
      any = true
    }
    if (active[tab] !== 0) {
      out.a = { ...out.a, [tab]: active[tab] }
      any = true
    }
  }
  return any ? out : undefined
}

function loadoutsFromWire(wire: LoadoutsWire | undefined): {
  loadoutSlots: LoadoutSlotsMap
  activeLoadouts: LoadoutIndexMap
} {
  const loadoutSlots = emptyLoadoutSlots()
  const activeLoadouts = initialLoadoutIndexes()
  if (!wire) return { loadoutSlots, activeLoadouts }
  for (const tab of LOADOUT_TABS) {
    const sparse = wire[tab]
    if (sparse) {
      const decoded: SparseSlots<LoadoutData> = {}
      for (const [index, entry] of Object.entries(sparse)) {
        decoded[index] = {
          ...(entry.n ? { n: entry.n } : {}),
          ...(entry.d ? { d: wireToPayload(tab, entry.d) } : {}),
        }
      }
      loadoutSlots[tab] = fromSparse(decoded)
    }
    const activeIndex = wire.a?.[tab]
    if (activeIndex != null && isValidSlotIndex(activeIndex)) {
      activeLoadouts[tab] = activeIndex
    }
  }
  // The active slot's payload lives in the snapshot's own fields, never in the
  // slot itself; a code that carried both loses the duplicate here.
  for (const tab of LOADOUT_TABS) {
    loadoutSlots[tab] = dematerializeSlots(loadoutSlots[tab], activeLoadouts[tab]).slots
  }
  return { loadoutSlots, activeLoadouts }
}

const shareableBuildSchema = z.object({
  v: z.number(),
  c: z.string().max(MAX_KEY_LENGTH).nullable(),
  l: NON_NEGATIVE_NUMBER,
  a: recordOfNonNegativeNumbers,
  i: inventorySchema,
  s: recordOfNonNegativeNumbers,
  ss: recordOfNonNegativeNumbers,
  t: z.array(FINITE_NUMBER).max(MAX_TREE_NODES),
  m: z
    .union([
      z.string().max(MAX_KEY_LENGTH),
      z.array(z.string().max(MAX_KEY_LENGTH)).max(64),
    ])
    .nullable(),
  u: z.string().max(MAX_KEY_LENGTH).nullable(),
  buf: recordOfBooleans,
  ec: recordOfBooleans,
  pc: recordOfBooleans.optional(),
  sp: recordOfNonNegativeNumbers.optional(),
  er: recordOfNumbers.optional(),
  pt: recordOfBooleans,
  dp: recordOfBooleans.optional(),
  kps: NON_NEGATIVE_NUMBER,
  n: z.string().max(MAX_NOTES_LENGTH).optional(),
  cs: z
    .array(
      z.object({
        k: z.string().max(MAX_KEY_LENGTH),
        v: z.string().max(MAX_KEY_LENGTH),
      }),
    )
    .max(MAX_CUSTOM_STATS)
    .optional(),
  ts: treeSocketedSchema.optional(),
  se: SAFE_STRING.optional(),
  et: z.array(FINITE_NUMBER).max(MAX_TREE_NODES).optional(),
  it: z.array(FINITE_NUMBER).max(MAX_TREE_NODES).optional(),
  mc: z.string().max(MAX_KEY_LENGTH).nullable().optional(),
  ms: recordOfNonNegativeNumbers.optional(),
  mi: inventorySchema.optional(),
  mda: recordOfBooleans.optional(),
  ld: loadoutsSchema.optional(),
})

export interface ShareableBuild {
  v: number
  c: string | null
  l: number
  a: Record<AttributeKey, number>
  i: Inventory
  s: Record<string, number>
  ss: Record<string, number>
  t: number[]
  m: string | string[] | null
  u: string | null
  buf: Record<string, boolean>
  ec: Record<string, boolean>
  pc?: Record<string, boolean>
  sp?: Record<string, number>
  er?: Record<string, number>
  pt: Record<string, boolean>
  dp?: Record<string, boolean>
  kps: number
  n?: string
  cs?: { k: string; v: string }[]
  ts?: Record<string, TreeSocketContent | null>
  se?: string
  et?: number[]
  it?: number[]
  mc?: string | null
  ms?: Record<string, number>
  mi?: Inventory
  mda?: Record<string, boolean>
  ld?: LoadoutsWire
}

export interface BuildSnapshot {
  classId: string | null
  level: number
  allocated: Record<AttributeKey, number>
  inventory: Inventory
  skillRanks: Record<string, number>
  subskillRanks: Record<string, number>
  allocatedTreeNodes: Set<number>
  activeSkillIds: string[]
  activeAuraId: string | null
  activeBuffs: Record<string, boolean>
  enemyConditions: Record<string, boolean>
  playerConditions: Record<string, boolean>
  skillProjectiles: Record<string, number>
  enemyResistances: Record<string, number>
  procToggles: Record<string, boolean>
  disabledPotions: Record<string, boolean>
  killsPerSec: number
  // Local-only Config knobs; deliberately absent from the share wire format.
  entityRates?: EntityRates
  /// Pre-split builds carried one rate for all three entity kinds.
  entityAttacksPerSecond?: number
  customStats: CustomStat[]
  treeSocketed: Record<number, TreeSocketContent | null>
  allocatedEtherNodes: Set<number>
  mercClassId: string | null
  mercSkillRanks: Record<string, number>
  mercInventory: Inventory
  mercDisabledAuras: Record<string, boolean>
  /**
   * Per-tab loadout slots. Optional so pre-loadout snapshots and fixtures stay
   * valid; the active slot's payload lives in the fields above, not here.
   */
  loadoutSlots?: LoadoutSlotsMap
  activeLoadouts?: LoadoutIndexMap
}

function serialize(
  snapshot: BuildSnapshot,
  notes: string | undefined,
  seasonId: string,
): ShareableBuild {
  const out: ShareableBuild = {
    v: SCHEMA_VERSION,
    c: snapshot.classId,
    l: snapshot.level,
    a: snapshot.allocated,
    i: snapshot.inventory,
    s: snapshot.skillRanks,
    ss: snapshot.subskillRanks,
    t: [...snapshot.allocatedTreeNodes].sort((x, y) => x - y),
    m: snapshot.activeSkillIds,
    u: snapshot.activeAuraId,
    buf: snapshot.activeBuffs,
    ec: snapshot.enemyConditions,
    pt: snapshot.procToggles,
    ...(Object.keys(snapshot.disabledPotions ?? {}).length > 0
      ? { dp: snapshot.disabledPotions }
      : {}),
    ...(Object.keys(snapshot.playerConditions ?? {}).length > 0
      ? { pc: snapshot.playerConditions }
      : {}),
    ...(Object.keys(snapshot.skillProjectiles ?? {}).length > 0
      ? { sp: snapshot.skillProjectiles }
      : {}),
    ...(Object.keys(snapshot.enemyResistances ?? {}).length > 0
      ? { er: snapshot.enemyResistances }
      : {}),
    kps: snapshot.killsPerSec,
    se: seasonId,
  }
  if (snapshot.allocatedEtherNodes.size > 0) {
    out.et = [...snapshot.allocatedEtherNodes].sort((x, y) => x - y)
  }
  if (snapshot.mercClassId) out.mc = snapshot.mercClassId
  if (Object.keys(snapshot.mercSkillRanks ?? {}).length > 0) {
    out.ms = snapshot.mercSkillRanks
  }
  if (Object.keys(snapshot.mercInventory ?? {}).length > 0) {
    out.mi = snapshot.mercInventory
  }
  if (Object.keys(snapshot.mercDisabledAuras ?? {}).length > 0) {
    out.mda = snapshot.mercDisabledAuras
  }
  if (notes) out.n = notes
  if (snapshot.customStats.length > 0) {
    out.cs = snapshot.customStats.map((s) => ({
      k: s.statKey,
      v: s.value,
    }))
  }
  if (snapshot.loadoutSlots && snapshot.activeLoadouts) {
    const ld = loadoutsToWire(snapshot.loadoutSlots, snapshot.activeLoadouts)
    if (ld) out.ld = ld
  }
  if (snapshot.treeSocketed && Object.keys(snapshot.treeSocketed).length > 0) {
    const ts: Record<string, TreeSocketContent | null> = {}
    for (const [id, content] of Object.entries(snapshot.treeSocketed)) {
      if (content == null) continue
      ts[id] = content
    }
    if (Object.keys(ts).length > 0) out.ts = ts
  }
  return out
}

export interface DecodedShare {
  snapshot: BuildSnapshot
  notes: string
  season: string
}

function clampLevel(n: number): number {
  if (!Number.isFinite(n)) return 1
  return Math.max(1, Math.min(MAX_LEVEL, Math.floor(n)))
}

function deserialize(encoded: ShareableBuild): DecodedShare {
  // Every version up to the current one decodes; new fields are optional, so a
  // v2 code simply carries no loadouts. Bumping SCHEMA_VERSION must never lock
  // out codes people already shared.
  if (
    !Number.isInteger(encoded.v) ||
    encoded.v < 1 ||
    encoded.v > SCHEMA_VERSION
  ) {
    throw new Error(
      `Unsupported share schema v${encoded.v} (expected v1..v${SCHEMA_VERSION})`,
    )
  }
  const knownSeason = encoded.se && isKnownSeasonId(encoded.se) ? encoded.se : null
  const snapshot: BuildSnapshot = {
    classId: encoded.c ?? null,
    level: clampLevel(encoded.l ?? 1),
    allocated: encoded.a ?? {},
    inventory: normalizeInventory(encoded.i),
    skillRanks: encoded.s ?? {},
    subskillRanks: encoded.ss ?? {},
    activeSkillIds: Array.isArray(encoded.m)
      ? encoded.m
      : encoded.m
        ? [encoded.m]
        : [],
    activeAuraId: encoded.u ?? null,
    activeBuffs: encoded.buf ?? {},
    enemyConditions: encoded.ec ?? {},
    playerConditions: encoded.pc ?? {},
    skillProjectiles: encoded.sp ?? {},
    enemyResistances: encoded.er ?? defaultEnemyResistances(),
    procToggles: encoded.pt ?? {},
    disabledPotions: encoded.dp ?? {},
    killsPerSec: Number.isFinite(encoded.kps) ? encoded.kps : 1,
    customStats: Array.isArray(encoded.cs)
      ? encoded.cs
          .filter((s) => s && typeof s.v === 'string')
          .map((s) => ({
            statKey: typeof s.k === 'string' ? s.k : '',
            value: s.v,
          }))
      : [],
    treeSocketed: encoded.ts
      ? Object.fromEntries(
          Object.entries(encoded.ts)
            .filter(([, v]) => v != null)
            .map(([id, content]) => {
              const n = Number(id)
              return [n, content as TreeSocketContent] as const
            })
            .filter(([n]) => Number.isInteger(n) && n >= 0),
        )
      : {},
    allocatedTreeNodes: new Set([...(encoded.t ?? []), ...(encoded.it ?? [])]),
    allocatedEtherNodes: new Set(encoded.et ?? []),
    mercClassId: encoded.mc ?? null,
    mercSkillRanks: encoded.ms ?? {},
    mercInventory: normalizeInventory(encoded.mi),
    mercDisabledAuras: encoded.mda ?? {},
    ...loadoutsFromWire(encoded.ld),
  }
  // Codes from a season we no longer ship open in the current one; tree ids
  // do not carry over, so the season-bound allocations start empty.
  return {
    snapshot: knownSeason ? snapshot : clearSeasonBoundAllocations(snapshot),
    notes: encoded.n ? sanitizeHtml(encoded.n) : '',
    season: knownSeason ?? DEFAULT_SEASON_ID,
  }
}

function normalizeInventory(inv: Inventory | undefined): Inventory {
  if (!inv) return {}
  const out: Inventory = {}
  for (const [slot, item] of Object.entries(inv)) {
    if (!item) continue
    const socketCount = item.socketCount ?? 0
    const socketed = Array.isArray(item.socketed)
      ? item.socketed.slice(0, socketCount)
      : []
    while (socketed.length < socketCount) socketed.push(null)
    const socketTypes: SocketType[] = Array.isArray(item.socketTypes)
      ? (item.socketTypes.slice(0, socketCount) as SocketType[])
      : []
    while (socketTypes.length < socketCount) socketTypes.push('normal')
    const rawStars =
      typeof item.stars === 'number' && Number.isFinite(item.stars)
        ? Math.max(0, Math.min(5, Math.floor(item.stars)))
        : 0
    const aug =
      item.augment &&
      typeof item.augment === 'object' &&
      typeof item.augment.id === 'string' &&
      Number.isFinite(item.augment.level)
        ? {
            id: item.augment.id,
            level: Math.max(1, Math.min(AUGMENT_MAX_LEVEL, Math.floor(item.augment.level))),
          }
        : undefined
    const implicitOverrides =
      item.implicitOverrides &&
      typeof item.implicitOverrides === 'object' &&
      !Array.isArray(item.implicitOverrides)
        ? item.implicitOverrides
        : undefined
    const skillBonusOverrides =
      item.skillBonusOverrides &&
      typeof item.skillBonusOverrides === 'object' &&
      !Array.isArray(item.skillBonusOverrides)
        ? item.skillBonusOverrides
        : undefined
    out[slot as SlotKey] = {
      baseId: item.baseId,
      affixes: Array.isArray(item.affixes) ? item.affixes : [],
      socketCount,
      socketed,
      socketTypes,
      runewordId: item.runewordId,
      stars: rawStars,
      forgedMods: Array.isArray(item.forgedMods) ? item.forgedMods : [],
      ...(aug ? { augment: aug } : {}),
      ...(implicitOverrides ? { implicitOverrides } : {}),
      ...(skillBonusOverrides ? { skillBonusOverrides } : {}),
      ...(typeof item.randomSkillId === 'string'
        ? { randomSkillId: item.randomSkillId }
        : {}),
      ...(item.randomSkillElement ? { randomSkillElement: item.randomSkillElement } : {}),
    }
  }
  return out
}

/** What an oversized payload had to give up. `oversize` means nothing was left to cut. */
export type ShareDegradation = 'loadouts-dropped' | 'notes-truncated' | 'oversize'

export interface EncodeShareOptions {
  /** Called once per part left out. The active build itself always survives. */
  onDegraded?: (what: ShareDegradation) => void
}

export function encodeBuildToShare(
  snapshot: BuildSnapshot,
  notes?: string,
  seasonId: string = activeSeasonId,
  options?: EncodeShareOptions,
): string {
  const payload = serialize(snapshot, notes, seasonId)
  let json = JSON.stringify(payload)
  if (json.length <= MAX_SHARE_INPUT_LENGTH) return compressToEncodedURIComponent(json)

  // `decodeShareToBuild` rejects decompressed JSON over MAX_SHARE_INPUT_LENGTH,
  // so an oversized code is unreadable rather than merely large. Shed the parts
  // the build can lose, cheapest first, and re-measure after each cut — notes
  // alone reach the limit, so dropping the loadouts is not guaranteed to help.
  let trimmed: ShareableBuild = payload
  if (trimmed.ld) {
    const activeIndexes = trimmed.ld.a
    // The active indexes stay, so the receiver still lands on the same slot.
    trimmed = { ...trimmed, ld: activeIndexes ? { a: activeIndexes } : undefined }
    json = JSON.stringify(trimmed)
    options?.onDegraded?.('loadouts-dropped')
  }
  if (json.length > MAX_SHARE_INPUT_LENGTH && trimmed.n) {
    // Dropping n characters of notes drops at least n from the JSON, so this
    // one cut is enough; escaping can only make the saving larger.
    const keep = trimmed.n.length - (json.length - MAX_SHARE_INPUT_LENGTH)
    trimmed = { ...trimmed, n: keep > 0 ? trimmed.n.slice(0, keep) : undefined }
    json = JSON.stringify(trimmed)
    options?.onDegraded?.('notes-truncated')
  }
  if (json.length > MAX_SHARE_INPUT_LENGTH) options?.onDegraded?.('oversize')
  return compressToEncodedURIComponent(json)
}

export function decodeShareToBuild(code: string): DecodedShare | null {
  try {
    if (typeof code !== 'string' || code.length > MAX_SHARE_INPUT_LENGTH) {
      return null
    }
    const json = decompressFromEncodedURIComponent(code)
    if (!json || json.length > MAX_SHARE_INPUT_LENGTH) return null
    const parsed: unknown = JSON.parse(json)
    const result = shareableBuildSchema.safeParse(parsed)
    if (!result.success) return null
    return deserialize(result.data as ShareableBuild)
  } catch {
    return null
  }
}

export function parseBuildCodeFromInput(input: string): string {
  const trimmed = input.trim()
  const m = trimmed.match(BUILD_CODE_RE_INPUT)
  return m && m[1] ? decodeURIComponent(m[1]) : trimmed
}

