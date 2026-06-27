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
import { activeSeasonId } from '../../data'
import { isKnownSeasonId, LEGACY_SEASON_ID } from '../../data/seasons/registry'
import { sanitizeHtml } from '../sanitizeHtml'

const SCHEMA_VERSION = 2

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
  kps: number
  n?: string
  cs?: { k: string; v: string }[]
  ts?: Record<string, TreeSocketContent | null>
  se?: string
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
  killsPerSec: number
  customStats: CustomStat[]
  treeSocketed: Record<number, TreeSocketContent | null>
}

function serialize(snapshot: BuildSnapshot, notes?: string): ShareableBuild {
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
    se: activeSeasonId,
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
  season: string
}

function clampLevel(n: number): number {
  if (!Number.isFinite(n)) return 1
  return Math.max(1, Math.min(MAX_LEVEL, Math.floor(n)))
}

function deserialize(encoded: ShareableBuild): DecodedShare {
  if (encoded.v !== 1 && encoded.v !== SCHEMA_VERSION) {
    throw new Error(
      `Unsupported share schema v${encoded.v} (expected v1..v${SCHEMA_VERSION})`,
    )
  }
  const season =
    encoded.v === 1
      ? LEGACY_SEASON_ID
      : encoded.se && isKnownSeasonId(encoded.se)
        ? encoded.se
        : LEGACY_SEASON_ID
  const snapshot: BuildSnapshot = {
    classId: encoded.c ?? null,
    level: clampLevel(encoded.l ?? 1),
    allocated: encoded.a ?? {},
    inventory: normalizeInventory(encoded.i),
    skillRanks: encoded.s ?? {},
    subskillRanks: encoded.ss ?? {},
    allocatedTreeNodes: new Set(encoded.t ?? []),
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
  }
  return {
    snapshot,
    notes: encoded.n ? sanitizeHtml(encoded.n) : '',
    season,
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
    }
  }
  return out
}

export function encodeBuildToShare(
  snapshot: BuildSnapshot,
  notes?: string,
): string {
  const payload = serialize(snapshot, notes)
  const json = JSON.stringify(payload)
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

