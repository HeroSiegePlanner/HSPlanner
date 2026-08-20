import { gameConfig, getItem } from '@data'
import type { EquippedItem, SlotKey } from '../../../types'
import { canOffhand } from '../../../utils/tree/dualWield'

function slotGroup(slotKey: SlotKey): string {
  return slotKey.replace(/_\d+$/, '')
}

// First free slot matching the item's slot group; falls back to the first
// matching slot (overwrite) when none are free. null = base unknown.
export function targetSlotFor(
  item: EquippedItem,
  inventory: Partial<Record<SlotKey, EquippedItem | undefined>>,
): SlotKey | null {
  const base = getItem(item.baseId)
  if (!base) return null
  const group = slotGroup(base.slot)
  const candidates = gameConfig.slots
    .map((s) => s.key)
    .filter((k) => slotGroup(k) === group)
  if (candidates.length === 0) return null
  return candidates.find((k) => !inventory[k]) ?? candidates[0]!
}

// Slots the user picks between when equipping from the stash: weapons offer
// the offhand only when dual wielding allows it, rings and potions every slot.
export function equipTargets(
  item: EquippedItem,
  inventory: Partial<Record<SlotKey, EquippedItem | undefined>>,
  allocatedTreeNodes: Set<number>,
): SlotKey[] {
  const base = getItem(item.baseId)
  if (!base) return []
  if (base.slot === 'weapon' || base.slot === 'offhand') {
    const mainhand = inventory.weapon ? getItem(inventory.weapon.baseId) : undefined
    const targets: SlotKey[] = base.slot === 'weapon' ? ['weapon'] : []
    if (canOffhand(base, mainhand, allocatedTreeNodes)) targets.push('offhand')
    return targets
  }
  const group = slotGroup(base.slot)
  if (group === 'ring' || group === 'potion') {
    return gameConfig.slots
      .map((s) => s.key)
      .filter((k) => slotGroup(k) === group)
  }
  const auto = targetSlotFor(item, inventory)
  return auto ? [auto] : []
}
