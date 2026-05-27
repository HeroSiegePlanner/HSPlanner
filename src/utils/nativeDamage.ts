import { invoke } from '@tauri-apps/api/core'
import type { RangedValue } from '../types/game'
import type { BonusSource, DamageFormula, DamageRange, DamageType } from '../types/skill'
import type { SkillDamageBreakdown, WeaponDamageBreakdown } from './item/stats'

export interface NativeSkillRef {
  name: string
  tags?: string[]
  damageType?: DamageType
  damageFormula?: DamageFormula
  damagePerRank?: DamageRange[]
  bonusSources?: BonusSource[]
}

export interface NativeWeaponRef {
  name: string
  damageMin: number
  damageMax: number
}

export interface NativeSkillDamageInput {
  skill: NativeSkillRef
  allocatedRank: number
  attributes?: Record<string, RangedValue>
  stats?: Record<string, RangedValue>
  skillRanksByName?: Record<string, number>
  itemSkillBonuses?: Record<string, [number, number]>
  enemyConditions?: Record<string, boolean>
  enemyResistances?: Record<string, number>
  skillsByName?: Record<string, NativeSkillRef>
  projectileCount?: number
}

export interface NativeWeaponDamageInput {
  weapon?: NativeWeaponRef
  stats?: Record<string, RangedValue>
  enemyConditions?: Record<string, boolean>
  enemyResistances?: Record<string, number>
  projectileCount?: number
}

export function computeSkillDamageNative(
  input: NativeSkillDamageInput,
): Promise<SkillDamageBreakdown | null> {
  return invoke('compute_skill_damage', { input })
}

export function computeWeaponDamageNative(
  input: NativeWeaponDamageInput,
): Promise<WeaponDamageBreakdown> {
  return invoke('compute_weapon_damage', { input })
}
