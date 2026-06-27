import type { StateCreator } from 'zustand'
import { getItem } from '../../data'
import * as itemEdits from '../../views/gear/lib/itemEdits'
import type { BuildStore } from './types'

type InventorySlice = Pick<
  BuildStore,
  | 'inventory'
  | 'equipItem'
  | 'unequipItem'
  | 'commitEquippedItem'
  | 'setSocketCount'
  | 'setSocketed'
  | 'setSocketType'
  | 'setStars'
  | 'applyRuneword'
  | 'setAugment'
  | 'setAugmentLevel'
  | 'addAffix'
  | 'removeAffix'
  | 'setAffixRoll'
  | 'addForgedMod'
  | 'removeForgedMod'
  | 'moveItem'
>

export const createInventorySlice: StateCreator<
  BuildStore,
  [],
  [],
  InventorySlice
> = (set, get) => ({
  inventory: {},

  equipItem: (slot, baseId) => {
    const item = itemEdits.makeEquippedItem(baseId)
    if (!item) return
    get().commitEquippedItem(slot, item)
  },

  unequipItem: (slot) => {
    set((s) => {
      const next = { ...s.inventory }
      delete next[slot]
      return { inventory: next }
    })
  },

  commitEquippedItem: (slot, item) => {
    set((s) => {
      if (item === null) {
        const next = { ...s.inventory }
        delete next[slot]
        return { inventory: next }
      }
      const base = getItem(item.baseId)
      if (!base) return s
      const next = { ...s.inventory, [slot]: item }
      if (slot === 'weapon' && base.twoHanded) {
        delete next.offhand
      }
      return { inventory: next }
    })
  },

  setSocketCount: (slot, count) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withSocketCount(cur, count) } }
    })
  },

  setSocketed: (slot, idx, socketableId) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withSocketed(cur, idx, socketableId) } }
    })
  },

  setSocketType: (slot, idx, type) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withSocketType(cur, idx, type) } }
    })
  },

  setStars: (slot, count) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withStars(cur, count) } }
    })
  },

  applyRuneword: (slot, runewordId) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withRuneword(cur, runewordId) } }
    })
  },

  setAugment: (augmentId) => {
    set((s) => {
      const cur = s.inventory.armor
      if (!cur) return s
      return { inventory: { ...s.inventory, armor: itemEdits.withAugment(cur, augmentId) } }
    })
  },

  setAugmentLevel: (level) => {
    set((s) => {
      const cur = s.inventory.armor
      if (!cur) return s
      return { inventory: { ...s.inventory, armor: itemEdits.withAugmentLevel(cur, level) } }
    })
  },

  addAffix: (slot, affixId, tier) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withAffixAdded(cur, affixId, tier) } }
    })
  },

  removeAffix: (slot, index) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withAffixRemoved(cur, index) } }
    })
  },

  setAffixRoll: (slot, index, roll) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur || index < 0 || index >= cur.affixes.length) return s
      const clamped = Math.max(0, Math.min(1, roll))
      const affixes = cur.affixes.map((a, i) =>
        i === index ? { ...a, roll: clamped } : a,
      )
      return { inventory: { ...s.inventory, [slot]: { ...cur, affixes } } }
    })
  },

  addForgedMod: (slot, modId, tier) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withForgedModAdded(cur, modId, tier) } }
    })
  },

  removeForgedMod: (slot, index) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withForgedModRemoved(cur, index) } }
    })
  },

  moveItem: (fromSlot, toSlot) => {
    set((s) => {
      if (fromSlot === toSlot) return s
      const fromItem = s.inventory[fromSlot]
      const toItem = s.inventory[toSlot]
      const next = { ...s.inventory }

      if (fromItem) {
        next[toSlot] = fromItem
      } else {
        delete next[toSlot]
      }

      if (toItem) {
        next[fromSlot] = toItem
      } else {
        delete next[fromSlot]
      }

      return { inventory: next }
    })
  },
})
