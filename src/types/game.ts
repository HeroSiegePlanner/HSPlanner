export type StatKey = string
export type AttributeKey = string
export type SlotKey = string

export type StatMap = Record<StatKey, number>

export interface CustomStat {
  statKey: string
  value: string
}

export type RangedValue = number | [number, number]
export type RangedStatMap = Record<StatKey, RangedValue>

export interface AttributeDef {
  key: AttributeKey
  name: string
  description?: string
}

export interface StatDef {
  key: StatKey
  name: string
  category: 'base' | 'offense' | 'defense' | 'utility' | 'resource'
  format?: 'flat' | 'percent'
  description?: string
  modifiesAttribute?: AttributeKey | 'all'
  cap?: number
  itemOnly?: boolean
  skillScoped?: boolean
}

export interface SlotDef {
  key: SlotKey
  name: string
  group: 'weapon' | 'armor' | 'jewelry' | 'special'
}

export interface GameConfig {
  version: string
  attributes: AttributeDef[]
  stats: StatDef[]
  slots: SlotDef[]
  maxCharacterLevel: number
  etherMaxLevel: number
  attributePointsPerLevel: number
  talentPointsPerLevel: number
  skillPointsPerLevel: number
  defaultBaseStats?: StatMap
  defaultBaseAttributes?: Record<AttributeKey, number>
  defaultStatsPerAttribute?: Record<AttributeKey, StatMap>
  attributeDividedStats?: Record<AttributeKey, Record<string, number>>
}
