import type { AttributeKey, StatMap } from './game'

export interface CharacterClass {
  id: string
  name: string
  description?: string
  role?: 'melee' | 'ranged' | 'caster' | 'hybrid'
  primaryAttribute?: AttributeKey
  baseAttributes: Record<AttributeKey, number>
  baseStats: StatMap
  statsPerLevel?: StatMap
  statsPerAttribute?: Record<AttributeKey, StatMap>
  startingSkills?: string[]
  gameClassId?: number
}
