export interface LootFilterTier {
  rs: number
  hidden: number[]
  highlighted: number[]
}

export interface LootFilterType {
  tiers: LootFilterTier[]
  soc: number
  soch: number
}

export interface LootFilter {
  version: number
  types: Record<number, LootFilterType>
  wtc: number
}

export interface SavedLootFilter {
  id: string
  buildId: string
  name: string
  code: string
  favorite: boolean
  createdAt: string
  updatedAt: string
}
