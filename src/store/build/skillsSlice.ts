import type { StateCreator } from 'zustand'
import { skills as ALL_SKILLS } from '../../data'
import { skillPointsFor, subskillKey, subskillPointsFor } from './helpers'
import type { BuildStore } from './types'

type SkillsSlice = Pick<
  BuildStore,
  | 'skillRanks'
  | 'subskillRanks'
  | 'activeSkillIds'
  | 'activeAuraId'
  | 'procToggles'
  | 'activeBuffs'
  | 'skillProjectiles'
  | 'setSkillRank'
  | 'incSkillRank'
  | 'decSkillRank'
  | 'resetSkillRanks'
  | 'setSubskillRank'
  | 'incSubskillRank'
  | 'decSubskillRank'
  | 'resetSubskillsFor'
  | 'toggleActiveSkill'
  | 'setActiveAura'
  | 'setProcToggle'
  | 'setBuffActive'
  | 'setSkillProjectiles'
>

export const createSkillsSlice: StateCreator<
  BuildStore,
  [],
  [],
  SkillsSlice
> = (set, get) => ({
  skillRanks: {},
  subskillRanks: {},
  activeSkillIds: [],
  activeAuraId: null,
  procToggles: {},
  activeBuffs: {},
  skillProjectiles: {},

  setSkillRank: (skillId, rank, maxRank) => {
    const { skillRanks, level } = get()
    const total = skillPointsFor(level)
    const currentSkillRank = skillRanks[skillId] ?? 0
    const otherSpent =
      Object.entries(skillRanks).reduce(
        (s, [id, r]) => (id === skillId ? s : s + r),
        0,
      )
    const clamped = Math.max(0, Math.min(maxRank, rank, total - otherSpent))
    if (clamped === currentSkillRank) return

    const skillDef = ALL_SKILLS.find((s) => s.id === skillId)
    if (clamped > 0 && skillDef?.requiresSkill) {
      const reqRank = skillRanks[skillDef.requiresSkill] ?? 0
      if (reqRank === 0) return
    }

    const next = { ...skillRanks }
    if (clamped === 0) delete next[skillId]
    else next[skillId] = clamped

    if (clamped === 0) {
      let changed = true
      while (changed) {
        changed = false
        for (const dep of ALL_SKILLS) {
          if (
            dep.requiresSkill &&
            (next[dep.id] ?? 0) > 0 &&
            !next[dep.requiresSkill]
          ) {
            delete next[dep.id]
            changed = true
          }
        }
      }
    }

    set({ skillRanks: next })
  },

  incSkillRank: (skillId, maxRank) => {
    const { skillRanks } = get()
    const cur = skillRanks[skillId] ?? 0
    get().setSkillRank(skillId, cur + 1, maxRank)
  },

  decSkillRank: (skillId) => {
    const { skillRanks } = get()
    const cur = skillRanks[skillId] ?? 0
    if (cur <= 0) return
    const next = { ...skillRanks }
    if (cur - 1 === 0) delete next[skillId]
    else next[skillId] = cur - 1
    set({ skillRanks: next })
  },

  resetSkillRanks: () => set({ skillRanks: {} }),

  setSubskillRank: (skillId, subskillId, rank, maxRank) => {
    const { subskillRanks, level } = get()
    const total = subskillPointsFor(level)
    const key = subskillKey(skillId, subskillId)
    const cur = subskillRanks[key] ?? 0
    const otherSpent = Object.entries(subskillRanks).reduce(
      (s, [k, r]) => (k === key ? s : s + r),
      0,
    )
    const clamped = Math.max(0, Math.min(maxRank, rank, total - otherSpent))
    if (clamped === cur) return
    const next = { ...subskillRanks }
    if (clamped === 0) delete next[key]
    else next[key] = clamped
    set({ subskillRanks: next })
  },

  incSubskillRank: (skillId, subskillId, maxRank) => {
    const cur =
      get().subskillRanks[subskillKey(skillId, subskillId)] ?? 0
    get().setSubskillRank(skillId, subskillId, cur + 1, maxRank)
  },

  decSubskillRank: (skillId, subskillId) => {
    const key = subskillKey(skillId, subskillId)
    const cur = get().subskillRanks[key] ?? 0
    if (cur <= 0) return
    set((s) => {
      const next = { ...s.subskillRanks }
      if (cur - 1 === 0) delete next[key]
      else next[key] = cur - 1
      return { subskillRanks: next }
    })
  },

  resetSubskillsFor: (skillId) =>
    set((s) => {
      const next: Record<string, number> = {}
      for (const [k, v] of Object.entries(s.subskillRanks)) {
        if (!k.startsWith(`${skillId}:`)) next[k] = v
      }
      return { subskillRanks: next }
    }),

  toggleActiveSkill: (skillId) =>
    set((s) => ({
      activeSkillIds: s.activeSkillIds.includes(skillId)
        ? s.activeSkillIds.filter((id) => id !== skillId)
        : [...s.activeSkillIds, skillId],
    })),

  setActiveAura: (skillId) => set({ activeAuraId: skillId }),

  setProcToggle: (skillId, enabled) =>
    set((s) => {
      const next = { ...s.procToggles }
      if (enabled) next[skillId] = true
      else delete next[skillId]
      return { procToggles: next }
    }),

  setBuffActive: (skillId, enabled) =>
    set((s) => {
      const next = { ...s.activeBuffs }
      if (enabled) next[skillId] = true
      else delete next[skillId]
      return { activeBuffs: next }
    }),

  setSkillProjectiles: (skillId, count) =>
    set((s) => {
      const next = { ...s.skillProjectiles }
      if (
        count === null ||
        !Number.isFinite(count) ||
        (count as number) <= 1
      ) {
        delete next[skillId]
      } else {
        next[skillId] = Math.max(1, Math.floor(count as number))
      }
      return { skillProjectiles: next }
    }),
})
