import { invoke } from '@tauri-apps/api/core'
import type { LootFilter, LootFilterTier, LootFilterType } from '../../types'
import { ITEM_TYPES } from './constants'

const FILTER_VERSION = 2
const TIER_COUNT = 5
export const DEFAULT_RS = 0b11111100000
export const DEFAULT_SOC = 0b111111
export const DEFAULT_SOCH = 0
export const DEFAULT_WTC = 0b111111111111111111

// What engine/src/calc/lootfilter.rs emits for an untouched filter.
export const DEFAULT_LOOT_FILTER_CODE = btoa(`{"version":${FILTER_VERSION}}`)

const FILTER_TYPE_IDS: readonly number[] = ITEM_TYPES.map((t) => t.id).sort((a, b) => a - b)

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

export async function decodeLootFilter(code: string): Promise<LootFilter | null> {
  return await invoke<LootFilter | null>('lootfilter_decode', { code })
}

export async function encodeLootFilter(filter: LootFilter): Promise<string> {
  return await invoke<string>('lootfilter_encode', { filter })
}
