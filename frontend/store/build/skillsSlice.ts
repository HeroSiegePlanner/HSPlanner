import type { StateCreator } from 'zustand'
import { skills as ALL_SKILLS } from '@data'
import {
  skillPointsFor,
  subskillKey,
  subskillPointsFor,
  subskillSpentFor,
} from './helpers'
import type { BuildStore } from './types'

const SKILL_BY_ID = new Map(ALL_SKILLS.map((s) => [s.id, s]))
const DEPENDENT_SKILL_IDS = new Map<string, string[]>()
for (const s of ALL_SKILLS) {
  if (!s.requiresSkill) continue
  const list = DEPENDENT_SKILL_IDS.get(s.requiresSkill) ?? []
  list.push(s.id)
  DEPENDENT_SKILL_IDS.set(s.requiresSkill, list)
}

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

    const skillDef = SKILL_BY_ID.get(skillId)
    if (clamped > 0 && skillDef?.requiresSkill) {
      const reqRank = skillRanks[skillDef.requiresSkill] ?? 0
      if (reqRank === 0) return
    }

    const next = { ...skillRanks }
    if (clamped === 0) delete next[skillId]
    else next[skillId] = clamped

    if (clamped === 0) {
      const queue = [skillId]
      while (queue.length > 0) {
        const removedId = queue.pop()!
        for (const depId of DEPENDENT_SKILL_IDS.get(removedId) ?? []) {
          if ((next[depId] ?? 0) > 0) {
            delete next[depId]
            queue.push(depId)
          }
        }
      }
    }

    set({ skillRanks: next })
  },

  incSkillRank: (skillId, maxRank, amount = 1) => {
    const cur = get().skillRanks[skillId] ?? 0
    get().setSkillRank(skillId, cur + amount, maxRank)
  },

  decSkillRank: (skillId, amount = 1) => {
    const cur = get().skillRanks[skillId] ?? 0
    if (cur <= 0) return
    get().setSkillRank(skillId, cur - amount, cur)
  },

  resetSkillRanks: () => set({ skillRanks: {} }),

  setSubskillRank: (skillId, subskillId, rank, maxRank) => {
    const { subskillRanks, level } = get()
    const total = subskillPointsFor(level)
    const key = subskillKey(skillId, subskillId)
    const cur = subskillRanks[key] ?? 0
    const otherSpent = subskillSpentFor(subskillRanks, skillId) - cur
    const clamped = Math.max(0, Math.min(maxRank, rank, total - otherSpent))
    if (clamped === cur) return
    const next = { ...subskillRanks }
    if (clamped === 0) delete next[key]
    else next[key] = clamped
    set({ subskillRanks: next })
  },

  incSubskillRank: (skillId, subskillId, maxRank, amount = 1) => {
    const cur = get().subskillRanks[subskillKey(skillId, subskillId)] ?? 0
    get().setSubskillRank(skillId, subskillId, cur + amount, maxRank)
  },

  decSubskillRank: (skillId, subskillId, amount = 1) => {
    const cur = get().subskillRanks[subskillKey(skillId, subskillId)] ?? 0
    if (cur <= 0) return
    get().setSubskillRank(skillId, subskillId, cur - amount, cur)
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
