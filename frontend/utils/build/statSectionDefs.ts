import type { RangedValue } from '../../types'
import type { Tone } from './sharePayload'

export const ATTRIBUTE_ORDER: string[] = [
  'strength',
  'dexterity',
  'intelligence',
  'energy',
  'vitality',
  'armor',
]

export const OFFENSE_KEYS = [
  'enhanced_damage',
  'attack_damage',
  'increased_attack_speed',
  'faster_cast_rate',
  'crit_chance',
  'crit_damage',
  'life_steal',
  'mana_steal',
]

export const DEFENSE_KEYS = [
  'life',
  'mana',
  'life_replenish',
  'mana_replenish',
  'block_chance',
  'physical_damage_reduction',
  'magic_damage_reduction',
]

export interface ResistanceStyle {
  key: string
  label: string
  className: string
}

export const RESISTANCES: ResistanceStyle[] = [
  { key: 'fire_resistance', label: 'Fire', className: 'text-stat-red' },
  { key: 'cold_resistance', label: 'Cold', className: 'text-stat-blue' },
  {
    key: 'lightning_resistance',
    label: 'Lightning',
    className: 'text-stat-orange',
  },
  { key: 'poison_resistance', label: 'Poison', className: 'text-stat-green' },
  { key: 'arcane_resistance', label: 'Arcane', className: 'text-stat-purple' },
]

export const ATTR_COLOR: Record<string, string> = {
  strength: 'text-stat-orange',
  dexterity: 'text-stat-green',
  intelligence: 'text-stat-purple',
  energy: 'text-stat-blue',
  vitality: 'text-stat-red',
  armor: 'text-text',
}

export const GOLD_OFFENSE = new Set(['enhanced_damage', 'crit_chance', 'crit_damage'])
export const GOLD_DEFENSE = new Set(['life'])
export const BLUE_DEFENSE = new Set(['mana', 'mana_replenish'])

export function effectiveStatValue(
  stats: Record<string, RangedValue>,
  statsCombined: Record<string, RangedValue>,
  key: string,
): RangedValue {
  return statsCombined[key] ?? stats[key] ?? 0
}

const CLASS_TO_TONE: Record<string, Tone> = {
  'text-stat-orange': 'orange',
  'text-stat-green': 'green',
  'text-stat-purple': 'purple',
  'text-stat-blue': 'blue',
  'text-stat-red': 'red',
}

export function offenseRowTone(key: string): Tone | undefined {
  return GOLD_OFFENSE.has(key) ? 'gold' : undefined
}

export function defenseRowTone(key: string): Tone | undefined {
  if (GOLD_DEFENSE.has(key)) return 'gold'
  if (BLUE_DEFENSE.has(key)) return 'blue'
  return undefined
}

export function attributeRowTone(key: string): Tone | undefined {
  const cls = ATTR_COLOR[key]
  return cls ? CLASS_TO_TONE[cls] : undefined
}

export function resistanceRowTone(className: string): Tone | undefined {
  return CLASS_TO_TONE[className]
}
