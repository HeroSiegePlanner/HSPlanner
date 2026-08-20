import {
  SELF_CONDITION_KEYS,
  SELF_CONDITION_LABELS,
} from '../../utils/tree/treeStats'

export const ENEMY_CONDITIONS: {
  key: string
  label: string
  color?: string
}[] = [
  { key: 'burning', label: 'Enemy is Burning', color: 'text-stat-red' },
  { key: 'poisoned', label: 'Enemy is Poisoned', color: 'text-stat-green' },
  {
    key: 'frozenbite',
    label: 'Enemy is Frost Bitten',
    color: 'text-stat-blue',
  },
  { key: 'stunned', label: 'Enemy is Stunned' },
  { key: 'bleeding', label: 'Enemy is Bleeding' },
  { key: 'shocked', label: 'Enemy is Stasis', color: 'text-stat-orange' },
  {
    key: 'deep_frozen',
    label: 'Enemy is Deep Frozen',
    color: 'text-stat-blue',
  },
  {
    key: 'shadow_burn',
    label: 'Enemy is Shadow Burned',
    color: 'text-stat-purple',
  },
  { key: 'frozen', label: 'Enemy is Frozen', color: 'text-stat-blue' },
  { key: 'slow', label: 'Enemy is Slowed' },
  { key: 'low_life', label: 'Enemy is Low Life' },
  { key: 'serrated_chains', label: 'Enemy has Serrated Chains' },
  {
    key: 'lightning_break',
    label: 'Enemy has Lightning Break',
    color: 'text-stat-orange',
  },
  { key: 'fire_break', label: 'Enemy has Fire Break', color: 'text-stat-red' },
  { key: 'cold_break', label: 'Enemy has Cold Break', color: 'text-stat-blue' },
  {
    key: 'arcane_break',
    label: 'Enemy has Arcane Break',
    color: 'text-stat-purple',
  },
  {
    key: 'poison_break',
    label: 'Enemy has Poison Break',
    color: 'text-stat-green',
  },
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
