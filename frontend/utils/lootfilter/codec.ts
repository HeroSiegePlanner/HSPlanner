import type { LootFilter, LootFilterTier, LootFilterType } from '../../types'
import { ITEM_TYPES } from './constants'

const FILTER_VERSION = 2
const TIER_COUNT = 5
export const DEFAULT_RS = 0b11111100000
export const DEFAULT_SOC = 0b111111
export const DEFAULT_SOCH = 0
export const DEFAULT_WTC = 0b111111111111111111

export const FILTER_TYPE_IDS: readonly number[] = ITEM_TYPES.map((t) => t.id).sort(
  (a, b) => a - b,
)

function defaultTier(): LootFilterTier {
  return { rs: DEFAULT_RS, hidden: [], highlighted: [] }
}

function defaultType(): LootFilterType {
  return {
    tiers: Array.from({ length: TIER_COUNT }, defaultTier),
    soc: DEFAULT_SOC,
    soch: DEFAULT_SOCH,
  }
}

export function createDefaultLootFilter(): LootFilter {
  const types: Record<number, LootFilterType> = {}
  for (const id of FILTER_TYPE_IDS) types[id] = defaultType()
  return { version: FILTER_VERSION, types, wtc: DEFAULT_WTC }
}

function toInt(value: unknown, fallback: number): number {
  const n = Number(value)
  return Number.isFinite(n) ? Math.trunc(n) : fallback
}

function toIdList(value: unknown): number[] {
  if (!Array.isArray(value)) return []
  const out: number[] = []
  for (const entry of value) {
    const n = Number(entry)
    if (Number.isFinite(n) && n >= 0) out.push(Math.trunc(n))
  }
  return out
}

function parseTier(raw: unknown): LootFilterTier {
  if (!raw || typeof raw !== 'object') return defaultTier()
  const t = raw as Record<string, unknown>
  return {
    rs: toInt(t.rs, DEFAULT_RS),
    hidden: toIdList(t.hs),
    highlighted: toIdList(t.hls),
  }
}

function parseType(raw: unknown): LootFilterType {
  if (!raw || typeof raw !== 'object') return defaultType()
  const t = raw as Record<string, unknown>
  return {
    tiers: Array.from({ length: TIER_COUNT }, (_, i) =>
      `tr${i}` in t ? parseTier(t[`tr${i}`]) : defaultTier(),
    ),
    soc: toInt(t.soc, DEFAULT_SOC),
    soch: toInt(t.soch, DEFAULT_SOCH),
  }
}

export function decodeLootFilter(code: string): LootFilter | null {
  const trimmed = code.trim()
  if (!trimmed) return null
  let parsed: unknown
  try {
    parsed = JSON.parse(atob(trimmed))
  } catch {
    return null
  }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return null
  const obj = parsed as Record<string, unknown>
  const types: Record<number, LootFilterType> = {}
  for (const id of FILTER_TYPE_IDS) types[id] = defaultType()
  for (const [key, value] of Object.entries(obj)) {
    const match = /^t(\d+)$/.exec(key)
    if (match) types[Number(match[1])] = parseType(value)
  }
  return {
    version: toInt(obj.version, FILTER_VERSION),
    types,
    wtc: toInt(obj.wtc, DEFAULT_WTC),
  }
}

function serializeIds(ids: number[]): string {
  const sorted = [...ids].sort((a, b) => a - b)
  return `[${sorted
    .map((n, i) => (i === sorted.length - 1 ? `${n}.0` : String(n)))
    .join(',')}]`
}

function serializeTier(tier: LootFilterTier): string | null {
  const parts: string[] = []
  if (tier.rs !== DEFAULT_RS) parts.push(`"rs":${tier.rs}`)
  if (tier.hidden.length > 0) parts.push(`"hs":${serializeIds(tier.hidden)}`)
  if (tier.highlighted.length > 0) parts.push(`"hls":${serializeIds(tier.highlighted)}`)
  return parts.length > 0 ? `{${parts.join(',')}}` : null
}

function serializeType(type: LootFilterType): string | null {
  const parts: string[] = []
  type.tiers.forEach((tier, i) => {
    const body = serializeTier(tier)
    if (body) parts.push(`"tr${i}":${body}`)
  })
  if (type.soc !== DEFAULT_SOC) parts.push(`"soc":${type.soc}`)
  if (type.soch !== DEFAULT_SOCH) parts.push(`"soch":${type.soch}`)
  return parts.length > 0 ? `{${parts.join(',')}}` : null
}

export function encodeLootFilter(filter: LootFilter): string {
  const parts: string[] = [`"version":${filter.version}`]
  const ids = Object.keys(filter.types)
    .map(Number)
    .sort((a, b) => a - b)
  for (const id of ids) {
    const body = serializeType(filter.types[id]!)
    if (body) parts.push(`"t${id}":${body}`)
  }
  if (filter.wtc !== DEFAULT_WTC) parts.push(`"wtc":${filter.wtc}`)
  return btoa(`{${parts.join(',')}}`)
}
