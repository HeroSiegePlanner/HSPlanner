import type { StateCreator } from 'zustand'
import { findPath, reachableFromAny } from '../../utils/tree/treeGraph'
import { preserveOrder } from '../../utils/tree/progressionOrder'
import {
  ETHER_ADJ,
  ETHER_START_IDS,
} from '../../utils/tree/etherGraph'
import type { BuildStore } from './types'

type EtherSlice = Pick<
  BuildStore,
  'allocatedEtherNodes' | 'toggleEtherNode' | 'resetEtherNodes'
>

export const createEtherSlice: StateCreator<
  BuildStore,
  [],
  [],
  EtherSlice
> = (set) => ({
  allocatedEtherNodes: new Set<number>(),

  toggleEtherNode: (nodeId) =>
    set((s) => {
      const cur = s.allocatedEtherNodes
      if (cur.has(nodeId)) {
        const next = new Set(cur)
        next.delete(nodeId)
        const stillReachable = reachableFromAny(
          ETHER_START_IDS,
          next,
          ETHER_ADJ,
        )
        return { allocatedEtherNodes: preserveOrder(next, stillReachable) }
      }
      const sources = new Set<number>([...cur, ...ETHER_START_IDS])
      const path = findPath(sources, nodeId, ETHER_ADJ)
      if (!path) return s
      const next = new Set(cur)
      for (const id of path) next.add(id)
      return { allocatedEtherNodes: next }
    }),

  resetEtherNodes: () => set({ allocatedEtherNodes: new Set<number>() }),
})
