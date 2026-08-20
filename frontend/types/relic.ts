import type { StatMap } from './game'

export interface Relic {
  id: string
  name: string
  tier: number
  stats: StatMap
  description?: string
}

export type AugmentTrigger =
  | 'passive'
  | 'on_strike'
  | 'on_attack'
  | 'on_cast'
  | 'on_spell_hit'
  | 'on_kill'
  | 'on_pickup'
  | 'on_low_hp'
  | 'on_dodge_block'
  | 'on_hit'
  | 'periodic'
  | 'stacking'

export interface AugmentLevel {
  level: number
  stats: StatMap
  procChance?: number
  procDurationSec?: number
  cost: number
  meta?: Record<string, number>
}

export interface AngelicAugment {
  id: string
  name: string
  description: string
  trigger: AugmentTrigger
  triggerNote: string
  primaryStats: string[]
  rangedOnly?: boolean
  levels: AugmentLevel[]
}

export const AUGMENT_MAX_LEVEL = 7
