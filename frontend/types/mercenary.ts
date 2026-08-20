import type { SlotKey } from './game'

export type MercSkillKind = 'active' | 'passive'

export interface MercSkill {
  id: string
  name: string
  kind: MercSkillKind
  damageType: string | null
  shared: boolean
  description: string
}

export interface MercClass {
  id: string
  name: string
  role: string
  location: string
  skills: MercSkill[]
}

export interface MercData {
  maxSkillRank: number
  slots: SlotKey[]
  classes: MercClass[]
}
