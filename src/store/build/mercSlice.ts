import type { StateCreator } from 'zustand'
import { getItem, getMercClass, mercData } from '../../data'
import type { BuildStore } from './types'

type MercSlice = Pick<
  BuildStore,
  | 'mercClassId'
  | 'mercSkillRanks'
  | 'mercInventory'
  | 'mercDisabledAuras'
  | 'setMercClass'
  | 'setMercSkillRank'
  | 'commitMercItem'
  | 'setMercAuraDisabled'
  | 'resetMerc'
>

export const createMercSlice: StateCreator<
  BuildStore,
  [],
  [],
  MercSlice
> = (set) => ({
  mercClassId: null,
  mercSkillRanks: {},
  mercInventory: {},
  mercDisabledAuras: {},

  setMercClass: (id) =>
    set((s) => {
      if (id !== null && !getMercClass(id)) return s
      if (id === s.mercClassId) return s
      return { mercClassId: id, mercSkillRanks: {} }
    }),

  setMercSkillRank: (skillId, rank, maxRank = mercData.maxSkillRank) =>
    set((s) => {
      const clamped = Math.max(0, Math.min(maxRank, Math.floor(rank)))
      const next = { ...s.mercSkillRanks }
      if (clamped <= 0) delete next[skillId]
      else next[skillId] = clamped
      return { mercSkillRanks: next }
    }),

  commitMercItem: (slot, item) =>
    set((s) => {
      if (item === null) {
        const next = { ...s.mercInventory }
        delete next[slot]
        return { mercInventory: next }
      }
      const base = getItem(item.baseId)
      if (!base) return s
      const next = { ...s.mercInventory, [slot]: item }
      if (slot === 'weapon' && base.twoHanded) {
        delete next.offhand
      }
      return { mercInventory: next }
    }),

  setMercAuraDisabled: (auraKey, disabled) =>
    set((s) => {
      const next = { ...s.mercDisabledAuras }
      if (disabled) next[auraKey] = true
      else delete next[auraKey]
      return { mercDisabledAuras: next }
    }),

  resetMerc: () =>
    set({
      mercClassId: null,
      mercSkillRanks: {},
      mercInventory: {},
      mercDisabledAuras: {},
    }),
})
