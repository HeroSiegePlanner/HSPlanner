import type { SlotKey } from '../../../types'

export interface SlotDefLite {
  key: SlotKey
  name: string
}

export interface GearSlotGroups {
  gear: SlotDefLite[]
  potions: SlotDefLite[]
  relics: SlotDefLite[]
}

export function groupGearSlots(slots: SlotDefLite[]): GearSlotGroups {
  // Display grouping for the gear view panels; charm slots are excluded.
  const gear: SlotDefLite[] = []
  const potions: SlotDefLite[] = []
  const relics: SlotDefLite[] = []
  for (const s of slots) {
    if (s.key.startsWith('charm_')) continue
    if (s.key.startsWith('potion_')) potions.push(s)
    else if (s.key.startsWith('relic_')) relics.push(s)
    else gear.push(s)
  }
  return { gear, potions, relics }
}
