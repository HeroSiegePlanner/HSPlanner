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
import { AUGMENT_MAX_LEVEL } from '../../types'
import { sanitizeHtml } from '../sanitizeHtml'

const SCHEMA_VERSION = 1

const DEFAULT_ENEMY_RESISTANCE_PCT = 85

export function defaultEnemyResistances(): Record<string, number> {
  // Returns a fresh map containing the default 85% enemy resistance for each elemental damage type. Used when an imported share lacks the optional `er` field and when the build store needs a clean baseline.
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
const SAFE_STRING = z.string().max(MAX_KEY_LENGTH)

const recordOfNumbers = z
  .record(SAFE_STRING, FINITE_NUMBER)
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
  })
  .passthrough()

const inventorySchema = z
  .record(SAFE_STRING, equippedItemSchema)
  .refine((r) => Object.keys(r).length <= MAX_RECORD_ENTRIES, {
    message: 'too many slots',
  })

const shareableBuildSchema = z.object({
  v: z.number(),
  c: z.string().max(MAX_KEY_LENGTH).nullable(),
  l: FINITE_NUMBER,
  a: recordOfNumbers,
  i: inventorySchema,
  s: recordOfNumbers,
  ss: recordOfNumbers,
  t: z.array(FINITE_NUMBER).max(MAX_TREE_NODES),
  m: z.string().max(MAX_KEY_LENGTH).nullable(),
  u: z.string().max(MAX_KEY_LENGTH).nullable(),
  buf: recordOfBooleans,
  ec: recordOfBooleans,
  pc: recordOfBooleans.optional(),
  sp: recordOfNumbers.optional(),
  er: recordOfNumbers.optional(),
  pt: recordOfBooleans,
  kps: FINITE_NUMBER,
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
  m: string | null
  u: string | null
  buf: Record<string, boolean>
  ec: Record<string, boolean>
  pc?: Record<string, boolean>
  sp?: Record<string, number>
  er?: Record<string, number>
  pt: Record<string, boolean>
  kps: number
  n?: string
  cs?: { k: string; v: string }[]
  ts?: Record<string, TreeSocketContent | null>
}

export interface BuildSnapshot {
  classId: string | null
  level: number
  allocated: Record<AttributeKey, number>
  inventory: Inventory
  skillRanks: Record<string, number>
  subskillRanks: Record<string, number>
  allocatedTreeNodes: Set<number>
  mainSkillId: string | null
  activeAuraId: string | null
  activeBuffs: Record<string, boolean>
  enemyConditions: Record<string, boolean>
  playerConditions: Record<string, boolean>
  skillProjectiles: Record<string, number>
  enemyResistances: Record<string, number>
  procToggles: Record<string, boolean>
  killsPerSec: number
  customStats: CustomStat[]
  treeSocketed: Record<number, TreeSocketContent | null>
}

function serialize(snapshot: BuildSnapshot, notes?: string): ShareableBuild {
  // Translates an in-memory BuildSnapshot (and optional notes) into the compact ShareableBuild wire format with short single-letter keys, sorting tree ids and dropping empty optional fields. Used internally by encodeBuildToShare before lz-string compression.
  const out: ShareableBuild = {
    v: SCHEMA_VERSION,
    c: snapshot.classId,
    l: snapshot.level,
    a: snapshot.allocated,
    i: snapshot.inventory,
    s: snapshot.skillRanks,
    ss: snapshot.subskillRanks,
    t: [...snapshot.allocatedTreeNodes].sort((x, y) => x - y),
    m: snapshot.mainSkillId,
    u: snapshot.activeAuraId,
    buf: snapshot.activeBuffs,
    ec: snapshot.enemyConditions,
    pt: snapshot.procToggles,
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
  }
  if (notes) out.n = notes
  if (snapshot.customStats.length > 0) {
    out.cs = snapshot.customStats.map((s) => ({
      k: s.statKey,
      v: s.value,
    }))
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
}

function clampLevel(n: number): number {
  // Coerces any number into a valid character level in the inclusive range [1, MAX_LEVEL], defaulting non-finite inputs to 1. Used during deserialization so a hostile share cannot push the level into a degenerate state.
  if (!Number.isFinite(n)) return 1
  return Math.max(1, Math.min(MAX_LEVEL, Math.floor(n)))
}

function deserialize(encoded: ShareableBuild): DecodedShare {
  // Converts a validated ShareableBuild back into the in-memory BuildSnapshot consumed by the build store, applying defaults for optional fields and sanitising the notes HTML. Used by decodeShareToBuild to produce the runtime shape from a parsed wire payload.
  if (encoded.v !== SCHEMA_VERSION) {
    throw new Error(
      `Unsupported share schema v${encoded.v} (expected v${SCHEMA_VERSION})`,
    )
  }
  const snapshot: BuildSnapshot = {
    classId: encoded.c ?? null,
    level: clampLevel(encoded.l ?? 1),
    allocated: encoded.a ?? {},
    inventory: normalizeInventory(encoded.i),
    skillRanks: encoded.s ?? {},
    subskillRanks: encoded.ss ?? {},
    allocatedTreeNodes: new Set(encoded.t ?? []),
    mainSkillId: encoded.m ?? null,
    activeAuraId: encoded.u ?? null,
    activeBuffs: encoded.buf ?? {},
    enemyConditions: encoded.ec ?? {},
    playerConditions: encoded.pc ?? {},
    skillProjectiles: encoded.sp ?? {},
    enemyResistances: encoded.er ?? defaultEnemyResistances(),
    procToggles: encoded.pt ?? {},
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
            .map(([id, content]) => [Number(id), content as TreeSocketContent]),
        )
      : {},
  }
  return {
    snapshot,
    notes: encoded.n ? sanitizeHtml(encoded.n) : '',
  }
}

function normalizeInventory(inv: Inventory | undefined): Inventory {
  // Reshapes a possibly-malformed Inventory into a strictly valid form: arrays are clamped/back-filled to match `socketCount`, star count is clamped to 0–5, and augment level to 1–7. Used by deserialize to repair shares produced by older or hostile clients.
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
    }
  }
  return out
}

export function encodeBuildToShare(
  snapshot: BuildSnapshot,
  notes?: string,
): string {
  // Serialises a BuildSnapshot (plus optional notes) into the compact ShareableBuild form, JSON-stringifies it, and lz-string-compresses it into a URL-safe string. Used by buildShareUrl, the saved-builds store, and the share-button flow.
  const payload = serialize(snapshot, notes)
  const json = JSON.stringify(payload)
  return compressToEncodedURIComponent(json)
}

export function decodeShareToBuild(code: string): DecodedShare | null {
  // Reverses encodeBuildToShare: lz-string-decompresses the input, validates the JSON against the zod schema, and deserialises it into a DecodedShare. Returns null on length, decode, parse, or validation failure so callers can treat any failure uniformly. Used by share-import and saved-build hydration.
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
  // Extracts the raw build code from arbitrary user input that may be either a bare code or a full share URL with a `#b=`/`?b=`/`&b=` parameter. Used by the import dialog so users can paste either form.
  const trimmed = input.trim()
  const m = trimmed.match(BUILD_CODE_RE_INPUT)
  return m && m[1] ? decodeURIComponent(m[1]) : trimmed
}

