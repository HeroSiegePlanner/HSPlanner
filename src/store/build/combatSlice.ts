import type { StateCreator } from 'zustand'
import { defaultEnemyResistances } from '../../utils/build/shareBuild'
import type { BuildStore } from './types'

type CombatSlice = Pick<
  BuildStore,
  | 'killsPerSec'
  | 'enemyConditions'
  | 'playerConditions'
  | 'enemyResistances'
  | 'setKillsPerSec'
  | 'setEnemyCondition'
  | 'setPlayerCondition'
  | 'setEnemyResistance'
>

export const createCombatSlice: StateCreator<
  BuildStore,
  [],
  [],
  CombatSlice
> = (set) => ({
  killsPerSec: 1,
  enemyConditions: {},
  playerConditions: {},
  enemyResistances: defaultEnemyResistances(),

  setKillsPerSec: (rate) =>
    set({ killsPerSec: Math.max(0, rate) }),

  setEnemyCondition: (key, enabled) =>
    set((s) => {
      const next = { ...s.enemyConditions }
      if (enabled) next[key] = true
      else delete next[key]
      return { enemyConditions: next }
    }),

  setPlayerCondition: (key, enabled) =>
    set((s) => {
      const next = { ...s.playerConditions }
      if (enabled) next[key] = true
      else delete next[key]
      return { playerConditions: next }
    }),

  setEnemyResistance: (damageType, value) =>
    set((s) => {
      const next = { ...s.enemyResistances }
      if (value === null || !Number.isFinite(value)) {
        delete next[damageType]
      } else {
        next[damageType] = value
      }
      return { enemyResistances: next }
    }),
})
