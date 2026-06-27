import type { StateCreator } from 'zustand'
import { classes, gameConfig } from '../../data'
import { attrPointsFor, emptyAllocation } from './helpers'
import type { BuildStore } from './types'

type CharacterSlice = Pick<
  BuildStore,
  | 'classId'
  | 'level'
  | 'allocated'
  | 'setClass'
  | 'setLevel'
  | 'incAttr'
  | 'decAttr'
  | 'resetAttrs'
>

export const createCharacterSlice: StateCreator<
  BuildStore,
  [],
  [],
  CharacterSlice
> = (set, get) => ({
  classId: classes[0]?.id ?? null,
  level: 1,
  allocated: emptyAllocation(),

  setClass: (id) =>
    set((s) => {
      if (s.classId === id) return s
      return {
        classId: id,
        allocated: emptyAllocation(),
        skillRanks: {},
        activeSkillIds: [],
        activeAuraId: null,
        procToggles: {},
        activeBuffs: {},
        subskillRanks: {},
        treeSocketed: {},
        skillProjectiles: {},
        activeBuildId: null,
        activeProfileId: null,
        notes: '',
        customStats: [],
      }
    }),

  setLevel: (lvl) => {
    const clamped = Math.max(1, Math.min(gameConfig.maxCharacterLevel, lvl))
    set({ level: clamped })
  },

  incAttr: (key, amount = 1) => {
    const { allocated, level } = get()
    const total = Object.values(allocated).reduce((s, v) => s + v, 0)
    const available = attrPointsFor(level) - total
    const step = Math.min(amount, available)
    if (step <= 0) return
    set({ allocated: { ...allocated, [key]: (allocated[key] ?? 0) + step } })
  },

  decAttr: (key, amount = 1) => {
    const { allocated } = get()
    const cur = allocated[key] ?? 0
    const step = Math.min(amount, cur)
    if (step <= 0) return
    set({ allocated: { ...allocated, [key]: cur - step } })
  },

  resetAttrs: () => set({ allocated: emptyAllocation() }),
})
