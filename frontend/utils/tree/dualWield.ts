import { getItem, incarnationNodeInfo } from '@data'
import type { Inventory, ItemBase } from '../../types'

// Incarnation notes that widen what the offhand takes. Wording drifts between
// seasons, so match loosely rather than by node id.
const HERCULES_GRIP =
  /dual wield(?:ing)?[^|]*(?:melee weapons|swords, maces and axes)/i
const MASTER_OF_WANDS = /dual wield(?:ing)? wands/i

const GRIP_TYPES = ['Sword', 'Mace', 'Axe', 'Polearm', 'Claw']

function hasNote(allocatedTreeNodes: Set<number>, note: RegExp): boolean {
  for (const nodeId of allocatedTreeNodes) {
    const info = incarnationNodeInfo[String(nodeId)]
    if (!info) continue
    if (note.test([...(info.l ?? []), info.note ?? ''].join(' | '))) return true
  }
  return false
}

function hasGrip(allocatedTreeNodes: Set<number>): boolean {
  return hasNote(allocatedTreeNodes, HERCULES_GRIP)
}

/** Nothing at all fits the offhand — a two-handed weapon holds both hands. */
export function isOffhandLocked(
  mainhand: ItemBase | undefined,
  allocatedTreeNodes: Set<number>,
): boolean {
  if (!mainhand?.twoHanded) return false
  return !(GRIP_TYPES.includes(mainhand.baseType) && hasGrip(allocatedTreeNodes))
}

export function canOffhand(
  base: ItemBase,
  mainhand: ItemBase | undefined,
  allocatedTreeNodes: Set<number>,
): boolean {
  // Hercules Grip is the only way a two-handed weapon shares a hand, and it
  // covers swords, maces and axes on both sides — never a shield.
  if (mainhand?.twoHanded || base.twoHanded) {
    if (base.slot !== 'weapon' || !hasGrip(allocatedTreeNodes)) return false
    if (mainhand && !GRIP_TYPES.includes(mainhand.baseType)) return false
    return GRIP_TYPES.includes(base.baseType)
  }
  if (base.slot === 'offhand') return true
  if (base.slot !== 'weapon') return false
  if (base.baseType === 'Wand') return hasNote(allocatedTreeNodes, MASTER_OF_WANDS)
  return true
}

/** Drops an offhand item the current mainhand and tree no longer allow. */
export function withValidOffhand(
  inventory: Inventory,
  allocatedTreeNodes: Set<number>,
): Inventory {
  const offhand = inventory.offhand
  const offhandBase = offhand ? getItem(offhand.baseId) : undefined
  if (!offhandBase) return inventory
  const mainhand = inventory.weapon ? getItem(inventory.weapon.baseId) : undefined
  if (canOffhand(offhandBase, mainhand, allocatedTreeNodes)) return inventory
  const next = { ...inventory }
  delete next.offhand
  return next
}
