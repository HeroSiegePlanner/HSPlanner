import { invoke } from '@tauri-apps/api/core'
import { notifyBridgeError } from './calc/bridge'
import type { RangedValue } from '../types/game'
import type { BonusSource, DamageFormula, DamageRange, DamageType } from '../types/skill'
import type {
  AttackSkillDamageBreakdown,
  SkillDamageBreakdown,
  WeaponDamageBreakdown,
} from './item/stats'

export interface NativeSkillRef {
  name: string
  tags?: string[]
  damageType?: DamageType
  damageFormula?: DamageFormula
  damagePerRank?: DamageRange[]
  bonusSources?: BonusSource[]
  attackKind?: 'attack' | 'spell'
  attackScaling?: {
    weaponDamagePct?: DamageFormula
    flatPhysicalMin?: DamageFormula
    flatPhysicalMax?: DamageFormula
    attackRatingPct?: DamageFormula
  }
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
  ofTotalDamage?: number
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

export interface NativeAttackSkillDamageInput extends NativeSkillDamageInput {
  weapon?: NativeWeaponRef
}

export async function computeAttackSkillDamageNative(
  input: NativeAttackSkillDamageInput,
): Promise<AttackSkillDamageBreakdown | null> {
  try {
    return await invoke('compute_attack_skill_damage', { input })
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
