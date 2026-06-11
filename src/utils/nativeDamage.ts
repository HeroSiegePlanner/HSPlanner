import { invoke } from '@tauri-apps/api/core'
import { notifyBridgeError } from '../lib/calc/bridge'
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

export async function computeSkillDamageNative(
  input: NativeSkillDamageInput,
): Promise<SkillDamageBreakdown | null> {
  try {
    return await invoke('compute_skill_damage', { input })
  } catch (err) {
    throw notifyBridgeError(err)
  }
}

export async function computeWeaponDamageNative(
  input: NativeWeaponDamageInput,
): Promise<WeaponDamageBreakdown> {
  try {
    return await invoke('compute_weapon_damage', { input })
  } catch (err) {
    throw notifyBridgeError(err)
  }
}
