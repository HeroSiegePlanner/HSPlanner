import type { RangedStatMap, SlotKey, StatMap } from './game'

export type ItemRarity =
  | 'common'
  | 'uncommon'
  | 'rare'
  | 'mythic'
  | 'satanic'
  | 'heroic'
  | 'angelic'
  | 'satanic_set'
  | 'unholy'
  | 'relic'

export type ItemGrade = 'SSS' | 'SS' | 'S' | 'A' | 'B' | 'C' | 'D'

export interface ItemBase {
  id: string
  name: string
  baseType: string
  slot: SlotKey
  rarity: ItemRarity
  grade?: ItemGrade
  defenseMin?: number
  defenseMax?: number
  blockChance?: number
  damageMin?: number
  damageMax?: number
  attackSpeed?: number
  twoHanded?: boolean
  itemLevel?: number
  requiresLevel?: number
  implicit?: RangedStatMap
  sockets?: number
  maxSockets?: number
  setId?: string
  description?: string
  herobound?: boolean
  questItem?: boolean
  flavor?: string
  source?: string
  procs?: ProcEffect[]
  skillBonuses?: Record<string, RangedStatMap[string]>
  uniqueEffects?: string[]
  width?: number
  height?: number
  maxAffixes?: number
  socketTransforms?: Record<string, StatMap>
  randomAffixGroupId?: string
}

export type ProcTrigger =
  | 'on_hit'
  | 'when_struck'
  | 'on_kill'
  | 'on_cast'
  | 'on_block'
  | 'on_death'
  | 'aura'
  | 'passive'

export interface ProcEffect {
  chance?: number
  trigger: ProcTrigger
  description: string
  details?: string
}

export interface Affix {
  id: string
  groupId: string
  tier: number
  name: string
  description: string
  statKey: string | null
  sign: '+' | '-'
  format: 'flat' | 'percent'
  valueMin: number | null
  valueMax: number | null
  kind?: 'prefix' | 'suffix'
  slots?: SlotKey[]
}

export interface Rune {
  id: string
  name: string
  tier: number
  stats: StatMap
  description?: string
}

export interface Gem {
  id: string
  name: string
  tier: number
  color?: string
  stats: StatMap
  description?: string
}

export interface Runeword {
  id: string
  name: string
  runes: string[]
  allowedBaseTypes: string[]
  stats: StatMap
  requiresLevel?: number
  requiresItemLevel?: number
  description?: string
}

export interface ItemSetPiece {
  slot: string
  name: string
  itemId: string
}

export interface ItemSetBonus {
  pieces: number
  stats: StatMap
  descriptions?: string[]
}

export interface ItemSet {
  id: string
  name: string
  items: ItemSetPiece[]
  bonuses: ItemSetBonus[]
}
