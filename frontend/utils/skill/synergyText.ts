import { formatValue, statName } from '../item/stats'
import type { BonusSource } from '../../types'

export interface SkillSynergy {
  name: string
  text: string
}

export function synergyEffectText(source: BonusSource): string {
  const unit = source.per === 'skill_level' ? 'rank' : 'point'
  return `${formatValue(source.value, source.stat)} ${statName(source.stat)} per ${unit}`
}

export function bonusSourceSynergy(source: BonusSource): SkillSynergy {
  return { name: source.source, text: synergyEffectText(source) }
}
