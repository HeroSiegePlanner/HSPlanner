import statsJson from '@data/lootfilter-stats.json'
import type { ItemRarity } from '../../types'

export interface LootFilterStat {
  id: number
  name: string
  col: number
}

export const FILTER_STATS: LootFilterStat[] = statsJson as LootFilterStat[]

export const FILTER_STAT_BY_ID: ReadonlyMap<number, LootFilterStat> = new Map(
  FILTER_STATS.map((s) => [s.id, s]),
)

export const ITEM_TYPES: { id: number; label: string }[] = [
  { id: 0, label: 'Helmet' },
  { id: 3, label: 'Weapon' },
  { id: 6, label: 'Shield' },
  { id: 10, label: 'Charm' },
  { id: 1, label: 'Armor' },
  { id: 4, label: 'Gloves' },
  { id: 7, label: 'Ring' },
  { id: 15, label: 'Socketable' },
  { id: 2, label: 'Boots' },
  { id: 5, label: 'Amulet' },
  { id: 8, label: 'Belt' },
  { id: 18, label: 'Potion' },
]

export const ITEM_TYPE_LABELS: ReadonlyMap<number, string> = new Map(
  ITEM_TYPES.map((t) => [t.id, t.label]),
)

export const SOCKET_COUNT = 6

export const WEAPON_TYPES: readonly (string | null)[] = [
  null,
  'Sword',
  'Dagger',
  'Mace',
  'Axe',
  'Claw',
  'Polearm',
  'Chainsaw',
  'Staff',
  'Cane',
  'Wand',
  'Book',
  'Spellblade',
  'Bow',
  'Gun',
  'Flask',
  'Throwing',
  null,
]

export const FILTER_RARITIES: readonly ItemRarity[] = [
  'common',
  'uncommon',
  'rare',
  'mythic',
  'satanic',
]

export const TIER_LABELS = ['D', 'C', 'B', 'A', 'S'] as const
