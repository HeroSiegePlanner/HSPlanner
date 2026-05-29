import type { ItemRarity } from '../../../types'

export const RARITY_TEXT: Record<ItemRarity, string> = {
  common: 'text-white',
  uncommon: 'text-sky-400',
  rare: 'text-accent-hot',
  mythic: 'text-purple-400',
  satanic: 'text-red-500',
  heroic: 'text-lime-400',
  angelic: 'text-yellow-200',
  satanic_set: 'text-green-400',
  unholy: 'text-pink-400',
  relic: 'text-orange-300',
}

export const RARITY_BG: Record<ItemRarity, string> = {
  common: 'bg-white/5',
  uncommon: 'bg-sky-500/10',
  rare: 'bg-yellow-500/10',
  mythic: 'bg-purple-500/10',
  satanic: 'bg-red-500/10',
  heroic: 'bg-lime-500/10',
  angelic: 'bg-yellow-400/10',
  satanic_set: 'bg-green-500/10',
  unholy: 'bg-pink-500/10',
  relic: 'bg-orange-500/10',
}

export const RARITY_ORDER: Record<ItemRarity, number> = {
  unholy: 0,
  angelic: 1,
  heroic: 2,
  satanic_set: 3,
  satanic: 4,
  common: 5,
  relic: 6,
  mythic: 7,
  rare: 8,
  uncommon: 9,
}

export const RARITY_BORDER: Record<ItemRarity, string> = {
  common: 'border-white/30',
  uncommon: 'border-sky-400/40',
  rare: 'border-accent/50',
  mythic: 'border-purple-400/40',
  satanic: 'border-red-500/40',
  heroic: 'border-lime-400/40',
  angelic: 'border-yellow-200/40',
  satanic_set: 'border-green-400/40',
  unholy: 'border-pink-400/40',
  relic: 'border-orange-300/40',
}

export const RARITY_LABEL: Record<ItemRarity, string> = {
  common: 'Common',
  uncommon: 'Superior',
  rare: 'Rare',
  mythic: 'Mythic',
  satanic: 'Satanic',
  heroic: 'Heroic',
  angelic: 'Angelic',
  satanic_set: 'Satanic Set',
  unholy: 'Unholy',
  relic: 'Relic',
}
