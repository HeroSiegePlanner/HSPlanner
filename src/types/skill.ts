import type { StatMap } from './game'

export type SkillKind = 'active' | 'passive' | 'aura' | 'buff'

export type DamageType =
  | 'physical'
  | 'lightning'
  | 'cold'
  | 'fire'
  | 'poison'
  | 'arcane'
  | 'explosion'
  | 'magic'

export interface SkillRank {
  rank: number
  description?: string
  stats?: StatMap
  manaCost?: number
  cooldown?: number
}

export interface DamageRange {
  min: number
  max: number
}

export interface DamageFormula {
  base: number
  perLevel: number
}

export type AttackKind = 'attack' | 'spell'

export interface AttackSkillScaling {
  weaponDamagePct?: DamageFormula
  flatPhysicalMin?: DamageFormula
  flatPhysicalMax?: DamageFormula
  attackRatingPct?: DamageFormula
}

export interface ManaCostFormula {
  base: number
  perLevel: number
}

export interface BonusSource {
  source: string
  stat: string
  value: number
  per: 'skill_level' | 'attribute_point'
}

export interface SkillPosition {
  row: number
  col: number
}

export interface PassiveStats {
  base?: Record<string, number>
  perRank?: Record<string, number>
}

export type SkillProcTrigger = 'on_kill' | 'on_cast' | 'on_hit'

export interface SkillProc {
  chance: number
  trigger: SkillProcTrigger
  target: string
}

export interface Skill {
  id: string
  classId: string
  name: string
  kind: SkillKind
  description?: string
  maxRank: number
  requiresLevel?: number
  requiresSkill?: string
  ranks: SkillRank[]
  damageType?: DamageType
  tags?: string[]
  movementDuringUse?: number
  range?: number
  baseCastRate?: number
  baseCooldown?: number
  effectDuration?: number
  damagePerRank?: DamageRange[]
  damageFormula?: DamageFormula
  manaCostFormula?: ManaCostFormula
  bonusSources?: BonusSource[]
  attackKind?: AttackKind
  attackScaling?: AttackSkillScaling
  passiveStats?: PassiveStats
  proc?: SkillProc
  subskills?: SubskillNode[]
  tree?: string
  position?: SkillPosition
  icon?: string
}

export type SubskillRole = 'minor' | 'notable' | 'keystone'

export interface SubskillEffect {
  base?: Record<string, number>
  perRank?: Record<string, number>
}

export interface AppliedState {
  state: string
  amount?: { base?: number; perRank?: number }
}

export interface SubskillProc {
  trigger: SkillProcTrigger
  chance: { base?: number; perRank?: number }
  effects?: SubskillEffect
  tags?: string[]
  target?: string
  appliesStates?: (string | AppliedState)[]
}

export interface SubskillNode {
  id: string
  positionIndex: number
  name: string
  description?: string
  icon?: string
  maxRank: number
  effects?: SubskillEffect
  proc?: SubskillProc
  requiresSubskill?: string
}

export interface ItemGrantedSkill {
  id: string
  name: string
  description?: string
  passiveStats?: {
    base?: Record<string, number>
    perRank?: Record<string, number>
  }
  passiveConverts?: {
    perRank: Array<{ from: string; to: string; pct: number }>
  }
}
