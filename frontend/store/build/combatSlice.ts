import type { StateCreator } from 'zustand'
import { defaultEnemyResistances } from '../../utils/build/shareBuild'
import { defaultEntityRates } from '../../utils/build/entityRates'
import type { BuildStore } from './types'

type CombatSlice = Pick<
  BuildStore,
  | 'killsPerSec'
  | 'entityRates'
  | 'stackCounts'
  | 'enemyConditions'
  | 'playerConditions'
  | 'enemyResistances'
  | 'setKillsPerSec'
  | 'setEntityRate'
  | 'setStackCount'
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
  entityRates: defaultEntityRates(),
  stackCounts: {},
  enemyConditions: {},
  playerConditions: {},
  enemyResistances: defaultEnemyResistances(),

  setKillsPerSec: (rate) =>
    set({ killsPerSec: Math.max(0, rate) }),

  setEntityRate: (kind, rate) =>
    set((s) => ({
      entityRates: { ...s.entityRates, [kind]: Math.max(0, rate) },
    })),

  setStackCount: (key, count) =>
    set((s) => {
      const next = { ...s.stackCounts }
      if (count === null || !Number.isFinite(count)) delete next[key]
      else next[key] = Math.max(0, Math.floor(count))
      return { stackCounts: next }
    }),

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
