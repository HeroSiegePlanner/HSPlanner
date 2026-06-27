import {
  SELF_CONDITION_KEYS,
  SELF_CONDITION_LABELS,
} from '../../utils/tree/treeStats'

export const ENEMY_CONDITIONS: { key: string; label: string }[] = [
  { key: 'burning', label: 'Enemy is Burning' },
  { key: 'poisoned', label: 'Enemy is Poisoned' },
  { key: 'frozenbite', label: 'Enemy is Frost Bitten' },
  { key: 'stunned', label: 'Enemy is Stunned' },
  { key: 'bleeding', label: 'Enemy is Bleeding' },
  { key: 'shocked', label: 'Enemy is Stasis' },
  { key: 'deep_frozen', label: 'Enemy is Deep Frozen' },
  { key: 'shadow_burn', label: 'Enemy is Shadow Burned' },
  { key: 'is_boss', label: 'Target is Boss' },
]

export const PLAYER_CONDITIONS: { key: string; label: string }[] =
  SELF_CONDITION_KEYS.map((k) => ({ key: k, label: SELF_CONDITION_LABELS[k] }))

export const ENEMY_RESISTANCE_TYPES: { key: string; label: string }[] = [
  { key: 'fire', label: 'Fire' },
  { key: 'cold', label: 'Cold' },
  { key: 'lightning', label: 'Lightning' },
  { key: 'poison', label: 'Poison' },
  { key: 'arcane', label: 'Arcane' },
]

export const RESIST_COLOR: Record<string, string> = {
  fire: 'text-stat-red',
  cold: 'text-stat-blue',
  lightning: 'text-stat-orange',
  poison: 'text-stat-green',
  arcane: 'text-stat-purple',
}
