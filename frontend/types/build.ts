import type { AttributeKey, SlotKey } from './game'

export type SocketType = 'normal' | 'rainbow'

export interface EquippedAffix {
  affixId: string
  tier: number
  roll: number
  customValue?: number
}

export interface TreeSocketEquipped {
  kind: 'item'
  id: string
}

export interface TreeSocketCrafted {
  kind: 'uncut'
  affixes: EquippedAffix[]
}

export type TreeSocketContent = TreeSocketEquipped | TreeSocketCrafted

export const UNCUT_JEWEL_MAX_AFFIXES = 4

export const SKILL_ELEMENTS = ['fire', 'cold', 'lightning', 'poison', 'arcane'] as const
export type SkillElement = (typeof SKILL_ELEMENTS)[number]

export interface EquippedItem {
  baseId: string
  affixes: EquippedAffix[]
  socketCount: number
  socketed: (string | null)[]
  socketTypes: SocketType[]
  runewordId?: string
  stars?: number
  forgedMods?: EquippedAffix[]
  augment?: { id: string; level: number }
  implicitOverrides?: Record<string, number>
  /** Pinned totals for item-granted skill ranks, keyed by base skillBonuses name. */
  skillBonusOverrides?: Record<string, number>
  randomSkillId?: string
  randomSkillElement?: SkillElement
  subskillBoostSkillId?: string
}

export type Inventory = Partial<Record<SlotKey, EquippedItem>>

export interface StashEntry {
  id: string
  item: EquippedItem
  savedAt: number
}

export interface Build {
  id: string
  name: string
  classId: string
  level: number
  attributePoints: Record<AttributeKey, number>
  skillRanks: Record<string, number>
  talentRanks: Record<string, number>
  inventory: Inventory
  relicIds: string[]
  augmentIds: string[]
  notes?: string
  createdAt: string
  updatedAt: string
}
