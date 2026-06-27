import type { ItemRarity } from '../../types'
import { rangedMax, rangedMin } from './stats'
import { displayValuesNative } from '../../lib/calc/bridge'
import type {
  AffixValueRequest,
  DisplayValuesOutput,
  ScaledValueRequest,
} from '../../lib/calc/bridge'

export interface AffixMathProvider {
  batch(input: {
    affixes?: AffixValueRequest[]
    scaled?: ScaledValueRequest[]
  }): Promise<DisplayValuesOutput>
}

export const nativeAffixMath: AffixMathProvider = {
  batch: displayValuesNative,
}

export function toPair(v: number | [number, number]): [number, number] {
  return [rangedMin(v), rangedMax(v)]
}

export const RARITY_LABELS: Record<ItemRarity, string> = {
  common: 'COMMON',
  uncommon: 'UNCOMMON',
  rare: 'RARE',
  mythic: 'MYTHIC',
  satanic: 'SATANIC',
  heroic: 'HEROIC',
  angelic: 'ANGELIC',
  satanic_set: 'SATANIC_SET',
  unholy: 'UNHOLY',
  relic: 'RELIC',
}

export const SEP = '--------'

export function descriptionWithoutValue(description: string): string {
  return normalizeWhitespace(
    description.replace(/^[+-]?(?:\[[^\]]*]|[0-9][0-9.]*)%?\s*/, ''),
  )
}

export function normalizeWhitespace(s: string): string {
  return s.replace(/\s+/g, ' ').trim()
}
